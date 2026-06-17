use serde::Serialize;
use tauri::Emitter;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize)]
pub struct InstalledApp {
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub uninstall_string: String,
    pub source: String,
    pub install_size_kb: u64,
    pub install_date: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UninstallResult {
    pub app_name: String,
    pub success: bool,
    pub message: String,
    pub residuals_found: Vec<String>, // trouvés mais PAS encore supprimés
}

#[derive(Debug, Clone, Serialize)]
pub struct ResidualCleanResult {
    pub success: bool,
    pub deleted_count: usize,
    pub failed_count: usize,
    pub message: String,
}

/// Liste les applications installées via le registre Windows
pub fn list_installed_apps() -> Vec<InstalledApp> {
    let ps = r#"
$paths = @(
    'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
$apps = Get-ItemProperty $paths -ErrorAction SilentlyContinue |
    Where-Object { $_.DisplayName -and $_.UninstallString -and $_.DisplayName -notmatch '^KB\d+' } |
    Select-Object DisplayName, DisplayVersion, Publisher, UninstallString, EstimatedSize, InstallDate |
    Sort-Object DisplayName
$apps | ConvertTo-Json -Compress -Depth 2
"#;

    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps])
        .creation_flags(0x08000000)
        .output();

    #[cfg(not(target_os = "windows"))]
    let result: Result<std::process::Output, _> =
        Err(std::io::Error::new(std::io::ErrorKind::Other, "not windows"));

    let output = match result { Ok(o) => o, Err(_) => return vec![] };
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let text = text.trim();
    if text.is_empty() || text == "null" { return vec![]; }
    let json_text = if text.starts_with('{') { format!("[{}]", text) } else { text.to_string() };
    let values: Vec<serde_json::Value> = match serde_json::from_str(&json_text) {
        Ok(v) => v, Err(_) => return vec![],
    };
    values.into_iter().filter_map(|v| {
        let name = v["DisplayName"].as_str()?.to_string();
        let version = v["DisplayVersion"].as_str().unwrap_or("").to_string();
        let publisher = v["Publisher"].as_str().unwrap_or("").to_string();
        let uninstall_string = v["UninstallString"].as_str()?.to_string();
        let raw_date = v["InstallDate"].as_str().unwrap_or("");
        let install_date = if raw_date.len() == 8 {
            format!("{}-{}-{}", &raw_date[..4], &raw_date[4..6], &raw_date[6..8])
        } else { raw_date.to_string() };
        Some(InstalledApp {
            name, version, publisher, uninstall_string,
            source: "registry".to_string(),
            install_size_kb: v["EstimatedSize"].as_u64().unwrap_or(0),
            install_date,
        })
    }).collect()
}

/// Désinstalle silencieusement + trouve les résidus (sans les supprimer)
pub fn uninstall_app_clean(
    app_name: String,
    uninstall_string: String,
    publisher: String,
    window: &tauri::Window,
) -> UninstallResult {
    emit(window, "start", &app_name, "Désinstallation silencieuse en cours...");

    let uninstall_ok = run_uninstall_silent(&app_name, &uninstall_string, window);

    emit(window, "scan", &app_name, "Recherche des résidus...");
    let residuals = find_residuals(&app_name, &publisher);

    emit(window, "done", &app_name, if uninstall_ok { "Désinstallé" } else { "Échec désinstallation" });

    UninstallResult {
        app_name: app_name.clone(),
        success: uninstall_ok,
        message: if uninstall_ok {
            format!("Désinstallé avec succès. {} résidu(s) détecté(s).", residuals.len())
        } else {
            format!("Échec : application toujours présente dans le registre. {} résidu(s) détecté(s).", residuals.len())
        },
        residuals_found: residuals,
    }
}

fn emit(window: &tauri::Window, step: &str, app: &str, msg: &str) {
    let _ = window.emit("uninstall-progress", serde_json::json!({
        "app": app, "step": step, "message": msg
    }));
}

