use serde::Serialize;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use super::{parse_json_arr, ps};

// ─── Températures ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct TemperatureReading {
    pub sensor_name: String,
    pub sensor_type: String,
    pub temp_celsius: f32,
    pub source: String,
}

#[tauri::command]
pub async fn get_temperatures() -> Result<Vec<TemperatureReading>, String> {
    tokio::task::spawn_blocking(get_temperatures_sync)
        .await
        .map_err(|e| e.to_string())?
}

fn get_temperatures_sync() -> Result<Vec<TemperatureReading>, String> {
    let mut temps: Vec<TemperatureReading> = Vec::new();

    // 1. LibreHardwareMonitor WMI
    let script_lhm = r#"
try {
    $sensors = Get-CimInstance -Namespace root/LibreHardwareMonitor -ClassName Sensor -ErrorAction SilentlyContinue |
        Where-Object { $_.SensorType -eq 'Temperature' -and $_.Value -gt 0 -and $_.Value -lt 120 } |
        Select-Object Name, Value, Hardware
    if ($sensors) { $sensors | ConvertTo-Json -Compress -Depth 2 }
} catch {}
"#;
    let mut lhm_found = false;
    if let Ok(out) = ps(script_lhm) {
        if !out.is_empty() {
            let arr: Vec<serde_json::Value> = parse_json_arr(&out);
            for v in &arr {
                let t = v["Value"].as_f64().unwrap_or(0.0) as f32;
                let name = v["Name"].as_str().unwrap_or("?").to_string();
                let hw = v["Hardware"].as_str().unwrap_or("").to_string();
                if t > 0.0 && t < 120.0 {
                    let sensor_type = if hw.to_lowercase().contains("cpu") || name.to_lowercase().contains("core") || name.to_lowercase().contains("package") { "CPU" }
                        else if hw.to_lowercase().contains("gpu") || hw.to_lowercase().contains("video") { "GPU" }
                        else if hw.to_lowercase().contains("ssd") || hw.to_lowercase().contains("hdd") || hw.to_lowercase().contains("nvme") { "Storage" }
                        else { "Autre" };
                    temps.push(TemperatureReading { sensor_name: name, sensor_type: sensor_type.to_string(), temp_celsius: t, source: "LibreHardwareMonitor".to_string() });
                    lhm_found = true;
                }
            }
        }
    }

    // 2. OpenHardwareMonitor WMI
    if !lhm_found {
        let script_ohm = r#"
try {
    $sensors = Get-CimInstance -Namespace root/OpenHardwareMonitor -ClassName Sensor -ErrorAction SilentlyContinue |
        Where-Object { $_.SensorType -eq 'Temperature' -and $_.Value -gt 0 }
    if ($sensors) { $sensors | Select-Object Name, Value | ConvertTo-Json -Compress }
} catch {}
"#;
        if let Ok(out) = ps(script_ohm) {
            if !out.is_empty() {
                let arr: Vec<serde_json::Value> = parse_json_arr(&out);
                for v in &arr {
                    let t = v["Value"].as_f64().unwrap_or(0.0) as f32;
                    let name = v["Name"].as_str().unwrap_or("?").to_string();
                    if t > 0.0 && t < 120.0 {
                        let st = if name.to_lowercase().contains("core") || name.to_lowercase().contains("package") { "CPU" }
                            else if name.to_lowercase().contains("gpu") { "GPU" } else { "Autre" };
                        temps.push(TemperatureReading { sensor_name: name, sensor_type: st.to_string(), temp_celsius: t, source: "OpenHardwareMonitor".to_string() });
                    }
                }
            }
        }
    }

    // 3. NVIDIA GPU via nvidia-smi
    if let Ok(out) = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=name,temperature.gpu,pci.sub_device_id", "--format=csv,noheader"])
        .creation_flags(0x08000000)
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for (i, line) in text.lines().enumerate() {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 2 {
                if let Ok(t) = parts[1].parse::<f32>() {
                    if t > 0.0 {
                        let name = if i == 0 { format!("GPU — {}", parts[0]) } else { format!("GPU {} — {}", i + 1, parts[0]) };
                        if !temps.iter().any(|r| r.sensor_name == name) {
                            temps.push(TemperatureReading { sensor_name: name, sensor_type: "GPU".to_string(), temp_celsius: t, source: "nvidia-smi".to_string() });
                        }
                    }
                }
            }
        }
    }

    // 4. Disques SMART
    let script_disk = r#"
