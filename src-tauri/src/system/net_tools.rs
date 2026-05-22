use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Valide un nom d'hôte ou une adresse IP.
/// Autorise : lettres, chiffres, tirets, points, deux-points (IPv6), crochets.
/// Bloque tout le reste (injection PowerShell, backticks, $, etc.)
fn validate_host(host: &str) -> Result<String, String> {
    let h = host.trim().to_string();
    if h.is_empty() {
        return Err("Hôte vide".into());
    }
    if h.len() > 255 {
        return Err(format!("Hôte trop long ({} caractères, max 255)", h.len()));
    }
    let valid = h.chars().all(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']' | '_'));
    if !valid {
        return Err(format!("Caractères invalides dans l'hôte: {}", h));
    }
    Ok(h)
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PingDiagResult {
    pub host: String, pub success: bool, pub avg_ms: f64, pub min_ms: f64,
    pub max_ms: f64, pub packets_sent: u32, pub packets_received: u32, pub loss_percent: f64,
}
#[derive(Debug, Clone, Serialize, Default)]
pub struct TracertHop { pub hop: u32, pub address: String, pub ms: f64 }

#[derive(Debug, Clone, Serialize, Default)]
pub struct DnsResult { pub host: String, pub records: Vec<String>, pub query_type: String, pub success: bool }

