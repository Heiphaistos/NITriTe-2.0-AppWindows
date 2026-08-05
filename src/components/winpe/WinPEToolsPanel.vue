<script setup lang="ts">
import { ref, computed, type Component } from "vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import { invoke } from "@/utils/invoke";
import NButton from "@/components/ui/NButton.vue";
import { useNotificationStore } from "@/stores/notifications";
import {
  HardDrive, Shield, Search, Download, Key, Database, Terminal, Play, Rocket,
  Wrench, Wifi, Trash2, Users, LayoutGrid, Settings, Activity, Lock, Power,
  BookOpen, Zap, RefreshCw, Cpu, Printer,
} from "lucide-vue-next";

const notify = useNotificationStore();

// ─── Outils de récupération ─────────────────────────────────────────────────

interface RecoveryTool {
  id: string; name: string; type: string; description: string;
  url: string; size: string; icon: Component; localId?: string;
}

const RECOVERY_TOOLS: RecoveryTool[] = [
  { id: "jayro",      name: "Jayro's Lockpick",     type: "WinPE ISO",    description: "Réinitialisation MDP Windows 10/11, comptes Microsoft, contournement PIN.",             url: "https://www.jayro.eu/jayros-lockpick/",                              size: "~2 GB",   icon: Key,      localId: "lockpick" },
  { id: "lazesoft",  name: "Lazesoft Recovery",     type: "ISO bootable", description: "Récupération données, réinitialisation MDP, réparation démarrage, clonage disque.",      url: "https://www.lazesoft.com/lazesoft-recovery-suite-free.html",         size: "~500 MB", icon: Database },
  { id: "hirens",    name: "Hiren's BootCD PE",     type: "WinPE ISO",    description: "Collection 100+ outils de diagnostic et réparation. Standard du secteur.",               url: "https://www.hirensbootcd.org/download/",                             size: "~2 GB",   icon: HardDrive },
  { id: "sysrescue", name: "SystemRescue",           type: "Linux ISO",    description: "Distribution Linux live : chntpw, testdisk, partitionnement avancé.",                    url: "https://www.system-rescue.org/Download/",                            size: "~800 MB", icon: Shield },
  { id: "ntpasswd",  name: "NT Password Edit",       type: "Executable",   description: "Réinitialise les mots de passe NT hors ligne. Comptes locaux uniquement.",               url: "https://www.cdslow.org.ru/en/ntpwedit/",                             size: "~1 MB",   icon: Key },
  { id: "medicat",   name: "MediCat USB",            type: "Mega-ISO",     description: "USB multiboot : WinPE, Linux, antivirus, diagnostic, réparation.",                      url: "https://medicatusb.com/",                                            size: "~50 GB",  icon: HardDrive },
  { id: "macrium",   name: "Macrium Reflect WinPE", type: "ISO bootable", description: "Backup/restore image disque depuis WinPE, clonage bare-metal.",                          url: "https://www.macrium.com/reflectfree.aspx",                           size: "~600 MB", icon: Database },
  { id: "drivexml",  name: "DriveImage XML",         type: "Executable",   description: "Clone disque/partition, sauvegarde image, restauration sans redémarrage.",               url: "https://www.runtime.org/driveimage-xml.htm",                         size: "~10 MB",  icon: Database },
  { id: "testdisk",  name: "TestDisk / PhotoRec",    type: "Executable",   description: "Récupération partitions perdues et fichiers supprimés sur tout support.",                url: "https://www.cgsecurity.org/wiki/TestDisk_Download",                  size: "~5 MB",   icon: Search },
  { id: "gparted",   name: "GParted Live",           type: "Linux ISO",    description: "Gestionnaire partitions avancé. Redimensionner, déplacer, créer des volumes.",           url: "https://gparted.org/download.php",                                   size: "~350 MB", icon: HardDrive },
  { id: "kaspersky", name: "Kaspersky Rescue Disk", type: "Linux ISO",    description: "Antivirus de secours Kaspersky. Scan complet depuis environnement sain.",                 url: "https://www.kaspersky.fr/downloads/free-rescue-disk",                size: "~850 MB", icon: Shield },
  { id: "frst",      name: "FRST (Malwarebytes)",    type: "Executable",   description: "Farbar Recovery Scan Tool — diagnostic avancé et suppression malware WinPE.",            url: "https://www.bleepingcomputer.com/download/farbar-recovery-scan-tool/", size: "~2 MB",  icon: Shield },
];

// ─── Commandes rapides ───────────────────────────────────────────────────────

type CmdCat =
  | "Réparation" | "Réseau" | "Nettoyage" | "Comptes" | "Disques"
  | "Services" | "Diagnostic" | "Sécurité" | "Démarrage"
  | "Registre" | "Windows Update" | "Activation" | "Performances";

interface QuickCmd { label: string; cmd: string; cat: CmdCat; dangerous?: boolean; }