try {
    Get-PhysicalDisk -ErrorAction SilentlyContinue | ForEach-Object {
        $rel = $_ | Get-StorageReliabilityCounter -ErrorAction SilentlyContinue
        if ($rel -and $rel.Temperature -gt 0) {
            [PSCustomObject]@{ name=$_.FriendlyName; temp=$rel.Temperature; media=$_.MediaType }
        }
    } | ConvertTo-Json -Compress
} catch {}
"#;
    if let Ok(out) = ps(script_disk) {
        if !out.is_empty() {
            let arr: Vec<serde_json::Value> = parse_json_arr(&out);
            for v in &arr {
                let t = v["temp"].as_f64().unwrap_or(0.0) as f32;
                let name = v["name"].as_str().unwrap_or("Disque").to_string();
                if t > 0.0 && !temps.iter().any(|r| r.sensor_name == name && r.sensor_type == "Storage") {
                    temps.push(TemperatureReading { sensor_name: name, sensor_type: "Storage".to_string(), temp_celsius: t, source: "SMART".to_string() });
                }
            }
        }
    }

    // 5. Fallback ACPI thermal zones
    if temps.iter().filter(|r| r.sensor_type == "CPU").count() == 0 {
        let script_acpi = r#"
try {
    $tz = Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature -ErrorAction SilentlyContinue
    if ($tz) { $tz | ForEach-Object { [PSCustomObject]@{ temp=[math]::Round(($_.CurrentTemperature/10)-273.15,1) } } | ConvertTo-Json -Compress }
} catch {}
"#;
        if let Ok(out) = ps(script_acpi) {
            if !out.is_empty() {
                let arr: Vec<serde_json::Value> = parse_json_arr(&out);
                for (i, v) in arr.iter().enumerate() {
                    let t = v["temp"].as_f64().unwrap_or(0.0) as f32;
                    if t > 0.0 && t < 120.0 {
                        temps.push(TemperatureReading {
                            sensor_name: format!("CPU Package (Zone {})", i + 1),
                            sensor_type: "CPU".to_string(), temp_celsius: t, source: "ACPI WMI".to_string(),
                        });
                    }
                }
            }
        }
    }

    if temps.is_empty() {
        temps.push(TemperatureReading {
            sensor_name: "Aucun capteur accessible".to_string(),
            sensor_type: "Info".to_string(), temp_celsius: -1.0,
            source: "Installez LibreHardwareMonitor et lancez-le en administrateur".to_string(),
        });
    }

    Ok(temps)
}

#[derive(Serialize)]
pub struct CoreTemp {
    pub core: u32,
    pub label: String,
    pub temp_celsius: f32,
}

#[tauri::command]
pub fn get_cpu_core_temps() -> Result<Vec<CoreTemp>, String> {
    let script = r#"
$result = @()
try {
    $zones = Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature -ErrorAction SilentlyContinue
    if ($zones) {
        $i = 0
        foreach ($z in $zones) {
            $c = [math]::Round(($z.CurrentTemperature / 10) - 273.15, 1)
            if ($c -gt 0 -and $c -lt 120) {
                $result += [PSCustomObject]@{ core=$i; label=$z.InstanceName; temp=$c }
                $i++
            }
        }
    }
} catch {}
$result | ConvertTo-Json -Compress"#;
    let out = ps(script)?;
    if out.is_empty() { return Ok(vec![]); }
    let arr: Vec<serde_json::Value> = parse_json_arr(&out);
    Ok(arr.iter().filter_map(|v| {
        let t = v["temp"].as_f64().unwrap_or(0.0) as f32;
        if t > 0.0 && t < 120.0 {
            Some(CoreTemp {
                core: v["core"].as_u64().unwrap_or(0) as u32,
                label: v["label"].as_str().unwrap_or("").to_string(),
                temp_celsius: t,
            })
        } else { None }
    }).collect())
}

#[derive(Serialize)]
pub struct GpuTemp {
    pub name: String,
    pub temp_celsius: f32,
    pub source: String,
}