#[derive(Debug, Clone, Serialize, Default)]
pub struct IpConfigAdapter {
    pub name: String, pub ipv4: String, pub ipv6: String, pub prefix_len: u32,
    pub gateway: String, pub dns_servers: Vec<String>, pub mac: String, pub dhcp_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ArpEntry { pub ip: String, pub mac: String, pub entry_type: String, pub interface: String }

#[derive(Debug, Clone, Serialize, Default)]
pub struct RouteEntry {
    pub network: String, pub netmask: String, pub gateway: String,
    pub interface: String, pub metric: u32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PortScanResult { pub port: u16, pub open: bool, pub service: String }

#[derive(Debug, Clone, Serialize, Default)]
pub struct WifiNetwork {
    pub ssid: String, pub bssid: String, pub signal_percent: u32,
    pub channel: String, pub auth: String, pub encryption: String, pub band: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct OpenPort {
    pub protocol: String, pub local_address: String, pub local_port: u16,
    pub remote_address: String, pub state: String, pub pid: u32, pub process: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct HttpCheckResult {
    pub url: String, pub status_code: u32, pub status_text: String,
    pub headers: Vec<String>, pub time_ms: u64, pub success: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct NetShareEntry { pub name: String, pub path: String, pub comment: String, pub host: String }

// ─── Ping ──────────────────────────────────────────────────────────────────────
#[tauri::command]
pub fn run_ping(host: String, count: u32) -> PingDiagResult {
    let count = count.clamp(1, 10);
    let h = match validate_host(&host) {
        Ok(h) => h,
        Err(e) => { tracing::warn!("run_ping: {}", e); return PingDiagResult::default(); }
    };
    let ps = format!(
        r#"try {{ $r = Test-Connection '{host}' -Count {count} -ErrorAction SilentlyContinue; if ($r) {{ $t = @($r | Select-Object -ExpandProperty ResponseTime); @{{s=$true;avg=[math]::Round(($t|Measure-Object -Average).Average,1);min=[math]::Round(($t|Measure-Object -Minimum).Minimum,1);max=[math]::Round(($t|Measure-Object -Maximum).Maximum,1);sent={count};recv=$t.Count;loss=[math]::Round((({count}-$t.Count)/{count})*100,1)}} | ConvertTo-Json -Compress }} else {{ '@{{\"s\":false,\"avg\":0,\"min\":0,\"max\":0,\"sent\":{count},\"recv\":0,\"loss\":100}}' }} }} catch {{ '@{{\"s\":false,\"avg\":0,\"min\":0,\"max\":0,\"sent\":{count},\"recv\":0,\"loss\":100}}' }}"#,
        host = h, count = count
    );
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",&ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout); let t = t.trim().trim_matches('\'');
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(t) {
                return PingDiagResult { host: h, success: v["s"].as_bool().unwrap_or(false), avg_ms: v["avg"].as_f64().unwrap_or(0.0), min_ms: v["min"].as_f64().unwrap_or(0.0), max_ms: v["max"].as_f64().unwrap_or(0.0), packets_sent: v["sent"].as_u64().unwrap_or(count as u64) as u32, packets_received: v["recv"].as_u64().unwrap_or(0) as u32, loss_percent: v["loss"].as_f64().unwrap_or(100.0) };
            }
        }
    }
    PingDiagResult { host: h, ..Default::default() }
}

// ─── Traceroute ────────────────────────────────────────────────────────────────
#[tauri::command]
pub fn run_traceroute(host: String) -> Vec<TracertHop> {
    let h = match validate_host(&host) {
        Ok(h) => h,
        Err(e) => { tracing::warn!("run_traceroute: {}", e); return vec![]; }
    };
    // Use tracert cmd (more reliable than Test-NetConnection for real ms)
    let cmd = format!("tracert -h 20 -w 1000 {}", h);
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("cmd").args(["/C",&cmd]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let text = String::from_utf8_lossy(&o.stdout);
            let mut hops = Vec::new();
            for line in text.lines() {
                let line = line.trim();
                // Tracert output: "  1    <1 ms    <1 ms    <1 ms  192.168.1.1"
                if line.is_empty() || line.starts_with("Tra") || line.starts_with("Rout") || line.starts_with("Over") { continue; }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(hop_num) = parts[0].parse::<u32>() {
                        let addr = parts.iter().rev().find(|s| s.contains('.') || s.contains(':') || **s == "*").unwrap_or(&"*");
                        let ms_val = parts.iter().find_map(|s| s.trim_end_matches("ms").parse::<f64>().ok()).unwrap_or(0.0);
                        hops.push(TracertHop { hop: hop_num, address: addr.to_string(), ms: ms_val });
                    }
                }
            }
            if !hops.is_empty() { return hops; }
        }
    }
    vec![]
}

// ─── DNS Lookup ────────────────────────────────────────────────────────────────
#[tauri::command]
pub fn run_nslookup(host: String, record_type: String) -> DnsResult {
    let h = match validate_host(&host) {
        Ok(h) => h,
        Err(e) => { tracing::warn!("run_nslookup: {}", e); return DnsResult::default(); }
    };
    let rtype = match record_type.to_uppercase().as_str() {
        "A"|"AAAA"|"MX"|"NS"|"TXT"|"CNAME"|"SOA"|"PTR"|"SRV" => record_type.to_uppercase(),
        _ => "A".to_string(),
    };
    let ps = format!(r#"try {{ $r = Resolve-DnsName '{host}' -Type {rtype} -ErrorAction SilentlyContinue; if ($r) {{ $recs = @($r | ForEach-Object {{ if($_.IPAddress){{[string]$_.IPAddress}}elseif($_.NameHost){{[string]$_.NameHost}}elseif($_.Exchange){{[string]$_.Exchange}}elseif($_.Strings){{$_.Strings -join ' '}}else{{[string]$_.Name}} }}); @{{ok=$true;recs=$recs;qt='{rtype}'}} | ConvertTo-Json -Compress }} else {{ '@{{\"ok\":false,\"recs\":[],\"qt\":\"{rtype}\"}}' }} }} catch {{ '@{{\"ok\":false,\"recs\":[],\"qt\":\"{rtype}\"}}' }}"#, host=h, rtype=rtype);
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",&ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout); let t = t.trim().trim_matches('\'');
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(t) {
                let records = v["recs"].as_array().map(|a| a.iter().filter_map(|r| r.as_str().map(|s| s.to_string())).collect()).unwrap_or_default();
                return DnsResult { host: h, records, query_type: rtype, success: v["ok"].as_bool().unwrap_or(false) };
            }
        }
    }
    DnsResult { host: h, query_type: rtype, ..Default::default() }
}

