use serde::Serialize;
use std::io::{Read, Seek, SeekFrom, Write};
use std::process::Command;
use std::time::Instant;
use tauri::Emitter;

#[cfg(target_os = "windows")]
use std::fs::OpenOptions;
#[cfg(target_os = "windows")]
use std::os::windows::fs::OpenOptionsExt;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const CHUNK: usize = 65_536; // 64 KB — taille standard secteur logique aligné

// ── Structs publiques ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct DiskImageResult {
    pub success: bool,
    pub message: String,
    pub output_path: String,
    pub bad_sectors: u32,
    pub bytes_copied: u64,
    pub speed_mbs: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SurfaceBlock {
    pub index: u32,
    pub offset_gb: f64,
    pub speed_mbs: f64,
    pub status: String, // "ok" | "slow" | "bad"
}

#[derive(Debug, Clone, Serialize)]
pub struct SurfaceTestResult {
    pub drive: String,
    pub total_blocks: u32,
    pub tested_blocks: u32,
    pub bad_blocks: u32,
    pub slow_blocks: u32,
    pub blocks: Vec<SurfaceBlock>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeepMftFile {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified: String,
    pub extension: String,
    pub is_deleted: bool,
    pub source: String, // "recycle_bin" | "usn_journal" | "temp"
}

// ── Payloads d'événements (Tauri events) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
struct ImageProgress {
    bytes_done: u64,
    total_bytes: u64,
    percent: f64,
    speed_mbs: f64,
    bad_sectors: u32,
    eta_secs: u64,
}

#[derive(Debug, Clone, Serialize)]
struct SurfaceProgress {
    blocks_done: u32,
    total_blocks: u32,
    bad_blocks: u32,
    slow_blocks: u32,
    percent: f64,
    last_speed_mbs: f64,
}

// ── Création d'image disque brute ──────────────────────────────────────────────

fn get_disk_size_ps(disk_index: u32) -> Option<u64> {
    let ps = format!(
        "try {{ (Get-Disk -Number {} -ErrorAction Stop).Size }} catch {{ 0 }}",
        disk_index
    );
    run_ps_opt(&ps)?.trim().parse::<u64>().ok().filter(|&v| v > 0)
}

pub fn create_disk_image(
    disk_index: u32,
    output_path: String,
    window: &tauri::Window,
) -> DiskImageResult {
    let fail = |msg: &str| DiskImageResult {
        success: false,
        message: msg.to_string(),
        output_path: output_path.clone(),
        bad_sectors: 0,
        bytes_copied: 0,
        speed_mbs: 0.0,
    };

    let total_bytes = match get_disk_size_ps(disk_index) {
        Some(s) => s,
        None => return fail("Taille disque introuvable — droits Administrateur requis."),
    };

    #[cfg(not(target_os = "windows"))]
    return fail("Disponible sur Windows uniquement.");

    #[cfg(target_os = "windows")]
    {
        let disk_path = format!(r"\\.\PhysicalDrive{}", disk_index);
        let mut src = match OpenOptions::new()
            .read(true)
            .share_mode(3) // FILE_SHARE_READ | FILE_SHARE_WRITE
            .custom_flags(0x08000000) // FILE_FLAG_SEQUENTIAL_SCAN
            .open(&disk_path)
        {
            Ok(f) => f,
            Err(e) => return fail(&format!("Ouverture disque {} impossible: {} (admin requis)", disk_path, e)),
        };

        let mut dst = match std::fs::File::create(&output_path) {
            Ok(f) => f,
            Err(e) => return fail(&format!("Création fichier image impossible: {}", e)),
        };

        let mut buf = vec![0u8; CHUNK];
        let zero_buf = vec![0u8; CHUNK];
        let mut bytes_done: u64 = 0;
        let mut bad_sectors: u32 = 0;
        let global_start = Instant::now();
        let mut last_emit = Instant::now();

        loop {
            match src.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if dst.write_all(&buf[..n]).is_err() { break; }
                    bytes_done += n as u64;
                }
                Err(_) => {
                    bad_sectors += 1;
                    let _ = dst.write_all(&zero_buf); // secteur défectueux → zéros
                    bytes_done += CHUNK as u64;
                    let _ = src.seek(SeekFrom::Start(bytes_done)); // sauter le secteur
                }
            }

            if last_emit.elapsed().as_millis() > 300 {
                let secs = global_start.elapsed().as_secs_f64().max(0.001);
                let speed_mbs = (bytes_done as f64 / 1_048_576.0) / secs;
                let percent = (bytes_done as f64 / total_bytes as f64 * 100.0).min(100.0);
                let remaining = total_bytes.saturating_sub(bytes_done);
                let eta_secs = if speed_mbs > 0.1 {
                    ((remaining as f64 / 1_048_576.0) / speed_mbs) as u64
                } else { 0 };
                let _ = window.emit("disk-image-progress", ImageProgress {
                    bytes_done, total_bytes, percent, speed_mbs, bad_sectors, eta_secs,
                });
                last_emit = Instant::now();
            }
        }

