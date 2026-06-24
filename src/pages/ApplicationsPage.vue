<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke, invokeRaw } from "@/utils/invoke";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { cachedInvoke, refreshCached } from "@/composables/useCachedInvoke";
import NButton from "@/components/ui/NButton.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NSearchBar from "@/components/ui/NSearchBar.vue";
import NTabs from "@/components/ui/NTabs.vue";
import { useNotificationStore } from "@/stores/notifications";
import {
  Package, Download, CheckCircle, AlertTriangle,
  Globe, FileText, Code, Film, MessageSquare,
  Shield, Settings, Wifi, Gamepad2, RefreshCw,
  Trash2, Search, HardDrive,
} from "lucide-vue-next";

const notifications = useNotificationStore();

interface AppInfo {
  id: string;
  name: string;
  description: string;
  winget_id: string;
  category: string;
  icon?: string;
  version?: string;
  homepage?: string;
  url?: string;       // champ Rust (programs.json) — alias de homepage
  size_mb?: number;
}

const loading = ref(true);
const apps = ref<AppInfo[]>([]);
const wingetOk = ref(false);
const searchQuery = ref("");
const activeCategory = ref("all");
const installingIds = ref<Set<string>>(new Set());
const installedIds = ref<Set<string>>(new Set());
const installLogs = ref<Record<string, string[]>>({});
const uninstallingIds = ref<Set<string>>(new Set());
const checkingUpdateIds = ref<Set<string>>(new Set());

// Map app.id → version installée détectée
const installedVersions = ref<Record<string, string>>({});

let unlistenLog: UnlistenFn | null = null;

const categoryTabs = [
  { id: "all", label: "Tout" },
  { id: "navigateurs", label: "Navigateurs" },
  { id: "bureautique", label: "Bureautique" },
  { id: "dev", label: "Dev" },
  { id: "multimedia", label: "Multimedia" },
  { id: "communication", label: "Communication" },
  { id: "securite", label: "Securite" },
  { id: "systeme", label: "Systeme" },
  { id: "reseau", label: "Reseau" },
  { id: "gaming", label: "Gaming" },
];

const categoryIcons: Record<string, any> = {
  navigateurs: Globe,
  bureautique: FileText,
  dev: Code,
  multimedia: Film,
  communication: MessageSquare,
  securite: Shield,
  systeme: Settings,
  reseau: Wifi,
  gaming: Gamepad2,
};

function normalizeStr(s: string | null | undefined) {
  if (s == null) return "";
  return s.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "");
}

const filteredApps = computed(() => {
  let result = apps.value;
  if (activeCategory.value !== "all") {
    result = result.filter((a) => normalizeStr(a.category) === normalizeStr(activeCategory.value));
  }
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase();
    result = result.filter(
      (a) =>
        (a.name ?? "").toLowerCase().includes(q) ||
        (a.description ?? "").toLowerCase().includes(q) ||
        (a.winget_id ?? "").toLowerCase().includes(q)
    );
  }
  return result;
});

// Mapping catégories programs.json → ApplicationsPage
const CATEGORY_MAP: Record<string, string> = {
  "navigateurs": "navigateurs", "bureautique": "bureautique", "developpement": "dev",
  "multimedia": "multimedia", "streaming video": "multimedia", "streaming audio": "multimedia",
  "communication": "communication", "reseaux sociaux": "communication",
  "securite": "securite", "antivirus": "securite", "desinstallateurs antivirus": "securite",
  "utilitaires systeme": "systeme", "utilitaires": "systeme", "compression": "systeme",
  "outils essentiels": "systeme", "suites professionnelles": "bureautique",
  "productivite": "bureautique", "pdf et documents": "bureautique",
  "ia & assistants": "dev", "imprimantes & scan": "systeme", "services apple": "systeme",
  "internet": "reseau", "stockage cloud": "reseau",
  "jeux": "gaming",
};