// ─── IP Config ─────────────────────────────────────────────────────────────────
#[tauri::command]
pub fn get_ip_config() -> Vec<IpConfigAdapter> {
    let ps = r#"
$result = @(Get-NetAdapter -ErrorAction SilentlyContinue | Where-Object { $_.Status -eq 'Up' } | ForEach-Object {
    $a = $_
    $ip4 = Get-NetIPAddress -InterfaceIndex $a.ifIndex -AddressFamily IPv4 -EA SilentlyContinue | Select-Object -First 1
    $ip6 = Get-NetIPAddress -InterfaceIndex $a.ifIndex -AddressFamily IPv6 -EA SilentlyContinue | Where-Object { $_.PrefixOrigin -ne 'WellKnown' } | Select-Object -First 1
    $gw  = (Get-NetRoute -InterfaceIndex $a.ifIndex -DestinationPrefix '0.0.0.0/0' -EA SilentlyContinue | Sort-Object RouteMetric | Select-Object -First 1).NextHop
    $dns = @(Get-DnsClientServerAddress -InterfaceIndex $a.ifIndex -AddressFamily IPv4 -EA SilentlyContinue | Select-Object -ExpandProperty ServerAddresses)
    $dhcp = (Get-NetIPInterface -InterfaceIndex $a.ifIndex -AddressFamily IPv4 -EA SilentlyContinue).Dhcp
    @{ name=$a.InterfaceDescription; ipv4=if($ip4){[string]$ip4.IPAddress}else{''}; ipv6=if($ip6){[string]$ip6.IPAddress}else{''}; plen=if($ip4){[int]$ip4.PrefixLength}else{0}; gw=if($gw){[string]$gw}else{''}; dns=$dns; mac=[string]$a.MacAddress; dhcp=($dhcp -eq 'Enabled') }
})
$result | ConvertTo-Json -Depth 4 -Compress
"#;
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout); let t = t.trim();
            let arr_t = if t.starts_with('{') { format!("[{}]",t) } else { t.to_string() };
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&arr_t) {
                return arr.iter().map(|a| IpConfigAdapter {
                    name: a["name"].as_str().unwrap_or("").to_string(), ipv4: a["ipv4"].as_str().unwrap_or("").to_string(), ipv6: a["ipv6"].as_str().unwrap_or("").to_string(), prefix_len: a["plen"].as_u64().unwrap_or(0) as u32, gateway: a["gw"].as_str().unwrap_or("").to_string(),
                    dns_servers: a["dns"].as_array().map(|d| d.iter().filter_map(|s| s.as_str().map(|x| x.to_string())).collect()).unwrap_or_default(),
                    mac: a["mac"].as_str().unwrap_or("").to_string(), dhcp_enabled: a["dhcp"].as_bool().unwrap_or(false),
                }).collect();
            }
        }
    }
    vec![]
}

// ─── Table ARP ─────────────────────────────────────────────────────────────────
#[tauri::command]
pub fn get_arp_table() -> Vec<ArpEntry> {
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("arp").args(["-a"]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let text = String::from_utf8_lossy(&o.stdout);
            let mut entries = Vec::new();
            let mut current_if = String::new();
            for line in text.lines() {
                let line = line.trim();
                if line.starts_with("Interface:") {
                    current_if = line.split(':').nth(1).unwrap_or("").trim().split("---").next().unwrap_or("").trim().to_string();
                    continue;
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 && (parts[0].contains('.') || parts[0].contains(':')) {
                    let mac = parts[1];
                    if mac.len() >= 11 && (mac.contains('-') || mac.contains(':')) {
                        entries.push(ArpEntry { ip: parts[0].to_string(), mac: mac.to_string(), entry_type: parts[2].to_string(), interface: current_if.clone() });
                    }
                }
            }
            return entries;
        }
    }
    vec![]
}

