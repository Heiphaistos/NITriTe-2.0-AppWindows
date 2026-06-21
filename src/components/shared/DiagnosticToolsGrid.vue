<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import { useNotificationStore } from "@/stores/notifications";

const notify = useNotificationStore();

interface PortableInfo { id: string; name: string; }
interface ToolDef {
  label: string;
  emoji: string;
  type: "portable" | "exe" | "url" | "cmd" | "battery";
  match?: string;        // portable: id.includes(match)
  exclude?: string;      // portable: !id.includes(exclude)
  path?: string;         // exe: relative from logiciel/
  url?: string;          // url
  cmd?: string;          // cmd: passed to execute_tool
  officialUrl?: string;  // portable: fallback si non trouvé
}

const TOOLS: ToolDef[] = [
  { label: "Test Clavier AZERTY",             emoji: "⌨️",  type: "url",      url: "https://www.test-clavier.fr/" },
  { label: "CrystalDiskInfo",                 emoji: "💿",  type: "portable", match: "crystaldisk",    exclude: "mark", officialUrl: "https://crystalmark.info/en/software/crystaldiskinfo/" },
  { label: "OCCT (Temp & Stress)",             emoji: "🌡️",  type: "url",      url: "https://www.ocbase.com/download" },
  { label: "Test Batterie OrdiPlus",           emoji: "🔋",  type: "exe",      path: "Standalone Tools/Ordi Plus - Battery Tester.exe" },
  { label: "Test Batterie NiTrite",            emoji: "⚡",  type: "battery" },
  { label: "Autoruns",                         emoji: "🚀",  type: "portable", match: "autoruns",      officialUrl: "https://learn.microsoft.com/en-us/sysinternals/downloads/autoruns" },
  { label: "Malwarebytes Portable",            emoji: "🛡️",  type: "portable", match: "malwarebytes",  officialUrl: "https://www.malwarebytes.com/mwb-download/" },
  { label: "Spybot Search & Destroy",          emoji: "🕵️",  type: "portable", match: "spybot" },
  { label: "BulkCrap Uninstaller",         emoji: "🗑️",  type: "portable", match: "bcuninstaller" },
  { label: "GetDataBack (Récupération)",   emoji: "💾",  type: "portable", match: "getdataback" },
  { label: "WinDirStat",                   emoji: "📊",  type: "url",      url: "https://windirstat.net/download.html" },
  { label: "Process Explorer (Sysinternals)",emoji:"🔍", type: "portable", match: "processexplorer", officialUrl: "https://learn.microsoft.com/en-us/sysinternals/downloads/process-explorer" },
  { label: "AdwCleaner Portable",              emoji: "🧹",  type: "portable", match: "adwcleaner",     officialUrl: "https://www.malwarebytes.com/adwcleaner" },
  { label: "Wise Disk Cleaner",                emoji: "💾",  type: "portable", match: "wisedisk" },
  { label: "HWMonitor",                        emoji: "📊",  type: "portable", match: "hwmonitor",     exclude: "portable", officialUrl: "https://www.cpuid.com/softwares/hwmonitor.html" },
  { label: "HWinfo",                           emoji: "🖥️",  type: "portable", match: "hwinfo",        officialUrl: "https://www.hwinfo.com/download/" },
  { label: "CrystalDiskMark",                  emoji: "⚡",  type: "portable", match: "crystaldiskmark", officialUrl: "https://crystalmark.info/en/software/crystaldiskmark/" },
  { label: "CPU-Z",                            emoji: "🔬",  type: "portable", match: "cpu",            officialUrl: "https://www.cpuid.com/softwares/cpu-z.html" },
  { label: "GPU-Z",                            emoji: "🎮",  type: "exe",      path: "Standalone Tools/GPU-Z.0.7.0.exe" },
  { label: "Wise Care 365",                    emoji: "✨",  type: "portable", match: "wisecare" },
  { label: "UserDiag (Diagnostic Complet)",    emoji: "🔍",  type: "portable", match: "userdiag" },
  { label: "BenchMaster.AI",                   emoji: "🤖",  type: "url",      url: "https://benchmaster.ai/" },
  { label: "Activation Windows/Office",        emoji: "🔑",  type: "cmd",      cmd: "ms-settings:activation" },
  { label: "MSCONFIG",                         emoji: "⚙️",  type: "cmd",      cmd: "msconfig" },
  { label: "Gestionnaire des Tâches",          emoji: "📋",  type: "cmd",      cmd: "taskmgr" },
  { label: "i MSINFO",                         emoji: "ℹ️",  type: "cmd",      cmd: "msinfo32" },
  { label: "Dossier Temp",                     emoji: "📁",  type: "cmd",      cmd: "explorer %TEMP%" },
  { label: "AppData Local",                    emoji: "📂",  type: "cmd",      cmd: "explorer %LOCALAPPDATA%" },
  { label: "Version Windows",                  emoji: "🪟",  type: "cmd",      cmd: "winver" },
  { label: "Tout Mettre à Jour",               emoji: "🔄",  type: "cmd",      cmd: "start cmd /k winget upgrade --all --include-unknown" },
  { label: "Drivers NVIDIA",                   emoji: "🟢",  type: "url",      url: "https://www.nvidia.com/fr-fr/drivers/" },
  { label: "Drivers AMD",                      emoji: "🔴",  type: "url",      url: "https://www.amd.com/fr/support/download/drivers.html" },
  { label: "Réparer Image Windows",            emoji: "🔧",  type: "cmd",      cmd: "start cmd /k DISM /Online /Cleanup-Image /RestoreHealth" },
  { label: "Propriétés Utilisateur",           emoji: "👤",  type: "cmd",      cmd: "netplwiz" },
  { label: "Système",                          emoji: "🖥️",  type: "cmd",      cmd: "sysdm.cpl" },
  { label: "CHKDSK Complet",                   emoji: "💽",  type: "cmd",      cmd: "start cmd /k chkdsk C: /f /r" },
];

