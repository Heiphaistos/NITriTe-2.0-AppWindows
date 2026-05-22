//! winpe.rs — Commandes spécifiques au mode Windows PE
//! Détection, réparation MBR/BCD, SFC offline, reset mot de passe, etc.
use serde::{Deserialize, Serialize};
use crate::error::NiTriTeError;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const NO_WINDOW: u32 = 0x08000000;

// ── Structures ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeDrive {
    pub letter: String,
    pub label: String,
    pub size_gb: f64,
    pub free_gb: f64,
    pub fs: String,
    pub is_system: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeSystemInfo {
    pub is_winpe: bool,
    pub pe_version: String,
    pub cpu: String,
    pub ram_gb: f64,
    pub drives: Vec<PeDrive>,
    pub arch: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepairResult {
    pub success: bool,
    pub output: String,
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OfflineUser {
    pub username: String,
    pub full_name: String,
    pub account_type: String,
    pub enabled: bool,
}

// ── Détection WinPE ─────────────────────────────────────────────────────────

fn detect_winpe() -> bool {
    // Méthode 1 : clé registre MiniNT présente uniquement en WinPE
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let out = Command::new("reg")
            .args(["query", r"HKLM\System\CurrentControlSet\Control\MiniNT"])
            .creation_flags(NO_WINDOW)
            .output();
        if let Ok(o) = out {
            if o.status.success() {
                return true;
            }
        }
    }
    // Méthode 2 : disque X: (lettre de la RAM disk WinPE)
    if std::env::var("SYSTEMDRIVE").map(|v| v.to_uppercase() == "X:").unwrap_or(false) {
        return true;
    }
    // Méthode 3 : fichier wpeinit.exe sur X:
    std::path::Path::new(r"X:\Windows\System32\wpeinit.exe").exists()
}

// ── Commandes Tauri ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn is_winpe_mode() -> Result<bool, NiTriTeError> {
    tokio::task::spawn_blocking(detect_winpe)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
pub async fn get_pe_system_info() -> Result<PeSystemInfo, NiTriTeError> {
    tokio::task::spawn_blocking(|| {
        let is_winpe = detect_winpe();

        // CPU
        let cpu = {
            #[cfg(target_os = "windows")]
            {
                let out = std::process::Command::new("wmic")
                    .args(["cpu", "get", "name", "/value"])
                    .creation_flags(NO_WINDOW)
                    .output()
                    .map(|o| o.stdout)
                    .unwrap_or_default();
                String::from_utf8_lossy(&out)
                    .lines()
                    .find(|l| l.starts_with("Name="))
                    .map(|l| l.trim_start_matches("Name=").trim().to_string())
                    .unwrap_or_else(|| "Inconnu".to_string())
            }
            #[cfg(not(target_os = "windows"))]
            { "Inconnu".to_string() }
        };

        // RAM
        let ram_gb = {
            #[cfg(target_os = "windows")]
            {
                let out = std::process::Command::new("wmic")
                    .args(["ComputerSystem", "get", "TotalPhysicalMemory", "/value"])
                    .creation_flags(NO_WINDOW)
                    .output()
                    .map(|o| o.stdout)
                    .unwrap_or_default();
                let raw = String::from_utf8_lossy(&out);
                raw.lines()
                    .find(|l| l.starts_with("TotalPhysicalMemory="))
                    .and_then(|l| l.trim_start_matches("TotalPhysicalMemory=").trim().parse::<u64>().ok())
                    .map(|b| b as f64 / 1_073_741_824.0)
                    .unwrap_or(0.0)
            }
            #[cfg(not(target_os = "windows"))]
            { 0.0 }
        };

        // Drives
        let drives = collect_pe_drives();

        // PE version
        let pe_version = if is_winpe {
            get_pe_version().unwrap_or_else(|| "WinPE 11".to_string())
        } else {
            "N/A".to_string()
        };

        let arch = std::env::var("PROCESSOR_ARCHITECTURE").unwrap_or_else(|_| "AMD64".to_string());

        Ok(PeSystemInfo { is_winpe, pe_version, cpu, ram_gb, drives, arch })
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

fn get_pe_version() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let out = std::process::Command::new("reg")
            .args(["query", r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion", "/v", "ReleaseId"])
            .creation_flags(NO_WINDOW)
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout);
        let ver = s.lines()
            .find(|l| l.contains("ReleaseId"))?
            .split_whitespace()
            .last()?
            .to_string();
        Some(format!("WinPE {}", ver))
    }
    #[cfg(not(target_os = "windows"))]
    { None }
}

fn collect_pe_drives() -> Vec<PeDrive> {
    #[cfg(target_os = "windows")]
    {
        let out = std::process::Command::new("wmic")
            .args(["logicaldisk", "get", "DeviceID,VolumeName,Size,FreeSpace,FileSystem", "/format:csv"])
            .creation_flags(NO_WINDOW)
            .output()
            .map(|o| o.stdout)
            .unwrap_or_default();
        let raw = String::from_utf8_lossy(&out);
        raw.lines()
            .skip(2) // header + blank
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| {
                let cols: Vec<&str> = line.split(',').collect();
                if cols.len() < 6 { return None; }
                let letter = cols[1].trim().to_string();
                if letter.is_empty() { return None; }
                let free_bytes = cols[2].trim().parse::<u64>().unwrap_or(0);
                let fs = cols[3].trim().to_string();
                let size_bytes = cols[4].trim().parse::<u64>().unwrap_or(0);
                let label = cols[5].trim().to_string();
                let is_system = letter == "C:" || letter == "X:";
                Some(PeDrive {
                    letter,
                    label,
                    size_gb: size_bytes as f64 / 1_073_741_824.0,
                    free_gb: free_bytes as f64 / 1_073_741_824.0,
                    fs,
                    is_system,
                })
            })
            .collect()
    }
    #[cfg(not(target_os = "windows"))]
    { vec![] }
}

#[tauri::command]
pub async fn get_pe_drives() -> Result<Vec<PeDrive>, NiTriTeError> {
    tokio::task::spawn_blocking(collect_pe_drives)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

// ── Réparation Boot ─────────────────────────────────────────────────────────

fn run_bootrec(args: &[&str]) -> RepairResult {
    #[cfg(target_os = "windows")]
    {
        let out = std::process::Command::new("bootrec")
            .args(args)
            .creation_flags(NO_WINDOW)
            .output();
        match out {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                let combined = if stderr.is_empty() { stdout } else { format!("{}\n{}", stdout, stderr) };
                RepairResult {
                    success: o.status.success(),
                    output: combined.trim().to_string(),
                    command: format!("bootrec {}", args.join(" ")),
                }
            }
            Err(e) => RepairResult {
                success: false,
                output: e.to_string(),
                command: format!("bootrec {}", args.join(" ")),
            },
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        RepairResult {
            success: false,
            output: "Non disponible hors Windows".to_string(),
            command: format!("bootrec {}", args.join(" ")),
        }
    }
}

#[tauri::command]
pub async fn repair_mbr() -> Result<RepairResult, NiTriTeError> {
    tokio::task::spawn_blocking(|| run_bootrec(&["/fixmbr"]))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
pub async fn repair_boot() -> Result<RepairResult, NiTriTeError> {
    tokio::task::spawn_blocking(|| run_bootrec(&["/fixboot"]))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
pub async fn rebuild_bcd() -> Result<RepairResult, NiTriTeError> {
    tokio::task::spawn_blocking(|| run_bootrec(&["/rebuildbcd"]))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
pub async fn scan_os_installations() -> Result<RepairResult, NiTriTeError> {
    tokio::task::spawn_blocking(|| run_bootrec(&["/scanos"]))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

// ── ChkDsk & SFC Offline ────────────────────────────────────────────────────

#[tauri::command]
pub async fn run_chkdsk_pe(drive: String, fix: bool) -> Result<RepairResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let mut args = vec![drive.as_str()];
            if fix { args.push("/f"); }
            args.push("/r");
            let out = std::process::Command::new("chkdsk")
                .args(&args)
                .creation_flags(NO_WINDOW)
                .output();
            match out {
                Ok(o) => {
                    let output = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    Ok(RepairResult {
                        success: o.status.code().map(|c| c < 8).unwrap_or(false),
                        output,
                        command: format!("chkdsk {} {}", drive, if fix { "/f /r" } else { "/r" }),
                    })
                }
                Err(e) => Ok(RepairResult { success: false, output: e.to_string(), command: "chkdsk".to_string() }),
            }
        }
        #[cfg(not(target_os = "windows"))]
        { Ok(RepairResult { success: false, output: "Non Windows".to_string(), command: "chkdsk".to_string() }) }
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
pub async fn run_sfc_offline(windows_dir: String) -> Result<RepairResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            // SFC /scannow /offbootdir=C:\ /offwindir=C:\Windows
            let drive = if windows_dir.len() >= 2 { windows_dir[..2].to_string() } else { "C:".to_string() } + "\\";
            let windir = windows_dir.clone();
            let out = std::process::Command::new("sfc")
                .args([
                    "/scannow",
                    &format!("/offbootdir={}", drive),
                    &format!("/offwindir={}", windir),
                ])
                .creation_flags(NO_WINDOW)
                .output();
            match out {
                Ok(o) => {
                    let output = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    Ok(RepairResult {
                        success: o.status.success(),
                        output,
                        command: format!("sfc /scannow /offbootdir={} /offwindir={}", drive, windir),
                    })
                }
                Err(e) => Ok(RepairResult { success: false, output: e.to_string(), command: "sfc".to_string() }),
            }
        }
        #[cfg(not(target_os = "windows"))]
        { Ok(RepairResult { success: false, output: "Non Windows".to_string(), command: "sfc".to_string() }) }
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// ── Comptes utilisateurs offline ────────────────────────────────────────────

#[tauri::command]
pub async fn list_offline_users(windows_dir: String) -> Result<Vec<OfflineUser>, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            // Monte la ruche SAM de l'OS offline
            let sam_path = format!("{}\\System32\\config\\SAM", windows_dir);
            if !std::path::Path::new(&sam_path).exists() {
                return Err(NiTriTeError::System(format!("SAM introuvable: {}", sam_path)));
            }
            // Charge la ruche dans HKLM\OFFLINE_SAM
            let _ = std::process::Command::new("reg")
                .args(["load", r"HKLM\OFFLINE_SAM", &sam_path])
                .creation_flags(NO_WINDOW)
                .status();

            // Enumère les comptes sous HKLM\OFFLINE_SAM\SAM\Domains\Account\Users\Names
            let out = std::process::Command::new("reg")
                .args(["query", r"HKLM\OFFLINE_SAM\SAM\Domains\Account\Users\Names"])
                .creation_flags(NO_WINDOW)
                .output()
                .map(|o| o.stdout)
                .unwrap_or_default();

            let users: Vec<OfflineUser> = String::from_utf8_lossy(&out)
                .lines()
                .filter(|l| l.trim().starts_with(r"HKLM\"))
                .map(|l| {
                    let username = l.rsplit('\\').next().unwrap_or("?").to_string();
                    OfflineUser {
                        username: username.clone(),
                        full_name: username,
                        account_type: "Utilisateur".to_string(),
                        enabled: true,
                    }
                })
                .collect();

            // Décharge la ruche
            let _ = std::process::Command::new("reg")
                .args(["unload", r"HKLM\OFFLINE_SAM"])
                .creation_flags(NO_WINDOW)
                .status();

            Ok(users)
        }
        #[cfg(not(target_os = "windows"))]
        { Ok(vec![]) }
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
pub async fn reset_user_password(windows_dir: String, username: String, new_password: String) -> Result<RepairResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            // Utilise net user via l'image offline (Windows installé) — nécessite que l'OS soit accessible
            let drive = if windows_dir.len() >= 2 { windows_dir[..2].to_string() } else { "C:".to_string() };
            let system32 = format!("{}\\Windows\\System32", drive);
            let net_exe = format!("{}\\net.exe", system32);

            if !std::path::Path::new(&net_exe).exists() {
                return Ok(RepairResult {
                    success: false,
                    output: format!("net.exe introuvable dans {}", system32),
                    command: "net user".to_string(),
                });
            }

            let out = std::process::Command::new(&net_exe)
                .args(["user", &username, &new_password])
                .creation_flags(NO_WINDOW)
                .output();

            match out {
                Ok(o) => Ok(RepairResult {
                    success: o.status.success(),
                    output: String::from_utf8_lossy(&o.stdout).trim().to_string(),
                    command: format!("net user {} ****", username),
                }),
                Err(e) => Ok(RepairResult { success: false, output: e.to_string(), command: "net user".to_string() }),
            }
        }
        #[cfg(not(target_os = "windows"))]
        { Ok(RepairResult { success: false, output: "Non Windows".to_string(), command: "net user".to_string() }) }
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// ── Effacement disque ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn disk_wipe(disk_index: u32, method: String) -> Result<RepairResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            // method: "quick" = clean, "secure" = clean all (zeros)
            let diskpart_cmd = if method == "secure" {
                format!("select disk {}\nclean all\n", disk_index)
            } else {
                format!("select disk {}\nclean\n", disk_index)
            };

            let script_path = std::env::temp_dir().join("nitrite_wipe.txt");
            std::fs::write(&script_path, &diskpart_cmd)
                .map_err(|e| NiTriTeError::System(e.to_string()))?;

            let out = std::process::Command::new("diskpart")
                .args(["/s", script_path.to_str().unwrap_or("")])
                .creation_flags(NO_WINDOW)
                .output();

            let _ = std::fs::remove_file(&script_path);

            match out {
                Ok(o) => Ok(RepairResult {
                    success: o.status.success(),
                    output: String::from_utf8_lossy(&o.stdout).trim().to_string(),
                    command: format!("diskpart clean{}", if method == "secure" { " all" } else { "" }),
                }),
                Err(e) => Ok(RepairResult { success: false, output: e.to_string(), command: "diskpart".to_string() }),
            }
        }
        #[cfg(not(target_os = "windows"))]
        { Ok(RepairResult { success: false, output: "Non Windows".to_string(), command: "diskpart".to_string() }) }
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// ── DISM offline ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn run_dism_offline_repair(windows_dir: String) -> Result<RepairResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let out = std::process::Command::new("dism")
                .args([
                    "/image:".to_string() + &windows_dir.replace("\\Windows", "\\"),
                    "/cleanup-image".to_string(),
                    "/restorehealth".to_string(),
                ])
                .creation_flags(NO_WINDOW)
                .output();
            match out {
                Ok(o) => {
                    let output = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    Ok(RepairResult { success: o.status.success(), output, command: "dism /cleanup-image /restorehealth".to_string() })
                }
                Err(e) => Ok(RepairResult { success: false, output: e.to_string(), command: "dism".to_string() }),
            }
        }
        #[cfg(not(target_os = "windows"))]
        { Ok(RepairResult { success: false, output: "Non Windows".to_string(), command: "dism".to_string() }) }
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// ── Détection installations Windows ─────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WindowsInstall {
    pub drive: String,
    pub windows_dir: String,
    pub version: String,
    pub build: String,
}

#[tauri::command]
pub async fn detect_windows_installs() -> Result<Vec<WindowsInstall>, NiTriTeError> {
    tokio::task::spawn_blocking(|| {
        let mut installs = Vec::new();

        #[cfg(target_os = "windows")]
        {
            // Cherche Windows dans chaque lecteur (sauf X: qui est la RAM disk WinPE)
            for letter in ['C', 'D', 'E', 'F', 'G', 'H', 'Y', 'Z'] {
                let win_dir = format!("{}:\\Windows", letter);
                let sys32 = format!("{}:\\Windows\\System32", letter);
                if std::path::Path::new(&sys32).exists() {
                    // Lit la version depuis le registre offline
                    let version_key = format!("{}:\\Windows\\System32\\config\\SOFTWARE", letter);
                    let (version, build) = if std::path::Path::new(&version_key).exists() {
                        let hive_key = format!("HKLM\\OFFLINE_SW_{}", letter);
                        let _ = std::process::Command::new("reg")
                            .args(["load", &hive_key, &version_key])
                            .creation_flags(NO_WINDOW)
                            .status();
                        let ver_out = std::process::Command::new("reg")
                            .args(["query", &format!("{}\\Microsoft\\Windows NT\\CurrentVersion", hive_key), "/v", "DisplayVersion"])
                            .creation_flags(NO_WINDOW)
                            .output()
                            .map(|o| o.stdout)
                            .unwrap_or_default();
                        let build_out = std::process::Command::new("reg")
                            .args(["query", &format!("{}\\Microsoft\\Windows NT\\CurrentVersion", hive_key), "/v", "CurrentBuildNumber"])
                            .creation_flags(NO_WINDOW)
                            .output()
                            .map(|o| o.stdout)
                            .unwrap_or_default();
                        let _ = std::process::Command::new("reg")
                            .args(["unload", &hive_key])
                            .creation_flags(NO_WINDOW)
                            .status();
                        let ver = String::from_utf8_lossy(&ver_out)
                            .lines()
                            .find(|l| l.contains("DisplayVersion"))
                            .and_then(|l| l.split_whitespace().last())
                            .map(|s| format!("Windows {}", s))
                            .unwrap_or_else(|| "Windows".to_string());
                        let bld = String::from_utf8_lossy(&build_out)
                            .lines()
                            .find(|l| l.contains("CurrentBuildNumber"))
                            .and_then(|l| l.split_whitespace().last())
                            .unwrap_or("?")
                            .to_string();
                        (ver, bld)
                    } else {
                        ("Windows".to_string(), "?".to_string())
                    };
                    installs.push(WindowsInstall {
                        drive: format!("{}:", letter),
                        windows_dir: win_dir,
                        version,
                        build,
                    });
                }
            }
        }

        Ok(installs)
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// ── BitLocker ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BitLockerStatus {
    pub drive: String,
    pub encrypted: bool,
    pub locked: bool,
    pub status_text: String,
}

#[tauri::command]
pub async fn get_bitlocker_status(drive: String) -> Result<BitLockerStatus, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let out = std::process::Command::new("manage-bde")
                .args(["-status", &drive])
                .creation_flags(NO_WINDOW)
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();
            let lower = out.to_lowercase();
            let encrypted = lower.contains("bitlocker") && !lower.contains("protection off") && !lower.contains("fully decrypted");
            let locked = lower.contains("locked") || lower.contains("key protectors: none");
            let status_text = if encrypted && locked { "Chiffré et verrouillé".to_string() }
                else if encrypted { "Chiffré mais accessible".to_string() }
                else { "Pas de BitLocker".to_string() };
            Ok(BitLockerStatus { drive, encrypted, locked, status_text })
        }
        #[cfg(not(target_os = "windows"))]
        { Ok(BitLockerStatus { drive, encrypted: false, locked: false, status_text: "Non Windows".to_string() }) }
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
pub async fn unlock_bitlocker(drive: String, recovery_key: String) -> Result<RepairResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let out = std::process::Command::new("manage-bde")
                .args(["-unlock", &drive, "-RecoveryPassword", &recovery_key])
                .creation_flags(NO_WINDOW)
                .output();
            match out {
                Ok(o) => {
                    let output = format!("{}\n{}", String::from_utf8_lossy(&o.stdout).trim(), String::from_utf8_lossy(&o.stderr).trim()).trim().to_string();
                    Ok(RepairResult { success: o.status.success(), output, command: format!("manage-bde -unlock {} -RecoveryPassword ****", drive) })
                }
                Err(e) => Ok(RepairResult { success: false, output: e.to_string(), command: "manage-bde -unlock".to_string() }),
            }
        }
        #[cfg(not(target_os = "windows"))]
        { Ok(RepairResult { success: false, output: "Non Windows".to_string(), command: "manage-bde".to_string() }) }
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// ── Outils mot de passe avancés ───────────────────────────────────────────────

#[tauri::command]
pub async fn clear_offline_password(windows_dir: String, username: String) -> Result<RepairResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let drive = if windows_dir.len() >= 2 { windows_dir[..2].to_string() } else { "C:".to_string() };
            let net_exe = format!("{}\\Windows\\System32\\net.exe", drive);
            if !std::path::Path::new(&net_exe).exists() {
                return Ok(RepairResult { success: false, output: format!("net.exe introuvable dans {}", drive), command: "net user".to_string() });
            }
            // net user <username> "" — mot de passe vide
            let out = std::process::Command::new(&net_exe)
                .args(["user", &username, ""])
                .creation_flags(NO_WINDOW)
                .output();
            match out {
                Ok(o) => Ok(RepairResult {
                    success: o.status.success(),
                    output: String::from_utf8_lossy(&o.stdout).trim().to_string(),
                    command: format!("net user {} [vide]", username),
                }),
                Err(e) => Ok(RepairResult { success: false, output: e.to_string(), command: "net user".to_string() }),
            }
        }
        #[cfg(not(target_os = "windows"))]
        { Ok(RepairResult { success: false, output: "Non Windows".to_string(), command: "net user".to_string() }) }
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
pub async fn enable_offline_account(windows_dir: String, username: String) -> Result<RepairResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let drive = if windows_dir.len() >= 2 { windows_dir[..2].to_string() } else { "C:".to_string() };
            let net_exe = format!("{}\\Windows\\System32\\net.exe", drive);
            if !std::path::Path::new(&net_exe).exists() {
                return Ok(RepairResult { success: false, output: format!("net.exe introuvable dans {}", drive), command: "net user".to_string() });
            }
            let out = std::process::Command::new(&net_exe)
                .args(["user", &username, "/active:yes"])
                .creation_flags(NO_WINDOW)
                .output();
            match out {
                Ok(o) => Ok(RepairResult {
                    success: o.status.success(),
                    output: String::from_utf8_lossy(&o.stdout).trim().to_string(),
                    command: format!("net user {} /active:yes", username),
                }),
                Err(e) => Ok(RepairResult { success: false, output: e.to_string(), command: "net user".to_string() }),
            }
        }
        #[cfg(not(target_os = "windows"))]
        { Ok(RepairResult { success: false, output: "Non Windows".to_string(), command: "net user".to_string() }) }
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// ── Exécution de commande générique WinPE ─────────────────────────────────────

