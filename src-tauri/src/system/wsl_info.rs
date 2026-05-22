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


#[tauri::command]
pub fn get_wsl_info() -> WslInfo {
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

    $raw = & wsl --list --verbose 2>&1
    $lines = $raw | Where-Object { $_ -and $_.Trim() -ne '' } | Select-Object -Skip 1

    $distros = @($lines | ForEach-Object {
        $line = $_.TrimEnd()
        if ($line -match '^\*?\s+(\S+)\s+(\S+)\s+(\d+)') {
            $isDefault = $line.TrimStart().StartsWith('*')
            @{
                name    = $Matches[1]
                state   = $Matches[2]
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
            let t = String::from_utf8_lossy(&o.stdout);
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

#[tauri::command]
pub fn wsl_run_command(distro: String, command: String) -> Result<String, String> {
    let dist = distro.replace(['"', '\''], "");
    let cmd = command.replace('"', "'");
    #[cfg(target_os = "windows")]
    {
        let mut args = vec![];
        if !dist.is_empty() {
            args.push("-d".to_string());
            args.push(dist);
        }
        args.push("--".to_string());
        args.extend(cmd.split_whitespace().map(|s| s.to_string()));
        let o = Command::new("wsl")
            .args(&args)
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

#[tauri::command]
pub fn wsl_set_default_version(version: u32) -> Result<String, String> {
    let v = if version == 1 { 1u32 } else { 2u32 };
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("wsl")
            .args(["--set-default-version", &v.to_string()])
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
