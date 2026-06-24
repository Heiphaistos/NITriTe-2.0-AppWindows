use serde::Serialize;
use std::process::Command;
use std::path::Path;
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// ─── Windows Update driver status ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Default)]
pub struct WuDriverUpdate {
    pub title: String,
    pub driver_class: String,
    pub driver_manufacturer: String,
    pub driver_model: String,
    pub driver_version: String,
    pub size_mb: f64,
    pub update_id: String,
    pub is_downloaded: bool,
    pub is_mandatory: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct WuDriverSummary {
    pub pending_count: u32,
    pub total_size_mb: f64,
    pub updates: Vec<WuDriverUpdate>,
    pub wu_enabled: bool,
    pub last_check: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct HardwareDevice {
    pub name: String,
    pub device_id: String,
    pub hardware_ids: Vec<String>,
    pub compatible_ids: Vec<String>,
    pub manufacturer: String,
    pub class: String,
    pub driver_version: String,
    pub driver_date: String,
    pub status: String,
    pub config_error: u32,
    pub has_driver_problem: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DriverMatch {
    pub device_name: String,
    pub device_id: String,
    pub matched_hw_id: String,
    pub inf_path: String,
    pub inf_name: String,
    pub driver_provider: String,
    pub driver_version: String,
    pub driver_date: String,
    pub match_type: String, // "exact" | "compatible" | "partial"
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DriverScanResult {
    pub devices: Vec<HardwareDevice>,
    pub matches: Vec<DriverMatch>,
    pub inf_count: u32,
    pub scan_time_ms: u64,
    pub pack_folder: String,
    pub devices_with_match: u32,
    pub devices_with_problem: u32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DriverInstallResult {
    pub inf_path: String,
    pub success: bool,
    pub output: String,
    pub duration_secs: u64,
}

// ─── Récupération des périphériques hardware ───────────────────────────────────
#[tauri::command]
pub fn get_hardware_devices() -> Vec<HardwareDevice> {
    let ps = r#"
$devs = @(Get-WmiObject Win32_PnPEntity -ErrorAction SilentlyContinue | ForEach-Object {
    $d = $_
    $hwids = @()
    $compids = @()
    try { $hwids = @($d.HardwareID | Where-Object { $_ }) } catch {}
    try { $compids = @($d.CompatibleID | Where-Object { $_ }) } catch {}
    @{
        name    = [string]$d.Name
        id      = [string]$d.DeviceID
        hwids   = $hwids
        compids = $compids
        mfr     = [string]$d.Manufacturer
        class   = [string]$d.PNPClass
        drv_ver = ''
        drv_date= ''
        status  = [string]$d.Status
        err     = [int]$d.ConfigManagerErrorCode
    }
})
$devs | ConvertTo-Json -Depth 4 -Compress
"#;

    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output();

        if let Ok(o) = o {
            let text = String::from_utf8_lossy(&o.stdout);
            let t = text.trim();
            let arr_t = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&arr_t) {
                return arr.iter().map(|d| {
                    let hwids = d["hwids"].as_array().map(|a| {
                        a.iter().filter_map(|s| s.as_str().map(|x| x.to_uppercase())).collect::<Vec<_>>()
                    }).unwrap_or_default();
                    let compids = d["compids"].as_array().map(|a| {
                        a.iter().filter_map(|s| s.as_str().map(|x| x.to_uppercase())).collect::<Vec<_>>()
                    }).unwrap_or_default();
                    let err = d["err"].as_u64().unwrap_or(0) as u32;
                    HardwareDevice {
                        name: d["name"].as_str().unwrap_or("").to_string(),
                        device_id: d["id"].as_str().unwrap_or("").to_string(),
                        hardware_ids: hwids,
                        compatible_ids: compids,
                        manufacturer: d["mfr"].as_str().unwrap_or("").to_string(),
                        class: d["class"].as_str().unwrap_or("").to_string(),
                        driver_version: d["drv_ver"].as_str().unwrap_or("").to_string(),
                        driver_date: d["drv_date"].as_str().unwrap_or("").to_string(),
                        status: d["status"].as_str().unwrap_or("").to_string(),
                        config_error: err,
                        has_driver_problem: err != 0,
                    }
                }).collect();
            }
        }
    }
    vec![]
}

// ─── Parser INF — extrait les Hardware IDs ─────────────────────────────────────
fn extract_hw_ids_from_inf(content: &str) -> Vec<String> {
    let mut ids = Vec::new();
    // Common hardware ID prefixes in INF files
    let prefixes = [
        "PCI\\", "USB\\", "ACPI\\", "HDAUDIO\\", "HID\\", "SCSI\\",
        "IDE\\", "STORAGE\\", "DISPLAY\\", "MONITOR\\", "BLUETOOTH\\",
        "BTH\\", "SWD\\", "SD\\", "WBFP\\", "WSD\\", "ROOT\\",
        "*PNP", "ISAPNP\\", "MF\\", "BTHENUM\\", "USBVIDEO\\",
        "MEDIA\\", "NET\\", "PRINTER\\", "MODEM\\", "1394\\",
    ];

    for line in content.lines() {
        let line = line.trim();
        // Skip comments, section headers, empty lines
        if line.starts_with(';') || line.starts_with('[') || line.is_empty() {
            continue;
        }
        // Look for hardware ID patterns in lines with '='
        if line.contains('=') {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                // Split RHS by commas, check each token for hardware ID patterns
                for part in parts[1].split(',') {
                    let candidate = part.trim().trim_matches('"').trim_matches('%').to_uppercase();
                    if candidate.len() < 5 || candidate.len() > 200 { continue; }
                    if prefixes.iter().any(|p| candidate.starts_with(p))
                        || candidate.contains("VEN_")
                        || candidate.contains("VID_")
                        || candidate.contains("DEV_")
                        || candidate.contains("PID_")
                        || candidate.contains("SUBSYS_")
                        || (candidate.contains('\\') && !candidate.contains(' ') && candidate.len() > 8)
                    {
                        let clean = candidate.trim().to_string();
                        if !ids.contains(&clean) {
                            ids.push(clean);
                        }
                    }
                }
            }
        }
    }
    ids
}

// ─── Infos version depuis section [Version] du INF ────────────────────────────
fn parse_inf_meta(content: &str) -> (String, String, String) {
    let mut provider = String::new();
    let mut version = String::new();
    let mut date = String::new();
    let mut in_version = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            let sec = line.trim_matches(|c| c == '[' || c == ']').to_lowercase();
            in_version = sec == "version";
            continue;
        }
        if !in_version { continue; }
        let low = line.to_lowercase();
        if low.starts_with("provider") {
            provider = line.split_once('=').map(|x| x.1).unwrap_or("").trim().trim_matches('"').trim_matches('%').to_string();
        } else if low.starts_with("driverversion") || low.starts_with("driverver") {
            let rhs = line.split_once('=').map(|x| x.1).unwrap_or("").trim();
            // Format: MM/DD/YYYY,x.x.x.x
            let parts: Vec<&str> = rhs.splitn(2, ',').collect();
            date = parts.first().unwrap_or(&"").trim().to_string();
            version = parts.get(1).unwrap_or(&"").trim().to_string();
        }
    }
    (provider, version, date)
}

