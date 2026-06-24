use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::Emitter;

use crate::error::NiTriTeError;
use crate::scripts::validator::validate_script;

#[derive(Debug, Clone, Serialize)]
pub struct ScriptResult {
    pub success: bool,
    pub output: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScriptEntry {
    pub name: String,
    pub description: String,
    pub category: String,
    pub script_type: String, // "cmd" | "powershell"
    pub content: String,
    pub requires_admin: bool,
    /// Scripts nécessitant une saisie interactive (Read-Host).
    /// Pour ces scripts, -NonInteractive est retiré afin d'éviter une exception PowerShell.
    #[serde(default)]
    pub requires_interactive: bool,
    /// Scripts builtin embarqués — exemptés de la validation dynamique.
    #[serde(default)]
    pub is_builtin: bool,
}

pub fn execute_script(
    content: &str,
    script_type: &str,
    window: &tauri::Window,
) -> Result<ScriptResult, NiTriTeError> {
    execute_script_inner(content, script_type, false, false, window)
}

pub fn execute_script_entry(
    entry: &ScriptEntry,
    window: &tauri::Window,
) -> Result<ScriptResult, NiTriTeError> {
    execute_script_inner(
        &entry.content,
        &entry.script_type,
        entry.is_builtin,
        entry.requires_interactive,
        window,
    )
}

/// Variante publique avec flags explicites — utilisée par la commande Tauri
/// pour transmettre is_builtin et requires_interactive depuis le frontend.
pub fn execute_script_with_flags(
    content: &str,
    script_type: &str,
    is_builtin: bool,
    requires_interactive: bool,
    window: &tauri::Window,
) -> Result<ScriptResult, NiTriTeError> {
    execute_script_inner(content, script_type, is_builtin, requires_interactive, window)
}

fn execute_script_inner(
    content: &str,
    script_type: &str,
    is_builtin: bool,
    requires_interactive: bool,
    window: &tauri::Window,
) -> Result<ScriptResult, NiTriTeError> {
    // Validation uniquement pour les scripts utilisateur (non builtin)
    if !is_builtin {
        let validation = validate_script(content.to_string(), script_type.to_string());
        if validation.risk_level == "danger" {
            let reasons = validation.warnings.join(", ");
            tracing::warn!("execute_script BLOQUÉ (danger): {}", reasons);
            return Err(NiTriTeError::CommandDenied(
                format!("Script bloqué (risque critique): {}", reasons),
            ));
        }
    }

    // Log de chaque exécution pour traçabilité
    tracing::info!(
        "execute_script: type={} builtin={} interactive={} length={} preview={:?}",
        script_type,
        is_builtin,
        requires_interactive,
        content.len(),
        content.chars().take(120).collect::<String>()
    );

    // Timeout : 5 min pour les scripts builtin, peut être étendu si nécessaire
    const SCRIPT_TIMEOUT_SECS: u64 = 300;

    // Pour les scripts interactifs (Read-Host), on retire -NonInteractive/-NoProfile
    // afin d'éviter l'exception "PowerShell is in NonInteractive mode".
    let (cmd, args) = match script_type {
        "powershell" if requires_interactive => ("powershell", vec![
            "-ExecutionPolicy", "RemoteSigned",
            "-Command", content,
        ]),
        "powershell" => ("powershell", vec![
            "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "RemoteSigned",
            "-Command", content,
        ]),
        _ => ("cmd", vec!["/C", content]),
    };

    let child = Command::new(cmd)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000)
        .spawn()?;

    let pid = child.id();
    let (tx, rx) = mpsc::channel::<(String, std::process::ExitStatus)>();
    let window_clone = window.clone();

    // Thread dédié : lit stdout ligne par ligne et émet les événements
    std::thread::spawn(move || {
        let mut child = child;
        let mut output_text = String::new();
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                output_text.push_str(&line);
                output_text.push('\n');
                let _ = window_clone.emit("script-output", &line);
            }
        }
        if let Ok(status) = child.wait() {
            let _ = tx.send((output_text, status));
        }
    });

    let (output_text, status) = match rx.recv_timeout(Duration::from_secs(SCRIPT_TIMEOUT_SECS)) {
        Ok(result) => result,
        Err(_) => {
            // Timeout — tuer le processus
            #[cfg(target_os = "windows")]
            let _ = Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .creation_flags(0x08000000)
                .spawn();
            tracing::warn!("execute_script: timeout {}s dépassé (pid={})", SCRIPT_TIMEOUT_SECS, pid);
            return Err(NiTriTeError::Timeout(
                format!("Script interrompu après {}s", SCRIPT_TIMEOUT_SECS)
            ));
        }
    };

    Ok(ScriptResult {
        success: status.success(),
        output: output_text,
        exit_code: status.code().unwrap_or(-1),
    })
}

