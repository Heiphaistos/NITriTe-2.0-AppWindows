use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use super::{WingetUpgrade, SuspiciousProcess, DiskUsage, EventEntry};

// === System Summary ===

#[derive(Default)]
pub struct SystemSummary {
    pub uptime_hours: f64, pub cpu_name: String, pub cpu_cores: u32,
    pub cpu_usage: f32, pub ram_total: f64, pub ram_used: f64, pub ram_pct: f32, pub win_version: String,
}

pub fn collect_system_summary() -> SystemSummary {
    use sysinfo::System;
    let mut sys = System::new_all();
    std::thread::sleep(std::time::Duration::from_millis(300));
    sys.refresh_cpu_usage();
    let uptime = System::uptime() as f64 / 3600.0;
    let cpus = sys.cpus();
    let cpu_name = cpus.first().map(|c| c.brand().to_string()).unwrap_or_default();
    let cpu_cores = sysinfo::System::physical_core_count().unwrap_or(cpus.len()) as u32;
    let cpu_usage = sys.global_cpu_usage();
    let total = sys.total_memory() as f64 / 1_073_741_824.0;
    let used = sys.used_memory() as f64 / 1_073_741_824.0;
    let pct = if total > 0.0 { (used / total * 100.0) as f32 } else { 0.0 };
    let win_version = format!("{} {}", System::name().unwrap_or_else(|| "Windows".into()), System::os_version().unwrap_or_default());
    SystemSummary { uptime_hours: (uptime * 10.0).round() / 10.0, cpu_name, cpu_cores, cpu_usage, ram_total: total, ram_used: used, ram_pct: pct, win_version }
}

// === Activation ===

pub fn check_windows_activation() -> String {
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            r#"try { $s = Get-WmiObject -Query "SELECT LicenseStatus FROM SoftwareLicensingProduct WHERE PartialProductKey IS NOT NULL AND Name LIKE '*Windows*'" -ErrorAction Stop | Select-Object -First 1; switch($s.LicenseStatus){1{"Activé"}2{"Grâce OOB"}5{"Notification"}default{"Non activé"}} } catch { "Inconnu" }"#])
        .creation_flags(0x08000000).output();
    match out {
        // decode_output : le switch renvoie des libellés accentués hardcodés
        // ("Activé", "Grâce OOB") sans préambule $OutputEncoding.
        Ok(o) => crate::maintenance::commands::decode_output(&o.stdout).trim().to_string(),
        Err(_) => "Inconnu".to_string(),
    }
}

// === Security ===

pub fn check_security() -> (bool, bool) {
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            r#"$fw = try { (Get-NetFirewallProfile -ErrorAction SilentlyContinue | Where-Object {$_.Enabled}).Count -gt 0 } catch { $false }; $def = try { (Get-MpComputerStatus -ErrorAction SilentlyContinue).RealTimeProtectionEnabled } catch { $false }; Write-Output "$fw,$def""#])
        .creation_flags(0x08000000).output();
    if let Ok(o) = out {
        let s = String::from_utf8_lossy(&o.stdout);
        let parts: Vec<&str> = s.trim().splitn(2, ',').collect();
        let fw = parts.first().map(|v| *v == "True").unwrap_or(false);
        let def = parts.get(1).map(|v| *v == "True").unwrap_or(false);
        return (fw, def);
    }
    (false, false)
}

// === Suspicious Processes ===