const devApps: AppInfo[] = [
  // Navigateurs
  { id: "1", name: "Google Chrome", description: "Navigateur web rapide et securise", winget_id: "Google.Chrome", category: "navigateurs", homepage: "https://www.google.com/chrome/" },
  { id: "2", name: "Mozilla Firefox", description: "Navigateur web open source", winget_id: "Mozilla.Firefox", category: "navigateurs", homepage: "https://www.mozilla.org/firefox/" },
  { id: "3", name: "Brave", description: "Navigateur axe sur la vie privee", winget_id: "Brave.Brave", category: "navigateurs", homepage: "https://brave.com/" },
  { id: "4", name: "Microsoft Edge", description: "Navigateur Microsoft Chromium", winget_id: "Microsoft.Edge", category: "navigateurs", homepage: "https://www.microsoft.com/edge" },
  { id: "5", name: "Opera GX", description: "Navigateur gaming", winget_id: "Opera.OperaGX", category: "navigateurs", homepage: "https://www.opera.com/gx" },
  // Bureautique
  { id: "10", name: "LibreOffice", description: "Suite bureautique complete et gratuite", winget_id: "TheDocumentFoundation.LibreOffice", category: "bureautique", homepage: "https://www.libreoffice.org/", size_mb: 350 },
  { id: "11", name: "Notepad++", description: "Editeur de texte avance", winget_id: "Notepad++.Notepad++", category: "bureautique", homepage: "https://notepad-plus-plus.org/", size_mb: 4 },
  { id: "12", name: "Obsidian", description: "Prise de notes en Markdown", winget_id: "Obsidian.Obsidian", category: "bureautique", homepage: "https://obsidian.md/", size_mb: 85 },
  { id: "13", name: "Notion", description: "Workspace collaboratif tout-en-un", winget_id: "Notion.Notion", category: "bureautique", homepage: "https://notion.so/", size_mb: 120 },
  // Dev
  { id: "20", name: "Visual Studio Code", description: "Editeur de code source", winget_id: "Microsoft.VisualStudioCode", category: "dev", homepage: "https://code.visualstudio.com/", size_mb: 90 },
  { id: "21", name: "Git", description: "Systeme de controle de version", winget_id: "Git.Git", category: "dev", homepage: "https://git-scm.com/", size_mb: 48 },
  { id: "22", name: "Node.js LTS", description: "Runtime JavaScript", winget_id: "OpenJS.NodeJS.LTS", category: "dev", homepage: "https://nodejs.org/", size_mb: 30 },
  { id: "23", name: "Python 3.12", description: "Langage de programmation", winget_id: "Python.Python.3.12", category: "dev", homepage: "https://www.python.org/", size_mb: 25 },
  { id: "24", name: "Docker Desktop", description: "Conteneurisation d'applications", winget_id: "Docker.DockerDesktop", category: "dev", homepage: "https://www.docker.com/", size_mb: 800 },
  { id: "25", name: "Postman", description: "Test d'API REST", winget_id: "Postman.Postman", category: "dev", homepage: "https://www.postman.com/", size_mb: 200 },
  // Multimedia
  { id: "30", name: "VLC", description: "Lecteur multimedia universel", winget_id: "VideoLAN.VLC", category: "multimedia", homepage: "https://www.videolan.org/vlc/", size_mb: 40 },
  { id: "31", name: "OBS Studio", description: "Streaming et enregistrement video", winget_id: "OBSProject.OBSStudio", category: "multimedia", homepage: "https://obsproject.com/", size_mb: 300 },
  { id: "32", name: "GIMP", description: "Editeur d'images open source", winget_id: "GIMP.GIMP", category: "multimedia", homepage: "https://www.gimp.org/", size_mb: 250 },
  { id: "33", name: "Audacity", description: "Editeur audio open source", winget_id: "Audacity.Audacity", category: "multimedia", homepage: "https://www.audacityteam.org/", size_mb: 35 },
  { id: "34", name: "Spotify", description: "Streaming musical", winget_id: "Spotify.Spotify", category: "multimedia", homepage: "https://www.spotify.com/", size_mb: 180 },
  { id: "35", name: "HandBrake", description: "Transcodeur video open source", winget_id: "HandBrake.HandBrake", category: "multimedia", homepage: "https://handbrake.fr/", size_mb: 15 },
  // Communication
  { id: "40", name: "Discord", description: "Communication vocale et textuelle", winget_id: "Discord.Discord", category: "communication", homepage: "https://discord.com/", size_mb: 300 },
  { id: "41", name: "Zoom", description: "Visioconference", winget_id: "Zoom.Zoom", category: "communication", homepage: "https://zoom.us/", size_mb: 150 },
  { id: "42", name: "Slack", description: "Messagerie professionnelle", winget_id: "SlackTechnologies.Slack", category: "communication", homepage: "https://slack.com/", size_mb: 250 },
  { id: "43", name: "Microsoft Teams", description: "Collaboration Microsoft", winget_id: "Microsoft.Teams", category: "communication", homepage: "https://www.microsoft.com/teams", size_mb: 400 },
  { id: "44", name: "Telegram", description: "Messagerie securisee", winget_id: "Telegram.TelegramDesktop", category: "communication", homepage: "https://telegram.org/", size_mb: 60 },
  // Securite
  { id: "50", name: "Malwarebytes", description: "Protection anti-malware", winget_id: "Malwarebytes.Malwarebytes", category: "securite", homepage: "https://www.malwarebytes.com/", size_mb: 200 },
  { id: "51", name: "Bitwarden", description: "Gestionnaire de mots de passe open source", winget_id: "Bitwarden.Bitwarden", category: "securite", homepage: "https://bitwarden.com/", size_mb: 80 },
  { id: "52", name: "KeePass", description: "Gestionnaire de mots de passe local", winget_id: "DominikReichl.KeePass", category: "securite", homepage: "https://keepass.info/", size_mb: 3 },
  { id: "53", name: "ProtonVPN", description: "VPN securise et prive", winget_id: "ProtonTechnologies.ProtonVPN", category: "securite", homepage: "https://protonvpn.com/", size_mb: 100 },
  { id: "54", name: "VeraCrypt", description: "Chiffrement de volumes", winget_id: "IDRIX.VeraCrypt", category: "securite", homepage: "https://www.veracrypt.fr/", size_mb: 35 },
  { id: "55", name: "GlassWire", description: "Moniteur reseau et firewall", winget_id: "GlassWire.GlassWire", category: "securite", homepage: "https://www.glasswire.com/", size_mb: 60 },
  // Systeme
  { id: "60", name: "7-Zip", description: "Archiveur de fichiers multi-format", winget_id: "7zip.7zip", category: "systeme", homepage: "https://www.7-zip.org/", size_mb: 2 },
  { id: "61", name: "CPU-Z", description: "Informations detaillees sur le materiel", winget_id: "CPUID.CPU-Z", category: "systeme", homepage: "https://www.cpuid.com/softwares/cpu-z.html", size_mb: 3 },
  { id: "62", name: "HWiNFO64", description: "Monitoring materiel complet", winget_id: "REALiX.HWiNFO", category: "systeme", homepage: "https://www.hwinfo.com/", size_mb: 12 },
  { id: "63", name: "CrystalDiskInfo", description: "Sante des disques (SMART)", winget_id: "CrystalDewWorld.CrystalDiskInfo", category: "systeme", homepage: "https://crystalmark.info/", size_mb: 8 },
  { id: "64", name: "Autoruns", description: "Gestion des programmes au demarrage (Sysinternals)", winget_id: "Microsoft.Sysinternals.Autoruns", category: "systeme", homepage: "https://learn.microsoft.com/sysinternals/downloads/autoruns", size_mb: 2 },
  { id: "65", name: "Process Explorer", description: "Gestionnaire de processus avance (Sysinternals)", winget_id: "Microsoft.Sysinternals.ProcessExplorer", category: "systeme", homepage: "https://learn.microsoft.com/sysinternals/downloads/process-explorer", size_mb: 2 },
  { id: "66", name: "WinDirStat", description: "Visualisation espace disque", winget_id: "WinDirStat.WinDirStat", category: "systeme", homepage: "https://windirstat.net/", size_mb: 1 },
  { id: "67", name: "Everything", description: "Recherche de fichiers instantanee", winget_id: "voidtools.Everything", category: "systeme", homepage: "https://www.voidtools.com/", size_mb: 2 },
  { id: "68", name: "TreeSize Free", description: "Analyse taille des dossiers", winget_id: "JAMSoftware.TreeSize.Free", category: "systeme", homepage: "https://www.jam-software.com/treesize_free", size_mb: 10 },
  { id: "69", name: "PowerToys", description: "Outils systeme Microsoft avances", winget_id: "Microsoft.PowerToys", category: "systeme", homepage: "https://github.com/microsoft/PowerToys", size_mb: 120 },
  { id: "70", name: "Speccy", description: "Informations systeme completes", winget_id: "Piriform.Speccy", category: "systeme", homepage: "https://www.ccleaner.com/speccy", size_mb: 8 },
  { id: "71", name: "GPU-Z", description: "Informations carte graphique", winget_id: "TechPowerUp.GPU-Z", category: "systeme", homepage: "https://www.techpowerup.com/gpuz/", size_mb: 4 },
  { id: "72", name: "Rufus", description: "Creation de cles USB bootables", winget_id: "Rufus.Rufus", category: "systeme", homepage: "https://rufus.ie/", size_mb: 1 },
  { id: "73", name: "Ventoy", description: "Multi-boot USB open source", winget_id: "Ventoy.Ventoy", category: "systeme", homepage: "https://www.ventoy.net/", size_mb: 5 },
  { id: "74", name: "ShutUp10++", description: "Confidentialite Windows 10/11", winget_id: "OO-Software.ShutUp10", category: "systeme", homepage: "https://www.oo-software.com/en/shutup10", size_mb: 2 },
  // Reseau
  { id: "80", name: "WinSCP", description: "Client SFTP/FTP graphique", winget_id: "WinSCP.WinSCP", category: "reseau", homepage: "https://winscp.net/", size_mb: 15 },
  { id: "81", name: "PuTTY", description: "Client SSH et Telnet", winget_id: "PuTTY.PuTTY", category: "reseau", homepage: "https://www.putty.org/", size_mb: 2 },
  { id: "82", name: "Advanced IP Scanner", description: "Scanner reseau local rapide", winget_id: "Famatech.AdvancedIPScanner", category: "reseau", homepage: "https://www.advanced-ip-scanner.com/", size_mb: 10 },
  { id: "83", name: "Wireshark", description: "Analyseur de trafic reseau", winget_id: "WiresharkFoundation.Wireshark", category: "reseau", homepage: "https://www.wireshark.org/", size_mb: 65 },
  { id: "84", name: "FileZilla", description: "Client FTP/SFTP/FTPS", winget_id: "TimKosse.FileZilla.Client", category: "reseau", homepage: "https://filezilla-project.org/", size_mb: 15 },
  { id: "85", name: "qBittorrent", description: "Client BitTorrent open source", winget_id: "qBittorrent.qBittorrent", category: "reseau", homepage: "https://www.qbittorrent.org/", size_mb: 30 },
  { id: "86", name: "Free Download Manager", description: "Gestionnaire de telechargements", winget_id: "SoftdeluxeGroup.FreeDownloadManager", category: "reseau", homepage: "https://www.freedownloadmanager.org/", size_mb: 50 },
  { id: "87", name: "JDownloader 2", description: "Telechargeur automatique multi-hotes", winget_id: "AppWork.JDownloader", category: "reseau", homepage: "https://jdownloader.org/", size_mb: 100 },
  { id: "88", name: "NetLimiter 4", description: "Controle de bande passante", winget_id: "Locktime.NetLimiter.4", category: "reseau", homepage: "https://www.netlimiter.com/", size_mb: 20 },
  { id: "89", name: "mRemoteNG", description: "Gestionnaire de connexions distantes", winget_id: "mRemoteNG.mRemoteNG", category: "reseau", homepage: "https://mremoteng.org/", size_mb: 25 },
  { id: "90", name: "Cyberduck", description: "Client FTP/SFTP/WebDAV/Cloud", winget_id: "iterate.Cyberduck", category: "reseau", homepage: "https://cyberduck.io/", size_mb: 60 },
  { id: "91", name: "Nmap", description: "Scanner de ports et reseau", winget_id: "Nmap.Nmap", category: "reseau", homepage: "https://nmap.org/", size_mb: 25 },
  { id: "92", name: "Angry IP Scanner", description: "Scanner IP leger et rapide", winget_id: "AngryIPScanner.AngryIPScanner", category: "reseau", homepage: "https://angryip.org/", size_mb: 5 },
  { id: "93", name: "Speedtest CLI", description: "Test de vitesse internet (Ookla)", winget_id: "Ookla.Speedtest", category: "reseau", homepage: "https://www.speedtest.net/apps/cli", size_mb: 5 },
  // Gaming
  { id: "100", name: "Steam", description: "Plateforme de jeux PC principale", winget_id: "Valve.Steam", category: "gaming", homepage: "https://store.steampowered.com/", size_mb: 350 },
  { id: "101", name: "Epic Games Launcher", description: "Launcher Epic Games et jeux gratuits", winget_id: "EpicGames.EpicGamesLauncher", category: "gaming", homepage: "https://www.epicgames.com/", size_mb: 600 },
  { id: "102", name: "GOG Galaxy", description: "Client de jeux DRM-free GOG.com", winget_id: "GOG.Galaxy", category: "gaming", homepage: "https://www.gog.com/galaxy", size_mb: 250 },
  { id: "103", name: "Battle.net", description: "Launcher jeux Blizzard", winget_id: "Blizzard.BattleNet", category: "gaming", homepage: "https://www.blizzard.com/", size_mb: 400 },
  { id: "104", name: "Ubisoft Connect", description: "Launcher et social Ubisoft", winget_id: "Ubisoft.Connect", category: "gaming", homepage: "https://www.ubisoft.com/ubisoft-connect", size_mb: 300 },
  { id: "105", name: "EA App", description: "Launcher EA (remplace Origin)", winget_id: "ElectronicArts.EADesktop", category: "gaming", homepage: "https://www.ea.com/ea-app", size_mb: 300 },
  { id: "106", name: "Xbox App", description: "Xbox et Game Pass sur PC", winget_id: "Microsoft.GamingApp", category: "gaming", homepage: "https://www.xbox.com/", size_mb: 200 },
  { id: "107", name: "Heroic Games Launcher", description: "Launcher Epic + GOG open source", winget_id: "HeroicGamesLauncher.HeroicGamesLauncher", category: "gaming", homepage: "https://heroicgameslauncher.com/", size_mb: 150 },
  { id: "108", name: "Playnite", description: "Bibliotheque de jeux unifiee", winget_id: "Playnite.Playnite", category: "gaming", homepage: "https://playnite.link/", size_mb: 40 },
  { id: "109", name: "MSI Afterburner", description: "Overclock GPU et monitoring", winget_id: "Guru3D.Afterburner", category: "gaming", homepage: "https://www.msi.com/Landing/afterburner/", size_mb: 30 },
  { id: "110", name: "GeForce Experience", description: "Drivers et optimisations NVIDIA", winget_id: "Nvidia.GeForceExperience", category: "gaming", homepage: "https://www.nvidia.com/geforce/geforce-experience/", size_mb: 200 },
  { id: "111", name: "Parsec", description: "Remote gaming et bureau distant", winget_id: "Parsec.Parsec", category: "gaming", homepage: "https://parsec.app/", size_mb: 50 },
  { id: "112", name: "Moonlight", description: "Game streaming (NVIDIA GameStream)", winget_id: "MoonlightGameStreamingProject.Moonlight", category: "gaming", homepage: "https://moonlight-stream.org/", size_mb: 25 },
  { id: "113", name: "RetroArch", description: "Emulateur multi-systemes", winget_id: "Libretro.RetroArch", category: "gaming", homepage: "https://www.retroarch.com/", size_mb: 30 },
  { id: "114", name: "Dolphin Emulator", description: "Emulateur GameCube/Wii", winget_id: "DolphinEmu.Dolphin", category: "gaming", homepage: "https://dolphin-emu.org/", size_mb: 45 },
  { id: "115", name: "Cheat Engine", description: "Modification de valeurs en memoire", winget_id: "CheatEngine.CheatEngine", category: "gaming", homepage: "https://www.cheatengine.org/", size_mb: 15 },
  { id: "116", name: "Razer Cortex", description: "Optimiseur gaming Razer", winget_id: "RazerInc.RazerCortex", category: "gaming", homepage: "https://www.razer.com/cortex", size_mb: 80 },
];