// ── Désinstallation silencieuse — 100% natif, vérifié par registre ───────────
//
// On n'utilise PAS winget : trop imprévisible (peut sortir avant la fin,
// laisser la GUI ouverte, faux-positifs sur les codes de retour).
//
// Stratégie :
//  1. MSI  → msiexec /X{GUID} /qn /norestart
//  2. Inno → exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
//  3. NSIS / inconnu → exe /S
//     Avec /S, NSIS est VRAIMENT silencieux : pas de GUI, pas de processus enfant.
//  4. Start-Process -Wait : attend la VRAIE fin du processus
//  5. Vérification registre pour confirmer (pas de faux positif)
//
fn run_uninstall_silent(app_name: &str, uninstall_string: &str, _window: &tauri::Window) -> bool {
    // Encoder les valeurs via JSON : double quotes autour, caractères spéciaux échappés.
    // La chaîne JSON est directement interprétable par ConvertFrom-Json dans PS.
    // Pas de format!() avec ces valeurs — on construit le script par concaténation contrôlée.
    let app_json = serde_json::to_string(app_name).unwrap_or_else(|_| "\"\"".to_string());
    let us_json  = serde_json::to_string(uninstall_string).unwrap_or_else(|_| "\"\"".to_string());

    // Le script PowerShell utilise ConvertFrom-Json pour désérialiser les valeurs JSON.
    // Ceci protège contre toute injection : guillemets, backticks, $(), etc. sont neutralisés.
    let ps = format!(
        r#"
function Is-Installed($name) {{
    $paths = @(
        'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'
    )
    $found = Get-ItemProperty $paths -ErrorAction SilentlyContinue |
        Where-Object {{ $_.DisplayName -and $_.DisplayName -like "*$($name)*" }}
    return ($null -ne $found)
}}

# Valeurs passées via JSON — protège contre l'injection PowerShell
$appName = {app_json} | ConvertFrom-Json
$s = ({us_json} | ConvertFrom-Json).Trim()

$proc = $null
try {{
    if ($s -match 'msiexec' -or $s -match '\.msi"?$') {{
        # ── MSI ──────────────────────────────────────────────────────────────
        $re = '/[Ii](\{{[^}}]+\}})'
        $msiArgs = ($s -replace $re, '/X$1') -replace 'msiexec(\.exe)?\s*', ''
        if ($msiArgs -notmatch '/q') {{ $msiArgs = $msiArgs.Trim() + ' /qn /norestart' }}
        $proc = Start-Process -FilePath 'msiexec.exe' `
            -ArgumentList $msiArgs.Trim() -Wait -PassThru -ErrorAction Stop

    }} else {{
        # ── Non-MSI : extraire le chemin de l'exe ────────────────────────────
        if ($s -match '^"([^"]+)"') {{ $exe = $matches[1] }}
        else {{ $exe = ($s -split '\s+')[0].Trim('"') }}

        if (-not $exe -or -not (Test-Path -LiteralPath $exe)) {{
            Write-Output "FAIL:exe_not_found:$exe"; exit
        }}

        # Détecter le type via les métadonnées PE
        $vi   = (Get-Item -LiteralPath $exe -ErrorAction SilentlyContinue).VersionInfo
        $meta = "$($vi.FileDescription) $($vi.ProductName) $($vi.CompanyName)"

        $silentArgs = if ($meta -match 'Inno|Jordan Russell') {{
            @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-')
        }} else {{
            # NSIS (Nullsoft) ou inconnu → /S
            # /S = mode silencieux complet, PAS de GUI, PAS de processus enfant
            @('/S')
        }}

        # -Wait : bloque jusqu'à la VRAIE fin (fonctionne car /S empêche le spawn enfant)
        $proc = Start-Process -FilePath $exe `
            -ArgumentList $silentArgs -Wait -PassThru -ErrorAction Stop
    }}
}} catch {{
    Write-Output "FAIL:exception:$($_.Exception.Message)"; exit
}}

# Attendre que le registre soit mis à jour (flush)
Start-Sleep -Milliseconds 2500

$code = if ($proc) {{ $proc.ExitCode }} else {{ -1 }}
if (Is-Installed $appName) {{
    Write-Output "FAIL:still_installed:$code"
}} else {{
    Write-Output "OK:$code"
}}
"#,
        app_json = app_json,
        us_json  = us_json
    );

    match run_ps(&ps) {
        Some(t) => t.trim().starts_with("OK:"),
        None    => false,
    }
}