pub fn scan_suspicious_processes() -> Vec<SuspiciousProcess> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            "Get-Process | Select-Object Id,ProcessName,Path | ConvertTo-Json -Depth 1 -Compress"])
        .creation_flags(0x08000000)
        .output();
    let text = match output { Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(), Err(_) => return vec![] };
    let val: serde_json::Value = match serde_json::from_str(&text) { Ok(v) => v, Err(_) => return vec![] };
    let processes: Vec<serde_json::Value> = match val {
        serde_json::Value::Array(arr) => arr,
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => return vec![],
    };
    let safe_prefixes = [
        r"C:\Windows\System32", r"C:\Windows\SysWOW64", r"C:\Windows\",
        r"C:\Program Files\", r"C:\Program Files (x86)\", r"C:\ProgramData\Microsoft",
    ];
    let mut suspicious = Vec::new();
    for proc in &processes {
        let name = proc["ProcessName"].as_str().unwrap_or("").to_string();
        let pid = proc["Id"].as_u64().unwrap_or(0) as u32;
        let path = proc["Path"].as_str().unwrap_or("").to_string();
        if path.is_empty() || name.is_empty() { continue; }
        let path_up = path.to_uppercase();
        let is_safe = safe_prefixes.iter().any(|p| path_up.starts_with(&p.to_uppercase()));
        if !is_safe {
            let reason = if path_up.contains("TEMP") || path_up.contains("TMP") {
                "Exécuté depuis un dossier temporaire".to_string()
            } else if path_up.contains("APPDATA\\LOCAL") && !path_up.contains("PROGRAMS") {
                "Exécuté depuis AppData (hors Programs)".to_string()
            } else { "Chemin inhabituel".to_string() };
            suspicious.push(SuspiciousProcess { name, pid, path, reason });
        }
    }
    suspicious
}

// === Disk ===

pub fn get_disk_usage() -> Vec<DiskUsage> {
    use sysinfo::Disks;
    Disks::new_with_refreshed_list().iter().map(|d| {
        let total = d.total_space() as f64 / 1_073_741_824.0;
        let free = d.available_space() as f64 / 1_073_741_824.0;
        let used = total - free;
        let pct = if total > 0.0 { (used / total * 100.0) as f32 } else { 0.0 };
        DiskUsage {
            drive: d.mount_point().to_string_lossy().to_string(),
            total_gb: (total * 100.0).round() / 100.0,
            free_gb: (free * 100.0).round() / 100.0,
            used_percent: (pct * 10.0).round() / 10.0,
        }
    }).collect()
}

// === Startup + Reboot ===

pub fn check_startup_and_reboot() -> (usize, bool) {
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            r#"$sc = (Get-CimInstance -ClassName Win32_StartupCommand -ErrorAction SilentlyContinue | Measure-Object).Count; $rb = Test-Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending" -ErrorAction SilentlyContinue; Write-Output "$sc,$rb""#])
        .creation_flags(0x08000000).output();
    if let Ok(o) = out {
        let s = String::from_utf8_lossy(&o.stdout);
        let parts: Vec<&str> = s.trim().splitn(2, ',').collect();
        let sc = parts.first().and_then(|v| v.parse().ok()).unwrap_or(0);
        let rb = parts.get(1).map(|v| *v == "True").unwrap_or(false);
        return (sc, rb);
    }
    (0, false)
}

// === Recent Errors ===

pub fn get_recent_errors() -> Vec<EventEntry> {
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            r#"Get-WinEvent -FilterHashtable @{LogName='System','Application';Level=1,2;StartTime=(Get-Date).AddHours(-48)} -MaxEvents 20 -ErrorAction SilentlyContinue | Select-Object TimeCreated,ProviderName,Message,LevelDisplayName | ConvertTo-Json -Depth 1 -Compress"#])
        .creation_flags(0x08000000).output();
    // decode_output : Message d'event log est pleinement localisé FR.
    let text = match out { Ok(o) => crate::maintenance::commands::decode_output(&o.stdout), Err(_) => return vec![] };
    let val: Vec<serde_json::Value> = serde_json::from_str(text.trim())
        .unwrap_or_else(|_| serde_json::from_str(&format!("[{}]", text.trim())).unwrap_or_default());
    val.iter().map(|v| EventEntry {
        time: v["TimeCreated"].as_str().unwrap_or("").chars().take(16).collect(),
        source: v["ProviderName"].as_str().unwrap_or("").to_string(),
        message: v["Message"].as_str().unwrap_or("").chars().take(120).collect(),
        level: v["LevelDisplayName"].as_str().unwrap_or("").to_string(),
    }).collect()
}