// installedMap = union de installedIds (installé dans session) et éventuellement détecté
const installedMap = computed<Record<string, boolean>>(() => {
  const m: Record<string, boolean> = {};
  for (const id of installedIds.value) m[id] = true;
  return m;
});

function mapCategory(raw: string): string {
  return CATEGORY_MAP[normalizeStr(raw)] ?? normalizeStr(raw);
}

function formatSize(mb: number): string {
  if (mb >= 1000) return `${(mb / 1000).toFixed(1)} GB`;
  return `${mb} MB`;
}

async function loadApps() {
  loading.value = true;
  try {
    const wg = await invoke<boolean>("check_winget");
    wingetOk.value = wg;
    const raw = await cachedInvoke<AppInfo[]>("get_apps");
    apps.value = raw.map(a => ({ ...a, category: mapCategory(a.category) }));
  } catch {
    wingetOk.value = false;
    apps.value = devApps;
  }
  loading.value = false;
}

async function installApp(app: AppInfo) {
  if (installingIds.value.has(app.id)) return;
  installingIds.value = new Set([...installingIds.value, app.id]);
  installLogs.value[app.id] = [];
  try {
    await invokeRaw("install_app", { wingetId: app.winget_id });
    installedIds.value = new Set([...installedIds.value, app.id]);
    notifications.success(`${app.name} installé`);
  } catch {
    // Simulation dev
    await new Promise((r) => setTimeout(r, 2000));
    installLogs.value[app.id].push(`Installation de ${app.name}...`);
    await new Promise((r) => setTimeout(r, 1000));
    installedIds.value = new Set([...installedIds.value, app.id]);
    notifications.success(`${app.name} installé`);
  }
  installingIds.value = new Set([...installingIds.value].filter(id => id !== app.id));
}

