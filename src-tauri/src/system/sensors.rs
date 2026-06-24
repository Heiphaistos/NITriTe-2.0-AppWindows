use serde::Serialize;
use crate::utils::ps::ps;

#[derive(Serialize, Default, Clone)]
pub struct SensorReading {
    pub hardware: String,
    pub hardware_type: String,
    pub name: String,
    pub sensor_type: String,
    pub value: f64,
    pub unit: String,
    pub min: f64,
    pub max: f64,
    pub source: String,
}

fn unit_for(sensor_type: &str) -> &'static str {
    match sensor_type {
        "Temperature" => "°C",
        "Fan"         => "RPM",
        "Voltage"     => "V",
        "Load"        => "%",
        "Clock"       => "MHz",
        "Power"       => "W",
        "Control"     => "%",
        "Data"        => "GB",
        "SmallData"   => "MB",
        _             => "",
    }
}

fn hw_type_label(hw_type: &str) -> &'static str {
    match hw_type {
        "CPU"              => "CPU",
        "GpuNvidia"        => "GPU NVIDIA",
        "GpuAmd"           => "GPU AMD",
        "GpuIntel"         => "GPU Intel",
        "Motherboard"      => "Carte mère",
        "SuperIO"          => "SuperIO",
        "Ram"              => "RAM",
        "HDD"              => "Stockage",
        "SSD"              => "Stockage",
        "NVMe"             => "Stockage NVMe",
        "Network"          => "Réseau",
        "EmbeddedController" => "EC",
        _                  => "Autre",
    }
}

fn query_lhm() -> Vec<SensorReading> {
    let script = r#"
$ns = 'root/LibreHardwareMonitor'
$sensors = @()
try {
    $hwList = Get-WmiObject -Namespace $ns -Class Hardware -ErrorAction Stop
    $hwMap  = @{}
    foreach ($h in $hwList) { $hwMap[$h.Identifier] = $h }
    $sensorList = Get-WmiObject -Namespace $ns -Class Sensor -ErrorAction Stop
    foreach ($s in $sensorList) {
        $hw = $hwMap[$s.Parent]
        $hwName = if ($hw) { $hw.Name } else { $s.Parent }
        $hwType = if ($hw) { $hw.HardwareType } else { '' }
        $sensors += [PSCustomObject]@{
            hardware     = $hwName
            hardware_type= $hwType
            name         = $s.Name
            sensor_type  = $s.SensorType
            value        = [double]$s.Value
            min          = [double]$s.Min
            max          = [double]$s.Max
        }
    }
} catch {}
@($sensors) | ConvertTo-Json -Compress -Depth 2
"#;
    let out = ps(script).unwrap_or_default();
    if out.trim().is_empty() || out.trim() == "null" { return vec![]; }
    let arr: Vec<serde_json::Value> = serde_json::from_str(out.trim()).unwrap_or_default();
    arr.iter().filter_map(|v| {
        let value = v["value"].as_f64().unwrap_or(0.0);
        if value == 0.0 && v["sensor_type"].as_str().unwrap_or("") != "Voltage" { return None; }
        let st = v["sensor_type"].as_str().unwrap_or("").to_string();
        let hw_type = v["hardware_type"].as_str().unwrap_or("").to_string();
        Some(SensorReading {
            hardware:      v["hardware"].as_str().unwrap_or("").to_string(),
            hardware_type: hw_type_label(&hw_type).to_string(),
            name:          v["name"].as_str().unwrap_or("").to_string(),
            unit:          unit_for(&st).to_string(),
            min:           v["min"].as_f64().unwrap_or(0.0),
            max:           v["max"].as_f64().unwrap_or(0.0),
            sensor_type:   st,
            value,
            source:        "LibreHardwareMonitor".to_string(),
        })
    }).collect()
}

