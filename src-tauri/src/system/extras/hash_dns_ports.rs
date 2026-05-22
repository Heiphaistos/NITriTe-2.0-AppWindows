use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use super::{parse_json_arr, ps};

// ─── Hash Fichier ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HashResult {
    pub path: String,
    pub algorithm: String,
    pub hash: String,
    pub size_bytes: u64,
}

#[tauri::command]
pub fn hash_file(path: String, algorithm: String) -> Result<HashResult, String> {
    let algo = match algorithm.to_uppercase().as_str() {
        "MD5" => "MD5",
        "SHA1" => "SHA1",
        "SHA256" => "SHA256",
        _ => "SHA256",
    };
    let script = format!(
        "Get-FileHash -Path '{}' -Algorithm {} | Select-Object -ExpandProperty Hash",
        path.replace('\'', "''"), algo
    );
    let hash = ps(&script)?;
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    Ok(HashResult { path, algorithm: algo.to_string(), hash: hash.trim().to_string(), size_bytes: size })
}

#[tauri::command]
pub fn hash_folder(path: String, algorithm: String, max_files: u32) -> Result<Vec<HashResult>, String> {
    let algo = match algorithm.to_uppercase().as_str() {
        "MD5"    => "MD5",
        "SHA1"   => "SHA1",
        "SHA256" => "SHA256",
        _        => "SHA256",
    };
    let n = max_files.min(500);
    let script = format!(r#"
Get-ChildItem -Path '{path}' -File -ErrorAction SilentlyContinue | Select-Object -First {n} | ForEach-Object {{
    $h = (Get-FileHash $_.FullName -Algorithm {algo} -ErrorAction SilentlyContinue).Hash
    if ($h) {{ [PSCustomObject]@{{ path=$_.FullName; size=$_.Length; hash=$h }} }}
}} | ConvertTo-Json -Compress"#,
        path = path.replace('\'', "''"), n = n, algo = algo);
    let out = ps(&script)?;
    if out.is_empty() { return Ok(vec![]); }
    let arr: Vec<serde_json::Value> = parse_json_arr(&out);
    Ok(arr.iter().filter_map(|v| {
        let hash = v["hash"].as_str().unwrap_or("").to_string();
        if hash.is_empty() { return None; }
        Some(HashResult {
            path:       v["path"].as_str().unwrap_or("").to_string(),
            algorithm:  algo.to_string(),
            hash,
            size_bytes: v["size"].as_u64().unwrap_or(0),
        })
    }).collect())
}

// ─── DNS Switcher ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct DnsPreset {
    pub id: String,
    pub name: String,
    pub primary: String,
    pub secondary: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct NetworkAdapterInfo {
    pub name: String,
    pub description: String,
    pub status: String,
    pub current_dns: Vec<String>,
}

#[tauri::command]
pub fn get_dns_presets() -> Vec<DnsPreset> {
    vec![
        DnsPreset { id: "google".into(),      name: "Google".into(),            primary: "8.8.8.8".into(),         secondary: "8.8.4.4".into(),        description: "Rapide, fiable, mondial".into() },
        DnsPreset { id: "cloudflare".into(),   name: "Cloudflare".into(),        primary: "1.1.1.1".into(),         secondary: "1.0.0.1".into(),        description: "Le plus rapide, confidentialité".into() },
        DnsPreset { id: "cloudflare-doh".into(),name: "Cloudflare Family".into(),primary: "1.1.1.3".into(),         secondary: "1.0.0.3".into(),        description: "Filtrage malware + adult".into() },
        DnsPreset { id: "opendns".into(),      name: "OpenDNS".into(),           primary: "208.67.222.222".into(),  secondary: "208.67.220.220".into(),  description: "Filtrage contenus, Cisco".into() },
        DnsPreset { id: "nextdns".into(),      name: "NextDNS".into(),           primary: "45.90.28.0".into(),      secondary: "45.90.30.0".into(),      description: "DNS personnalisable avec dashboard".into() },
        DnsPreset { id: "quad9".into(),        name: "Quad9".into(),             primary: "9.9.9.9".into(),         secondary: "149.112.112.112".into(), description: "Sécurisé, bloque malwares".into() },
        DnsPreset { id: "adguard".into(),      name: "AdGuard".into(),           primary: "94.140.14.14".into(),    secondary: "94.140.15.15".into(),    description: "Bloque pubs et trackers".into() },
        DnsPreset { id: "auto".into(),         name: "Automatique (DHCP)".into(),primary: "".into(),                secondary: "".into(),               description: "Remettre le DNS automatique".into() },
    ]
}

