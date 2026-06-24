use serde::Serialize;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// ── Structs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Default)]
pub struct DiskSmartInfo {
    pub disk_index: u32,
    pub label: String,
    pub health: String,          // "Healthy" | "Warning" | "Unhealthy" | "Unknown"
    pub temperature: Option<u32>,
    pub power_on_hours: Option<u32>,
    pub serial: String,
    pub size_gb: f64,
    pub media_type: String,      // "SSD" | "HDD" | "SCM" | "Unspecified"
    pub wear_level: Option<u32>,
    pub reallocated_sectors: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PartitionDetail {
    pub disk_index: u32,
    pub part_index: u32,
    pub disk_label: String,
    pub letter: String,
    pub label: String,
    pub size_gb: f64,
    pub free_gb: f64,
    pub file_system: String,
    pub part_type: String,       // "Basic" | "System" | "Recovery" | "Unallocated" | etc.
    pub is_system: bool,
    pub is_boot: bool,
    pub health: String,
}

// ── SMART via Get-PhysicalDisk + Get-StorageReliabilityCounter ─────────────────

pub fn get_disks_smart() -> Vec<DiskSmartInfo> {
    let ps = r#"
try {
    $result = @()
    Get-PhysicalDisk -ErrorAction SilentlyContinue | ForEach-Object {
        $d = $_
        $smart = $null
        try { $smart = Get-StorageReliabilityCounter -PhysicalDisk $d -ErrorAction SilentlyContinue } catch {}
        $result += @{
            disk_index          = try { [int]$d.DeviceId } catch { 0 }
            label               = [string]$d.FriendlyName
            health              = [string]$d.HealthStatus
            temperature         = if ($smart -and $smart.Temperature   -ne $null) { [int]$smart.Temperature   } else { $null }
            power_on_hours      = if ($smart -and $smart.PowerOnHours  -ne $null) { [int]$smart.PowerOnHours  } else { $null }
            serial              = [string]$d.SerialNumber
            size_gb             = [math]::Round([double]$d.Size / 1GB, 1)
            media_type          = [string]$d.MediaType
            wear_level          = if ($smart -and $smart.Wear          -ne $null) { [int]$smart.Wear          } else { $null }
            reallocated_sectors = if ($smart -and $smart.ReadErrorsTotal -ne $null) { [int]$smart.ReadErrorsTotal } else { $null }
        }
    }
    if ($result.Count -eq 0) { Write-Output '[]' }
    elseif ($result.Count -eq 1) { Write-Output "[$($result | ConvertTo-Json -Compress -Depth 2)]" }
    else { $result | ConvertTo-Json -Compress -Depth 2 }
} catch { Write-Output '[]' }
"#;

    run_ps(ps)
        .and_then(|t| {
            let t = t.trim();
            if t.is_empty() || t == "[]" { return Some(vec![]); }
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().map(|v| {
            DiskSmartInfo {
                disk_index:          v["disk_index"].as_u64().unwrap_or(0) as u32,
                label:               v["label"].as_str().unwrap_or("Disque").to_string(),
                health:              v["health"].as_str().unwrap_or("Unknown").to_string(),
                temperature:         v["temperature"].as_u64().map(|x| x as u32),
                power_on_hours:      v["power_on_hours"].as_u64().map(|x| x as u32),
                serial:              v["serial"].as_str().unwrap_or("").to_string(),
                size_gb:             v["size_gb"].as_f64().unwrap_or(0.0),
                media_type:          v["media_type"].as_str().unwrap_or("HDD").to_string(),
                wear_level:          v["wear_level"].as_u64().map(|x| x as u32),
                reallocated_sectors: v["reallocated_sectors"].as_u64().map(|x| x as u32),
            }
        }).collect())
        .unwrap_or_default()
}

// ── Liste des partitions avec espace non alloué ────────────────────────────────

pub fn get_partition_list() -> Vec<PartitionDetail> {
    let ps = r#"
try {
    $result = @()
    $disks = Get-Disk -ErrorAction SilentlyContinue
    foreach ($disk in $disks) {
        $diskLabel = [string]$disk.FriendlyName
        $parts = Get-Partition -DiskNumber $disk.Number -ErrorAction SilentlyContinue
        $totalUsed = [long]0
        foreach ($part in $parts) {
            $vol = $null
            try { $vol = Get-Volume -Partition $part -ErrorAction SilentlyContinue } catch {}
            $totalUsed += [long]$part.Size
            $result += @{
                disk_index  = [int]$disk.Number
                part_index  = [int]$part.PartitionNumber
                disk_label  = $diskLabel
                letter      = if ($part.DriveLetter) { "$($part.DriveLetter):" } else { '' }
                label       = if ($vol) { [string]$vol.FileSystemLabel } else { '' }
                size_gb     = [math]::Round([double]$part.Size / 1GB, 2)
                free_gb     = if ($vol) { [math]::Round([double]$vol.SizeRemaining / 1GB, 2) } else { 0 }
                file_system = if ($vol) { [string]$vol.FileSystem } else { 'RAW' }
                part_type   = [string]$part.Type
                is_system   = [bool]$part.IsSystem
                is_boot     = [bool]$part.IsBoot
                health      = if ($vol -and $vol.HealthStatus) { [string]$vol.HealthStatus } else { 'Unknown' }
            }
        }
        $unalloc = [double]$disk.Size - [double]$totalUsed
        if ($unalloc -gt 5MB) {
            $result += @{
                disk_index  = [int]$disk.Number
                part_index  = 0
                disk_label  = $diskLabel
                letter      = ''
                label       = 'Espace non alloue'
                size_gb     = [math]::Round($unalloc / 1GB, 2)
                free_gb     = [math]::Round($unalloc / 1GB, 2)
                file_system = 'RAW'
                part_type   = 'Unallocated'
                is_system   = $false
                is_boot     = $false
                health      = 'Unknown'
            }
        }
    }
    if ($result.Count -eq 0) { Write-Output '[]' }
    elseif ($result.Count -eq 1) { Write-Output "[$($result | ConvertTo-Json -Compress -Depth 2)]" }
    else { $result | ConvertTo-Json -Compress -Depth 2 }
} catch { Write-Output '[]' }
"#;

    run_ps(ps)
        .and_then(|t| {
            let t = t.trim();
            if t.is_empty() || t == "[]" { return Some(vec![]); }
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().map(|v| {
            PartitionDetail {
                disk_index:  v["disk_index"].as_u64().unwrap_or(0) as u32,
                part_index:  v["part_index"].as_u64().unwrap_or(0) as u32,
                disk_label:  v["disk_label"].as_str().unwrap_or("").to_string(),
                letter:      v["letter"].as_str().unwrap_or("").to_string(),
                label:       v["label"].as_str().unwrap_or("").to_string(),
                size_gb:     v["size_gb"].as_f64().unwrap_or(0.0),
                free_gb:     v["free_gb"].as_f64().unwrap_or(0.0),
                file_system: v["file_system"].as_str().unwrap_or("").to_string(),
                part_type:   v["part_type"].as_str().unwrap_or("").to_string(),
                is_system:   v["is_system"].as_bool().unwrap_or(false),
                is_boot:     v["is_boot"].as_bool().unwrap_or(false),
                health:      v["health"].as_str().unwrap_or("Unknown").to_string(),
            }
        }).collect())
        .unwrap_or_default()
}

// ── Opérations sur partitions (Admin requis) ───────────────────────────────────

pub fn format_partition(letter: String, fs: String, label: String) -> Result<String, String> {
    let clean = letter.trim().trim_end_matches(':').to_uppercase();
    // Bloquer C: et toute lettre de plus d'un caractère (invalide)
    if clean == "C" || clean.len() != 1 || !clean.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
        return Err("Lettre de lecteur invalide ou formatage du lecteur système interdit.".into());
    }
    // Valider le système de fichiers
    let allowed_fs = ["NTFS", "FAT32", "exFAT", "ReFS"];
    if !allowed_fs.contains(&fs.to_uppercase().as_str()) {
        return Err(format!("Système de fichiers non autorisé: {}", fs));
    }
    // Limiter le label à 32 caractères (limite NTFS)
    let label = if label.len() > 32 { label[..32].to_string() } else { label };

    // Vérification WMI : refuser les partitions système, boot, EFI ou Recovery
    // avant tout formatage destructif.
    let check_ps = format!(
        r#"try {{
  $p = Get-Partition -DriveLetter '{letter}' -ErrorAction Stop
  if ($p.IsSystem -or $p.IsBoot) {{ Write-Output 'SYSTEM'; exit }}
  $vol = $p | Get-Volume -ErrorAction SilentlyContinue
  if ($vol) {{
    $t = [string]$p.Type
    if ($t -eq 'System' -or $t -eq 'Recovery' -or $vol.FileSystem -eq 'FAT32' -and $vol.Size -lt 1GB -and $p.IsActive) {{
      Write-Output 'SYSTEM'; exit
    }}
  }}
  Write-Output 'OK'
}} catch {{ Write-Output 'OK' }}"#,
        letter = clean
    );
    let guard = run_ps_cmd(&check_ps).unwrap_or_else(|_| "OK".to_string());
    if guard.trim() == "SYSTEM" {
        return Err(format!(
            "Formatage interdit : le lecteur {}:\\ est une partition système, boot, EFI ou Recovery.",
            clean
        ));
    }