// ─── Table de routage ──────────────────────────────────────────────────────────
#[tauri::command]
pub fn get_route_table() -> Vec<RouteEntry> {
    let ps = r#"
@(Get-NetRoute -ErrorAction SilentlyContinue | ForEach-Object {
    @{ net=$_.DestinationPrefix; gw=[string]$_.NextHop; iface=[string]$_.InterfaceAlias; metric=[int]$_.RouteMetric }
}) | ConvertTo-Json -Compress
"#;
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout); let t = t.trim();
            let arr_t = if t.starts_with('{') { format!("[{}]",t) } else { t.to_string() };
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&arr_t) {
                return arr.iter().map(|r| {
                    let net = r["net"].as_str().unwrap_or("").to_string();
                    let (network, netmask) = if net.contains('/') {
                        let p: Vec<&str> = net.splitn(2,'/').collect();
                        (p[0].to_string(), p.get(1).unwrap_or(&"").to_string())
                    } else { (net, String::new()) };
                    RouteEntry { network, netmask, gateway: r["gw"].as_str().unwrap_or("").to_string(), interface: r["iface"].as_str().unwrap_or("").to_string(), metric: r["metric"].as_u64().unwrap_or(0) as u32 }
                }).collect();
            }
        }
    }
    vec![]
}

// ─── Scan de ports ─────────────────────────────────────────────────────────────
#[tauri::command]
pub fn scan_ports(host: String, ports: Vec<u16>) -> Vec<PortScanResult> {
    let h = host.replace(['\'', '"'], "");
    let ports_limited: Vec<u16> = ports.into_iter().take(100).collect();
    let ports_str: Vec<String> = ports_limited.iter().map(|p| p.to_string()).collect();
    let ports_joined = ports_str.join(",");
    let ps = format!(r#"
$results = @()
$ports = @({ports})
foreach ($p in $ports) {{
    $tcp = $null
    try {{
        $tcp = New-Object System.Net.Sockets.TcpClient
        $conn = $tcp.BeginConnect('{host}', $p, $null, $null)
        $wait = $conn.AsyncWaitHandle.WaitOne(500, $false)
        if ($wait -and -not $tcp.Client.Connected) {{ $wait = $false }}
        if ($wait) {{ try {{ $tcp.EndConnect($conn) }} catch {{ $wait = $false }} }}
    }} catch {{ $wait = $false }}
    finally {{ if ($tcp) {{ $tcp.Close() }} }}
    $results += @{{ port=$p; open=$wait }}
}}
$results | ConvertTo-Json -Compress
"#, host=h, ports=ports_joined);
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",&ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout); let t = t.trim();
            let arr_t = if t.starts_with('{') { format!("[{}]",t) } else { t.to_string() };
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&arr_t) {
                return arr.iter().map(|r| {
                    let port = r["port"].as_u64().unwrap_or(0) as u16;
                    let open = r["open"].as_bool().unwrap_or(false);
                    let service = known_service(port);
                    PortScanResult { port, open, service }
                }).collect();
            }
        }
    }
    vec![]
}

fn known_service(port: u16) -> String {
    match port {
        21=>"FTP",22=>"SSH",23=>"Telnet",25=>"SMTP",53=>"DNS",80=>"HTTP",110=>"POP3",
        143=>"IMAP",443=>"HTTPS",445=>"SMB",3306=>"MySQL",3389=>"RDP",5432=>"PostgreSQL",
        5900=>"VNC",6379=>"Redis",8080=>"HTTP-Alt",8443=>"HTTPS-Alt",27017=>"MongoDB",
        135=>"RPC",139=>"NetBIOS",389=>"LDAP",636=>"LDAPS",993=>"IMAPS",995=>"POP3S",
        1433=>"MSSQL",1521=>"Oracle",5985=>"WinRM",5986=>"WinRM-S",_=>"",
    }.to_string()
}