const QUICK_CMDS: QuickCmd[] = [

  // ── Réparation ──────────────────────────────────────────────────────────────
  { cat: "Réparation", label: "SFC scan",                    cmd: "sfc /scannow" },
  { cat: "Réparation", label: "SFC offline",                 cmd: "sfc /scannow /offbootdir=C:\\ /offwindir=C:\\Windows" },
  { cat: "Réparation", label: "SFC vérifie fichier",         cmd: "sfc /verifyfile=C:\\Windows\\System32\\kernel32.dll" },
  { cat: "Réparation", label: "DISM CheckHealth",            cmd: "DISM /Online /Cleanup-Image /CheckHealth" },
  { cat: "Réparation", label: "DISM ScanHealth",             cmd: "DISM /Online /Cleanup-Image /ScanHealth" },
  { cat: "Réparation", label: "DISM RestoreHealth",          cmd: "DISM /Online /Cleanup-Image /RestoreHealth" },
  { cat: "Réparation", label: "DISM ComponentCleanup",       cmd: "DISM /Online /Cleanup-Image /StartComponentCleanup" },
  { cat: "Réparation", label: "DISM ResetBase",              cmd: "DISM /Online /Cleanup-Image /StartComponentCleanup /ResetBase" },
  { cat: "Réparation", label: "DISM image offline",          cmd: "DISM /Image:C:\\ /Cleanup-Image /RestoreHealth" },
  { cat: "Réparation", label: "Réparer MBR",                 cmd: "bootrec /fixmbr" },
  { cat: "Réparation", label: "Réparer secteur boot",        cmd: "bootrec /fixboot" },
  { cat: "Réparation", label: "Reconstruire BCD",            cmd: "bootrec /rebuildbcd" },
  { cat: "Réparation", label: "Scanner OS",                  cmd: "bootrec /scanos" },
  { cat: "Réparation", label: "BCD Boot C:",                 cmd: "bcdboot C:\\Windows /s C: /f ALL" },
  { cat: "Réparation", label: "Export BCD",                  cmd: "bcdedit /export C:\\bcd_backup" },
  { cat: "Réparation", label: "Import BCD",                  cmd: "bcdedit /import C:\\bcd_backup" },
  { cat: "Réparation", label: "ChkDsk C: rapide",            cmd: "chkdsk C: /f" },
  { cat: "Réparation", label: "ChkDsk C: complet",           cmd: "chkdsk C: /f /r /x" },
  { cat: "Réparation", label: "ChkDsk D:",                   cmd: "chkdsk D: /f /r" },
  { cat: "Réparation", label: "Reset droits C:",             cmd: "icacls C:\\ /reset /T /C /Q" },
  { cat: "Réparation", label: "Prendre propriété fichier",   cmd: "takeown /f C:\\Windows\\System32\\[fichier] /a" },
  { cat: "Réparation", label: "Registre défaut",             cmd: "secedit /configure /db secedit.sdb /cfg %windir%\\inf\\defltbase.inf /overwrite" },
  { cat: "Réparation", label: "Réinitialiser WinRE",         cmd: "reagentc /disable && reagentc /enable" },
  { cat: "Réparation", label: "Reconstruire icônes",         cmd: "ie4uinit.exe -show" },
  { cat: "Réparation", label: "Réparer association fichiers", cmd: "assoc .exe=exefile && ftype exefile=\"%1\" %*" },
  { cat: "Réparation", label: "Vérif signatures système",    cmd: "sigverif" },
  { cat: "Réparation", label: "Réparer WMI",                 cmd: "winmgmt /resetrepository" },
  { cat: "Réparation", label: "Re-enregistrer DLL shell",    cmd: "regsvr32 /s shell32.dll" },

  // ── Réseau ──────────────────────────────────────────────────────────────────
  { cat: "Réseau", label: "Config IP complète",              cmd: "ipconfig /all" },
  { cat: "Réseau", label: "Libérer IP",                      cmd: "ipconfig /release" },
  { cat: "Réseau", label: "Renouveler IP",                   cmd: "ipconfig /renew" },
  { cat: "Réseau", label: "Vider cache DNS",                 cmd: "ipconfig /flushdns" },
  { cat: "Réseau", label: "Afficher DNS cache",              cmd: "ipconfig /displaydns" },
  { cat: "Réseau", label: "Reset Winsock",                   cmd: "netsh winsock reset" },
  { cat: "Réseau", label: "Reset TCP/IP",                    cmd: "netsh int ip reset" },
  { cat: "Réseau", label: "Reset IPv4",                      cmd: "netsh int ipv4 reset" },
  { cat: "Réseau", label: "Reset IPv6",                      cmd: "netsh int ipv6 reset" },
  { cat: "Réseau", label: "Reset pare-feu",                  cmd: "netsh advfirewall reset" },
  { cat: "Réseau", label: "TCP autotune off",                cmd: "netsh int tcp set global autotuninglevel=disabled" },
  { cat: "Réseau", label: "TCP autotune on",                 cmd: "netsh int tcp set global autotuninglevel=normal" },
  { cat: "Réseau", label: "Ping continu Google",             cmd: "ping 8.8.8.8 -t" },
  { cat: "Réseau", label: "Ping Cloudflare",                 cmd: "ping 1.1.1.1 -t" },
  { cat: "Réseau", label: "Traceroute",                      cmd: "tracert 8.8.8.8" },
  { cat: "Réseau", label: "PathPing",                        cmd: "pathping 8.8.8.8" },
  { cat: "Réseau", label: "DNS lookup",                      cmd: "nslookup google.com" },
  { cat: "Réseau", label: "DNS lookup (IP→nom)",             cmd: "nslookup 8.8.8.8" },
  { cat: "Réseau", label: "Connexions actives",              cmd: "netstat -ano" },
  { cat: "Réseau", label: "Ports en écoute",                 cmd: "netstat -ano | findstr LISTENING" },
  { cat: "Réseau", label: "Processus par port",              cmd: "netstat -b" },
  { cat: "Réseau", label: "Table ARP",                       cmd: "arp -a" },
  { cat: "Réseau", label: "Table de routage",                cmd: "route print" },
  { cat: "Réseau", label: "Profils WiFi",                    cmd: "netsh wlan show profiles" },
  { cat: "Réseau", label: "Infos WiFi courantes",            cmd: "netsh wlan show interfaces" },
  { cat: "Réseau", label: "Export MDP WiFi",                 cmd: "netsh wlan export profile folder=C:\\ key=clear" },
  { cat: "Réseau", label: "Déconnecter WiFi",                cmd: "netsh wlan disconnect" },
  { cat: "Réseau", label: "Partages réseau",                 cmd: "net share" },
  { cat: "Réseau", label: "Connexions montées",              cmd: "net use" },
  { cat: "Réseau", label: "Interfaces réseau",               cmd: "netsh interface show interface" },
  { cat: "Réseau", label: "Interfaces (PS)",                 cmd: "powershell Get-NetAdapter | Select Name,Status,LinkSpeed,MacAddress" },
  { cat: "Réseau", label: "IP config (PS)",                  cmd: "powershell Get-NetIPAddress | Select InterfaceAlias,IPAddress,PrefixLength" },
  { cat: "Réseau", label: "DNS config (PS)",                 cmd: "powershell Get-DnsClientServerAddress | Select InterfaceAlias,ServerAddresses" },
  { cat: "Réseau", label: "Test connexion (PS)",             cmd: "powershell Test-NetConnection -ComputerName 8.8.8.8 -Port 80" },
  { cat: "Réseau", label: "Voisins réseau",                  cmd: "nbtstat -n" },
  { cat: "Réseau", label: "Stats réseau",                    cmd: "net statistics workstation" },

  // ── Nettoyage ────────────────────────────────────────────────────────────────
  { cat: "Nettoyage", label: "Nettoyage disque C:",          cmd: "cleanmgr /d C:" },
  { cat: "Nettoyage", label: "Vider %TEMP%",                 cmd: "del /f /s /q %TEMP%\\*" },
  { cat: "Nettoyage", label: "Vider Temp Windows",           cmd: "del /f /s /q C:\\Windows\\Temp\\*" },
  { cat: "Nettoyage", label: "Vider Prefetch",               cmd: "del /f /s /q C:\\Windows\\Prefetch\\*" },
  { cat: "Nettoyage", label: "Supprimer .tmp C:",            cmd: "del /f /s /q C:\\*.tmp" },
  { cat: "Nettoyage", label: "Supprimer .log C:",            cmd: "del /f /s /q C:\\*.log" },
  { cat: "Nettoyage", label: "Vider corbeille C:",           cmd: "rd /s /q C:\\$Recycle.Bin" },
  { cat: "Nettoyage", label: "Vider cache WU",               cmd: "net stop wuauserv && del /f /q C:\\Windows\\SoftwareDistribution\\Download\\* && net start wuauserv" },
  { cat: "Nettoyage", label: "Vider cache DNS",              cmd: "ipconfig /flushdns" },
  { cat: "Nettoyage", label: "Effacer espace libre",         cmd: "cipher /w:C:\\" },
  { cat: "Nettoyage", label: "Nettoyer WinSxS",              cmd: "DISM /Online /Cleanup-Image /StartComponentCleanup /ResetBase" },
  { cat: "Nettoyage", label: "Vider cache miniatures",       cmd: "del /f /s /q %LocalAppData%\\Microsoft\\Windows\\Explorer\\thumbcache_*.db" },
  { cat: "Nettoyage", label: "Vider journaux d'événements",  cmd: "for /F \"tokens=*\" %1 in ('wevtutil.exe el') DO wevtutil.exe cl \"%1\"" },
  { cat: "Nettoyage", label: "Vider cache DNS (pipe PS)",    cmd: "powershell Clear-DnsClientCache" },
  { cat: "Nettoyage", label: "Supprimer points restauration",cmd: "vssadmin delete shadows /all /quiet", dangerous: true },
  { cat: "Nettoyage", label: "Vider spooler imprimante",     cmd: "net stop spooler && del /q %systemroot%\\system32\\spool\\printers\\* && net start spooler" },
  { cat: "Nettoyage", label: "Compact C: (NTFS)",            cmd: "compact /u /s:C:\\" },
  { cat: "Nettoyage", label: "Supprimer hiberfil.sys",       cmd: "powercfg /h off" },
  { cat: "Nettoyage", label: "Rapport nettoyage DISM",       cmd: "DISM /Online /Cleanup-Image /AnalyzeComponentStore" },
  { cat: "Nettoyage", label: "Purger store Windows",         cmd: "wsreset.exe" },

  // ── Comptes ──────────────────────────────────────────────────────────────────
  { cat: "Comptes", label: "Lister utilisateurs",            cmd: "net user" },
  { cat: "Comptes", label: "Activer Administrateur",         cmd: "net user Administrator /active:yes" },
  { cat: "Comptes", label: "Désactiver Administrateur",      cmd: "net user Administrator /active:no" },
  { cat: "Comptes", label: "Admins locaux",                  cmd: "net localgroup administrators" },
  { cat: "Comptes", label: "Tous les groupes locaux",        cmd: "net localgroup" },
  { cat: "Comptes", label: "Profils utilisateurs WMIC",      cmd: "wmic useraccount list brief" },
  { cat: "Comptes", label: "Users PowerShell",               cmd: "powershell Get-LocalUser | Select Name,Enabled,LastLogon,Description" },
  { cat: "Comptes", label: "Groupes PowerShell",             cmd: "powershell Get-LocalGroup | Select Name,Description" },
  { cat: "Comptes", label: "Membres groupe admin (PS)",      cmd: "powershell Get-LocalGroupMember Administrators" },
  { cat: "Comptes", label: "Derniers logins (PS)",           cmd: "powershell Get-LocalUser | Select Name,LastLogon | Sort LastLogon -Desc" },
  { cat: "Comptes", label: "Profils chargés",                cmd: "wmic userprofile list brief" },
  { cat: "Comptes", label: "Sessions actives",               cmd: "query user" },
  { cat: "Comptes", label: "Politique MDP",                  cmd: "net accounts" },

  // ── Disques ──────────────────────────────────────────────────────────────────
  { cat: "Disques", label: "Diskpart",                       cmd: "diskpart" },
  { cat: "Disques", label: "Disques WMIC",                   cmd: "wmic diskdrive get caption,size,status,serialnumber,interfacetype" },
  { cat: "Disques", label: "Volumes WMIC",                   cmd: "wmic logicaldisk get caption,size,freespace,filesystem,volumename" },
  { cat: "Disques", label: "Disques PowerShell",             cmd: "powershell Get-Disk | Select Number,FriendlyName,PartitionStyle,Size,HealthStatus" },
  { cat: "Disques", label: "Volumes PowerShell",             cmd: "powershell Get-Volume | Select DriveLetter,FileSystemLabel,FileSystem,SizeRemaining,Size,HealthStatus" },
  { cat: "Disques", label: "Partitions PowerShell",          cmd: "powershell Get-Partition | Select DiskNumber,PartitionNumber,DriveLetter,Size,Type" },
  { cat: "Disques", label: "Physique disques (PS)",          cmd: "powershell Get-PhysicalDisk | Select FriendlyName,MediaType,HealthStatus,OperationalStatus,Size" },
  { cat: "Disques", label: "Volumes BitLocker",              cmd: "manage-bde -status" },
  { cat: "Disques", label: "Déchiffrer BitLocker C:",        cmd: "manage-bde -off C:" },
  { cat: "Disques", label: "Shadow copies",                  cmd: "vssadmin list shadows" },
  { cat: "Disques", label: "Taille shadow copies",           cmd: "vssadmin list shadowstorage" },
  { cat: "Disques", label: "Défrag C: analyse",              cmd: "defrag C: /a" },
  { cat: "Disques", label: "Défrag C: verbose",              cmd: "defrag C: /u /v" },
  { cat: "Disques", label: "Espace disque C: (PS)",          cmd: "powershell Get-PSDrive C | Select Used,Free" },

  // ── Services ─────────────────────────────────────────────────────────────────
  { cat: "Services", label: "Tous les services",             cmd: "sc query type= all state= all" },
  { cat: "Services", label: "Services actifs",               cmd: "sc query state= running" },
  { cat: "Services", label: "Services arrêtés",              cmd: "sc query state= stopped" },
  { cat: "Services", label: "Services net démarrés",         cmd: "net start" },
  { cat: "Services", label: "Services actifs (PS)",          cmd: "powershell Get-Service | Where-Object Status -eq Running | Select Name,DisplayName" },
  { cat: "Services", label: "Services désactivés (PS)",      cmd: "powershell Get-Service | Where-Object StartType -eq Disabled | Select Name,DisplayName" },
  { cat: "Services", label: "Services par process (PS)",     cmd: "tasklist /svc" },
  { cat: "Services", label: "Stop Windows Update",           cmd: "net stop wuauserv" },
  { cat: "Services", label: "Start Windows Update",          cmd: "net start wuauserv" },
  { cat: "Services", label: "Stop Defender",                 cmd: "net stop WinDefend" },
  { cat: "Services", label: "Stop BITS",                     cmd: "net stop bits" },
  { cat: "Services", label: "Restart spooler",               cmd: "net stop spooler && net start spooler" },
  { cat: "Services", label: "Start Remote Registry",         cmd: "sc start RemoteRegistry" },
  { cat: "Services", label: "Stop SuperFetch",               cmd: "net stop SysMain" },
  { cat: "Services", label: "Restart DHCP client",           cmd: "net stop dhcp && net start dhcp" },
  { cat: "Services", label: "Restart DNS client",            cmd: "net stop dnscache && net start dnscache" },
  { cat: "Services", label: "Restart Audio",                 cmd: "net stop audiosrv && net start audiosrv" },
  { cat: "Services", label: "Services crashés (PS)",         cmd: "powershell Get-EventLog System -Source 'Service Control Manager' -EntryType Error -Newest 20" },

  // ── Diagnostic ───────────────────────────────────────────────────────────────
  { cat: "Diagnostic", label: "Infos système",               cmd: "systeminfo" },
  { cat: "Diagnostic", label: "Infos hardware GUI",          cmd: "msinfo32" },
  { cat: "Diagnostic", label: "Version Windows",             cmd: "winver" },
  { cat: "Diagnostic", label: "DirectX GUI",                 cmd: "dxdiag" },
  { cat: "Diagnostic", label: "Visionneur événements",       cmd: "eventvwr" },
  { cat: "Diagnostic", label: "Moniteur performances",       cmd: "perfmon" },
  { cat: "Diagnostic", label: "Moniteur ressources",         cmd: "resmon" },
  { cat: "Diagnostic", label: "Gestionnaire tâches",         cmd: "taskmgr" },
  { cat: "Diagnostic", label: "Processus verbose",           cmd: "tasklist /v" },
  { cat: "Diagnostic", label: "CPU WMIC",                    cmd: "wmic cpu get name,numberofcores,maxclockspeed,loadpercentage,architecture" },
  { cat: "Diagnostic", label: "RAM WMIC",                    cmd: "wmic memorychip get capacity,speed,manufacturer,banklabel,devicelocator" },
  { cat: "Diagnostic", label: "GPU WMIC",                    cmd: "wmic path win32_VideoController get name,driverversion,adapterram,currenthorizontalresolution" },
  { cat: "Diagnostic", label: "BIOS WMIC",                   cmd: "wmic bios get serialnumber,version,releasedate,manufacturer,name" },
  { cat: "Diagnostic", label: "Carte mère WMIC",             cmd: "wmic baseboard get manufacturer,product,version,serialnumber" },
  { cat: "Diagnostic", label: "Modèle PC WMIC",              cmd: "wmic computersystem get manufacturer,model,totalphysicalmemory,systemtype" },
  { cat: "Diagnostic", label: "Numéro de série BIOS",        cmd: "wmic bios get serialnumber" },
  { cat: "Diagnostic", label: "UUID machine",                cmd: "wmic csproduct get uuid" },
  { cat: "Diagnostic", label: "Pilotes installés (PS)",      cmd: "powershell Get-WindowsDriver -Online | Select Driver,OriginalFileName,Version,Date | Sort Date -Desc | Select -First 30" },
  { cat: "Diagnostic", label: "Pilotes tiers (PS)",          cmd: "powershell Get-WmiObject Win32_PnPSignedDriver | Where-Object IsSigned -eq $false | Select DeviceName,DriverVersion" },
  { cat: "Diagnostic", label: "Uptime système (PS)",         cmd: "powershell (Get-Date) - (gcim Win32_OperatingSystem).LastBootUpTime" },
  { cat: "Diagnostic", label: "Erreurs système (PS)",        cmd: "powershell Get-EventLog -LogName System -Newest 30 -EntryType Error | Select TimeGenerated,Source,Message" },
  { cat: "Diagnostic", label: "Infos batterie",              cmd: "powercfg /batteryreport /output C:\\battery_report.html && start C:\\battery_report.html" },
  { cat: "Diagnostic", label: "Rapport énergie",             cmd: "powercfg /energy /output C:\\energy_report.html && start C:\\energy_report.html" },
  { cat: "Diagnostic", label: "Variables d'environnement",   cmd: "set" },
  { cat: "Diagnostic", label: "Explorateur fichiers",        cmd: "explorer" },
  { cat: "Diagnostic", label: "Regedit",                     cmd: "regedit" },
  { cat: "Diagnostic", label: "Températures WMI (PS)",       cmd: "powershell Get-WmiObject MSAcpi_ThermalZoneTemperature -Namespace root/wmi" },
  { cat: "Diagnostic", label: "Périphériques PnP (PS)",      cmd: "powershell Get-PnpDevice | Where-Object Status -ne OK | Select Status,Class,FriendlyName" },
  { cat: "Diagnostic", label: "Mises à jour installées (PS)",cmd: "powershell Get-HotFix | Sort-Object InstalledOn -Descending | Select HotFixID,Description,InstalledOn | Select -First 20" },

  // ── Sécurité ─────────────────────────────────────────────────────────────────
  { cat: "Sécurité", label: "Forcer GPO",                    cmd: "gpupdate /force" },
  { cat: "Sécurité", label: "Résultat GPO",                  cmd: "gpresult /r" },
  { cat: "Sécurité", label: "Rapport GPO HTML",              cmd: "gpresult /h C:\\gpo_report.html && start C:\\gpo_report.html" },
  { cat: "Sécurité", label: "Politiques d'audit",            cmd: "auditpol /get /category:*" },
  { cat: "Sécurité", label: "Règles pare-feu",               cmd: "netsh advfirewall firewall show rule name=all" },
  { cat: "Sécurité", label: "Désactiver pare-feu",           cmd: "netsh advfirewall set allprofiles state off" },
  { cat: "Sécurité", label: "Activer pare-feu",              cmd: "netsh advfirewall set allprofiles state on" },
  { cat: "Sécurité", label: "Statut pare-feu",               cmd: "netsh advfirewall show allprofiles" },
  { cat: "Sécurité", label: "Scan Defender complet (PS)",    cmd: "powershell Start-MpScan -ScanType FullScan" },
  { cat: "Sécurité", label: "Scan Defender rapide (PS)",     cmd: "powershell Start-MpScan -ScanType QuickScan" },
  { cat: "Sécurité", label: "Désact realtime Defender (PS)", cmd: "powershell Set-MpPreference -DisableRealtimeMonitoring $true" },
  { cat: "Sécurité", label: "Activ realtime Defender (PS)",  cmd: "powershell Set-MpPreference -DisableRealtimeMonitoring $false" },
  { cat: "Sécurité", label: "Statut Defender (PS)",          cmd: "powershell Get-MpComputerStatus | Select AMRunningMode,RealTimeProtectionEnabled,AntivirusEnabled" },
  { cat: "Sécurité", label: "TestSigning off",               cmd: "bcdedit /set testsigning off" },
  { cat: "Sécurité", label: "TestSigning on",                cmd: "bcdedit /set testsigning on" },
  { cat: "Sécurité", label: "NoIntegrityChecks off",         cmd: "bcdedit /set nointegritychecks off" },
  { cat: "Sécurité", label: "Gestionnaire certificats",      cmd: "certmgr.msc" },
  { cat: "Sécurité", label: "Politique sécurité locale",     cmd: "secpol.msc" },
  { cat: "Sécurité", label: "Connexions suspectes",          cmd: "netstat -ano | findstr ESTABLISHED" },
  { cat: "Sécurité", label: "Programmes au démarrage (reg)", cmd: "reg query HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run" },
  { cat: "Sécurité", label: "Programmes au démarrage user",  cmd: "reg query HKCU\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run" },
  { cat: "Sécurité", label: "Politique comptes",             cmd: "net accounts" },

  // ── Démarrage ────────────────────────────────────────────────────────────────
  { cat: "Démarrage", label: "MSConfig",                     cmd: "msconfig" },
  { cat: "Démarrage", label: "Liste BCD complète",           cmd: "bcdedit /enum all" },
  { cat: "Démarrage", label: "Entrées boot actives",         cmd: "bcdedit /enum active" },
  { cat: "Démarrage", label: "Export BCD",                   cmd: "bcdedit /export C:\\bcd_backup" },
  { cat: "Démarrage", label: "Import BCD",                   cmd: "bcdedit /import C:\\bcd_backup" },
  { cat: "Démarrage", label: "Activer Mode sans échec",      cmd: "bcdedit /set {current} safeboot minimal" },
  { cat: "Démarrage", label: "Safe mode réseau",             cmd: "bcdedit /set {current} safeboot network" },
  { cat: "Démarrage", label: "Safe mode ligne de commande",  cmd: "bcdedit /set {current} safeboot network && bcdedit /set {current} safebootalternateshell yes" },
  { cat: "Démarrage", label: "Désact Mode sans échec",       cmd: "bcdedit /deletevalue {current} safeboot" },
  { cat: "Démarrage", label: "BCD Boot C:",                  cmd: "bcdboot C:\\Windows /s C: /f ALL" },
  { cat: "Démarrage", label: "BCD Boot (MBR+UEFI)",         cmd: "bcdboot C:\\Windows /s C: /f ALL /m" },
  { cat: "Démarrage", label: "Info WinRE",                   cmd: "reagentc /info" },
  { cat: "Démarrage", label: "Activer WinRE",                cmd: "reagentc /enable" },
  { cat: "Démarrage", label: "Désactiver WinRE",             cmd: "reagentc /disable" },
  { cat: "Démarrage", label: "Démarrer vers WinRE",          cmd: "reagentc /boottore" },
  { cat: "Démarrage", label: "Timeout boot",                 cmd: "bcdedit /timeout 10" },
  { cat: "Démarrage", label: "Tâches démarrage (PS)",        cmd: "powershell Get-CimInstance Win32_StartupCommand | Select Name,Command,Location,User" },
  { cat: "Démarrage", label: "Historique BSOD (PS)",         cmd: "powershell Get-EventLog System -Source 'Microsoft-Windows-WER*' -EntryType Error -Newest 20" },

  // ── Registre ─────────────────────────────────────────────────────────────────
  { cat: "Registre", label: "Regedit",                       cmd: "regedit" },
  { cat: "Registre", label: "Export HKLM",                   cmd: "reg export HKLM C:\\backup_HKLM.reg" },
  { cat: "Registre", label: "Export HKCU",                   cmd: "reg export HKCU C:\\backup_HKCU.reg" },
  { cat: "Registre", label: "Export HKLM Run",               cmd: "reg export HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run C:\\run_backup.reg" },
  { cat: "Registre", label: "Démarrage HKLM",                cmd: "reg query HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run" },
  { cat: "Registre", label: "Démarrage HKCU",                cmd: "reg query HKCU\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run" },
  { cat: "Registre", label: "Services registre",             cmd: "reg query HKLM\\SYSTEM\\CurrentControlSet\\Services" },
  { cat: "Registre", label: "Version Windows (reg)",         cmd: "reg query \"HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\" /v CurrentVersion" },
  { cat: "Registre", label: "Product ID (reg)",              cmd: "reg query \"HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\" /v ProductId" },
  { cat: "Registre", label: "Clé produit (reg)",             cmd: "reg query \"HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\SoftwareProtectionPlatform\" /v BackupProductKeyDefault" },
  { cat: "Registre", label: "Rechercher malware run",        cmd: "reg query HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\RunOnce" },
  { cat: "Registre", label: "Import fichier .reg",           cmd: "reg import C:\\backup_HKLM.reg" },
  { cat: "Registre", label: "Shell (explorateur)",           cmd: "reg query \"HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon\" /v Shell" },

  // ── Windows Update ────────────────────────────────────────────────────────────
  { cat: "Windows Update", label: "Lancer détection MàJ",    cmd: "UsoClient StartScan" },
  { cat: "Windows Update", label: "Télécharger MàJ",         cmd: "UsoClient StartDownload" },
  { cat: "Windows Update", label: "Installer MàJ",           cmd: "UsoClient StartInstall" },
  { cat: "Windows Update", label: "Forcer détection (legacy)",cmd: "wuauclt /detectnow" },
  { cat: "Windows Update", label: "Forcer rapport (legacy)",  cmd: "wuauclt /reportnow" },
  { cat: "Windows Update", label: "Stop service WU",          cmd: "net stop wuauserv" },
  { cat: "Windows Update", label: "Start service WU",         cmd: "net start wuauserv" },
  { cat: "Windows Update", label: "Vider cache WU",           cmd: "net stop wuauserv && rd /s /q C:\\Windows\\SoftwareDistribution && net start wuauserv" },
  { cat: "Windows Update", label: "Réinitialiser WU complet", cmd: "net stop wuauserv && net stop cryptsvc && net stop bits && rd /s /q C:\\Windows\\SoftwareDistribution && net start bits && net start cryptsvc && net start wuauserv" },
  { cat: "Windows Update", label: "MàJ installées (PS)",      cmd: "powershell Get-HotFix | Sort-Object InstalledOn -Descending | Select -First 20" },
  { cat: "Windows Update", label: "Historique WU (PS)",       cmd: "powershell Get-WindowsUpdateLog" },
  { cat: "Windows Update", label: "Rapport WU (reg)",         cmd: "reg query HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\WindowsUpdate\\Auto Update\\Results\\Install" },

  // ── Activation ────────────────────────────────────────────────────────────────
  { cat: "Activation", label: "Statut activation",            cmd: "slmgr /xpr" },
  { cat: "Activation", label: "Infos licence détaillées",     cmd: "slmgr /dlv" },
  { cat: "Activation", label: "Activer en ligne",             cmd: "slmgr /ato" },
  { cat: "Activation", label: "Installer clé produit",        cmd: "slmgr /ipk XXXXX-XXXXX-XXXXX-XXXXX-XXXXX" },
  { cat: "Activation", label: "Désinstaller clé",             cmd: "slmgr /upk" },
  { cat: "Activation", label: "Réinitialiser période essai",  cmd: "slmgr /rearm" },
  { cat: "Activation", label: "Statut licence court",         cmd: "slmgr /dli" },
  { cat: "Activation", label: "Définir serveur KMS",          cmd: "slmgr /skms kms.exemple.com" },
  { cat: "Activation", label: "Edition Windows (WMIC)",       cmd: "wmic os get caption,version,buildnumber,osarchitecture" },
  { cat: "Activation", label: "Clé produit (PowerShell)",     cmd: "powershell (Get-WmiObject -query 'select * from SoftwareLicensingService').OA3xOriginalProductKey" },

  // ── Performances ──────────────────────────────────────────────────────────────
  { cat: "Performances", label: "Plans d'alimentation",       cmd: "powercfg /list" },
  { cat: "Performances", label: "Activer Haute Perf.",        cmd: "powercfg /setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c" },
  { cat: "Performances", label: "Activer Économie",           cmd: "powercfg /setactive a1841308-3541-4fab-bc81-f71556f20b4a" },
  { cat: "Performances", label: "Activer Équilibré",          cmd: "powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e" },
  { cat: "Performances", label: "Plan actif",                 cmd: "powercfg /getactivescheme" },
  { cat: "Performances", label: "Désactiver hibernation",     cmd: "powercfg /h off" },
  { cat: "Performances", label: "Activer hibernation",        cmd: "powercfg /h on" },
  { cat: "Performances", label: "Rapport batterie",           cmd: "powercfg /batteryreport /output C:\\battery.html && start C:\\battery.html" },
  { cat: "Performances", label: "Rapport énergie",            cmd: "powercfg /energy /output C:\\energy.html && start C:\\energy.html" },
  { cat: "Performances", label: "Appareils wake (réseau)",    cmd: "powercfg /devicequery wake_armed" },
  { cat: "Performances", label: "Désact indexation C:",       cmd: "sc config WSearch start= disabled && net stop WSearch" },
  { cat: "Performances", label: "Activ indexation",           cmd: "sc config WSearch start= delayed-auto && net start WSearch" },
  { cat: "Performances", label: "CPU usage (PS live)",        cmd: "powershell while($true){$c=(Get-Counter '\\Processor(_Total)\\% Processor Time').CounterSamples.CookedValue; Write-Host \"CPU: $([math]::Round($c,1))%\"; Start-Sleep 2}" },
  { cat: "Performances", label: "Mémoire disponible (PS)",    cmd: "powershell [math]::Round((Get-WmiObject Win32_OperatingSystem).FreePhysicalMemory/1MB,2)" },
];