        let elapsed = global_start.elapsed().as_secs_f64().max(0.001);
        let speed_mbs = (bytes_done as f64 / 1_048_576.0) / elapsed;

        let _ = window.emit("disk-image-progress", ImageProgress {
            bytes_done, total_bytes, percent: 100.0, speed_mbs, bad_sectors, eta_secs: 0,
        });

        DiskImageResult {
            success: true,
            message: format!(
                "{:.2} GB copiés en {:.0}s à {:.1} MB/s — {} secteur(s) défectueux.",
                bytes_done as f64 / 1_073_741_824.0,
                elapsed,
                speed_mbs,
                bad_sectors
            ),
            output_path,
            bad_sectors,
            bytes_copied: bytes_done,
            speed_mbs,
        }
    }
}

// ── Test de surface (lecture secteur par secteur) ──────────────────────────────

pub fn surface_test_volume(
    drive_letter: String,
    window: &tauri::Window,
) -> SurfaceTestResult {
    let clean = drive_letter.trim_end_matches(':').to_uppercase();
    // Validation : exactement une lettre A-Z
    if clean.len() != 1 || !clean.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) {
        return SurfaceTestResult { drive: clean, total_blocks: 0, tested_blocks: 0, bad_blocks: 0, slow_blocks: 0, blocks: vec![] };
    }
    let empty_result = SurfaceTestResult {
        drive: clean.clone(),
        total_blocks: 0,
        tested_blocks: 0,
        bad_blocks: 0,
        slow_blocks: 0,
        blocks: vec![],
    };

    #[cfg(not(target_os = "windows"))]
    return empty_result;

    #[cfg(target_os = "windows")]
    {
        let vol_size_ps = format!(
            "try {{ (Get-Volume -DriveLetter '{}' -ErrorAction Stop).Size }} catch {{ 0 }}",
            clean
        );
        let total_bytes: u64 = run_ps_opt(&vol_size_ps)
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);

        if total_bytes == 0 { return empty_result; }

        let vol_path = format!(r"\\.\{}:", clean);
        let mut f = match OpenOptions::new()
            .read(true)
            .share_mode(3)
            .custom_flags(0x08000000) // FILE_FLAG_SEQUENTIAL_SCAN
            .open(&vol_path)
        {
            Ok(f) => f,
            Err(_) => return empty_result,
        };

        let total_blocks = (total_bytes / CHUNK as u64) as u32;
        let max_sample_points: usize = 2000;
        let sample_every = std::cmp::max(1, total_blocks as usize / max_sample_points);

        let mut buf = vec![0u8; CHUNK];
        let mut bad_blocks: u32 = 0;
        let mut slow_blocks: u32 = 0;
        let mut blocks: Vec<SurfaceBlock> = Vec::with_capacity(max_sample_points);
        let mut last_emit = Instant::now();

        for i in 0..total_blocks {
            let t0 = Instant::now();
            let status = match f.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    let ms = t0.elapsed().as_millis();
                    if ms > 200 {
                        slow_blocks += 1;
                        "slow"
                    } else {
                        "ok"
                    }
                }
                Err(_) => {
                    bad_blocks += 1;
                    let next_pos = (i as u64 + 1) * CHUNK as u64;
                    let _ = f.seek(SeekFrom::Start(next_pos));
                    "bad"
                }
            };

            if (i as usize).is_multiple_of(sample_every) {
                let ms = t0.elapsed().as_millis().max(1);
                let speed_mbs = (CHUNK as f64 / 1_048_576.0) / (ms as f64 / 1000.0);
                blocks.push(SurfaceBlock {
                    index: i,
                    offset_gb: (i as f64 * CHUNK as f64) / 1_073_741_824.0,
                    speed_mbs,
                    status: status.to_string(),
                });
            }

            if last_emit.elapsed().as_millis() > 250 {
                let ms = t0.elapsed().as_millis().max(1);
                let speed_mbs = (CHUNK as f64 / 1_048_576.0) / (ms as f64 / 1000.0);
                let _ = window.emit("surface-test-progress", SurfaceProgress {
                    blocks_done: i,
                    total_blocks,
                    bad_blocks,
                    slow_blocks,
                    percent: (i as f64 / total_blocks as f64 * 100.0).min(100.0),
                    last_speed_mbs: speed_mbs,
                });
                last_emit = Instant::now();
            }
        }

        let _ = window.emit("surface-test-progress", SurfaceProgress {
            blocks_done: total_blocks,
            total_blocks,
            bad_blocks,
            slow_blocks,
            percent: 100.0,
            last_speed_mbs: 0.0,
        });

        SurfaceTestResult {
            drive: clean,
            total_blocks,
            tested_blocks: total_blocks,
            bad_blocks,
            slow_blocks,
            blocks,
        }
    }
}

