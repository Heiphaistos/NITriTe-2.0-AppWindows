use serde::Serialize;
use std::net::UdpSocket;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::error::NiTriTeError;

fn get_hostname() -> String {
    Command::new("hostname")
        .creation_flags(0x08000000)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkOverview {
    pub hostname: String,
    pub public_ip: Option<String>,
    pub default_gateway: String,
    pub interfaces: Vec<NetworkInterface>,
    pub dns_servers: Vec<String>,
    pub proxy_enabled: bool,
    pub proxy_server: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkInterface {
    pub name: String,
    pub description: String,
    pub mac: String,
    pub ip_v4: Vec<String>,
    pub ip_v6: Vec<String>,
    pub is_up: bool,
    pub is_loopback: bool,
    pub speed_mbps: u64,
    pub received_bytes: u64,
    pub sent_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionInfo {
    pub protocol: String,
    pub local_addr: String,
    pub remote_addr: String,
    pub state: String,
    pub pid: u32,
    pub process_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PingResult {
    pub host: String,
    pub avg_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub packet_loss: f64,
    pub success: bool,
}

pub fn get_network_overview() -> Result<NetworkOverview, NiTriTeError> {
    let hostname = get_hostname();

    // Script PS complet — retourne tout en JSON
    let ps = r#"
$OutputEncoding = [System.Text.Encoding]::UTF8; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8;
try {
    # Interfaces avec adresses IP
    $ifaces = @()
    $adapters = Get-NetAdapter -EA SilentlyContinue
    $ipAddrs  = Get-NetIPAddress  -EA SilentlyContinue
    foreach ($a in $adapters) {
        $v4 = @($ipAddrs | Where-Object { $_.InterfaceAlias -eq $a.Name -and $_.AddressFamily -eq 'IPv4' } | ForEach-Object { $_.IPAddress })
        $v6 = @($ipAddrs | Where-Object { $_.InterfaceAlias -eq $a.Name -and $_.AddressFamily -eq 'IPv6' } | ForEach-Object { $_.IPAddress })
        $stats = Get-NetAdapterStatistics -Name $a.Name -EA SilentlyContinue
        $ifaces += @{
            name        = $a.Name
            description = $a.InterfaceDescription
            mac         = $a.MacAddress
            ip_v4       = $v4
            ip_v6       = $v6
            is_up       = ($a.Status -eq 'Up')
            is_loopback = ($a.Name -match 'Loopback' -or $a.InterfaceDescription -match 'Loopback')
            speed_mbps  = if ($a.LinkSpeed -gt 0) { [math]::Round($a.LinkSpeed / 1000000) } else { 0 }
            rx_bytes    = if ($stats) { $stats.ReceivedBytes } else { 0 }
            tx_bytes    = if ($stats) { $stats.SentBytes } else { 0 }
        }
    }
    # Passerelle par defaut
    $gw = (Get-NetRoute -DestinationPrefix '0.0.0.0/0' -EA SilentlyContinue | Sort-Object RouteMetric | Select-Object -First 1).NextHop
    # DNS
    $dns = @((Get-DnsClientServerAddress -AddressFamily IPv4 -EA SilentlyContinue).ServerAddresses | Select-Object -Unique)
    # Proxy
    $p = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings' -EA SilentlyContinue
    @{
        interfaces      = $ifaces
        gateway         = if ($gw) { $gw } else { '' }
        dns             = $dns
        proxy_enabled   = ($p.ProxyEnable -eq 1)
        proxy_server    = if ($p.ProxyServer) { $p.ProxyServer } else { '' }
    } | ConvertTo-Json -Depth 5 -Compress
} catch {
    @{ interfaces=@(); gateway=''; dns=@(); proxy_enabled=$false; proxy_server=''; error=$_.Exception.Message } | ConvertTo-Json -Compress
}
"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", ps])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| NiTriTeError::System(e.to_string()))?;

    let text = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(text.trim()).unwrap_or(serde_json::Value::Null);

    let interfaces = v["interfaces"].as_array().unwrap_or(&vec![]).iter().map(|a| {
        let to_str_vec = |val: &serde_json::Value| -> Vec<String> {
            match val {
                serde_json::Value::Array(arr) => arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect(),
                serde_json::Value::String(s) => vec![s.clone()],
                _ => vec![],
            }
        };
        NetworkInterface {
            name:          a["name"].as_str().unwrap_or("").to_string(),
            description:   a["description"].as_str().unwrap_or("").to_string(),
            mac:           a["mac"].as_str().unwrap_or("").to_string(),
            ip_v4:         to_str_vec(&a["ip_v4"]),
            ip_v6:         to_str_vec(&a["ip_v6"]),
            is_up:         a["is_up"].as_bool().unwrap_or(false),
            is_loopback:   a["is_loopback"].as_bool().unwrap_or(false),
            speed_mbps:    a["speed_mbps"].as_u64().unwrap_or(0),
            received_bytes: a["rx_bytes"].as_u64().unwrap_or(0),
            sent_bytes:    a["tx_bytes"].as_u64().unwrap_or(0),
        }
    }).collect();

    let dns_servers: Vec<String> = v["dns"].as_array().unwrap_or(&vec![]).iter()
        .filter_map(|x| x.as_str().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect();

    Ok(NetworkOverview {
        hostname,
        public_ip: None, // Récupéré côté frontend via fetch
        default_gateway: v["gateway"].as_str().unwrap_or("").to_string(),
        interfaces,
        dns_servers,
        proxy_enabled: v["proxy_enabled"].as_bool().unwrap_or(false),
        proxy_server:  v["proxy_server"].as_str().unwrap_or("").to_string(),
    })
}

#[allow(dead_code)]
fn get_local_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("8.8.8.8:80")?;
            socket.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

pub fn get_connections() -> Result<Vec<ConnectionInfo>, NiTriTeError> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command",
            "Get-NetTCPConnection | Select-Object -First 100 LocalAddress, LocalPort, RemoteAddress, RemotePort, State, OwningProcess | ConvertTo-Json"])
        .creation_flags(0x08000000).output()?;

    let text = String::from_utf8_lossy(&output.stdout);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap_or_default();

    // Resoudre les noms de process via sysinfo
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    Ok(parsed.iter().map(|c| {
        let pid = c["OwningProcess"].as_u64().unwrap_or(0) as u32;
        let process_name = sys.process(sysinfo::Pid::from_u32(pid))
            .map(|p| p.name().to_string_lossy().to_string())
            .unwrap_or_default();
        ConnectionInfo {
            protocol: "TCP".to_string(),
            local_addr: format!("{}:{}", c["LocalAddress"].as_str().unwrap_or(""), c["LocalPort"].as_u64().unwrap_or(0)),
            remote_addr: format!("{}:{}", c["RemoteAddress"].as_str().unwrap_or(""), c["RemotePort"].as_u64().unwrap_or(0)),
            state: c["State"].as_u64().map(|s| match s { 2 => "Listen", 5 => "Established", 6 => "FinWait1", 8 => "CloseWait", 11 => "TimeWait", _ => "Other" }).unwrap_or("Unknown").to_string(),
            pid,
            process_name,
        }
    }).collect())
}