const portables = ref<PortableInfo[]>([]);
const loading = ref<string | null>(null); // label of loading tool

function findPortableId(match: string, exclude?: string): string | undefined {
  const m = match.toLowerCase();
  const x = exclude?.toLowerCase();
  return portables.value.find(p => {
    const id = p.id;
    return id.includes(m) && (!x || !id.includes(x));
  })?.id;
}

async function launchTool(tool: ToolDef) {
  loading.value = tool.label;
  try {

    if (tool.type === "url") {
      await invoke("open_url", { url: tool.url });

    } else if (tool.type === "portable") {
      const appId = findPortableId(tool.match!, tool.exclude);
      if (!appId) {
        if (tool.officialUrl) {
          notify.info(`${tool.label} non trouvé — ouverture du site officiel`, "");
          await invoke("open_url", { url: tool.officialUrl });
        } else {
          notify.warning(`${tool.label} non trouvé`, "Vérifiez que l'outil est dans le dossier logiciel/");
        }
        return;
      }
      await invoke("launch_portable", { appId });

    } else if (tool.type === "exe") {
      await invoke("launch_exe", { relativePath: tool.path });

    } else if (tool.type === "cmd") {
      const cmd = tool.cmd!;
      const isUrl = cmd.startsWith("ms-settings:");
      await invoke("execute_tool", { command: cmd, isUrl });

    } else if (tool.type === "battery") {
      const path = await invoke<string>("run_battery_report");
      await invoke("open_path", { path });
      notify.success("Rapport batterie généré", "Ouverture dans le navigateur...");
    }
  } catch (e: any) {
    notify.error(`Erreur: ${tool.label}`, e?.toString());
  } finally {
    loading.value = null;
  }
}

onMounted(async () => {
  try {
    portables.value = await invoke<PortableInfo[]>("get_portable_apps");
  } catch {
    // dev fallback
  }
});
</script>

<template>
  <div class="tools-grid-wrap">
    <div class="tools-grid">
      <button
        v-for="tool in TOOLS"
        :key="tool.label"
        class="tool-btn"
        :class="{ 'tool-btn--loading': loading === tool.label }"
        @click="launchTool(tool)"
        :disabled="loading !== null"
      >
        <span class="tool-emoji">{{ tool.emoji }}</span>
        <span class="tool-label">{{ tool.label }}</span>
        <span v-if="loading === tool.label" class="tool-spinner" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.tools-grid-wrap {
  padding: 4px 0;
}

.tools-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}

@media (max-width: 900px) {
  .tools-grid { grid-template-columns: repeat(2, 1fr); }
}

@media (max-width: 560px) {
  .tools-grid { grid-template-columns: 1fr; }
}

.tool-btn {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background: var(--bg-secondary);
  border: 1.5px solid var(--accent-primary);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
  text-align: left;
  width: 100%;
  min-height: 44px;
  position: relative;
}

.tool-btn:hover:not(:disabled) {
  background: var(--accent-muted);
  border-color: var(--accent-primary);
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(var(--accent-primary-rgb, 124, 106, 247), 0.25);
}

.tool-btn:active:not(:disabled) {
  transform: translateY(0);
}

.tool-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.tool-btn--loading {
  border-color: var(--accent-primary);
  background: var(--accent-muted);
}

.tool-emoji {
  font-size: 18px;
  flex-shrink: 0;
  width: 24px;
  text-align: center;
}

.tool-label {
  flex: 1;
  line-height: 1.3;
}

.tool-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid var(--accent-primary);
  border-top-color: transparent;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
  flex-shrink: 0;
}

@keyframes spin { to { transform: rotate(360deg); } }
</style>