fn query_ohm() -> Vec<SensorReading> {
    let script = r#"
$ns = 'root/OpenHardwareMonitor'
$sensors = @()
try {
    $hwList = Get-WmiObject -Namespace $ns -Class Hardware -ErrorAction Stop
    $hwMap  = @{}
    foreach ($h in $hwList) { $hwMap[$h.Identifier] = $h }
    $sensorList = Get-WmiObject -Namespace $ns -Class Sensor -ErrorAction Stop
    foreach ($s in $sensorList) {
        $hw = $hwMap[$s.Parent]
        $hwName = if ($hw) { $hw.Name } else { $s.Parent }
        $hwType = if ($hw) { $hw.HardwareType } else { '' }
        $sensors += [PSCustomObject]@{
            hardware=$hwName; hardware_type=$hwType; name=$s.Name;
            sensor_type=$s.SensorType; value=[double]$s.Value; min=[double]$s.Min; max=[double]$s.Max
        }
    }
} catch {}
@($sensors) | ConvertTo-Json -Compress -Depth 2
"#;
    let out = ps(script).unwrap_or_default();
    if out.trim().is_empty() || out.trim() == "null" { return vec![]; }
    let arr: Vec<serde_json::Value> = serde_json::from_str(out.trim()).unwrap_or_default();
    arr.iter().filter_map(|v| {
        let value = v["value"].as_f64().unwrap_or(0.0);
        if value == 0.0 { return None; }
        let st = v["sensor_type"].as_str().unwrap_or("").to_string();
        let hw_type = v["hardware_type"].as_str().unwrap_or("").to_string();
        Some(SensorReading {
            hardware:      v["hardware"].as_str().unwrap_or("").to_string(),
            hardware_type: hw_type_label(&hw_type).to_string(),
            name:          v["name"].as_str().unwrap_or("").to_string(),
            unit:          unit_for(&st).to_string(),
            min:           v["min"].as_f64().unwrap_or(0.0),
            max:           v["max"].as_f64().unwrap_or(0.0),
            sensor_type:   st,
            value,
            source:        "OpenHardwareMonitor".to_string(),
        })
    }).collect()
}

fn query_nvidia_smi() -> Vec<SensorReading> {
    use std::process::Command;
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    // Chemins absolus connus uniquement — pas de fallback PATH pour éviter le PATH hijacking
    let known_paths = [
        r"C:\Program Files\NVIDIA Corporation\NVSMI\nvidia-smi.exe",
        r"C:\Windows\System32\nvidia-smi.exe",
    ];
    let paths: Vec<&str> = known_paths.iter()
        .filter(|p| std::path::Path::new(p).exists())
        .copied()
        .collect();
    if paths.is_empty() {
        return vec![];
    }
    for path in &paths {
        let mut cmd = Command::new(path);
        cmd.args([
            "--query-gpu=name,temperature.gpu,fan.speed,power.draw,clocks.gr,clocks.mem,memory.used,memory.total,utilization.gpu",
            "--format=csv,noheader,nounits",
        ]);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);
        if let Ok(out) = cmd.output() {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut result = vec![];
            for line in text.lines() {
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() < 9 { continue; }
                let hw = parts[0].to_string();
                let parse = |s: &str| s.trim().parse::<f64>().unwrap_or(0.0);
                let sensors = [
                    ("GPU Température",  "Temperature", parse(parts[1]), "°C"),
                    ("GPU Ventilateur",  "Fan",         parse(parts[2]), "RPM"),
                    ("GPU Puissance",    "Power",       parse(parts[3]), "W"),
                    ("GPU Core Clock",   "Clock",       parse(parts[4]), "MHz"),
                    ("GPU Mem Clock",    "Clock",       parse(parts[5]), "MHz"),
                    ("GPU Mémoire utilisée", "Data",    parse(parts[6]), "MB"),
                    ("GPU Utilisation",  "Load",        parse(parts[8]), "%"),
                ];
                for (name, st, value, unit) in sensors {
                    if value > 0.0 {
                        result.push(SensorReading {
                            hardware: hw.clone(),
                            hardware_type: "GPU NVIDIA".to_string(),
                            name: name.to_string(),
                            sensor_type: st.to_string(),
                            value,
                            unit: unit.to_string(),
                            min: 0.0, max: 0.0,
                            source: "nvidia-smi".to_string(),
                        });
                    }
                }
            }
            if !result.is_empty() { return result; }
        }
    }
    vec![]
}

fn query_storage_temps() -> Vec<SensorReading> {
    let script = r#"
$res = @()
try {
    Get-PhysicalDisk -ErrorAction Stop | ForEach-Object {
        $disk = $_
        $rel = Get-StorageReliabilityCounter -PhysicalDisk $disk -ErrorAction SilentlyContinue
        if ($rel -and $rel.Temperature -gt 0) {
            $res += [PSCustomObject]@{
                hardware="$($disk.FriendlyName)"; name='Température'; value=[double]$rel.Temperature
                hardware_type='Stockage'; sensor_type='Temperature'
            }
        }
        if ($rel -and $rel.TemperatureMax -gt 0) {
            $res += [PSCustomObject]@{
                hardware="$($disk.FriendlyName)"; name='Température Max'; value=[double]$rel.TemperatureMax
                hardware_type='Stockage'; sensor_type='Temperature'
            }
        }
    }
} catch {}
@($res) | ConvertTo-Json -Compress -Depth 2
"#;
    let out = ps(script).unwrap_or_default();
    if out.trim().is_empty() || out.trim() == "null" { return vec![]; }
    let arr: Vec<serde_json::Value> = serde_json::from_str(out.trim()).unwrap_or_default();
    arr.iter().filter_map(|v| {
        let value = v["value"].as_f64().unwrap_or(0.0);
        if value == 0.0 { return None; }
        Some(SensorReading {
            hardware:      v["hardware"].as_str().unwrap_or("Disque").to_string(),
            hardware_type: "Stockage".to_string(),
            name:          v["name"].as_str().unwrap_or("Température").to_string(),
            sensor_type:   "Temperature".to_string(),
            value,
            unit:          "°C".to_string(),
            min: 0.0, max: 0.0,
            source:        "SMART".to_string(),
        })
    }).collect()
}