pub fn ping_host(host: &str) -> Result<PingResult, NiTriTeError> {
    // Validation de l'hôte : lettres, chiffres, tirets, points, deux-points, crochets uniquement
    let host = host.trim();
    if host.is_empty() || host.len() > 255 {
        return Err(NiTriTeError::System("Hôte invalide (vide ou trop long)".into()));
    }
    let valid = host.chars().all(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']' | '_'));
    if !valid {
        return Err(NiTriTeError::System(format!("Caractères invalides dans l'hôte: {}", host)));
    }
    let output = Command::new("ping")
        .args(["-n", "4", host])
        .creation_flags(0x08000000).output()?;

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let success = output.status.success();

    let mut min = 0.0f64;
    let mut max = 0.0f64;
    let mut avg = 0.0f64;

    for line in text.lines() {
        if line.contains("Minimum") || line.contains("Maximum") {
            let parts: Vec<&str> = line.split(',').collect();
            for part in &parts {
                let clean = part.replace("ms", "").replace("Minimum", "").replace("Maximum", "").replace("Moyenne", "").replace("Average", "").replace("=", "").trim().to_string();
                if let Ok(v) = clean.parse::<f64>() {
                    if part.contains("Minimum") || part.contains("Min") { min = v; }
                    else if part.contains("Maximum") || part.contains("Max") { max = v; }
                    else { avg = v; }
                }
            }
        }
    }

    Ok(PingResult {
        host: host.to_string(),
        avg_ms: avg,
        min_ms: min,
        max_ms: max,
        packet_loss: if success { 0.0 } else { 100.0 },
        success,
    })
}