async function uninstallApp(app: AppInfo) {
  const confirmed = window.confirm(`Désinstaller ${app.name} ?\n\nCette action supprimera l'application de votre système.`);
  if (!confirmed) return;

  uninstallingIds.value = new Set([...uninstallingIds.value, app.id]);
  try {
    await invoke("run_system_command", {
      cmd: "winget",
      args: ["uninstall", "--id", app.winget_id, "--silent"],
    });
    installedIds.value = new Set([...installedIds.value].filter(id => id !== app.id));
    notifications.success(`${app.name} désinstallé`);
  } catch (e) {
    notifications.error(`Erreur désinstallation ${app.name}`, String(e));
  }
  uninstallingIds.value = new Set([...uninstallingIds.value].filter(id => id !== app.id));
}

async function checkAppUpdate(app: AppInfo) {
  checkingUpdateIds.value = new Set([...checkingUpdateIds.value, app.id]);
  try {
    const result = await invoke<any>("run_system_command", {
      cmd: "winget",
      args: ["upgrade", "--id", app.winget_id],
    });
    const out: string = result?.stdout ?? result?.output ?? "";
    if (out.includes("No applicable upgrade")) {
      notifications.success(`${app.name} est à jour`);
    } else if (out.trim()) {
      notifications.info(`MAJ disponible pour ${app.name}`, out.split("\n").slice(0, 3).join(" "));
    } else {
      notifications.info(`Vérification terminée pour ${app.name}`);
    }
  } catch (e) {
    notifications.error(`Impossible de vérifier MAJ pour ${app.name}`, String(e));
  }
  checkingUpdateIds.value = new Set([...checkingUpdateIds.value].filter(id => id !== app.id));
}

