use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Default)]
pub struct WslDistro {
    pub name: String,
    pub state: String,
    pub version: u32,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct WslInfo {
    pub installed: bool,
    pub default_version: u32,
    pub distros: Vec<WslDistro>,
    pub kernel_version: String,
    pub wsl_version: String,
    pub error: String,
}


// Anti-freeze : wsl.exe est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn get_wsl_info() -> WslInfo {
    tokio::task::spawn_blocking(get_wsl_info_blocking)
        .await
        .unwrap_or_default()
}

fn get_wsl_info_blocking() -> WslInfo {
    // Check if wsl.exe exists
    #[cfg(target_os = "windows")]
    {
        let check = Command::new("wsl")
            .args(["--status"])
            .creation_flags(0x08000000)
            .output();

        if check.is_err() {
            return WslInfo {
                installed: false,
                error: "WSL non installé ou non disponible".to_string(),
                ..Default::default()
            };
        }

        // Get distro list via PowerShell parsing wsl --list --verbose
        let ps = r#"
try {
    $wslExe = (Get-Command wsl -EA SilentlyContinue)
    if (-not $wslExe) { throw "WSL non trouvé" }

    # wsl.exe émet de l'UTF-16LE ; capturé en codepage OEM par PowerShell, la
    # sortie devient du mojibake et le parsing renvoie 0 distro. WSL_UTF8=1 force
    # une sortie UTF-8 correctement décodée (variable officielle de wsl.exe).
    $env:WSL_UTF8 = "1"

    # wsl.exe peut exister (stub Windows) sans que WSL soit reellement installe :
    # `--list --verbose` echoue alors et 2>&1 renvoie des objets ErrorRecord, pas
    # des chaines — appeler .Trim() dessus plante avec une exception technique
    # brute ("ne contient pas de methode nommee Trim"). "$_" force la conversion
    # en texte (fonctionne pour String ET ErrorRecord) avant tout .Trim().
    $raw = & wsl --list --verbose 2>&1
    $lines = $raw | Where-Object { $_ -and "$_".Trim() -ne '' } | Select-Object -Skip 1

    $distros = @($lines | ForEach-Object {
        $line = "$_".TrimEnd()
        # STATE peut contenir des espaces selon la locale (FR « En cours
        # d'exécution ») : capture non-gourmande jusqu'au numéro de version final.
        if ($line -match '^\*?\s+(\S+)\s+(.+?)\s+(\d+)\s*$') {
            $isDefault = $line.TrimStart().StartsWith('*')
            @{
                name    = $Matches[1]
                state   = $Matches[2].Trim()
                version = [int]$Matches[3]
                default = $isDefault
            }
        }
    } | Where-Object { $_ })

    # WSL version
    $verRaw = & wsl --version 2>&1 | Select-Object -First 3
    $wslVer = ($verRaw | Where-Object { $_ -match 'WSL version' } | Select-Object -First 1) -replace '.*:\s*',''
    $kernelVer = ($verRaw | Where-Object { $_ -match 'Kernel version' } | Select-Object -First 1) -replace '.*:\s*',''

    # Default WSL version
    $defVer = 2
    try {
        $defRaw = & wsl --status 2>&1
        if ($defRaw -match 'Default Version:\s*(\d+)') { $defVer = [int]$Matches[1] }
    } catch {}

    @{
        installed = $true
        defaultVersion = $defVer
        distros = $distros
        kernelVersion = if($kernelVer){$kernelVer.Trim()}else{''}
        wslVersion = if($wslVer){$wslVer.Trim()}else{''}
        error = ''
    } | ConvertTo-Json -Depth 4 -Compress
} catch {
    @{ installed=$false; defaultVersion=0; distros=@(); kernelVersion=''; wslVersion=''; error=$_.Exception.Message } | ConvertTo-Json -Compress
}
"#;
        let o = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output();
        if let Ok(o) = o {
            // decode_output : le champ state ("En cours d'exécution") survit à la
            // capture wsl.exe (WSL_UTF8 dans le script) mais pas forcément à
            // l'écriture finale ConvertTo-Json de PowerShell lui-même vers Rust.
            let t = crate::maintenance::commands::decode_output(&o.stdout);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(t.trim()) {
                let distros = v["distros"].as_array().map(|arr| {
                    arr.iter().map(|d| WslDistro {
                        name: d["name"].as_str().unwrap_or("").to_string(),
                        state: d["state"].as_str().unwrap_or("").to_string(),
                        version: d["version"].as_u64().unwrap_or(2) as u32,
                        is_default: d["default"].as_bool().unwrap_or(false),
                    }).collect()
                }).unwrap_or_default();

                return WslInfo {
                    installed: v["installed"].as_bool().unwrap_or(false),
                    default_version: v["defaultVersion"].as_u64().unwrap_or(2) as u32,
                    distros,
                    kernel_version: v["kernelVersion"].as_str().unwrap_or("").to_string(),
                    wsl_version: v["wslVersion"].as_str().unwrap_or("").to_string(),
                    error: v["error"].as_str().unwrap_or("").to_string(),
                };
            }
        }
    }
    WslInfo { installed: false, error: "Erreur lecture WSL".to_string(), ..Default::default() }
}

// Anti-freeze : wsl.exe est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn wsl_run_command(distro: String, command: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || wsl_run_command_blocking(distro, command))
        .await
        .map_err(|e| e.to_string())?
}

fn wsl_run_command_blocking(distro: String, command: String) -> Result<String, String> {
    let dist = distro.replace(['"', '\''], "");
    let cmd = command.trim().to_string();
    #[cfg(target_os = "windows")]
    {
        let mut args = vec![];
        if !dist.is_empty() {
            args.push("-d".to_string());
            args.push(dist);
        }
        // Pass via sh -c so pipes, quoted spaces, and shell operators work correctly
        args.push("--".to_string());
        args.push("sh".to_string());
        args.push("-c".to_string());
        args.push(cmd);
        let o = Command::new("wsl")
            .args(&args)
            // WSL_UTF8=1 : sans cette variable officielle, wsl.exe emet en
            // UTF-16LE quand sa sortie est redirigee (pas une console) — sans
            // elle, from_utf8_lossy produirait du texte illisible, pas juste
            // du mojibake sur les accents.
            .env("WSL_UTF8", "1")
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&o.stdout).to_string();
        let stderr = String::from_utf8_lossy(&o.stderr).to_string();
        if o.status.success() {
            return Ok(stdout);
        }
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
    #[cfg(not(target_os = "windows"))]
    Err("Non disponible".to_string())
}

// Anti-freeze : wsl.exe est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn wsl_set_default_version(version: u32) -> Result<String, String> {
    tokio::task::spawn_blocking(move || wsl_set_default_version_blocking(version))
        .await
        .map_err(|e| e.to_string())?
}

fn wsl_set_default_version_blocking(version: u32) -> Result<String, String> {
    let v = if version == 1 { 1u32 } else { 2u32 };
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("wsl")
            .args(["--set-default-version", &v.to_string()])
            .env("WSL_UTF8", "1")
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| e.to_string())?;
        let out = String::from_utf8_lossy(&o.stdout).to_string();
        if o.status.success() {
            return Ok(format!("Version WSL par défaut : {}", v));
        }
        Err(out)
    }
    #[cfg(not(target_os = "windows"))]
    Err("Non disponible".to_string())
}
