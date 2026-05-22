use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use sysinfo::{Disks, Networks, System};
use tauri::Emitter;

#[derive(Debug, Clone, Serialize)]
pub struct GpuData {
    pub name: String,
    pub usage_percent: f32,
    pub vram_used_mb: u64,
    pub vram_total_mb: u64,
    pub temperature_c: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskTemp {
    pub name: String,
    pub temp_c: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonitorData {
    pub cpu_percent: f32,
    pub cpu_per_core: Vec<f32>,
    pub cpu_freq_mhz: u64,
    pub cpu_name: String,
    pub cpu_temp_c: Option<i32>,
    pub ram_percent: f32,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub disk_percent: f32,
    pub disk_read_kbs: f64,
    pub disk_write_kbs: f64,
    pub disk_temps: Vec<DiskTemp>,
    pub network_up_kbs: f64,
    pub network_down_kbs: f64,
    pub battery: Option<BatteryData>,
    pub gpu_data: Vec<GpuData>,
    pub top_processes: Vec<ProcessData>,
    pub alerts: Vec<AlertData>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatteryData {
    pub percent: f32,
    pub plugged: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessData {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlertData {
    pub alert_type: String,
    pub level: String,
    pub message: String,
}

pub fn start_monitoring(
    window: tauri::Window,
    running: Arc<AtomicBool>,
    interval_ms: u64,
) {
    // Guard : si déjà actif, on ne spawne pas un second thread
    if running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    // GPU + temperature data shared between polling thread and main monitoring thread
    let gpu_shared: Arc<Mutex<Vec<GpuData>>> = Arc::new(Mutex::new(Vec::new()));
    let gpu_shared_writer = gpu_shared.clone();
    let temp_shared: Arc<Mutex<(Option<i32>, Vec<DiskTemp>)>> = Arc::new(Mutex::new((None, vec![])));
    let temp_shared_writer = temp_shared.clone();
    let running_gpu = running.clone();

    // Dedicated GPU + temperature polling thread (every ~4s)
    std::thread::spawn(move || {
        while running_gpu.load(Ordering::SeqCst) {
            if let Ok(gpus) = collect_gpu_data() {
                if let Ok(mut g) = gpu_shared_writer.lock() {
                    *g = gpus;
                }
            }
            if let Ok((cpu_t, disk_t)) = collect_temperatures() {
                if let Ok(mut t) = temp_shared_writer.lock() {
                    *t = (cpu_t, disk_t);
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(4));
        }
    });

    std::thread::spawn(move || {
        let mut sys = System::new_all();
        let mut networks = Networks::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        let mut prev_rx: u64 = networks.values().map(|n| n.total_received()).sum();
        let mut prev_tx: u64 = networks.values().map(|n| n.total_transmitted()).sum();
        let mut prev_disk_read: u64 = 0u64;
        let mut prev_disk_write: u64 = 0u64;
        // Compteur de ticks : refresh processus tous les 5 ticks seulement
        // (ex: interval 1s → refresh processus toutes les 5s au lieu de chaque seconde)
        let mut tick: u8 = 0;

        while running.load(Ordering::SeqCst) {
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            // Refresh processus allégé : seulement CPU/mémoire des processus existants,
            // re-scan complet uniquement toutes les 5 itérations.
            if tick == 0 {
                sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            } else {
                sys.refresh_processes(sysinfo::ProcessesToUpdate::All, false);
            }
            tick = (tick + 1) % 5;
            networks.refresh();
            disks.refresh();

            let cpu_percent = sys.global_cpu_usage();
            let cpu_per_core: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
            let cpu_freq = sys.cpus().first().map(|c| c.frequency()).unwrap_or(0);
            let cpu_name = sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default();

            let total_mem = sys.total_memory() as f64 / 1_073_741_824.0;
            let used_mem = sys.used_memory() as f64 / 1_073_741_824.0;
            let ram_percent = if total_mem > 0.0 { (used_mem / total_mem * 100.0) as f32 } else { 0.0 };

            // Disque usage
            let disk_percent = disks.iter()
                .find(|d| d.mount_point().to_string_lossy().starts_with("C:"))
                .map(|d| {
                    let total = d.total_space() as f64;
                    let avail = d.available_space() as f64;
                    if total > 0.0 { ((total - avail) / total * 100.0) as f32 } else { 0.0 }
                })
                .unwrap_or(0.0);
            // Disk I/O via WMI (sysinfo 0.32 ne fournit pas de bytes I/O)
            let interval_s = interval_ms as f64 / 1000.0;
            let (disk_read_kbs, disk_write_kbs) = {
                let (cur_r, cur_w) = collect_disk_io().unwrap_or((0, 0));
                let r = cur_r.saturating_sub(prev_disk_read) as f64 / 1024.0 / interval_s;
                let w = cur_w.saturating_sub(prev_disk_write) as f64 / 1024.0 / interval_s;
                prev_disk_read = cur_r;
                prev_disk_write = cur_w;
                (r, w)
            };

            // Réseau
            let cur_rx: u64 = networks.values().map(|n| n.total_received()).sum();
            let cur_tx: u64 = networks.values().map(|n| n.total_transmitted()).sum();
            let down_kbs = (cur_rx.saturating_sub(prev_rx)) as f64 / 1024.0 / interval_s;
            let up_kbs = (cur_tx.saturating_sub(prev_tx)) as f64 / 1024.0 / interval_s;
            prev_rx = cur_rx;
            prev_tx = cur_tx;

            // Top processus
            let mut procs: Vec<ProcessData> = sys.processes().values()
                .map(|p| ProcessData {
                    pid: p.pid().as_u32(),
                    name: p.name().to_string_lossy().to_string(),
                    cpu_percent: p.cpu_usage(),
                    memory_mb: p.memory() as f64 / 1_048_576.0,
                })
                .collect();
            procs.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
            procs.truncate(10);

            // Alertes
            let mut alerts = Vec::new();
            if cpu_percent > 90.0 {
                alerts.push(AlertData { alert_type: "cpu".into(), level: "critical".into(), message: format!("CPU a {}%", cpu_percent as u32) });
            } else if cpu_percent > 80.0 {
                alerts.push(AlertData { alert_type: "cpu".into(), level: "warning".into(), message: format!("CPU a {}%", cpu_percent as u32) });
            }
            if ram_percent > 90.0 {
                alerts.push(AlertData { alert_type: "ram".into(), level: "critical".into(), message: format!("RAM a {}%", ram_percent as u32) });
            } else if ram_percent > 85.0 {
                alerts.push(AlertData { alert_type: "ram".into(), level: "warning".into(), message: format!("RAM a {}%", ram_percent as u32) });
            }
            if disk_percent > 95.0 {
                alerts.push(AlertData { alert_type: "disk".into(), level: "critical".into(), message: format!("Disque a {}%", disk_percent as u32) });
            } else if disk_percent > 90.0 {
                alerts.push(AlertData { alert_type: "disk".into(), level: "warning".into(), message: format!("Disque a {}%", disk_percent as u32) });
            }

            let gpu_snapshot = gpu_shared.lock().map(|g| g.clone()).unwrap_or_default();
            let (cpu_temp_c, disk_temps) = temp_shared.lock().map(|t| t.clone()).unwrap_or((None, vec![]));

            let data = MonitorData {
                cpu_percent,
                cpu_per_core,
                cpu_freq_mhz: cpu_freq,
                cpu_name,
                cpu_temp_c,
                ram_percent,
                ram_used_gb: used_mem,
                ram_total_gb: total_mem,
                disk_percent,
                disk_read_kbs,
                disk_write_kbs,
                disk_temps,
                network_up_kbs: up_kbs,
                network_down_kbs: down_kbs,
                battery: get_battery_info(),
                gpu_data: gpu_snapshot,
                top_processes: procs,
                alerts,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };

            let _ = window.emit("system-monitor", &data);
            std::thread::sleep(std::time::Duration::from_millis(interval_ms));
        }
    });
}

fn collect_gpu_data() -> Result<Vec<GpuData>, String> {
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    // Try nvidia-smi first (NVIDIA GPUs)
    let nvidia = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu",
               "--format=csv,noheader,nounits"])
        .creation_flags(0x08000000)
        .output();

    if let Ok(out) = nvidia {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            let gpus: Vec<GpuData> = text.lines()
                .filter(|l| !l.trim().is_empty())
                .map(|line| {
                    let parts: Vec<&str> = line.splitn(5, ',').map(str::trim).collect();
                    GpuData {
                        name: parts.first().unwrap_or(&"GPU").to_string(),
                        usage_percent: parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                        vram_used_mb: parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
                        vram_total_mb: parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0),
                        temperature_c: parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
                    }
                })
                .collect();
            if !gpus.is_empty() {
                return Ok(gpus);
            }
        }
    }

    // Fallback: PowerShell GPU counters (AMD/Intel/others)
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let ps = r#"
try {
    $gpus = Get-WmiObject Win32_VideoController -ErrorAction Stop | Where-Object { $_.Name -notlike '*Basic*' -and $_.Name -notlike '*Miroir*' }
    $result = foreach ($g in $gpus) {
        $usage = 0
        try {
            $counters = (Get-Counter '\GPU Engine(*_phys_*engrtype_3D)\Utilization Percentage' -ErrorAction SilentlyContinue).CounterSamples
            if ($counters) { $usage = [math]::Round(($counters | Measure-Object -Property CookedValue -Sum).Sum) }
        } catch {}
        $vramTotal = if ($g.AdapterRAM -gt 0) { [math]::Round($g.AdapterRAM / 1MB) } else { 0 }
        [PSCustomObject]@{ Name=$g.Name; Usage=$usage; VramUsed=0; VramTotal=$vramTotal; Temp=0 }
    }
    $result | ConvertTo-Json -Compress
} catch { Write-Output '[]' }
"#;
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout);
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed == "[]" { return Ok(vec![]); }
        let arr: Vec<serde_json::Value> = serde_json::from_str(trimmed)
            .unwrap_or_else(|_| serde_json::from_str(&format!("[{}]", trimmed)).unwrap_or_default());
        Ok(arr.iter().map(|v| GpuData {
            name: v["Name"].as_str().unwrap_or("GPU").to_string(),
            usage_percent: v["Usage"].as_f64().unwrap_or(0.0) as f32,
            vram_used_mb: v["VramUsed"].as_u64().unwrap_or(0),
            vram_total_mb: v["VramTotal"].as_u64().unwrap_or(0),
            temperature_c: v["Temp"].as_i64().unwrap_or(0) as i32,
        }).collect())
    }