    let ps = format!(
        r#"Format-Volume -DriveLetter '{}' -FileSystem '{}' -NewFileSystemLabel '{}' -Confirm:$false -Force | Out-Null; 'OK'"#,
        clean,
        fs.replace('\'', "''"),
        label.replace('\'', "''")
    );
    run_ps_cmd(&ps)
}

pub fn assign_drive_letter(disk_index: u32, part_index: u32, letter: String) -> Result<String, String> {
    let clean = letter.trim_end_matches(':').to_uppercase();
    if clean.len() != 1 || !clean.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
        return Err("Lettre invalide (A-Z uniquement).".into());
    }
    let ps = format!(
        r#"Set-Partition -DiskNumber {} -PartitionNumber {} -NewDriveLetter '{}'; 'OK'"#,
        disk_index, part_index, clean
    );
    run_ps_cmd(&ps)
}

pub fn create_partition(disk_index: u32, size_mb: Option<u32>) -> Result<String, String> {
    let size_arg = match size_mb {
        Some(mb) if mb > 0 => format!("-Size {}MB", mb),
        _ => "-UseMaximumSize".to_string(),
    };
    let ps = format!(
        r#"New-Partition -DiskNumber {} {} | Format-Volume -FileSystem NTFS -NewFileSystemLabel 'Nouveau volume' -Confirm:$false -Force | Out-Null; 'OK'"#,
        disk_index, size_arg
    );
    run_ps_cmd(&ps)
}