// ─── Réseaux WiFi proches ──────────────────────────────────────────────────────
#[tauri::command]
pub fn get_wifi_networks() -> Vec<WifiNetwork> {
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("netsh").args(["wlan","show","networks","mode=bssid"]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let text = String::from_utf8_lossy(&o.stdout);
            let mut networks: Vec<WifiNetwork> = Vec::new();
            let mut current = WifiNetwork::default();
            for line in text.lines() {
                let line = line.trim();
                if line.starts_with("SSID") && !line.starts_with("BSSID") {
                    if !current.ssid.is_empty() { networks.push(current.clone()); }
                    current = WifiNetwork::default();
                    current.ssid = line.split_once(':').map(|x| x.1).unwrap_or("").trim().to_string();
                } else if line.starts_with("BSSID") {
                    current.bssid = line.split_once(':').map(|x| x.1).unwrap_or("").trim().to_string();
                } else if line.to_lowercase().starts_with("signal") {
                    let v = line.split_once(':').map(|x| x.1).unwrap_or("0%").trim().trim_end_matches('%').parse::<u32>().unwrap_or(0);
                    current.signal_percent = v;
                } else if line.to_lowercase().starts_with("channel") {
                    current.channel = line.split_once(':').map(|x| x.1).unwrap_or("").trim().to_string();
                } else if line.to_lowercase().contains("authentication") {
                    current.auth = line.split_once(':').map(|x| x.1).unwrap_or("").trim().to_string();
                } else if line.to_lowercase().contains("cipher") {
                    current.encryption = line.split_once(':').map(|x| x.1).unwrap_or("").trim().to_string();
                } else if line.to_lowercase().contains("radio type") {
                    current.band = line.split_once(':').map(|x| x.1).unwrap_or("").trim().to_string();
                }
            }
            if !current.ssid.is_empty() { networks.push(current); }
            networks.sort_by_key(|a| std::cmp::Reverse(a.signal_percent));
            return networks;
        }
    }
    vec![]
}

// ─── Ports ouverts locaux (netstat) ───────────────────────────────────────────
#[tauri::command]
pub fn get_local_open_ports() -> Vec<OpenPort> {
    let ps = r#"
$procs = @{}; Get-Process | ForEach-Object { $procs[[string]$_.Id] = $_.ProcessName }
@(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | ForEach-Object {
    $pid_s = [string]$_.OwningProcess
    @{ proto='TCP'; local=[string]$_.LocalAddress; port=[int]$_.LocalPort; remote=[string]$_.RemoteAddress; state=[string]$_.State; pid=$_.OwningProcess; proc=if($procs[$pid_s]){$procs[$pid_s]}else{''} }
}) | ConvertTo-Json -Compress
"#;
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout); let t = t.trim();
            let arr_t = if t.starts_with('{') { format!("[{}]",t) } else { t.to_string() };
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&arr_t) {
                return arr.iter().map(|r| OpenPort {
                    protocol: r["proto"].as_str().unwrap_or("TCP").to_string(),
                    local_address: r["local"].as_str().unwrap_or("").to_string(),
                    local_port: r["port"].as_u64().unwrap_or(0) as u16,
                    remote_address: r["remote"].as_str().unwrap_or("").to_string(),
                    state: r["state"].as_str().unwrap_or("").to_string(),
                    pid: r["pid"].as_u64().unwrap_or(0) as u32,
                    process: r["proc"].as_str().unwrap_or("").to_string(),
                }).collect();
            }
        }
    }
    vec![]
}

