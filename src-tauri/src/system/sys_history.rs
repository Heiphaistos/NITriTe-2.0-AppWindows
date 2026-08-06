use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Default)]
pub struct RecentInstall {
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub install_date: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RecentEvent {
    pub time: String,
    pub source: String,
    pub message: String,
    pub id: u64,
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SystemHistory {
    pub windows_install_date: String,
    pub last_boot_time: String,
    pub current_uptime_hours: f64,
    pub total_uptime_days_since_install: f64,
    pub bsod_count_30d: u32,
    pub bsod_list: Vec<String>,
    pub recent_installs: Vec<RecentInstall>,
    pub recent_uninstalls: Vec<String>,
    pub critical_events_7d: u32,
    pub error_events_7d: u32,
    pub warning_events_7d: u32,
    pub last_logon_user: String,
    pub shutdown_count: u32,
    pub hibernation_count: u32,
    pub crash_count_30d: u32,
    pub recent_critical: Vec<RecentEvent>,
}

// Anti-freeze : PowerShell/registre est bloquant — jamais inline sur le
// thread de commande.
#[tauri::command]
pub async fn get_system_history() -> SystemHistory {
    tokio::task::spawn_blocking(get_system_history_blocking)
        .await
        .unwrap_or_default()
}

fn get_system_history_blocking() -> SystemHistory {
    let ps = r#"
# Sortie UTF-8 : messages d'événements et libellés FR accentués (« Critique »,
# « Erreur », texte des événements) seraient sinon mojibake (from_utf8_lossy).
$OutputEncoding = [System.Text.Encoding]::UTF8; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$out = @{}

# Date d'installation Windows
try {
    $inst = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion' `
        -ErrorAction SilentlyContinue).InstallDate
    if ($inst) {
        $instDate = [System.DateTimeOffset]::FromUnixTimeSeconds([long]$inst).LocalDateTime
        $out.InstallDate = $instDate.ToString('yyyy-MM-dd')
        $out.UptimeTotalDays = [math]::Round(((Get-Date) - $instDate).TotalDays, 0)
    } else { $out.InstallDate = "Inconnu"; $out.UptimeTotalDays = 0 }
} catch { $out.InstallDate = "Inconnu"; $out.UptimeTotalDays = 0 }

# Dernier démarrage
try {
    $lastBoot = (Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue).LastBootUpTime
    $out.LastBoot = $lastBoot.ToString('yyyy-MM-dd HH:mm')
    $out.UptimeHours = [math]::Round(((Get-Date) - $lastBoot).TotalHours, 1)
} catch { $out.LastBoot = "Inconnu"; $out.UptimeHours = 0.0 }

# BSODs (Event 41 - noyau) des 30 derniers jours
try {
    $bsods = Get-WinEvent -FilterHashtable @{
        LogName='System'; Id=41; StartTime=(Get-Date).AddDays(-30)
    } -ErrorAction SilentlyContinue | Select-Object -First 20
    $out.BsodCount = [int]($bsods | Measure-Object).Count
    $out.BsodList  = @($bsods | ForEach-Object { $_.TimeCreated.ToString('yyyy-MM-dd HH:mm') })
} catch { $out.BsodCount = 0; $out.BsodList = @() }

# Événements 7 jours
try {
    $since = (Get-Date).AddDays(-7)
    $evts  = Get-WinEvent -FilterHashtable @{LogName='System','Application';StartTime=$since} `
        -ErrorAction SilentlyContinue
    $out.CritEvents = [int]($evts | Where-Object {$_.Level -eq 1} | Measure-Object).Count
    $out.ErrEvents  = [int]($evts | Where-Object {$_.Level -eq 2} | Measure-Object).Count
    $out.WarnEvents = [int]($evts | Where-Object {$_.Level -eq 3} | Measure-Object).Count
    $out.RecentCrit = @($evts | Where-Object {$_.Level -le 2} | Select-Object -First 20 | ForEach-Object {
        @{time=$_.TimeCreated.ToString('yyyy-MM-dd HH:mm'); source=$_.ProviderName;
          msg=$_.Message.Substring(0,[Math]::Min(120,$_.Message.Length));
          id=$_.Id; level=if($_.Level -eq 1){'Critique'}else{'Erreur'}}
    })
} catch { $out.CritEvents=0; $out.ErrEvents=0; $out.WarnEvents=0; $out.RecentCrit=@() }

# Dernière session utilisateur
try {
    $lastLogon = Get-WinEvent -FilterHashtable @{LogName='Security';Id=4624} `
        -MaxEvents 5 -ErrorAction SilentlyContinue | Select-Object -First 1
    $out.LastLogonUser = if ($lastLogon) { [string]$lastLogon.Properties[5].Value } else { $env:USERNAME }
} catch { $out.LastLogonUser = $env:USERNAME }

# Logiciels installés récemment (30 jours)
try {
    $date30 = (Get-Date).AddDays(-30).ToString('yyyyMMdd')
    $recent = Get-ItemProperty `
        "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKLM:\Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*" `
        -ErrorAction SilentlyContinue |
        Where-Object { $_.DisplayName -and $_.InstallDate -and $_.InstallDate -ge $date30 } |
        Sort-Object InstallDate -Descending | Select-Object -First 30
    $out.RecentInstalls = @($recent | ForEach-Object {
        # InstallDate registre = "yyyyMMdd" brut (ex: "20260802") — reformaté en
        # "yyyy-MM-dd" pour être lisible dans DiagTabHistory.vue, comme le fait déjà
        # get_installed_apps() (installer/uninstaller.rs) pour la même donnée sur
        # UninstallerPage.vue. Sans ça la table "Logiciels installés" affichait un
        # nombre brut illisible à la place d'une date.
        $d = [string]$_.InstallDate
        $dFmt = if ($d -match '^(\d{4})(\d{2})(\d{2})$') { "$($matches[1])-$($matches[2])-$($matches[3])" } else { $d }
        @{name=[string]$_.DisplayName; version=[string]$_.DisplayVersion;
          publisher=[string]$_.Publisher; installDate=$dFmt}
    })
} catch { $out.RecentInstalls = @() }

# Désinstallations récentes (Event 11707 / 11724)
try {
    $uninstEvts = Get-WinEvent -FilterHashtable @{
        LogName='Application'; Id=11707,11724; StartTime=(Get-Date).AddDays(-30)
    } -ErrorAction SilentlyContinue | Select-Object -First 15
    $out.RecentUninstalls = @($uninstEvts | ForEach-Object {
        "$($_.TimeCreated.ToString('yyyy-MM-dd')) — $($_.Message.Substring(0,[Math]::Min(80,$_.Message.Length)))"
    })
} catch { $out.RecentUninstalls = @() }

# Nombre d'arrêts / hibernations (30 jours, Event 1074)
try {
    $shutdowns = Get-WinEvent -FilterHashtable @{
        LogName='System'; Id=1074; StartTime=(Get-Date).AddDays(-30)
    } -ErrorAction SilentlyContinue
    $out.ShutdownCount   = [int]($shutdowns | Measure-Object).Count
    $out.HibernateCount  = [int]($shutdowns | Where-Object { $_.Properties[4].Value -like '*hiber*' } | Measure-Object).Count
} catch { $out.ShutdownCount=0; $out.HibernateCount=0 }

$out | ConvertTo-Json -Depth 3 -Compress
"#;

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output();

        if let Ok(o) = output {
            let text = String::from_utf8_lossy(&o.stdout);
            let v: serde_json::Value = match serde_json::from_str(text.trim()) {
                Ok(val) => val, Err(_) => return SystemHistory::default(),
            };

            let recent_installs = v["RecentInstalls"].as_array().map(|a| a.iter().map(|r| RecentInstall {
                name: r["name"].as_str().unwrap_or("").to_string(),
                version: r["version"].as_str().unwrap_or("").to_string(),
                publisher: r["publisher"].as_str().unwrap_or("").to_string(),
                install_date: r["installDate"].as_str().unwrap_or("").to_string(),
            }).collect()).unwrap_or_default();

            let recent_critical = v["RecentCrit"].as_array().map(|a| a.iter().map(|e| RecentEvent {
                time: e["time"].as_str().unwrap_or("").to_string(),
                source: e["source"].as_str().unwrap_or("").to_string(),
                message: e["msg"].as_str().unwrap_or("").to_string(),
                id: e["id"].as_u64().unwrap_or(0),
                level: e["level"].as_str().unwrap_or("").to_string(),
            }).collect()).unwrap_or_default();

            let recent_uninstalls = v["RecentUninstalls"].as_array().map(|a| a.iter()
                .filter_map(|s| s.as_str().map(|x| x.to_string())).collect()).unwrap_or_default();

            return SystemHistory {
                windows_install_date: v["InstallDate"].as_str().unwrap_or("").to_string(),
                last_boot_time: v["LastBoot"].as_str().unwrap_or("").to_string(),
                current_uptime_hours: v["UptimeHours"].as_f64().unwrap_or(0.0),
                total_uptime_days_since_install: v["UptimeTotalDays"].as_f64().unwrap_or(0.0),
                bsod_count_30d: v["BsodCount"].as_u64().unwrap_or(0) as u32,
                bsod_list: v["BsodList"].as_array().map(|a| a.iter()
                    .filter_map(|s| s.as_str().map(|x| x.to_string())).collect()).unwrap_or_default(),
                recent_installs,
                recent_uninstalls,
                critical_events_7d: v["CritEvents"].as_u64().unwrap_or(0) as u32,
                error_events_7d: v["ErrEvents"].as_u64().unwrap_or(0) as u32,
                warning_events_7d: v["WarnEvents"].as_u64().unwrap_or(0) as u32,
                last_logon_user: v["LastLogonUser"].as_str().unwrap_or("").to_string(),
                shutdown_count: v["ShutdownCount"].as_u64().unwrap_or(0) as u32,
                hibernation_count: v["HibernateCount"].as_u64().unwrap_or(0) as u32,
                crash_count_30d: v["BsodCount"].as_u64().unwrap_or(0) as u32,
                recent_critical,
            };
        }
    }
    SystemHistory::default()
}
