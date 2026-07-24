use serde::Serialize;
use wmi::WMIConnection;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// ============================================================================
// Result Structs
// ============================================================================

#[derive(Debug, Serialize, Clone)]
pub struct StoragePhysical {
    pub model: String, pub serial_number: String, pub firmware_revision: String,
    pub size_bytes: u64, pub size_gb: f64, pub interface_type: String,
    pub media_type: String, pub status: String, pub pnp_device_id: String, pub partitions: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct NetworkAdapterDetail {
    pub name: String, pub description: String, pub mac_address: String,
    pub ip_addresses: Vec<String>, pub subnet_masks: Vec<String>,
    pub default_gateway: Vec<String>, pub dns_servers: Vec<String>,
    pub dhcp_enabled: bool, pub dhcp_server: String,
    pub speed_mbps: u64, pub net_connection_id: String, pub is_physical: bool, pub status: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct CpuCacheInfo { pub l1_instruction_kb: u32, pub l1_data_kb: u32, pub l2_kb: u32, pub l3_kb: u32, pub l4_kb: u32 }

#[derive(Debug, Serialize, Clone)]
pub struct InstalledSoftware {
    pub name: String, pub version: String, pub publisher: String,
    pub install_date: String, pub install_location: String, pub estimated_size_mb: f64,
}

// ============================================================================
// WMI typed structs
// ============================================================================

#[derive(serde::Deserialize)] #[allow(non_snake_case, dead_code)]
struct WmiDiskDrive {
    Model: Option<String>, SerialNumber: Option<String>, FirmwareRevision: Option<String>,
    Size: Option<u64>, InterfaceType: Option<String>, MediaType: Option<String>,
    Status: Option<String>, PNPDeviceID: Option<String>, Partitions: Option<u32>,
}

#[derive(serde::Deserialize)] #[allow(non_snake_case)]
struct WmiCacheMemory { Purpose: Option<String>, InstalledSize: Option<u32>, Level: Option<u32> }

// ============================================================================
// Helpers
// ============================================================================

fn wmi_con() -> Result<WMIConnection, String> {
    WMIConnection::new().map_err(|e| format!("WMI: {}", e))
}

// ============================================================================
// Commands
// ============================================================================

#[tauri::command]
pub async fn get_storage_physical_info() -> Result<Vec<StoragePhysical>, String> {
    tokio::task::spawn_blocking(|| {
        // PowerShell + timeout 15s — évite le blocage WMI Win32_DiskDrive
        #[cfg(target_os = "windows")]
        {
            use std::io::Read;
            let ps = r#"
try {
    $disks = Get-PhysicalDisk -ErrorAction SilentlyContinue
    if (-not $disks) { Write-Output '[]'; exit }
    $result = foreach ($d in $disks) {
        $partCount = 0
        try {
            $diskNum = $d.DeviceId
            $partCount = (Get-Partition -DiskNumber $diskNum -ErrorAction SilentlyContinue | Measure-Object).Count
        } catch {}
        $bus = [string]$d.BusType
        $iface = switch ($bus) {
            "NVMe"  { "NVMe PCIe" }
            "SATA"  { "SATA" }
            "SAS"   { "SAS" }
            "USB"   { "USB" }
            "RAID"  { "RAID" }
            default { $bus }
        }
        $mtype = [string]$d.MediaType
        if ($mtype -eq "Unspecified" -or $mtype -eq "") {
            $n = ([string]$d.FriendlyName).ToLower()
            $mtype = if ($n -match "nvme|ssd") { "SSD" } elseif ($n -match "hdd|hd") { "HDD" } else { "Inconnu" }
        }
        [PSCustomObject]@{
            model    = [string]$d.FriendlyName
            serial   = [string]$d.SerialNumber
            firmware = [string]$d.FirmwareVersion
            size     = [long]$d.Size
            iface    = $iface
            mtype    = $mtype
            health   = [string]$d.HealthStatus
            pnp      = [string]$d.DeviceId
            parts    = [int]$partCount
        }
    }
    $result | ConvertTo-Json -Compress
} catch { Write-Output '[]' }
"#;
            let mut child = match std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps])
                .creation_flags(0x08000000)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn() {
                    Ok(c) => c,
                    Err(e) => return Err(e.to_string()),
                };
            let timeout = std::time::Duration::from_secs(15);
            let start = std::time::Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        let mut buf = Vec::new();
                        if let Some(mut out) = child.stdout.take() { let _ = out.read_to_end(&mut buf); }
                        let text = crate::maintenance::commands::decode_output(&buf);
                        let t = text.trim();
                        if t.is_empty() || t == "[]" { return Ok(vec![]); }
                        let jt = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
                        let items: Vec<serde_json::Value> = serde_json::from_str(&jt).unwrap_or_default();
                        return Ok(items.into_iter().map(|v| {
                            let model = v["model"].as_str().unwrap_or("").trim().to_string();
                            let mtype_raw = v["mtype"].as_str().unwrap_or("").trim().to_string();
                            let iface = v["iface"].as_str().unwrap_or("").trim().to_string();
                            let ml = model.to_lowercase();
                            let media_type = if mtype_raw == "Inconnu" || mtype_raw.is_empty() {
                                if ml.contains("nvme") || iface.contains("NVMe") { "NVMe".to_string() }
                                else if ml.contains("ssd") { "SSD".to_string() }
                                else { "HDD".to_string() }
                            } else if mtype_raw.to_lowercase().contains("hdd") || mtype_raw.to_lowercase().contains("hard") {
                                "HDD".to_string()
                            } else { mtype_raw };
                            let size = v["size"].as_u64().unwrap_or(0);
                            StoragePhysical {
                                model,
                                serial_number: v["serial"].as_str().unwrap_or("").trim().to_string(),
                                firmware_revision: v["firmware"].as_str().unwrap_or("").trim().to_string(),
                                size_bytes: size,
                                size_gb: size as f64 / 1_000_000_000.0,
                                interface_type: iface,
                                media_type,
                                status: v["health"].as_str().unwrap_or("Unknown").to_string(),
                                pnp_device_id: v["pnp"].as_str().unwrap_or("").to_string(),
                                partitions: v["parts"].as_u64().unwrap_or(0) as u32,
                            }
                        }).collect());
                    }
                    Ok(None) => {
                        if start.elapsed() > timeout { let _ = child.kill(); let _ = child.wait(); return Ok(vec![]); }
                        std::thread::sleep(std::time::Duration::from_millis(200));
                    }
                    Err(e) => { let _ = child.kill(); return Err(e.to_string()); }
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        Ok(vec![])
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_network_adapters_detailed() -> Result<Vec<NetworkAdapterDetail>, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let ps = r#"
$OutputEncoding = [System.Text.Encoding]::UTF8; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
try {
    $adapters = Get-NetAdapter -ErrorAction Stop | Where-Object { $_.Status -eq 'Up' -or $_.Status -eq 'Disconnected' }
    if (-not $adapters) { $adapters = Get-NetAdapter -ErrorAction Stop }
    $result = foreach ($a in $adapters) {
        $ip  = Get-NetIPAddress -InterfaceIndex $a.InterfaceIndex -ErrorAction SilentlyContinue
        $cfg = Get-NetIPConfiguration -InterfaceIndex $a.InterfaceIndex -ErrorAction SilentlyContinue
        $dns = (Get-DnsClientServerAddress -InterfaceIndex $a.InterfaceIndex -AddressFamily IPv4 -ErrorAction SilentlyContinue).ServerAddresses
        $ipv4 = ($ip | Where-Object { $_.AddressFamily -eq 'IPv4' -and $_.PrefixOrigin -ne 'WellKnown' } | Select-Object -ExpandProperty IPAddress) -join ','
        $ipv6 = ($ip | Where-Object { $_.AddressFamily -eq 'IPv6' -and $_.PrefixOrigin -ne 'WellKnown' } | Select-Object -ExpandProperty IPAddress) -join ','
        $gw   = ($cfg.IPv4DefaultGateway | Select-Object -ExpandProperty NextHop) -join ','
        $wmiCfg = Get-WmiObject Win32_NetworkAdapterConfiguration -Filter "InterfaceIndex=$($a.InterfaceIndex)" -ErrorAction SilentlyContinue
        $dhcp = if ($wmiCfg) { $wmiCfg.DHCPEnabled } else { $false }
        $speedMbps = try { [math]::Round($a.LinkSpeed / 1MB) } catch { 0 }
        [PSCustomObject]@{
            Name = $a.Name; Description = $a.InterfaceDescription
            MacAddress = $a.MacAddress
            SpeedMbps = $speedMbps
            Status = $a.Status
            DHCPEnabled = $dhcp
            IPv4 = $ipv4; IPv6 = $ipv6
            Gateway = $gw; DNS = ($dns -join ',')
        }
    }
    $result | ConvertTo-Json -Depth 2 -Compress
} catch {
    # Fallback WMI
    try {
        $cfgs = Get-WmiObject Win32_NetworkAdapterConfiguration -Filter "IPEnabled=True" -ErrorAction Stop
        $result2 = foreach ($c in $cfgs) {
            [PSCustomObject]@{
                Name = $c.Description; Description = $c.Description
                MacAddress = $c.MACAddress
                SpeedMbps = 0; Status = "Up"; DHCPEnabled = $c.DHCPEnabled
                IPv4 = ($c.IPAddress | Where-Object { $_ -notlike '*:*' }) -join ','
                IPv6 = ($c.IPAddress | Where-Object { $_ -like '*:*' }) -join ','
                Gateway = ($c.DefaultIPGateway -join ','); DNS = ($c.DNSServerSearchOrder -join ',')
            }
        }
        $result2 | ConvertTo-Json -Depth 2 -Compress
    } catch { Write-Output "[]" }
}
"#;
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps])
                .creation_flags(0x08000000).output().map_err(|e| e.to_string())?;
            let stdout = String::from_utf8_lossy(&out.stdout);
            let trimmed = stdout.trim();
            if trimmed.is_empty() || trimmed == "[]" { return Ok(vec![]); }
            let items: Vec<serde_json::Value> = serde_json::from_str(trimmed)
                .unwrap_or_else(|_| serde_json::from_str(&format!("[{}]", trimmed)).unwrap_or_default());
            let csv = |v: &serde_json::Value| -> Vec<String> {
                v.as_str().unwrap_or("").split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
            };
            Ok(items.iter().map(|item| {
                let name = item["Name"].as_str().unwrap_or("").to_string();
                let description = item["Description"].as_str().unwrap_or("").to_string();
                let nl = name.to_lowercase(); let dl = description.to_lowercase();
                let is_physical = !nl.contains("virtual") && !nl.contains("loopback")
                    && !dl.contains("virtual") && !dl.contains("loopback");
                let mut ips = csv(&item["IPv4"]);
                ips.extend(csv(&item["IPv6"]));
                NetworkAdapterDetail {
                    name, description,
                    mac_address: item["MacAddress"].as_str().unwrap_or("").to_string(),
                    ip_addresses: ips, subnet_masks: vec![],
                    default_gateway: csv(&item["Gateway"]),
                    dns_servers: csv(&item["DNS"]),
                    dhcp_enabled: item["DHCPEnabled"].as_bool().unwrap_or(false),
                    dhcp_server: String::new(),
                    speed_mbps: item["SpeedMbps"].as_u64().unwrap_or(0),
                    net_connection_id: item["Name"].as_str().unwrap_or("").to_string(),
                    is_physical,
                    status: item["Status"].as_str().unwrap_or("").to_string(),
                }
            }).collect())
        }
        #[cfg(not(target_os = "windows"))]
        Ok(vec![])
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_cpu_cache_info() -> Result<CpuCacheInfo, String> {
    tokio::task::spawn_blocking(|| {
        let wmi = wmi_con()?;
        let r: Vec<WmiCacheMemory> = wmi.raw_query("SELECT * FROM Win32_CacheMemory").map_err(|e| e.to_string())?;
        let mut l1i = 0u32; let mut l1d = 0u32;
        let mut l2 = 0u32; let mut l3 = 0u32; let mut l4 = 0u32;
        for m in r {
            let level = m.Level.unwrap_or(0);
            let size = m.InstalledSize.unwrap_or(0);
            let purpose = m.Purpose.unwrap_or_default().to_lowercase();
            match level {
                3 => if purpose.contains("instruction") { l1i += size; }
                     else if purpose.contains("data") { l1d += size; }
                     else { l1i += size / 2; l1d += size / 2; },
                4 => l2 += size,
                5 => l3 += size,
                6 => l4 += size,
                _ => {}
            }
        }
        Ok(CpuCacheInfo { l1_instruction_kb: l1i, l1_data_kb: l1d, l2_kb: l2, l3_kb: l3, l4_kb: l4 })
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_installed_software() -> Result<Vec<InstalledSoftware>, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command",
                    "$OutputEncoding=[System.Text.Encoding]::UTF8;[Console]::OutputEncoding=[System.Text.Encoding]::UTF8;\
                     Get-ItemProperty 'HKLM:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*',\
                     'HKLM:\\Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*' 2>$null |\
                     Where-Object { $_.DisplayName } |\
                     Select-Object DisplayName,DisplayVersion,Publisher,InstallDate,InstallLocation,EstimatedSize |\
                     Sort-Object DisplayName | ConvertTo-Json -Compress"
                ])
                .creation_flags(0x08000000).output().map_err(|e| e.to_string())?;
            let stdout = crate::maintenance::commands::decode_output(&out.stdout);
            if stdout.trim().is_empty() { return Ok(vec![]); }
            let items: Vec<serde_json::Value> = serde_json::from_str(stdout.trim())
                .unwrap_or_else(|_| serde_json::from_str(&format!("[{}]", stdout.trim())).unwrap_or_default());
            Ok(items.iter().filter_map(|item| {
                let name = item["DisplayName"].as_str().unwrap_or("").to_string();
                if name.is_empty() { return None; }
                Some(InstalledSoftware {
                    name,
                    version: item["DisplayVersion"].as_str().unwrap_or("").to_string(),
                    publisher: item["Publisher"].as_str().unwrap_or("").to_string(),
                    install_date: item["InstallDate"].as_str().unwrap_or("").to_string(),
                    install_location: item["InstallLocation"].as_str().unwrap_or("").to_string(),
                    estimated_size_mb: item["EstimatedSize"].as_u64().unwrap_or(0) as f64 / 1024.0,
                })
            }).collect())
        }
        #[cfg(not(target_os = "windows"))]
        Ok(vec![])
    }).await.map_err(|e| e.to_string())?
}