// ── Scan MFT approfondi (USN Journal + Corbeille + Temp) ──────────────────────

pub fn deep_mft_scan_advanced(drive: String) -> Vec<DeepMftFile> {
    let clean = drive.trim_end_matches(':').to_uppercase();
    // Validation : exactement une lettre A-Z
    if clean.len() != 1 || !clean.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) {
        return vec![];
    }
    let vol = format!("{}:", clean);

    let ps = format!(
        r#"
$result = @()
$vol = '{vol}'

# 1 — Corbeille Windows (tous profils utilisateurs)
try {{
    $bin = "$vol\`$Recycle.Bin"
    if (Test-Path $bin) {{
        Get-ChildItem -Path $bin -Recurse -Force -ErrorAction SilentlyContinue |
        Where-Object {{ -not $_.PSIsContainer }} |
        Select-Object -First 300 |
        ForEach-Object {{
            $result += @{{
                name = $_.Name
                path = $_.FullName
                size_bytes = $_.Length
                modified = $_.LastWriteTime.ToString('s')
                extension = $_.Extension.ToLower()
                is_deleted = $true
                source = 'recycle_bin'
            }}
        }}
    }}
}} catch {{}}

# 2 — USN Journal (suppressions récentes)
try {{
    $usn = & fsutil usn readdata $vol 2>&1
    $prevLine = ''
    foreach ($line in $usn) {{
        if ($prevLine -match 'Reason.*DELETE' -or $line -match 'Reason.*DELETE') {{
            if ($line -match 'FileName\s*:\s+(.+)') {{
                $fn = $Matches[1].Trim()
                $result += @{{
                    name = $fn
                    path = "$vol\$fn"
                    size_bytes = 0
                    modified = (Get-Date).ToString('s')
                    extension = [System.IO.Path]::GetExtension($fn).ToLower()
                    is_deleted = $true
                    source = 'usn_journal'
                }}
            }}
        }}
        $prevLine = $line
    }}
}} catch {{}}

# 3 — Fichiers temporaires récupérables sur ce volume
try {{
    $temps = @("$vol\Windows\Temp")
    foreach ($t in $temps) {{
        if (Test-Path $t) {{
            Get-ChildItem -Path $t -Force -ErrorAction SilentlyContinue |
            Where-Object {{ -not $_.PSIsContainer -and $_.Length -gt 0 }} |
            Select-Object -First 50 |
            ForEach-Object {{
                $result += @{{
                    name = $_.Name
                    path = $_.FullName
                    size_bytes = $_.Length
                    modified = $_.LastWriteTime.ToString('s')
                    extension = $_.Extension.ToLower()
                    is_deleted = $false
                    source = 'temp'
                }}
            }}
        }}
    }}
}} catch {{}}

# 4 — Fichiers récents accédés mais supprimés (Recent folder)
try {{
    $recent = "$env:APPDATA\Microsoft\Windows\Recent"
    if (Test-Path $recent) {{
        Get-ChildItem -Path $recent -Force -ErrorAction SilentlyContinue |
        Where-Object {{ -not $_.PSIsContainer -and $_.Extension -eq '.lnk' }} |
        Select-Object -First 100 |
        ForEach-Object {{
            $sh = New-Object -ComObject WScript.Shell
            try {{
                $lnk = $sh.CreateShortcut($_.FullName)
                $target = $lnk.TargetPath
                if ($target -and -not (Test-Path $target)) {{
                    $result += @{{
                        name = [System.IO.Path]::GetFileName($target)
                        path = $target
                        size_bytes = 0
                        modified = $_.LastWriteTime.ToString('s')
                        extension = [System.IO.Path]::GetExtension($target).ToLower()
                        is_deleted = $true
                        source = 'recent_lnk'
                    }}
                }}
            }} catch {{}}
        }}
    }}
}} catch {{}}

$result = $result | Select-Object -Unique | Select-Object -First 500
if ($result.Count -eq 0) {{ Write-Output '[]' }}
elseif ($result.Count -eq 1) {{ Write-Output "[$($result | ConvertTo-Json -Compress -Depth 2)]" }}
else {{ $result | ConvertTo-Json -Compress -Depth 2 }}
"#,
        vol = vol
    );

    run_ps_opt(&ps)
        .and_then(|t| {
            let t = t.trim();
            if t.is_empty() || t == "[]" { return Some(vec![]); }
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().map(|v| {
            DeepMftFile {
                name:       v["name"].as_str().unwrap_or("").to_string(),
                path:       v["path"].as_str().unwrap_or("").to_string(),
                size_bytes: v["size_bytes"].as_u64().unwrap_or(0),
                modified:   v["modified"].as_str().unwrap_or("").to_string(),
                extension:  v["extension"].as_str().unwrap_or("").to_string(),
                is_deleted: v["is_deleted"].as_bool().unwrap_or(false),
                source:     v["source"].as_str().unwrap_or("").to_string(),
            }
        }).collect())
        .unwrap_or_default()
}