#[tauri::command]
pub fn get_gpu_temps() -> Result<Vec<GpuTemp>, String> {
    let script = r#"
$result = @()
$seen = @{}

function Add-Entry($name, $temp, $src) {
    $k = "$name|$src"
    if (-not $seen.ContainsKey($k) -and $temp -gt 1 -and $temp -lt 120) {
        $seen[$k] = $true
        $script:result += [PSCustomObject]@{ name=$name; temp=[math]::Round($temp,1); src=$src }
    }
}

# ── 1. nvidia-smi ──────────────────────────────────────────────────────────
try {
    $nsmi = Get-Command nvidia-smi -ErrorAction SilentlyContinue
    if ($nsmi) {
        $raw = & nvidia-smi --query-gpu=name,temperature.gpu --format=csv,noheader,nounits 2>$null
        if ($raw) {
            $raw -split "`n" | Where-Object { $_.Trim() } | ForEach-Object {
                $parts = $_ -split ',\s*'
                if ($parts.Count -ge 2) {
                    $t = [double]$parts[-1].Trim()
                    $n = ($parts[0..($parts.Count-2)] -join ', ').Trim()
                    Add-Entry $n $t 'nvidia-smi'
                }
            }
        }
    }
} catch {}

# ── 2. OpenHardwareMonitor WMI ──────────────────────────────────────────
try {
    $ohm = Get-CimInstance -Namespace root/OpenHardwareMonitor -ClassName Sensor -ErrorAction SilentlyContinue |
           Where-Object { $_.SensorType -eq 'Temperature' -and $_.Name -match 'GPU|Core|Hot' }
    if ($ohm) {
        $ohm | ForEach-Object { Add-Entry $_.Name $_.Value 'OpenHardwareMonitor' }
    }
} catch {}

# ── 3. LibreHardwareMonitor WMI ──────────────────────────────────────────
try {
    $lhm = Get-CimInstance -Namespace root/LibreHardwareMonitor -ClassName Sensor -ErrorAction SilentlyContinue |
           Where-Object { $_.SensorType -eq 'Temperature' -and ($_.Name -match 'GPU' -or $_.HardwareName -match 'GPU|Radeon|GeForce|RX |RTX |GTX ') }
    if ($lhm) {
        $lhm | ForEach-Object { Add-Entry "$($_.HardwareName) / $($_.Name)" $_.Value 'LibreHardwareMonitor' }
    }
} catch {}

# ── 4. ACPI Thermal Zones ────────────────────────────────────────────────
try {
    $zones = Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature -ErrorAction SilentlyContinue
    if ($zones) {
        $zones | ForEach-Object {
            $c = ($_.CurrentTemperature / 10) - 273.15
            $inst = $_.InstanceName.ToLower()
            if ($inst -match 'gpu|disp|vid|vga|thm1|thm2') {
                Add-Entry "GPU Thermal ($($_.InstanceName))" $c 'ACPI WMI'
            }
        }
    }
} catch {}

# ── 5. Win32_VideoController (legacy) ────────────────────────────────────
try {
    Get-WmiObject Win32_VideoController -ErrorAction SilentlyContinue | ForEach-Object {
        if ($_.CurrentTemperature -and $_.CurrentTemperature -gt 0) {
            $c = ($_.CurrentTemperature / 10) - 273.15
            Add-Entry $_.Name $c 'WMI Win32'
        }
    }
} catch {}

if ($result.Count -eq 0) {
    try {
        Get-WmiObject Win32_VideoController -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -and -not ($_.Name -match 'Microsoft Basic') } |
            ForEach-Object {
                $script:result += [PSCustomObject]@{ name=$_.Name; temp=-1; src='unavailable' }
            }
    } catch {}
}

$result | ConvertTo-Json -Compress -Depth 2"#;
    let out = ps(script)?;
    if out.is_empty() { return Ok(vec![]); }
    let arr: Vec<serde_json::Value> = parse_json_arr(&out);
    Ok(arr.iter().map(|v| {
        let t = v["temp"].as_f64().unwrap_or(-1.0) as f32;
        GpuTemp {
            name: v["name"].as_str().unwrap_or("GPU").to_string(),
            temp_celsius: t,
            source: v["src"].as_str().unwrap_or("unavailable").to_string(),
        }
    }).collect())
}

// ─── WiFi Analyzer ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub bssid: String,
    pub signal_percent: u32,
    pub channel: u32,
    pub band: String,
    pub authentication: String,
    pub network_type: String,
    pub radio_type: String,
}

