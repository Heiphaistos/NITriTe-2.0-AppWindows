use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, Clone)]
pub struct TcpConnection {
    pub protocol: String,
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: String,
    pub remote_port: u16,
    pub state: String,
    pub pid: u32,
    pub process_name: String,
    pub owning_module: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct WifiInfo {
    pub ssid: String,
    pub bssid: String,
    pub signal_percent: u32,
    pub band: String,
    pub channel: u32,
    pub security: String,
    pub receive_rate_mbps: f64,
    pub transmit_rate_mbps: f64,
    pub state: String,
    pub adapter_name: String,
    pub authentication: String,
    pub protocol: String,
}

#[cfg(target_os = "windows")]
fn run_ps(script: &str) -> String {
    // .output() n'avait aucun timeout — get_active_connections est mis en cache
    // 20s côté frontend (dataCache.ts) donc rappelé périodiquement tant que
    // l'onglet Connexions du Diagnostic reste consulté ; même famille de bug
    // que wmi_timeout/check_command/ps()/monitor.rs/perf_snapshot.rs (cycles
    // 153-157).
    match crate::maintenance::commands::execute_system_command(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", script],
        5,
    ) {
        Ok(o) => o.stdout.trim().to_string(),
        Err(_) => String::new(),
    }
}

pub fn collect_connections() -> Vec<TcpConnection> {
    #[cfg(target_os = "windows")]
    {
        // Get process names by PID for mapping
        let pids_ps = r#"
try {
    $procs = Get-Process | Select-Object Id, ProcessName
    $procs | ForEach-Object { "$($_.Id)=$($_.ProcessName)" }
} catch {}
"#;
        let pid_raw = run_ps(pids_ps);
        let mut pid_map: HashMap<u32, String> = HashMap::new();
        for line in pid_raw.lines() {
            if let Some((pid_str, name)) = line.split_once('=') {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    pid_map.insert(pid, name.trim().to_string());
                }
            }
        }

        let ps = r#"
try {
    $tcp = Get-NetTCPConnection -ErrorAction Stop
    $udp = Get-NetUDPEndpoint -ErrorAction SilentlyContinue
    $result = @()
    $result += $tcp | ForEach-Object {
        [PSCustomObject]@{
            Proto = "TCP"
            LocalAddr = $_.LocalAddress
            LocalPort = $_.LocalPort
            RemoteAddr = $_.RemoteAddress
            RemotePort = $_.RemotePort
            State = $_.State.ToString()
            Pid = $_.OwningProcess
        }
    }
    $result += $udp | Where-Object { $_.LocalPort -lt 65535 } | ForEach-Object {
        [PSCustomObject]@{
            Proto = "UDP"
            LocalAddr = $_.LocalAddress
            LocalPort = $_.LocalPort
            RemoteAddr = ""
            RemotePort = 0
            State = "Listen"
            Pid = $_.OwningProcess
        }
    }
    $result | ConvertTo-Json -Compress
} catch { "[]" }
"#;
        let raw = run_ps(ps);
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed == "[]" { return vec![]; }

        let arr: Vec<serde_json::Value> = serde_json::from_str(trimmed)
            .unwrap_or_else(|_| serde_json::from_str(&format!("[{}]", trimmed)).unwrap_or_default());

        let mut list: Vec<TcpConnection> = arr.iter().filter_map(|v| {
            let pid = v["Pid"].as_u64().unwrap_or(0) as u32;
            Some(TcpConnection {
                protocol: v["Proto"].as_str()?.to_string(),
                local_address: v["LocalAddr"].as_str().unwrap_or("").to_string(),
                local_port: v["LocalPort"].as_u64().unwrap_or(0) as u16,
                remote_address: v["RemoteAddr"].as_str().unwrap_or("").to_string(),
                remote_port: v["RemotePort"].as_u64().unwrap_or(0) as u16,
                state: v["State"].as_str().unwrap_or("Unknown").to_string(),
                pid,
                process_name: pid_map.get(&pid).cloned().unwrap_or_default(),
                owning_module: String::new(),
            })
        }).collect();
        list.sort_by(|a, b| a.state.cmp(&b.state).then(a.process_name.cmp(&b.process_name)));
        list
    }
    #[cfg(not(target_os = "windows"))]
    vec![]
}