pub fn delete_partition(disk_index: u32, part_index: u32) -> Result<String, String> {
    let ps = format!(
        r#"$p = Get-Partition -DiskNumber {} -PartitionNumber {} -ErrorAction Stop; if ($p.IsSystem -or $p.IsBoot) {{ throw 'Partition systeme/boot' }}; Remove-Partition -DiskNumber {} -PartitionNumber {} -Confirm:$false; 'OK'"#,
        disk_index, part_index, disk_index, part_index
    );
    run_ps_cmd(&ps)
}

pub fn initialize_disk(disk_index: u32, style: String) -> Result<String, String> {
    let ps_style = if style.eq_ignore_ascii_case("MBR") { "MBR" } else { "GPT" };
    let ps = format!(
        r#"Initialize-Disk -Number {} -PartitionStyle {} -Confirm:$false; 'OK'"#,
        disk_index, ps_style
    );
    run_ps_cmd(&ps)
}

// ── Resize Partition ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Default)]
pub struct PartitionSizeLimits {
    pub min_bytes: u64,
    pub max_bytes: u64,
    pub current_bytes: u64,
}

pub fn get_partition_resize_limits(disk_index: u32, part_index: u32) -> Result<PartitionSizeLimits, String> {
    let ps = format!(
        r#"try {{
  $s = Get-PartitionSupportedSize -DiskNumber {di} -PartitionNumber {pi} -ErrorAction Stop
  $p = Get-Partition -DiskNumber {di} -PartitionNumber {pi} -ErrorAction Stop
  @{{ min_bytes = [string]$s.SizeMin; max_bytes = [string]$s.SizeMax; current_bytes = [string]$p.Size }} | ConvertTo-Json -Compress
}} catch {{ Write-Error $_.Exception.Message }}"#,
        di = disk_index, pi = part_index
    );
    let out = run_ps_cmd(&ps)?;
    let v: serde_json::Value = serde_json::from_str(out.trim()).map_err(|e| e.to_string())?;
    Ok(PartitionSizeLimits {
        min_bytes:     v["min_bytes"].as_str().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0),
        max_bytes:     v["max_bytes"].as_str().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0),
        current_bytes: v["current_bytes"].as_str().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0),
    })
}