#[tauri::command]
pub fn get_nearby_wifi() -> Result<Vec<WifiNetwork>, String> {
    let out = std::process::Command::new("netsh")
        .args(["wlan", "show", "networks", "mode=bssid"])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut networks: Vec<WifiNetwork> = Vec::new();
    let mut current = WifiNetwork { ssid: String::new(), bssid: String::new(), signal_percent: 0, channel: 0, band: String::new(), authentication: String::new(), network_type: String::new(), radio_type: String::new() };
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("SSID") && !line.starts_with("SSID Name") && !line.starts_with("BSSID") {
            if !current.ssid.is_empty() { networks.push(current); current = WifiNetwork { ssid: String::new(), bssid: String::new(), signal_percent: 0, channel: 0, band: String::new(), authentication: String::new(), network_type: String::new(), radio_type: String::new() }; }
            if let Some(v) = line.split_once(':').map(|x| x.1) { current.ssid = v.trim().to_string(); }
        } else if line.starts_with("Network type") {
            if let Some(v) = line.split_once(':').map(|x| x.1) { current.network_type = v.trim().to_string(); }
        } else if line.starts_with("Authentication") {
            if let Some(v) = line.split_once(':').map(|x| x.1) { current.authentication = v.trim().to_string(); }
        } else if line.starts_with("BSSID 1") {
            if let Some(v) = line.split_once(':').map(|x| x.1) { current.bssid = v.trim().to_string(); }
        } else if line.starts_with("Signal") {
            if let Some(v) = line.split_once(':').map(|x| x.1) {
                current.signal_percent = v.trim().trim_end_matches('%').parse().unwrap_or(0);
            }
        } else if line.starts_with("Radio type") {
            if let Some(v) = line.split_once(':').map(|x| x.1) {
                let rt = v.trim().to_string();
                current.band = if rt.contains("5") { "5 GHz".to_string() } else { "2.4 GHz".to_string() };
                current.radio_type = rt;
            }
        } else if line.starts_with("Channel") {
            if let Some(v) = line.split_once(':').map(|x| x.1) { current.channel = v.trim().parse().unwrap_or(0); }
        }
    }
    if !current.ssid.is_empty() { networks.push(current); }
    networks.sort_by_key(|a| std::cmp::Reverse(a.signal_percent));
    Ok(networks)
}

// ─── Mode Turbo / Profils ─────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct TurboResult {
    pub actions_done: Vec<String>,
    pub errors: Vec<String>,
}

#[tauri::command]
pub fn apply_turbo_mode(mode: String) -> Result<TurboResult, String> {
    let mut done: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let run = |script: &str| ps(script);

    match mode.as_str() {
        "gaming" => {
            if run("powercfg /setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c 2>&1").is_ok() { done.push("Plan d'alimentation : Haute performance".into()); } else { errors.push("Plan d'alimentation non changé".into()); }
            if run("try { Get-AppxPackage Microsoft.XboxGamingOverlay | Disable-AppxPackage -ErrorAction SilentlyContinue } catch { Set-Service -Name 'GameBarPresenceWriter' -StartupType Disabled -ErrorAction SilentlyContinue }; Set-Service -Name 'GameBarPresenceWriter' -StartupType Disabled -ErrorAction SilentlyContinue; 'ok'").is_ok() { done.push("Xbox Game Bar désactivé (réversible)".into()); }
            if run("Set-ItemProperty -Path 'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers' -Name 'HwSchMode' -Value 2 -Type DWord -ErrorAction SilentlyContinue; 'ok'").is_ok() { done.push("GPU Hardware Scheduling activé".into()); }
            if run("Set-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\GameBar' -Name 'AllowAutoGameMode' -Value 1 -Type DWord -ErrorAction SilentlyContinue; Set-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\GameBar' -Name 'AutoGameModeEnabled' -Value 1 -Type DWord -ErrorAction SilentlyContinue; 'ok'").is_ok() { done.push("Game Mode Windows activé".into()); }
        }
        "work" => {
            if run("powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e 2>&1").is_ok() { done.push("Plan d'alimentation : Équilibré".into()); }
            if run("Set-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects' -Name 'VisualFXSetting' -Value 2 -ErrorAction SilentlyContinue; 'ok'").is_ok() { done.push("Effets visuels optimisés".into()); }
            if run("Clear-Clipboard -ErrorAction SilentlyContinue; 'ok'").is_ok() { done.push("Presse-papiers vidé".into()); }
        }
        "eco" => {
            if run("powercfg /setactive a1841308-3541-4fab-bc81-f71556f20b4a 2>&1").is_ok() { done.push("Plan d'alimentation : Économie d'énergie".into()); }
            if run("(Get-WmiObject -Namespace root/wmi -Class WmiMonitorBrightnessMethods).WmiSetBrightness(1,50) 2>&1; 'ok'").is_ok() { done.push("Luminosité réduite à 50%".into()); }
        }
        _ => {
            if run("powercfg /setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c 2>&1").is_ok() { done.push("Plan d'alimentation : Haute performance".into()); }
            if run("Clear-DnsClientCache; 'ok'").is_ok() { done.push("Cache DNS vidé".into()); }
            if run("$mem = [System.Runtime.InteropServices.Marshal]::AllocHGlobal(0); [System.Runtime.InteropServices.Marshal]::FreeHGlobal($mem); [System.GC]::Collect(); 'ok'").is_ok() { done.push("Mémoire libérée".into()); }
            if run("$protected = @('svchost','System','Registry','lsass','csrss','smss','winlogon','wininit','lsm','dwm','fontdrvhost','msseces','msmpeng','avgnt','avp','bdagent','ekrn','mbam','backupclient','veeam','veamagent','mmagent','nessus','falconhost'); Get-Process | Where-Object { $_.CPU -lt 0.1 -and $_.WorkingSet -gt 1GB -and $_.SessionId -ne 0 -and $_.Name -notin $protected } | Stop-Process -Force -ErrorAction SilentlyContinue; 'ok'").is_ok() { done.push("Processus inutiles terminés".into()); }
            if run("Set-ItemProperty -Path 'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers' -Name 'HwSchMode' -Value 2 -Type DWord -ErrorAction SilentlyContinue; 'ok'").is_ok() { done.push("GPU Hardware Scheduling activé".into()); }
        }
    }

    Ok(TurboResult { actions_done: done, errors })
}

