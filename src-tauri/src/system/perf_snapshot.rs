use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct DiskPerf {
    pub name: String,
    pub read_mb: f64,
    pub write_mb: f64,
    pub queue_length: f64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct NetPerf {
    pub name: String,
    pub recv_mb: f64,
    pub sent_mb: f64,
}

#[derive(Debug, Default, Serialize)]
pub struct PerfSnapshot {
    pub cpu_percent: f64,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub ram_percent: f64,
    pub page_file_used_gb: f64,
    pub page_file_total_gb: f64,
    pub disks: Vec<DiskPerf>,
    pub network: Vec<NetPerf>,
    pub handle_count: u64,
    pub thread_count: u64,
    pub process_count: u64,
    pub uptime_hours: f64,
}

// Anti-freeze : WMI est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn get_perf_snapshot() -> PerfSnapshot {
    tokio::task::spawn_blocking(get_perf_snapshot_blocking)
        .await
        .unwrap_or_default()
}

fn get_perf_snapshot_blocking() -> PerfSnapshot {
    let ps = r#"
$out = @{}

# CPU
try {
    $cpu = Get-WmiObject Win32_Processor -ErrorAction SilentlyContinue | Select-Object -First 1
    $out.CpuPct = [double]$cpu.LoadPercentage
} catch { $out.CpuPct = 0.0 }

# RAM
try {
    $os = Get-WmiObject Win32_OperatingSystem -ErrorAction SilentlyContinue
    $total = [double]$os.TotalVisibleMemorySize / 1024 / 1024
    $free  = [double]$os.FreePhysicalMemory  / 1024 / 1024
    $used  = $total - $free
    $out.RamUsedGB  = [math]::Round($used, 2)
    $out.RamTotalGB = [math]::Round($total, 2)
    $out.RamPct     = if ($total -gt 0) { [math]::Round($used / $total * 100, 1) } else { 0.0 }
    $pfTotal = [double]$os.TotalVirtualMemorySize / 1024 / 1024
    $pfFree  = [double]$os.FreeVirtualMemory  / 1024 / 1024
    $out.PageUsedGB  = [math]::Round($pfTotal - $pfFree, 2)
    $out.PageTotalGB = [math]::Round($pfTotal, 2)
} catch { $out.RamUsedGB=0;$out.RamTotalGB=0;$out.RamPct=0;$out.PageUsedGB=0;$out.PageTotalGB=0 }

# Disques IO (simple approche via Win32_PerfRawData)
try {
    $di = Get-WmiObject Win32_PerfFormattedData_PerfDisk_PhysicalDisk -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -ne '_Total' }
    $out.Disks = @($di | ForEach-Object {
        @{
            name  = [string]$_.Name
            readM = [math]::Round($_.DiskReadBytesPersec / 1MB, 2)
            writeM= [math]::Round($_.DiskWriteBytesPersec / 1MB, 2)
            queue = [double]$_.CurrentDiskQueueLength
        }
    })
} catch { $out.Disks = @() }

# Réseau IO
try {
    $ni = Get-WmiObject Win32_PerfFormattedData_Tcpip_NetworkInterface -ErrorAction SilentlyContinue
    $out.Network = @($ni | Where-Object { $_.Name -notmatch 'Loopback|isatap|Teredo' } | ForEach-Object {
        @{
            name  = [string]$_.Name
            recvM = [math]::Round($_.BytesReceivedPerSec / 1MB, 3)
            sentM = [math]::Round($_.BytesSentPerSec / 1MB, 3)
        }
    })
} catch { $out.Network = @() }

# Process/thread/handle counts
try {
    $procs = Get-Process -ErrorAction SilentlyContinue
    $out.ProcessCt = [int]($procs | Measure-Object).Count
    $out.ThreadCt  = [long]($procs | Measure-Object -Property Threads -Sum).Sum
    $out.HandleCt  = [long]($procs | Measure-Object -Property HandleCount -Sum).Sum
} catch { $out.ProcessCt=0; $out.ThreadCt=0; $out.HandleCt=0 }

# Uptime
try {
    $os2 = Get-WmiObject Win32_OperatingSystem -ErrorAction SilentlyContinue
    $last = [Management.ManagementDateTimeConverter]::ToDateTime($os2.LastBootUpTime)
    $out.UptimeH = [math]::Round((New-TimeSpan -Start $last).TotalHours, 1)
} catch { $out.UptimeH = 0.0 }

$out | ConvertTo-Json -Depth 3 -Compress
"#;

    #[cfg(target_os = "windows")]
    {
        // .output() n'avait aucun timeout — interrogé toutes les 3s par
        // DiagTabPerf.vue tant que l'auto-refresh est actif (même famille de bug
        // que wmi_timeout/check_command/ps()/monitor.rs, cycles 153-156) : un
        // figement WMI ici gelait l'onglet Performances indéfiniment.
        let output = crate::maintenance::commands::execute_system_command(
            "powershell",
            &["-NoProfile", "-NonInteractive", "-Command", ps],
            5,
        );

        if let Ok(o) = output {
            let v: serde_json::Value = match serde_json::from_str(o.stdout.trim()) {
                Ok(v) => v,
                Err(_) => return PerfSnapshot::default(),
            };

            let disks = v["Disks"].as_array().map(|arr| {
                arr.iter().map(|d| DiskPerf {
                    name: d["name"].as_str().unwrap_or("").to_string(),
                    read_mb: d["readM"].as_f64().unwrap_or(0.0),
                    write_mb: d["writeM"].as_f64().unwrap_or(0.0),
                    queue_length: d["queue"].as_f64().unwrap_or(0.0),
                }).collect()
            }).unwrap_or_default();

            let network = v["Network"].as_array().map(|arr| {
                arr.iter().map(|n| NetPerf {
                    name: n["name"].as_str().unwrap_or("").to_string(),
                    recv_mb: n["recvM"].as_f64().unwrap_or(0.0),
                    sent_mb: n["sentM"].as_f64().unwrap_or(0.0),
                }).collect()
            }).unwrap_or_default();

            return PerfSnapshot {
                cpu_percent: v["CpuPct"].as_f64().unwrap_or(0.0),
                ram_used_gb: v["RamUsedGB"].as_f64().unwrap_or(0.0),
                ram_total_gb: v["RamTotalGB"].as_f64().unwrap_or(0.0),
                ram_percent: v["RamPct"].as_f64().unwrap_or(0.0),
                page_file_used_gb: v["PageUsedGB"].as_f64().unwrap_or(0.0),
                page_file_total_gb: v["PageTotalGB"].as_f64().unwrap_or(0.0),
                disks,
                network,
                handle_count: v["HandleCt"].as_u64().unwrap_or(0),
                thread_count: v["ThreadCt"].as_u64().unwrap_or(0),
                process_count: v["ProcessCt"].as_u64().unwrap_or(0),
                uptime_hours: v["UptimeH"].as_f64().unwrap_or(0.0),
            };
        }
    }
    PerfSnapshot::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test d'intégration en conditions réelles sur cette machine — vérifie que
    // le refactor .output() → execute_system_command (ajout du timeout, cycle
    // 157) n'a pas cassé le chemin heureux. Le mécanisme timeout+kill lui-même
    // est déjà prouvé par le test dédié d'execute_system_command (cycle 149).
    #[test]
    fn get_perf_snapshot_blocking_does_not_panic_and_returns_ram_total() {
        let snap = get_perf_snapshot_blocking();
        assert!(snap.ram_total_gb > 0.0, "une vraie machine a toujours au moins un peu de RAM totale");
    }
}