// ─── Catégories ──────────────────────────────────────────────────────────────

interface CatDef { label: string; icon: Component; }
const CATEGORIES: CatDef[] = [
  { label: "Tous",           icon: LayoutGrid },
  { label: "Réparation",     icon: Wrench },
  { label: "Réseau",         icon: Wifi },
  { label: "Nettoyage",      icon: Trash2 },
  { label: "Comptes",        icon: Users },
  { label: "Disques",        icon: HardDrive },
  { label: "Services",       icon: Settings },
  { label: "Diagnostic",     icon: Activity },
  { label: "Sécurité",       icon: Lock },
  { label: "Démarrage",      icon: Power },
  { label: "Registre",       icon: BookOpen },
  { label: "Windows Update", icon: RefreshCw },
  { label: "Activation",     icon: Key },
  { label: "Performances",   icon: Zap },
];

const activecat = ref<string>("Tous");

const filteredCmds = computed(() =>
  activecat.value === "Tous"
    ? QUICK_CMDS
    : QUICK_CMDS.filter(c => c.cat === activecat.value)
);

// ─── Actions ─────────────────────────────────────────────────────────────────

async function openRecoveryTool(tool: RecoveryTool) {
  try {
    await invoke("open_url", { url: tool.url });
  } catch {
    window.open(tool.url, "_blank");
  }
  notify.info(`Téléchargement ${tool.name}`, "Page officielle ouverte dans le navigateur.");
}

