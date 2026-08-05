use serde::Serialize;
use super::{run_ps, run_ps_with_args};

#[derive(Debug, Clone, Serialize)]
pub struct ShadowCopy {
    pub id: String,
    pub volume: String,
    pub creation_time: String,
    pub provider: String,
    pub device_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecoveredFile {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub deleted_date: String,
    pub source: String, // "recycle_bin" | "shadow_copy" | "mft"
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RestoreResult {
    pub success: bool,
    pub message: String,
    pub restored_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchRestoreResult {
    pub success: bool,
    pub restored_count: usize,
    pub failed_count: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComparedFile {
    pub name: String,
    pub shadow_path: String,
    pub live_path: String,
    pub status: String, // "deleted" | "modified" | "added"
    pub shadow_size: u64,
    pub live_size: u64,
    pub shadow_modified: String,
    pub live_modified: String,
}

/// Normalise un chemin shadow copy : device_path inclut déjà \\?\GLOBALROOT
pub fn shadow_path(device_path: &str, sub_path: &str) -> String {
    let base = device_path.trim_end_matches('\\');
    if sub_path.is_empty() {
        base.to_string()
    } else if sub_path.starts_with("\\\\?\\") || sub_path.starts_with("\\\\") {
        sub_path.trim_end_matches('\\').to_string()
    } else {
        format!("{}\\{}", base, sub_path.trim_matches('\\'))
    }
}

/// Liste les Shadow Copies (Points de restauration VSS)
pub fn list_shadow_copies() -> Vec<ShadowCopy> {
    let ps = r#"
try {
    $shadows = Get-WmiObject Win32_ShadowCopy -ErrorAction Stop
    $result = @()
    foreach ($s in $shadows) {
        $result += @{
            id            = $s.ID
            volume        = $s.VolumeName
            creation_time = $s.InstallDate
            provider      = $s.ProviderID
            device_path   = $s.DeviceObject
        }
    }
    $result | ConvertTo-Json -Compress -Depth 2
} catch { Write-Output '[]' }
"#;
    run_ps(ps)
        .and_then(|t| {
            let t = t.trim();
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().filter_map(|v| {
            Some(ShadowCopy {
                id: v["id"].as_str()?.to_string(),
                volume: v["volume"].as_str().unwrap_or("").to_string(),
                creation_time: v["creation_time"].as_str().unwrap_or("").to_string(),
                provider: v["provider"].as_str().unwrap_or("").to_string(),
                device_path: v["device_path"].as_str().unwrap_or("").to_string(),
            })
        }).collect())
        .unwrap_or_default()
}

/// Lister le contenu du dossier dans une Shadow Copy
pub fn browse_shadow_copy(device_path: String, relative_path: String) -> Vec<RecoveredFile> {
    let path = shadow_path(&device_path, &relative_path);
    // Chemin passé via $args[0] (arg processus séparé) — aucune interpolation PS possible
    let ps = r#"
$p = $args[0]
try {
    $items = @()
    Get-ChildItem -LiteralPath $p -Force -ErrorAction SilentlyContinue | Select-Object -First 2000 | ForEach-Object {
        try {
            $items += @{
                name       = $_.Name
                path       = $_.FullName
                size_bytes = if ($_.PSIsContainer) { 0 } else { try { $_.Length } catch { 0 } }
                is_dir     = $_.PSIsContainer
                modified   = try { $_.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss') } catch { '' }
            }
        } catch {}
    }
    if ($items.Count -eq 0) { Write-Output '[]' }
    elseif ($items.Count -eq 1) { Write-Output "[$($items | ConvertTo-Json -Compress -Depth 2)]" }
    else { $items | ConvertTo-Json -Compress -Depth 2 }
} catch { Write-Output '[]' }
"#;

    run_ps_with_args(ps, &[&path])
        .and_then(|t| {
            let t = t.trim();
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().filter_map(|v| {
            Some(RecoveredFile {
                name: v["name"].as_str()?.to_string(),
                path: v["path"].as_str().unwrap_or("").to_string(),
                size_bytes: v["size_bytes"].as_u64().unwrap_or(0),
                deleted_date: v["modified"].as_str().unwrap_or("").to_string(),
                source: "shadow_copy".to_string(),
                is_dir: v["is_dir"].as_bool().unwrap_or(false),
            })
        }).collect())
        .unwrap_or_default()
}

/// Chercher des fichiers dans une Shadow Copy (récursif, filtre par nom)
pub fn search_shadow_copy(device_path: String, query: String, base_path: String) -> Vec<RecoveredFile> {
    let shadow_root = shadow_path(&device_path, &base_path);
    // Chemin et requête passés via $args[0]/$args[1] — aucune interpolation PS possible
    let ps = r#"
$root = $args[0]
$q = $args[1]
try {
    $items = @()
    Get-ChildItem -LiteralPath $root -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -like "*$q*" } |
        Select-Object -First 500 |
        ForEach-Object {
            try {
                $items += @{
                    name       = $_.Name
                    path       = $_.FullName
                    size_bytes = if ($_.PSIsContainer) { 0 } else { try { $_.Length } catch { 0 } }
                    is_dir     = $_.PSIsContainer
                    modified   = try { $_.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss') } catch { '' }
                }
            } catch {}
        }
    if ($items.Count -eq 0) { Write-Output '[]' }
    elseif ($items.Count -eq 1) { Write-Output "[$($items | ConvertTo-Json -Compress -Depth 2)]" }
    else { $items | ConvertTo-Json -Compress -Depth 2 }
} catch { Write-Output '[]' }
"#;

    run_ps_with_args(ps, &[&shadow_root, &query])
        .and_then(|t| {
            let t = t.trim();
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().filter_map(|v| {
            Some(RecoveredFile {
                name: v["name"].as_str()?.to_string(),
                path: v["path"].as_str().unwrap_or("").to_string(),
                size_bytes: v["size_bytes"].as_u64().unwrap_or(0),
                deleted_date: v["modified"].as_str().unwrap_or("").to_string(),
                source: "shadow_copy".to_string(),
                is_dir: v["is_dir"].as_bool().unwrap_or(false),
            })
        }).collect())
        .unwrap_or_default()
}

pub fn restore_files_batch(files: Vec<String>, target_folder: String) -> BatchRestoreResult {
    if files.is_empty() {
        return BatchRestoreResult { success: false, restored_count: 0, failed_count: 0, message: "Aucun fichier sélectionné".into() };
    }
    let files_array = files.iter()
        .map(|f| format!("'{}'", f.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");
    let ps = format!(r#"
$target = '{}'
$files = @({})
if (-not (Test-Path $target)) {{ New-Item -ItemType Directory -Path $target -Force | Out-Null }}
$ok = 0; $fail = 0; $errs = @()
foreach ($src in $files) {{
    try {{
        Copy-Item -Path $src -Destination $target -Recurse -Force -ErrorAction Stop
        $ok++
    }} catch {{
        $fail++
        $errs += $_.Exception.Message
    }}
}}
@{{ ok=$ok; fail=$fail; errors=($errs | Select-Object -First 5) -join '|' }} | ConvertTo-Json -Compress
"#, target_folder.replace('\'', "''"), files_array);

    match run_ps(&ps) {
        Some(t) => {
            let t = t.trim();
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(t) {
                let ok = v["ok"].as_u64().unwrap_or(0) as usize;
                let fail = v["fail"].as_u64().unwrap_or(0) as usize;
                let errs = v["errors"].as_str().unwrap_or("").to_string();
                let msg = if fail == 0 {
                    format!("{} fichier(s) restauré(s) dans {}", ok, target_folder)
                } else {
                    format!("{} restauré(s), {} échec(s){}", ok, fail,
                        if errs.is_empty() { String::new() } else { format!(": {}", errs.chars().take(200).collect::<String>()) })
                };
                BatchRestoreResult { success: fail == 0 || ok > 0, restored_count: ok, failed_count: fail, message: msg }
            } else {
                BatchRestoreResult { success: false, restored_count: 0, failed_count: files.len(), message: "Erreur parse résultat".into() }
            }
        }
        None => BatchRestoreResult { success: false, restored_count: 0, failed_count: files.len(), message: "Erreur PowerShell".into() },
    }
}

/// Restaurer un fichier depuis une Shadow Copy vers un dossier cible
pub fn restore_from_shadow(source_path: String, target_folder: String) -> RestoreResult {
    let target = format!("{}\\", target_folder.trim_end_matches('\\'));
    let ps = format!(r#"
try {{
    $src = '{}'
    $dst = '{}'
    if (-not (Test-Path $dst)) {{ New-Item -ItemType Directory -Path $dst -Force | Out-Null }}
    Copy-Item -Path $src -Destination $dst -Recurse -Force -ErrorAction Stop
    Write-Output 'OK'
}} catch {{ Write-Output "ERR:$($_.Exception.Message)" }}
"#,
        source_path.replace('\'', "''"),
        target.replace('\'', "''")
    );

    match run_ps(&ps) {
        Some(t) if t.trim() == "OK" => RestoreResult {
            success: true,
            message: "Fichier restauré avec succès".into(),
            restored_path: target,
        },
        Some(t) => RestoreResult {
            success: false,
            message: t.trim().trim_start_matches("ERR:").to_string(),
            restored_path: String::new(),
        },
        None => RestoreResult {
            success: false,
            message: "Impossible de lancer PowerShell".into(),
            restored_path: String::new(),
        },
    }
}

/// Scanner la Corbeille
pub fn scan_recycle_bin() -> Vec<RecoveredFile> {
    let ps = r#"
try {
    $shell = New-Object -ComObject Shell.Application
    $recycle = $shell.Namespace(0xA)
    $items = @()
    foreach ($item in $recycle.Items()) {
        $items += @{
            name       = $item.Name
            path       = $item.Path
            size_bytes = $item.Size
            deleted    = $item.ExtendedProperty("System.Recycle.DateDeleted")
        }
    }
    $items | ConvertTo-Json -Compress -Depth 2
} catch { Write-Output '[]' }
"#;
    run_ps(ps)
        .and_then(|t| {
            let t = t.trim();
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().filter_map(|v| {
            Some(RecoveredFile {
                name: v["name"].as_str()?.to_string(),
                path: v["path"].as_str().unwrap_or("").to_string(),
                size_bytes: v["size_bytes"].as_u64().unwrap_or(0),
                deleted_date: v["deleted"].as_str().unwrap_or("").to_string(),
                source: "recycle_bin".to_string(),
                is_dir: false,
            })
        }).collect())
        .unwrap_or_default()
}

/// Restaurer un élément de la Corbeille
///
/// N'utilise PAS `$item.InvokeVerb('Restore')` : confirmé en exécution réelle
/// (2026-08-05) que ce verbe COM ne fait RIEN quand il est invoqué depuis un
/// powershell.exe non-interactif (exactement le contexte de `run_ps` ici) — le
/// fichier reste dans la Corbeille, `InvokeVerb` ne renvoie aucune valeur
/// permettant de le détecter, et l'ancien code écrivait 'OK' sans jamais
/// vérifier — la fonctionnalité de restauration ne fonctionnait donc jamais.
/// Restauration réelle via `Move-Item` vers le vrai chemin d'origine, obtenu
/// via `System.Recycle.DeletedFrom` (même mécanisme `ExtendedProperty` déjà
/// utilisé par `scan_recycle_bin` pour la date de suppression) — vérifié en
/// direct : le fichier réapparaît bien à son emplacement d'origine.
pub fn restore_recycle_bin_item(original_path: String) -> RestoreResult {
    let ps = format!(r#"
try {{
    $shell = New-Object -ComObject Shell.Application
    $recycle = $shell.Namespace(0xA)
    $target = $null
    foreach ($item in $recycle.Items()) {{
        if ($item.Path -eq '{}') {{ $target = $item; break }}
    }}
    if (-not $target) {{ Write-Output 'ERR:Élément introuvable dans la Corbeille'; exit }}
    $deletedFrom = $target.ExtendedProperty("System.Recycle.DeletedFrom")
    if (-not $deletedFrom) {{ Write-Output 'ERR:Emplacement d''origine introuvable'; exit }}
    $trueOriginal = Join-Path $deletedFrom $target.Name
    if (-not (Test-Path $deletedFrom)) {{ New-Item -ItemType Directory -Path $deletedFrom -Force | Out-Null }}
    Move-Item -LiteralPath $target.Path -Destination $trueOriginal -Force -ErrorAction Stop
    Write-Output "OK:$trueOriginal"
}} catch {{ Write-Output "ERR:$($_.Exception.Message)" }}
"#, original_path.replace('\'', "''"));

    match run_ps(&ps) {
        Some(t) if t.trim().starts_with("OK:") => {
            let restored_path = t.trim()[3..].to_string();
            RestoreResult {
                success: true,
                message: "Fichier restauré à son emplacement d'origine".into(),
                restored_path,
            }
        }
        Some(t) => RestoreResult {
            success: false,
            message: t.trim().trim_start_matches("ERR:").to_string(),
            restored_path: String::new(),
        },
        None => RestoreResult {
            success: false,
            message: "Erreur PowerShell".into(),
            restored_path: String::new(),
        },
    }
}

/// Scan MFT pour fichiers récemment supprimés via fsutil
pub fn scan_deleted_files(drive: String) -> Vec<RecoveredFile> {
    // fsutil exige le volume avec ':' (ex: "C:") — le frontend envoie une lettre nue
    // ("C"), et un ancien trim_end_matches(':') supprimait aussi le ':' si présent,
    // donc fsutil recevait toujours "C" seul, échouait silencieusement, et le scan
    // retournait systématiquement une liste vide.
    let letter = drive.trim_end_matches(':').replace('\'', "''");
    let ps = format!(r#"
try {{
    $drive = '{}:'
    $result = fsutil usn readjournal $drive maxcount=500 2>$null
    if (-not $result) {{ Write-Output '[]'; exit }}

    $deleted = @()
    $lines = $result -split "`n"
    $current = @{{}}
    foreach ($line in $lines) {{
        if ($line -match '^\s*File name\s*:\s*(.+)') {{ $current.name = $matches[1].Trim() }}
        if ($line -match '^\s*Reason\s*:\s*(.+)') {{ $current.reason = $matches[1].Trim() }}
        if ($line -match '^\s*Time stamp\s*:\s*(.+)') {{ $current.time = $matches[1].Trim() }}
        if ($line -match '^\s*$' -and $current.name -and $current.reason -like '*Delete*') {{
            $deleted += @{{ name=$current.name; time=$current.time; reason=$current.reason }}
            $current = @{{}}
        }}
    }}
    $deleted | Select-Object -First 200 | ConvertTo-Json -Compress
}} catch {{ Write-Output '[]' }}
"#, letter);

    let drive_letter = drive.trim_end_matches(':');
    run_ps(&ps)
        .and_then(|t| {
            let t = t.trim();
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().filter_map(|v| {
            Some(RecoveredFile {
                name: v["name"].as_str()?.to_string(),
                path: format!("{}:\\[supprimé]\\{}", drive_letter, v["name"].as_str().unwrap_or("")),
                size_bytes: 0,
                deleted_date: v["time"].as_str().unwrap_or("").to_string(),
                source: "mft_journal".to_string(),
                is_dir: false,
            })
        }).collect())
        .unwrap_or_default()
}

/// Scan USN Journal — Tous les lecteurs NTFS
pub fn scan_all_deleted_files() -> Vec<RecoveredFile> {
    let ps = r#"
try {
    $drives = Get-WmiObject Win32_LogicalDisk -ErrorAction SilentlyContinue |
        Where-Object { $_.FileSystem -eq 'NTFS' } |
        Select-Object -ExpandProperty DeviceID
    $deleted = @()
    foreach ($d in $drives) {
        $result = & fsutil usn readjournal $d maxcount=300 2>$null
        if (-not $result) { continue }
        $lines = $result -split "`n"
        $current = @{}
        foreach ($line in $lines) {
            if ($line -match '^\s*File name\s*:\s*(.+)')  { $current.name   = $matches[1].Trim() }
            if ($line -match '^\s*Reason\s*:\s*(.+)')     { $current.reason = $matches[1].Trim() }
            if ($line -match '^\s*Time stamp\s*:\s*(.+)') { $current.time   = $matches[1].Trim() }
            if ($line -match '^\s*$' -and $current.name -and $current.reason -like '*Delete*') {
                $deleted += @{ name=$current.name; time=$current.time; drive=$d }
                $current = @{}
            }
        }
    }
    if ($deleted.Count -eq 0) { Write-Output '[]' }
    else { $deleted | Select-Object -First 500 | ConvertTo-Json -Compress }
} catch { Write-Output '[]' }
"#;

    run_ps(ps)
        .and_then(|t| {
            let t = t.trim();
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().filter_map(|v| {
            let drive = v["drive"].as_str().unwrap_or("?:");
            let name  = v["name"].as_str()?.to_string();
            Some(RecoveredFile {
                name:         name.clone(),
                path:         format!("{}\\[supprimé]\\{}", drive, name),
                size_bytes:   0,
                deleted_date: v["time"].as_str().unwrap_or("").to_string(),
                source:       "mft_journal".to_string(),
                is_dir:       false,
            })
        }).collect())
        .unwrap_or_default()
}

pub fn compare_shadow_with_current(device_path: String, sub_path: String, live_path: String) -> Vec<ComparedFile> {
    let shadow_full = shadow_path(&device_path, &sub_path);
    let ps = format!(r#"
$shadowPath = '{sp}'
$livePath   = '{lp}'
try {{
    $shadowItems = @{{}}
    if (Test-Path -LiteralPath $shadowPath) {{
        Get-ChildItem -LiteralPath $shadowPath -Force -ErrorAction SilentlyContinue | ForEach-Object {{
            $shadowItems[$_.Name] = @{{
                size     = if ($_.PSIsContainer) {{ 0 }} else {{ try {{ $_.Length }} catch {{ 0 }} }}
                modified = try {{ $_.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss') }} catch {{ '' }}
                is_dir   = $_.PSIsContainer
                full     = $_.FullName
            }}
        }}
    }}
    $liveItems = @{{}}
    if (Test-Path -LiteralPath $livePath) {{
        Get-ChildItem -LiteralPath $livePath -Force -ErrorAction SilentlyContinue | ForEach-Object {{
            $liveItems[$_.Name] = @{{
                size     = if ($_.PSIsContainer) {{ 0 }} else {{ try {{ $_.Length }} catch {{ 0 }} }}
                modified = try {{ $_.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss') }} catch {{ '' }}
                is_dir   = $_.PSIsContainer
                full     = $_.FullName
            }}
        }}
    }}
    $result = @()
    foreach ($name in $shadowItems.Keys) {{
        $s = $shadowItems[$name]
        if (-not $liveItems.ContainsKey($name)) {{
            $result += @{{ name=$name; shadow_path=$s.full; live_path=''; status='deleted';
                          shadow_size=$s.size; live_size=0; shadow_modified=$s.modified; live_modified='' }}
        }} else {{
            $l = $liveItems[$name]
            if (-not $s.is_dir -and ($s.size -ne $l.size -or $s.modified -ne $l.modified)) {{
                $result += @{{ name=$name; shadow_path=$s.full; live_path=$l.full; status='modified';
                              shadow_size=$s.size; live_size=$l.size; shadow_modified=$s.modified; live_modified=$l.modified }}
            }}
        }}
    }}
    foreach ($name in $liveItems.Keys) {{
        if (-not $shadowItems.ContainsKey($name)) {{
            $l = $liveItems[$name]
            $result += @{{ name=$name; shadow_path=''; live_path=$l.full; status='added';
                          shadow_size=0; live_size=$l.size; shadow_modified=''; live_modified=$l.modified }}
        }}
    }}
    if ($result.Count -eq 0) {{ Write-Output '[]' }}
    elseif ($result.Count -eq 1) {{ Write-Output "[$($result | ConvertTo-Json -Compress -Depth 2)]" }}
    else {{ $result | Select-Object -First 300 | ConvertTo-Json -Compress -Depth 2 }}
}} catch {{ Write-Output '[]' }}
"#,
        sp = shadow_full.replace('\'', "''"),
        lp = live_path.replace('\'', "''")
    );

    run_ps(&ps)
        .and_then(|t| {
            let t = t.trim();
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().filter_map(|v| {
            Some(ComparedFile {
                name:            v["name"].as_str()?.to_string(),
                shadow_path:     v["shadow_path"].as_str().unwrap_or("").to_string(),
                live_path:       v["live_path"].as_str().unwrap_or("").to_string(),
                status:          v["status"].as_str().unwrap_or("").to_string(),
                shadow_size:     v["shadow_size"].as_u64().unwrap_or(0),
                live_size:       v["live_size"].as_u64().unwrap_or(0),
                shadow_modified: v["shadow_modified"].as_str().unwrap_or("").to_string(),
                live_modified:   v["live_modified"].as_str().unwrap_or("").to_string(),
            })
        }).collect())
        .unwrap_or_default()
}

/// Crée une nouvelle Shadow Copy VSS
pub fn create_shadow_copy(volume: String) -> Result<String, String> {
    let v = volume.trim_end_matches('\\').trim_end_matches(':');
    let drive = v.to_uppercase();
    if drive.len() != 1 || !drive.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
        return Err(format!("Volume invalide : lettre de lecteur requise (ex: C), reçu: '{}'", v));
    }
    let vol_path = format!("{}:\\", drive);
    let ps = format!(
        r#"$r = (Get-WmiObject -List Win32_ShadowCopy).Create('{}', 'ClientAccessible'); if ($r.ReturnValue -eq 0) {{ 'OK:' + $r.ShadowID }} else {{ 'ERR:Code ' + $r.ReturnValue }}"#,
        vol_path
    );
    match run_ps(&ps) {
        Some(t) if t.trim().starts_with("OK:") => Ok(t.trim()[3..].to_string()),
        Some(t) => Err(t.trim().trim_start_matches("ERR:").to_string()),
        None => Err("Erreur PowerShell".into()),
    }
}

/// Supprime une Shadow Copy par son ID VSS
pub fn delete_shadow_copy(shadow_id: String) -> Result<String, String> {
    let ps = format!(
        r#"$s = Get-WmiObject Win32_ShadowCopy | Where-Object {{ $_.ID -eq '{}' }}; if ($s) {{ $s.Delete(); 'OK' }} else {{ 'ERR:Shadow copy introuvable' }}"#,
        shadow_id.replace('\'', "''")
    );
    match run_ps(&ps) {
        Some(t) if t.trim() == "OK" => Ok("Shadow copy supprimée".into()),
        Some(t) => Err(t.trim().trim_start_matches("ERR:").to_string()),
        None => Err("Erreur PowerShell".into()),
    }
}

/// Retourne les lettres des volumes NTFS connectés
pub fn get_ntfs_drives() -> Vec<String> {
    let ps = r#"Get-WmiObject Win32_LogicalDisk -ErrorAction SilentlyContinue | Where-Object { $_.FileSystem -eq 'NTFS' } | Select-Object -ExpandProperty DeviceID"#;
    run_ps(ps)
        .map(|t| t.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default()
}
