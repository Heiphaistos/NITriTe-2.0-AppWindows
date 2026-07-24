use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Default)]
pub struct PnpDriver {
    pub name: String,
    pub provider: String,
    pub version: String,
    pub date: String,
    pub class: String,
    pub inf: String,
    pub signed: bool,
    pub status: String,
    pub config_error: u32,
}

#[derive(Debug, Default, Serialize)]
pub struct SysDriversData {
    pub drivers: Vec<PnpDriver>,
    pub total: u32,
    pub unsigned_count: u32,
    pub error_count: u32,
}

// Anti-freeze : Get-WmiObject via PowerShell est bloquant — jamais inline sur
// le thread de commande.
#[tauri::command]
pub async fn get_sys_drivers_list() -> SysDriversData {
    tokio::task::spawn_blocking(get_sys_drivers_list_blocking)
        .await
        .unwrap_or_default()
}

fn get_sys_drivers_list_blocking() -> SysDriversData {
    let ps = r#"
# Sortie UTF-8 : les noms de périphériques/fabricants accentués (FR
# « Périphérique système »…) seraient sinon mojibake côté Rust (from_utf8_lossy).
$OutputEncoding = [System.Text.Encoding]::UTF8; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$out = @{}
try {
    $drv = Get-WmiObject Win32_PnPSignedDriver -ErrorAction SilentlyContinue |
        Where-Object { $_.DeviceName -and $_.DeviceName.Trim() -ne '' }
    $list = @($drv | ForEach-Object {
        @{
            name     = [string]$_.DeviceName
            provider = [string]$_.Manufacturer
            version  = [string]$_.DriverVersion
            date     = [string]$_.DriverDate
            class    = [string]$_.DeviceClass
            inf      = [string]$_.InfName
            signed   = ($_.IsSigned -eq $true)
            status   = [string]$_.Status
            err      = [int]$_.ConfigManagerErrorCode
        }
    })
    $out.Drivers    = $list
    $out.Total      = [int]$list.Count
    $out.UnsignedCt = [int]($list | Where-Object { -not $_['signed'] }).Count
    $out.ErrorCt    = [int]($list | Where-Object { $_['err'] -ne 0 }).Count
} catch {
    $out.Drivers    = @()
    $out.Total      = 0
    $out.UnsignedCt = 0
    $out.ErrorCt    = 0
}
$out | ConvertTo-Json -Depth 4 -Compress
"#;

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output();

        if let Ok(o) = output {
            let text = String::from_utf8_lossy(&o.stdout);
            let v: serde_json::Value = match serde_json::from_str(text.trim()) {
                Ok(v) => v,
                Err(_) => return SysDriversData::default(),
            };

            let drivers = v["Drivers"].as_array().map(|arr| {
                arr.iter().map(|d| PnpDriver {
                    name: d["name"].as_str().unwrap_or("").to_string(),
                    provider: d["provider"].as_str().unwrap_or("").to_string(),
                    version: d["version"].as_str().unwrap_or("").to_string(),
                    date: d["date"].as_str().unwrap_or("").to_string(),
                    class: d["class"].as_str().unwrap_or("").to_string(),
                    inf: d["inf"].as_str().unwrap_or("").to_string(),
                    signed: d["signed"].as_bool().unwrap_or(true),
                    status: d["status"].as_str().unwrap_or("").to_string(),
                    config_error: d["err"].as_u64().unwrap_or(0) as u32,
                }).collect()
            }).unwrap_or_default();

            return SysDriversData {
                total: v["Total"].as_u64().unwrap_or(0) as u32,
                unsigned_count: v["UnsignedCt"].as_u64().unwrap_or(0) as u32,
                error_count: v["ErrorCt"].as_u64().unwrap_or(0) as u32,
                drivers,
            };
        }
    }
    SysDriversData::default()
}