pub fn collect_wifi_info() -> Option<WifiInfo> {
    #[cfg(target_os = "windows")]
    {
        // .output() n'avait aucun timeout ; netsh écrit en codepage OEM —
        // execute_system_command applique déjà decode_output (UTF-8 first, repli
        // OEM) en interne, évitant le mojibake des libellés/valeurs FR.
        let out = crate::maintenance::commands::execute_system_command(
            "netsh",
            &["wlan", "show", "interfaces"],
            5,
        ).ok()?;
        let text = out.stdout;
        let mut wifi = WifiInfo {
            ssid: String::new(), bssid: String::new(),
            signal_percent: 0, band: String::new(), channel: 0,
            security: String::new(), receive_rate_mbps: 0.0,
            transmit_rate_mbps: 0.0, state: String::new(),
            adapter_name: String::new(), authentication: String::new(),
            protocol: String::new(),
        };
        // Parsing locale-indépendant : les libellés netsh sont traduits ET alignés
        // différemment (« État », « Authentification », « Canal »…). On coupe sur le
        // premier ':', on normalise le libellé (minuscule, accents retirés) et on
        // matche des mots-clés EN + FR — au lieu de préfixes anglais à espaces fixes
        // qui rendaient toutes les infos WiFi vides sur Windows FR.
        let strip_accents = |s: &str| s.chars().map(|c| match c {
            'é'|'è'|'ê'|'ë' => 'e', 'à'|'â'|'ä' => 'a', 'î'|'ï' => 'i',
            'ô'|'ö' => 'o', 'û'|'ü'|'ù' => 'u', 'ç' => 'c', _ => c,
        }).collect::<String>();
        for line in text.lines() {
            let Some((label, val)) = line.split_once(':') else { continue };
            let label = strip_accents(&label.trim().to_lowercase());
            let val = val.trim();
            if val.is_empty() { continue; }
            let l = label.as_str();
            if l == "ssid" { wifi.ssid = val.to_string(); }
            else if l == "bssid" { wifi.bssid = val.to_string(); }
            else if l == "signal" { wifi.signal_percent = val.trim_end_matches('%').trim().parse().unwrap_or(0); }
            else if l.contains("radio") { wifi.protocol = val.to_string(); }
            else if l == "channel" || l.contains("canal") { wifi.channel = val.parse().unwrap_or(0); }
            else if l.contains("authentic") || l.contains("authentif") { wifi.authentication = val.to_string(); }
            else if l.contains("cipher") || l.contains("chiffrement") { wifi.security = val.to_string(); }
            else if l.contains("receive") || l.contains("reception") { wifi.receive_rate_mbps = val.parse().unwrap_or(0.0); }
            else if l.contains("transmit") || l.contains("transmission") { wifi.transmit_rate_mbps = val.parse().unwrap_or(0.0); }
            else if l == "state" || l.contains("etat") { wifi.state = val.to_string(); }
            else if l == "name" || l == "nom" { wifi.adapter_name = val.to_string(); }
        }
        if wifi.channel > 0 {
            wifi.band = if wifi.channel <= 14 { "2.4 GHz".to_string() }
                        else if wifi.channel <= 196 { "5 GHz".to_string() }
                        else { "6 GHz".to_string() };
        }
        if wifi.ssid.is_empty() && wifi.state.is_empty() { return None; }
        Some(wifi)
    }
    #[cfg(not(target_os = "windows"))]
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests d'intégration en conditions réelles sur cette machine — vérifient que
    // le refactor .output() → execute_system_command (ajout du timeout, cycle
    // 157) n'a pas cassé le chemin heureux. Le mécanisme timeout+kill lui-même
    // est déjà prouvé par le test dédié d'execute_system_command (cycle 149).

    #[test]
    fn run_ps_does_not_error_on_quick_script() {
        assert_eq!(run_ps("Write-Output 'ok'"), "ok");
    }

    #[test]
    fn collect_connections_does_not_panic() {
        // Pas d'assertion de contenu (dépend des connexions actives au moment du
        // test), juste l'absence de panique après le refactor.
        let _ = collect_connections();
    }
}
