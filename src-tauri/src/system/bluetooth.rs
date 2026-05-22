use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Default)]
pub struct BluetoothDevice {
    pub name: String,
    pub address: String,
    pub device_class: String,
    pub paired: bool,
    pub connected: bool,
    pub trusted: bool,
    pub rssi: i32,
    pub manufacturer: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct BluetoothAdapter {
    pub name: String,
    pub address: String,
    pub enabled: bool,
    pub manufacturer: String,
    pub driver_version: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct BluetoothReport {
    pub adapters: Vec<BluetoothAdapter>,
    pub devices: Vec<BluetoothDevice>,
    pub bt_available: bool,
    pub error: String,
}

#[tauri::command]
pub fn get_bluetooth_info() -> BluetoothReport {
    let ps = r#"
try {
    # Get Bluetooth adapters via WMI
    $adapters = @(Get-WmiObject Win32_PnPEntity -EA SilentlyContinue |
        Where-Object { $_.Name -match 'Bluetooth' -and $_.PNPClass -eq 'Bluetooth' } |
        ForEach-Object {
            @{
                name    = $_.Name
                address = ''
                enabled = ($_.ConfigManagerErrorCode -eq 0)
                mfr     = $_.Manufacturer
                driver  = ''
            }
        })

    # Also check via DeviceClass for radio adapters
    $radios = @(Get-WmiObject Win32_PnPEntity -EA SilentlyContinue |
        Where-Object { $_.PNPDeviceID -match '^BTH\\' -or $_.PNPDeviceID -match 'BTHENUM' } |
        ForEach-Object {
            @{
                name    = $_.Name
                address = $_.PNPDeviceID
                enabled = ($_.ConfigManagerErrorCode -eq 0)
                mfr     = if($_.Manufacturer){$_.Manufacturer}else{''}
                driver  = ''
            }
        })

    if ($adapters.Count -eq 0) { $adapters = $radios }

    # Get paired/connected BT devices
    $devices = @(Get-WmiObject Win32_PnPEntity -EA SilentlyContinue |
        Where-Object {
            ($_.PNPDeviceID -match '^BTHENUM\\' -or $_.PNPDeviceID -match '^BTH\\') -and
            $_.Name -notmatch 'Enumerator|Generic|Radio|Adapter|Host'
        } |
        ForEach-Object {
            $cat = switch -Wildcard ($_.Name) {
                '*headphone*' { 'Audio' }
                '*earphone*'  { 'Audio' }
                '*speaker*'   { 'Audio' }
                '*headset*'   { 'Audio' }
                '*mouse*'     { 'Souris' }
                '*keyboard*'  { 'Clavier' }
                '*gamepad*'   { 'Manette' }
                '*controller*'{ 'Manette' }
                '*phone*'     { 'Téléphone' }
                default       { 'Autre' }
            }
            @{
                name      = $_.Name
                address   = $_.PNPDeviceID -replace '.*&(\w+)_\w+$','$1'
                class     = if($_.PNPClass){$_.PNPClass}else{'Unknown'}
                paired    = $true
                connected = ($_.ConfigManagerErrorCode -eq 0)
                trusted   = $true
                rssi      = 0
                mfr       = if($_.Manufacturer){$_.Manufacturer}else{''}
                cat       = $cat
            }
        })

    $btAvail = ($adapters.Count -gt 0 -or $radios.Count -gt 0)
    @{ adapters=$adapters; devices=$devices; available=$btAvail; error='' } | ConvertTo-Json -Depth 4 -Compress
} catch {
    @{ adapters=@(); devices=@(); available=$false; error=$_.Exception.Message } | ConvertTo-Json -Compress
}
"#;
    #[cfg(target_os = "windows")]
    {
        let ps_utf8 = format!("$OutputEncoding = [System.Text.Encoding]::UTF8; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; {}", ps);
        let o = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_utf8])
            .creation_flags(0x08000000)
            .output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(t.trim()) {
                let adapters = v["adapters"].as_array().map(|arr| {
                    arr.iter().map(|a| BluetoothAdapter {
                        name: a["name"].as_str().unwrap_or("").to_string(),
                        address: a["address"].as_str().unwrap_or("").to_string(),
                        enabled: a["enabled"].as_bool().unwrap_or(false),
                        manufacturer: a["mfr"].as_str().unwrap_or("").to_string(),
                        driver_version: a["driver"].as_str().unwrap_or("").to_string(),
                    }).collect()
                }).unwrap_or_default();

                let devices = v["devices"].as_array().map(|arr| {
                    arr.iter().map(|d| BluetoothDevice {
                        name: d["name"].as_str().unwrap_or("").to_string(),
                        address: d["address"].as_str().unwrap_or("").to_string(),
                        device_class: d["class"].as_str().unwrap_or("").to_string(),
                        paired: d["paired"].as_bool().unwrap_or(false),
                        connected: d["connected"].as_bool().unwrap_or(false),
                        trusted: d["trusted"].as_bool().unwrap_or(false),
                        rssi: d["rssi"].as_i64().unwrap_or(0) as i32,
                        manufacturer: d["mfr"].as_str().unwrap_or("").to_string(),
                        category: d["cat"].as_str().unwrap_or("Autre").to_string(),
                    }).collect()
                }).unwrap_or_default();

                return BluetoothReport {
                    adapters,
                    devices,
                    bt_available: v["available"].as_bool().unwrap_or(false),
                    error: v["error"].as_str().unwrap_or("").to_string(),
                };
            }
        }
    }
    BluetoothReport { error: "Erreur lecture Bluetooth".to_string(), ..Default::default() }
}

#[tauri::command]
pub fn toggle_bluetooth(enable: bool) -> Result<String, String> {
    let action = if enable {
        "Start-Service bthserv -EA SilentlyContinue; Enable-PnpDevice -InstanceId (Get-PnpDevice | Where-Object {$_.Class -eq 'Bluetooth' -and $_.Status -ne 'OK'} | Select-Object -First 1 -ExpandProperty InstanceId) -Confirm:$false -EA SilentlyContinue"
    } else {
        "Stop-Service bthserv -Force -EA SilentlyContinue"
    };
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", action])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| e.to_string())?;
        let msg = if enable { "Bluetooth activé" } else { "Bluetooth désactivé" };
        if o.status.success() {
            return Ok(msg.to_string());
        }
        Err(String::from_utf8_lossy(&o.stderr).to_string())
    }
    #[cfg(not(target_os = "windows"))]
    Err("Non disponible".to_string())
}