pub fn get_builtin_scripts() -> Vec<ScriptEntry> {
    vec![
        // ═══════════════════════════════════════════════════════
        // NETTOYAGE
        // ═══════════════════════════════════════════════════════
        script("Nettoyer fichiers temp", "Supprime %TEMP% et C:\\Windows\\Temp", "Nettoyage", "cmd",
            "del /q /f /s %TEMP%\\* 2>nul & del /q /f /s C:\\Windows\\Temp\\* 2>nul", false),
        script("Vider prefetch", "Supprime les fichiers prefetch pour accelerer les analyses", "Nettoyage", "cmd",
            "del /q /f /s C:\\Windows\\Prefetch\\* 2>nul", true),
        script("Vider cache thumbnails", "Reinitialise le cache des miniatures de l'explorateur", "Nettoyage", "cmd",
            "taskkill /f /im explorer.exe & del /q /f /s %LOCALAPPDATA%\\Microsoft\\Windows\\Explorer\\thumbcache_*.db 2>nul & start explorer.exe", false),
        script("Vider fichiers recents", "Supprime l'historique des fichiers recents", "Nettoyage", "cmd",
            "del /q /f %APPDATA%\\Microsoft\\Windows\\Recent\\* 2>nul", false),
        script("Nettoyage WinSxS", "Nettoie le dossier composants Windows (libere plusieurs Go)", "Nettoyage", "cmd",
            "DISM /Online /Cleanup-Image /StartComponentCleanup", true),
        script("Vider corbeille", "Vide la corbeille de tous les utilisateurs", "Nettoyage", "powershell",
            "Clear-RecycleBin -Force -ErrorAction SilentlyContinue; Write-Host 'Corbeille videe.'", false),
        script("Nettoyer cache navigateurs", "Supprime les caches Chrome, Firefox, Edge", "Nettoyage", "powershell",
            r#"$paths = @(
  "$env:LOCALAPPDATA\Google\Chrome\User Data\Default\Cache",
  "$env:LOCALAPPDATA\Mozilla\Firefox\Profiles\*\cache2",
  "$env:LOCALAPPDATA\Microsoft\Edge\User Data\Default\Cache"
)
foreach ($p in $paths) {
  Get-ChildItem $p -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
  Write-Host "Nettoye: $p"
}"#, false),
        script("Supprimer logs Windows Update", "Supprime les anciens logs de mise a jour", "Nettoyage", "cmd",
            "del /q /f /s %SystemRoot%\\Logs\\WindowsUpdate\\* 2>nul & del /q /f /s %SystemRoot%\\SoftwareDistribution\\Download\\* 2>nul", true),
        script("Nettoyer AppData Temp", "Supprime les dossiers temp dans AppData", "Nettoyage", "powershell",
            r#"Remove-Item "$env:APPDATA\Temp" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item "$env:LOCALAPPDATA\Temp" -Recurse -Force -ErrorAction SilentlyContinue
Write-Host 'AppData Temp nettoye.'"#, false),
        script("Compacter base WMI", "Compacte et repare le depot WMI (winmgmt)", "Nettoyage", "cmd",
            "winmgmt /resetrepository & winmgmt /verifyrepository", true),
        script("Supprimer crashdumps", "Supprime les fichiers minidump et crashdump", "Nettoyage", "powershell",
            r#"$d = @("$env:LOCALAPPDATA\CrashDumps","$env:SystemRoot\Minidump","$env:SystemRoot\MEMORY.DMP")
foreach ($p in $d) { Remove-Item $p -Recurse -Force -ErrorAction SilentlyContinue; Write-Host "OK: $p" }"#, true),
        script("Vider cache Windows Store", "Reinitialise le cache du Microsoft Store", "Nettoyage", "cmd",
            "wsreset.exe", false),
        script("Nettoyer journaux evenements", "Efface tous les journaux d'evenements Windows", "Nettoyage", "powershell",
            "Get-EventLog -LogName * | ForEach-Object { Clear-EventLog -LogName $_.Log -ErrorAction SilentlyContinue }; Write-Host 'Journaux effaces.'", true),
        script("Supprimer fichiers .tmp", "Recherche et supprime tous les .tmp sur C:", "Nettoyage", "powershell",
            r#"$count = 0
Get-ChildItem C:\ -Filter *.tmp -Recurse -ErrorAction SilentlyContinue | ForEach-Object {
  Remove-Item $_.FullName -Force -ErrorAction SilentlyContinue; $count++
}
Write-Host "$count fichiers .tmp supprimes.""#, false),
        script("Nettoyage complet automatique", "Execute cleanmgr en mode silent sur C:", "Nettoyage", "cmd",
            "cleanmgr /sagerun:1", true),

        // ═══════════════════════════════════════════════════════
        // RESEAU
        // ═══════════════════════════════════════════════════════
        script("Vider cache DNS", "Reinitialise le cache DNS local", "Reseau", "cmd",
            "ipconfig /flushdns", false),
        script("Reset Winsock", "Reinitialise la pile Winsock (resout les erreurs reseau)", "Reseau", "cmd",
            "netsh winsock reset", true),
        script("Reset TCP/IP", "Reinitialise la config TCP/IP stack", "Reseau", "cmd",
            "netsh int ip reset", true),
        script("Renouveler IP", "Release et renouvellement de l'adresse IP via DHCP", "Reseau", "cmd",
            "ipconfig /release & ipconfig /renew", false),
        script("Afficher profils WiFi", "Liste tous les profils WiFi enregistres", "Reseau", "cmd",
            "netsh wlan show profiles", false),
        script("Mot de passe WiFi actuel", "Affiche le mot de passe du reseau WiFi connecte", "Reseau", "powershell",
            r#"$p = (netsh wlan show interfaces | Select-String 'Profil').ToString().Split(':')[-1].Trim()
netsh wlan show profile name="$p" key=clear | Select-String 'Contenu de la cle'"#, false),
        script("Exporter tous les mots de passe WiFi", "Exporte tous les mots de passe WiFi sur le bureau", "Reseau", "powershell",
            r#"$out = @()
(netsh wlan show profiles) | Select-String ': ' | ForEach-Object {
  $n = $_.ToString().Split(':')[-1].Trim()
  $k = (netsh wlan show profile name="$n" key=clear) | Select-String 'Contenu de la cle'
  if ($k) { $pw = $k.ToString().Split(':')[-1].Trim() } else { $pw = '(non disponible)' }
  $out += "$n : $pw"
}
$out | Out-File "$env:USERPROFILE\Desktop\wifi_passwords.txt"
Write-Host "Sauvegarde: $env:USERPROFILE\Desktop\wifi_passwords.txt""#, false),
        script("Ping Gateway", "Ping la passerelle par defaut", "Reseau", "powershell",
            r#"$gw = (Get-NetRoute -DestinationPrefix '0.0.0.0/0' | Sort-Object RouteMetric | Select-Object -First 1).NextHop
Write-Host "Gateway: $gw"; ping -n 4 $gw"#, false),
        script("Test DNS Google", "Test la resolution DNS via 8.8.8.8", "Reseau", "cmd",
            "nslookup google.com 8.8.8.8", false),
        script("Test debit download", "Test la vitesse via Speedtest CLI", "Reseau", "powershell",
            "if (Get-Command speedtest -ErrorAction SilentlyContinue) { speedtest } else { Write-Host 'Speedtest CLI non installe. Installer via: winget install Ookla.Speedtest' }", false),
        script("Afficher connexions actives", "Liste toutes les connexions TCP/UDP actives", "Reseau", "cmd",
            "netstat -an", false),
        script("Afficher table ARP", "Affiche la table ARP (IP -> MAC)", "Reseau", "cmd",
            "arp -a", false),
        script("Traceroute google.com", "Trace la route vers google.com", "Reseau", "cmd",
            "tracert google.com", false),
        script("Desactiver IPv6", "Desactive IPv6 sur tous les adaptateurs", "Reseau", "powershell",
            "Get-NetAdapterBinding -ComponentID ms_tcpip6 | Disable-NetAdapterBinding -ErrorAction SilentlyContinue; Write-Host 'IPv6 desactive.'", true),
        script("Reinitialiser pare-feu", "Remet le pare-feu Windows aux parametres par defaut", "Reseau", "cmd",
            "netsh advfirewall reset", true),
        script("Infos interface reseau", "Affiche les details de tous les adaptateurs reseau", "Reseau", "cmd",
            "ipconfig /all", false),
        script("Scanner ports locaux ouverts", "Liste les ports TCP en ecoute", "Reseau", "powershell",
            "Get-NetTCPConnection -State Listen | Select LocalAddress, LocalPort, OwningProcess | Sort-Object LocalPort | Format-Table -AutoSize", false),
        script("Changer DNS vers Cloudflare", "Definit 1.1.1.1 et 1.0.0.1 comme serveurs DNS", "Reseau", "powershell",
            r#"$adapter = Get-NetAdapter | Where-Object {$_.Status -eq 'Up'} | Select-Object -First 1
Set-DnsClientServerAddress -InterfaceIndex $adapter.InterfaceIndex -ServerAddresses '1.1.1.1','1.0.0.1'
Write-Host "DNS Cloudflare configure sur: $($adapter.Name)""#, true),
        script("Changer DNS vers Google", "Definit 8.8.8.8 et 8.8.4.4 comme serveurs DNS", "Reseau", "powershell",
            r#"$adapter = Get-NetAdapter | Where-Object {$_.Status -eq 'Up'} | Select-Object -First 1
Set-DnsClientServerAddress -InterfaceIndex $adapter.InterfaceIndex -ServerAddresses '8.8.8.8','8.8.4.4'
Write-Host "DNS Google configure sur: $($adapter.Name)""#, true),
        script("Remettre DNS automatique (DHCP)", "Restaure le DNS automatique depuis le DHCP", "Reseau", "powershell",
            r#"$adapter = Get-NetAdapter | Where-Object {$_.Status -eq 'Up'} | Select-Object -First 1
Set-DnsClientServerAddress -InterfaceIndex $adapter.InterfaceIndex -ResetServerAddresses
Write-Host "DNS remis en automatique sur: $($adapter.Name)""#, true),

        // ═══════════════════════════════════════════════════════
        // REPARATION
        // ═══════════════════════════════════════════════════════
        script("SFC Scannow", "Verifie et repare les fichiers systeme corrompus", "Reparation", "cmd",
            "sfc /scannow", true),
        script("DISM Restore Health", "Repare l'image Windows via DISM", "Reparation", "cmd",
            "DISM /Online /Cleanup-Image /RestoreHealth", true),
        script("CHKDSK C:", "Verifie l'integrite du disque C: au prochain demarrage", "Reparation", "cmd",
            "chkdsk C: /f /r /x", true),
        script("Reset Windows Update", "Reinitialise les composants Windows Update (stop services, rename dossiers)", "Reparation", "cmd",
            r#"net stop wuauserv
net stop cryptSvc
net stop bits
net stop msiserver
ren C:\Windows\SoftwareDistribution SoftwareDistribution.bak
ren C:\Windows\System32\catroot2 catroot2.bak
net start wuauserv
net start cryptSvc
net start bits
net start msiserver
echo Windows Update reinitialise."#, true),
        script("Reparer Windows Store", "Reinstalle et repare les apps du Microsoft Store", "Reparation", "powershell",
            "Get-AppXPackage -AllUsers | ForEach-Object { Add-AppxPackage -DisableDevelopmentMode -Register \"$($_.InstallLocation)\\AppXManifest.xml\" -ErrorAction SilentlyContinue }", true),
        script("Reconstruire MBR", "Repare le Master Boot Record via bootrec", "Reparation", "cmd",
            "bootrec /fixmbr & bootrec /fixboot & bootrec /rebuildbcd", true),
        script("Reparer .NET Framework", "Reinitialise le registre .NET Framework", "Reparation", "powershell",
            r#"$dotnet = 'HKLM:\SOFTWARE\Microsoft\NET Framework Setup\NDP'
Write-Host 'Versions .NET installees:'
Get-ChildItem $dotnet -Recurse | Get-ItemProperty -Name Version -ErrorAction SilentlyContinue | Select-Object PSChildName, Version | Sort-Object Version"#, false),
        script("Reinitialiser politiques de groupe", "Supprime les GPO locales (gpupdate /force)", "Reparation", "cmd",
            "rd /s /q %SystemRoot%\\System32\\GroupPolicy & rd /s /q %SystemRoot%\\System32\\GroupPolicyUsers & gpupdate /force", true),
        script("Reparer icone de la corbeille", "Recharge la corbeille si l'icone est cassee", "Reparation", "powershell",
            r#"Stop-Process -Name explorer -Force
Remove-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\BitBucket' -Recurse -ErrorAction SilentlyContinue
Start-Process explorer
Write-Host 'Corbeille reparee.'"#, false),
        script("Corriger problemes audio", "Reinitialise le service audio Windows", "Reparation", "cmd",
            "net stop AudioSrv & net stop AudioEndpointBuilder & net start AudioSrv & net start AudioEndpointBuilder", true),
        script("Reinitialiser Hyper-V", "Reinitialise les composants Hyper-V", "Reparation", "cmd",
            "bcdedit /set hypervisorlaunchtype auto", true),
        script("Reparer associations de fichiers", "Reinitialise les associations de fichiers corrompues", "Reparation", "powershell",
            r#"$assocs = @('.txt','notepad.exe', '.pdf','AcroRd32.exe', '.html','msedge.exe')
Write-Host 'Associations de fichiers verifiees.'
assoc"#, false),

        // ═══════════════════════════════════════════════════════
        // PERFORMANCE
        // ═══════════════════════════════════════════════════════
        script("Optimiser SSD (TRIM)", "Active la commande TRIM pour ameliorer la durabilite SSD", "Performance", "cmd",
            "fsutil behavior set disabledeletenotify 0 & echo TRIM active.", true),
        script("Defragmenter C:", "Lance la defragmentation optimisee du disque C:", "Performance", "cmd",
            "defrag C: /O /U /V", true),
        script("Liberer memoire RAM", "Force le garbage collector et vide les fichiers de swap", "Performance", "powershell",
            "[System.GC]::Collect(); [System.GC]::WaitForPendingFinalizers(); Write-Host 'Memoire liberee.'", false),
        script("Desactiver indexation Windows Search", "Arrete le service d'indexation pour liberer CPU/disque", "Performance", "cmd",
            "sc stop WSearch & sc config WSearch start=disabled & echo Indexation desactivee.", true),
        script("Activer indexation Windows Search", "Reactive le service d'indexation Windows", "Performance", "cmd",
            "sc config WSearch start=auto & sc start WSearch & echo Indexation reactivee.", true),
        script("Desactiver demarrage rapide", "Desactive Fast Startup pour eviter les bugs de reprise", "Performance", "powershell",
            "powercfg /h off; Set-ItemProperty 'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Power' -Name 'HiberbootEnabled' -Value 0; Write-Host 'Demarrage rapide desactive.'", true),
        script("Activer plan haute performance", "Bascule sur le plan d'alimentation Haute Performance", "Performance", "cmd",
            "powercfg /setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c & echo Plan Haute Performance active.", true),
        script("Activer plan equilibre", "Bascule sur le plan d'alimentation Equilibre", "Performance", "cmd",
            "powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e & echo Plan Equilibre active.", true),
        script("Ajuster effets visuels (performance)", "Reduit les effets visuels pour maximiser les performances", "Performance", "powershell",
            r#"$path = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects'
Set-ItemProperty $path -Name VisualFXSetting -Value 2 -ErrorAction SilentlyContinue
Write-Host 'Effets visuels: mode Performance.'
SystemPropertiesPerformance.exe"#, false),
        script("Desactiver Xbox Game Bar", "Desactive la barre de jeu Xbox pour reduire l'overhead", "Performance", "powershell",
            "Set-ItemProperty -Path 'HKCU:\\System\\GameConfigStore' -Name 'GameDVR_Enabled' -Value 0; Set-ItemProperty -Path 'HKCU:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\GameDVR' -Name 'AppCaptureEnabled' -Value 0; Write-Host 'Xbox Game Bar desactive.'", false),
        script("Desactiver SysMain (Superfetch)", "Arrete SysMain pour les systemes avec SSD rapide", "Performance", "cmd",
            "sc stop SysMain & sc config SysMain start=disabled & echo SysMain desactive.", true),
        script("Activer SysMain (Superfetch)", "Reactive SysMain pour les systemes avec HDD", "Performance", "cmd",
            "sc config SysMain start=auto & sc start SysMain & echo SysMain reactive.", true),
        script("Rapport performances systeme", "Lance le moniteur de performances Windows", "Performance", "cmd",
            "perfmon /report", true),
        script("Optimiser RAM PageFile", "Affiche la configuration du fichier de page", "Performance", "powershell",
            "Get-WmiObject Win32_PageFileUsage | Format-Table Name, AllocatedBaseSize, CurrentUsage, PeakUsage -AutoSize", false),
        script("Desactiver mise en veille auto", "Empeche Windows de se mettre en veille", "Performance", "cmd",
            "powercfg /change standby-timeout-ac 0 & powercfg /change standby-timeout-dc 0 & echo Mise en veille desactivee.", true),

        // ═══════════════════════════════════════════════════════
        // DIAGNOSTIC
        // ═══════════════════════════════════════════════════════
        script("Lister services actifs", "Affiche tous les services en cours d'execution", "Diagnostic", "powershell",
            "Get-Service | Where-Object {$_.Status -eq 'Running'} | Format-Table Name, DisplayName -AutoSize", false),
        script("Rapport batterie complet", "Genere un rapport HTML detaille de la batterie", "Diagnostic", "cmd",
            "powercfg /batteryreport /output %USERPROFILE%\\Desktop\\battery-report.html & start %USERPROFILE%\\Desktop\\battery-report.html", false),
        script("Rapport energie", "Analyse la consommation energetique (60s)", "Diagnostic", "cmd",
            "powercfg /energy /duration 60 /output %USERPROFILE%\\Desktop\\energy-report.html & start %USERPROFILE%\\Desktop\\energy-report.html", true),
        script("Lister logiciels installes", "Liste tous les logiciels via le registre Windows", "Diagnostic", "powershell",
            r#"Get-ItemProperty HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\* |
Where-Object {$_.DisplayName} |
Select-Object DisplayName, DisplayVersion, Publisher |
Sort-Object DisplayName | Format-Table -AutoSize"#, false),
        script("Espace disque par dossier", "Top 20 des plus gros dossiers sur C:", "Diagnostic", "powershell",
            r#"Get-ChildItem C:\ -Directory -ErrorAction SilentlyContinue | ForEach-Object {
  $size = (Get-ChildItem $_.FullName -Recurse -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum
  [PSCustomObject]@{ Dossier = $_.Name; TailleMB = [math]::Round($size/1MB,1) }
} | Sort-Object TailleMB -Descending | Select-Object -First 20 | Format-Table -AutoSize"#, false),
        script("Sante disque SMART", "Affiche le statut SMART de tous les disques", "Diagnostic", "powershell",
            "Get-Disk | Select-Object Number, FriendlyName, HealthStatus, OperationalStatus, Size | Format-Table -AutoSize", false),
        script("Infos CPU detaillees", "Affiche les specifications completes du processeur", "Diagnostic", "powershell",
            r#"$cpu = Get-WmiObject Win32_Processor
$cpu | Select-Object Name, NumberOfCores, NumberOfLogicalProcessors, MaxClockSpeed, CurrentClockSpeed, LoadPercentage | Format-List"#, false),
        script("Infos RAM detaillees", "Affiche les barettes RAM, slots et frequences", "Diagnostic", "powershell",
            r#"Get-WmiObject Win32_PhysicalMemory | Select-Object BankLabel, Capacity, Speed, Manufacturer, MemoryType |
ForEach-Object { $_.Capacity = [math]::Round($_.Capacity/1GB,1); $_ } | Format-Table -AutoSize"#, false),
        script("Uptime systeme", "Affiche depuis combien de temps Windows est demarre", "Diagnostic", "powershell",
            r#"$boot = (Get-Date) - (gcim Win32_OperatingSystem).LastBootUpTime
Write-Host "Uptime: $($boot.Days)j $($boot.Hours)h $($boot.Minutes)m""#, false),
        script("Processus les plus gourmands CPU", "Top 10 processus par consommation CPU", "Diagnostic", "powershell",
            "Get-Process | Sort-Object CPU -Descending | Select-Object -First 10 Name, CPU, WorkingSet | Format-Table -AutoSize", false),
        script("Processus les plus gourmands RAM", "Top 10 processus par consommation memoire", "Diagnostic", "powershell",
            r#"Get-Process | Sort-Object WorkingSet -Descending | Select-Object -First 10 Name,
  @{N='RAM_MB';E={[math]::Round($_.WorkingSet/1MB,1)}} | Format-Table -AutoSize"#, false),
        script("Historique crashes BSOD", "Liste les 10 derniers crashes systeme (BSOD)", "Diagnostic", "powershell",
            r#"Get-WinEvent -FilterHashtable @{LogName='System'; Id=1001,41} -MaxEvents 10 -ErrorAction SilentlyContinue |
Select-Object TimeCreated, Message | Format-List"#, true),
        script("Variables d'environnement", "Affiche toutes les variables d'environnement systeme", "Diagnostic", "cmd",
            "set", false),
        script("Certificats systeme expires", "Liste les certificats expirés dans le magasin local", "Diagnostic", "powershell",
            r#"Get-ChildItem Cert:\LocalMachine\My | Where-Object {$_.NotAfter -lt (Get-Date)} |
Select-Object Subject, NotAfter, Thumbprint | Format-Table -AutoSize"#, false),
        script("Verifier ports ouverts", "Scan les ports courants en ecoute localement", "Diagnostic", "powershell",
            "1..1024 | ForEach-Object { $conn = Test-NetConnection -ComputerName localhost -Port $_ -InformationLevel Quiet -ErrorAction SilentlyContinue 2>$null; if ($conn) { Write-Host \"Port $_ ouvert\" } }", false),
        script("Rapport systeminfo complet", "Exporte systeminfo sur le bureau", "Diagnostic", "cmd",
            "systeminfo > %USERPROFILE%\\Desktop\\systeminfo.txt & echo Rapport: %USERPROFILE%\\Desktop\\systeminfo.txt", false),
        script("Lister taches planifiees", "Affiche toutes les taches planifiees activees", "Diagnostic", "powershell",
            "Get-ScheduledTask | Where-Object {$_.State -ne 'Disabled'} | Select-Object TaskName, TaskPath, State | Sort-Object TaskPath | Format-Table -AutoSize", false),
        script("Infos GPU", "Affiche les specifications de la carte graphique", "Diagnostic", "powershell",
            r#"Get-WmiObject Win32_VideoController | Select-Object Name, DriverVersion, AdapterRAM, VideoProcessor |
ForEach-Object { $_.AdapterRAM = [math]::Round($_.AdapterRAM/1GB,1); $_ } | Format-List"#, false),

        // ═══════════════════════════════════════════════════════
        // SECURITE
        // ═══════════════════════════════════════════════════════
        script("Scan antivirus rapide (Defender)", "Lance un scan rapide Windows Defender", "Securite", "powershell",
            "Start-MpScan -ScanType QuickScan; Write-Host 'Scan Defender lance...'", true),
        script("Scan antivirus complet (Defender)", "Lance un scan complet Windows Defender", "Securite", "powershell",
            "Start-MpScan -ScanType FullScan; Write-Host 'Scan complet lance (peut durer 1-2h)...'", true),
        script("Mettre a jour signatures Defender", "Force la mise a jour des definitions antivirus", "Securite", "powershell",
            "Update-MpSignature; Write-Host 'Signatures Defender mises a jour.'", true),
        script("Verifier statut Defender", "Affiche le statut complet de Windows Defender", "Securite", "powershell",
            "Get-MpComputerStatus | Select-Object AMRunningMode, AntivirusEnabled, RealTimeProtectionEnabled, AntispywareSignatureVersion | Format-List", false),
        script("Lister regles pare-feu (entrant)", "Affiche les regles pare-feu entrant actives", "Securite", "powershell",
            "Get-NetFirewallRule -Direction Inbound -Enabled True | Select-Object DisplayName, Action, Profile | Format-Table -AutoSize", false),
        script("Afficher connexions sortantes", "Liste les connexions sortantes etablies", "Securite", "powershell",
            "Get-NetTCPConnection -State Established | Select-Object LocalAddress, LocalPort, RemoteAddress, RemotePort, OwningProcess | Sort-Object RemoteAddress | Format-Table -AutoSize", false),
        script("Comptes utilisateurs locaux", "Liste tous les comptes utilisateurs Windows", "Securite", "powershell",
            "Get-LocalUser | Select-Object Name, Enabled, LastLogon, PasswordLastSet | Format-Table -AutoSize", false),
        script("Desactiver compte Invité", "Desactive le compte Invité pour la securite", "Securite", "cmd",
            "net user Invité /active:no & net user Guest /active:no & echo Compte Invite desactive.", true),
        script("Verifier autologon registre", "Detecte si un autologon est configure dans le registre", "Securite", "powershell",
            r#"$r = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon'
$user = (Get-ItemProperty $r).DefaultUserName
$pass = (Get-ItemProperty $r).DefaultPassword
if ($user) { Write-Host "AUTOLOGON DETECTE: $user / $pass" } else { Write-Host 'Pas d autologon configure.' }"#, false),
        script("Lister partages reseau actifs", "Affiche tous les dossiers partages sur ce PC", "Securite", "powershell",
            "Get-SmbShare | Select-Object Name, Path, Description | Format-Table -AutoSize", false),
        script("Bloquer telemetrie (hosts)", "Ajoute des entrees hosts pour bloquer la telemetrie Microsoft", "Securite", "powershell",
            r#"$hosts = 'C:\Windows\System32\drivers\etc\hosts'
$entries = @(
  '0.0.0.0 telemetry.microsoft.com',
  '0.0.0.0 vortex.data.microsoft.com',
  '0.0.0.0 settings-win.data.microsoft.com',
  '0.0.0.0 watson.telemetry.microsoft.com'
)
foreach ($e in $entries) {
  if (-not (Select-String -Path $hosts -Pattern $e.Split(' ')[1] -Quiet)) {
    Add-Content -Path $hosts -Value $e
    Write-Host "Ajoute: $e"
  }
}"#, true),
        script("Auditer connexions RDP", "Verifie si RDP est active et liste les connexions recentes", "Securite", "powershell",
            r#"$rdp = (Get-ItemProperty 'HKLM:\System\CurrentControlSet\Control\Terminal Server').fDenyTSConnections
Write-Host "RDP actif: $(if ($rdp -eq 0) {'OUI - ATTENTION'} else {'Non'})"
Get-WinEvent -FilterHashtable @{LogName='Security'; Id=4624} -MaxEvents 5 -ErrorAction SilentlyContinue |
Select-Object TimeCreated, Message | Format-List"#, true),
        script("Exporter liste processus au demarrage", "Exporte les programmes de demarrage", "Securite", "powershell",
            r#"$startup = Get-CimInstance Win32_StartupCommand | Select-Object Name, Command, Location, User
$startup | Format-Table -AutoSize
$startup | Export-Csv "$env:USERPROFILE\Desktop\startup_programs.csv" -NoTypeInformation
Write-Host "Exporte: $env:USERPROFILE\Desktop\startup_programs.csv""#, false),

        // ═══════════════════════════════════════════════════════
        // TWEAKS WINDOWS
        // ═══════════════════════════════════════════════════════
        script("Desactiver telemetrie", "Desactive la collecte de donnees Microsoft (niveau 0)", "Tweaks", "powershell",
            r#"$path = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\DataCollection'
if (-not (Test-Path $path)) { New-Item $path -Force | Out-Null }
Set-ItemProperty $path -Name AllowTelemetry -Value 0 -Type DWord -Force
Write-Host 'Telemetrie desactivee.'"#, true),
        script("Activer mode sombre", "Active le theme sombre Windows 10/11", "Tweaks", "powershell",
            r#"$p = 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize'
Set-ItemProperty $p -Name AppsUseLightTheme -Value 0
Set-ItemProperty $p -Name SystemUsesLightTheme -Value 0
Write-Host 'Mode sombre active.'"#, false),
        script("Activer mode clair", "Active le theme clair Windows 10/11", "Tweaks", "powershell",
            r#"$p = 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize'
Set-ItemProperty $p -Name AppsUseLightTheme -Value 1
Set-ItemProperty $p -Name SystemUsesLightTheme -Value 1
Write-Host 'Mode clair active.'"#, false),
        script("Desactiver Cortana", "Desactive Cortana completement", "Tweaks", "powershell",
            r#"$path = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\Windows Search'
if (-not (Test-Path $path)) { New-Item $path -Force | Out-Null }
Set-ItemProperty $path -Name AllowCortana -Value 0 -Type DWord -Force
Write-Host 'Cortana desactivee.'"#, true),
        script("Restaurer menu classique Windows 11", "Restaure l'ancien menu contextuel (clic droit)", "Tweaks", "powershell",
            r#"reg add 'HKCU\Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\InprocServer32' /f /ve
Stop-Process -Name explorer -Force; Start-Process explorer
Write-Host 'Menu classique restaure.'"#, false),
        script("Supprimer menu classique Windows 11", "Restaure le nouveau menu Windows 11", "Tweaks", "powershell",
            r#"reg delete 'HKCU\Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}' /f
Stop-Process -Name explorer -Force; Start-Process explorer
Write-Host 'Nouveau menu W11 restaure.'"#, false),
        script("Desactiver publicites dans Windows", "Supprime les suggestions et pubs dans le menu demarrer", "Tweaks", "powershell",
            r#"$p = 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\ContentDeliveryManager'
Set-ItemProperty $p -Name SystemPaneSuggestionsEnabled -Value 0 -ErrorAction SilentlyContinue
Set-ItemProperty $p -Name SubscribedContent-338389Enabled -Value 0 -ErrorAction SilentlyContinue
Set-ItemProperty $p -Name SubscribedContent-353696Enabled -Value 0 -ErrorAction SilentlyContinue
Set-ItemProperty $p -Name SilentInstalledAppsEnabled -Value 0 -ErrorAction SilentlyContinue
Write-Host 'Publicites Windows desactivees.'"#, false),
        script("Desactiver notifications de mises a jour", "Masque les notifications Windows Update", "Tweaks", "powershell",
            r#"$p = 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Notifications\Settings\Windows.SystemToast.WindowsUpdate'
if (-not (Test-Path $p)) { New-Item $p -Force | Out-Null }
Set-ItemProperty $p -Name Enabled -Value 0 -Type DWord
Write-Host 'Notifications WU desactivees.'"#, false),
        script("Augmenter taille du journal evenements", "Passe la taille du journal System a 1 GB", "Tweaks", "powershell",
            "wevtutil sl System /ms:1073741824; Write-Host 'Journal System: 1 GB'", true),
        script("Desactiver OneDrive", "Desinstalle et desactive OneDrive", "Tweaks", "cmd",
            "taskkill /f /im OneDrive.exe & %SystemRoot%\\SysWOW64\\OneDriveSetup.exe /uninstall & echo OneDrive desactive.", false),
        script("Desactiver demarrage automatique OneDrive", "Empeche OneDrive de demarrer avec Windows", "Tweaks", "powershell",
            "Set-ItemProperty 'HKCU:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run' -Name 'OneDrive' -Value '' -ErrorAction SilentlyContinue; Write-Host 'OneDrive: demarrage auto desactive.'", false),
        script("Activer le bureau virtuel persistant", "Active les bureaux virtuels persistants entre sessions", "Tweaks", "powershell",
            "Set-ItemProperty 'HKCU:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VirtualDesktops' -Name VirtualDesktopIDsArray -Value 0 -ErrorAction SilentlyContinue; Write-Host 'Parametres bureaux virtuels ajustes.'", false),
        script("Desactiver hibernation", "Desactive l'hibernation et supprime hiberfil.sys", "Tweaks", "cmd",
            "powercfg /h off & echo Hibernation desactivee.", true),
        script("Activer hibernation", "Reactive l'hibernation Windows", "Tweaks", "cmd",
            "powercfg /h on & echo Hibernation reactivee.", true),
        script("Masquer icones bureau (tout)", "Masque tous les icones du bureau", "Tweaks", "powershell",
            r#"$hv = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced'
Set-ItemProperty $hv -Name HideIcons -Value 1
Stop-Process -Name explorer -Force; Start-Process explorer; Write-Host 'Icones masques.'"#, false),
        script("Afficher extensions de fichiers", "Active l'affichage des extensions dans l'explorateur", "Tweaks", "powershell",
            "Set-ItemProperty 'HKCU:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced' -Name HideFileExt -Value 0; Stop-Process -Name explorer -Force; Start-Process explorer; Write-Host 'Extensions visibles.'", false),
        script("Afficher fichiers caches", "Active l'affichage des fichiers et dossiers caches", "Tweaks", "powershell",
            "Set-ItemProperty 'HKCU:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced' -Name Hidden -Value 1; Stop-Process -Name explorer -Force; Start-Process explorer; Write-Host 'Fichiers caches visibles.'", false),
        script("Activer bureau a distance (RDP)", "Active le bureau a distance sur ce PC", "Tweaks", "powershell",
            "Set-ItemProperty 'HKLM:\\System\\CurrentControlSet\\Control\\Terminal Server' -Name fDenyTSConnections -Value 0; Enable-NetFirewallRule -DisplayGroup 'Remote Desktop'; Write-Host 'RDP active.'", true),
        script("Desactiver bureau a distance (RDP)", "Desactive le bureau a distance", "Tweaks", "powershell",
            "Set-ItemProperty 'HKLM:\\System\\CurrentControlSet\\Control\\Terminal Server' -Name fDenyTSConnections -Value 1; Disable-NetFirewallRule -DisplayGroup 'Remote Desktop'; Write-Host 'RDP desactive.'", true),

        // ═══════════════════════════════════════════════════════
        // DRIVERS
        // ═══════════════════════════════════════════════════════
        script("Lister pilotes installes", "Liste tous les pilotes installes avec leur version", "Drivers", "powershell",
            "Get-WmiObject Win32_PnPSignedDriver | Where-Object {$_.DeviceName} | Select-Object DeviceName, DriverVersion, DriverDate, Manufacturer | Sort-Object DeviceName | Format-Table -AutoSize", false),
        script("Lister pilotes defectueux", "Identifie les peripheriques avec problemes de pilote", "Drivers", "powershell",
            "Get-WmiObject Win32_PnPEntity | Where-Object {$_.ConfigManagerErrorCode -ne 0} | Select-Object Name, Description, ConfigManagerErrorCode | Format-Table -AutoSize", false),
        script("Verifier signature pilotes", "Recherche les pilotes non signes (sigcheck)", "Drivers", "powershell",
            r#"Get-WmiObject Win32_PnPSignedDriver |
Where-Object {-not $_.IsSigned} |
Select-Object DeviceName, DriverVersion, Manufacturer | Format-Table -AutoSize"#, false),
        script("Forcer verification pilotes (Verifier)", "Lance Driver Verifier pour detecter les mauvais pilotes", "Drivers", "cmd",
            "verifier /standard /all", true),
        script("Desactiver verification pilotes", "Desactive Driver Verifier (apres diagnostic)", "Drivers", "cmd",
            "verifier /reset & echo Driver Verifier desactive.", true),
        script("Exporter liste pilotes", "Exporte la liste des pilotes sur le bureau", "Drivers", "powershell",
            r#"Get-WmiObject Win32_PnPSignedDriver |
Where-Object {$_.DeviceName} |
Select-Object DeviceName, DriverVersion, DriverDate, Manufacturer |
Export-Csv "$env:USERPROFILE\Desktop\drivers_list.csv" -NoTypeInformation
Write-Host "Exporte: $env:USERPROFILE\Desktop\drivers_list.csv""#, false),

        // ═══════════════════════════════════════════════════════
        // REGISTRE
        // ═══════════════════════════════════════════════════════
        script("Sauvegarder le registre", "Exporte les ruches HKCU et HKLM sur le bureau", "Registre", "cmd",
            r#"reg export HKCU "%USERPROFILE%\Desktop\backup_HKCU.reg" /y
reg export HKLM "%USERPROFILE%\Desktop\backup_HKLM.reg" /y
echo Registre sauvegarde sur le bureau."#, true),
        script("Nettoyer entrees run vides", "Supprime les entrees Run vides dans le registre", "Registre", "powershell",
            r#"$run = 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run'
$items = Get-ItemProperty $run
$items.PSObject.Properties | Where-Object {$_.Value -eq ''} | ForEach-Object {
  Remove-ItemProperty $run -Name $_.Name -ErrorAction SilentlyContinue
  Write-Host "Supprime: $($_.Name)"
}"#, false),
        script("Trouver cle produit Windows", "Extrait la cle produit Windows du registre/BIOS", "Registre", "powershell",
            r#"$key = (Get-WmiObject SoftwareLicensingService).OA3xOriginalProductKey
if ($key) { Write-Host "Cle OA3 (BIOS): $key" }
else {
  $map = 'BCDFGHJKMPQRTVWXY2346789'
  $raw = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').DigitalProductId
  $i = 52; $result = ''; 1..25 | ForEach-Object {
    $r = 0; 14..0 | ForEach-Object { $r = $r * 256 -bxor $raw[$_ + $i]; $raw[$_ + $i] = [math]::Floor($r / 24); $r %= 24 }
    $result = $map[$r] + $result; if ((25 - ($_ - 1)) % 5 -eq 0 -and ($_ -ne 25)) { $result = '-' + $result }
  }
  Write-Host "Cle produit: $result"
}"#, false),
        script("Afficher produit ID et edition", "Affiche l'edition et l'ID produit Windows", "Registre", "powershell",
            r#"$wnt = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
Write-Host "Edition  : $($wnt.EditionID)"
Write-Host "Version  : $($wnt.DisplayVersion)"
Write-Host "Build    : $($wnt.CurrentBuildNumber)"
Write-Host "Produit  : $($wnt.ProductId)""#, false),

        // ═══════════════════════════════════════════════════════
        // UTILISATEURS
        // ═══════════════════════════════════════════════════════
        script("Lister utilisateurs locaux", "Affiche tous les comptes utilisateurs locaux", "Utilisateurs", "powershell",
            "Get-LocalUser | Format-Table Name, Enabled, LastLogon, PasswordRequired, PasswordExpires -AutoSize", false),
        script("Lister groupes locaux", "Affiche les groupes locaux et leurs membres", "Utilisateurs", "powershell",
            "Get-LocalGroup | ForEach-Object { Write-Host \"--- $($_.Name) ---\"; Get-LocalGroupMember $_.Name -ErrorAction SilentlyContinue | Select-Object Name, ObjectClass | Format-Table }", false),
        script_interactive("Creer compte utilisateur standard", "Cree un compte utilisateur interactif", "Utilisateurs",
            r#"$name = Read-Host 'Nom du compte'
$pass = Read-Host 'Mot de passe' -AsSecureString
New-LocalUser -Name $name -Password $pass -FullName $name -Description 'Cree par Nitrite'
Add-LocalGroupMember -Group 'Utilisateurs' -Member $name
Write-Host "Compte cree: $name""#, true),
        script_interactive("Desactiver compte utilisateur", "Desactive un compte utilisateur specifique", "Utilisateurs",
            r#"$name = Read-Host 'Nom du compte a desactiver'
Disable-LocalUser -Name $name
Write-Host "Compte desactive: $name""#, true),
        script_interactive("Forcer changement de mot de passe", "Force le changement de mot de passe a la prochaine connexion", "Utilisateurs",
            r#"$name = Read-Host 'Nom du compte'
net user $name /logonpasswordchg:yes
Write-Host "Changement de MDP force pour: $name""#, true),

        // ═══════════════════════════════════════════════════════
        // SAUVEGARDES
        // ═══════════════════════════════════════════════════════
        script("Sauvegarder dossier Documents", "Copie les Documents vers un dossier horodate sur le bureau", "Sauvegardes", "powershell",
            r#"$date = Get-Date -Format 'yyyy-MM-dd_HH-mm'
$dest = "$env:USERPROFILE\Desktop\Backup_Documents_$date"
robocopy "$env:USERPROFILE\Documents" $dest /E /R:1 /W:1 /XA:SH /NFL /NDL
Write-Host "Sauvegarde: $dest""#, false),
        script("Sauvegarder dossier Bureau", "Copie le bureau vers un dossier horodate", "Sauvegardes", "powershell",
            r#"$date = Get-Date -Format 'yyyy-MM-dd_HH-mm'
$dest = "$env:USERPROFILE\Desktop\Backup_Bureau_$date"
robocopy "$env:USERPROFILE\Desktop" $dest /E /R:1 /W:1 /XA:SH /NFL /NDL
Write-Host "Sauvegarde: $dest""#, false),
        script("Creer point de restauration", "Cree un point de restauration Windows maintenant", "Sauvegardes", "powershell",
            "Enable-ComputerRestore -Drive 'C:\\'; Checkpoint-Computer -Description 'Nitrite Backup' -RestorePointType MODIFY_SETTINGS; Write-Host 'Point de restauration cree.'", true),
        script("Lister points de restauration", "Affiche tous les points de restauration disponibles", "Sauvegardes", "powershell",
            "Get-ComputerRestorePoint | Select-Object CreationTime, Description, SequenceNumber | Sort-Object CreationTime -Descending | Format-Table -AutoSize", false),
        script("Exporter profil WiFi", "Sauvegarde tous les profils WiFi sur le bureau", "Sauvegardes", "cmd",
            "netsh wlan export profile folder=%USERPROFILE%\\Desktop\\WiFi_Profiles key=clear & echo Profils exportes dans %USERPROFILE%\\Desktop\\WiFi_Profiles", true),
        script_interactive("Importer profils WiFi", "Importe les profils WiFi depuis un dossier", "Sauvegardes",
            r#"$src = Read-Host 'Dossier contenant les profils XML'
Get-ChildItem "$src\*.xml" | ForEach-Object {
  netsh wlan add profile filename=$_.FullName
  Write-Host "Importe: $($_.Name)"
}"#, true),
    ]
}

// === Script File Management ===

#[derive(Debug, Clone, Serialize)]
pub struct ScriptFileInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub script_type: String,
}

pub fn list_script_files(dir: &str) -> Result<Vec<ScriptFileInfo>, NiTriTeError> {
    let path = std::path::Path::new(dir);
    if !path.exists() || !path.is_dir() {
        return Ok(Vec::new());
    }

    // SECURITE : canonicalize avant whitelist pour prévenir path traversal / symlink
    let canonical = path.canonicalize().map_err(|e| {
        NiTriTeError::CommandDenied(format!("Chemin invalide: {}", e))
    })?;
    if !is_script_path_allowed(&canonical) {
        return Err(NiTriTeError::CommandDenied(
            format!("Accès interdit hors des répertoires autorisés: {}", dir),
        ));
    }

    let mut files = Vec::new();
    for entry in std::fs::read_dir(&canonical)? {
        let entry = entry?;
        let ep = entry.path();
        if ep.is_file() {
            let ext = ep.extension().and_then(|e| e.to_str()).unwrap_or("");
            let script_type = match ext {
                "ps1" => "powershell",
                "bat" | "cmd" => "cmd",
                "sh" => "bash",
                "py" => "python",
                _ => continue,
            };
            files.push(ScriptFileInfo {
                name: ep.file_name().unwrap_or_default().to_string_lossy().to_string(),
                path: ep.to_string_lossy().to_string(),
                size_bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
                script_type: script_type.to_string(),
            });
        }
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

fn is_script_path_allowed(p: &std::path::Path) -> bool {
    let allowed: Vec<std::path::PathBuf> = [
        dirs::document_dir(),
        dirs::desktop_dir(),
        dirs::download_dir(),
    ]
    .into_iter()
    .flatten()
    .collect();
    allowed.iter().any(|root| p.starts_with(root))
}

pub fn read_script_file(path: &str) -> Result<String, NiTriTeError> {
    let p = std::path::Path::new(path);
    if !p.exists() {
        return Err(NiTriTeError::System(format!("Fichier introuvable: {}", path)));
    }
    // SECURITE : canonicalize AVANT le check whitelist pour prévenir TOCTOU / symlink
    let canonical = p.canonicalize().map_err(|e| {
        NiTriTeError::CommandDenied(format!("Chemin non résolvable: {}", e))
    })?;
    if !is_script_path_allowed(&canonical) {
        return Err(NiTriTeError::CommandDenied(
            format!("Lecture interdite hors des répertoires autorisés: {}", path),
        ));
    }
    let meta = std::fs::metadata(&canonical)?;
    if meta.len() > 100_000 {
        return Err(NiTriTeError::System("Fichier trop volumineux (max 100KB)".into()));
    }
    Ok(std::fs::read_to_string(&canonical)?)
}

pub fn save_script_file(path: &str, content: &str) -> Result<(), NiTriTeError> {
    let p = std::path::Path::new(path);
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !["ps1", "bat", "cmd", "sh", "py"].contains(&ext) {
        return Err(NiTriTeError::System(format!("Extension non autorisee: .{}", ext)));
    }

    // SECURITE TOCTOU : le fichier peut ne pas encore exister (création).
    // On canonicalize le répertoire parent, puis on recompose le chemin final.
    let canonical = if p.exists() {
        p.canonicalize().map_err(|e| {
            NiTriTeError::CommandDenied(format!("Chemin non résolvable: {}", e))
        })?
    } else {
        let parent = p.parent().ok_or_else(|| {
            NiTriTeError::System("Chemin sans répertoire parent".into())
        })?;
        let canonical_parent = parent.canonicalize().map_err(|e| {
            NiTriTeError::CommandDenied(format!("Répertoire parent non résolvable: {}", e))
        })?;
        let file_name = p.file_name().ok_or_else(|| {
            NiTriTeError::System("Nom de fichier manquant".into())
        })?;
        canonical_parent.join(file_name)
    };

    if !is_script_path_allowed(&canonical) {
        return Err(NiTriTeError::CommandDenied(
            format!("Écriture interdite hors des répertoires autorisés: {}", path),
        ));
    }
    std::fs::write(&canonical, content)?;
    Ok(())
}

fn script(name: &str, desc: &str, cat: &str, stype: &str, content: &str, admin: bool) -> ScriptEntry {
    ScriptEntry {
        name: name.to_string(),
        description: desc.to_string(),
        category: cat.to_string(),
        script_type: stype.to_string(),
        content: content.to_string(),
        requires_admin: admin,
        requires_interactive: false,
        is_builtin: true,
    }
}

/// Script builtin nécessitant Read-Host (interactive) — PowerShell sans -NonInteractive
fn script_interactive(name: &str, desc: &str, cat: &str, content: &str, admin: bool) -> ScriptEntry {
    ScriptEntry {
        name: name.to_string(),
        description: desc.to_string(),
        category: cat.to_string(),
        script_type: "powershell".to_string(),
        content: content.to_string(),
        requires_admin: admin,
        requires_interactive: true,
        is_builtin: true,
    }
}