fn query_acpi_temps() -> Vec<SensorReading> {
    let script = r#"
$res = @()
$zoneIdx = 0
function Label-Zone($raw) {
    switch -Wildcard ($raw.ToUpper()) {
        '*CPU*'  { return 'Processeur' }
        '*GPU*'  { return 'GPU intégré' }
        '*THM0*' { return 'Zone CPU' }
        '*THM1*' { return 'Zone GPU/PCH' }
        '*TZ0*'  { return 'Zone 0' }
        '*TZ1*'  { return 'Zone 1' }
        '*TZ2*'  { return 'Zone 2' }
        '*THRM*' { return 'Processeur (THRM)' }
        '*SKIN*' { return 'Boîtier' }
        '*AMB*'  { return 'Ambiance' }
        default  { return "Zone $($raw -replace '.*\\','')".Substring(0,[math]::Min(20,$raw.Length)) }
    }
}
# Essaie d'abord Get-CimInstance (plus permissif sur certains systèmes)
$items = $null
try { $items = @(Get-CimInstance -Namespace 'root/wmi' -ClassName 'MSAcpi_ThermalZoneTemperature' -ErrorAction Stop) } catch {}
if (-not $items) {
    try { $items = @(Get-WmiObject -Namespace 'root/wmi' -Class MSAcpi_ThermalZoneTemperature -ErrorAction Stop) } catch {}
}
if ($items) {
    foreach ($z in $items) {
        $c = [math]::Round(($z.CurrentTemperature - 2732) / 10.0, 1)
        if ($c -gt 5 -and $c -lt 125) {
            $lbl = Label-Zone ($z.InstanceName)
            $res += [PSCustomObject]@{ name=$lbl; value=[double]$c }
        }
    }
}
@($res) | ConvertTo-Json -Compress -Depth 2
"#;
    let out = ps(script).unwrap_or_default();
    if out.trim().is_empty() || out.trim() == "null" { return vec![]; }
    let arr: Vec<serde_json::Value> = serde_json::from_str(out.trim()).unwrap_or_default();
    arr.iter().filter_map(|v| {
        let value = v["value"].as_f64().unwrap_or(0.0);
        if value == 0.0 { return None; }
        Some(SensorReading {
            hardware:      "Système".to_string(),
            hardware_type: "CPU".to_string(),
            name:          v["name"].as_str().unwrap_or("Zone thermique").to_string(),
            sensor_type:   "Temperature".to_string(),
            value,
            unit:          "°C".to_string(),
            min: 0.0, max: 0.0,
            source:        "ACPI".to_string(),
        })
    }).collect()
}

/// Température CPU via WMI thermal zones (plus fiable qu'ACPI sur certains systèmes)
fn query_wmi_cpu_thermal() -> Vec<SensorReading> {
    let script = r#"
$res = @()
# Approche 1 : Win32_PerfFormattedData via WMI
$items = $null
try { $items = @(Get-WmiObject -Namespace 'root/cimv2' -Class Win32_PerfFormattedData_Counters_ThermalZoneInformation -ErrorAction Stop) } catch {}
if (-not $items) {
    try { $items = @(Get-CimInstance -Namespace 'root/cimv2' -ClassName 'Win32_PerfFormattedData_Counters_ThermalZoneInformation' -ErrorAction Stop) } catch {}
}
if ($items) {
    foreach ($z in $items) {
        $hpt = if ($z.HighPrecisionTemperature) { [double]$z.HighPrecisionTemperature } else { 0.0 }
        if ($hpt -gt 0) {
            $temp = [math]::Round($hpt / 10.0 - 273.15, 1)
            if ($temp -gt 5 -and $temp -lt 120) {
                $name = switch -Wildcard ($z.Name) {
                    '*CPU*'    { 'Processeur (WMI)' }
                    '*Proc*'   { 'Processeur (WMI)' }
                    default    { "Zone WMI: $($z.Name -replace '_',' ')" }
                }
                $res += [PSCustomObject]@{ name=$name; value=[double]$temp }
            }
        }
    }
}
@($res) | ConvertTo-Json -Compress -Depth 2
"#;
    let out = ps(script).unwrap_or_default();
    if out.trim().is_empty() || out.trim() == "null" { return vec![]; }
    let arr: Vec<serde_json::Value> = serde_json::from_str(out.trim()).unwrap_or_default();
    arr.iter().filter_map(|v| {
        let value = v["value"].as_f64().unwrap_or(0.0);
        if value == 0.0 { return None; }
        Some(SensorReading {
            hardware:      "Processeur".to_string(),
            hardware_type: "CPU".to_string(),
            name:          v["name"].as_str().unwrap_or("Température CPU").to_string(),
            sensor_type:   "Temperature".to_string(),
            value,
            unit:          "°C".to_string(),
            min: 0.0, max: 0.0,
            source:        "WMI-Thermal".to_string(),
        })
    }).collect()
}