#[tauri::command]
pub async fn winpe_run_command(command: String) -> Result<RepairResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let is_ps = command.starts_with("Get-") || command.starts_with("$")
                || command.starts_with("Set-") || command.starts_with("Start-")
                || command.starts_with("Stop-") || command.contains("-Object")
                || command.contains("Write-Host") || command.contains("Where-Object");
            let output = if is_ps {
                let ps_utf8 = format!(
                    "$OutputEncoding = [System.Text.Encoding]::UTF8; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; {}",
                    command
                );
                std::process::Command::new("powershell.exe")
                    .args(["-NoProfile", "-NonInteractive", "-Command", &ps_utf8])
                    .creation_flags(NO_WINDOW).output()
            } else {
                std::process::Command::new("cmd.exe")
                    .args(["/c", &command])
                    .creation_flags(NO_WINDOW).output()
            };
            match output {
                Ok(o) => {
                    let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                    let combined = if stderr.trim().is_empty() { stdout.trim().to_string() }
                        else { format!("{}\n[stderr] {}", stdout.trim(), stderr.trim()) };
                    Ok(RepairResult { success: o.status.success(), output: combined, command })
                }
                Err(e) => Ok(RepairResult { success: false, output: e.to_string(), command }),
            }
        }
        #[cfg(not(target_os = "windows"))]
        { Ok(RepairResult { success: false, output: "Disponible en WinPE uniquement.".to_string(), command }) }
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}
