use serde::Serialize;
use std::process::Command;
use wmi::{COMLibrary, WMIConnection};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::error::NiTriTeError;

// === Types ===

#[derive(Debug, Clone, Serialize)]
pub struct BiosInfo {
    pub manufacturer: String,
    pub version: String,
    pub release_date: String,
    pub serial_number: String,
    pub smbios_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatteryInfo {
    pub name: String,
    pub status: String,
    pub estimated_charge_remaining: u32,
    pub design_capacity: u32,
    pub full_charge_capacity: u32,
    pub health_percent: f64,
    pub is_plugged: bool,
    pub estimated_runtime_minutes: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderSizeEntry {
    pub label: String,
    pub path: String,
    pub size_mb: f64,
    pub size_gb: f64,
    pub file_count: u64,
}

// === WMI structs ===

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct Win32Bios {
    Manufacturer: Option<String>,
    Name: Option<String>,
    Version: Option<String>,
    ReleaseDate: Option<String>,
    SerialNumber: Option<String>,
    SMBIOSMajorVersion: Option<u8>,
    SMBIOSMinorVersion: Option<u8>,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct Win32Battery {
    Name: Option<String>,
    BatteryStatus: Option<u16>,
    EstimatedChargeRemaining: Option<u16>,
    DesignCapacity: Option<u32>,
    FullChargeCapacity: Option<u32>,
    EstimatedRunTime: Option<u32>,
}

// === BIOS ===

pub fn get_bios_info() -> Result<BiosInfo, NiTriTeError> {
    let com = COMLibrary::new().map_err(|e| NiTriTeError::Wmi(e.to_string()))?;
    let wmi = WMIConnection::new(com).map_err(|e| NiTriTeError::Wmi(e.to_string()))?;

    let results: Vec<Win32Bios> = wmi.raw_query("SELECT * FROM Win32_BIOS").map_err(|e| NiTriTeError::Wmi(e.to_string()))?;
    let bios = results
        .into_iter()
        .next()
        .ok_or_else(|| NiTriTeError::System("BIOS non detecte".into()))?;

    let release_date = bios
        .ReleaseDate
        .as_deref()
        .map(parse_wmi_date)
        .unwrap_or_default();

    let smbios = format!(
        "{}.{}",
        bios.SMBIOSMajorVersion.unwrap_or(0),
        bios.SMBIOSMinorVersion.unwrap_or(0)
    );

    Ok(BiosInfo {
        manufacturer: bios.Manufacturer.unwrap_or_default().trim().to_string(),
        version: bios
            .Name
            .or(bios.Version)
            .unwrap_or_default()
            .trim()
            .to_string(),
        release_date,
        serial_number: bios.SerialNumber.unwrap_or_default().trim().to_string(),
        smbios_version: smbios,
    })
}

fn parse_wmi_date(s: &str) -> String {
    // Format WMI : "20230115000000.000000+000"
    if s.len() >= 8 {
        let y = &s[0..4];
        let m = &s[4..6];
        let d = &s[6..8];
        format!("{}/{}/{}", d, m, y)
    } else {
        s.to_string()
    }
}

// === Battery ===

pub fn get_battery_extended() -> Result<Option<BatteryInfo>, NiTriTeError> {
    let com = COMLibrary::new().map_err(|e| NiTriTeError::Wmi(e.to_string()))?;
    let wmi = WMIConnection::new(com).map_err(|e| NiTriTeError::Wmi(e.to_string()))?;

    let results: Vec<Win32Battery> = wmi.raw_query("SELECT * FROM Win32_Battery").map_err(|e| NiTriTeError::Wmi(e.to_string()))?;
    let bat = match results.into_iter().next() {
        Some(b) => b,
        None => return Ok(None),
    };

    let design = bat.DesignCapacity.unwrap_or(0);
    let full = bat.FullChargeCapacity.unwrap_or(0);
    let health = if design > 0 {
        (full as f64 / design as f64) * 100.0
    } else {
        100.0
    };

    // BatteryStatus: 1=Discharging, 2=AC, 3=Fully Charged
    let status_code = bat.BatteryStatus.unwrap_or(0);
    let (status, is_plugged) = match status_code {
        1 => ("Décharge", false),
        2 => ("Branchée", true),
        3 => ("Chargée", true),
        4 => ("Faible", false),
        5 => ("Critique", false),
        6 => ("Charge", true),
        7 => ("Charge haute", true),
        _ => ("Inconnu", false),
    };

    let runtime = bat.EstimatedRunTime.unwrap_or(0);
    let runtime_minutes = if runtime == 71582788 || runtime == 0 {
        -1i64 // indisponible
    } else {
        runtime as i64
    };

    Ok(Some(BatteryInfo {
        name: bat.Name.unwrap_or_else(|| "Batterie".to_string()).trim().to_string(),
        status: status.to_string(),
        estimated_charge_remaining: bat.EstimatedChargeRemaining.unwrap_or(0) as u32,
        design_capacity: design,
        full_charge_capacity: full,
        health_percent: (health * 10.0).round() / 10.0,
        is_plugged,
        estimated_runtime_minutes: runtime_minutes,
    }))
}

// === Folder sizes ===

pub fn get_folder_sizes() -> Result<Vec<FolderSizeEntry>, NiTriTeError> {
    let entries = vec![
        ("Bureau", "%USERPROFILE%\\Desktop"),
        ("Documents", "%USERPROFILE%\\Documents"),
        ("Téléchargements", "%USERPROFILE%\\Downloads"),
        ("AppData Roaming", "%APPDATA%"),
        ("AppData Local", "%LOCALAPPDATA%"),
        ("Temp", "%TEMP%"),
        ("Profil utilisateur", "%USERPROFILE%"),
        ("Program Files", "%PROGRAMFILES%"),
        ("Program Files (x86)", "%PROGRAMFILES(X86)%"),
    ];

    let mut result = Vec::with_capacity(entries.len());

    for (label, path_env) in &entries {
        let expanded = expand_env_path(path_env);
        let (size_mb, file_count) = measure_folder(&expanded);
        result.push(FolderSizeEntry {
            label: label.to_string(),
            path: expanded,
            size_mb,
            size_gb: (size_mb / 1024.0 * 100.0).round() / 100.0,
            file_count,
        });
    }

    Ok(result)
}

/// Expansion des variables `%VAR%` en pur Rust (sans spawn de `cmd`).
/// Une variable absente est laissée telle quelle, comme le fait `cmd echo`.
fn expand_env_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut rest = path;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(val) => out.push_str(&val),
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn measure_folder(path: &str) -> (f64, u64) {
    let script = format!(
        r#"try {{ $f=Get-ChildItem -LiteralPath '{}' -Recurse -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum; Write-Output "$($f.Sum),$($f.Count)" }} catch {{ Write-Output '0,0' }}"#,
        path.replace('\'', "''")
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(0x08000000)
        .output();

    match output {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let parts: Vec<&str> = s.splitn(2, ',').collect();
            let bytes: f64 = parts.first().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let count: u64 = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);
            let mb = (bytes / 1_048_576.0 * 100.0).round() / 100.0;
            (mb, count)
        }
        Err(_) => (0.0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::expand_env_path;

    #[test]
    fn expands_known_var() {
        std::env::set_var("NITRITE_TEST_VAR", "C:\\Data");
        assert_eq!(expand_env_path("%NITRITE_TEST_VAR%\\x"), "C:\\Data\\x");
    }

    #[test]
    fn keeps_unknown_var_literal() {
        assert_eq!(expand_env_path("%NOPE_UNSET_123%\\y"), "%NOPE_UNSET_123%\\y");
    }

    #[test]
    fn lone_percent_is_preserved() {
        assert_eq!(expand_env_path("50% done"), "50% done");
    }
}