// ─── Scan récursif des INF files ───────────────────────────────────────────────
fn collect_inf_files(folder: &Path, max_files: usize) -> Vec<std::path::PathBuf> {
    const MAX_DEPTH: usize = 12;
    let mut result = Vec::new();
    // Stack contient (chemin, profondeur)
    let mut stack: Vec<(std::path::PathBuf, usize)> = vec![(folder.to_path_buf(), 0)];
    // Ensemble des chemins canoniques déjà visités — détecte les cycles via symlinks
    let mut visited: std::collections::HashSet<std::path::PathBuf> = std::collections::HashSet::new();
    if let Ok(canon) = folder.canonicalize() { visited.insert(canon); }
    while let Some((dir, depth)) = stack.pop() {
        if result.len() >= max_files || depth >= MAX_DEPTH { continue; }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if result.len() >= max_files { break; }
                let path = entry.path();
                if path.is_dir() {
                    // Canonicalise pour détecter les symlinks cycliques
                    match path.canonicalize() {
                        Ok(canon) => {
                            if visited.contains(&canon) { continue; } // cycle détecté
                            visited.insert(canon);
                        }
                        Err(_) => continue, // chemin inaccessible ou symlink cassé
                    }
                    stack.push((path, depth + 1));
                } else if let Some(ext) = path.extension() {
                    if ext.to_string_lossy().to_lowercase() == "inf" {
                        result.push(path);
                    }
                }
            }
        }
    }
    result
}