// === Network ===

pub fn test_network() -> bool {
    Command::new("ping").args(["-n", "1", "-w", "2000", "8.8.8.8"])
        .creation_flags(0x08000000).output()
        .map(|o| o.status.success()).unwrap_or(false)
}

pub fn get_listening_ports() -> Vec<u16> {
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            r#"Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Where-Object {$_.LocalAddress -eq '0.0.0.0' -or $_.LocalAddress -eq '::'} | Select-Object -ExpandProperty LocalPort | Sort-Object -Unique | Select-Object -First 30 | ConvertTo-Json -Compress"#])
        .creation_flags(0x08000000).output();
    let text = match out { Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(), Err(_) => return vec![] };
    let val: Vec<serde_json::Value> = serde_json::from_str(text.trim()).unwrap_or_default();
    val.iter().filter_map(|v| v.as_u64().map(|p| p as u16)).collect()
}

// === WinGet / Choco ===

pub fn list_winget_upgradable() -> Vec<WingetUpgrade> {
    // Délègue à installer::winget pour avoir la même source de vérité que DiagTabUpdates
    crate::installer::winget::list_upgradable()
        .unwrap_or_default()
        .into_iter()
        .map(|p| WingetUpgrade {
            name: p.name,
            id: p.id,
            current_version: p.version,
            available_version: p.available,
        })
        .collect()
}

pub fn is_chocolatey_installed() -> bool {
    Command::new("choco").arg("--version").creation_flags(0x08000000).output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn list_choco_upgradable() -> Vec<String> {
    // Délègue à installer::chocolatey pour avoir la même source de vérité que DiagTabUpdates
    crate::installer::chocolatey::list_chocolatey_upgrades()
        .unwrap_or_default()
        .into_iter()
        .map(|p| format!("{}  {}  →  {}", p.name, p.current_version, p.available_version))
        .collect()
}

pub fn is_scoop_installed() -> bool {
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            "if (Get-Command scoop -ErrorAction SilentlyContinue) { 'true' } else { 'false' }"])
        .creation_flags(0x08000000).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "true")
        .unwrap_or(false)
}

pub fn list_scoop_upgradable() -> Vec<String> {
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            r#"try { $lines = scoop status 2>&1; $res = @($lines | Where-Object { $_ -is [string] -and $_.Trim() -ne '' -and $_ -notmatch '^Name|^----|\s+Installed Version' } | Select-Object -First 25 | ForEach-Object { [string]$_.Trim() }); if ($res.Count -gt 0) { $res | ConvertTo-Json -Compress } else { '[]' } } catch { '[]' }"#])
        .creation_flags(0x08000000).output();
    let text = match out { Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(), Err(_) => return vec![] };
    let val: Vec<serde_json::Value> = serde_json::from_str(text.trim()).unwrap_or_default();
    val.iter().filter_map(|v| v.as_str().filter(|s| !s.is_empty()).map(|s| s.to_string())).collect()
}

// === DISM / SFC ===

pub fn run_dism_check() -> (String, String) {
    let output = Command::new("DISM").args(["/Online", "/Cleanup-Image", "/CheckHealth"]).creation_flags(0x08000000).output();
    match output {
        Ok(o) => {
            // decode_output : la sortie DISM est pleinement localisée FR
            // ("détectée", "n'a été détectée"...).
            let text = crate::maintenance::commands::decode_output(&o.stdout);
            let status = if text.contains("No component store corruption") || text.to_lowercase().contains("aucune corruption") {
                "Sain — Aucune corruption détectée".to_string()
            } else if !o.status.success() {
                "Avertissement — Vérification requise".to_string()
            } else { "OK".to_string() };
            let details: String = text.lines()
                .filter(|l| !l.trim().is_empty())
                .take(40)
                .collect::<Vec<_>>()
                .join("\n");
            (status, details)
        }
        Err(_) => ("DISM non disponible".to_string(), String::new()),
    }
}