pub fn resize_partition_ps(disk_index: u32, part_index: u32, new_size_mb: u64) -> Result<String, String> {
    if new_size_mb == 0 {
        return Err("Taille invalide (0 MB).".into());
    }
    let ps = format!(
        r#"Resize-Partition -DiskNumber {} -PartitionNumber {} -Size {}MB -Confirm:$false; 'OK'"#,
        disk_index, part_index, new_size_mb
    );
    run_ps_cmd(&ps)
}

// ── Backup / Restore MBR (512 premiers octets du disque physique) ──────────────

pub fn backup_mbr(disk_index: u32, output_path: String) -> Result<String, String> {
    #[cfg(not(target_os = "windows"))]
    return Err("Windows uniquement.".into());

    #[cfg(target_os = "windows")]
    {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;
        use std::io::Read;

        let disk_path = format!(r"\\.\PhysicalDrive{}", disk_index);
        let mut f = OpenOptions::new()
            .read(true)
            .share_mode(3)
            .custom_flags(0x08000000) // FILE_FLAG_SEQUENTIAL_SCAN
            .open(&disk_path)
            .map_err(|e| format!("Ouverture disque impossible: {} (admin requis)", e))?;

        let mut mbr = [0u8; 512];
        f.read_exact(&mut mbr).map_err(|e| format!("Lecture MBR échouée: {}", e))?;

        std::fs::write(&output_path, mbr)
            .map_err(|e| format!("Écriture fichier MBR: {}", e))?;

        // Vérifie signature MBR (0x55AA en fin de secteur)
        let sig = u16::from_le_bytes([mbr[510], mbr[511]]);
        let sig_str = if sig == 0xAA55 { "valide (0x55AA)" } else { "non-standard" };
        Ok(format!("MBR sauvegardé ({} octets, signature {}) → {}", mbr.len(), sig_str, output_path))
    }
}

pub fn restore_mbr(disk_index: u32, mbr_path: String) -> Result<String, String> {
    #[cfg(not(target_os = "windows"))]
    return Err("Windows uniquement.".into());

    #[cfg(target_os = "windows")]
    {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;
        use std::io::Write;

        let mbr_data = std::fs::read(&mbr_path)
            .map_err(|e| format!("Lecture fichier MBR: {}", e))?;

        // Validation taille : 512 octets exactement (secteur MBR complet)
        // ou 446 octets (code MBR seul, sans table de partitions ni signature)
        if mbr_data.len() != 512 && mbr_data.len() != 446 {
            return Err(format!(
                "Fichier MBR invalide ({} octets, 512 ou 446 attendus).",
                mbr_data.len()
            ));
        }

        // Validation signature MBR : bytes 510-511 doivent être 0x55 0xAA
        // (uniquement vérifiable si le fichier fait 512 octets)
        if mbr_data.len() == 512 {
            let sig = u16::from_le_bytes([mbr_data[510], mbr_data[511]]);
            if sig != 0xAA55 {
                return Err(format!(
                    "Fichier MBR invalide : signature boot absente (attendu 0x55AA, trouvé 0x{:04X}).",
                    sig
                ));
            }
        }

        let disk_path = format!(r"\\.\PhysicalDrive{}", disk_index);
        let mut f = OpenOptions::new()
            .write(true)
            .share_mode(3)
            .custom_flags(0x08000000)
            .open(&disk_path)
            .map_err(|e| format!("Ouverture disque en écriture impossible: {} (admin requis)", e))?;

        f.write_all(&mbr_data[..512])
            .map_err(|e| format!("Écriture MBR échouée: {}", e))?;

        Ok(format!("MBR restauré depuis {} → Disque physique {}.", mbr_path, disk_index))
    }
}