// ─── Moteur de matching principal ─────────────────────────────────────────────
#[tauri::command]
pub fn scan_driver_folder(folder_path: String, device_ids: Vec<String>) -> DriverScanResult {
    let start = std::time::Instant::now();
    let folder = Path::new(&folder_path);

    if !folder.exists() || !folder.is_dir() {
        return DriverScanResult { pack_folder: folder_path, ..Default::default() };
    }

    // Build index: hardware_id_uppercase -> Vec<(inf_path, provider, version, date)>
    let inf_files = collect_inf_files(folder, 50_000);
    let inf_count = inf_files.len() as u32;

    let mut index: HashMap<String, Vec<(String, String, String, String)>> = HashMap::new();

    for inf_path in &inf_files {
        let content = match std::fs::read_to_string(inf_path) {
            Ok(c) => c,
            Err(_) => {
                // Try windows-1252 encoding fallback
                if let Ok(bytes) = std::fs::read(inf_path) {
                    bytes.iter().map(|&b| b as char).collect()
                } else { continue; }
            }
        };
        let (provider, version, date) = parse_inf_meta(&content);
        let hw_ids = extract_hw_ids_from_inf(&content);
        let inf_str = inf_path.to_string_lossy().to_string();
        for hw_id in hw_ids {
            index.entry(hw_id).or_default().push((inf_str.clone(), provider.clone(), version.clone(), date.clone()));
        }
    }

    // Match device hardware IDs against index
    let mut matches = Vec::new();
    let device_ids_upper: Vec<String> = device_ids.iter().map(|s| s.to_uppercase()).collect();

    // Try to get names for device IDs (simple lookup)
    for dev_id in &device_ids_upper {
        // Try exact match first
        if let Some(entries) = index.get(dev_id.as_str()) {
            for (inf_path, provider, version, date) in entries {
                let inf_name = Path::new(inf_path).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
                matches.push(DriverMatch {
                    device_name: String::new(),
                    device_id: dev_id.clone(),
                    matched_hw_id: dev_id.clone(),
                    inf_path: inf_path.clone(),
                    inf_name,
                    driver_provider: provider.clone(),
                    driver_version: version.clone(),
                    driver_date: date.clone(),
                    match_type: "exact".to_string(),
                    score: 100,
                });
            }
        } else {
            // Try prefix match: strip REV_ and SUBSYS_ for broader match
            let stripped = strip_hw_id_rev(dev_id);
            if let Some(entries) = index.get(&stripped) {
                for (inf_path, provider, version, date) in entries {
                    let inf_name = Path::new(inf_path).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
                    matches.push(DriverMatch {
                        device_name: String::new(),
                        device_id: dev_id.clone(),
                        matched_hw_id: stripped.clone(),
                        inf_path: inf_path.clone(),
                        inf_name,
                        driver_provider: provider.clone(),
                        driver_version: version.clone(),
                        driver_date: date.clone(),
                        match_type: "compatible".to_string(),
                        score: 80,
                    });
                }
            }
        }
    }

    // Deduplicate matches per device_id (keep best score)
    matches.sort_by_key(|a| std::cmp::Reverse(a.score));
    matches.dedup_by(|a, b| a.device_id == b.device_id);

    let devices_with_match = matches.len() as u32;

    DriverScanResult {
        devices: vec![],
        matches,
        inf_count,
        scan_time_ms: start.elapsed().as_millis() as u64,
        pack_folder: folder_path,
        devices_with_match,
        devices_with_problem: 0,
    }
}

fn strip_hw_id_rev(hw_id: &str) -> String {
    // PCI\VEN_8086&DEV_1234&SUBSYS_5678&REV_01 -> PCI\VEN_8086&DEV_1234
    let parts: Vec<&str> = hw_id.split('&').collect();
    let filtered: Vec<&str> = parts.iter().take_while(|s| !s.starts_with("REV_") && !s.starts_with("SUBSYS_")).cloned().collect();
    // If nothing was stripped, return original; otherwise take VEN+DEV
    if filtered.len() >= 2 {
        filtered.join("&")
    } else {
        hw_id.to_string()
    }
}