async function openHomepage(url: string) {
  try {
    await invoke("open_url", { url });
  } catch {
    window.open(url, "_blank");
  }
}

function getCategoryIcon(category: string) {
  return categoryIcons[category] || Package;
}

// Set des IDs d'apps dont l'icône a échoué → bascule sur l'icône Lucide
const iconErrors = ref<Set<string>>(new Set());

function sanitizeWingetId(id: string | null | undefined): string {
  if (id == null) return "unknown";
  return id.toLowerCase().replace(/[^a-z0-9]/g, '_');
}

// Map winget_id → domaine pour les apps sans URL dans programs.json
const WINGET_DOMAIN: Record<string, string> = {
  // Navigateurs
  "Mozilla.Firefox": "mozilla.org", "Brave.Brave": "brave.com",
  "Microsoft.Edge": "microsoft.com", "Opera.OperaGX": "opera.com",
  // Bureautique
  "Microsoft.Office": "microsoft.com", "Kingsoft.WPSOffice": "wps.com",
  "Foxit.FoxitReader": "foxit.com", "Microsoft.OneNote": "microsoft.com",
  "dotPDN.PaintDotNet": "getpaint.net", "Obsidian.Obsidian": "obsidian.md",
  "Notion.Notion": "notion.so", "TheDocumentFoundation.LibreOffice": "libreoffice.org",
  "TheDocumentFoundation.LibreOffice.Draw": "libreoffice.org",
  "Notepad++.Notepad++": "notepad-plus-plus.org",
  "Trello.Trello": "trello.com", "Monday.Monday": "monday.com",
  "Asana.Asana": "asana.com", "Automattic.Simplenote": "simplenote.com",
  "geeksoftwareGmbH.PDF24Creator": "pdf24.org", "Softland.doPDF": "dopdf.com",
  // Dev
  "Microsoft.VisualStudioCode": "code.visualstudio.com",
  "Microsoft.VisualStudio.2022.Community": "visualstudio.microsoft.com",
  "Git.Git": "git-scm.com", "OpenJS.NodeJS.LTS": "nodejs.org",
  "Python.Python.3.12": "python.org", "Docker.DockerDesktop": "docker.com",
  "Postman.Postman": "postman.com", "GitHub.GitHubDesktop": "desktop.github.com",
  "Axosoft.GitKraken": "gitkraken.com", "Insomnia.Insomnia": "insomnia.rest",
  "Google.AndroidStudio": "developer.android.com",
  "Telerik.Fiddler.Classic": "telerik.com", "Microsoft.WindowsTerminal": "microsoft.com",
  "Oracle.VirtualBox": "virtualbox.org", "VMware.WorkstationPlayer": "vmware.com",
  "Hashicorp.Vagrant": "vagrantup.com",
  // Multimedia
  "VideoLAN.VLC": "videolan.org", "OBSProject.OBSStudio": "obsproject.com",
  "GIMP.GIMP": "gimp.org", "Audacity.Audacity": "audacityteam.org",
  "Spotify.Spotify": "spotify.com", "HandBrake.HandBrake": "handbrake.fr",
  "PeterPawlowski.foobar2000": "foobar2000.org", "Plex.PlexMediaServer": "plex.tv",
  "KDE.Krita": "krita.org", "Inkscape.Inkscape": "inkscape.org",
  "Meltytech.Shotcut": "shotcut.org", "OpenShot.OpenShot": "openshot.org",
  "NickeManarin.ScreenToGif": "screentogif.com", "ShareX.ShareX": "getsharex.com",
  "Greenshot.Greenshot": "getgreenshot.org", "Streamlabs.Streamlabs": "streamlabs.com",
  "clsid2.mpc-hc": "mpc-hc.org", "PandoraTV.KMPlayer": "kmplayer.com",
  "th-ch.YouTubeMusic": "youtube.com", "9NBLGGH5W6HK": "music.youtube.com",
  "9NBLGGH6JNLR": "tidal.com", "Apple.iTunes": "apple.com",
  "BlenderFoundation.Blender": "blender.org",
  "SerifEurope.AffinityDesigner2": "affinity.serif.com",
  "SerifEurope.AffinityPhoto2": "affinity.serif.com",
  "SerifEurope.AffinityPublisher2": "affinity.serif.com",
  "Figma.Figma": "figma.com", "Canva.Canva": "canva.com",
  "AaronFeng753.Waifu2x-Extension-GUI": "github.com",
  "Upscayl.Upscayl": "upscayl.org",
  // Communication
  "Discord.Discord": "discord.com", "Zoom.Zoom": "zoom.us",
  "SlackTechnologies.Slack": "slack.com", "Microsoft.Teams": "microsoft.com",
  "Telegram.TelegramDesktop": "telegram.org",
  "OpenWhisperSystems.Signal": "signal.org", "MeetFranz.Franz": "meetfranz.com",
  "Mozilla.Thunderbird": "thunderbird.net", "Mailbird.Mailbird": "getmailbird.com",
  "9WZDNCRFJ2WL": "facebook.com", "9NH2GPH4JZS4": "tiktok.com",
  "9WZDNCRFJ0J7": "snapchat.com", "9WZDNCRFJ140": "x.com",
  "Twitch.Twitch": "twitch.tv", "9P6RC76MSMMJ": "primevideo.com",
  // Securite
  "NortonLifeLock.Norton360": "norton.com",
  "NortonLifeLock.NortonRemoveAndReinstall": "norton.com",
  "GlassWire.GlassWire": "glasswire.com", "Avira.Avira": "avira.com",
  "XPDC2RH70K22MN": "avast.com", "Avast.Avast": "avast.com",
  "Malwarebytes.Malwarebytes": "malwarebytes.com",
  "Bitwarden.Bitwarden": "bitwarden.com", "DominikReichl.KeePass": "keepass.info",
  "ProtonTechnologies.ProtonVPN": "protonvpn.com", "IDRIX.VeraCrypt": "veracrypt.fr",
  // Systeme
  "7zip.7zip": "7-zip.org", "RARLab.WinRAR": "rarlab.com",
  "M2Team.NanaZip": "github.com",
  "CPUID.CPU-Z": "cpuid.com", "REALiX.HWiNFO": "hwinfo.com",
  "CrystalDewWorld.CrystalDiskInfo": "crystalmark.info",
  "CrystalDewWorld.CrystalDiskMark": "crystalmark.info",
  "Microsoft.Sysinternals.Autoruns": "microsoft.com",
  "Microsoft.Sysinternals.ProcessExplorer": "microsoft.com",
  "WinDirStat.WinDirStat": "windirstat.net", "voidtools.Everything": "voidtools.com",
  "JAMSoftware.TreeSize.Free": "jam-software.com", "Microsoft.PowerToys": "microsoft.com",
  "Piriform.Speccy": "ccleaner.com", "TechPowerUp.GPU-Z": "techpowerup.com",
  "Rufus.Rufus": "rufus.ie", "Ventoy.Ventoy": "ventoy.net",
  "Balena.Etcher": "etcher.balena.io", "TeamViewer.TeamViewer": "teamviewer.com",
  "GlavSoft.TightVNC": "tightvnc.com",
  "Piriform.Recuva": "ccleaner.com", "Piriform.Defraggler": "ccleaner.com",
  "Glarysoft.GlaryUtilities": "glarysoft.com", "BleachBit.BleachBit": "bleachbit.org",
  "AntibodySoftware.WizTree": "diskanalyzer.com",
  "Eraser.Eraser": "eraser.heidi.ie", "Auslogics.DiskDefrag": "auslogics.com",
  "Auslogics.RegistryCleaner": "auslogics.com", "Goversoft.PrivaZer": "privazer.com",
  "ORPALIS.PaperScan": "orpalis.com",
  "MarekOtulakowski.BulkCrapUninstaller": "bcuninstaller.com",
  "RevoUninstaller.RevoUninstaller": "revouninstaller.com",
  "IObit.IObitUninstaller": "iobit.com",
  "CGSecurity.TestDisk": "cgsecurity.org", "Cleverfiles.DiskDrill": "cleverfiles.com",
  "Maxon.CinebenchR23": "maxon.net", "Geeks3D.FurMark": "geeks3d.com",
  "FinalWire.AIDA64Extreme": "aida64.com",
  "Ookla.Speedtest.Desktop": "speedtest.net", "nPerf.nPerf": "nperf.com",
  "OO-Software.ShutUp10": "oo-software.com",
  // Reseau
  "WinSCP.WinSCP": "winscp.net", "PuTTY.PuTTY": "putty.org",
  "Famatech.AdvancedIPScanner": "advanced-ip-scanner.com",
  "WiresharkFoundation.Wireshark": "wireshark.org",
  "TimKosse.FileZilla.Client": "filezilla-project.org",
  "qBittorrent.qBittorrent": "qbittorrent.org",
  "SoftdeluxeGroup.FreeDownloadManager": "freedownloadmanager.org",
  "AppWork.JDownloader": "jdownloader.org", "Locktime.NetLimiter.4": "netlimiter.com",
  "mRemoteNG.mRemoteNG": "mremoteng.org", "iterate.Cyberduck": "cyberduck.io",
  "Nmap.Nmap": "nmap.org", "AngryIPScanner.AngryIPScanner": "angryip.org",
  "BitTorrent.uTorrent": "utorrent.com", "aria2.aria2": "aria2.github.io",
  "agalwood.Motrix": "motrix.app",
  // Gaming
  "Valve.Steam": "store.steampowered.com", "EpicGames.EpicGamesLauncher": "epicgames.com",
  "GOG.Galaxy": "gog.com", "Blizzard.BattleNet": "blizzard.com",
  "Ubisoft.Connect": "ubisoft.com", "ElectronicArts.EADesktop": "ea.com",
  "ElectronicArts.Origin": "ea.com",
  "Microsoft.GamingApp": "xbox.com", "Nvidia.GeForceExperience": "nvidia.com",
  "Guru3D.Afterburner": "msi.com", "Parsec.Parsec": "parsec.app",
  "MoonlightGameStreamingProject.Moonlight": "moonlight-stream.org",
  "HeroicGamesLauncher.HeroicGamesLauncher": "heroicgameslauncher.com",
  "Playnite.Playnite": "playnite.link",
  "Ryochan7.DS4Windows": "ds4-windows.com",
  // IA
  "OpenAI.ChatGPT": "openai.com", "Ollama.Ollama": "ollama.com",
  "LMStudio.LMStudio": "lmstudio.ai", "Jan.Jan": "jan.ai",
  "MintplexLabs.AnythingLLM": "anythingllm.com",
  "Const-me.Whisper": "github.com", "LykosAI.StabilityMatrix": "lykos.ai",
  // Pro / Creation
  "Adobe.CreativeCloud": "adobe.com", "Adobe.Acrobat.Reader.64-bit": "adobe.com",
  "Autodesk.AutodeskDesktopApp": "autodesk.com", "Autodesk.AutoCAD": "autodesk.com",
  "Trimble.SketchUp": "sketchup.com", "Corel.CorelDRAW": "corel.com",
};