// ── Scan partitions perdues ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Default)]
pub struct LostPartition {
    pub disk_index: u32,
    pub offset_bytes: u64,
    pub size_bytes: u64,
    pub signature: String,
    pub fs_hint: String,
    pub description: String,
}

pub fn scan_lost_partitions(disk_index: u32) -> Vec<LostPartition> {
    // Étape 1 — via PowerShell : chercher espace non alloué et résidus de partition
    let ps = format!(r#"
try {{
    $disk = Get-Disk -Number {di} -ErrorAction Stop
    $parts = @(Get-Partition -DiskNumber {di} -ErrorAction SilentlyContinue)
    $allocated = 0
    foreach ($p in $parts) {{ $allocated += [long]$p.Size }}
    $free = [long]$disk.Size - $allocated
    $result = @()
    if ($free -gt 10MB) {{
        $result += @{{
            disk_index   = [int]{di}
            offset_bytes = [string]$allocated
            size_bytes   = [string]$free
            signature    = 'UNALLOCATED'
            fs_hint      = 'Espace non alloué'
            description  = "Région non allouée de $([math]::Round($free / 1GB, 1)) GB détectée"
        }}
    }}
    # Cherche partitions cachées via Get-Disk PartitionStyle
    if ($disk.PartitionStyle -eq 'MBR') {{
        # MBR peut avoir des partitions étendues non déclarées
        $extParts = $parts | Where-Object {{ $_.Type -eq 'Extended' }}
        foreach ($ep in $extParts) {{
            $result += @{{
                disk_index   = [int]{di}
                offset_bytes = [string]$ep.Offset
                size_bytes   = [string]$ep.Size
                signature    = 'MBR_EXTENDED'
                fs_hint      = 'Partition étendue MBR'
                description  = "Partition étendue MBR de $([math]::Round($ep.Size/1GB,1)) GB"
            }}
        }}
    }}
    if ($result.Count -eq 0) {{ Write-Output '[]' }}
    elseif ($result.Count -eq 1) {{ Write-Output "[$($result | ConvertTo-Json -Compress -Depth 2)]" }}
    else {{ $result | ConvertTo-Json -Compress -Depth 2 }}
}} catch {{ Write-Output '[]' }}
"#, di = disk_index);

    run_ps(&ps)
        .and_then(|t| {
            let t = t.trim();
            if t.is_empty() || t == "[]" { return Some(vec![]); }
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().map(|v| {
            LostPartition {
                disk_index:   v["disk_index"].as_u64().unwrap_or(0) as u32,
                offset_bytes: v["offset_bytes"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0),
                size_bytes:   v["size_bytes"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0),
                signature:    v["signature"].as_str().unwrap_or("").to_string(),
                fs_hint:      v["fs_hint"].as_str().unwrap_or("").to_string(),
                description:  v["description"].as_str().unwrap_or("").to_string(),
            }
        }).collect())
        .unwrap_or_default()
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn run_ps_cmd(script: &str) -> Result<String, String> {
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() { "Échec (droits admin requis?)".into() } else { err })
    }
}

fn run_ps(script: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(0x08000000)
            .output().ok()?;
        Some(String::from_utf8_lossy(&o.stdout).to_string())
    }
    #[cfg(not(target_os = "windows"))]
    None
}