// Classification pure et testable de la sortie `sfc /verifyonly`. Extraite de
// run_sfc_verify() pour pouvoir vérifier les 2 langues sans relancer sfc (lent,
// plusieurs minutes) à chaque test. Sortie FR confirmée en exécution réelle
// (2026-08-05, sfc /VERIFYFILE) : "...n'a trouvé aucune violation d'intégrité." —
// le check anglais seul ("no integrity violations") ne matche jamais sur un
// Windows FR (100% du public cible de l'app), même piège que le check winget
// upgrade corrigé au cycle 55.
fn classify_sfc_status(text: &str, exit_success: bool) -> String {
    let low = text.to_lowercase();
    if low.contains("no integrity violations") || low.contains("aucune violation d") {
        "Intègre — Aucune violation détectée".to_string()
    } else if low.contains("found corrupt") || low.contains("fichiers corrompus") {
        "Fichiers corrompus détectés".to_string()
    } else if !exit_success {
        "Avertissement — Relancer en administrateur".to_string()
    } else {
        "Vérification complète".to_string()
    }
}

pub fn run_sfc_verify() -> (String, String) {
    let output = Command::new("sfc").args(["/verifyonly"]).creation_flags(0x08000000).output();
    match output {
        Ok(o) => {
            // sfc emet parfois en UTF-16 (detecte via octets nuls), sinon en OEM
            // (decode_output) plutot que from_utf8_lossy brut — la sortie FR
            // ("Aucune violation d'integrite...") est localisee.
            let raw = String::from_utf8_lossy(&o.stdout).to_string();
            let text = if raw.contains('\0') {
                let bytes = o.stdout.clone();
                let utf16: Vec<u16> = bytes.chunks(2).map(|c| u16::from_le_bytes([c[0], c.get(1).copied().unwrap_or(0)])).collect();
                String::from_utf16_lossy(&utf16)
            } else { crate::maintenance::commands::decode_output(&o.stdout) };
            let status = classify_sfc_status(&text, o.status.success());
            let details: String = text.lines()
                .filter(|l| !l.trim().is_empty())
                .take(40)
                .collect::<Vec<_>>()
                .join("\n");
            (status, details)
        }
        Err(_) => ("SFC non disponible".to_string(), String::new()),
    }
}

#[cfg(test)]
mod sfc_status_tests {
    use super::classify_sfc_status;

    #[test]
    fn real_french_clean_scan_detected() {
        // Sortie réelle capturée en direct (sfc /VERIFYFILE, 2026-08-05).
        let text = "Le programme de protection des ressources Windows n'a trouvé aucune violation d'intégrité.";
        assert_eq!(classify_sfc_status(text, true), "Intègre — Aucune violation détectée");
    }

    #[test]
    fn english_clean_scan_still_detected() {
        let text = "Windows Resource Protection did not find any integrity violations. no integrity violations found";
        assert_eq!(classify_sfc_status(text, true), "Intègre — Aucune violation détectée");
    }

    #[test]
    fn french_corrupt_files_detected() {
        let text = "Le programme de protection des ressources Windows a trouvé des fichiers corrompus.";
        assert_eq!(classify_sfc_status(text, true), "Fichiers corrompus détectés");
    }

    #[test]
    fn nonzero_exit_without_recognized_text_is_warning() {
        assert_eq!(classify_sfc_status("erreur inattendue", false), "Avertissement — Relancer en administrateur");
    }

    #[test]
    fn success_with_unrecognized_text_is_generic() {
        assert_eq!(classify_sfc_status("sortie inattendue", true), "Vérification complète");
    }
}