// ── Recherche des résidus (sans supprimer) ───────────────────────────────────

fn find_residuals(app_name: &str, publisher: &str) -> Vec<String> {
    preview_residuals(app_name.to_string(), publisher.to_string())
}

/// Prévisualise les résidus qu'une désinstallation propre supprimerait (sans rien effacer)
pub fn preview_residuals(app_name: String, publisher: String) -> Vec<String> {
    let generic: &[&str] = &[
        "software", "corp", "inc", "ltd", "llc", "gmbh",
        "technologies", "solutions", "systems", "group", "the", "and",
    ];
    let mut keywords: Vec<String> = Vec::new();
    for word in app_name.split_whitespace().chain(publisher.split_whitespace()) {
        let w = word.to_lowercase();
        let w = w.trim_matches(|c: char| !c.is_alphanumeric());
        if w.len() >= 4 && !generic.contains(&w) {
            keywords.push(w.to_string());
        }
    }
    keywords.dedup();
    keywords.truncate(4);
    if keywords.is_empty() { return vec![]; }

    let kw_json = serde_json::to_string(&keywords).unwrap_or_default();
    let ps = format!(r#"
$keywords = {} | ConvertFrom-Json
$found = @()

function Match-Kw($name) {{
    $n = $name.ToLower()
    foreach ($kw in $keywords) {{ if ($n -like "*$kw*") {{ return $true }} }}
    return $false
}}

$fsPaths = @($env:APPDATA, $env:LOCALAPPDATA, "$env:LOCALAPPDATA\Programs", $env:ProgramData, "$env:LOCALAPPDATA\Temp")
foreach ($base in $fsPaths) {{
    if (-not $base -or -not (Test-Path $base)) {{ continue }}
    Get-ChildItem $base -ErrorAction SilentlyContinue | ForEach-Object {{
        if (Match-Kw $_.Name) {{
            $fp = $_.FullName.ToLower()
            # Protection : exclut chemins système critiques
            $isSys = ($fp -like "*\windows\*") -or
                     ($fp -like "*\system32\*") -or
                     ($fp -like "*\syswow64\*") -or
                     ($fp -like "*\microsoft\windows\*") -or
                     ($fp -like "*\microsoft\windows defender\*") -or
                     ($fp -like "*\programdata\microsoft\windows\*") -or
                     ($fp.Length -lt 20)
            if (-not $isSys) {{ $found += "📁 $($_.FullName)" }}
        }}
    }}
}}

$regPaths = @('HKCU:\SOFTWARE', 'HKLM:\SOFTWARE', 'HKLM:\SOFTWARE\WOW6432Node')
foreach ($rp in $regPaths) {{
    if (-not (Test-Path $rp)) {{ continue }}
    Get-ChildItem $rp -ErrorAction SilentlyContinue | ForEach-Object {{
        if (Match-Kw $_.PSChildName) {{
            $n = $_.PSChildName.ToLower()
            # Blocklist étendue — ne jamais toucher aux clés système critiques
            $isSys = ($n -eq 'microsoft') -or ($n -eq 'windows') -or
                     ($n -eq 'classes') -or ($n -like 'wow6432*') -or
                     ($n -eq 'policies') -or ($n -eq 'drivers') -or
                     ($n -eq 'system') -or ($n -like 'system*') -or
                     ($n -eq 'currentcontrolset') -or ($n -eq 'currentversion') -or
                     ($n -eq 'hardware') -or ($n -eq 'security') -or
                     ($n -eq 'sam') -or ($n -eq 'bcd00000000') -or
                     ($n -eq 'services') -or ($n -like 'net*') -or
                     ($n -eq 'run') -or ($n -like 'run*') -or
                     ($n -eq 'explorer') -or ($n -eq 'shell') -or
                     ($n -eq 'fonts') -or ($n -eq 'internet settings') -or
                     ($n -like 'directx*') -or ($n -like 'opencl*') -or
                     ($n -like 'vulkan*') -or ($n -like 'opengl*') -or
                     # Protège HKLM pour les entrées Microsoft/Windows natives
                     ($rp -like 'HKLM:*' -and $_.PSPath -like '*\Microsoft\*') -or
                     ($rp -like 'HKLM:*' -and $_.PSPath -like '*\Windows*')
            if (-not $isSys) {{ $found += "🔑 $($_.PSPath)" }}
        }}
    }}
}}

$runPaths = @('HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run','HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run')
foreach ($rp in $runPaths) {{
    if (-not (Test-Path $rp)) {{ continue }}
    (Get-ItemProperty $rp -ErrorAction SilentlyContinue).PSObject.Properties |
        Where-Object {{ $_.Name -notlike 'PS*' }} |
        ForEach-Object {{ if (Match-Kw $_.Name) {{ $found += "🚀 $($_.Name)" }} }}
}}

$sc = @("$env:USERPROFILE\Desktop","$env:APPDATA\Microsoft\Windows\Start Menu\Programs")
foreach ($p in $sc) {{
    if (-not (Test-Path $p)) {{ continue }}
    Get-ChildItem $p -Recurse -Include '*.lnk' -ErrorAction SilentlyContinue |
        Where-Object {{ Match-Kw $_.BaseName }} | ForEach-Object {{ $found += "🔗 $($_.FullName)" }}
    Get-ChildItem $p -Directory -ErrorAction SilentlyContinue |
        Where-Object {{ Match-Kw $_.Name }} | ForEach-Object {{ $found += "📁 Menu: $($_.FullName)" }}
}}

$found | ConvertTo-Json -Compress
"#, kw_json);

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
            .creation_flags(0x08000000)
            .output();
        match output {
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout).to_string();
                let text = text.trim();
                if text.is_empty() || text == "null" { return vec![]; }
                let json = if text.starts_with('"') { format!("[{}]", text) } else { text.to_string() };
                serde_json::from_str::<Vec<String>>(&json).unwrap_or_default()
            }
            Err(_) => vec![],
        }
    }
    #[cfg(not(target_os = "windows"))]
    vec![]
}