async function launchLocalTool(tool: RecoveryTool) {
  if (!tool.localId) return;
  try {
    await invoke("launch_portable", { appId: tool.localId });
    notify.success(`${tool.name} lancé`, "Application démarrée.");
  } catch (e) {
    notify.error(`Impossible de lancer ${tool.name}`, String(e));
  }
}

async function runQuickCmd(cmd: QuickCmd) {
  // "Supprimer points restauration" (vssadmin delete shadows /all /quiet) est le
  // seul item irréversible/destructif de ce catalogue (efface tous les points de
  // restauration ET les shadow copies, sans possibilité de retour) — même seuil
  // de gravité que les formatages/wipes déjà protégés ailleurs, d'où le flag
  // `dangerous` + confirmation dédiée au lieu du Run silencieux des autres commandes.
  if (cmd.dangerous) {
    const ok = await confirm(
      `Supprimer TOUS les points de restauration et copies shadow ?\n\nAction irréversible : plus aucun retour arrière possible via la Restauration système ou les "versions précédentes" de fichiers.\n\nCommande : ${cmd.cmd}`,
      { title: "Nitrite", kind: "warning" }
    );
    if (!ok) return;
  }
  try {
    await invoke("winpe_run_command", { command: cmd.cmd });
    notify.info("Commande exécutée", cmd.cmd);
  } catch {
    notify.info("Copier dans le terminal WinPE", cmd.cmd);
  }
}
</script>

