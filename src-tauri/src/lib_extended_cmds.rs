
// === Nouveaux modules Diagnostic ===

#[tauri::command]
async fn get_running_processes() -> Result<Vec<system::processes::ProcessInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(system::processes::collect_processes)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn get_windows_services() -> Result<Vec<system::services::ServiceInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(system::services::collect_services)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
        .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn get_security_status() -> Result<system::security::SecurityStatus, NiTriTeError> {
    tokio::task::spawn_blocking(system::security::collect_security_status)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn get_scheduled_tasks() -> Result<Vec<system::tasks::ScheduledTask>, NiTriTeError> {
    tokio::task::spawn_blocking(system::tasks::collect_scheduled_tasks)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn get_active_connections() -> Result<Vec<system::connections::TcpConnection>, NiTriTeError> {
    tokio::task::spawn_blocking(system::connections::collect_connections)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn get_wifi_status() -> Result<Option<system::connections::WifiInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(system::connections::collect_wifi_info)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn get_logical_volumes() -> Result<Vec<system::extra::VolumeInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(system::extra::collect_volumes)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
        .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn get_cpu_extended() -> Result<system::extra::CpuExtended, NiTriTeError> {
    tokio::task::spawn_blocking(system::extra::collect_cpu_extended)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
        .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn get_os_extended() -> Result<system::extra::OsExtended, NiTriTeError> {
    tokio::task::spawn_blocking(system::extra::collect_os_extended)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
        .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn get_folder_sizes_detailed() -> Result<Vec<system::extra::FolderSizeInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(system::extra::collect_folder_sizes)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn get_startup_programs_detailed() -> Result<Vec<system::extra::StartupProgram>, NiTriTeError> {
    tokio::task::spawn_blocking(system::extra::collect_startup_programs)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn get_smart_info() -> Vec<system::extra::SmartDiskInfo> {
    tokio::task::spawn_blocking(system::extra::collect_smart_info)
        .await
        .unwrap_or_default()
}

// === Debloat ===

#[tauri::command]
async fn debloat_disable_telemetry() -> Result<maintenance::debloat::DebloatResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::debloat::disable_telemetry)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn debloat_disable_cortana() -> Result<maintenance::debloat::DebloatResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::debloat::disable_cortana)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn debloat_disable_xbox() -> Result<maintenance::debloat::DebloatResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::debloat::disable_xbox_services)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn debloat_disable_superfetch() -> Result<maintenance::debloat::DebloatResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::debloat::disable_superfetch)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn debloat_disable_tips() -> Result<maintenance::debloat::DebloatResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::debloat::disable_windows_tips)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn debloat_optimize_power() -> Result<maintenance::debloat::DebloatResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::debloat::optimize_power_plan)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn debloat_disable_visual_effects() -> Result<maintenance::debloat::DebloatResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::debloat::disable_visual_effects)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn debloat_clear_event_logs() -> Result<maintenance::debloat::DebloatResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::debloat::clear_event_logs)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn debloat_clear_wu_cache() -> Result<maintenance::debloat::DebloatResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::debloat::clear_windowsupdate_cache)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn debloat_flush_dns() -> Result<maintenance::debloat::DebloatResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::debloat::flush_dns)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn debloat_reset_network() -> Result<maintenance::debloat::DebloatResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::debloat::reset_network_stack)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn debloat_enable_trim() -> Result<maintenance::debloat::DebloatResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::debloat::enable_trim)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn debloat_remove_bloatware(apps: Vec<String>) -> Result<Vec<maintenance::debloat::DebloatResult>, NiTriTeError> {
    tokio::task::spawn_blocking(move || maintenance::debloat::remove_bloatware_uwp(apps))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// === Chocolatey ===

#[tauri::command]
async fn check_chocolatey() -> bool {
    tokio::task::spawn_blocking(installer::chocolatey::check_chocolatey)
        .await
        .unwrap_or(false)
}

#[tauri::command]
async fn list_chocolatey_upgrades() -> Result<Vec<installer::chocolatey::ChocoPackage>, NiTriTeError> {
    tokio::task::spawn_blocking(installer::chocolatey::list_chocolatey_upgrades)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn upgrade_chocolatey_all() -> Result<installer::chocolatey::ChocoUpgradeResult, NiTriTeError> {
    tokio::task::spawn_blocking(installer::chocolatey::upgrade_chocolatey_all)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// === Windows Updates ===

#[derive(serde::Serialize, Clone)]
pub struct WinUpdate {
    pub hotfix_id: String,
    pub description: String,
    pub installed_on: String,
}

#[tauri::command]
async fn check_windows_updates() -> Result<Vec<WinUpdate>, NiTriTeError> {
    tokio::task::spawn_blocking(|| {
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-HotFix | Sort-Object InstalledOn -Descending | Select-Object -First 30 HotFixID,Description,InstalledOn | ConvertTo-Json -Compress",
            ])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| NiTriTeError::System(e.to_string()))?;

        let text = String::from_utf8_lossy(&output.stdout).to_string();
        let json: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap_or_default();

        Ok(json
            .iter()
            .map(|v| WinUpdate {
                hotfix_id: v["HotFixID"].as_str().unwrap_or("").to_string(),
                description: v["Description"].as_str().unwrap_or("").to_string(),
                installed_on: v["InstalledOn"]
                    .as_str()
                    .unwrap_or("")
                    .split('T')
                    .next()
                    .unwrap_or("")
                    .to_string(),
            })
            .collect())
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// === Windows Updates Pending ===

#[derive(serde::Serialize, Clone)]
pub struct PendingUpdate {
    pub title: String,
    pub kb_ids: String,
    pub severity: String,
    pub size_mb: f64,
    pub is_downloaded: bool,
}

#[tauri::command]
async fn scan_pending_windows_updates() -> Vec<PendingUpdate> {
    tokio::task::spawn_blocking(|| {
        let ps = r#"
try {
    $session = New-Object -ComObject Microsoft.Update.Session -ErrorAction Stop
    $searcher = $session.CreateUpdateSearcher()
    $searcher.Online = $true
    $res = $searcher.Search("IsInstalled=0 and Type='Software'")
    $out = @()
    for ($i = 0; $i -lt $res.Updates.Count; $i++) {
        $u = $res.Updates.Item($i)
        $kbs = @(); for ($k=0;$k -lt $u.KBArticleIDs.Count;$k++) { $kbs += "KB$($u.KBArticleIDs.Item($k))" }
        $out += [PSCustomObject]@{
            title    = [string]$u.Title
            kb_ids   = $kbs -join ","
            severity = if ($u.MsrcSeverity) { [string]$u.MsrcSeverity } else { "Normal" }
            size_mb  = [math]::Round($u.MaxDownloadSize / 1MB, 1)
            dl       = [bool]$u.IsDownloaded
        }
    }
    $out | ConvertTo-Json -Compress -Depth 2
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
        let timeout = std::time::Duration::from_secs(60);
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    let mut buf = Vec::new();
                    if let Some(mut out) = child.stdout.take() { let _ = out.read_to_end(&mut buf); }
                    let text = String::from_utf8_lossy(&buf);
                    let t = text.trim();
                    if t.is_empty() || t == "[]" { return vec![]; }
                    let json_text = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
                    return serde_json::from_str::<Vec<serde_json::Value>>(&json_text)
                        .unwrap_or_default()
                        .iter()
                        .map(|v| PendingUpdate {
                            title: v["title"].as_str().unwrap_or("").to_string(),
                            kb_ids: v["kb_ids"].as_str().unwrap_or("").to_string(),
                            severity: v["severity"].as_str().unwrap_or("Normal").to_string(),
                            size_mb: v["size_mb"].as_f64().unwrap_or(0.0),
                            is_downloaded: v["dl"].as_bool().unwrap_or(false),
                        })
                        .collect();
                }
                Ok(None) => {
                    if start.elapsed() > timeout { let _ = child.kill(); let _ = child.wait(); return vec![]; }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(_) => { let _ = child.kill(); return vec![]; }
            }
        }
    }).await.unwrap_or_default()
}

#[tauri::command]
async fn trigger_windows_update() -> String {
    tokio::task::spawn_blocking(|| {
        // Déclenche le scan et l'install via UsoClient (Windows 10/11)
        // Chemin absolu pour éviter le PATH hijacking
        let r1 = std::process::Command::new(r"C:\Windows\System32\UsoClient.exe")
            .arg("StartInteractiveScan")
            .creation_flags(0x08000000)
            .output();
        if r1.is_ok() { return "Scan Windows Update déclenché".to_string(); }
        // Fallback: ouvre les paramètres Windows Update
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start ms-settings:windowsupdate"])
            .creation_flags(0x08000000)
            .spawn();
        "Paramètres Windows Update ouverts".to_string()
    }).await.unwrap_or_else(|_| "Erreur".to_string())
}

// === MAS Activation ===
// SÉCURITÉ: N'exécute PAS de script tiers depuis internet (iex/irm).
// Ouvre uniquement la page officielle Microsoft pour l'activation.

#[tauri::command]
async fn open_mas_window() -> Result<(), String> {
    // Ouvrir la page d'activation Windows dans le navigateur (plus sûr que IEX d'un script tiers)
    open::that("ms-settings:activation")
        .map_err(|e| e.to_string())
}

// === Network Extended ===

#[tauri::command]
async fn get_network_extended() -> serde_json::Value {
    tokio::task::spawn_blocking(|| {
        let ps = r#"
$result = @{}
function Do-Ping { param($h)
    try {
        $r = Test-Connection $h -Count 2 -ErrorAction SilentlyContinue
        if ($r) { $times = @($r | Select-Object -ExpandProperty ResponseTime); @{success=$true;avg=[math]::Round(($times|Measure-Object -Average).Average,1);host=$h} }
        else { @{success=$false;avg=0;host=$h} }
    } catch { @{success=$false;avg=0;host=$h} }
}
$gw = (Get-NetRoute -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue | Sort-Object RouteMetric | Select-Object -First 1).NextHop
if ($gw) { $result.ping_gateway = Do-Ping $gw } else { $result.ping_gateway = $null }
$result.ping_google = Do-Ping '8.8.8.8'
$result.ping_cloudflare = Do-Ping '1.1.1.1'
try { $result.public_ip = (Invoke-RestMethod -Uri 'https://api.ipify.org' -TimeoutSec 6 -ErrorAction SilentlyContinue) } catch { $result.public_ip = "" }
try {
    $entries = @()
    arp -a 2>$null | ForEach-Object { if ($_ -match '^\s+(\d+\.\d+\.\d+\.\d+)\s+([\w-]+)\s+(\w+)') { $entries += @{ip=$matches[1];mac=$matches[2];type=$matches[3]} } }
    $result.arp_table = $entries
} catch { $result.arp_table = @() }
try {
    $result.routes = @(Get-NetRoute -AddressFamily IPv4 -ErrorAction SilentlyContinue |
        Where-Object { $_.NextHop -ne '0.0.0.0' } | Sort-Object RouteMetric | Select-Object -First 40 |
        ForEach-Object { @{prefix=$_.DestinationPrefix;next_hop=$_.NextHop;metric=[int]$_.RouteMetric;iface=$_.InterfaceAlias} })
} catch { $result.routes = @() }
try {
    $prx = Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings' -ErrorAction SilentlyContinue
    $result.proxy = @{enabled=[bool]$prx.ProxyEnable;server=[string]$prx.ProxyServer;bypass=[string]$prx.ProxyOverride}
} catch { $result.proxy = @{enabled=$false;server="";bypass=""} }
try {
    $fw = Get-NetFirewallProfile -ErrorAction SilentlyContinue
    $result.firewall = @{
        domain=[bool]($fw | Where-Object Name -eq 'Domain' | Select-Object -ExpandProperty Enabled -ErrorAction SilentlyContinue)
        private=[bool]($fw | Where-Object Name -eq 'Private' | Select-Object -ExpandProperty Enabled -ErrorAction SilentlyContinue)
        public=[bool]($fw | Where-Object Name -eq 'Public' | Select-Object -ExpandProperty Enabled -ErrorAction SilentlyContinue)
    }
} catch { $result.firewall = @{domain=$false;private=$false;public=$false} }
try {
    $result.shares = @(Get-SmbShare -ErrorAction SilentlyContinue |
        ForEach-Object { @{name=$_.Name;path=[string]$_.Path;desc=[string]$_.Description} })
} catch { $result.shares = @() }
try {
    $result.stats = @(Get-NetAdapterStatistics -ErrorAction SilentlyContinue |
        ForEach-Object { @{name=$_.Name;recv_bytes=[long]$_.ReceivedBytes;sent_bytes=[long]$_.SentBytes} })
} catch { $result.stats = @() }
try {
    $he = @()
    Get-Content 'C:\Windows\System32\drivers\etc\hosts' -ErrorAction SilentlyContinue | ForEach-Object {
        if ($_ -notmatch '^\s*#' -and $_.Trim() -ne '') {
            $p = $_.Trim() -split '\s+'; if ($p.Count -ge 2) { $he += @{ip=$p[0];host=$p[1]} }
        }
    }
    $result.hosts_entries = $he
} catch { $result.hosts_entries = @() }
try {
    $result.dns_test = @(Resolve-DnsName 'google.com' -ErrorAction SilentlyContinue | Select-Object -First 5 |
        ForEach-Object { @{name=[string]$_.Name;ip=if($_.IPAddress){[string]$_.IPAddress}else{""};type=[string]$_.Type} })
} catch { $result.dns_test = @() }
try {
    $wf = @(); netsh wlan show networks 2>$null | ForEach-Object { if ($_ -match 'SSID\s+\d+\s*:\s*(.+)') { $wf += $matches[1].Trim() } }
    $result.wifi_networks = $wf
} catch { $result.wifi_networks = @() }
$result | ConvertTo-Json -Depth 4 -Compress
"#;
        use std::io::Read;
        let mut child = match std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn() {
                Ok(c) => c,
                Err(_) => return serde_json::Value::Object(serde_json::Map::new()),
            };
        let timeout = std::time::Duration::from_secs(35);
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    let mut buf = Vec::new();
                    if let Some(mut out) = child.stdout.take() { let _ = out.read_to_end(&mut buf); }
                    return serde_json::from_str(String::from_utf8_lossy(&buf).trim())
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                }
                Ok(None) => {
                    if start.elapsed() > timeout { let _ = child.kill(); let _ = child.wait(); break; }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(_) => { let _ = child.kill(); break; }
            }
        }
        serde_json::Value::Object(serde_json::Map::new())
    }).await.unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
}

