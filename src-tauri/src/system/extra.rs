use serde::Serialize;
use std::process::Command;
use wmi::{COMLibrary, WMIConnection};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// ============================================================================
// Volumes / Partitions
// ============================================================================

#[derive(Debug, Serialize, Clone)]
pub struct VolumeInfo {
    pub drive_letter: String,
    pub label: String,
    pub filesystem: String,
    pub total_gb: f64,
    pub free_gb: f64,
    pub used_gb: f64,
    pub used_percent: f64,
    pub drive_type: String,
    pub health_status: String,
    pub operational_status: String,
    pub serial_number: String,
}

fn wmi_con() -> Result<WMIConnection, String> {
    let com = COMLibrary::new().map_err(|e| format!("COM: {}", e))?;
    WMIConnection::new(com).map_err(|e| format!("WMI: {}", e))
}

pub fn collect_volumes() -> Result<Vec<VolumeInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        let ps = r#"
try {
    $vols = Get-Volume | Where-Object { $null -ne $_.DriveLetter -and $_.DriveLetter -ne '' }
    $vols | ForEach-Object {
        [PSCustomObject]@{
            letter = [string]$_.DriveLetter
            label  = [string]$_.FileSystemLabel
            fs     = [string]$_.FileSystem
            free   = [long]$_.SizeRemaining
            total  = [long]$_.Size
            dtype  = [string]$_.DriveType
            health = [string]$_.HealthStatus
            opstat = [string]$_.OperationalStatus
        }
    } | ConvertTo-Json -Compress
} catch { Write-Output '[]' }
"#;
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| format!("PowerShell: {}", e))?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let text = stdout.trim();
        if text.is_empty() || text == "[]" { return Ok(vec![]); }
        let json_text = if text.starts_with('{') { format!("[{}]", text) } else { text.to_string() };
        let items: Vec<serde_json::Value> = serde_json::from_str(&json_text).unwrap_or_default();
        Ok(items.into_iter().filter_map(|v| {
            let total = v["total"].as_f64().unwrap_or(0.0);
            let free = v["free"].as_f64().unwrap_or(0.0);
            let used = (total - free).max(0.0);
            let letter = v["letter"].as_str().unwrap_or("").trim().to_string();
            if letter.is_empty() { return None; }
            Some(VolumeInfo {
                drive_letter: format!("{}:", letter),
                label: v["label"].as_str().unwrap_or("").trim().to_string(),
                filesystem: v["fs"].as_str().unwrap_or("").trim().to_string(),
                total_gb: (total / 1_073_741_824.0 * 100.0).round() / 100.0,
                free_gb: (free / 1_073_741_824.0 * 100.0).round() / 100.0,
                used_gb: (used / 1_073_741_824.0 * 100.0).round() / 100.0,
                used_percent: if total > 0.0 { (used / total * 1000.0).round() / 10.0 } else { 0.0 },
                drive_type: v["dtype"].as_str().unwrap_or("").trim().to_string(),
                health_status: v["health"].as_str().unwrap_or("Unknown").trim().to_string(),
                operational_status: v["opstat"].as_str().unwrap_or("").trim().to_string(),
                serial_number: String::new(),
            })
        }).collect())
    }
    #[cfg(not(target_os = "windows"))]
    Ok(vec![])
}

// ============================================================================
// CPU Extended Info
// ============================================================================