// ─── Vérification HTTP/HTTPS ───────────────────────────────────────────────────
#[tauri::command]
pub fn check_http(url: String) -> HttpCheckResult {
    let url_clean = url.replace(['\'', '"'], "");
    let ps = format!(r#"
try {{
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $req = [System.Net.HttpWebRequest]::Create('{url}')
    $req.Method = 'HEAD'; $req.Timeout = 8000; $req.AllowAutoRedirect = $true
    $resp = $req.GetResponse()
    $sw.Stop()
    $headers = @($resp.Headers.AllKeys | ForEach-Object {{ "$_: $($resp.Headers[$_])" }})
    @{{ ok=$true; code=[int]$resp.StatusCode; text=[string]$resp.StatusDescription; ms=$sw.ElapsedMilliseconds; hdrs=$headers }} | ConvertTo-Json -Compress
    $resp.Close()
}} catch [System.Net.WebException] {{
    $c = 0; $t = $_.Exception.Message
    if ($_.Exception.Response) {{ $c = [int]$_.Exception.Response.StatusCode; $t = [string]$_.Exception.Response.StatusDescription }}
    @{{ ok=$false; code=$c; text=$t; ms=0; hdrs=@() }} | ConvertTo-Json -Compress
}} catch {{
    @{{ ok=$false; code=0; text=$_.Exception.Message; ms=0; hdrs=@() }} | ConvertTo-Json -Compress
}}"#, url=url_clean);
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",&ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(t.trim()) {
                return HttpCheckResult {
                    url: url_clean,
                    status_code: v["code"].as_u64().unwrap_or(0) as u32,
                    status_text: v["text"].as_str().unwrap_or("").to_string(),
                    headers: v["hdrs"].as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(|x| x.to_string())).collect()).unwrap_or_default(),
                    time_ms: v["ms"].as_u64().unwrap_or(0),
                    success: v["ok"].as_bool().unwrap_or(false),
                };
            }
        }
    }
    HttpCheckResult { url: url_clean, ..Default::default() }
}

// ─── Partages réseau ───────────────────────────────────────────────────────────
#[tauri::command]
pub fn get_net_shares(host: String) -> Vec<NetShareEntry> {
    let h = host.replace(['\'', '"'], "");
    let ps = format!(r#"
try {{
    $shares = @(Get-WmiObject -ComputerName '{host}' Win32_Share -EA SilentlyContinue |
        ForEach-Object {{ @{{ name=[string]$_.Name; path=[string]$_.Path; comment=[string]$_.Description; host='{host}' }} }})
    $shares | ConvertTo-Json -Compress
}} catch {{ '[]' }}"#, host=h);
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",&ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout); let t = t.trim();
            let arr_t = if t.starts_with('{') { format!("[{}]",t) } else { t.to_string() };
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&arr_t) {
                return arr.iter().map(|r| NetShareEntry {
                    name: r["name"].as_str().unwrap_or("").to_string(),
                    path: r["path"].as_str().unwrap_or("").to_string(),
                    comment: r["comment"].as_str().unwrap_or("").to_string(),
                    host: h.clone(),
                }).collect();
            }
        }
    }
    vec![]
}

// ─── Test vitesse simple ───────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Default)]
pub struct BandwidthResult { pub download_mbps: f64, pub latency_ms: f64, pub test_host: String, pub success: bool }

#[tauri::command]
pub fn test_bandwidth() -> BandwidthResult {
    // Simple: download 10MB from Cloudflare speed test server
    let ps = r#"
try {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $ping = [math]::Round((Test-Connection '1.1.1.1' -Count 2 -ErrorAction SilentlyContinue | Measure-Object ResponseTime -Average).Average, 1)
    $sw2 = [System.Diagnostics.Stopwatch]::StartNew()
    $wc = New-Object System.Net.WebClient
    $data = $wc.DownloadData('https://speed.cloudflare.com/__down?bytes=5000000')
    $sw2.Stop()
    $mb = $data.Length / 1048576
    $secs = $sw2.Elapsed.TotalSeconds
    $mbps = [math]::Round($mb / $secs * 8, 2)
    @{ ok=$true; mbps=$mbps; lat=$ping; host='speed.cloudflare.com' } | ConvertTo-Json -Compress
} catch {
    @{ ok=$false; mbps=0; lat=0; host='speed.cloudflare.com' } | ConvertTo-Json -Compress
}"#;
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(t.trim()) {
                return BandwidthResult { download_mbps: v["mbps"].as_f64().unwrap_or(0.0), latency_ms: v["lat"].as_f64().unwrap_or(0.0), test_host: v["host"].as_str().unwrap_or("").to_string(), success: v["ok"].as_bool().unwrap_or(false) };
            }
        }
    }
    BandwidthResult::default()
}