// ─── Installation d'un driver via pnputil ─────────────────────────────────────
#[tauri::command]
pub fn install_driver(inf_path: String) -> DriverInstallResult {
    let inf_clean = inf_path.replace('"', "");
    if !Path::new(&inf_clean).exists() {
        return DriverInstallResult { inf_path: inf_clean, success: false, output: "Fichier INF introuvable".to_string(), ..Default::default() };
    }

    let start = std::time::Instant::now();
    #[cfg(target_os = "windows")]
    {
        // Appel direct à pnputil sans passer par cmd /C — élimine l'injection via cmd.exe
        let o = Command::new("pnputil")
            .args(["/add-driver", &inf_clean, "/install"])
            .creation_flags(0x08000000)
            .output();

        let duration = start.elapsed().as_secs();
        if let Ok(o) = o {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            let combined = if stderr.is_empty() { stdout } else { format!("{}\n{}", stdout, stderr) };
            return DriverInstallResult {
                inf_path: inf_clean,
                success: o.status.success() || combined.to_lowercase().contains("installed"),
                output: combined.chars().take(2000).collect(),
                duration_secs: duration,
            };
        }
    }
    DriverInstallResult { inf_path: inf_clean, ..Default::default() }
}

// ─── Recherche de MAJ pilotes via Windows Update (WUA COM) ────────────────────
#[tauri::command]
pub fn check_driver_updates_winupdate() -> WuDriverSummary {
    let ps = r#"
try {
    $sess = New-Object -ComObject Microsoft.Update.Session -EA Stop
    $searcher = $sess.CreateUpdateSearcher()
    $results = $searcher.Search("IsInstalled=0 and Type='Driver' and IsHidden=0")
    $updates = @($results.Updates | ForEach-Object {
        $u = $_
        $szMb = if($u.MaxDownloadSize -gt 0){[math]::Round($u.MaxDownloadSize/1MB,1)}else{0}
        @{
            title   = [string]$u.Title
            class   = [string]$u.DriverClass
            mfr     = [string]$u.DriverManufacturer
            model   = [string]$u.DriverModel
            ver     = [string]$u.DriverVersion
            mb      = $szMb
            id      = [string]$u.Identity.UpdateID
            dl      = [bool]$u.IsDownloaded
            mand    = [bool]$u.IsMandatory
        }
    })
    $total_mb = [math]::Round(($updates | Measure-Object -Property mb -Sum).Sum, 1)
    @{
        ok      = $true
        count   = [int]$updates.Count
        mb      = $total_mb
        updates = $updates
        last    = (Get-Date).ToString('yyyy-MM-dd HH:mm')
    } | ConvertTo-Json -Depth 4 -Compress
} catch {
    # Fallback: count devices with ConfigManagerErrorCode != 0
    $problematic = @(Get-WmiObject Win32_PnPSignedDriver -EA SilentlyContinue |
        Where-Object { $_.ConfigManagerErrorCode -ne 0 } |
        ForEach-Object { @{ title=[string]$_.DeviceName; class=[string]$_.DeviceClass; mfr=[string]$_.Manufacturer; model=''; ver=[string]$_.DriverVersion; mb=0; id=''; dl=$false; mand=$false } })
    @{ ok=$false; count=[int]$problematic.Count; mb=0; updates=$problematic; last='N/A' } | ConvertTo-Json -Depth 4 -Compress
}
"#;
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(t.trim()) {
                let updates = v["updates"].as_array().map(|arr| arr.iter().map(|u| WuDriverUpdate {
                    title: u["title"].as_str().unwrap_or("").to_string(),
                    driver_class: u["class"].as_str().unwrap_or("").to_string(),
                    driver_manufacturer: u["mfr"].as_str().unwrap_or("").to_string(),
                    driver_model: u["model"].as_str().unwrap_or("").to_string(),
                    driver_version: u["ver"].as_str().unwrap_or("").to_string(),
                    size_mb: u["mb"].as_f64().unwrap_or(0.0),
                    update_id: u["id"].as_str().unwrap_or("").to_string(),
                    is_downloaded: u["dl"].as_bool().unwrap_or(false),
                    is_mandatory: u["mand"].as_bool().unwrap_or(false),
                }).collect()).unwrap_or_default();
                return WuDriverSummary {
                    pending_count: v["count"].as_u64().unwrap_or(0) as u32,
                    total_size_mb: v["mb"].as_f64().unwrap_or(0.0),
                    updates,
                    wu_enabled: v["ok"].as_bool().unwrap_or(false),
                    last_check: v["last"].as_str().unwrap_or("").to_string(),
                };
            }
        }
    }
    WuDriverSummary::default()
}

