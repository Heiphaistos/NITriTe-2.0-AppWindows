use serde::Serialize;
use sysinfo::{Disks, System};
use wmi::WMIConnection;
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::error::NiTriTeError;

// === Types serialisables ===

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub os: OsInfo,
    pub cpu: CpuInfo,
    pub ram: RamInfo,
    pub gpus: Vec<GpuInfo>,
    pub disks: Vec<DiskInfo>,
    pub motherboard: MotherboardInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct OsInfo {
    pub name: String,
    pub version: String,
    pub architecture: String,
    pub hostname: String,
    pub build_number: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpuInfo {
    pub name: String,
    pub manufacturer: String,
    pub cores: u32,
    pub threads: u32,
    pub base_speed_mhz: u32,
    pub max_speed_mhz: u32,
    pub usage_percent: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RamModule {
    pub capacity_gb: f64,
    pub speed_mhz: u32,
    pub memory_type: String,
    pub manufacturer: String,
    pub slot: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RamInfo {
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub usage_percent: f32,
    pub modules: Vec<RamModule>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub name: String,
    pub vram_gb: f64,
    pub driver_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub model: String,
    pub size_gb: f64,
    pub interface_type: String,
    pub partitions: Vec<DiskPartition>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskPartition {
    pub mount_point: String,
    pub fs_type: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub free_gb: f64,
    pub usage_percent: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MotherboardInfo {
    pub manufacturer: String,
    pub model: String,
}

// === WMI query structs ===

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct Win32Processor {
    Name: Option<String>,
    Manufacturer: Option<String>,
    NumberOfCores: Option<u32>,
    NumberOfLogicalProcessors: Option<u32>,
    MaxClockSpeed: Option<u32>,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct Win32PhysicalMemory {
    Capacity: Option<u64>,
    Speed: Option<u32>,
    SMBIOSMemoryType: Option<u32>,
    Manufacturer: Option<String>,
    DeviceLocator: Option<String>,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct Win32VideoController {
    Name: Option<String>,
    AdapterRAM: Option<u64>,
    DriverVersion: Option<String>,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct Win32DiskDrive {
    Model: Option<String>,
    Size: Option<u64>,
    InterfaceType: Option<String>,
    PNPDeviceID: Option<String>,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct Win32BaseBoard {
    Manufacturer: Option<String>,
    Product: Option<String>,
}

// === GPU VRAM : nvidia-smi + registre ===

/// Retourne map { gpu_name_lowercase → vram_gb } depuis nvidia-smi
fn nvidia_vram_map() -> HashMap<String, f64> {
    let mut map = HashMap::new();
    let output = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])
        .creation_flags(0x08000000)
        .output();

    let Ok(out) = output else { return map };
    if !out.status.success() { return map; }

    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let parts: Vec<&str> = line.splitn(2, ',').collect();
        if parts.len() == 2 {
            let name = parts[0].trim().to_lowercase();
            if let Ok(mb) = parts[1].trim().parse::<f64>() {
                map.insert(name, mb / 1024.0);
            }
        }
    }
    map
}

/// Lit la VRAM depuis le registre Windows (clé VideoController)
/// Retourne map { driver_desc_lowercase → vram_gb }
fn registry_vram_map() -> HashMap<String, f64> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut map = HashMap::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let class_path = r"SYSTEM\ControlSet001\Control\Class\{4D36E968-E325-11CE-BFC1-08002BE10318}";
    let Ok(class_key) = hklm.open_subkey(class_path) else { return map };

    for subkey_name in class_key.enum_keys().filter_map(|k| k.ok()) {
        let Ok(subkey) = class_key.open_subkey(&subkey_name) else { continue };
        let desc: String = subkey.get_value("DriverDesc").unwrap_or_default();
        if desc.is_empty() { continue; }

        // Essayer d'abord qwMemorySize (64-bit)
        let vram_bytes: Option<u64> = subkey
            .get_raw_value("HardwareInformation.qwMemorySize")
            .ok()
            .and_then(|r| {
                if r.bytes.len() >= 8 {
                    Some(u64::from_le_bytes(r.bytes[..8].try_into().ok()?))
                } else {
                    None
                }
            });

        // Sinon MemorySize (32-bit)
        let vram_bytes = vram_bytes.or_else(|| {
            subkey.get_raw_value("HardwareInformation.MemorySize").ok().and_then(|r| {
                if r.bytes.len() >= 4 {
                    Some(u32::from_le_bytes(r.bytes[..4].try_into().ok()?) as u64)
                } else {
                    None
                }
            })
        });

        if let Some(bytes) = vram_bytes {
            if bytes > 0 {
                map.insert(desc.to_lowercase(), bytes as f64 / 1_073_741_824.0);
            }
        }
    }
    map
}

/// Résout la VRAM réelle pour un GPU
fn resolve_vram(gpu_name: &str, wmi_vram_gb: f64,
    nvidia_map: &HashMap<String, f64>,
    registry_map: &HashMap<String, f64>) -> f64
{
    let name_lower = gpu_name.to_lowercase();

    // nvidia-smi : correspondance partielle
    for (k, v) in nvidia_map {
        if name_lower.contains(k.as_str()) || k.contains(name_lower.as_str()) {
            return *v;
        }
    }

    // Registre : correspondance partielle
    for (k, v) in registry_map {
        if name_lower.contains(k.as_str()) || k.contains(name_lower.as_str()) {
            return *v;
        }
    }

    // Fallback WMI (souvent plafonné à 4GB)
    wmi_vram_gb
}

// === Interface disque : détection NVMe / SATA ===

fn detect_interface(model: &str, pnp_id: &str, wmi_interface: &str) -> String {
    let m = model.to_uppercase();
    let p = pnp_id.to_uppercase();

    if m.contains("NVME") || m.contains("NVM") || p.contains("NVME") || p.contains("NVM") {
        return "NVMe".to_string();
    }
    if p.contains("USBSTOR") || wmi_interface.eq_ignore_ascii_case("USB") {
        return "USB".to_string();
    }
    // WMI retourne "IDE" pour SATA et "SCSI" pour NVMe/SATA selon le pilote
    if wmi_interface.eq_ignore_ascii_case("IDE") || wmi_interface.eq_ignore_ascii_case("SCSI") {
        // Distinction par nom
        if m.contains("SSD") || m.contains("NVME") || m.contains("NVM") {
            return "SATA SSD".to_string();
        }
        return "SATA".to_string();
    }
    if wmi_interface.is_empty() {
        return "SATA".to_string();
    }
    wmi_interface.to_string()
}

// === Collection ===

pub fn collect_system_info() -> Result<SystemInfo, NiTriTeError> {
    let wmi = WMIConnection::new().map_err(|e| NiTriTeError::Wmi(e.to_string()))?;

    let os = collect_os_info();
    let cpu = collect_cpu_info(&wmi)?;
    let ram = collect_ram_info(&wmi)?;
    let gpus = collect_gpu_info(&wmi)?;
    let disks = collect_disk_info(&wmi)?;
    let motherboard = collect_motherboard_info(&wmi)?;

    Ok(SystemInfo { os, cpu, ram, gpus, disks, motherboard })
}

fn collect_os_info() -> OsInfo {
    OsInfo {
        name: System::name().unwrap_or_else(|| "Windows".to_string()),
        version: System::os_version().unwrap_or_default(),
        architecture: if cfg!(target_arch = "x86_64") { "x64".to_string() } else { "x86".to_string() },
        hostname: System::host_name().unwrap_or_default(),
        build_number: System::os_version().unwrap_or_default(),
    }
}

fn collect_cpu_info(wmi: &WMIConnection) -> Result<CpuInfo, NiTriTeError> {
    let results: Vec<Win32Processor> = wmi.raw_query("SELECT * FROM Win32_Processor").map_err(|e| NiTriTeError::Wmi(e.to_string()))?;
    let proc = results.first().ok_or_else(|| NiTriTeError::System("Aucun CPU".into()))?;

    let mut sys = System::new();
    sys.refresh_cpu_usage();
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_cpu_usage();
    let usage = sys.global_cpu_usage();

    let max_mhz = proc.MaxClockSpeed.unwrap_or(0);

    Ok(CpuInfo {
        name: proc.Name.clone().unwrap_or_default().trim().to_string(),
        manufacturer: proc.Manufacturer.clone().unwrap_or_default(),
        cores: proc.NumberOfCores.unwrap_or(0),
        threads: proc.NumberOfLogicalProcessors.unwrap_or(0),
        base_speed_mhz: max_mhz,
        max_speed_mhz: max_mhz,
        usage_percent: usage,
    })
}

fn collect_ram_info(wmi: &WMIConnection) -> Result<RamInfo, NiTriTeError> {
    let results: Vec<Win32PhysicalMemory> = wmi.raw_query("SELECT * FROM Win32_PhysicalMemory").map_err(|e| NiTriTeError::Wmi(e.to_string()))?;

    let modules: Vec<RamModule> = results
        .iter()
        .map(|m| {
            let capacity_gb = m.Capacity.unwrap_or(0) as f64 / 1_073_741_824.0;
            let memory_type = match m.SMBIOSMemoryType.unwrap_or(0) {
                20 => "DDR",
                21 => "DDR2",
                24 => "DDR3",
                26 => "DDR4",
                34 => "DDR5",
                _ => {
                    let speed = m.Speed.unwrap_or(0);
                    if speed >= 4800 { "DDR5" }
                    else if speed >= 2133 { "DDR4" }
                    else if speed >= 800 { "DDR3" }
                    else { "DDR" }
                }
            };
            RamModule {
                capacity_gb,
                speed_mhz: m.Speed.unwrap_or(0),
                memory_type: memory_type.to_string(),
                manufacturer: m.Manufacturer.clone().unwrap_or_default().trim().to_string(),
                slot: m.DeviceLocator.clone().unwrap_or_default(),
            }
        })
        .collect();

    let sys = System::new_all();
    let total = sys.total_memory() as f64 / 1_073_741_824.0;
    let used = sys.used_memory() as f64 / 1_073_741_824.0;
    let available = total - used;
    let usage = if total > 0.0 { (used / total * 100.0) as f32 } else { 0.0 };

    Ok(RamInfo { total_gb: total, used_gb: used, available_gb: available, usage_percent: usage, modules })
}

fn collect_gpu_info(wmi: &WMIConnection) -> Result<Vec<GpuInfo>, NiTriTeError> {
    let results: Vec<Win32VideoController> = wmi.raw_query("SELECT * FROM Win32_VideoController").map_err(|e| NiTriTeError::Wmi(e.to_string()))?;

    // Charger les maps VRAM une seule fois
    let nvidia_map = nvidia_vram_map();
    let registry_map = registry_vram_map();

    Ok(results
        .iter()
        .filter(|g| {
            let name = g.Name.as_deref().unwrap_or("");
            !name.contains("Microsoft") && !name.contains("Basic")
        })
        .map(|g| {
            let name = g.Name.clone().unwrap_or_default().trim().to_string();
            let wmi_vram = g.AdapterRAM.unwrap_or(0) as f64 / 1_073_741_824.0;
            let vram_gb = resolve_vram(&name, wmi_vram, &nvidia_map, &registry_map);
            GpuInfo {
                name,
                vram_gb,
                driver_version: g.DriverVersion.clone().unwrap_or_default(),
            }
        })
        .collect())
}

fn collect_disk_info(wmi: &WMIConnection) -> Result<Vec<DiskInfo>, NiTriTeError> {
    let drives: Vec<Win32DiskDrive> = wmi.raw_query("SELECT * FROM Win32_DiskDrive").map_err(|e| NiTriTeError::Wmi(e.to_string()))?;
    let sysinfo_disks = Disks::new_with_refreshed_list();

    let mut partitions: Vec<DiskPartition> = sysinfo_disks
        .iter()
        .map(|d| {
            let total = d.total_space() as f64 / 1_073_741_824.0;
            let available = d.available_space() as f64 / 1_073_741_824.0;
            let used = total - available;
            let usage = if total > 0.0 { (used / total * 100.0) as f32 } else { 0.0 };
            DiskPartition {
                mount_point: d.mount_point().to_string_lossy().to_string(),
                fs_type: d.file_system().to_string_lossy().to_string(),
                total_gb: total,
                used_gb: used,
                free_gb: available,
                usage_percent: usage,
            }
        })
        .collect();

    let disks: Vec<DiskInfo> = drives
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let size_bytes = d.Size.unwrap_or(0);
            let model = d.Model.clone().unwrap_or_default().trim().to_string();
            let pnp = d.PNPDeviceID.clone().unwrap_or_default();
            let wmi_iface = d.InterfaceType.clone().unwrap_or_default();
            let interface_type = detect_interface(&model, &pnp, &wmi_iface);

            let disk_partitions = if drives.len() == 1 {
                std::mem::take(&mut partitions)
            } else if i == 0 {
                let c_parts: Vec<_> = partitions
                    .iter()
                    .filter(|p| p.mount_point.starts_with("C:"))
                    .cloned()
                    .collect();
                if !c_parts.is_empty() {
                    partitions.retain(|p| !p.mount_point.starts_with("C:"));
                    c_parts
                } else {
                    vec![]
                }
            } else {
                std::mem::take(&mut partitions)
            };

            DiskInfo {
                model,
                size_gb: size_bytes as f64 / 1_073_741_824.0,
                interface_type,
                partitions: disk_partitions,
            }
        })
        .collect();

    Ok(disks)
}

fn collect_motherboard_info(wmi: &WMIConnection) -> Result<MotherboardInfo, NiTriTeError> {
    let results: Vec<Win32BaseBoard> = wmi.raw_query("SELECT * FROM Win32_BaseBoard").map_err(|e| NiTriTeError::Wmi(e.to_string()))?;
    let board = results.first();
    Ok(MotherboardInfo {
        manufacturer: board.and_then(|b| b.Manufacturer.clone()).unwrap_or_default().trim().to_string(),
        model: board.and_then(|b| b.Product.clone()).unwrap_or_default().trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_interface_nvme_by_model_name() {
        assert_eq!(detect_interface("Samsung NVMe SSD 970", "", ""), "NVMe");
        assert_eq!(detect_interface("WD BLACK NVM", "", "IDE"), "NVMe");
    }

    #[test]
    fn detect_interface_nvme_by_pnp_id() {
        assert_eq!(detect_interface("Generic Drive", "NVME\\DISK", ""), "NVMe");
    }

    #[test]
    fn detect_interface_usb_by_pnp_id() {
        assert_eq!(detect_interface("USB Drive", "USBSTOR\\DISK", ""), "USB");
    }

    #[test]
    fn detect_interface_sata_ssd_via_ide_wmi() {
        assert_eq!(detect_interface("Samsung SSD 870 EVO", "", "IDE"), "SATA SSD");
    }

    #[test]
    fn detect_interface_sata_hdd_via_ide_wmi() {
        assert_eq!(detect_interface("WD Blue 1TB HDD", "", "IDE"), "SATA");
    }

    #[test]
    fn detect_interface_empty_wmi_defaults_sata() {
        assert_eq!(detect_interface("Unknown Drive", "", ""), "SATA");
    }

    #[test]
    fn detect_interface_usb_by_wmi_flag() {
        assert_eq!(detect_interface("Lexar USB", "", "USB"), "USB");
    }

    #[test]
    fn resolve_vram_falls_back_to_wmi() {
        let empty: HashMap<String, f64> = HashMap::new();
        assert_eq!(resolve_vram("Unknown GPU", 4.0, &empty, &empty), 4.0);
    }

    #[test]
    fn resolve_vram_nvidia_map_match() {
        let mut nvidia: HashMap<String, f64> = HashMap::new();
        nvidia.insert("rtx 3080".to_string(), 10.0);
        let empty: HashMap<String, f64> = HashMap::new();
        assert_eq!(resolve_vram("NVIDIA GeForce RTX 3080", 4.0, &nvidia, &empty), 10.0);
    }
}
