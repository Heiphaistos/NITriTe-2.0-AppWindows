use serde::Serialize;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use super::{parse_json_arr, ps, ps_ok};

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

// Anti-freeze : scan WiFi via PowerShell est bloquant — jamais inline sur
// le thread de commande.
#[tauri::command]
pub async fn get_nearby_wifi() -> Result<Vec<WifiNetwork>, String> {
    tokio::task::spawn_blocking(get_nearby_wifi_blocking)
        .await
        .map_err(|e| e.to_string())?
}

fn get_nearby_wifi_blocking() -> Result<Vec<WifiNetwork>, String> {
    let out = std::process::Command::new("netsh")
        .args(["wlan", "show", "networks", "mode=bssid"])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| e.to_string())?;
    // decode_output (repli OEM) : netsh écrit en codepage OEM → SSID accentués
    // sinon mojibake.
    let text = crate::maintenance::commands::decode_output(&out.stdout);
    // Parsing locale-indépendant : les libellés netsh sont traduits (« Type de
    // réseau », « Authentification », « Type de radio », « Canal »…). Sans ça,
    // sur Windows FR l'auth/type restaient vides et la bande était toujours
    // « 2.4 GHz ». On coupe sur ':', normalise le libellé (accents retirés) et
    // matche des mots-clés EN + FR.
    let strip_accents = |s: &str| s.chars().map(|c| match c {
        'é'|'è'|'ê'|'ë' => 'e', 'à'|'â'|'ä' => 'a', 'î'|'ï' => 'i',
        'ô'|'ö' => 'o', 'û'|'ü'|'ù' => 'u', 'ç' => 'c', _ => c,
    }).collect::<String>();
    let mut networks: Vec<WifiNetwork> = Vec::new();
    let mut current = WifiNetwork { ssid: String::new(), bssid: String::new(), signal_percent: 0, channel: 0, band: String::new(), authentication: String::new(), network_type: String::new(), radio_type: String::new() };
    for line in text.lines() {
        let Some((label, val)) = line.split_once(':') else { continue };
        let l = strip_accents(&label.trim().to_lowercase());
        let val = val.trim();
        if l.starts_with("ssid") {
            // Nouvelle entrée réseau (« SSID 1 »). « bssid » commence par 'b' → exclu.
            if !current.ssid.is_empty() { networks.push(current); current = WifiNetwork { ssid: String::new(), bssid: String::new(), signal_percent: 0, channel: 0, band: String::new(), authentication: String::new(), network_type: String::new(), radio_type: String::new() }; }
            current.ssid = val.to_string();
        } else if l.starts_with("bssid") {
            current.bssid = val.to_string();
        } else if l.contains("radio") {
            let rt = val.to_string();
            current.band = if rt.contains('5') { "5 GHz".to_string() } else { "2.4 GHz".to_string() };
            current.radio_type = rt;
        } else if l.contains("network") || l.contains("reseau") {
            current.network_type = val.to_string();
        } else if l.contains("authentic") || l.contains("authentif") {
            current.authentication = val.to_string();
        } else if l == "signal" {
            current.signal_percent = val.trim_end_matches('%').trim().parse().unwrap_or(0);
        } else if l == "channel" || l.contains("canal") {
            current.channel = val.parse().unwrap_or(0);
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

// Protocole de succès réel : chaque script prouve son résultat — try/catch
// avec -ErrorAction Stop puis `exit 1`, ou test de $LASTEXITCODE pour les
// exécutables natifs — et est lancé via ps_ok() qui vérifie le code de sortie.
// Avant, le succès était jugé par ps().is_ok() (vrai dès que powershell.exe
// démarre) et les -ErrorAction SilentlyContinue rendaient invisibles tous les
// échecs, notamment sans droits admin : l'UI affichait « fait » à tort.

const PS_HIGH_PERF: &str = "powercfg /setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c 2>&1 | Out-Null; if ($LASTEXITCODE -ne 0) { Write-Output 'powercfg a échoué (plan absent ?)'; exit 1 }";
const PS_BALANCED: &str = "powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e 2>&1 | Out-Null; if ($LASTEXITCODE -ne 0) { Write-Output 'powercfg a échoué (plan absent ?)'; exit 1 }";
const PS_POWER_SAVER: &str = "powercfg /setactive a1841308-3541-4fab-bc81-f71556f20b4a 2>&1 | Out-Null; if ($LASTEXITCODE -ne 0) { Write-Output 'powercfg a échoué (plan absent ?)'; exit 1 }";
const PS_HW_SCH: &str = r#"try {
    Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\GraphicsDrivers' -Name 'HwSchMode' -Value 2 -Type DWord -ErrorAction Stop
} catch { Write-Output $_.Exception.Message; exit 1 }"#;

#[tauri::command]
pub fn apply_turbo_mode(mode: String) -> Result<TurboResult, String> {
    let mut done: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    fn run(script: &str, ok_msg: &str, err_label: &str, done: &mut Vec<String>, errors: &mut Vec<String>) {
        match ps_ok(script) {
            Ok(_) => done.push(ok_msg.to_string()),
            Err(e) => errors.push(format!("{err_label} : {e}")),
        }
    }

    match mode.as_str() {
        "gaming" => {
            run(PS_HIGH_PERF, "Plan d'alimentation : Haute performance", "Plan d'alimentation", &mut done, &mut errors);
            // Disable-AppxPackage n'existe pas (l'ancien script échouait toujours
            // en silence) → désactivation Game Bar/DVR via registre HKCU, réversible.
            run(r#"try {
    foreach ($p in 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR', 'HKCU:\System\GameConfigStore') {
        if (-not (Test-Path $p)) { New-Item -Path $p -Force -ErrorAction Stop | Out-Null }
    }
    Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR' -Name 'AppCaptureEnabled' -Value 0 -Type DWord -ErrorAction Stop
    Set-ItemProperty -Path 'HKCU:\System\GameConfigStore' -Name 'GameDVR_Enabled' -Value 0 -Type DWord -ErrorAction Stop
} catch { Write-Output $_.Exception.Message; exit 1 }"#,
                "Xbox Game Bar / DVR désactivé (réversible)", "Xbox Game Bar", &mut done, &mut errors);
            run(PS_HW_SCH, "GPU Hardware Scheduling activé", "GPU Hardware Scheduling", &mut done, &mut errors);
            run(r#"try {
    $p = 'HKCU:\Software\Microsoft\GameBar'
    if (-not (Test-Path $p)) { New-Item -Path $p -Force -ErrorAction Stop | Out-Null }
    Set-ItemProperty -Path $p -Name 'AllowAutoGameMode' -Value 1 -Type DWord -ErrorAction Stop
    Set-ItemProperty -Path $p -Name 'AutoGameModeEnabled' -Value 1 -Type DWord -ErrorAction Stop
} catch { Write-Output $_.Exception.Message; exit 1 }"#,
                "Game Mode Windows activé", "Game Mode", &mut done, &mut errors);
        }
        "work" => {
            run(PS_BALANCED, "Plan d'alimentation : Équilibré", "Plan d'alimentation", &mut done, &mut errors);
            run(r#"try {
    $p = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects'
    if (-not (Test-Path $p)) { New-Item -Path $p -Force -ErrorAction Stop | Out-Null }
    Set-ItemProperty -Path $p -Name 'VisualFXSetting' -Value 2 -Type DWord -ErrorAction Stop
} catch { Write-Output $_.Exception.Message; exit 1 }"#,
                "Effets visuels optimisés", "Effets visuels", &mut done, &mut errors);
            // Clear-Clipboard n'existe pas en PowerShell 5.1 → WinForms.
            run(r#"try {
    Add-Type -AssemblyName System.Windows.Forms -ErrorAction Stop
    [System.Windows.Forms.Clipboard]::Clear()
} catch { Write-Output $_.Exception.Message; exit 1 }"#,
                "Presse-papiers vidé", "Presse-papiers", &mut done, &mut errors);
        }
        "eco" => {
            run(PS_POWER_SAVER, "Plan d'alimentation : Économie d'énergie", "Plan d'alimentation", &mut done, &mut errors);
            run(r#"try {
    $m = Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBrightnessMethods -ErrorAction Stop | Select-Object -First 1
    if (-not $m) { throw 'Aucun écran à luminosité pilotable (poste fixe ?)' }
    Invoke-CimMethod -InputObject $m -MethodName WmiSetBrightness -Arguments @{ Timeout = [uint32]1; Brightness = [byte]50 } -ErrorAction Stop | Out-Null
} catch { Write-Output $_.Exception.Message; exit 1 }"#,
                "Luminosité réduite à 50%", "Luminosité", &mut done, &mut errors);
        }
        _ => {
            run(PS_HIGH_PERF, "Plan d'alimentation : Haute performance", "Plan d'alimentation", &mut done, &mut errors);
            run("try { Clear-DnsClientCache -ErrorAction Stop } catch { Write-Output $_.Exception.Message; exit 1 }",
                "Cache DNS vidé", "Cache DNS", &mut done, &mut errors);
            // L'ancien script (AllocHGlobal(0) + GC.Collect dans le process
            // PowerShell) ne libérait rien : EmptyWorkingSet réel + comptage.
            match ps_ok(r#"try {
    $sig = '[System.Runtime.InteropServices.DllImport("psapi.dll")] public static extern int EmptyWorkingSet(System.IntPtr h);'
    Add-Type -MemberDefinition $sig -Name 'MemUtil' -Namespace 'Nitrite' -ErrorAction Stop
    $n = 0
    Get-Process | ForEach-Object { try { if ([Nitrite.MemUtil]::EmptyWorkingSet($_.Handle) -ne 0) { $n++ } } catch {} }
    Write-Output $n
} catch { Write-Output $_.Exception.Message; exit 1 }"#) {
                Ok(n) => done.push(format!("Mémoire libérée ({} processus allégés)", n)),
                Err(e) => errors.push(format!("Libération mémoire : {e}")),
            }
            match ps_ok(r#"$protected = @('svchost','System','Registry','lsass','csrss','smss','winlogon','wininit','lsm','dwm','fontdrvhost','msseces','msmpeng','avgnt','avp','bdagent','ekrn','mbam','backupclient','veeam','veamagent','mmagent','nessus','falconhost')
$n = 0
Get-Process | Where-Object { $_.CPU -lt 0.1 -and $_.WorkingSet -gt 1GB -and $_.SessionId -ne 0 -and $_.Name -notin $protected } |
    ForEach-Object { try { Stop-Process -Id $_.Id -Force -ErrorAction Stop; $n++ } catch {} }
Write-Output $n"#) {
                Ok(n) => done.push(format!("Processus inutiles terminés ({})", n)),
                Err(e) => errors.push(format!("Processus inutiles : {e}")),
            }
            run(PS_HW_SCH, "GPU Hardware Scheduling activé", "GPU Hardware Scheduling", &mut done, &mut errors);
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
        // Succès jugé sur $LASTEXITCODE — l'ancien sniffing de « error »/« erreur »
        // dans stdout dépendait de la langue de Windows.
        let script = format!(
            "$out = powercfg /duplicatescheme {} 2>&1; if ($LASTEXITCODE -ne 0) {{ Write-Output $out; exit 1 }}",
            guid
        );
        let (success, message) = match ps_ok(&script) {
            Ok(_) => (true, "Plan ajouté / déjà disponible".to_string()),
            Err(e) => (false, e),
        };
        results.push(PowerPlanResult {
            name: name.to_string(),
            guid: guid.to_string(),
            success,
            message,
        });
    }
    Ok(results)
}

