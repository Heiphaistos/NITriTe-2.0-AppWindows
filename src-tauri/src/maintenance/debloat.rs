use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::error::NiTriTeError;

#[derive(Debug, Clone, Serialize)]
pub struct DebloatResult {
    pub action: String,
    pub success: bool,
    pub message: String,
}

fn run_ps(script: &str) -> DebloatResult {
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x08000000)
        .output();

    match out {
        Ok(o) => DebloatResult {
            action: script[..script.len().min(60)].to_string(),
            success: o.status.success(),
            message: String::from_utf8_lossy(&o.stdout).trim().to_string(),
        },
        Err(e) => DebloatResult {
            action: "powershell".to_string(),
            success: false,
            message: e.to_string(),
        },
    }
}

fn run_cmd(args: &[&str]) -> bool {
    Command::new("cmd")
        .args(["/C"].iter().chain(args.iter()).copied())
        .creation_flags(0x08000000)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn disable_telemetry() -> Result<DebloatResult, NiTriTeError> {
    let script = r#"
        Stop-Service DiagTrack -Force -ErrorAction SilentlyContinue
        Set-Service DiagTrack -StartupType Disabled -ErrorAction SilentlyContinue
        Stop-Service dmwappushsvc -Force -ErrorAction SilentlyContinue
        Set-Service dmwappushsvc -StartupType Disabled -ErrorAction SilentlyContinue
        reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\DataCollection" /v AllowTelemetry /t REG_DWORD /d 0 /f
        Write-Output "Telemetrie desactivee"
    "#;
    Ok(run_ps(script))
}

pub fn disable_cortana() -> Result<DebloatResult, NiTriTeError> {
    let script = r#"
        reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\Windows Search" /v AllowCortana /t REG_DWORD /d 0 /f
        reg add "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Search" /v BingSearchEnabled /t REG_DWORD /d 0 /f
        Write-Output "Cortana desactivee"
    "#;
    Ok(run_ps(script))
}

pub fn disable_xbox_services() -> Result<DebloatResult, NiTriTeError> {
    let script = r#"
        $services = @('XblAuthManager','XblGameSave','XboxGipSvc','XboxNetApiSvc')
        foreach ($s in $services) {
            Stop-Service $s -Force -ErrorAction SilentlyContinue
            Set-Service $s -StartupType Disabled -ErrorAction SilentlyContinue
        }
        Write-Output "Services Xbox desactives"
    "#;
    Ok(run_ps(script))
}

pub fn disable_superfetch() -> Result<DebloatResult, NiTriTeError> {
    let script = r#"
        Stop-Service SysMain -Force -ErrorAction SilentlyContinue
        Set-Service SysMain -StartupType Disabled -ErrorAction SilentlyContinue
        Write-Output "SysMain (Superfetch) desactive"
    "#;
    Ok(run_ps(script))
}

pub fn disable_windows_tips() -> Result<DebloatResult, NiTriTeError> {
    let script = r#"
        reg add "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\ContentDeliveryManager" /v SystemPaneSuggestionsEnabled /t REG_DWORD /d 0 /f
        reg add "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\ContentDeliveryManager" /v SoftLandingEnabled /t REG_DWORD /d 0 /f
        reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\CloudContent" /v DisableWindowsConsumerFeatures /t REG_DWORD /d 1 /f
        Write-Output "Conseils Windows desactives"
    "#;
    Ok(run_ps(script))
}

pub fn optimize_power_plan() -> Result<DebloatResult, NiTriTeError> {
    let ok = run_cmd(&["powercfg", "/setactive", "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"]);
    Ok(DebloatResult {
        action: "Power plan Haute performance".to_string(),
        success: ok,
        message: if ok {
            "Plan Haute performance activé".to_string()
        } else {
            "Erreur activation plan".to_string()
        },
    })
}

pub fn disable_visual_effects() -> Result<DebloatResult, NiTriTeError> {
    let script = r#"
        reg add "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects" /v VisualFXSetting /t REG_DWORD /d 2 /f
        reg add "HKCU\Control Panel\Desktop" /v UserPreferencesMask /t REG_BINARY /d 9012038010000000 /f
        Write-Output "Effets visuels reduits"
    "#;
    Ok(run_ps(script))
}

pub fn clear_event_logs() -> Result<DebloatResult, NiTriTeError> {
    let script = r#"
        $logs = wevtutil el
        $count = 0
        foreach ($log in $logs) {
            try { wevtutil cl $log 2>$null; $count++ } catch {}
        }
        Write-Output "$count journaux vides"
    "#;
    Ok(run_ps(script))
}

pub fn clear_windowsupdate_cache() -> Result<DebloatResult, NiTriTeError> {
    let script = r#"
        Stop-Service wuauserv -Force -ErrorAction SilentlyContinue
        Stop-Service bits -Force -ErrorAction SilentlyContinue
        Remove-Item "$env:SystemRoot\SoftwareDistribution\Download\*" -Recurse -Force -ErrorAction SilentlyContinue
        Start-Service wuauserv -ErrorAction SilentlyContinue
        Start-Service bits -ErrorAction SilentlyContinue
        Write-Output "Cache Windows Update vide"
    "#;
    Ok(run_ps(script))
}

pub fn flush_dns() -> Result<DebloatResult, NiTriTeError> {
    let ok = run_cmd(&["ipconfig", "/flushdns"]);
    Ok(DebloatResult {
        action: "Flush DNS".to_string(),
        success: ok,
        message: if ok {
            "Cache DNS vidé".to_string()
        } else {
            "Erreur flush DNS".to_string()
        },
    })
}

pub fn reset_network_stack() -> Result<DebloatResult, NiTriTeError> {
    let script = r#"
        netsh winsock reset
        netsh int ip reset
        Write-Output "Stack reseau reinitialise"
    "#;
    Ok(run_ps(script))
}

pub fn enable_trim() -> Result<DebloatResult, NiTriTeError> {
    let ok = run_cmd(&["fsutil", "behavior", "set", "DisableDeleteNotify", "0"]);
    Ok(DebloatResult {
        action: "TRIM SSD".to_string(),
        success: ok,
        message: if ok {
            "TRIM activé pour SSD".to_string()
        } else {
            "Erreur activation TRIM".to_string()
        },
    })
}

// ============================================================================
// Extra debloat actions — dispatched via run_extra_action
// ============================================================================

pub fn run_extra_action(action: &str) -> DebloatResult {
    let label = action.to_string();
    let result = match action {
        // --- Debloat Windows ---
        "disable_gamebar" => run_ps(r#"
            reg add "HKCU\SOFTWARE\Microsoft\GameBar" /v UseNexusForGameBarEnabled /t REG_DWORD /d 0 /f
            reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\GameDVR" /v AllowGameDVR /t REG_DWORD /d 0 /f
            Write-Output "Game Bar et DVR desactives"
        "#),
        "disable_bing_search" => run_ps(r#"
            reg add "HKCU\SOFTWARE\Policies\Microsoft\Windows\Explorer" /v DisableSearchBoxSuggestions /t REG_DWORD /d 1 /f
            reg add "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Search" /v BingSearchEnabled /t REG_DWORD /d 0 /f
            Write-Output "Recherche Bing desactivee"
        "#),
        "disable_widgets" => run_ps(r#"
            reg add "HKLM\SOFTWARE\Policies\Microsoft\Dsh" /v AllowNewsAndInterests /t REG_DWORD /d 0 /f
            reg add "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced" /v TaskbarDa /t REG_DWORD /d 0 /f
            Write-Output "Widgets desactives"
        "#),
        "disable_ads" => run_ps(r#"
            reg add "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\ContentDeliveryManager" /v SubscribedContent-338387Enabled /t REG_DWORD /d 0 /f
            reg add "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\ContentDeliveryManager" /v SubscribedContent-353694Enabled /t REG_DWORD /d 0 /f
            reg add "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\ContentDeliveryManager" /v SubscribedContent-353696Enabled /t REG_DWORD /d 0 /f
            reg add "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\AdvertisingInfo" /v Enabled /t REG_DWORD /d 0 /f
            Write-Output "Publicites Windows desactivees"
        "#),
        "disable_activity_history" => run_ps(r#"
            reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\System" /v EnableActivityFeed /t REG_DWORD /d 0 /f
            reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\System" /v PublishUserActivities /t REG_DWORD /d 0 /f
            Write-Output "Historique d'activite desactive"
        "#),
        "disable_remote_assistance" => run_ps(r#"
            reg add "HKLM\SYSTEM\CurrentControlSet\Control\Remote Assistance" /v fAllowToGetHelp /t REG_DWORD /d 0 /f
            Write-Output "Assistance a distance desactivee"
        "#),
        "disable_startup_sound" => run_ps(r#"
            reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System" /v DisableStartupSound /t REG_DWORD /d 1 /f
            Write-Output "Son de demarrage desactive"
        "#),
        "clear_prefetch" => run_ps(r#"
            Remove-Item "$env:SystemRoot\Prefetch\*" -Force -ErrorAction SilentlyContinue
            Write-Output "Prefetch vide"
        "#),
        "remove_teams" => run_ps(r#"
            Get-AppxPackage -Name 'MicrosoftTeams' | Remove-AppxPackage -ErrorAction SilentlyContinue
            $path = "${env:ProgramData}\Microsoft\Teams"
            if (Test-Path $path) { Remove-Item $path -Recurse -Force -ErrorAction SilentlyContinue }
            Write-Output "Teams supprime"
        "#),
        "disable_recall" => run_ps(r#"
            reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\WindowsAI" /v DisableAIDataAnalysis /t REG_DWORD /d 1 /f
            Disable-WindowsOptionalFeature -FeatureName "Recall" -Online -ErrorAction SilentlyContinue
            Write-Output "Windows Recall desactive"
        "#),
        "disable_ink_workspace" => run_ps(r#"
            reg add "HKLM\SOFTWARE\Policies\Microsoft\WindowsInkWorkspace" /v AllowWindowsInkWorkspace /t REG_DWORD /d 0 /f
            Write-Output "Espace de travail Windows Ink desactive"
        "#),
        "disable_location" => run_ps(r#"
            reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\LocationAndSensors" /v DisableLocation /t REG_DWORD /d 1 /f
            Stop-Service lfsvc -Force -ErrorAction SilentlyContinue
            Set-Service lfsvc -StartupType Disabled -ErrorAction SilentlyContinue
            Write-Output "Service de localisation desactive"
        "#),
        "disable_feedback" => run_ps(r#"
            reg add "HKCU\SOFTWARE\Microsoft\Siuf\Rules" /v NumberOfSIUFInPeriod /t REG_DWORD /d 0 /f
            reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\DataCollection" /v DoNotShowFeedbackNotifications /t REG_DWORD /d 1 /f
            Write-Output "Retours d'experience desactives"
        "#),
        "disable_consumer_features" => run_ps(r#"
            reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\CloudContent" /v DisableWindowsConsumerFeatures /t REG_DWORD /d 1 /f
            reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\CloudContent" /v DisableSoftLanding /t REG_DWORD /d 1 /f
            Write-Output "Fonctionnalites promotionnelles Windows desactivees"
        "#),
        // --- Réseau ---
        "renew_ip" => run_ps(r#"
            ipconfig /release
            ipconfig /renew
            Write-Output "IP renouvelle"
        "#),
        "disable_ipv6" => run_ps(r#"
            Get-NetAdapter | ForEach-Object { Disable-NetAdapterBinding -InterfaceAlias $_.Name -ComponentID ms_tcpip6 -ErrorAction SilentlyContinue }
            Write-Output "IPv6 desactive sur toutes les interfaces"
        "#),
        "disable_teredo" => run_ps(r#"
            netsh interface teredo set state disabled
            Write-Output "Teredo desactive"
        "#),
        "disable_llmnr" => run_ps(r#"
            reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows NT\DNSClient" /v EnableMulticast /t REG_DWORD /d 0 /f
            Write-Output "LLMNR desactive"
        "#),
        "disable_netbios" => run_ps(r#"
            $adapters = Get-WmiObject Win32_NetworkAdapterConfiguration -Filter "IPEnabled=True"
            foreach ($a in $adapters) { $a.SetTcpipNetbios(2) | Out-Null }
            Write-Output "NetBIOS over TCP/IP desactive"
        "#),
        "reset_firewall" => run_ps(r#"
            netsh advfirewall reset
            Write-Output "Pare-feu reinitialise aux parametres par defaut"
        "#),
        "disable_wifi_sense" => run_ps(r#"
            reg add "HKLM\SOFTWARE\Microsoft\WcmSvc\wifinetworkmanager\config" /v AutoConnectAllowedOEM /t REG_DWORD /d 0 /f
            Write-Output "Wi-Fi Sense desactive"
        "#),
        "disable_nagle" => run_ps(r#"
            $ifaces = Get-ChildItem "HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces"
            foreach ($if in $ifaces) {
                Set-ItemProperty -Path $if.Name -Name TcpAckFrequency -Value 1 -Force -ErrorAction SilentlyContinue
                Set-ItemProperty -Path $if.Name -Name TCPNoDelay -Value 1 -Force -ErrorAction SilentlyContinue
            }
            Write-Output "Algorithme de Nagle desactive"
        "#),
        "purge_arp" => run_ps(r#"
            netsh interface ip delete arpcache
            Write-Output "Cache ARP purge"
        "#),
        "register_dns" => run_ps(r#"
            ipconfig /registerdns
            Write-Output "DNS re-enregistre"
        "#),
        "set_dns_google" => run_ps(r#"
            $adapters = Get-NetAdapter | Where-Object Status -eq 'Up'
            foreach ($a in $adapters) {
                Set-DnsClientServerAddress -InterfaceIndex $a.InterfaceIndex -ServerAddresses ('8.8.8.8','8.8.4.4') -ErrorAction SilentlyContinue
            }
            Write-Output "DNS Google (8.8.8.8 / 8.8.4.4) configure"
        "#),
        "set_dns_cloudflare" => run_ps(r#"
            $adapters = Get-NetAdapter | Where-Object Status -eq 'Up'
            foreach ($a in $adapters) {
                Set-DnsClientServerAddress -InterfaceIndex $a.InterfaceIndex -ServerAddresses ('1.1.1.1','1.0.0.1') -ErrorAction SilentlyContinue
            }
            Write-Output "DNS Cloudflare (1.1.1.1 / 1.0.0.1) configure"
        "#),
        "optimize_mtu" => run_ps(r#"
            netsh interface ipv4 set subinterface "Ethernet" mtu=1500 store=persistent 2>$null
            netsh interface ipv4 set subinterface "Wi-Fi" mtu=1500 store=persistent 2>$null
            Write-Output "MTU optimise a 1500"
        "#),
        "show_net_stats" => run_ps(r#"
            $stats = netstat -e
            Write-Output $stats
        "#),
        "reset_winsock_only" => run_ps(r#"
            netsh winsock reset
            Write-Output "Winsock reinitialise (redemarrage requis)"
        "#),
        "reset_tcpip_only" => run_ps(r#"
            netsh int ip reset
            Write-Output "TCP/IP reinitialise (redemarrage requis)"
        "#),
        "disable_rdp" => run_ps(r#"
            reg add "HKLM\SYSTEM\CurrentControlSet\Control\Terminal Server" /v fDenyTSConnections /t REG_DWORD /d 1 /f
            Write-Output "Bureau a distance desactive"
        "#),
        // --- Performance ---
        "empty_standby" => run_ps(r#"
            $code = @"
using System;using System.Runtime.InteropServices;
public class MemUtil { [DllImport("psapi.dll")] public static extern int EmptyWorkingSet(IntPtr h); }
"@
            Add-Type -TypeDefinition $code -ErrorAction SilentlyContinue
            Get-Process | ForEach-Object { try { [MemUtil]::EmptyWorkingSet($_.Handle) | Out-Null } catch {} }
            Write-Output "Memoire en veille liberee"
        "#),
        "disable_search_index" => run_ps(r#"
            Stop-Service WSearch -Force -ErrorAction SilentlyContinue
            Set-Service WSearch -StartupType Disabled -ErrorAction SilentlyContinue
            Write-Output "Indexation Windows Search desactivee"
        "#),
        "disable_error_reporting" => run_ps(r#"
            reg add "HKLM\SOFTWARE\Microsoft\Windows\Windows Error Reporting" /v Disabled /t REG_DWORD /d 1 /f
            Stop-Service WerSvc -Force -ErrorAction SilentlyContinue
            Set-Service WerSvc -StartupType Disabled -ErrorAction SilentlyContinue
            Write-Output "Rapport d'erreurs desactive"
        "#),
        "boost_foreground" => run_ps(r#"
            reg add "HKLM\SYSTEM\CurrentControlSet\Control\PriorityControl" /v Win32PrioritySeparation /t REG_DWORD /d 38 /f
            Write-Output "Priorite applications premier plan augmentee"
        "#),
        "disable_write_cache" => run_ps(r#"
            $disks = Get-WmiObject Win32_DiskDrive -ErrorAction SilentlyContinue
            $count = 0
            foreach ($disk in $disks) {
                try {
                    $diskNum = $disk.Index
                    $policy = Get-StoragePolicy -ErrorAction SilentlyContinue
                    Set-Disk -Number $diskNum -IsReadOnly $false -ErrorAction SilentlyContinue
                    $count++
                } catch {}
            }
            Write-Output "Etat du cache d'ecriture verifie sur $count disque(s) (desactivation complete via Gestionnaire de peripheriques)"
        "#),
        "disable_auto_maintenance" => run_ps(r#"
            reg add "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Schedule\Maintenance" /v MaintenanceDisabled /t REG_DWORD /d 1 /f
            Write-Output "Maintenance automatique desactivee"
        "#),
        "disable_bg_apps" => run_ps(r#"
            reg add "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications" /v GlobalUserDisabled /t REG_DWORD /d 1 /f
            Write-Output "Applications en arriere-plan desactivees"
        "#),
        "disable_start_animations" => run_ps(r#"
            reg add "HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced" /v ListviewAlphaSelect /t REG_DWORD /d 0 /f
            reg add "HKCU\Control Panel\Desktop\WindowMetrics" /v MinAnimate /t REG_SZ /d 0 /f
            Write-Output "Animations menus desactivees"
        "#),
        "optimize_pagefile" => run_ps(r#"
            $cs = Get-WmiObject Win32_ComputerSystem
            $cs.AutomaticManagedPagefile = $true
            $cs.Put() | Out-Null
            Write-Output "Fichier d'echange configure en automatique"
        "#),
        "disable_hpet" => run_ps(r#"
            bcdedit /deletevalue useplatformclock 2>$null
            bcdedit /set disabledynamictick yes 2>$null
            Write-Output "HPET optimise pour le gaming"
        "#),
        "enable_msi_mode" => run_ps(r#"
            $pci = Get-WmiObject Win32_VideoController
            Write-Output "Mode MSI (Message Signaled Interrupts) : configurer via Device Manager > Properties > Interrupts"
        "#),
        "clean_old_shadows" => run_ps(r#"
            vssadmin delete shadows /for=C: /oldest /quiet
            Write-Output "Anciens points de restauration supprimes"
        "#),
        "set_power_min_processor" => run_ps(r#"
            powercfg /SETACVALUEINDEX SCHEME_CURRENT SUB_PROCESSOR PROCTHROTTLEMIN 5
            powercfg /SETDCVALUEINDEX SCHEME_CURRENT SUB_PROCESSOR PROCTHROTTLEMIN 5
            powercfg /setactive SCHEME_CURRENT
            Write-Output "Etat min processeur = 5% (evite throttling)"
        "#),
        "disable_spectre_meltdown" => DebloatResult {
            action: "disable_spectre_meltdown".to_string(),
            success: false,
            message: "Action non appliquée : désactivation Spectre/Meltdown non recommandée (correctifs de sécurité critiques)".to_string(),
        },
        _ => DebloatResult {
            action: action.to_string(),
            success: false,
            message: format!("Action inconnue: {}", action),
        },
    };
    DebloatResult { action: label, ..result }
}

#[tauri::command]
pub async fn debloat_run_extra(action: String) -> Result<DebloatResult, NiTriTeError> {
    let result = tokio::task::spawn_blocking(move || run_extra_action(&action))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?;
    Ok(result)
}

pub fn remove_bloatware_uwp(apps: Vec<String>) -> Result<Vec<DebloatResult>, NiTriTeError> {
    let mut results = Vec::new();

    let default_bloatware = if apps.is_empty() {
        vec![
            "Microsoft.XboxApp",
            "Microsoft.Xbox.TCUI",
            "Microsoft.XboxGameOverlay",
            "Microsoft.XboxGamingOverlay",
            "Microsoft.XboxIdentityProvider",
            "Microsoft.ZuneMusic",
            "Microsoft.ZuneVideo",
            "Microsoft.BingWeather",
            "Microsoft.BingNews",
            "Microsoft.BingFinance",
            "Microsoft.BingSports",
            "Microsoft.GetHelp",
            "Microsoft.Getstarted",
            "Microsoft.MicrosoftSolitaireCollection",
            "Microsoft.People",
            "Microsoft.SkypeApp",
            "king.com.CandyCrushSaga",
            "king.com.CandyCrushFriends",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
    } else {
        apps
    };

    for app in &default_bloatware {
        // PS single-quoted string escape : '' (double quote), pas \'
        let script = format!(
            r#"Get-AppxPackage -Name '{}' | Remove-AppxPackage -ErrorAction SilentlyContinue; Write-Output 'done'"#,
            app.replace('\'', "''")
        );
        let mut res = run_ps(&script);
        res.action = format!("Suppression {}", app);
        results.push(res);
    }

    Ok(results)
}