#[derive(Debug, Serialize, Clone)]
pub struct CpuExtended {
    pub socket_designation: String,
    pub family: String,
    pub stepping: String,
    pub revision: String,
    pub processor_id: String,
    pub max_clock_speed_mhz: u32,
    pub current_voltage: String,
    pub external_clock_mhz: u32,
    pub l2_cache_size_kb: u32,
    pub l3_cache_size_kb: u32,
    pub number_of_physical_packages: usize,
    pub virtualization_enabled: bool,
    pub address_width: u32,
    pub data_width: u32,
    pub characteristics: Vec<String>,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct WmiProcessor {
    SocketDesignation: Option<String>,
    Family: Option<u32>,
    Stepping: Option<String>,
    Revision: Option<u32>,
    ProcessorId: Option<String>,
    MaxClockSpeed: Option<u32>,
    CurrentVoltage: Option<u32>,
    ExternalClock: Option<u32>,
    L2CacheSize: Option<u32>,
    L3CacheSize: Option<u32>,
    VirtualizationFirmwareEnabled: Option<bool>,
    AddressWidth: Option<u32>,
    DataWidth: Option<u32>,
}

pub fn collect_cpu_extended() -> Result<CpuExtended, String> {
    let wmi = wmi_con()?;
    let results: Vec<WmiProcessor> = wmi
        .raw_query("SELECT * FROM Win32_Processor")
        .map_err(|e| e.to_string())?;

    let pkgs = results.len();
    let p = results.into_iter().next().ok_or("No CPU found")?;

    let family_str = match p.Family.unwrap_or(0) {
        3 => "Intel 8086", 6 => "Intel 486", 11 => "Other Intel",
        17 => "Intel 80386", 19 => "Intel 80486", 26 => "Pentium",
        97 => "Pentium II", 171 => "Core i3", 172 => "Core i5",
        173 => "Core i7", 174 => "Core i9", 182 => "Intel Core",
        198 => "Intel Core Ultra", 200 => "AMD duron", 203 => "AMD Athlon XP",
        220 => "AMD Opteron", 221 => "AMD Athlon 64", 222 => "AMD FX",
        232 => "AMD Ryzen 3", 233 => "AMD Ryzen 5", 234 => "AMD Ryzen 7",
        235 => "AMD Ryzen 9", 252 => "AMD Ryzen",
        _ => "Autre",
    };

    // Voltage: WMI CurrentVoltage = voltage in 1/10 volt (e.g., 12 = 1.2V)
    let voltage_str = p.CurrentVoltage
        .map(|v| if v > 0 { format!("{:.1}V", v as f64 / 10.0) } else { "N/A".to_string() })
        .unwrap_or_else(|| "N/A".to_string());

    Ok(CpuExtended {
        socket_designation: p.SocketDesignation.unwrap_or_default().trim().to_string(),
        family: family_str.to_string(),
        stepping: p.Stepping.unwrap_or_default().trim().to_string(),
        revision: p.Revision.map(|r| format!("{}", r)).unwrap_or_default(),
        processor_id: p.ProcessorId.unwrap_or_default().trim().to_string(),
        max_clock_speed_mhz: p.MaxClockSpeed.unwrap_or(0),
        current_voltage: voltage_str,
        external_clock_mhz: p.ExternalClock.unwrap_or(0),
        l2_cache_size_kb: p.L2CacheSize.unwrap_or(0),
        l3_cache_size_kb: p.L3CacheSize.unwrap_or(0),
        number_of_physical_packages: pkgs,
        virtualization_enabled: p.VirtualizationFirmwareEnabled.unwrap_or(false),
        address_width: p.AddressWidth.unwrap_or(64),
        data_width: p.DataWidth.unwrap_or(64),
        characteristics: vec![],
    })
}

// ============================================================================
// OS Extended Info
// ============================================================================

#[derive(Debug, Serialize, Clone)]
pub struct OsExtended {
    pub install_date: String,
    pub registered_user: String,
    pub organization: String,
    pub windows_directory: String,
    pub system_directory: String,
    pub locale: String,
    pub country_code: String,
    pub system_drive: String,
    pub boot_device: String,
    pub total_virtual_memory_gb: f64,
    pub free_virtual_memory_gb: f64,
    pub total_swap_gb: f64,
    pub free_physical_gb: f64,
    pub timezone: String,
    pub dotnet_versions: Vec<String>,
    pub powershell_version: String,
    pub pending_reboot: bool,
    pub last_boot_time: String,
    pub page_file_path: String,
    pub os_language: String,
    pub product_type: String,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct WmiOS {
    InstallDate: Option<String>,
    RegisteredUser: Option<String>,
    Organization: Option<String>,
    WindowsDirectory: Option<String>,
    SystemDirectory: Option<String>,
    Locale: Option<String>,
    CountryCode: Option<String>,
    SystemDrive: Option<String>,
    BootDevice: Option<String>,
    TotalVirtualMemorySize: Option<u64>,
    FreeVirtualMemory: Option<u64>,
    TotalSwapSpaceSize: Option<u64>,
    FreePhysicalMemory: Option<u64>,
    LastBootUpTime: Option<String>,
    OSLanguage: Option<u32>,
    ProductType: Option<u32>,
}

fn parse_wmi_date(s: &str) -> String {
    if s.len() >= 14 {
        let y = &s[0..4]; let m = &s[4..6]; let d = &s[6..8];
        let hh = &s[8..10]; let mm = &s[10..12]; let ss = &s[12..14];
        format!("{}/{}/{} {}:{}:{}", d, m, y, hh, mm, ss)
    } else if s.len() >= 8 {
        format!("{}/{}/{}", &s[6..8], &s[4..6], &s[0..4])
    } else { s.to_string() }
}

pub fn collect_os_extended() -> Result<OsExtended, String> {
    let wmi = wmi_con()?;
    let results: Vec<WmiOS> = wmi
        .raw_query("SELECT * FROM Win32_OperatingSystem")
        .map_err(|e| e.to_string())?;
    let os = results.into_iter().next().ok_or("No OS found")?;

    // PowerShell for extras
    #[cfg(target_os = "windows")]
    let ps_data = {
        let ps = r#"
$out = @{}
# Timezone
$out.Timezone = (Get-TimeZone).DisplayName
# .NET versions
try {
    $dotnet = Get-ChildItem 'HKLM:\SOFTWARE\Microsoft\NET Framework Setup\NDP' -Recurse |
        Get-ItemProperty -Name Version -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty Version | Sort-Object -Unique
    $out.DotNet = ($dotnet -join ',')
} catch { $out.DotNet = "" }
# PowerShell
$out.PSVersion = $PSVersionTable.PSVersion.ToString()
# Pending reboot
$pendingReboot = $false
try { if (Get-Item "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending" -ErrorAction SilentlyContinue) { $pendingReboot = $true } } catch {}
try { if ((Get-ItemProperty "HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager" -ErrorAction SilentlyContinue).PendingFileRenameOperations) { $pendingReboot = $true } } catch {}
$out.PendingReboot = $pendingReboot
# Page file
try {
    $pf = Get-CimInstance -ClassName Win32_PageFileUsage -ErrorAction SilentlyContinue | Select-Object -First 1
    $out.PageFilePath = if ($pf) { $pf.Name } else { "N/A" }
} catch { $out.PageFilePath = "N/A" }
$out | ConvertTo-Json -Compress
"#;
        let raw_out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| e.to_string())?;
        let raw = String::from_utf8_lossy(&raw_out.stdout).trim().to_string();
        serde_json::from_str::<serde_json::Value>(&raw).unwrap_or_default()
    };
    #[cfg(not(target_os = "windows"))]
    let ps_data = serde_json::Value::Null;

