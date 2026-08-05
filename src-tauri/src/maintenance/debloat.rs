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

/// Exécute un script PowerShell en propageant les échecs réels.
///
/// Historiquement chaque script se terminait par un `Write-Output` → le
/// process sortait toujours en code 0, même quand les `reg add`/`netsh`
/// échouaient (droits admin manquants) : tous les boutons rapportaient un
/// faux succès. Le script est donc enveloppé : `$ErrorActionPreference =
/// 'Stop'`, shims `reg`/`netsh`/`powercfg`/`wevtutil` qui convertissent un
/// code retour non nul en exception, et try/catch global qui sort en code 1
/// avec le message d'erreur. Les `-ErrorAction SilentlyContinue` explicites
/// des scripts (actions volontairement best-effort) restent silencieux.
fn run_ps(script: &str) -> DebloatResult {
    let wrapped = format!(
        r#"$ErrorActionPreference = 'Stop'
function reg {{ & "$env:SystemRoot\System32\reg.exe" @args | Out-Null; if ($LASTEXITCODE -ne 0) {{ throw "reg a echoue (code $LASTEXITCODE) : $($args -join ' ')" }} }}
function netsh {{ & "$env:SystemRoot\System32\netsh.exe" @args | Out-Null; if ($LASTEXITCODE -ne 0) {{ throw "netsh a echoue (code $LASTEXITCODE) : $($args -join ' ')" }} }}
function powercfg {{ & "$env:SystemRoot\System32\powercfg.exe" @args | Out-Null; if ($LASTEXITCODE -ne 0) {{ throw "powercfg a echoue (code $LASTEXITCODE) : $($args -join ' ')" }} }}
function wevtutil {{ & "$env:SystemRoot\System32\wevtutil.exe" @args; if ($LASTEXITCODE -ne 0) {{ throw "wevtutil a echoue (code $LASTEXITCODE)" }} }}
try {{
{script}
}} catch {{ Write-Output $_.Exception.Message; exit 1 }}"#
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &wrapped])
        .creation_flags(0x08000000)
        .output();

    match out {
        Ok(o) => {
            let stdout = crate::maintenance::commands::decode_output(&o.stdout).trim().to_string();
            let success = o.status.success();
            let message = if !success && stdout.is_empty() {
                crate::maintenance::commands::decode_output(&o.stderr).trim().to_string()
            } else {
                stdout
            };
            DebloatResult {
                action: script.chars().take(60).collect::<String>(),
                success,
                message,
            }
        }
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
    // Set-Service ... -EA SilentlyContinue avalait un vrai échec (droits admin
    // manquants) sans jamais le refléter : le message annonçait "Telemetrie
    // desactivee" même quand aucun des 2 services n'avait réellement été
    // désactivé. Reproduit en direct avec un nom de service inexistant (même
    // échec réel qu'un accès refusé) : Set-Service échoue silencieusement,
    // le script continue, exit 0. La clé de registre AllowTelemetry (vérifiée
    // via le shim reg qui lève une exception sur échec) reste le contrôle
    // faisant autorité, donc pas d'échec bloquant si les services échouent —
    // mais le message doit refléter honnêtement leur vrai statut.
    let script = r#"
        $services = @('DiagTrack','dmwappushsvc')
        $n = 0
        foreach ($s in $services) {
            Stop-Service $s -Force -ErrorAction SilentlyContinue
            try { Set-Service $s -StartupType Disabled -ErrorAction Stop; $n++ } catch {}
        }
        reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\DataCollection" /v AllowTelemetry /t REG_DWORD /d 0 /f
        if ($n -eq $services.Count) { Write-Output "Telemetrie desactivee (cle de registre + $n/$($services.Count) services)" }
        else { Write-Output "Telemetrie desactivee par cle de registre - $n/$($services.Count) service(s) seulement (droits administrateur requis pour les services)" }
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
        $n = 0
        foreach ($s in $services) {
            Stop-Service $s -Force -ErrorAction SilentlyContinue
            try { Set-Service $s -StartupType Disabled -ErrorAction Stop; $n++ } catch {}
        }
        if ($n -eq 0) { throw "Aucun service Xbox desactive (droits administrateur requis ?)" }
        Write-Output "$n services Xbox desactives"
    "#;
    Ok(run_ps(script))
}