<template>
  <div class="winpe-tools-panel">

    <!-- Outils de récupération -->
    <div class="section-card">
      <h2 class="section-title"><Download :size="16" /> Outils de Récupération Avancés</h2>
      <p class="section-desc">Outils tiers pour les cas complexes (comptes Microsoft, TPM, BitLocker avancé…). Téléchargez sur une clé USB <strong>avant</strong> d'en avoir besoin.</p>
      <div class="recovery-grid">
        <div v-for="tool in RECOVERY_TOOLS" :key="tool.id" class="rec-card">
          <div class="rec-header">
            <component :is="tool.icon" :size="18" class="rec-icon" />
            <div>
              <div class="rec-name">{{ tool.name }}</div>
              <div class="rec-type">{{ tool.type }}</div>
            </div>
          </div>
          <p class="rec-desc">{{ tool.description }}</p>
          <div class="rec-footer">
            <span class="rec-size">{{ tool.size }}</span>
            <div class="rec-actions">
              <NButton v-if="tool.localId" size="sm" variant="primary" @click="launchLocalTool(tool)">
                <Rocket :size="11" /> Lancer
              </NButton>
              <NButton size="sm" variant="ghost" @click="openRecoveryTool(tool)">
                <Download :size="11" /> Télécharger
              </NButton>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Commandes Windows -->
    <div class="section-card">
      <div class="cmd-header">
        <h2 class="section-title"><Terminal :size="16" /> Commandes Windows</h2>
        <span class="cmd-count">{{ filteredCmds.length }} / {{ QUICK_CMDS.length }} commandes</span>
      </div>
      <p class="section-desc">Toutes les commandes Windows essentielles : réparation, réseau, nettoyage, comptes, disques, services, diagnostic, sécurité, démarrage, registre, Windows Update, activation et performances.</p>

      <!-- Pills filtre -->
      <div class="cat-pills">
        <button
          v-for="cat in CATEGORIES"
          :key="cat.label"
          class="cat-pill"
          :class="{ active: activecat === cat.label }"
          @click="activecat = cat.label"
        >
          <component :is="cat.icon" :size="11" />
          {{ cat.label }}
        </button>
      </div>

      <!-- Grille -->
      <div class="quick-grid">
        <div v-for="cmd in filteredCmds" :key="cmd.label + cmd.cat" class="quick-card" :class="{ 'quick-card-danger': cmd.dangerous }">
          <div class="quick-top">
            <span class="quick-label">{{ cmd.label }}</span>
            <span v-if="activecat === 'Tous'" class="quick-cat-badge">{{ cmd.cat }}</span>
          </div>
          <code class="quick-code" :title="cmd.cmd">{{ cmd.cmd }}</code>
          <NButton size="sm" :variant="cmd.dangerous ? 'danger' : 'ghost'" @click="runQuickCmd(cmd)">
            <Play :size="11" /> Run
          </NButton>
        </div>
      </div>
    </div>

  </div>