    #[cfg(not(target_os = "windows"))]
    Ok(vec![])
}

fn collect_temperatures() -> Result<(Option<i32>, Vec<DiskTemp>), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let ps = r#"
$out = @{ cpu_temp = $null; disk_temps = @() }
try {
    $zones = Get-WmiObject -Namespace root/wmi -Class MSAcpi_ThermalZoneTemperature -ErrorAction Stop
    $vals = $zones | ForEach-Object { [math]::Round($_.CurrentTemperature / 10.0 - 273.15) } | Where-Object { $_ -gt 0 -and $_ -lt 120 }
    if ($vals) { $out.cpu_temp = ($vals | Measure-Object -Minimum).Minimum }
} catch {}
try {
    $disks = Get-PhysicalDisk -ErrorAction Stop | Where-Object { $_.Temperature -gt 0 } | Select-Object FriendlyName, Temperature
    $out.disk_temps = @($disks | ForEach-Object { @{ name = $_.FriendlyName; temp = [int]$_.Temperature } })
} catch {}
$out | ConvertTo-Json -Compress -Depth 3
"#;
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout);
        let v: serde_json::Value = serde_json::from_str(text.trim()).unwrap_or_default();
        let cpu_temp = v.get("cpu_temp").and_then(|t| t.as_i64()).map(|t| t as i32);
        let disk_temps = v.get("disk_temps").and_then(|a| a.as_array()).map(|arr| {
            arr.iter().filter_map(|item| {
                let name = item.get("name")?.as_str()?.to_string();
                let temp = item.get("temp")?.as_i64()? as i32;
                if temp > 0 { Some(DiskTemp { name, temp_c: temp }) } else { None }
            }).collect()
        }).unwrap_or_default();
        Ok((cpu_temp, disk_temps))
    }
    #[cfg(not(target_os = "windows"))]
    Ok((None, vec![]))
}