// ─── Installer une mise à jour pilote via Windows Update ───────────────────────
#[tauri::command]
pub fn install_driver_winupdate(update_id: String) -> DriverInstallResult {
    // Uses Windows Update Agent COM to download + install specific update
    let uid = update_id.replace(['"', '\''], "");
    let ps = format!(r#"
try {{
    $sess = New-Object -ComObject Microsoft.Update.Session
    $searcher = $sess.CreateUpdateSearcher()
    $results = $searcher.Search("IsInstalled=0 and Type='Driver'")
    $target = $results.Updates | Where-Object {{ $_.Identity.UpdateID -eq '{uid}' }} | Select-Object -First 1
    if (-not $target) {{ throw "Update $env:uid not found" }}
    # Download
    $downloader = $sess.CreateUpdateDownloader()
    $coll = New-Object -ComObject Microsoft.Update.UpdateColl
    $coll.Add($target) | Out-Null
    $downloader.Updates = $coll
    $dlResult = $downloader.Download()
    # Install
    $installer = $sess.CreateUpdateInstaller()
    $installer.Updates = $coll
    $instResult = $installer.Install()
    "Installed: $($target.Title) — ResultCode: $($instResult.ResultCode)"
}} catch {{
    throw $_.Exception.Message
}}"#, uid=uid);
    let start = std::time::Instant::now();
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",&ps]).creation_flags(0x08000000).output();
        let dur = start.elapsed().as_secs();
        if let Ok(o) = o {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            let combined = if stderr.is_empty() { stdout } else { format!("{}\n{}", stdout, stderr) };
            return DriverInstallResult { inf_path: uid, success: o.status.success(), output: combined.chars().take(2000).collect(), duration_secs: dur };
        }
    }
    DriverInstallResult { inf_path: uid, ..Default::default() }
}

// ─── Installer TOUS les drivers en attente WU ─────────────────────────────────
#[tauri::command]
pub fn install_all_driver_updates() -> DriverInstallResult {
    let ps = r#"
try {
    $sess = New-Object -ComObject Microsoft.Update.Session
    $searcher = $sess.CreateUpdateSearcher()
    $results = $searcher.Search("IsInstalled=0 and Type='Driver' and IsHidden=0")
    if ($results.Updates.Count -eq 0) { "Aucune mise a jour de pilote en attente."; exit 0 }
    $coll = New-Object -ComObject Microsoft.Update.UpdateColl
    $results.Updates | ForEach-Object { $coll.Add($_) | Out-Null }
    $downloader = $sess.CreateUpdateDownloader()
    $downloader.Updates = $coll
    Write-Host "Telechargement de $($coll.Count) mise(s) a jour..."
    $downloader.Download() | Out-Null
    $installer = $sess.CreateUpdateInstaller()
    $installer.Updates = $coll
    Write-Host "Installation en cours..."
    $instResult = $installer.Install()
    "Installe $($coll.Count) pilote(s). Code: $($instResult.ResultCode). Reboot requis: $($instResult.RebootRequired)"
} catch {
    throw $_.Exception.Message
}"#;
    let start = std::time::Instant::now();
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",ps]).creation_flags(0x08000000).output();
        let dur = start.elapsed().as_secs();
        if let Ok(o) = o {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            let combined = if stderr.is_empty() { stdout } else { format!("{}\n{}", stdout, stderr) };
            return DriverInstallResult { inf_path: "Windows Update — All".to_string(), success: o.status.success(), output: combined.chars().take(3000).collect(), duration_secs: dur };
        }
    }
    DriverInstallResult::default()
}

// ─── Export liste hardware IDs pour debug ─────────────────────────────────────
#[tauri::command]
pub fn get_all_hardware_ids() -> Vec<String> {
    let ps = r#"
@(Get-WmiObject Win32_PnPEntity -EA SilentlyContinue | ForEach-Object {
    @($_.HardwareID | Where-Object { $_ })
} | ForEach-Object { $_ }) | Select-Object -Unique | ConvertTo-Json -Compress
"#;
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout); let t = t.trim();
            let arr_t = if t.starts_with('"') { format!("[{}]",t) } else { t.to_string() };
            if let Ok(arr) = serde_json::from_str::<Vec<String>>(&arr_t) {
                return arr;
            }
        }
    }
    vec![]
}