// ─── Hidden Power Plans ───────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct PowerPlanResult {
    pub name: String,
    pub guid: String,
    pub success: bool,
    pub message: String,
}

#[tauri::command]
pub fn enable_hidden_power_plans() -> Result<Vec<PowerPlanResult>, String> {
    let hidden_plans = vec![
        ("Performances maximales", "e9a42b02-d5df-448d-aa00-03f14749eb61"),
    ];
    let mut results = Vec::new();
    for (name, guid) in &hidden_plans {
        let script = format!(
            "powercfg /duplicatescheme {} 2>&1; echo 'OK'",
            guid
        );
        let out = ps(&script).unwrap_or_default();
        let success = !out.to_lowercase().contains("error") && !out.to_lowercase().contains("erreur");
        results.push(PowerPlanResult {
            name: name.to_string(),
            guid: guid.to_string(),
            success,
            message: if success { "Plan ajouté / déjà disponible".into() } else { out.trim().to_string() },
        });
    }
    Ok(results)
}

// ─── Quick Optimization Runner ────────────────────────────────────────────────

#[tauri::command]
pub fn run_quick_optimization(opt_id: String) -> Result<String, String> {
    let script = match opt_id.as_str() {
        "clean_temp" => r#"
$removed = 0
Get-ChildItem "$env:TEMP" -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
Get-ChildItem "C:\Windows\Temp" -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
"Fichiers temporaires supprimés"
"#,
        "flush_dns" => "ipconfig /flushdns; 'Cache DNS vidé'",
        "clean_eventlog" => r#"
$logs = @("Application","System","Security","Setup")
foreach ($l in $logs) { try { Clear-EventLog -LogName $l -ErrorAction SilentlyContinue } catch {} }
"Journaux d'événements vidés"
"#,
        "disable_prefetch" => r#"
Stop-Service -Name SysMain -ErrorAction SilentlyContinue
Set-Service -Name SysMain -StartupType Disabled -ErrorAction SilentlyContinue
"Superfetch/SysMain désactivé pour les SSD"
"#,
        "disable_telemetry" => r#"
$p = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\DataCollection'
if (-not (Test-Path $p)) { New-Item -Path $p -Force | Out-Null }
Set-ItemProperty $p -Name AllowTelemetry -Value 1 -Type DWord -Force -ErrorAction SilentlyContinue
"Télémétrie réduite au minimum"
"#,
        "visual_perf" => r#"
$p = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects'
Set-ItemProperty $p -Name VisualFXSetting -Value 2 -ErrorAction SilentlyContinue
"Effets visuels en mode performance"
"#,
        "optimize_drives" => r#"
$count = 0
Get-Volume | Where-Object { $_.DriveLetter -and $_.DriveType -eq 'Fixed' } | ForEach-Object {
    Optimize-Volume -DriveLetter $_.DriveLetter -ReTrim -ErrorAction SilentlyContinue
    $count++
}
"Optimisation TRIM lancée sur $count volume(s)"
"#,
        "clear_clipboard" => r#"
Add-Type -AssemblyName System.Windows.Forms -ErrorAction SilentlyContinue
[System.Windows.Forms.Clipboard]::Clear()
"Presse-papiers vidé"
"#,
        _ => return Err(format!("Optimisation '{}' inconnue", opt_id)),
    };
    ps(script)
}