#[tauri::command]
pub fn get_network_adapters_for_dns() -> Result<Vec<NetworkAdapterInfo>, String> {
    let script = r#"
Get-NetAdapter | Where-Object {$_.Status -eq 'Up'} | ForEach-Object {
    $dns = (Get-DnsClientServerAddress -InterfaceAlias $_.Name -AddressFamily IPv4 -ErrorAction SilentlyContinue).ServerAddresses
    [PSCustomObject]@{
        name = $_.Name
        description = $_.InterfaceDescription
        status = $_.Status
        dns = if($dns){$dns -join ','}else{''}
    }
} | ConvertTo-Json -Compress
"#;
    let out = ps(script)?;
    if out.is_empty() { return Ok(vec![]); }
    let arr: Vec<serde_json::Value> = if out.starts_with('[') {
        serde_json::from_str(&out).unwrap_or_default()
    } else {
        serde_json::from_str(&format!("[{}]", out)).unwrap_or_default()
    };
    Ok(arr.iter().map(|v| NetworkAdapterInfo {
        name: v["name"].as_str().unwrap_or("").to_string(),
        description: v["description"].as_str().unwrap_or("").to_string(),
        status: v["status"].as_str().unwrap_or("").to_string(),
        current_dns: v["dns"].as_str().unwrap_or("").split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect(),
    }).collect())
}

#[tauri::command]
pub fn switch_dns(adapter: String, primary: String, secondary: String) -> Result<String, String> {
    let script = if primary.is_empty() {
        format!("Set-DnsClientServerAddress -InterfaceAlias '{}' -ResetServerAddresses; 'OK'", adapter)
    } else if secondary.is_empty() {
        format!("Set-DnsClientServerAddress -InterfaceAlias '{}' -ServerAddresses '{}'; 'OK'", adapter, primary)
    } else {
        format!("Set-DnsClientServerAddress -InterfaceAlias '{}' -ServerAddresses '{}','{}'; 'OK'", adapter, primary, secondary)
    };
    ps(&script)
}

#[tauri::command]
pub fn flush_dns_cache() -> Result<String, String> {
    ps("Clear-DnsClientCache; 'DNS cache vidé avec succès'")
}

#[tauri::command]
pub fn ping_dns(ip: String) -> Result<i32, String> {
    let script = format!(
        "try {{ $r = Test-Connection -ComputerName '{}' -Count 2 -ErrorAction Stop; [int]($r | Measure-Object -Property ResponseTime -Average).Average }} catch {{ -1 }}",
        ip.replace('\'', "''")
    );
    let out = ps(&script)?;
    Ok(out.trim().parse::<i32>().unwrap_or(-1))
}

// ─── Port Scanner Local ────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct OpenPort {
    pub protocol: String,
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: String,
    pub state: String,
    pub pid: u32,
    pub process_name: String,
}

#[tauri::command]
pub fn get_local_ports() -> Result<Vec<OpenPort>, String> {
    let script = r#"
$procs = @{}
Get-Process | ForEach-Object { $procs[$_.Id] = $_.ProcessName }
$conns = @()
netstat -ano | Select-String -Pattern '^\s+(TCP|UDP)\s+(\S+)\s+(\S+)\s+(\w+|\s+)\s+(\d+)' | ForEach-Object {
    $m = $_.Matches[0]
    if ($m) {
        $parts = $_.Line.Trim() -split '\s+'
        if ($parts.Count -ge 4) {
            $proto = $parts[0]
            $local = $parts[1]
            $remote = $parts[2]
            $stateOrPid = $parts[3]
            $pidNum = if($proto -eq 'TCP' -and $parts.Count -ge 5){[int]$parts[4]}else{[int]$stateOrPid}
            $state = if($proto -eq 'TCP' -and $parts.Count -ge 5){$stateOrPid}else{''}
            $lp = if($local -match ':(\d+)$'){[int]$Matches[1]}else{0}
            $conns += [PSCustomObject]@{
                protocol=$proto; local=$local; remote=$remote
                state=$state; pid=$pidNum; port=$lp
                proc=if($procs.ContainsKey($pidNum)){$procs[$pidNum]}else{''}
            }
        }
    }
}
$conns | ConvertTo-Json -Compress
"#;
    let out = ps(script)?;
    if out.is_empty() { return Ok(vec![]); }
    let arr: Vec<serde_json::Value> = parse_json_arr(&out);
    let mut ports: Vec<OpenPort> = arr.iter().filter_map(|v| {
        let port = v["port"].as_u64().unwrap_or(0) as u16;
        if port == 0 { return None; }
        Some(OpenPort {
            protocol: v["protocol"].as_str().unwrap_or("").to_string(),
            local_address: v["local"].as_str().unwrap_or("").to_string(),
            local_port: port,
            remote_address: v["remote"].as_str().unwrap_or("").to_string(),
            state: v["state"].as_str().unwrap_or("").to_string(),
            pid: v["pid"].as_u64().unwrap_or(0) as u32,
            process_name: v["proc"].as_str().unwrap_or("").to_string(),
        })
    }).collect();
    ports.sort_by_key(|p| p.local_port);
    ports.dedup_by_key(|p| (p.protocol.clone(), p.local_port));
    Ok(ports)
}