pub fn disable_superfetch() -> Result<DebloatResult, NiTriTeError> {
    let script = r#"
        Stop-Service SysMain -Force -ErrorAction SilentlyContinue
        Set-Service SysMain -StartupType Disabled
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
        if ($count -eq 0) { throw "Aucun journal vide (droits administrateur requis ?)" }
        Write-Output "$count journaux vides"
    "#;
    Ok(run_ps(script))
}

pub fn clear_windowsupdate_cache() -> Result<DebloatResult, NiTriTeError> {
    let script = r#"
        Stop-Service wuauserv -Force
        Stop-Service bits -Force
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
            $pkg = Get-AppxPackage -Name 'MicrosoftTeams' -ErrorAction SilentlyContinue
            if ($pkg) { $pkg | Remove-AppxPackage }
            if (Get-AppxPackage -Name 'MicrosoftTeams' -ErrorAction SilentlyContinue) { throw "Echec suppression du package Teams" }
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
        // Même bug que disable_telemetry (Set-Service -EA SilentlyContinue non
        // vérifié) : la clé de registre DisableLocation fait autorité (shim reg
        // vérifié), le service lfsvc reste best-effort mais son vrai statut est
        // maintenant reflété dans le message plutôt que masqué.
        "disable_location" => run_ps(r#"
            reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\LocationAndSensors" /v DisableLocation /t REG_DWORD /d 1 /f
            Stop-Service lfsvc -Force -ErrorAction SilentlyContinue
            $svcOk = $false
            try { Set-Service lfsvc -StartupType Disabled -ErrorAction Stop; $svcOk = $true } catch {}
            if ($svcOk) { Write-Output "Service de localisation desactive" }
            else { Write-Output "Localisation desactivee par cle de registre - service lfsvc non modifie (droits administrateur requis ?)" }
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
            $n = 0
            Get-NetAdapter | ForEach-Object { try { Disable-NetAdapterBinding -InterfaceAlias $_.Name -ComponentID ms_tcpip6 -ErrorAction Stop; $n++ } catch {} }
            if ($n -eq 0) { throw "IPv6 non desactive (droits administrateur requis ?)" }
            Write-Output "IPv6 desactive sur $n interface(s)"
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
            $n = 0
            foreach ($a in $adapters) {
                $r = $a.SetTcpipNetbios(2)
                if ($r.ReturnValue -eq 0 -or $r.ReturnValue -eq 1) { $n++ }
            }
            if ($n -eq 0 -and $adapters) { throw "NetBIOS non desactive (droits administrateur requis ?)" }
            Write-Output "NetBIOS over TCP/IP desactive sur $n interface(s)"
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
            $n = 0
            foreach ($if in $ifaces) {
                try {
                    Set-ItemProperty -Path $if.PSPath -Name TcpAckFrequency -Value 1 -Force -ErrorAction Stop
                    Set-ItemProperty -Path $if.PSPath -Name TCPNoDelay -Value 1 -Force -ErrorAction Stop
                    $n++
                } catch {}
            }
            if ($n -eq 0) { throw "Nagle non desactive (droits administrateur requis ?)" }
            Write-Output "Algorithme de Nagle desactive sur $n interface(s)"
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
            $n = 0
            Get-NetAdapter | Where-Object Status -eq 'Up' | ForEach-Object {
                try { Set-DnsClientServerAddress -InterfaceIndex $_.InterfaceIndex -ServerAddresses ('8.8.8.8','8.8.4.4') -ErrorAction Stop; $n++ } catch {}
            }
            if ($n -eq 0) { throw "DNS non modifie (droits administrateur requis ?)" }
            Write-Output "DNS Google (8.8.8.8 / 8.8.4.4) configure sur $n interface(s)"
        "#),
        "set_dns_cloudflare" => run_ps(r#"
            $n = 0
            Get-NetAdapter | Where-Object Status -eq 'Up' | ForEach-Object {
                try { Set-DnsClientServerAddress -InterfaceIndex $_.InterfaceIndex -ServerAddresses ('1.1.1.1','1.0.0.1') -ErrorAction Stop; $n++ } catch {}
            }
            if ($n -eq 0) { throw "DNS non modifie (droits administrateur requis ?)" }
            Write-Output "DNS Cloudflare (1.1.1.1 / 1.0.0.1) configure sur $n interface(s)"
        "#),
        "optimize_mtu" => run_ps(r#"
            $n = 0
            foreach ($a in (Get-NetAdapter -Physical | Where-Object Status -eq 'Up')) {
                try { netsh interface ipv4 set subinterface "$($a.Name)" mtu=1500 store=persistent; $n++ } catch {}
            }
            if ($n -eq 0) { throw "MTU non modifie (droits administrateur requis ou aucune interface active)" }
            Write-Output "MTU 1500 applique sur $n interface(s)"
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
            Set-Service WSearch -StartupType Disabled
            Write-Output "Indexation Windows Search desactivee"
        "#),
        // Même bug que disable_telemetry/disable_location (Set-Service -EA
        // SilentlyContinue non vérifié).
        "disable_error_reporting" => run_ps(r#"
            reg add "HKLM\SOFTWARE\Microsoft\Windows\Windows Error Reporting" /v Disabled /t REG_DWORD /d 1 /f
            Stop-Service WerSvc -Force -ErrorAction SilentlyContinue
            $svcOk = $false
            try { Set-Service WerSvc -StartupType Disabled -ErrorAction Stop; $svcOk = $true } catch {}
            if ($svcOk) { Write-Output "Rapport d'erreurs desactive" }
            else { Write-Output "Rapport d'erreurs desactive par cle de registre - service WerSvc non modifie (droits administrateur requis ?)" }
        "#),
        "boost_foreground" => run_ps(r#"
            reg add "HKLM\SYSTEM\CurrentControlSet\Control\PriorityControl" /v Win32PrioritySeparation /t REG_DWORD /d 38 /f
            Write-Output "Priorite applications premier plan augmentee"
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
            try { bcdedit /deletevalue useplatformclock 2>&1 | Out-Null } catch {}
            try { bcdedit /set disabledynamictick yes 2>&1 | Out-Null } catch { throw "bcdedit a echoue (droits administrateur requis ?)" }
            if ($LASTEXITCODE -ne 0) { throw "bcdedit a echoue (droits administrateur requis ?)" }
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
        // PS single-quoted string escape : '' (double quote), pas \'.
        // On VÉRIFIE la suppression réelle (Get-AppxPackage après) + exit code :
        // sinon `Remove-AppxPackage -SilentlyContinue; Write-Output done` renvoyait
        // toujours success=true, même quand la suppression échouait.
        let script = format!(
            r#"$n = '{}'
$pkg = Get-AppxPackage -Name $n -ErrorAction SilentlyContinue
if (-not $pkg) {{ Write-Output 'Non installe'; exit 0 }}
$pkg | Remove-AppxPackage -ErrorAction SilentlyContinue
if (Get-AppxPackage -Name $n -ErrorAction SilentlyContinue) {{ Write-Output 'Echec suppression'; exit 1 }}
Write-Output 'Supprime'"#,
            app.replace('\'', "''")
        );
        let mut res = run_ps(&script);
        res.action = format!("Suppression {}", app);
        results.push(res);
    }

    Ok(results)
}