</template>

<style scoped>
.winpe-tools-panel { display: flex; flex-direction: column; gap: 16px; }

.section-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-xl);
  padding: 16px;
}

.section-title {
  display: flex; align-items: center; gap: 8px;
  font-size: 14px; font-weight: 700; color: var(--text-primary);
  margin-bottom: 8px;
}

.section-desc {
  font-size: 12px; color: var(--text-muted); margin-bottom: 14px; line-height: 1.6;
}

/* Recovery */
.recovery-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 10px; }
.rec-card {
  background: var(--bg-primary); border: 1px solid var(--border);
  border-radius: var(--radius-lg); padding: 12px;
  display: flex; flex-direction: column; gap: 8px;
  transition: border-color .2s, box-shadow .2s;
}
.rec-card:hover { border-color: var(--accent-primary); box-shadow: 0 0 0 1px rgba(249,115,22,.15); }
.rec-header { display: flex; align-items: flex-start; gap: 10px; }
.rec-icon { color: var(--accent-primary); flex-shrink: 0; margin-top: 2px; }
.rec-name { font-weight: 700; font-size: 12px; color: var(--text-primary); }
.rec-type { font-size: 10px; color: var(--text-muted); text-transform: uppercase; letter-spacing: .04em; }
.rec-desc { font-size: 11px; color: var(--text-muted); line-height: 1.5; flex: 1; }
.rec-footer { display: flex; align-items: center; justify-content: space-between; }
.rec-actions { display: flex; gap: 6px; }
.rec-size { font-size: 11px; color: var(--text-muted); font-family: monospace; }