// Nettoie les URL de téléchargement : strip download./dl./cdn. et paths
function extractDomain(rawUrl: string): string {
  try {
    const host = new URL(rawUrl).hostname;
    // Strip subdomains de téléchargement courants
    const cleaned = host.replace(/^(download|dl|cdn|get|files|releases|static)\./i, "");
    // Pour github.com/user/repo → garder github.com
    return cleaned;
  } catch { return ""; }
}

// Résout le meilleur domaine pour une app : homepage > url nettoyé > winget_id map
function resolveAppDomain(app: AppInfo): string {
  if (app.homepage) {
    try { return new URL(app.homepage).hostname; } catch { /* */ }
  }
  if (app.url) {
    const d = extractDomain(app.url);
    if (d) return d;
  }
  if (app.winget_id && WINGET_DOMAIN[app.winget_id]) {
    return WINGET_DOMAIN[app.winget_id];
  }
  return "";
}

// Ordre de priorité : local → Google Favicons → DuckDuckGo → icône Lucide
function appFaviconUrl(app: AppInfo): string {
  if (!iconErrors.value.has(app.id + "_local")) {
    return `/icons/apps/${sanitizeWingetId(app.winget_id)}.png`;
  }
  const domain = resolveAppDomain(app);
  if (!domain) return "";
  if (iconErrors.value.has(app.id + "_google")) {
    if (iconErrors.value.has(app.id + "_ddg")) return "";
    return `https://icons.duckduckgo.com/ip3/${domain}.ico`;
  }
  return `https://www.google.com/s2/favicons?domain=${domain}&sz=64`;
}