// === Scoop ===

#[tauri::command]
async fn check_scoop() -> bool {
    // Scoop est un script PowerShell — ne pas l'invoquer directement comme exe
    tokio::task::spawn_blocking(|| {
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command",
                "if (Get-Command scoop -ErrorAction SilentlyContinue) { exit 0 } else { exit 1 }"])
            .creation_flags(0x08000000)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }).await.unwrap_or(false)
}

#[derive(serde::Serialize, Clone)]
pub struct ScoopPackage { pub name: String, pub installed: String, pub available: String }

#[tauri::command]
async fn list_scoop_upgrades() -> Vec<ScoopPackage> {
    tokio::task::spawn_blocking(|| {
        // ConvertFrom-String est déprécié — on parse manuellement la sortie de "scoop status"
        let ps = r#"
$lines = @(scoop status 2>$null)
$rows = @()
foreach ($line in $lines) {
    $t = $line.Trim()
    if ($t -eq '' -or $t -match '^Name' -or $t -match '^-{3}' -or $t -match '^Scoop') { continue }
    $parts = $t -split '\s{2,}'
    if ($parts.Count -ge 2) {
        $rows += [PSCustomObject]@{
            name      = $parts[0].Trim()
            installed = if ($parts.Count -ge 2) { $parts[1].Trim() } else { '' }
            available = if ($parts.Count -ge 3) { $parts[2].Trim() } else { '' }
        }
    }
}
$rows | ConvertTo-Json -Compress
"#;
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output();
        if let Ok(o) = out {
            let text = String::from_utf8_lossy(&o.stdout);
            let t = text.trim();
            if t.is_empty() || t == "null" { return vec![]; }
            let json_text = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&json_text) {
                return arr.iter().filter_map(|v| {
                    let name = v["name"].as_str().filter(|s| !s.is_empty())?.to_string();
                    Some(ScoopPackage {
                        name,
                        installed: v["installed"].as_str().unwrap_or("").to_string(),
                        available: v["available"].as_str().unwrap_or("").to_string(),
                    })
                }).collect();
            }
        }
        vec![]
    }).await.unwrap_or_default()
}

#[tauri::command]
async fn upgrade_scoop_all(window: tauri::Window) -> Result<(), NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        // 1. Met à jour Scoop lui-même, 2. Met à jour tous les apps, 3. Nettoie les vieilles versions
        let ps = "scoop update; scoop update * 2>&1; scoop cleanup * 2>&1; Write-Output 'Mise a jour Scoop terminee.'";
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| NiTriTeError::System(e.to_string()))?;
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = window.emit("scoop-upgrade-done", &text);
        Ok(())
    }).await.map_err(|e| NiTriTeError::System(e.to_string()))?
}

