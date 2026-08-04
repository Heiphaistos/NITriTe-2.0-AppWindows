use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use super::GpuScanItem;

// === Matériel étendu (GPU, résolution, plan, logiciels, services) ===

#[derive(Default)]
pub struct HwExtended {
    pub gpu_name: String, pub gpu_vram_mb: u64, pub all_gpus: Vec<GpuScanItem>, pub screen_resolution: String,
    pub power_plan: String, pub installed_software_count: u32,
    pub services_running: u32, pub services_stopped: u32,
    pub network_adapters_summary: String, pub cpu_temperature: String,
}

pub fn collect_hw_extended() -> HwExtended {
    let ps = r#"
$out = @{}
# GPU (iGPU + dGPU) — [PSCustomObject] obligatoire sinon ConvertTo-Json compresse en {} au lieu de [{}]
try {
    $gpuList = @(Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue)
    if ($gpuList.Count -eq 0) { $gpuList = @(Get-WmiObject Win32_VideoController -ErrorAction SilentlyContinue) }
    # VRAM 64-bit depuis le registre (AdapterRAM limité à uint32 = 4 GB max, bugué au-delà)
    # Utilise CurrentControlSet (toujours valide) et matching par DriverDesc plutôt que par index
    $regBase = "HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}"
    $regMap = @{}
    try {
        Get-ChildItem $regBase -EA SilentlyContinue |
            Where-Object { $_.PSChildName -match '^\d{4}$' } |
            ForEach-Object {
                $rp = Get-ItemProperty $_.PSPath -EA SilentlyContinue
                if ($rp -and $rp.DriverDesc) { $regMap[[string]$rp.DriverDesc] = $rp }
            }
    } catch {}
    $gpuArr = [System.Collections.Generic.List[object]]::new()
    $gIdx = 0
    foreach ($g in $gpuList) {
        $name = if ($g.Name) { [string]$g.Name } elseif ($g.Caption) { [string]$g.Caption } else { "GPU $gIdx" }
        # VRAM registre 64-bit — matching par DriverDesc (fiable, indépendant de l'ordre d'énumération)
        # qwMemorySize (QWORD) prioritaire : MemorySize (DWORD legacy) plafonne à 4 Go tout comme AdapterRAM
        # WMI, vérifié en direct (RTX 3070 Laptop : AdapterRAM=MemorySize=4293918720≈4Go tronqué,
        # qwMemorySize=8589934592=8Go exact) — l'ancien ordre (MemorySize avant qwMemorySize) ne corrigeait rien.
        $vramBytes = [long]0
        $rp = $regMap[$name]
        if (-not $rp -and $g.Caption -and [string]$g.Caption -ne $name) { $rp = $regMap[[string]$g.Caption] }
        if ($rp) {
            try {
                if ($rp.'HardwareInformation.qwMemorySize') {
                    $v = [long]$rp.'HardwareInformation.qwMemorySize'
                    if ($v -gt 1MB) { $vramBytes = $v }
                }
                if ($vramBytes -le 0 -and $rp.'HardwareInformation.MemorySize') {
                    $v = [long]$rp.'HardwareInformation.MemorySize'
                    if ($v -gt 1MB) { $vramBytes = $v }
                }
            } catch {}
        }
        # Fallback AdapterRAM — cast uint32 pour gérer overflow int32 sur GPU >= 4 GB
        if ($vramBytes -le 0 -and $g.AdapterRAM) {
            try { $raw = [long][uint32]$g.AdapterRAM; if ($raw -gt 1MB) { $vramBytes = $raw } } catch {}
        }
        $isInt = $name -match 'Intel.*(HD|UHD|Iris|GMA|Xe|Arc A[0-9]|Arc B[0-9])' -or
                 $name -match 'AMD.*(Radeon Graphics$|Vega \d+$|Renoir|Cezanne|Lucienne|Rembrandt|Phoenix|Hawk)' -or
                 $name -match 'Microsoft Basic Display|VirtualBox|VMware|Remote Desktop|Parsec|WDDM'
        # [PSCustomObject] essentiel : ConvertTo-Json sérialise toujours en tableau même si 1 seul élément
        $gpuArr.Add([PSCustomObject]@{ name = $name; vram = $vramBytes; integrated = [bool]$isInt }) | Out-Null
        $gIdx++
    }
    $out.Gpus = @($gpuArr)
    # Priorité au dGPU (non-intégré) pour GpuName/GpuVram principaux
    $dGpu = $gpuArr | Where-Object { -not $_.integrated } | Select-Object -First 1
    $out.GpuName = if ($dGpu) { $dGpu.name } elseif ($gpuArr.Count -gt 0) { $gpuArr[0].name } else { "" }
    $out.GpuVram = if ($dGpu) { $dGpu.vram } elseif ($gpuArr.Count -gt 0) { $gpuArr[0].vram } else { 0 }
    $out.Resolution = if ($gpuList.Count -gt 0) { "$($gpuList[0].CurrentHorizontalResolution)x$($gpuList[0].CurrentVerticalResolution) @ $($gpuList[0].CurrentRefreshRate)Hz" } else { "" }
} catch { $out.GpuName = ""; $out.GpuVram = 0; $out.Resolution = ""; $out.Gpus = @() }
# Plan d'alimentation
try {
    $plan = powercfg /getactivescheme 2>$null
    if ($plan -match '\((.+)\)') { $out.PowerPlan = $matches[1] } else { $out.PowerPlan = $plan -replace '.*:\s*','' }
} catch { $out.PowerPlan = "" }
# Logiciels installés
try {
    $sw = (Get-ItemProperty "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*" -ErrorAction SilentlyContinue |
        Where-Object { $_.DisplayName -and $_.DisplayName -ne "" } | Measure-Object).Count
    $sw += (Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*" -ErrorAction SilentlyContinue |
        Where-Object { $_.DisplayName -and $_.DisplayName -ne "" } | Measure-Object).Count
    $out.SoftCount = $sw
} catch { $out.SoftCount = 0 }
# Services Running/Stopped
try {
    $svcs = Get-Service -ErrorAction SilentlyContinue
    $out.SvcRunning = ($svcs | Where-Object {$_.Status -eq 'Running'} | Measure-Object).Count
    $out.SvcStopped = ($svcs | Where-Object {$_.Status -eq 'Stopped'} | Measure-Object).Count
} catch { $out.SvcRunning = 0; $out.SvcStopped = 0 }
# Adaptateurs réseau actifs
try {
    $adapters = Get-NetAdapter -ErrorAction SilentlyContinue | Where-Object { $_.Status -eq 'Up' }
    $names = @()
    foreach ($a in $adapters) {
        $ip = (Get-NetIPAddress -InterfaceIndex $a.InterfaceIndex -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object -First 1).IPAddress
        $names += "$($a.Name) — $ip"
    }
    $out.NetSummary = $names -join " | "
} catch { $out.NetSummary = "" }
# Température CPU — essaie LHM → OHM → ACPI → WMI en cascade
$out.CpuTemp = "N/A"
try {
    $t = Get-WmiObject -Namespace "root\LibreHardwareMonitor" -Class Sensor -ErrorAction SilentlyContinue |
         Where-Object { $_.SensorType -eq 'Temperature' -and $_.Name -match 'CPU Package|CPU|Processeur' } |
         Sort-Object { if ($_.Name -match 'Package') { 0 } else { 1 } } | Select-Object -First 1
    if ($t) { $out.CpuTemp = "$([math]::Round([double]$t.Value,1))°C" }
} catch {}
if ($out.CpuTemp -eq "N/A") {
    try {
        $t = Get-WmiObject -Namespace "root\OpenHardwareMonitor" -Class Sensor -ErrorAction SilentlyContinue |
             Where-Object { $_.SensorType -eq 'Temperature' -and $_.Name -like '*CPU*' } |
             Select-Object -First 1
        if ($t) { $out.CpuTemp = "$([math]::Round([double]$t.Value,1))°C" }
    } catch {}
}
if ($out.CpuTemp -eq "N/A") {
    try {
        $tz = Get-WmiObject -Namespace 'root/wmi' -Class MSAcpi_ThermalZoneTemperature -ErrorAction Stop |
              Select-Object -First 1
        if ($tz) {
            $c = [math]::Round(($tz.CurrentTemperature - 2732) / 10.0, 1)
            if ($c -gt 5 -and $c -lt 120) { $out.CpuTemp = "${c}°C" }
        }
    } catch {}
}
if ($out.CpuTemp -eq "N/A") {
    try {
        $wt = Get-WmiObject -Namespace 'root/cimv2' -Class Win32_PerfFormattedData_Counters_ThermalZoneInformation -ErrorAction Stop |
              Select-Object -First 1
        if ($wt -and $wt.HighPrecisionTemperature -and [double]$wt.HighPrecisionTemperature -gt 0) {
            $c = [math]::Round([double]$wt.HighPrecisionTemperature / 10.0 - 273.15, 1)
            if ($c -gt 5 -and $c -lt 120) { $out.CpuTemp = "${c}°C" }
        }
    } catch {}
}
$out | ConvertTo-Json -Depth 4 -Compress
"#;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps])
        .creation_flags(0x08000000)
        .output();
    if let Ok(o) = output {
        // decode_output : PowerPlan (nom du plan actif, ex "Équilibré") et CpuTemp
        // ("°C") sont accentués/non-ASCII, sans préambule $OutputEncoding.
        let text = crate::maintenance::commands::decode_output(&o.stdout);
        let v: serde_json::Value = serde_json::from_str(text.trim()).unwrap_or_default();
        let all_gpus = v["Gpus"].as_array().map(|arr| {
            arr.iter().map(|g| GpuScanItem {
                name: g["name"].as_str().unwrap_or("").to_string(),
                vram_mb: g["vram"].as_u64().unwrap_or(0) / 1_048_576,
                is_integrated: g["integrated"].as_bool().unwrap_or(false),
            }).collect()
        }).unwrap_or_default();
        return HwExtended {
            gpu_name: v["GpuName"].as_str().unwrap_or("").to_string(),
            gpu_vram_mb: v["GpuVram"].as_u64().unwrap_or(0) / 1_048_576,
            all_gpus,
            screen_resolution: v["Resolution"].as_str().unwrap_or("").to_string(),
            power_plan: v["PowerPlan"].as_str().unwrap_or("").trim().to_string(),
            installed_software_count: v["SoftCount"].as_u64().unwrap_or(0) as u32,
            services_running: v["SvcRunning"].as_u64().unwrap_or(0) as u32,
            services_stopped: v["SvcStopped"].as_u64().unwrap_or(0) as u32,
            network_adapters_summary: v["NetSummary"].as_str().unwrap_or("").to_string(),
            cpu_temperature: v["CpuTemp"].as_str().unwrap_or("N/A").to_string(),
        };
    }
    HwExtended::default()
}