/// GPU intégré — utilisation + mémoire via Compteurs de performance Windows
/// Couvre Intel UHD/Iris, AMD Radeon intégré, sans LHM
fn query_igpu_perf() -> Vec<SensorReading> {
    let script = r#"
$res = @()
try {
    # Utilisation GPU par adaptateur
    $gpuCounters = Get-Counter '\GPU Adapter Memory(*)\Dedicated Usage' -ErrorAction SilentlyContinue
    if ($gpuCounters) {
        foreach ($sample in $gpuCounters.CounterSamples) {
            $adapter = ($sample.InstanceName -replace '.*luid_.*?_.*?_phys_.*?_eng_.*?_engt.*','GPU').Trim()
            if ($adapter -eq '' ) { $adapter = 'GPU' }
            $mb = [math]::Round($sample.CookedValue / 1MB, 0)
            if ($mb -gt 0) {
                $res += [PSCustomObject]@{ hardware=$adapter; hardware_type='GPU Intel'; name='VRAM utilisée'; sensor_type='Data'; value=[double]$mb; unit='MB' }
            }
        }
    }
    # Utilisation GPU engine 3D
    $util = Get-Counter '\GPU Engine(*engtype_3D*)\Utilization Percentage' -ErrorAction SilentlyContinue
    if ($util) {
        $total = ($util.CounterSamples | Measure-Object CookedValue -Sum).Sum
        $pct = [math]::Round([math]::Min($total, 100), 1)
        $res += [PSCustomObject]@{ hardware='GPU intégré'; hardware_type='GPU Intel'; name='Utilisation 3D'; sensor_type='Load'; value=[double]$pct; unit='%' }
    }
} catch {}
@($res) | ConvertTo-Json -Compress -Depth 2
"#;
    let out = ps(script).unwrap_or_default();
    if out.trim().is_empty() || out.trim() == "null" { return vec![]; }
    let arr: Vec<serde_json::Value> = serde_json::from_str(out.trim()).unwrap_or_default();
    arr.iter().filter_map(|v| {
        let value = v["value"].as_f64().unwrap_or(0.0);
        if value == 0.0 { return None; }
        let st = v["sensor_type"].as_str().unwrap_or("Load").to_string();
        Some(SensorReading {
            hardware:      v["hardware"].as_str().unwrap_or("GPU intégré").to_string(),
            hardware_type: v["hardware_type"].as_str().unwrap_or("GPU Intel").to_string(),
            name:          v["name"].as_str().unwrap_or("Utilisation").to_string(),
            sensor_type:   st.clone(),
            value,
            unit:          v["unit"].as_str().unwrap_or(unit_for(&st)).to_string(),
            min: 0.0, max: 0.0,
            source:        "PerfCounters".to_string(),
        })
    }).collect()
}

fn get_all_sensors_sync() -> Vec<SensorReading> {
    // 1. LHM — source la plus complète (CPU par cœur, iGPU temp, fans, voltages)
    let lhm = query_lhm();
    if !lhm.is_empty() { return lhm; }

    // 2. OHM — alternative LHM
    let ohm = query_ohm();
    if !ohm.is_empty() { return ohm; }

    // 3. Sources natives combinées — TOUJOURS toutes les sources
    let mut result = Vec::new();
    result.extend(query_nvidia_smi());       // GPU NVIDIA dédié
    result.extend(query_igpu_perf());        // iGPU Intel/AMD (performance counters)
    result.extend(query_storage_temps());    // Températures disques (SMART)
    result.extend(query_wmi_cpu_thermal());  // Temp CPU via WMI thermal
    result.extend(query_acpi_temps());       // Zones thermiques ACPI
    // Dédoublonnage par (hardware+name): on garde la valeur la plus précise
    result.sort_by(|a, b| a.hardware.cmp(&b.hardware).then(a.name.cmp(&b.name)));
    result.dedup_by(|a, b| a.hardware == b.hardware && a.name == b.name);
    result
}

#[tauri::command]
pub async fn get_all_sensors() -> Result<Vec<SensorReading>, String> {
    tokio::task::spawn_blocking(get_all_sensors_sync)
        .await
        .map_err(|e| e.to_string())
}
