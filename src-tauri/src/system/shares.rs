use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Default)]
pub struct NetworkShare {
    pub name: String,
    pub path: String,
    pub description: String,
    pub share_type: String,
    pub current_uses: u32,
    pub max_uses: i32,
    pub is_hidden: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct MappedDrive {
    pub drive_letter: String,
    pub remote_path: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SmbSession {
    pub client_name: String,
    pub client_ip: String,
    pub user: String,
    pub idle_time: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SharesInfo {
    pub shares: Vec<NetworkShare>,
    pub admin_shares: Vec<NetworkShare>,
    pub mapped_drives: Vec<MappedDrive>,
    pub smb_sessions: Vec<SmbSession>,
    pub open_files: u32,
    pub computer_name: String,
    pub workgroup: String,
}

// Anti-freeze : PowerShell est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn get_network_shares() -> SharesInfo {
    tokio::task::spawn_blocking(get_network_shares_blocking)
        .await
        .unwrap_or_default()
}

fn get_network_shares_blocking() -> SharesInfo {
    let ps = r#"
$out = @{}

# Partages
$allShares = Get-SmbShare -ErrorAction SilentlyContinue
$out.Shares = @($allShares | Where-Object { $_.Name -notmatch '^\w+\$$' -and $_.Name -ne 'IPC$' } | ForEach-Object {
    @{name=$_.Name; path=$_.Path; description=[string]$_.Description;
      shareType=[string]$_.ShareType; currentUses=[int]$_.CurrentUses;
      maxUses=if($_.MaximumAllowed -eq $null){-1}else{[int]$_.MaximumAllowed};
      isHidden=$false}
})
$out.AdminShares = @($allShares | Where-Object { $_.Name -match '^\w+\$$' -or $_.Name -eq 'IPC$' } | ForEach-Object {
    @{name=$_.Name; path=$_.Path; description=[string]$_.Description;
      shareType=[string]$_.ShareType; currentUses=[int]$_.CurrentUses;
      maxUses=-1; isHidden=$true}
})

# Lecteurs réseau mappés
try {
    $out.MappedDrives = @(Get-WmiObject -Class Win32_MappedLogicalDisk -ErrorAction SilentlyContinue | ForEach-Object {
        @{driveLetter=$_.DeviceID; remotePath=$_.ProviderName; status=[string]$_.Status}
    })
} catch { $out.MappedDrives = @() }

# Sessions SMB actives
try {
    $out.Sessions = @(Get-SmbSession -ErrorAction SilentlyContinue | Select-Object -First 20 | ForEach-Object {
        # Get-SmbSession n'expose pas d'IP séparée : ClientComputerName contient déjà
        # l'IP/nom du client. clientIp pointait par erreur vers ClientUserName
        # (copie du champ user) et affichait un nom d'utilisateur a la place d'une IP.
        @{clientName=[string]$_.ClientComputerName; clientIp=[string]$_.ClientComputerName;
          user=[string]$_.ClientUserName; idleTime=[string]$_.SecondsIdle}
    })
} catch { $out.Sessions = @() }

# Fichiers ouverts
try {
    $out.OpenFiles = [int](Get-SmbOpenFile -ErrorAction SilentlyContinue | Measure-Object).Count
} catch { $out.OpenFiles = 0 }

# Identité réseau
try {
    $cs = Get-WmiObject Win32_ComputerSystem -ErrorAction SilentlyContinue
    $out.ComputerName = [string]$env:COMPUTERNAME
    $out.Workgroup = [string]$cs.Workgroup
} catch { $out.ComputerName = $env:COMPUTERNAME; $out.Workgroup = "" }

$out | ConvertTo-Json -Depth 3 -Compress
"#;

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output();

        if let Ok(o) = output {
            // decode_output : Description de partage est un champ libre admin,
            // souvent accentué FR ("Partage des documents").
            let text = crate::maintenance::commands::decode_output(&o.stdout);
            let v: serde_json::Value = match serde_json::from_str(text.trim()) {
                Ok(val) => val, Err(_) => return SharesInfo::default(),
            };

            let parse_shares = |key: &str| -> Vec<NetworkShare> {
                v[key].as_array().map(|a| a.iter().map(|s| NetworkShare {
                    name: s["name"].as_str().unwrap_or("").to_string(),
                    path: s["path"].as_str().unwrap_or("").to_string(),
                    description: s["description"].as_str().unwrap_or("").to_string(),
                    share_type: s["shareType"].as_str().unwrap_or("").to_string(),
                    current_uses: s["currentUses"].as_u64().unwrap_or(0) as u32,
                    max_uses: s["maxUses"].as_i64().unwrap_or(-1) as i32,
                    is_hidden: s["isHidden"].as_bool().unwrap_or(false),
                }).collect()).unwrap_or_default()
            };

            let mapped_drives = v["MappedDrives"].as_array().map(|a| a.iter().map(|d| MappedDrive {
                drive_letter: d["driveLetter"].as_str().unwrap_or("").to_string(),
                remote_path: d["remotePath"].as_str().unwrap_or("").to_string(),
                status: d["status"].as_str().unwrap_or("").to_string(),
            }).collect()).unwrap_or_default();

            let smb_sessions = v["Sessions"].as_array().map(|a| a.iter().map(|s| SmbSession {
                client_name: s["clientName"].as_str().unwrap_or("").to_string(),
                client_ip: s["clientIp"].as_str().unwrap_or("").to_string(),
                user: s["user"].as_str().unwrap_or("").to_string(),
                idle_time: s["idleTime"].as_str().unwrap_or("").to_string(),
            }).collect()).unwrap_or_default();

            return SharesInfo {
                shares: parse_shares("Shares"),
                admin_shares: parse_shares("AdminShares"),
                mapped_drives,
                smb_sessions,
                open_files: v["OpenFiles"].as_u64().unwrap_or(0) as u32,
                computer_name: v["ComputerName"].as_str().unwrap_or("").to_string(),
                workgroup: v["Workgroup"].as_str().unwrap_or("").to_string(),
            };
        }
    }
    SharesInfo::default()
}