// ─── Quick Optimization Runner ────────────────────────────────────────────────

#[tauri::command]
pub fn run_quick_optimization(opt_id: String) -> Result<String, String> {
    // Même protocole que apply_turbo_mode : les actions qui exigent l'élévation
    // (journaux, SysMain, télémétrie HKLM, TRIM) sortent `exit 1` si rien n'a
    // réellement été appliqué — l'UI affiche alors le toast d'erreur au lieu
    // d'un faux succès. Lancé via ps_ok() (code de sortie vérifié).
    let script = match opt_id.as_str() {
        // Suppression best-effort : les fichiers verrouillés par des process
        // vivants sont normaux, on ne les compte pas comme échec.
        "clean_temp" => r#"
Get-ChildItem "$env:TEMP" -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
Get-ChildItem "C:\Windows\Temp" -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
"Fichiers temporaires supprimés (fichiers verrouillés ignorés)"
"#,
        "flush_dns" => "ipconfig /flushdns 2>&1 | Out-Null; if ($LASTEXITCODE -ne 0) { Write-Output 'ipconfig /flushdns a échoué'; exit 1 }; 'Cache DNS vidé'",
        "clean_eventlog" => r#"
$logs = @("Application","System","Security","Setup")
$n = 0
foreach ($l in $logs) { try { Clear-EventLog -LogName $l -ErrorAction Stop; $n++ } catch {} }
if ($n -eq 0) { Write-Output "Aucun journal vidé — élévation administrateur requise"; exit 1 }
"$n journaux d'événements vidés"
"#,
        "disable_prefetch" => r#"
try {
    Stop-Service -Name SysMain -Force -ErrorAction Stop
    Set-Service -Name SysMain -StartupType Disabled -ErrorAction Stop
} catch { Write-Output $_.Exception.Message; exit 1 }
"Superfetch/SysMain désactivé pour les SSD"
"#,
        "disable_telemetry" => r#"
try {
    $p = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\DataCollection'
    if (-not (Test-Path $p)) { New-Item -Path $p -Force -ErrorAction Stop | Out-Null }
    Set-ItemProperty $p -Name AllowTelemetry -Value 1 -Type DWord -Force -ErrorAction Stop
} catch { Write-Output $_.Exception.Message; exit 1 }
"Télémétrie réduite au minimum"
"#,
        "visual_perf" => r#"
try {
    $p = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects'
    if (-not (Test-Path $p)) { New-Item -Path $p -Force -ErrorAction Stop | Out-Null }
    Set-ItemProperty $p -Name VisualFXSetting -Value 2 -Type DWord -ErrorAction Stop
} catch { Write-Output $_.Exception.Message; exit 1 }
"Effets visuels en mode performance"
"#,
        "optimize_drives" => r#"
$count = 0
Get-Volume | Where-Object { $_.DriveLetter -and $_.DriveType -eq 'Fixed' } | ForEach-Object {
    try { Optimize-Volume -DriveLetter $_.DriveLetter -ReTrim -ErrorAction Stop; $count++ } catch {}
}
if ($count -eq 0) { Write-Output "Aucun volume optimisé — élévation administrateur requise ou volumes non compatibles TRIM"; exit 1 }
"Optimisation TRIM effectuée sur $count volume(s)"
"#,
        "clear_clipboard" => r#"
try {
    Add-Type -AssemblyName System.Windows.Forms -ErrorAction Stop
    [System.Windows.Forms.Clipboard]::Clear()
} catch { Write-Output $_.Exception.Message; exit 1 }
"Presse-papiers vidé"
"#,
        _ => return Err(format!("Optimisation '{}' inconnue", opt_id)),
    };
    ps_ok(script)
}