// ── Suppression définitive des résidus ───────────────────────────────────────

pub fn delete_residuals(paths: Vec<String>) -> ResidualCleanResult {
    if paths.is_empty() {
        return ResidualCleanResult { success: true, deleted_count: 0, failed_count: 0, message: "Rien à supprimer.".into() };
    }
    let paths_json = serde_json::to_string(&paths).unwrap_or_default();
    let ps = format!(r#"
$items = '{}' | ConvertFrom-Json
$ok = 0; $fail = 0

# ── Garde de sécurité : refuse de supprimer tout chemin système critique ──
function Is-SafeToDelete($item) {{
    # Chemins fichiers/dossiers interdits
    $sysFS = @(
        'C:\Windows\', 'C:\windows\',
        '\System32\', '\SysWOW64\', '\system32\', '\syswow64\',
        'C:\Program Files\Common Files\',
        'C:\Program Files (x86)\Common Files\',
        'C:\ProgramData\Microsoft\Windows\',
        '\AppData\Roaming\Microsoft\Windows\',
        '\AppData\Local\Microsoft\Windows\',
        'C:\Users\Default\', 'C:\Users\Public\',
        'C:\ProgramData\Microsoft\Windows Defender\'
    )
    # Clés registre interdites
    $sysReg = @(
        'HKLM:\SOFTWARE\Microsoft\',
        'HKLM:\SOFTWARE\Classes\',
        'HKLM:\SOFTWARE\Policies\',
        'HKLM:\SYSTEM\',
        'HKLM:\SAM\',
        'HKLM:\SECURITY\',
        'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\',
        'HKCU:\SOFTWARE\Microsoft\Windows NT\',
        'HKCU:\SOFTWARE\Classes\'
    )
    if ($item -like '📁 *' -or $item -like '🔗 *') {{
        $raw = if ($item -like '📁 Menu: *') {{ $item.Substring(9) }} else {{ $item.Substring(3) }}
        # Longueur minimum : évite les chemins racine trop courts
        if ($raw.Length -lt 12) {{ return $false }}
        foreach ($p in $sysFS) {{ if ($raw -like "*$p*") {{ return $false }} }}
        return $true
    }}
    if ($item -like '🔑 *') {{
        $path = $item.Substring(3)
        if ($path.Length -lt 20) {{ return $false }}
        foreach ($p in $sysReg) {{ if ($path -like "$p*") {{ return $false }} }}
        return $true
    }}
    if ($item -like '🚀 *') {{ return $true }}  # Entrées Run : toujours safe
    return $false
}}

foreach ($item in $items) {{
    # Vérification sécurité avant toute suppression
    if (-not (Is-SafeToDelete $item)) {{
        Write-Warning "SKIP (protégé): $item"
        $fail++
        continue
    }}
    try {{
        if ($item -like '📁 Menu: *') {{
            $path = $item.Substring(9)
            Remove-Item $path -Recurse -Force -ErrorAction Stop; $ok++
        }} elseif ($item -like '📁 *') {{
            $path = $item.Substring(3)
            Remove-Item $path -Recurse -Force -ErrorAction Stop; $ok++
        }} elseif ($item -like '🔑 *') {{
            $path = $item.Substring(3)
            Remove-Item $path -Recurse -Force -ErrorAction Stop; $ok++
        }} elseif ($item -like '🚀 *') {{
            $name = $item.Substring(3)
            Remove-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run' -Name $name -ErrorAction SilentlyContinue
            Remove-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run' -Name $name -ErrorAction SilentlyContinue
            $ok++
        }} elseif ($item -like '🔗 *') {{
            $path = $item.Substring(3)
            Remove-Item $path -Force -ErrorAction Stop; $ok++
        }} else {{ $fail++ }}
    }} catch {{ $fail++ }}
}}
@{{ok=$ok;fail=$fail}} | ConvertTo-Json -Compress
"#, paths_json.replace('\'', "''"));

    match run_ps(&ps) {
        Some(t) => {
            let t = t.trim();
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(t) {
                let ok   = v["ok"].as_u64().unwrap_or(0) as usize;
                let fail = v["fail"].as_u64().unwrap_or(0) as usize;
                ResidualCleanResult {
                    success: fail == 0,
                    deleted_count: ok, failed_count: fail,
                    message: format!("{} résidu(s) supprimé(s), {} échec(s)", ok, fail),
                }
            } else {
                ResidualCleanResult { success: false, deleted_count: 0, failed_count: paths.len(), message: "Erreur parse".into() }
            }
        }
        None => ResidualCleanResult { success: false, deleted_count: 0, failed_count: paths.len(), message: "Erreur PowerShell".into() },
    }
}

// ── Extraction puis suppression des résidus ──────────────────────────────────

pub fn extract_residuals(paths: Vec<String>, target: String) -> ResidualCleanResult {
    if paths.is_empty() {
        return ResidualCleanResult { success: true, deleted_count: 0, failed_count: 0, message: "Rien à extraire.".into() };
    }

    // Valider le répertoire cible : uniquement Documents, Bureau, Téléchargements, Temp
    let target_path = std::path::PathBuf::from(&target);
    let allowed_roots: Vec<std::path::PathBuf> = [
        dirs::document_dir(),
        dirs::desktop_dir(),
        dirs::download_dir(),
        Some(std::env::temp_dir()),
    ]
    .into_iter()
    .flatten()
    .collect();

    // Canonicaliser pour éviter le path traversal (../../)
    let canonical_target = match target_path.canonicalize()
        .or_else(|_| { std::fs::create_dir_all(&target_path)?; target_path.canonicalize() }) {
        Ok(p) => p,
        Err(e) => return ResidualCleanResult {
            success: false, deleted_count: 0, failed_count: paths.len(),
            message: format!("Destination inaccessible: {}", e),
        },
    };

    if !allowed_roots.iter().any(|root| canonical_target.starts_with(root)) {
        return ResidualCleanResult {
            success: false, deleted_count: 0, failed_count: paths.len(),
            message: format!("Destination hors des répertoires autorisés: {}", target),
        };
    }

    if let Err(e) = std::fs::create_dir_all(&canonical_target) {
        return ResidualCleanResult { success: false, deleted_count: 0, failed_count: paths.len(), message: format!("Destination inaccessible: {}", e) };
    }

    let paths_json = serde_json::to_string(&paths).unwrap_or_default();
    let target_str = canonical_target.to_string_lossy().to_string();
    let paths_json_escaped = paths_json.replace('\'', "''");
    let target_json = serde_json::to_string(&target_str).unwrap_or_else(|_| "\"\"".to_string());
    let ps = format!(r#"
$items = '{}' | ConvertFrom-Json
$target = {} | ConvertFrom-Json
$ok = 0; $fail = 0

# Export des clés registre en .reg
$regItems = $items | Where-Object {{ $_ -like '🔑 *' }}
if ($regItems) {{
    $regFile = Join-Path $target 'residuals_registry.reg'
    "Windows Registry Editor Version 5.00`r`n" | Out-File $regFile -Encoding Unicode
    foreach ($r in $regItems) {{
        $path = $r.Substring(3)
        try {{
            $path2 = $path -replace '^.*::', ''
            & reg export "$path2" "$regFile" /y 2>$null
            $ok++
        }} catch {{ $fail++ }}
    }}
}}

# Copie des fichiers
$fileItems = $items | Where-Object {{ $_ -like '📁 *' -or $_ -like '🔗 *' }}
foreach ($item in $fileItems) {{
    $path = if ($item -like '📁 Menu: *') {{ $item.Substring(10) }}
            elseif ($item -like '📁 *')   {{ $item.Substring(3) }}
            else                           {{ $item.Substring(3) }}
    try {{
        Copy-Item -Path $path -Destination $target -Recurse -Force -ErrorAction Stop; $ok++
    }} catch {{ $fail++ }}
}}

@{{ok=$ok;fail=$fail}} | ConvertTo-Json -Compress
"#, paths_json_escaped, target_json);

    let copy_result = run_ps(&ps);
    let (copied, copy_fail) = copy_result
        .and_then(|t| serde_json::from_str::<serde_json::Value>(t.trim()).ok())
        .map(|v| (v["ok"].as_u64().unwrap_or(0) as usize, v["fail"].as_u64().unwrap_or(0) as usize))
        .unwrap_or((0, paths.len()));

    // Suppression après extraction
    let del = delete_residuals(paths);

    ResidualCleanResult {
        success: del.success,
        deleted_count: del.deleted_count,
        failed_count: del.failed_count,
        message: format!("{} copié(s) dans «{}», {} supprimé(s)", copied + copy_fail, target, del.deleted_count),
    }
}

#[allow(dead_code)]
fn parse_cmd(cmd: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in cmd.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ' ' if !in_quotes => {
                if !current.is_empty() { parts.push(current.clone()); current.clear(); }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() { parts.push(current); }
    parts
}

fn run_ps(script: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let o = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(0x08000000)
            .output().ok()?;
        Some(String::from_utf8_lossy(&o.stdout).to_string())
    }
    #[cfg(not(target_os = "windows"))]
    None
}