fn collect_disk_io() -> Result<(u64, u64), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let ps = r#"
try {
    $d = Get-WmiObject -Class Win32_PerfRawData_PerfDisk_LogicalDisk -Filter "Name='_Total'" -ErrorAction Stop
    "$($d.DiskReadBytesPersec) $($d.DiskWriteBytesPersec)"
} catch { "0 0" }
"#;
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout);
        let parts: Vec<&str> = text.split_whitespace().collect();
        let r = parts.first().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        let w = parts.get(1).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        Ok((r, w))
    }
    #[cfg(not(target_os = "windows"))]
    Ok((0, 0))
}

fn get_battery_info() -> Option<BatteryData> {
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    let output = std::process::Command::new("powershell")
        .args(["-Command", "(Get-WmiObject Win32_Battery | Select-Object EstimatedChargeRemaining, BatteryStatus | ConvertTo-Json)"])
        .creation_flags(0x08000000)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.trim();
    if text.is_empty() {
        return None; // Pas de batterie (desktop)
    }

    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let percent = v.get("EstimatedChargeRemaining")?.as_f64()? as f32;
    // BatteryStatus: 2 = AC, 1 = discharging
    let status = v.get("BatteryStatus")?.as_u64().unwrap_or(0);
    let plugged = status == 2;

    Some(BatteryData { percent, plugged })
}