// ── Génération de rapport HTML professionnel ───────────────────────────────────

pub fn generate_recovery_report(
    title: String,
    files_json: String,
    output_path: String,
) -> Result<String, String> {
    let files: Vec<serde_json::Value> = serde_json::from_str(&files_json).unwrap_or_default();
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let total = files.len();
    let total_size: u64 = files.iter().filter_map(|f| f["size_bytes"].as_u64()).sum();
    let deleted_count = files.iter().filter(|f| f["is_deleted"].as_bool().unwrap_or(false)).count();

    let rows: String = files.iter().map(|f| {
        let name    = f["name"].as_str().unwrap_or("-");
        let path    = f["path"].as_str().unwrap_or("-");
        let size    = f["size_bytes"].as_u64().unwrap_or(0);
        let _modif  = f["modified"].as_str().unwrap_or("-");
        let ext     = f["extension"].as_str().unwrap_or("-");
        let source  = f["source"].as_str().unwrap_or("-");
        let deleted = if f["is_deleted"].as_bool().unwrap_or(false) {
            "<span class='badge-del'>Supprimé</span>"
        } else {
            "<span class='badge-ok'>Actif</span>"
        };
        format!(
            "<tr><td class='col-name'>{}</td><td class='col-path'>{}</td><td class='col-size'>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            esc(name), esc(path), fmt_size(size), esc(ext), esc(source), deleted
        )
    }).collect();

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="UTF-8">
<title>{title}</title>
<style>
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{ font-family: 'Segoe UI', Tahoma, sans-serif; background: #0d0d11; color: #e0e0e8; padding: 28px; line-height: 1.5; }}
  h1 {{ color: #f97316; font-size: 22px; border-bottom: 2px solid #f97316; padding-bottom: 10px; margin-bottom: 20px; }}
  .stats {{ display: flex; gap: 16px; flex-wrap: wrap; margin-bottom: 24px; }}
  .stat {{ background: #1a1a24; border: 1px solid #2e2e3a; border-radius: 8px; padding: 14px 22px; min-width: 140px; }}
  .stat .lbl {{ font-size: 10px; color: #888; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 4px; }}
  .stat .val {{ font-size: 22px; font-weight: 700; color: #f97316; }}
  table {{ width: 100%; border-collapse: collapse; font-size: 12.5px; }}
  thead th {{ background: #1a1a24; color: #888; font-size: 10px; text-transform: uppercase; letter-spacing: 0.8px; padding: 10px 12px; text-align: left; border-bottom: 1px solid #2e2e3a; }}
  td {{ padding: 7px 12px; border-bottom: 1px solid #1e1e28; vertical-align: top; }}
  tr:hover td {{ background: #15151f; }}
  .col-name {{ font-weight: 600; color: #e0e0f0; max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
  .col-path {{ color: #777; font-size: 11px; max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
  .col-size {{ color: #a0a0c0; white-space: nowrap; }}
  .badge-del {{ background: rgba(239,68,68,0.15); color: #f87171; border: 1px solid rgba(239,68,68,0.3); border-radius: 4px; padding: 1px 7px; font-size: 10px; }}
  .badge-ok {{ background: rgba(34,197,94,0.15); color: #4ade80; border: 1px solid rgba(34,197,94,0.3); border-radius: 4px; padding: 1px 7px; font-size: 10px; }}
  .footer {{ margin-top: 28px; font-size: 11px; color: #444; text-align: center; border-top: 1px solid #1e1e28; padding-top: 12px; }}
</style>
</head>
<body>
<h1>🔍 {title}</h1>
<div class="stats">
  <div class="stat"><div class="lbl">Généré le</div><div class="val" style="font-size:13px;margin-top:2px">{now}</div></div>
  <div class="stat"><div class="lbl">Fichiers trouvés</div><div class="val">{total}</div></div>
  <div class="stat"><div class="lbl">Fichiers supprimés</div><div class="val" style="color:#f87171">{deleted_count}</div></div>
  <div class="stat"><div class="lbl">Taille totale</div><div class="val" style="font-size:15px;margin-top:4px">{total_size_fmt}</div></div>
</div>
<table>
<thead>
  <tr><th>Nom</th><th>Chemin</th><th>Taille</th><th>Extension</th><th>Source</th><th>Statut</th></tr>
</thead>
<tbody>{rows}</tbody>
</table>
<div class="footer">Rapport généré par <strong>Nitrite v26.39.0</strong> — Outil de récupération et diagnostic professionnel</div>
</body>
</html>"#,
        title           = esc(&title),
        now             = now,
        total           = total,
        deleted_count   = deleted_count,
        total_size_fmt  = fmt_size(total_size),
        rows            = rows,
    );

    std::fs::write(&output_path, html.as_bytes())
        .map_err(|e| format!("Écriture rapport: {}", e))?;

    Ok(output_path)
}

// ── Helpers internes ───────────────────────────────────────────────────────────

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}

fn fmt_size(bytes: u64) -> String {
    match bytes {
        b if b >= 1_073_741_824 => format!("{:.1} GB", b as f64 / 1_073_741_824.0),
        b if b >= 1_048_576     => format!("{:.1} MB", b as f64 / 1_048_576.0),
        b if b >= 1_024         => format!("{:.1} KB", b as f64 / 1_024.0),
        b => format!("{} B", b),
    }
}

fn run_ps_opt(script: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(0x08000000)
            .output()
            .ok()?;
        Some(String::from_utf8_lossy(&o.stdout).to_string())
    }
    #[cfg(not(target_os = "windows"))]
    None
}