function onIconError(event: Event, app: AppInfo) {
  const src = (event.target as HTMLImageElement).src;
  if (src.includes("/icons/apps/")) {
    iconErrors.value = new Set([...iconErrors.value, app.id + "_local"]);
  } else if (src.includes("google.com/s2/favicons")) {
    iconErrors.value = new Set([...iconErrors.value, app.id + "_google"]);
  } else {
    iconErrors.value = new Set([...iconErrors.value, app.id + "_ddg"]);
  }
}

onMounted(async () => {
  await loadApps();
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlistenLog = await listen<{ app_id: string; line: string }>("install-log", (event) => {
      const { app_id, line } = event.payload;
      if (!installLogs.value[app_id]) installLogs.value[app_id] = [];
      installLogs.value[app_id].push(line);
    });
  } catch {
    // Mode dev
  }
});

onUnmounted(() => {
  if (unlistenLog) unlistenLog();
});
</script>

<template>
  <div class="apps-page">
    <!-- Header -->
    <div class="page-header">
      <div>
        <h1>Applications</h1>
        <p class="page-subtitle">Installer des applications via WinGet</p>
      </div>
      <div class="header-actions">
        <NBadge :variant="wingetOk ? 'success' : 'danger'">
          <Package :size="12" style="margin-right: 4px;" />
          WinGet {{ wingetOk ? "disponible" : "non detecte" }}
        </NBadge>
        <NButton variant="secondary" size="sm" :loading="loading" @click="loadApps">
          <RefreshCw :size="14" />
          Rafraichir
        </NButton>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="loading-state">
      <NSpinner :size="32" />
      <p>Chargement des applications...</p>
    </div>

    <template v-else>
      <!-- Search + Tabs -->
      <div class="toolbar">
        <NSearchBar v-model="searchQuery" placeholder="Rechercher une application..." />
      </div>

      <NTabs :tabs="categoryTabs" v-model="activeCategory">
        <template #default="{ activeTab }">
          <!-- Apps Grid -->
          <div v-if="filteredApps.length" class="apps-grid">
            <div v-for="app in filteredApps" :key="app.id" class="app-card">
              <div class="app-icon-wrap">
                <img
                  v-if="appFaviconUrl(app)"
                  :src="appFaviconUrl(app)"
                  :alt="app.name"
                  class="app-favicon"
                  loading="lazy"
                  @error="onIconError($event, app)"
                />
                <component
                  v-else
                  :is="getCategoryIcon(app.category)"
                  :size="24"
                  class="app-fallback-icon"
                />
              </div>
              <div class="app-info">
                <div class="app-name-row">
                  <span class="app-name">{{ app.name }}</span>
                  <!-- Lien homepage -->
                  <button
                    v-if="app.homepage"
                    class="homepage-btn"
                    :title="`Site officiel de ${app.name}`"
                    @click.stop="openHomepage(app.homepage!)"
                  >
                    <Globe :size="12" />
                  </button>
                </div>
                <!-- Version installée -->
                <div v-if="installedMap[app.id] && (app.version || installedVersions[app.id])" class="app-version">
                  v{{ installedVersions[app.id] || app.version }}
                </div>
                <div class="app-desc">{{ app.description }}</div>
                <div class="app-winget-row">
                  <span class="app-winget font-mono">{{ app.winget_id }}</span>
                  <!-- Badge taille -->
                  <NBadge v-if="app.size_mb" variant="neutral" class="size-badge">
                    <HardDrive :size="10" style="margin-right: 3px;" />
                    {{ formatSize(app.size_mb) }}
                  </NBadge>
                </div>
              </div>
              <div class="app-action">
                <!-- App installée -->
                <template v-if="installedMap[app.id]">
                  <div class="installed-actions">
                    <NBadge variant="success" style="margin-bottom: 4px;">
                      <CheckCircle :size="11" style="margin-right: 3px;" />
                      Installe
                    </NBadge>
                    <NButton
                      variant="secondary"
                      size="sm"
                      :loading="checkingUpdateIds.has(app.id)"
                      @click="checkAppUpdate(app)"
                      title="Vérifier mise à jour"
                    >
                      <Search :size="12" />
                      Vérifier MAJ
                    </NButton>
                    <NButton
                      variant="danger"
                      size="sm"
                      :loading="uninstallingIds.has(app.id)"
                      @click="uninstallApp(app)"
                    >
                      <Trash2 :size="12" />
                      Désinstaller
                    </NButton>
                  </div>
                </template>
                <!-- App non installée -->
                <NButton
                  v-else
                  variant="primary"
                  size="sm"
                  :loading="installingIds.has(app.id)"
                  @click="installApp(app)"
                >
                  <Download :size="14" />
                  Installer
                </NButton>
              </div>
            </div>
          </div>

          <div v-else class="empty-state">
            Aucune application trouvee pour cette recherche.
          </div>
        </template>
      </NTabs>
    </template>
  </div>
</template>

<style scoped src="./ApplicationsPage.css"></style>