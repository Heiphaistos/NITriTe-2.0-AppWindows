use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Default)]
pub struct FirewallRule {
    pub name: String,
    pub direction: String,
    pub action: String,
    pub enabled: bool,
    pub profile: String,
    pub protocol: String,
    pub local_port: String,
    pub program: String,
    pub group: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FirewallProfile {
    pub name: String,
    pub enabled: bool,
    pub default_inbound: String,
    pub default_outbound: String,
    pub log_file: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FirewallInfo {
    pub profiles: Vec<FirewallProfile>,
    pub rules: Vec<FirewallRule>,
    pub inbound_allow: u32,
    pub inbound_block: u32,
    pub outbound_allow: u32,
    pub outbound_block: u32,
    pub total_custom: u32,
}

// Anti-freeze : PowerShell est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn get_firewall_rules() -> FirewallInfo {
    tokio::task::spawn_blocking(get_firewall_rules_blocking)
        .await
        .unwrap_or_default()
}

fn get_firewall_rules_blocking() -> FirewallInfo {
    let ps = r#"
$out = @{}

# Profils
$out.Profiles = @(Get-NetFirewallProfile -ErrorAction SilentlyContinue | ForEach-Object {
    @{
        name=$_.Name; enabled=[bool]$_.Enabled;
        defIn=[string]$_.DefaultInboundAction;
        defOut=[string]$_.DefaultOutboundAction;
        logFile=[string]$_.LogFileName
    }
})

# Règles actives personnalisées (hors Windows/Microsoft)
$allRules = Get-NetFirewallRule -Enabled True -ErrorAction SilentlyContinue

$customRules = $allRules | Where-Object {
    $_.Group -eq '' -or ($_.Group -notmatch 'Windows|Microsoft|@')
} | Select-Object -First 100

$out.Rules = @($customRules | ForEach-Object {
    $r = $_
    $portFilter = $r | Get-NetFirewallPortFilter -ErrorAction SilentlyContinue
    $appFilter  = $r | Get-NetFirewallApplicationFilter -ErrorAction SilentlyContinue
    @{
        name=[string]$r.DisplayName; direction=[string]$r.Direction;
        action=[string]$r.Action; enabled=[bool]$r.Enabled;
        profile=[string]$r.Profile; protocol=[string]$portFilter.Protocol;
        localPort=[string]$portFilter.LocalPort;
        program=[string]$appFilter.Program; group=[string]$r.Group
    }
})

$out.InboundAllow  = [int]($allRules | Where-Object { $_.Direction -eq 'Inbound'  -and $_.Action -eq 'Allow' } | Measure-Object).Count
$out.InboundBlock  = [int]($allRules | Where-Object { $_.Direction -eq 'Inbound'  -and $_.Action -eq 'Block' } | Measure-Object).Count
$out.OutboundAllow = [int]($allRules | Where-Object { $_.Direction -eq 'Outbound' -and $_.Action -eq 'Allow' } | Measure-Object).Count
$out.OutboundBlock = [int]($allRules | Where-Object { $_.Direction -eq 'Outbound' -and $_.Action -eq 'Block' } | Measure-Object).Count
$out.TotalCustom   = [int]($customRules | Measure-Object).Count

$out | ConvertTo-Json -Depth 4 -Compress
"#;

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output();

        if let Ok(o) = output {
            // decode_output : DisplayName des règles intégrées est localisé FR
            // accentué (ex: "requête ICMP Echo") sans $OutputEncoding préalable.
            let text = crate::maintenance::commands::decode_output(&o.stdout);
            let v: serde_json::Value = match serde_json::from_str(text.trim()) {
                Ok(val) => val, Err(_) => return FirewallInfo::default(),
            };

            let profiles: Vec<FirewallProfile> = v["Profiles"].as_array().map(|a| a.iter().map(|p| FirewallProfile {
                name: p["name"].as_str().unwrap_or("").to_string(),
                enabled: p["enabled"].as_bool().unwrap_or(false),
                default_inbound: p["defIn"].as_str().unwrap_or("").to_string(),
                default_outbound: p["defOut"].as_str().unwrap_or("").to_string(),
                log_file: p["logFile"].as_str().unwrap_or("").to_string(),
            }).collect()).unwrap_or_default();

            let rules: Vec<FirewallRule> = v["Rules"].as_array().map(|a| a.iter().map(|r| FirewallRule {
                name: r["name"].as_str().unwrap_or("").to_string(),
                direction: r["direction"].as_str().unwrap_or("").to_string(),
                action: r["action"].as_str().unwrap_or("").to_string(),
                enabled: r["enabled"].as_bool().unwrap_or(false),
                profile: r["profile"].as_str().unwrap_or("").to_string(),
                protocol: r["protocol"].as_str().unwrap_or("").to_string(),
                local_port: r["localPort"].as_str().unwrap_or("").to_string(),
                program: r["program"].as_str().unwrap_or("").to_string(),
                group: r["group"].as_str().unwrap_or("").to_string(),
            }).collect()).unwrap_or_default();

            return FirewallInfo {
                profiles,
                rules,
                inbound_allow: v["InboundAllow"].as_u64().unwrap_or(0) as u32,
                inbound_block: v["InboundBlock"].as_u64().unwrap_or(0) as u32,
                outbound_allow: v["OutboundAllow"].as_u64().unwrap_or(0) as u32,
                outbound_block: v["OutboundBlock"].as_u64().unwrap_or(0) as u32,
                total_custom: v["TotalCustom"].as_u64().unwrap_or(0) as u32,
            };
        }
    }
    FirewallInfo::default()
}
