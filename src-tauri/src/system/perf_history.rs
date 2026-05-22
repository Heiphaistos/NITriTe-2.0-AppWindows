use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Default)]
pub struct PerfPoint {
    pub timestamp: String,
    pub cpu_percent: f64,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub disk_read_mbps: f64,
    pub disk_write_mbps: f64,
    pub net_recv_mbps: f64,
    pub net_send_mbps: f64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PerfHistory {
    pub points: Vec<PerfPoint>,
    pub sample_interval_secs: u32,
    pub duration_secs: u32,
    pub avg_cpu: f64,
    pub peak_cpu: f64,
    pub avg_ram_mb: u64,
    pub peak_ram_mb: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct TopProcess {
    pub name: String,
    pub pid: u32,
    pub cpu_percent: f64,
    pub ram_mb: u64,
    pub disk_mbps: f64,
}

/// Historique de performances — non-bloquant via spawn_blocking
#[tauri::command]
pub async fn get_perf_history(samples: u32, interval_secs: u32) -> PerfHistory {
    let n = samples.clamp(5, 120);
    let interval = interval_secs.clamp(1, 60);

    let ps = format!(
        r#"
$n = {n}
$iv = {interval}
$points = @()

for ($i = 0; $i -lt $n; $i++) {{
    $ts = (Get-Date).ToString('HH:mm:ss')
    $cpu = try {{
        [math]::Round((Get-WmiObject Win32_Processor -EA Stop | Measure-Object LoadPercentage -Average).Average, 1)
    }} catch {{ 0 }}
    $os = Get-WmiObject Win32_OperatingSystem -EA SilentlyContinue
    $ramUsed  = if($os) {{ [math]::Round(($os.TotalVisibleMemorySize - $os.FreePhysicalMemory)/1024, 0) }} else {{ 0 }}
    $ramTotal = if($os) {{ [math]::Round($os.TotalVisibleMemorySize/1024, 0) }} else {{ 0 }}

    $diskRead = 0; $diskWrite = 0
    try {{
        $dc = Get-Counter '\PhysicalDisk(_Total)\Disk Read Bytes/sec','\PhysicalDisk(_Total)\Disk Write Bytes/sec' -EA Stop
        $diskRead  = [math]::Round($dc.CounterSamples[0].CookedValue / 1MB, 2)
        $diskWrite = [math]::Round($dc.CounterSamples[1].CookedValue / 1MB, 2)
    }} catch {{}}

    $netRecv = 0; $netSend = 0
    try {{
        $nc = Get-Counter '\Network Interface(*)\Bytes Received/sec','\Network Interface(*)\Bytes Sent/sec' -EA Stop
        $netRecv = [math]::Round(($nc.CounterSamples | Where-Object {{$_.Path -match 'Bytes Received'}} | Measure-Object CookedValue -Sum).Sum / 1MB, 3)
        $netSend = [math]::Round(($nc.CounterSamples | Where-Object {{$_.Path -match 'Bytes Sent'}} | Measure-Object CookedValue -Sum).Sum / 1MB, 3)
    }} catch {{}}

    $points += @{{
        ts        = $ts
        cpu       = [double]$cpu
        ramUsed   = [long]$ramUsed
        ramTotal  = [long]$ramTotal
        diskRead  = [double]$diskRead
        diskWrite = [double]$diskWrite
        netRecv   = [double]$netRecv
        netSend   = [double]$netSend
    }}
    if ($i -lt $n - 1) {{ Start-Sleep -Seconds $iv }}
}}

$avgCpu  = if($points.Count) {{ [math]::Round(($points | Measure-Object -Property cpu -Average).Average, 1) }} else {{ 0 }}
$peakCpu = if($points.Count) {{ [math]::Round(($points | Measure-Object -Property cpu -Maximum).Maximum, 1) }} else {{ 0 }}
$avgRam  = if($points.Count) {{ [long]($points | Measure-Object -Property ramUsed -Average).Average }} else {{ 0 }}
$peakRam = if($points.Count) {{ [long]($points | Measure-Object -Property ramUsed -Maximum).Maximum }} else {{ 0 }}

@{{
    points   = $points
    interval = $iv
    duration = $n * $iv
    avgCpu   = $avgCpu
    peakCpu  = $peakCpu
    avgRam   = $avgRam
    peakRam  = $peakRam
}} | ConvertTo-Json -Depth 4 -Compress
"#,
        n = n,
        interval = interval
    );

    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let o = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .creation_flags(0x08000000)
                .output();
            if let Ok(o) = o {
                let t = String::from_utf8_lossy(&o.stdout);
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(t.trim()) {
                    let points = v["points"].as_array().map(|arr| {
                        arr.iter().map(|p| PerfPoint {
                            timestamp: p["ts"].as_str().unwrap_or("").to_string(),
                            cpu_percent: p["cpu"].as_f64().unwrap_or(0.0),
                            ram_used_mb: p["ramUsed"].as_u64().unwrap_or(0),
                            ram_total_mb: p["ramTotal"].as_u64().unwrap_or(0),
                            disk_read_mbps: p["diskRead"].as_f64().unwrap_or(0.0),
                            disk_write_mbps: p["diskWrite"].as_f64().unwrap_or(0.0),
                            net_recv_mbps: p["netRecv"].as_f64().unwrap_or(0.0),
                            net_send_mbps: p["netSend"].as_f64().unwrap_or(0.0),
                        }).collect()
                    }).unwrap_or_default();

                    return PerfHistory {
                        points,
                        sample_interval_secs: v["interval"].as_u64().unwrap_or(interval as u64) as u32,
                        duration_secs: v["duration"].as_u64().unwrap_or(0) as u32,
                        avg_cpu: v["avgCpu"].as_f64().unwrap_or(0.0),
                        peak_cpu: v["peakCpu"].as_f64().unwrap_or(0.0),
                        avg_ram_mb: v["avgRam"].as_u64().unwrap_or(0),
                        peak_ram_mb: v["peakRam"].as_u64().unwrap_or(0),
                    };
                }
            }
        }
        PerfHistory::default()
    })
    .await
    .unwrap_or_default()
}

/// Top processus par CPU — non-bloquant
#[tauri::command]
pub async fn get_top_processes_by_cpu(limit: u32) -> Vec<TopProcess> {
    let n = limit.clamp(5, 50);
    let ps = format!(
        r#"
@(Get-Process -EA SilentlyContinue |
    Sort-Object CPU -Descending |
    Select-Object -First {n} |
    ForEach-Object {{
        @{{
            name = $_.ProcessName
            pid  = [int]$_.Id
            cpu  = [math]::Round($_.CPU, 2)
            ram  = [long][math]::Round($_.WorkingSet64/1MB, 1)
            disk = 0.0
        }}
    }}) | ConvertTo-Json -Compress
"#,
        n = n
    );

    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let o = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .creation_flags(0x08000000)
                .output();
            if let Ok(o) = o {
                let t = String::from_utf8_lossy(&o.stdout);
                let t = t.trim();
                let arr_t = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
                if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&arr_t) {
                    return arr.iter().map(|p| TopProcess {
                        name: p["name"].as_str().unwrap_or("").to_string(),
                        pid: p["pid"].as_u64().unwrap_or(0) as u32,
                        cpu_percent: p["cpu"].as_f64().unwrap_or(0.0),
                        ram_mb: p["ram"].as_u64().unwrap_or(0),
                        disk_mbps: p["disk"].as_f64().unwrap_or(0.0),
                    }).collect();
                }
            }
        }
        vec![]
    })
    .await
    .unwrap_or_default()
}
