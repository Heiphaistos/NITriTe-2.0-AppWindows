/**
 * download-icons.mjs
 * Télécharge les favicons de toutes les apps (portables + installables)
 * et les stocke dans public/icons/ pour fonctionner hors-ligne.
 *
 * Usage: node scripts/download-icons.mjs
 */

import { readFileSync, mkdirSync, writeFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..');
const ICONS_PORTABLES = join(ROOT, 'public/icons/portables');
const ICONS_APPS      = join(ROOT, 'public/icons/apps');

mkdirSync(ICONS_PORTABLES, { recursive: true });
mkdirSync(ICONS_APPS,      { recursive: true });

// ── Helpers ───────────────────────────────────────────────────────────────────

function domainFromUrl(url) {
  try { return new URL(url).hostname; } catch { return ''; }
}

function sanitize(id) {
  return id.toLowerCase().replace(/[^a-z0-9]/g, '_');
}

let ok = 0, skip = 0, fail = 0;

async function downloadIcon(domain, destPath, label) {
  if (existsSync(destPath)) { skip++; return; }
  if (!domain) { fail++; console.warn(`  ✗ [no domain] ${label}`); return; }

  const url = `https://www.google.com/s2/favicons?domain=${domain}&sz=64`;
  try {
    const resp = await fetch(url, { signal: AbortSignal.timeout(8000) });
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
    const buf = await resp.arrayBuffer();
    // Google returns a 16×16 grey pixel if the favicon isn't found — skip tiny files
    if (buf.byteLength < 200) {
      // Try DuckDuckGo as fallback
      const resp2 = await fetch(`https://icons.duckduckgo.com/ip3/${domain}.ico`, {
        signal: AbortSignal.timeout(8000),
      });
      if (resp2.ok) {
        const buf2 = await resp2.arrayBuffer();
        if (buf2.byteLength > 200) {
          writeFileSync(destPath, Buffer.from(buf2));
          ok++;
          console.log(`  ✓ DDG  ${domain}`);
          return;
        }
      }
      skip++;
      console.log(`  ~ tiny ${domain} (no real favicon)`);
      return;
    }
    writeFileSync(destPath, Buffer.from(buf));
    ok++;
    console.log(`  ✓ ${domain}`);
  } catch (e) {
    fail++;
    console.warn(`  ✗ ${domain}: ${e.message}`);
  }
}

async function batchDownload(entries, batchSize = 15) {
  for (let i = 0; i < entries.length; i += batchSize) {
    await Promise.all(
      entries.slice(i, i + batchSize).map(e => downloadIcon(e.domain, e.dest, e.label))
    );
  }
}

// ── 1. Applications portables ─────────────────────────────────────────────────

console.log('\n=== Applications Portables ===');

const portableFiles = [
  'src/data/portableApps.ts',
  'src/data/portable/cat_systeme.ts',
  'src/data/portable/cat_reseau.ts',
  'src/data/portable/cat_dev.ts',
  'src/data/portable/cat_utils.ts',
  'src/data/portable/cat_media.ts',
  'src/data/portable/cat_bureau.ts',
  'src/data/portable/cat_extra.ts',
];

const portableEntries = [];
const seenPortable = new Set();

for (const file of portableFiles) {
  const content = readFileSync(join(ROOT, file), 'utf-8');
  // Match id:"..." + url:"..." in same object (multiline)
  const re = /id\s*:\s*"([^"]+)"[^}]*?url\s*:\s*"([^"]+)"/gs;
  let m;
  while ((m = re.exec(content)) !== null) {
    const id  = m[1];
    const url = m[2];
    if (seenPortable.has(id)) continue;
    seenPortable.add(id);
    portableEntries.push({
      domain: domainFromUrl(url),
      dest:   join(ICONS_PORTABLES, `${id}.png`),
      label:  id,
    });
  }
}

console.log(`  ${portableEntries.length} apps portables trouvées`);
await batchDownload(portableEntries);

// ── 2. Applications installables (winget) ─────────────────────────────────────

console.log('\n=== Applications Installables (winget) ===');

// Map winget_id → domain extrait de ApplicationsPage.vue
const WINGET_DOMAIN = {
  // Navigateurs
  "Google.Chrome": "google.com", "Mozilla.Firefox": "mozilla.org",
  "Brave.Brave": "brave.com", "Microsoft.Edge": "microsoft.com",
  "Opera.OperaGX": "opera.com",
  // Bureautique
  "TheDocumentFoundation.LibreOffice": "libreoffice.org",
  "Notepad++.Notepad++": "notepad-plus-plus.org",
  "Obsidian.Obsidian": "obsidian.md", "Notion.Notion": "notion.so",
  "Microsoft.Office": "microsoft.com", "Kingsoft.WPSOffice": "wps.com",
  "Foxit.FoxitReader": "foxit.com", "Microsoft.OneNote": "microsoft.com",
  "dotPDN.PaintDotNet": "getpaint.net",
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
  "Telerik.Fiddler.Classic": "telerik.com",
  "Microsoft.WindowsTerminal": "microsoft.com",
  "Oracle.VirtualBox": "virtualbox.org", "VMware.WorkstationPlayer": "vmware.com",
  "Hashicorp.Vagrant": "vagrantup.com",
  // Multimedia
  "VideoLAN.VLC": "videolan.org", "OBSProject.OBSStudio": "obsproject.com",
  "GIMP.GIMP": "gimp.org", "Audacity.Audacity": "audacityteam.org",
  "Spotify.Spotify": "spotify.com", "HandBrake.HandBrake": "handbrake.fr",
  "PeterPawlowski.foobar2000": "foobar2000.org",
  "KDE.Krita": "krita.org", "Inkscape.Inkscape": "inkscape.org",
  "Meltytech.Shotcut": "shotcut.org", "OpenShot.OpenShot": "openshot.org",
  "NickeManarin.ScreenToGif": "screentogif.com", "ShareX.ShareX": "getsharex.com",
  "Greenshot.Greenshot": "getgreenshot.org", "Streamlabs.Streamlabs": "streamlabs.com",
  "clsid2.mpc-hc": "mpc-hc.org", "PandoraTV.KMPlayer": "kmplayer.com",
  "Apple.iTunes": "apple.com", "BlenderFoundation.Blender": "blender.org",
  "SerifEurope.AffinityDesigner2": "affinity.serif.com",
  "SerifEurope.AffinityPhoto2": "affinity.serif.com",
  "SerifEurope.AffinityPublisher2": "affinity.serif.com",
  "Figma.Figma": "figma.com", "Canva.Canva": "canva.com",
  "Upscayl.Upscayl": "upscayl.org",
  // Communication
  "Discord.Discord": "discord.com", "Zoom.Zoom": "zoom.us",
  "SlackTechnologies.Slack": "slack.com", "Microsoft.Teams": "microsoft.com",
  "Telegram.TelegramDesktop": "telegram.org",
  "OpenWhisperSystems.Signal": "signal.org", "MeetFranz.Franz": "meetfranz.com",
  "Mozilla.Thunderbird": "thunderbird.net", "Mailbird.Mailbird": "getmailbird.com",
  "Twitch.Twitch": "twitch.tv",
  // Securite
  "GlassWire.GlassWire": "glasswire.com", "Avira.Avira": "avira.com",
  "Avast.Avast": "avast.com", "Malwarebytes.Malwarebytes": "malwarebytes.com",
  "Bitwarden.Bitwarden": "bitwarden.com", "DominikReichl.KeePass": "keepass.info",
  "ProtonTechnologies.ProtonVPN": "protonvpn.com", "IDRIX.VeraCrypt": "veracrypt.fr",
  "NortonLifeLock.Norton360": "norton.com",
  // Systeme
  "7zip.7zip": "7-zip.org", "RARLab.WinRAR": "rarlab.com",
  "M2Team.NanaZip": "github.com",
  "CPUID.CPU-Z": "cpuid.com", "REALiX.HWiNFO": "hwinfo.com",
  "CrystalDewWorld.CrystalDiskInfo": "crystalmark.info",
  "CrystalDewWorld.CrystalDiskMark": "crystalmark.info",
  "Microsoft.Sysinternals.Autoruns": "microsoft.com",
  "Microsoft.Sysinternals.ProcessExplorer": "microsoft.com",
  "WinDirStat.WinDirStat": "windirstat.net", "voidtools.Everything": "voidtools.com",
  "JAMSoftware.TreeSize.Free": "jam-software.com",
  "Microsoft.PowerToys": "microsoft.com",
  "Piriform.Speccy": "ccleaner.com", "TechPowerUp.GPU-Z": "techpowerup.com",
  "Rufus.Rufus": "rufus.ie", "Ventoy.Ventoy": "ventoy.net",
  "Balena.Etcher": "etcher.balena.io", "TeamViewer.TeamViewer": "teamviewer.com",
  "GlavSoft.TightVNC": "tightvnc.com",
  "Piriform.Recuva": "ccleaner.com", "Piriform.Defraggler": "ccleaner.com",
  "Glarysoft.GlaryUtilities": "glarysoft.com", "BleachBit.BleachBit": "bleachbit.org",
  "AntibodySoftware.WizTree": "diskanalyzer.com",
  "Eraser.Eraser": "eraser.heidi.ie",
  "MarekOtulakowski.BulkCrapUninstaller": "bcuninstaller.com",
  "RevoUninstaller.RevoUninstaller": "revouninstaller.com",
  "IObit.IObitUninstaller": "iobit.com",
  "CGSecurity.TestDisk": "cgsecurity.org", "Cleverfiles.DiskDrill": "cleverfiles.com",
  "Maxon.CinebenchR23": "maxon.net", "Geeks3D.FurMark": "geeks3d.com",
  "FinalWire.AIDA64Extreme": "aida64.com",
  "Ookla.Speedtest.Desktop": "speedtest.net",
  "OO-Software.ShutUp10": "oo-software.com",
  // Reseau
  "WinSCP.WinSCP": "winscp.net", "PuTTY.PuTTY": "putty.org",
  "Famatech.AdvancedIPScanner": "advanced-ip-scanner.com",
  "WiresharkFoundation.Wireshark": "wireshark.org",
  "TimKosse.FileZilla.Client": "filezilla-project.org",
  "qBittorrent.qBittorrent": "qbittorrent.org",
  "SoftdeluxeGroup.FreeDownloadManager": "freedownloadmanager.org",
  "AppWork.JDownloader": "jdownloader.org",
  "Locktime.NetLimiter.4": "netlimiter.com",
  "mRemoteNG.mRemoteNG": "mremoteng.org", "iterate.Cyberduck": "cyberduck.io",
  "Nmap.Nmap": "nmap.org", "AngryIPScanner.AngryIPScanner": "angryip.org",
  "BitTorrent.uTorrent": "utorrent.com", "agalwood.Motrix": "motrix.app",
  // Gaming
  "Valve.Steam": "store.steampowered.com",
  "EpicGames.EpicGamesLauncher": "epicgames.com",
  "GOG.Galaxy": "gog.com", "Blizzard.BattleNet": "blizzard.com",
  "Ubisoft.Connect": "ubisoft.com", "ElectronicArts.EADesktop": "ea.com",
  "Microsoft.GamingApp": "xbox.com",
  "Nvidia.GeForceExperience": "nvidia.com", "Guru3D.Afterburner": "msi.com",
  "Parsec.Parsec": "parsec.app",
  "MoonlightGameStreamingProject.Moonlight": "moonlight-stream.org",
  "HeroicGamesLauncher.HeroicGamesLauncher": "heroicgameslauncher.com",
  "Playnite.Playnite": "playnite.link",
  "Ryochan7.DS4Windows": "ds4-windows.com",
  "DolphinEmu.Dolphin": "dolphin-emu.org",
  "Libretro.RetroArch": "retroarch.com",
  "CheatEngine.CheatEngine": "cheatengine.org",
  "RazerInc.RazerCortex": "razer.com",
  // IA
  "OpenAI.ChatGPT": "openai.com", "Ollama.Ollama": "ollama.com",
  "LMStudio.LMStudio": "lmstudio.ai", "Jan.Jan": "jan.ai",
  "MintplexLabs.AnythingLLM": "anythingllm.com",
  "LykosAI.StabilityMatrix": "lykos.ai",
  // Pro / Creation
  "Adobe.CreativeCloud": "adobe.com",
  "Adobe.Acrobat.Reader.64-bit": "adobe.com",
  "Autodesk.AutoCAD": "autodesk.com", "Trimble.SketchUp": "sketchup.com",
  "Corel.CorelDRAW": "corel.com",
  // Speedtest
  "Ookla.Speedtest": "speedtest.net",
};

const wingetEntries = Object.entries(WINGET_DOMAIN).map(([wingetId, domain]) => ({
  domain,
  dest:  join(ICONS_APPS, `${sanitize(wingetId)}.png`),
  label: wingetId,
}));

console.log(`  ${wingetEntries.length} apps installables trouvées`);
await batchDownload(wingetEntries);

// ── Résumé ────────────────────────────────────────────────────────────────────

const total = portableEntries.length + wingetEntries.length;
console.log(`\n╔══════════════════════════════════════╗`);
console.log(`  Total: ${total} | ✓ ${ok} | ~ ${skip} déjà présentes | ✗ ${fail} échecs`);
console.log(`  Icônes portables → public/icons/portables/`);
console.log(`  Icônes apps     → public/icons/apps/`);
console.log(`╚══════════════════════════════════════╝\n`);
