use serde::Serialize;
use wmi::{COMLibrary, WMIConnection};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// ============================================================================
// Result Structs
// ============================================================================

#[derive(Debug, Serialize, Clone)]
pub struct MotherboardDetailed {
    pub manufacturer: String, pub product: String, pub serial_number: String,
    pub version: String, pub status: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct GpuDetailed {
    pub name: String, pub adapter_ram_mb: u64, pub driver_version: String,
    pub driver_date: String, pub video_processor: String, pub video_mode: String,
    pub current_resolution: String, pub current_refresh_rate: u32,
    pub status: String, pub pnp_device_id: String, pub adapter_dac_type: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct RamSlot {
    pub bank_label: String, pub device_locator: String, pub manufacturer: String,
    pub capacity_gb: f64, pub speed_mhz: u32, pub configured_speed_mhz: u32,
    pub memory_type: String, pub form_factor: String,
    pub serial_number: String, pub part_number: String, pub data_width: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct RamDetailed {
    pub total_slots: usize, pub used_slots: usize, pub total_capacity_gb: f64,
    pub slots: Vec<RamSlot>,
}

#[derive(Debug, Serialize, Clone)]
pub struct AudioDevice { pub name: String, pub manufacturer: String, pub status: String, pub device_id: String }

#[derive(Debug, Serialize, Clone)]
pub struct UsbDevice { pub name: String, pub device_id: String, pub manufacturer: String, pub status: String, pub pnp_class: String }

#[derive(Debug, Serialize, Clone)]
pub struct BatteryDetailed {
    pub name: String, pub status: String, pub estimated_charge_remaining: u32,
    pub estimated_run_time: String, pub design_capacity: u32,
    pub full_charge_capacity: u32, pub battery_health_percent: f32,
    pub chemistry: String, pub cycle_count: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct MonitorDetail {
    pub name: String, pub screen_width: u32, pub screen_height: u32,
    pub pixels_per_inch: u32, pub manufacturer: String, pub availability: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct PowerPlan { pub name: String, pub is_active: bool, pub guid: String }

#[derive(Debug, Serialize, Clone)]
pub struct PrinterDetail {
    pub name: String, pub driver_name: String, pub port_name: String,
    pub is_default: bool, pub is_network: bool, pub status: String, pub shared: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct EnvVar { pub name: String, pub value: String, pub var_type: String }

#[derive(Debug, Serialize, Clone)]
pub struct WindowsLicense {
    pub product_name: String, pub activation_status: String,
    pub partial_product_key: String, pub full_product_key: String,
    pub license_status: String, pub license_family: String,
    pub office_name: String, pub office_status: String, pub office_key: String,
    pub office_full_key: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct InstalledUpdate {
    pub title: String, pub hotfix_id: String, pub description: String,
    pub installed_on: String, pub installed_by: String,
}

// ============================================================================
// WMI typed query structs
// ============================================================================

#[derive(serde::Deserialize)] #[allow(non_snake_case)]
struct WmiBaseBoard { Manufacturer: Option<String>, Product: Option<String>, SerialNumber: Option<String>, Version: Option<String>, Status: Option<String> }

#[derive(serde::Deserialize)] #[allow(non_snake_case)]
struct WmiVideoController {
    Name: Option<String>, AdapterRAM: Option<u64>, DriverVersion: Option<String>,
    DriverDate: Option<String>, VideoProcessor: Option<String>,
    VideoModeDescription: Option<String>, CurrentHorizontalResolution: Option<u32>,
    CurrentVerticalResolution: Option<u32>, CurrentRefreshRate: Option<u32>,
    Status: Option<String>, PNPDeviceID: Option<String>, AdapterDACType: Option<String>,
}

#[derive(serde::Deserialize)] #[allow(non_snake_case)]
struct WmiPhysicalMemory {
    BankLabel: Option<String>, DeviceLocator: Option<String>, Manufacturer: Option<String>,
    Capacity: Option<u64>, Speed: Option<u32>, ConfiguredClockSpeed: Option<u32>,
    MemoryType: Option<u32>, FormFactor: Option<u32>,
    SerialNumber: Option<String>, PartNumber: Option<String>, DataWidth: Option<u32>,
}

#[derive(serde::Deserialize)] #[allow(non_snake_case)]
struct WmiMemoryArray { MemoryDevices: Option<u32> }

#[derive(serde::Deserialize)] #[allow(non_snake_case)]
struct WmiSoundDevice { Name: Option<String>, Manufacturer: Option<String>, Status: Option<String>, DeviceID: Option<String> }

#[derive(serde::Deserialize)] #[allow(non_snake_case)]
struct WmiPnPEntity { Name: Option<String>, DeviceID: Option<String>, Manufacturer: Option<String>, Status: Option<String>, PNPClass: Option<String> }

#[derive(serde::Deserialize)] #[allow(non_snake_case, dead_code)]
struct WmiDesktopMonitor { Name: Option<String>, ScreenWidth: Option<u32>, ScreenHeight: Option<u32>, PixelsPerXLogicalInch: Option<u32>, MonitorManufacturer: Option<String>, Availability: Option<u32> }

#[derive(serde::Deserialize)] #[allow(non_snake_case)]
struct WmiPrinter { Name: Option<String>, DriverName: Option<String>, PortName: Option<String>, Default: Option<bool>, Network: Option<bool>, Status: Option<String>, Shared: Option<bool> }

#[derive(serde::Deserialize)] #[allow(non_snake_case)]
struct WmiQuickFix { Caption: Option<String>, HotFixID: Option<String>, Description: Option<String>, InstalledOn: Option<String>, InstalledBy: Option<String> }

// ============================================================================
// Helpers
// ============================================================================

fn wmi_con() -> Result<WMIConnection, String> {
    let com = COMLibrary::new().map_err(|e| format!("COM: {}", e))?;
    WMIConnection::new(com).map_err(|e| format!("WMI: {}", e))
}

fn fmt_date(s: &str) -> String {
    if s.len() >= 8 && s[..8].chars().all(|c| c.is_ascii_digit()) {
        format!("{}/{}/{}", &s[6..8], &s[4..6], &s[0..4])
    } else { s.to_string() }
}

fn mem_type_str(v: u32) -> &'static str {
    match v { 20 => "DDR", 21 => "DDR2", 24 => "DDR3", 26 => "DDR4", 30 | 34 => "DDR5", _ => "Unknown" }
}

fn form_factor_str(v: u32) -> &'static str {
    match v { 8 => "DIMM", 12 => "SO-DIMM", _ => "Unknown" }
}

fn nvidia_vram_map() -> std::collections::HashMap<String, u64> {
    let mut map = std::collections::HashMap::new();
    #[cfg(target_os = "windows")]
    {
        let Ok(out) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])
            .creation_flags(0x08000000).output()
        else { return map; };
        if !out.status.success() { return map; }
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let mut p = line.splitn(2, ',');
            if let (Some(n), Some(m)) = (p.next(), p.next()) {
                if let Ok(mb) = m.trim().parse::<u64>() { map.insert(n.trim().to_string(), mb); }
            }
        }
    }
    map
}

// ============================================================================
// Commands
// ============================================================================

#[tauri::command]
pub async fn get_motherboard_detailed() -> Result<MotherboardDetailed, String> {
    tokio::task::spawn_blocking(|| {
        let wmi = wmi_con()?;
        let r: Vec<WmiBaseBoard> = wmi.raw_query("SELECT * FROM Win32_BaseBoard").map_err(|e| e.to_string())?;
        let b = r.into_iter().next().ok_or("Aucune carte mère")?;
        Ok(MotherboardDetailed {
            manufacturer: b.Manufacturer.unwrap_or_default().trim().to_string(),
            product: b.Product.unwrap_or_default().trim().to_string(),
            serial_number: b.SerialNumber.unwrap_or_default().trim().to_string(),
            version: b.Version.unwrap_or_default().trim().to_string(),
            status: b.Status.unwrap_or_default(),
        })
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_gpu_detailed() -> Result<Vec<GpuDetailed>, String> {
    tokio::task::spawn_blocking(|| {
        let wmi = wmi_con()?;
        let r: Vec<WmiVideoController> = wmi.raw_query("SELECT * FROM Win32_VideoController").map_err(|e| e.to_string())?;
        let nv = nvidia_vram_map();
        let mut gpus: Vec<GpuDetailed> = r.into_iter().map(|g| {
            let name = g.Name.unwrap_or_default().trim().to_string();
            let h = g.CurrentHorizontalResolution.unwrap_or(0);
            let v = g.CurrentVerticalResolution.unwrap_or(0);
            let resolution = if h > 0 && v > 0 { format!("{}x{}", h, v) } else { "N/A".to_string() };
            let wmi_mb = g.AdapterRAM.unwrap_or(0) / (1024 * 1024);
            let adapter_ram_mb = nv.get(&name).copied().unwrap_or(wmi_mb);
            GpuDetailed {
                adapter_ram_mb,
                driver_date: fmt_date(&g.DriverDate.unwrap_or_default()),
                driver_version: g.DriverVersion.unwrap_or_default(),
                video_processor: g.VideoProcessor.unwrap_or_default(),
                video_mode: g.VideoModeDescription.unwrap_or_default(),
                current_resolution: resolution,
                current_refresh_rate: g.CurrentRefreshRate.unwrap_or(0),
                status: g.Status.unwrap_or_default(),
                pnp_device_id: g.PNPDeviceID.unwrap_or_default(),
                adapter_dac_type: g.AdapterDACType.unwrap_or_default(),
                name,
            }
        }).collect();
        gpus.sort_by_key(|b| std::cmp::Reverse(!b.name.to_lowercase().contains("intel") ));
        Ok(gpus)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_ram_detailed() -> Result<RamDetailed, String> {
    tokio::task::spawn_blocking(|| {
        let wmi = wmi_con()?;
        let r: Vec<WmiPhysicalMemory> = wmi.raw_query("SELECT * FROM Win32_PhysicalMemory").map_err(|e| e.to_string())?;
        let mut total = 0.0f64;
        let slots: Vec<RamSlot> = r.into_iter().map(|m| {
            let cap = m.Capacity.unwrap_or(0) as f64 / 1_073_741_824.0;
            total += cap;
            RamSlot {
                bank_label: m.BankLabel.unwrap_or_default(),
                device_locator: m.DeviceLocator.unwrap_or_default(),
                manufacturer: m.Manufacturer.unwrap_or_default().trim().to_string(),
                capacity_gb: cap,
                speed_mhz: m.Speed.unwrap_or(0),
                configured_speed_mhz: m.ConfiguredClockSpeed.unwrap_or(0),
                memory_type: mem_type_str(m.MemoryType.unwrap_or(0)).to_string(),
                form_factor: form_factor_str(m.FormFactor.unwrap_or(0)).to_string(),
                serial_number: m.SerialNumber.unwrap_or_default(),
                part_number: m.PartNumber.unwrap_or_default().trim().to_string(),
                data_width: m.DataWidth.unwrap_or(0),
            }
        }).collect();
        let used = slots.len();
        // Get total slots: PowerShell for accuracy (BIOS may report theoretical max vs real count)
        let total_slots = {
            let ps_count: Option<usize> = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command",
                    "(Get-WmiObject Win32_PhysicalMemoryArray | Measure-Object -Property MemoryDevices -Sum).Sum"])
                .creation_flags(0x08000000)
                .output()
                .ok()
                .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<usize>().ok());
            let raw = ps_count.unwrap_or_else(|| wmi_con().ok()
                .and_then(|w| w.raw_query::<WmiMemoryArray>("SELECT * FROM Win32_PhysicalMemoryArray").ok())
                .map(|v| v.into_iter().filter_map(|m| m.MemoryDevices).map(|n| n as usize).sum::<usize>())
                .unwrap_or(used)).max(used);
            // Le BIOS rapporte parfois le max du chipset (ex: 8) au lieu des slots physiques réels.
            // Si le total dépasse 2× les slots occupés sur un système avec ≤4 sticks, on corrige.
            if raw > used * 2 && used > 0 && used <= 4 { used } else { raw }
        };
        Ok(RamDetailed { total_slots, used_slots: used, total_capacity_gb: total, slots })
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    tokio::task::spawn_blocking(|| {
        let wmi = wmi_con()?;
        let r: Vec<WmiSoundDevice> = wmi.raw_query("SELECT * FROM Win32_SoundDevice").map_err(|e| e.to_string())?;
        Ok(r.into_iter().map(|d| AudioDevice {
            name: d.Name.unwrap_or_default(),
            manufacturer: d.Manufacturer.unwrap_or_default(),
            status: d.Status.unwrap_or_default(),
            device_id: d.DeviceID.unwrap_or_default(),
        }).collect())
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_usb_devices() -> Result<Vec<UsbDevice>, String> {
    tokio::task::spawn_blocking(|| {
        let wmi = wmi_con()?;
        // Query all PnP entities and filter by DeviceID starting with USB
        let r: Vec<WmiPnPEntity> = wmi.raw_query("SELECT * FROM Win32_PnPEntity WHERE DeviceID LIKE 'USB%'").map_err(|e| e.to_string())?;
        Ok(r.into_iter()
            .filter(|d| !d.Name.as_deref().unwrap_or("").is_empty())
            .map(|d| UsbDevice {
                name: d.Name.unwrap_or_default(),
                device_id: d.DeviceID.unwrap_or_default(),
                manufacturer: d.Manufacturer.unwrap_or_default(),
                status: d.Status.unwrap_or_default(),
                pnp_class: d.PNPClass.unwrap_or_default(),
            }).collect())
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_battery_detailed() -> Result<Vec<BatteryDetailed>, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let ps = r#"
$results = @()
# Capacités réelles depuis namespace root\WMI
$designCap = 0; $fullCap = 0; $cycleCount = 0
try {
    $static = Get-WmiObject -Namespace "root\WMI" -Class "BatteryStaticData" -ErrorAction SilentlyContinue
    if ($static) { $designCap = [int]$static.DesignedCapacity }
    $full = Get-WmiObject -Namespace "root\WMI" -Class "BatteryFullChargedCapacity" -ErrorAction SilentlyContinue
    if ($full) { $fullCap = [int]$full.FullChargedCapacity }
    $cyc = Get-WmiObject -Namespace "root\WMI" -Class "BatteryCycleCount" -ErrorAction SilentlyContinue
    if ($cyc) { $cycleCount = [int]$cyc.CycleCount }
} catch {}
$batteries = Get-WmiObject -Class Win32_Battery -ErrorAction SilentlyContinue
if ($batteries) {
    foreach ($b in @($batteries)) {
        $dc = if ($designCap -gt 0) { $designCap } else { [int]$b.DesignCapacity }
        $fc = if ($fullCap -gt 0) { $fullCap } else { [int]$b.FullChargeCapacity }
        $health = if ($dc -gt 0 -and $fc -gt 0) { [math]::Round(($fc / $dc) * 100, 1) } elseif ($dc -gt 0) { 0 } else { 0 }
        $runTime = if ($b.EstimatedRunTime -eq 71582788 -or $null -eq $b.EstimatedRunTime -or $b.EstimatedRunTime -eq 0) { "En charge" } else { "$($b.EstimatedRunTime) min" }
        $results += [PSCustomObject]@{
            Name = if ($b.Name) { $b.Name } else { "Batterie" }
            Status = switch ($b.BatteryStatus) {
                1 { "Décharge" } 2 { "Secteur" } 3 { "Chargée complète" }
                4 { "Faible" } 5 { "Critique" } 6 { "En charge" }
                7 { "Charge haute" } 8 { "Charge faible" } default { "Inconnu" }
            }
            ChargeLevel = [int]$b.EstimatedChargeRemaining
            RunTime = $runTime
            DesignCapacity = $dc
            FullCapacity = $fc
            HealthPercent = $health
            Chemistry = switch ($b.Chemistry) {
                2 { "Inconnu" } 3 { "Lead Acid" } 4 { "NiCd" }
                5 { "NiMH" } 6 { "Lithium-ion" } 7 { "Zinc Air" } 8 { "Li-Po" }
                default { "Inconnu" }
            }
            CycleCount = $cycleCount
        }
    }
}
if ($results.Count -eq 0) {
    $pb = Get-WmiObject -Class Win32_PortableBattery -ErrorAction SilentlyContinue
    if ($pb) {
        foreach ($b in @($pb)) {
            $dc = if ($designCap -gt 0) { $designCap } else { [int]$b.DesignCapacity }
            $fc = if ($fullCap -gt 0) { $fullCap } else { [int]$b.FullChargeCapacity }
            $health = if ($dc -gt 0 -and $fc -gt 0) { [math]::Round(($fc / $dc) * 100, 1) } else { 0 }
            $results += [PSCustomObject]@{
                Name = if ($b.Name) { $b.Name } else { "Batterie portable" }
                Status = "Inconnu"; ChargeLevel = 0; RunTime = "N/A"
                DesignCapacity = $dc; FullCapacity = $fc
                HealthPercent = $health; Chemistry = "Inconnu"; CycleCount = $cycleCount
            }
        }
    }
}
$results | ConvertTo-Json -Depth 2 -Compress
"#;
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps])
                .creation_flags(0x08000000)
                .output().map_err(|e| e.to_string())?;
            let text = String::from_utf8_lossy(&out.stdout);
            let trimmed = text.trim();
            if trimmed.is_empty() || trimmed == "null" { return Ok(vec![]); }
            let items: Vec<serde_json::Value> = serde_json::from_str(trimmed)
                .unwrap_or_else(|_| serde_json::from_str(&format!("[{}]", trimmed)).unwrap_or_default());
            Ok(items.iter().map(|item| BatteryDetailed {
                name: item["Name"].as_str().unwrap_or("Batterie").to_string(),
                status: item["Status"].as_str().unwrap_or("Inconnu").to_string(),
                estimated_charge_remaining: item["ChargeLevel"].as_u64().unwrap_or(0) as u32,
                estimated_run_time: item["RunTime"].as_str().unwrap_or("N/A").to_string(),
                design_capacity: item["DesignCapacity"].as_u64().unwrap_or(0) as u32,
                full_charge_capacity: item["FullCapacity"].as_u64().unwrap_or(0) as u32,
                battery_health_percent: item["HealthPercent"].as_f64().unwrap_or(0.0) as f32,
                chemistry: item["Chemistry"].as_str().unwrap_or("Inconnu").to_string(),
                cycle_count: item["CycleCount"].as_u64().unwrap_or(0) as u32,
            }).collect())
        }
        #[cfg(not(target_os = "windows"))]
        Ok(vec![])
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_monitor_info() -> Result<Vec<MonitorDetail>, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let ps = r#"
try {
    Add-Type -AssemblyName System.Windows.Forms -ErrorAction SilentlyContinue
    $screens = [System.Windows.Forms.Screen]::AllScreens
    $wmiMonitors = @{}
    Get-WmiObject -Class Win32_DesktopMonitor -ErrorAction SilentlyContinue | ForEach-Object {
        $key = $_.Name -replace '[^a-zA-Z0-9]', ''
        $wmiMonitors[$key] = $_
    }
    $i = 0
    $result = foreach ($s in $screens) {
        $i++
        $label = if ($s.Primary) { "Ecran Principal" } else { "Ecran $i" }
        $dpi = try {
            Add-Type -TypeDefinition 'using System.Runtime.InteropServices; public class DpiHelper { [DllImport("gdi32.dll")] public static extern int GetDeviceCaps(IntPtr hdc, int nIndex); }' -ErrorAction SilentlyContinue
            0
        } catch { 0 }
        [PSCustomObject]@{
            Name = $label
            DeviceName = $s.DeviceName
            Width = $s.Bounds.Width
            Height = $s.Bounds.Height
            Primary = $s.Primary
            BitsPerPixel = $s.BitsPerPixel
            WorkWidth = $s.WorkingArea.Width
            WorkHeight = $s.WorkingArea.Height
        }
    }
    $result | ConvertTo-Json -Depth 2 -Compress
} catch { Write-Output "[]" }
"#;
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps])
                .creation_flags(0x08000000).output().map_err(|e| e.to_string())?;
            let text = String::from_utf8_lossy(&out.stdout);
            let trimmed = text.trim();
            if trimmed.is_empty() || trimmed == "[]" || trimmed == "null" { return Ok(vec![]); }
            let items: Vec<serde_json::Value> = serde_json::from_str(trimmed)
                .unwrap_or_else(|_| serde_json::from_str(&format!("[{}]", trimmed)).unwrap_or_default());
            Ok(items.iter().map(|item| {
                let w = item["Width"].as_u64().unwrap_or(0) as u32;
                let h = item["Height"].as_u64().unwrap_or(0) as u32;
                // Approximate PPI for common sizes (rough estimate)
                let ppi = if w > 0 && h > 0 {
                    // Standard 24" = 92ppi, 27" = 81ppi — fallback 96
                    96u32
                } else { 0 };
                MonitorDetail {
                    name: item["Name"].as_str().unwrap_or("Écran").to_string(),
                    screen_width: w,
                    screen_height: h,
                    pixels_per_inch: ppi,
                    manufacturer: item["DeviceName"].as_str().unwrap_or("").to_string(),
                    availability: if item["Primary"].as_bool().unwrap_or(false) { "Principal".to_string() } else { "Secondaire".to_string() },
                }
            }).collect())
        }
        #[cfg(not(target_os = "windows"))]
        Ok(vec![])
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_power_plans() -> Result<Vec<PowerPlan>, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let out = std::process::Command::new("powercfg")
                .args(["/LIST"]).creation_flags(0x08000000).output().map_err(|e| e.to_string())?;
            let mut plans = Vec::new();
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if !line.contains("GUID") || !line.contains('(') { continue; }
                let guid_s = line.find("GUID:").map(|i| i + 6).unwrap_or(0);
                let guid_e = line[guid_s..].find(' ').map(|i| guid_s + i).unwrap_or(line.len());
                let guid = line[guid_s..guid_e].trim().to_string();
                let n_s = line.find('(').map(|i| i + 1).unwrap_or(0);
                let n_e = line.rfind(')').unwrap_or(line.len());
                plans.push(PowerPlan { name: line[n_s..n_e].to_string(), is_active: line.contains('*'), guid });
            }
            Ok(plans)
        }
        #[cfg(not(target_os = "windows"))]
        Ok(vec![])
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_printers() -> Result<Vec<PrinterDetail>, String> {
    tokio::task::spawn_blocking(|| {
        let wmi = wmi_con()?;
        let r: Vec<WmiPrinter> = wmi.raw_query("SELECT * FROM Win32_Printer").map_err(|e| e.to_string())?;
        Ok(r.into_iter().map(|p| PrinterDetail {
            name: p.Name.unwrap_or_default(),
            driver_name: p.DriverName.unwrap_or_default(),
            port_name: p.PortName.unwrap_or_default(),
            is_default: p.Default.unwrap_or(false),
            is_network: p.Network.unwrap_or(false),
            status: p.Status.unwrap_or_default(),
            shared: p.Shared.unwrap_or(false),
        }).collect())
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_environment_variables() -> Result<Vec<EnvVar>, String> {
    let sys_keys = ["PATH", "PATHEXT", "ComSpec", "OS", "SystemRoot", "windir",
        "ProgramFiles", "ProgramData", "SystemDrive", "NUMBER_OF_PROCESSORS",
        "PROCESSOR_ARCHITECTURE", "PROCESSOR_IDENTIFIER", "TEMP", "TMP"];
    let mut vars: Vec<EnvVar> = std::env::vars().map(|(name, value)| {
        let var_type = if sys_keys.contains(&name.as_str()) { "Système" } else { "Utilisateur" }.to_string();
        EnvVar { name, value, var_type }
    }).collect();
    vars.sort_by_key(|a| a.name.to_lowercase());
    Ok(vars)
}

#[tauri::command]
pub async fn get_windows_license() -> Result<WindowsLicense, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let ps = r#"
$licStatus = { param($code) switch ($code) {
    0{"Non licencié"} 1{"Licencié"} 2{"Période de grâce OOB"} 3{"Période de grâce OOT"}
    4{"Non-authentique"} 5{"Notification"} 6{"Grâce étendue"} default{"Inconnu"}
} }
$allProducts = Get-WmiObject -Query "SELECT Name,LicenseStatus,PartialProductKey,LicenseFamily,Description FROM SoftwareLicensingProduct WHERE PartialProductKey IS NOT NULL" -ErrorAction SilentlyContinue
$win = $allProducts | Where-Object { $_.Name -like "*Windows*" } | Sort-Object LicenseStatus | Select-Object -First 1
# Office : requête séparée sans filtre PartialProductKey (Ohook/KMS n'en a pas)
$office = Get-WmiObject -Query "SELECT Name,LicenseStatus,PartialProductKey FROM SoftwareLicensingProduct WHERE (Name LIKE '%Office%' OR Name LIKE '%Microsoft 365%' OR Name LIKE '%Visio%' OR Name LIKE '%Project%')" -ErrorAction SilentlyContinue | Sort-Object LicenseStatus | Select-Object -First 1

# Décodage clé complète : OA3 (OEM UEFI) en priorité, sinon DigitalProductId
function Get-FullKey {
    # 1. Clé OEM dans le firmware UEFI/BIOS
    try {
        $sls = Get-WmiObject -Class SoftwareLicensingService -ErrorAction SilentlyContinue
        if ($sls -and $sls.OA3xOriginalProductKey) { return $sls.OA3xOriginalProductKey }
    } catch {}
    # 2. Décodage depuis le registre DigitalProductId
    try {
        $regPath = "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion"
        $raw = (Get-ItemProperty -Path $regPath -ErrorAction SilentlyContinue).DigitalProductId
        if ($null -eq $raw -or $raw.Length -lt 67) { return "" }
        $offset = 52
        $isWin8 = [int][math]::Floor($raw[66] / 6) -band 1
        $raw[66] = ($raw[66] -band 0xF7) -bor (($isWin8 -band 2) * 4)
        $maps = "BCDFGHJKMPQRTVWXY2346789"
        $result = ""; $n = 0
        for ($i = 24; $i -ge 0; $i--) {
            $n = 0
            for ($j = 14; $j -ge 0; $j--) {
                $n = ($n * 256) -bxor [int]$raw[$j + $offset]
                $raw[$j + $offset] = [int][math]::Floor($n / 24)
                $n = $n % 24
            }
            $result = $maps[$n] + $result
            if ($i % 5 -eq 0 -and $i -ne 0) { $result = "-" + $result }
        }
        return $result
    } catch { return "" }
}
$fullKey = Get-FullKey

function Get-OfficeKey {
    $maps = "BCDFGHJKMPQRTVWXY2346789"
    $isCTR = Test-Path 'HKLM:\SOFTWARE\Microsoft\Office\ClickToRun\Configuration' -ErrorAction SilentlyContinue
    $bases = @(
        'HKLM:\SOFTWARE\Microsoft\Office\16.0\Registration',
        'HKLM:\SOFTWARE\Microsoft\Office\15.0\Registration',
        'HKLM:\SOFTWARE\Microsoft\Office\14.0\Registration',
        'HKLM:\SOFTWARE\Wow6432Node\Microsoft\Office\16.0\Registration',
        'HKLM:\SOFTWARE\Wow6432Node\Microsoft\Office\15.0\Registration',
        'HKLM:\SOFTWARE\Wow6432Node\Microsoft\Office\14.0\Registration'
    )
    foreach ($b in $bases) {
        if (-not (Test-Path $b -ErrorAction SilentlyContinue)) { continue }
        foreach ($sk in (Get-ChildItem $b -ErrorAction SilentlyContinue)) {
            try {
                $raw = (Get-ItemProperty $sk.PSPath -ErrorAction SilentlyContinue).DigitalProductId
                if ($null -eq $raw -or $raw.Length -lt 67) { continue }
                # Clone obligatoire — l'algo modifie le tableau en place
                $rc = [byte[]]$raw.Clone()
                $offset = 52; $result = ""
                for ($i = 24; $i -ge 0; $i--) {
                    $n = 0
                    for ($j = 14; $j -ge 0; $j--) {
                        $n = ($n * 256) -bxor [int]$rc[$j + $offset]
                        $rc[$j + $offset] = [int][math]::Floor($n / 24)
                        $n = $n % 24
                    }
                    $result = $maps[$n] + $result
                    if ($i % 5 -eq 0 -and $i -ne 0) { $result = "-" + $result }
                }
                # Valider strictement le format XXXXX-XXXXX-XXXXX-XXXXX-XXXXX
                if ($result -match '^[BCDFGHJKMPQRTVWXY2346789]{5}-[BCDFGHJKMPQRTVWXY2346789]{5}-[BCDFGHJKMPQRTVWXY2346789]{5}-[BCDFGHJKMPQRTVWXY2346789]{5}-[BCDFGHJKMPQRTVWXY2346789]{5}$') {
                    return $result
                }
            } catch {}
        }
    }
    # Fallback : clé partielle + type (C2R/365 ne stocke pas la clé complète)
    try {
        $lp = Get-WmiObject -Query "SELECT Name,PartialProductKey FROM SoftwareLicensingProduct WHERE PartialProductKey IS NOT NULL AND (Name LIKE '%Office%' OR Name LIKE '%Microsoft 365%')" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($lp -and $lp.PartialProductKey) {
            $t = if ($isCTR) { "C2R/365 — clé non stockée" } else { "MSI" }
            return "XXXXX-XXXXX-XXXXX-XXXXX-$($lp.PartialProductKey) ($t)"
        }
    } catch {}
    if ($isCTR) { return "Office 365/C2R — clé non récupérable (abonnement)" }
    return ""
}
$officeFullKey = Get-OfficeKey

[PSCustomObject]@{
    ProductName = if ($win) { $win.Name } else { "Windows" }
    ActivationStatus = if ($win) { & $licStatus $win.LicenseStatus } else { "Inconnu" }
    PartialKey = if ($win -and $win.PartialProductKey) { $win.PartialProductKey } else { "N/A" }
    FullKey = if ($fullKey) { $fullKey } else { "" }
    LicenseFamily = if ($win) { $win.LicenseFamily } else { "" }
    Description = if ($win) { $win.Description } else { "" }
    OfficeName = if ($office) { $office.Name } else { "" }
    OfficeStatus = if ($office) { & $licStatus $office.LicenseStatus } else { "" }
    OfficeKey = if ($office) { $office.PartialProductKey } else { "" }
    OfficeFullKey = $officeFullKey
} | ConvertTo-Json -Compress
"#;
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps])
                .creation_flags(0x08000000).output().map_err(|e| e.to_string())?;
            let text = String::from_utf8_lossy(&out.stdout);
            let trimmed = text.trim();
            if trimmed.is_empty() || trimmed == "null" {
                return Err("Licence Windows non trouvée".to_string());
            }
            let v: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| e.to_string())?;
            Ok(WindowsLicense {
                product_name: v["ProductName"].as_str().unwrap_or("Windows").to_string(),
                activation_status: v["ActivationStatus"].as_str().unwrap_or("Inconnu").to_string(),
                partial_product_key: v["PartialKey"].as_str().unwrap_or("N/A").to_string(),
                full_product_key: v["FullKey"].as_str().unwrap_or("").to_string(),
                license_status: v["Description"].as_str().unwrap_or("").to_string(),
                license_family: v["LicenseFamily"].as_str().unwrap_or("").to_string(),
                office_name: v["OfficeName"].as_str().unwrap_or("").to_string(),
                office_status: v["OfficeStatus"].as_str().unwrap_or("").to_string(),
                office_key: v["OfficeKey"].as_str().unwrap_or("").to_string(),
                office_full_key: v["OfficeFullKey"].as_str().unwrap_or("").to_string(),
            })
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_installed_updates() -> Result<Vec<InstalledUpdate>, String> {
    tokio::task::spawn_blocking(|| {
        let wmi = wmi_con()?;
        let r: Vec<WmiQuickFix> = wmi.raw_query("SELECT * FROM Win32_QuickFixEngineering").map_err(|e| e.to_string())?;
        Ok(r.into_iter().map(|u| InstalledUpdate {
            title: u.Caption.unwrap_or_default(),
            hotfix_id: u.HotFixID.unwrap_or_default(),
            description: u.Description.unwrap_or_default(),
            installed_on: u.InstalledOn.unwrap_or_default(),
            installed_by: u.InstalledBy.unwrap_or_default(),
        }).collect())
    }).await.map_err(|e| e.to_string())?
}

/// Retourne les infos BitLocker de tous les volumes (pour l'export diagnostic).
#[tauri::command]
pub async fn get_bitlocker_report() -> Result<String, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            use std::os::windows::process::CommandExt;
            let script = r#"
$vols = Get-BitLockerVolume -ErrorAction SilentlyContinue
if (-not $vols) { "Aucun volume BitLocker détecté ou module non disponible."; exit }
foreach ($v in $vols) {
    $mount = $v.MountPoint
    $status = $v.VolumeStatus
    $pct = $v.EncryptionPercentage
    $prot = $v.ProtectionStatus
    Write-Output "Volume : $mount | Statut : $status | Chiffrement : $pct% | Protection : $prot"
    foreach ($kp in $v.KeyProtector) {
        $type = $kp.KeyProtectorType
        $rp = if ($kp.RecoveryPassword) { $kp.RecoveryPassword } else { "—" }
        Write-Output "  Protecteur : $type | Clé récupération : $rp"
    }
}
"#;
            let tmp = std::env::temp_dir().join("nitrite_bde.ps1");
            let full = format!("$OutputEncoding=[System.Text.Encoding]::UTF8;[Console]::OutputEncoding=[System.Text.Encoding]::UTF8;\n{}", script);
            let _ = std::fs::write(&tmp, full.as_bytes());
            let out = Command::new("powershell")
                .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", tmp.to_str().unwrap_or("")])
                .creation_flags(0x08000000)
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_else(|e| format!("Erreur: {}", e));
            let _ = std::fs::remove_file(&tmp);
            Ok(out)
        }
        #[cfg(not(target_os = "windows"))]
        { Ok("BitLocker non disponible hors Windows.".to_string()) }
    }).await.map_err(|e| e.to_string())?
}
