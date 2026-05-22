use serde::Serialize;
use tauri::Emitter;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use super::run_ps;
use super::shadow_vss::BatchRestoreResult;

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub letter: String,
    pub label: String,
    pub total_gb: f64,
    pub free_gb: f64,
    pub disk_type: String,
    pub file_system: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskEntry {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified: String,
    pub is_dir: bool,
}

/// Liste les lecteurs connectés (fixe, amovible, réseau)
pub fn list_connected_disks() -> Vec<DiskInfo> {
    let ps = r#"
try {
    Get-WmiObject Win32_LogicalDisk -ErrorAction SilentlyContinue | ForEach-Object {
        @{
            letter      = $_.DeviceID
            label       = if ($_.VolumeName) { $_.VolumeName } else { '' }
            total_gb    = if ($_.Size) { [math]::Round($_.Size / 1GB, 1) } else { 0 }
            free_gb     = if ($_.FreeSpace) { [math]::Round($_.FreeSpace / 1GB, 1) } else { 0 }
            disk_type   = switch ($_.DriveType) {
                2 { 'Amovible' } 3 { 'Fixe' } 4 { 'Réseau' } 5 { 'CD/DVD' } default { 'Inconnu' }
            }
            file_system = if ($_.FileSystem) { $_.FileSystem } else { 'NTFS' }
        }
    } | ConvertTo-Json -Compress -Depth 2
} catch { Write-Output '[]' }
"#;
    run_ps(ps)
        .and_then(|t| {
            let t = t.trim();
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().filter_map(|v| {
            Some(DiskInfo {
                letter:      v["letter"].as_str()?.to_string(),
                label:       v["label"].as_str().unwrap_or("").to_string(),
                total_gb:    v["total_gb"].as_f64().unwrap_or(0.0),
                free_gb:     v["free_gb"].as_f64().unwrap_or(0.0),
                disk_type:   v["disk_type"].as_str().unwrap_or("Fixe").to_string(),
                file_system: v["file_system"].as_str().unwrap_or("NTFS").to_string(),
            })
        }).collect())
        .unwrap_or_default()
}

/// Liste le contenu d'un chemin disque
pub fn browse_disk_path(path: String) -> Vec<DiskEntry> {
    let ps = format!(r#"
$p = '{}'
try {{
    $items = @()
    Get-ChildItem -LiteralPath $p -Force -ErrorAction SilentlyContinue | Select-Object -First 2000 | ForEach-Object {{
        try {{
            $items += @{{
                name       = $_.Name
                path       = $_.FullName
                size_bytes = if ($_.PSIsContainer) {{ 0 }} else {{ try {{ $_.Length }} catch {{ 0 }} }}
                modified   = try {{ $_.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss') }} catch {{ '' }}
                is_dir     = $_.PSIsContainer
            }}
        }} catch {{}}
    }}
    if ($items.Count -eq 0) {{ Write-Output '[]' }}
    elseif ($items.Count -eq 1) {{ Write-Output "[$($items | ConvertTo-Json -Compress -Depth 2)]" }}
    else {{ $items | ConvertTo-Json -Compress -Depth 2 }}
}} catch {{ Write-Output '[]' }}
"#, path.replace('\'', "''"));

    run_ps(&ps)
        .and_then(|t| {
            let t = t.trim();
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().filter_map(|v| {
            Some(DiskEntry {
                name:       v["name"].as_str()?.to_string(),
                path:       v["path"].as_str().unwrap_or("").to_string(),
                size_bytes: v["size_bytes"].as_u64().unwrap_or(0),
                modified:   v["modified"].as_str().unwrap_or("").to_string(),
                is_dir:     v["is_dir"].as_bool().unwrap_or(false),
            })
        }).collect())
        .unwrap_or_default()
}

/// Ouvre un dossier dans l'Explorateur Windows
pub fn open_in_explorer(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    { let _ = path; Err("Non supporté hors Windows".into()) }
}

/// Récupère des fichiers/dossiers depuis un disque vers une destination.
/// safe_mode=true : lecture par chunks 64 KB (style ddrescue).
pub fn recover_files_safe(
    files: Vec<String>,
    target_folder: String,
    safe_mode: bool,
    window: &tauri::Window,
) -> BatchRestoreResult {
    use std::path::Path;
    let total = files.len();
    if total == 0 {
        return BatchRestoreResult { success: false, restored_count: 0, failed_count: 0, message: "Aucun fichier sélectionné".into() };
    }
    if let Err(e) = std::fs::create_dir_all(&target_folder) {
        return BatchRestoreResult { success: false, restored_count: 0, failed_count: total, message: format!("Impossible de créer la destination: {}", e) };
    }
    let mut ok = 0usize;
    let mut fail = 0usize;
    let mut notices: Vec<String> = Vec::new();

    for (i, src) in files.iter().enumerate() {
        let name = Path::new(src).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let pct = ((i as f32 / total as f32) * 90.0) as u32 + 5;
        let _ = window.emit("recover-disk-progress", serde_json::json!({
            "percent": pct,
            "message": format!("({}/{}) {}", i + 1, total, name)
        }));

        let is_dir = Path::new(src).is_dir();
        let dst = format!("{}\\{}", target_folder.trim_end_matches('\\'), name);

        if is_dir {
            #[cfg(target_os = "windows")]
            {
                let (retry, wait) = if safe_mode { ("0", "0") } else { ("3", "5") };
                let status = std::process::Command::new("robocopy")
                    .args([src.as_str(), &dst, "/E", &format!("/R:{retry}"), &format!("/W:{wait}"),
                           "/B", "/COPYALL", "/NP", "/NFL", "/NDL", "/NJH", "/NJS"])
                    .creation_flags(0x08000000).status();
                match status {
                    Ok(s) if s.code().map(|c| c < 8).unwrap_or(false) => { ok += 1; }
                    Ok(s) => {
                        let code = s.code().unwrap_or(-1);
                        if code == 8 { ok += 1; notices.push(format!("{}: quelques fichiers inaccessibles ignorés", name)); }
                        else { fail += 1; notices.push(format!("{}: robocopy code {}", name, code)); }
                    }
                    Err(e) => { fail += 1; notices.push(format!("{}: {}", name, e)); }
                }
            }
            #[cfg(not(target_os = "windows"))]
            { fail += 1; notices.push(format!("{}: plateforme non supportée", name)); }
        } else if safe_mode {
            // Mode ddrescue : lecture par chunks 64 KB, zéros sur mauvais secteurs
            let ps = format!(r#"
$src = '{src}'
$dst = '{dst}'
try {{
    $dstDir = Split-Path $dst -Parent
    if (-not (Test-Path $dstDir)) {{ New-Item -ItemType Directory -Path $dstDir -Force | Out-Null }}
    $chunkSize = 65536
    $fs = [System.IO.File]::Open($src, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
    $fd = [System.IO.File]::Open($dst, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write, [System.IO.FileShare]::None)
    $buf   = New-Object byte[] $chunkSize
    $zeros = New-Object byte[] $chunkSize
    $recovered = [long]0
    $skipped   = [long]0
    while ($true) {{
        $read = 0
        try {{
            $read = $fs.Read($buf, 0, $chunkSize)
            if ($read -eq 0) {{ break }}
            $fd.Write($buf, 0, $read)
            $recovered += $read
        }} catch {{
            $fd.Write($zeros, 0, $chunkSize)
            $skipped += $chunkSize
            try {{ [void]$fs.Seek($chunkSize, [System.IO.SeekOrigin]::Current) }} catch {{}}
        }}
    }}
    $fs.Close(); $fd.Close()
    Write-Output "OK:$recovered`:$skipped"
}} catch {{
    try {{ if ($fs) {{ $fs.Close() }} }} catch {{}}
    try {{ if ($fd) {{ $fd.Close() }} }} catch {{}}
    Write-Output "ERR:$($_.Exception.Message)"
}}
"#,
                src = src.replace('\'', "''"),
                dst = dst.replace('\'', "''")
            );
            match run_ps(&ps) {
                Some(t) => {
                    let t = t.trim();
                    if let Some(rest) = t.strip_prefix("OK:") {
                        let parts: Vec<&str> = rest.splitn(2, ':').collect();
                        let skipped: u64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                        ok += 1;
                        if skipped > 0 {
                            notices.push(format!("{}: {} Ko de secteurs défaillants (remplacés par zéros)", name, skipped / 1024));
                        }
                    } else {
                        fail += 1;
                        notices.push(format!("{}: {}", name, t.trim_start_matches("ERR:")));
                    }
                }
                None => { fail += 1; notices.push(format!("{}: erreur PowerShell", name)); }
            }
        } else {
            // Mode normal : copie directe
            let ps = format!(r#"
try {{
    $dstDir = Split-Path '{}' -Parent
    if (-not (Test-Path $dstDir)) {{ New-Item -ItemType Directory -Path $dstDir -Force | Out-Null }}
    [System.IO.File]::Copy('{}', '{}', $true)
    Write-Output 'OK'
}} catch {{ Write-Output "ERR:$($_.Exception.Message)" }}
"#, dst.replace('\'', "''"), src.replace('\'', "''"), dst.replace('\'', "''"));
            match run_ps(&ps) {
                Some(t) if t.trim() == "OK" => { ok += 1; }
                Some(t) => { fail += 1; notices.push(format!("{}: {}", name, t.trim().trim_start_matches("ERR:"))); }
                None => { fail += 1; notices.push(format!("{}: erreur PowerShell", name)); }
            }
        }
    }
    let _ = window.emit("recover-disk-progress", serde_json::json!({
        "percent": 100,
        "message": format!("{}/{} éléments récupérés", ok, total)
    }));
    let msg = if notices.is_empty() {
        format!("{} élément(s) récupéré(s) avec succès dans «{}»", ok, target_folder)
    } else if fail == 0 {
        format!("{} récupéré(s) — {} avertissement(s): {}", ok, notices.len(), notices[..notices.len().min(3)].join("; "))
    } else {
        format!("{} récupéré(s), {} échec(s): {}", ok, fail, notices[..notices.len().min(3)].join("; "))
    };
    BatchRestoreResult { success: ok > 0, restored_count: ok, failed_count: fail, message: msg }
}