/* Cmd header */
.cmd-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }
.cmd-header .section-title { margin-bottom: 0; }
.cmd-count {
  font-size: 11px; color: var(--text-muted);
  background: var(--bg-tertiary); border: 1px solid var(--border);
  border-radius: 20px; padding: 2px 8px;
}

/* Pills */
.cat-pills { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 14px; }
.cat-pill {
  display: inline-flex; align-items: center; gap: 5px;
  padding: 4px 10px; border-radius: 20px;
  font-size: 11px; font-weight: 500;
  border: 1px solid var(--border);
  background: var(--bg-primary); color: var(--text-muted);
  cursor: pointer; transition: all .15s;
}
.cat-pill:hover { border-color: var(--accent-primary); color: var(--text-primary); }
.cat-pill.active {
  background: var(--accent-primary); border-color: var(--accent-primary);
  color: #fff; font-weight: 600;
}

/* Commands */
.quick-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
  gap: 8px;
}
.quick-card {
  background: var(--bg-primary); border: 1px solid var(--border);
  border-radius: var(--radius-md); padding: 10px 12px;
  display: flex; flex-direction: column; gap: 6px;
  transition: border-color .15s;
}
.quick-card:hover { border-color: var(--border-hover); }
.quick-card-danger { border-color: var(--danger); }
.quick-card-danger:hover { border-color: var(--danger); }
.quick-top { display: flex; align-items: center; justify-content: space-between; gap: 4px; }
.quick-label { font-size: 11px; font-weight: 600; color: var(--text-secondary); }
.quick-cat-badge {
  font-size: 9px; font-weight: 600; color: var(--accent-primary);
  background: rgba(249,115,22,.1); border: 1px solid rgba(249,115,22,.2);
  border-radius: 4px; padding: 1px 5px; white-space: nowrap; flex-shrink: 0;
}
.quick-code {
  font-size: 10px; font-family: monospace; color: var(--accent-primary);
  background: var(--bg-tertiary); padding: 3px 6px; border-radius: 4px;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis; display: block;
}
</style>