    let dotnet_raw = ps_data["DotNet"].as_str().unwrap_or("").to_string();
    let dotnet_versions: Vec<String> = if dotnet_raw.is_empty() { vec![] }
        else { dotnet_raw.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect() };

    let lang_code = os.OSLanguage.unwrap_or(0);
    let os_language = match lang_code {
        1036 => "Français", 1033 => "English (US)", 2052 => "Chinese Simplified",
        1031 => "Deutsch", 1034 => "Español", 1040 => "Italiano",
        1041 => "Japanese", 1049 => "Russian", 1046 => "Português",
        _ => "Inconnu",
    };
    let product_type = match os.ProductType.unwrap_or(0) {
        1 => "Workstation", 2 => "Domain Controller", 3 => "Server", _ => "Inconnu",
    };

    Ok(OsExtended {
        install_date: os.InstallDate.as_deref().map(parse_wmi_date).unwrap_or_default(),
        registered_user: os.RegisteredUser.unwrap_or_default().trim().to_string(),
        organization: os.Organization.unwrap_or_default().trim().to_string(),
        windows_directory: os.WindowsDirectory.unwrap_or_default().trim().to_string(),
        system_directory: os.SystemDirectory.unwrap_or_default().trim().to_string(),
        locale: os.Locale.unwrap_or_default().trim().to_string(),
        country_code: os.CountryCode.unwrap_or_default().trim().to_string(),
        system_drive: os.SystemDrive.unwrap_or_default().trim().to_string(),
        boot_device: os.BootDevice.unwrap_or_default().trim().to_string(),
        total_virtual_memory_gb: os.TotalVirtualMemorySize.unwrap_or(0) as f64 / 1_048_576.0,
        free_virtual_memory_gb: os.FreeVirtualMemory.unwrap_or(0) as f64 / 1_048_576.0,
        total_swap_gb: os.TotalSwapSpaceSize.unwrap_or(0) as f64 / 1_048_576.0,
        free_physical_gb: os.FreePhysicalMemory.unwrap_or(0) as f64 / 1_048_576.0,
        last_boot_time: os.LastBootUpTime.as_deref().map(parse_wmi_date).unwrap_or_default(),
        timezone: ps_data["Timezone"].as_str().unwrap_or("N/A").to_string(),
        dotnet_versions,
        powershell_version: ps_data["PSVersion"].as_str().unwrap_or("N/A").to_string(),
        pending_reboot: ps_data["PendingReboot"].as_bool().unwrap_or(false),
        page_file_path: ps_data["PageFilePath"].as_str().unwrap_or("N/A").to_string(),
        os_language: os_language.to_string(),
        product_type: product_type.to_string(),
    })
}

// ============================================================================
// Folder Sizes (expose extended_info.rs version as command)
// ============================================================================

#[derive(Debug, Serialize, Clone)]
pub struct FolderSizeInfo {
    pub label: String,
    pub path: String,
    pub size_mb: f64,
    pub size_gb: f64,
    pub file_count: u64,
}

pub fn collect_folder_sizes() -> Vec<FolderSizeInfo> {
    #[cfg(target_os = "windows")]
    {
        // Script unique : tous les dossiers en jobs parallèles, timeout 45s global
        let ps = r#"
$items = @(
    @{L='Bureau';          P=[Environment]::GetFolderPath('Desktop')},
    @{L='Documents';       P=[Environment]::GetFolderPath('MyDocuments')},
    @{L='Téléchargements'; P=[IO.Path]::Combine($env:USERPROFILE,'Downloads')},
    @{L='Images';          P=[Environment]::GetFolderPath('MyPictures')},
    @{L='Vidéos';          P=[Environment]::GetFolderPath('MyVideos')},
    @{L='Musique';         P=[Environment]::GetFolderPath('MyMusic')},
    @{L='AppData Roaming'; P=[Environment]::GetFolderPath('ApplicationData')},
    @{L='AppData Local';   P=[Environment]::GetFolderPath('LocalApplicationData')},
    @{L='Temp';            P=$env:TEMP},
    @{L='Windows\Temp';    P='C:\Windows\Temp'},
    @{L='Program Files';   P=[Environment]::GetFolderPath('ProgramFiles')},
    @{L='Program Files x86'; P=[Environment]::GetFolderPath('ProgramFilesX86')},
    @{L='ProgramData';     P=$env:ProgramData}
)
$global_sw = [System.Diagnostics.Stopwatch]::StartNew()
$results = [System.Collections.Generic.List[object]]::new()
foreach ($item in $items) {
    $p = $item.P; $l = $item.L
    if (-not $p -or -not (Test-Path $p -ErrorAction SilentlyContinue)) {
        $results.Add([PSCustomObject]@{label=$l;path=$p;bytes=0.0;count=0})
        continue
    }
    $fsw = [System.Diagnostics.Stopwatch]::StartNew()
    $bytes = 0.0; $count = [long]0
    try {
        $di = [IO.DirectoryInfo]::new($p)
        foreach ($f in $di.EnumerateFiles('*', [IO.SearchOption]::AllDirectories)) {
            if ($fsw.Elapsed.TotalSeconds -gt 6) { break }
            try { $bytes += $f.Length; $count++ } catch {}
        }
    } catch {}
    $results.Add([PSCustomObject]@{label=$l;path=$p;bytes=$bytes;count=$count})
    if ($global_sw.Elapsed.TotalSeconds -gt 40) { break }
}
if ($results.Count -gt 0) { $results | ConvertTo-Json -Compress } else { '[]' }
"#;
        use std::io::Read;
        let mut child = match std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn() {
                Ok(c) => c,
                Err(_) => return vec![],
            };
        let timeout = std::time::Duration::from_secs(55);
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    let mut buf = Vec::new();
                    if let Some(mut stdout) = child.stdout.take() {
                        let _ = stdout.read_to_end(&mut buf);
                    }
                    let text = String::from_utf8_lossy(&buf);
                    let trimmed = text.trim();
                    if trimmed.is_empty() || trimmed == "[]" { return vec![]; }
                    let json_text = if trimmed.starts_with('{') { format!("[{}]", trimmed) } else { trimmed.to_string() };
                    let items: Vec<serde_json::Value> = serde_json::from_str(&json_text).unwrap_or_default();
                    return items.into_iter().filter_map(|v| {
                        let label = v["label"].as_str()?.to_string();
                        let path = v["path"].as_str().unwrap_or("").to_string();
                        let bytes = v["bytes"].as_f64().unwrap_or(0.0);
                        let count = v["count"].as_u64().unwrap_or(0);
                        let size_mb = (bytes / 1_048_576.0 * 100.0).round() / 100.0;
                        Some(FolderSizeInfo { label, path, size_mb, size_gb: (size_mb / 1024.0 * 100.0).round() / 100.0, file_count: count })
                    }).collect();
                }
                Ok(None) => {
                    if start.elapsed() > timeout { let _ = child.kill(); let _ = child.wait(); return vec![]; }
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
                Err(_) => { let _ = child.kill(); return vec![]; }
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    vec![]
}

// ============================================================================
// Startup Programs (enhanced)
// ============================================================================

#[derive(Debug, Serialize, Clone)]
pub struct StartupProgram {
    pub name: String,
    pub command: String,
    pub location: String,
    pub user: String,
    pub enabled: bool,
    pub category: String,
}

pub fn collect_startup_programs() -> Vec<StartupProgram> {
    #[cfg(target_os = "windows")]
    {
        let ps = r#"
$result = @()
$keys = @(
    @{ Key="HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run"; Loc="HKCU\Run"; User="Utilisateur courant" },
    @{ Key="HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run"; Loc="HKLM\Run"; User="Tous les utilisateurs" },
    @{ Key="HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce"; Loc="HKCU\RunOnce"; User="Utilisateur courant" },
    @{ Key="HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce"; Loc="HKLM\RunOnce"; User="Tous les utilisateurs" },
    @{ Key="HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run"; Loc="HKLM\Run (x86)"; User="Tous les utilisateurs" }
)
foreach ($k in $keys) {
    try {
        $props = Get-ItemProperty -Path $k.Key -ErrorAction SilentlyContinue
        if ($props) {
            $props.PSObject.Properties | Where-Object { $_.Name -notlike "PS*" } | ForEach-Object {
                $result += [PSCustomObject]@{
                    Name = $_.Name
                    Command = $_.Value
                    Location = $k.Loc
                    User = $k.User
                    Enabled = $true
                }
            }
        }
    } catch {}
}
# Startup folders
$startupPaths = @(
    @{ Path=[System.Environment]::GetFolderPath("Startup"); Loc="Dossier démarrage utilisateur"; User="Courant" },
    @{ Path=[System.Environment]::GetFolderPath("CommonStartup"); Loc="Dossier démarrage commun"; User="Tous" }
)
foreach ($sp in $startupPaths) {
    if (Test-Path $sp.Path) {
        Get-ChildItem $sp.Path -File -ErrorAction SilentlyContinue | ForEach-Object {
            $result += [PSCustomObject]@{
                Name = $_.BaseName
                Command = $_.FullName
                Location = $sp.Loc
                User = $sp.User
                Enabled = $true
            }
        }
    }
}
$result | ConvertTo-Json -Compress
"#;
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output();
        if let Ok(o) = out {
            let raw = String::from_utf8_lossy(&o.stdout);
            let trimmed = raw.trim();
            if !trimmed.is_empty() && trimmed != "[]" {
                let arr: Vec<serde_json::Value> = serde_json::from_str(trimmed)
                    .unwrap_or_else(|_| serde_json::from_str(&format!("[{}]", trimmed)).unwrap_or_default());
                return arr.iter().filter_map(|v| {
                    let name = v["Name"].as_str()?.to_string();
                    let cmd = v["Command"].as_str().unwrap_or("").to_string();
                    let cat = categorize_startup(&name, &cmd);
                    Some(StartupProgram {
                        name,
                        command: cmd,
                        location: v["Location"].as_str().unwrap_or("").to_string(),
                        user: v["User"].as_str().unwrap_or("").to_string(),
                        enabled: v["Enabled"].as_bool().unwrap_or(true),
                        category: cat,
                    })
                }).collect();
            }
        }
        vec![]
    }
    #[cfg(not(target_os = "windows"))]
    vec![]
}

// ============================================================================
// SMART / Physical Disk Health
// ============================================================================

#[derive(Debug, Serialize, Clone)]
pub struct SmartDiskInfo {
    pub device_id: String,
    pub name: String,
    pub media_type: String,
    pub health_status: String,
    pub operational_status: String,
    pub size_gb: f64,
    pub temperature: i32,
    pub wear_level: i32,
    pub read_errors_uncorrected: u64,
    pub write_errors_uncorrected: u64,
    pub power_on_hours: u64,
    pub start_stop_cycles: u64,
    pub reallocated_sectors: u64,
}

pub fn collect_smart_info() -> Vec<SmartDiskInfo> {
    #[cfg(target_os = "windows")]
    {
        // Chaque Get-StorageReliabilityCounter est wrappé dans un job avec timeout 6s
        // pour éviter de bloquer indéfiniment sur des contrôleurs RAID/USB
        let ps = r#"
try {
    $disks = Get-PhysicalDisk -ErrorAction SilentlyContinue
    if (-not $disks) { Write-Output '[]'; exit }
    $result = @()
    foreach ($disk in $disks) {
        $rc = $null
        try {
            $job = Start-Job -ScriptBlock {
                param($d) Get-StorageReliabilityCounter -PhysicalDisk $d -ErrorAction SilentlyContinue
            } -ArgumentList $disk
            $done = $job | Wait-Job -Timeout 6
            if ($done) { $rc = Receive-Job $job -ErrorAction SilentlyContinue }
            Remove-Job $job -Force -ErrorAction SilentlyContinue
        } catch {}
        $result += [PSCustomObject]@{
            id      = [string]$disk.DeviceId
            name    = [string]$disk.FriendlyName
            mtype   = [string]$disk.MediaType
            health  = [string]$disk.HealthStatus
            opstat  = [string]$disk.OperationalStatus
            size    = [long]$disk.Size
            temp    = if ($rc -and $rc.Temperature)              { [int]$rc.Temperature }                  else { -1 }
            wear    = if ($rc -and $null -ne $rc.Wear)           { [int]$rc.Wear }                         else { -1 }
            rderr   = if ($rc -and $rc.ReadErrorsUncorrected)   { [long]$rc.ReadErrorsUncorrected }        else { 0 }
            wrerr   = if ($rc -and $rc.WriteErrorsUncorrected)  { [long]$rc.WriteErrorsUncorrected }       else { 0 }
            poh     = if ($rc -and $rc.PowerOnHours)            { [long]$rc.PowerOnHours }                else { 0 }
            ssc     = if ($rc -and $rc.StartStopCycleCount)     { [long]$rc.StartStopCycleCount }         else { 0 }
            realloc = if ($rc -and $rc.ReallocatedSectors)      { [long]$rc.ReallocatedSectors }          else { 0 }
        }
    }
    $result | ConvertTo-Json -Compress
} catch { Write-Output '[]' }
"#;
        use std::io::Read;
        let mut child = match std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn() {
                Ok(c) => c,
                Err(_) => return vec![],
            };
        // Timeout global 30s (nb_disks * 6s + overhead)
        let timeout = std::time::Duration::from_secs(30);
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    let mut buf = Vec::new();
                    if let Some(mut stdout) = child.stdout.take() {
                        let _ = stdout.read_to_end(&mut buf);
                    }
                    let text = String::from_utf8_lossy(&buf);
                    let trimmed = text.trim();
                    if trimmed.is_empty() || trimmed == "[]" { return vec![]; }
                    let json_text = if trimmed.starts_with('{') { format!("[{}]", trimmed) } else { trimmed.to_string() };
                    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&json_text) {
                        return items.into_iter().map(|v| SmartDiskInfo {
                            device_id: v["id"].as_str().unwrap_or("").to_string(),
                            name: v["name"].as_str().unwrap_or("").to_string(),
                            media_type: v["mtype"].as_str().unwrap_or("").to_string(),
                            health_status: v["health"].as_str().unwrap_or("Unknown").to_string(),
                            operational_status: v["opstat"].as_str().unwrap_or("").to_string(),
                            size_gb: (v["size"].as_f64().unwrap_or(0.0) / 1_073_741_824.0 * 10.0).round() / 10.0,
                            temperature: v["temp"].as_i64().unwrap_or(-1) as i32,
                            wear_level: v["wear"].as_i64().unwrap_or(-1) as i32,
                            read_errors_uncorrected: v["rderr"].as_u64().unwrap_or(0),
                            write_errors_uncorrected: v["wrerr"].as_u64().unwrap_or(0),
                            power_on_hours: v["poh"].as_u64().unwrap_or(0),
                            start_stop_cycles: v["ssc"].as_u64().unwrap_or(0),
                            reallocated_sectors: v["realloc"].as_u64().unwrap_or(0),
                        }).collect();
                    }
                    return vec![];
                }
                Ok(None) => {
                    if start.elapsed() > timeout { let _ = child.kill(); let _ = child.wait(); return vec![]; }
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
                Err(_) => { let _ = child.kill(); return vec![]; }
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    vec![]
}

fn categorize_startup(name: &str, cmd: &str) -> String {
    let nl = name.to_lowercase();
    let cl = cmd.to_lowercase();
    if cl.contains("microsoft") || cl.contains("windows") || nl.contains("windows") || cl.contains("\\windows\\") {
        "Microsoft / Windows".to_string()
    } else if cl.contains("nvidia") || cl.contains("amd") || cl.contains("intel") {
        "Pilotes".to_string()
    } else if cl.contains("google") || cl.contains("chrome") || cl.contains("firefox") || cl.contains("edge") {
        "Navigateur".to_string()
    } else if cl.contains("steam") || cl.contains("epic") || cl.contains("gaming") {
        "Jeux".to_string()
    } else if cl.contains("antivirus") || cl.contains("malware") || cl.contains("defender") || cl.contains("security") {
        "Sécurité".to_string()
    } else {
        "Tiers".to_string()
    }
}
