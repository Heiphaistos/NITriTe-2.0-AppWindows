<template>
  <div class="winpe-page">
    <!-- Banner WinPE -->
    <div class="winpe-banner">
      <div class="banner-icon"><HardDrive :size="28" /></div>
      <div class="banner-content">
        <h1>Mode WinPE — Support &amp; Réparation Professionnelle</h1>
        <p v-if="sysInfo">{{ sysInfo.pe_version }} &bull; {{ sysInfo.cpu }} &bull; {{ sysInfo.ram_gb?.toFixed(1) ?? '?' }} GB RAM &bull; {{ sysInfo.arch }}</p>
        <p v-else class="loading-text">Détection de l'environnement en cours...</p>
      </div>
      <NBadge variant="warning" class="pe-badge">WinPE</NBadge>
    </div>

    <!-- Installation Windows détectée -->
    <div class="section-card" v-if="winInstalls.length > 0">
      <h2 class="section-title"><MonitorCheck :size="16" /> Installation Windows détectée</h2>
      <div class="install-list">
        <button v-for="inst in winInstalls" :key="inst.drive"
          class="install-card" :class="{ active: selectedInstall?.drive === inst.drive }"
          @click="selectedInstall = inst; sfcWindowsDir = inst.windows_dir">
          <div class="install-info">
            <span class="install-version">{{ inst.version }}</span>
            <span class="install-path">{{ inst.windows_dir }}</span>
            <span class="install-build">Build {{ inst.build }}</span>
          </div>
          <CheckCircle v-if="selectedInstall?.drive === inst.drive" :size="16" class="check-icon" />
        </button>
      </div>
    </div>
    <div class="section-card" v-else-if="sysInfo">
      <h2 class="section-title"><MonitorCheck :size="16" /> Installation Windows détectée</h2>
      <div class="empty-state">
        <MonitorCheck :size="28" style="opacity:.25" />
        <p>Aucune installation Windows détectée sur les disques connectés</p>
      </div>
    </div>

    <!-- Disques disponibles -->
    <div class="section-card" v-if="drives.length > 0">
      <h2 class="section-title"><HardDrive :size="16" /> Disques disponibles</h2>
      <div class="drives-grid">
        <div v-for="drive in drives" :key="drive.letter" class="drive-card">
          <div class="drive-letter">{{ drive.letter }}</div>
          <div class="drive-info">
            <span class="drive-label">{{ drive.label || 'Sans étiquette' }}</span>
            <span class="drive-fs">{{ drive.fs }}</span>
          </div>
          <div class="drive-space">
            <div class="drive-bar">
              <div class="drive-bar-fill" :style="{ width: driveUsedPercent(drive) + '%' }" :class="driveUsedPercent(drive) > 85 ? 'danger' : ''" />
            </div>
            <span class="drive-sizes">{{ drive.free_gb.toFixed(1) }} GB libre / {{ drive.size_gb.toFixed(1) }} GB</span>
          </div>
        </div>
      </div>
    </div>
    <div class="section-card" v-else-if="sysInfo">
      <h2 class="section-title"><HardDrive :size="16" /> Disques disponibles</h2>
      <div class="empty-state">
        <HardDrive :size="28" style="opacity:.25" />
        <p>Aucun disque détecté</p>
      </div>
    </div>

    <!-- Navigation onglets -->
    <div class="tab-bar">
      <button v-for="tab in tabs" :key="tab.id" class="tab-btn" :class="{ active: activeTab === tab.id }" @click="activeTab = tab.id">
        <component :is="tab.icon" :size="14" />
        {{ tab.label }}
      </button>
    </div>

    <!-- Contenu onglets -->
    <WinPERepairTab v-if="activeTab === 'repair'" :drives="drives" :sfc-windows-dir="sfcWindowsDir" @result="onResult" />
    <WinPENetworkTab v-else-if="activeTab === 'network'" @result="onResult" />
    <WinPEDiskTab v-else-if="activeTab === 'disk'" @result="onResult" />
    <WinPESystemTab v-else-if="activeTab === 'system'" @result="onResult" />
    <WinPERegistryTab v-else-if="activeTab === 'registry'" @result="onResult" />
    <WinPEToolsPanel v-else-if="activeTab === 'tools'" />

    <!-- Journal des résultats -->
    <div class="results-panel" v-if="results.length > 0">
      <div class="results-header">
        <h2 class="section-title"><ScrollText :size="16" /> Journal des opérations</h2>
        <NButton size="sm" variant="ghost" @click="results = []">Effacer</NButton>
      </div>
      <div class="results-list">
        <div v-for="(r, i) in results" :key="i" class="result-item" :class="r.success ? 'success' : 'error'">
          <div class="result-header">
            <CheckCircle v-if="r.success" :size="13" class="result-icon success" />
            <XCircle v-else :size="13" class="result-icon error" />
            <code class="result-cmd">{{ r.command }}</code>
          </div>
          <pre class="result-output">{{ r.output }}</pre>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineOptions({ name: "WinPEModePage" });
import { ref, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import {
  HardDrive, CheckCircle, XCircle, ScrollText, MonitorCheck,
  Wrench, Wifi, Database as DiskIcon, Monitor, Archive, Download,
} from "lucide-vue-next";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import WinPEToolsPanel from "@/components/winpe/WinPEToolsPanel.vue";
import WinPERepairTab from "@/components/winpe/WinPERepairTab.vue";
import WinPENetworkTab from "@/components/winpe/WinPENetworkTab.vue";
import WinPEDiskTab from "@/components/winpe/WinPEDiskTab.vue";
import WinPESystemTab from "@/components/winpe/WinPESystemTab.vue";
import WinPERegistryTab from "@/components/winpe/WinPERegistryTab.vue";

interface PeDrive { letter: string; label: string; size_gb: number; free_gb: number; fs: string; is_system: boolean; }
interface PeSystemInfo { is_winpe: boolean; pe_version: string; cpu: string; ram_gb: number; drives: PeDrive[]; arch: string; }
interface RepairResult { success: boolean; output: string; command: string; }
interface WindowsInstall { drive: string; windows_dir: string; version: string; build: string; }

const sysInfo = ref<PeSystemInfo | null>(null);
const drives = ref<PeDrive[]>([]);
const winInstalls = ref<WindowsInstall[]>([]);
const selectedInstall = ref<WindowsInstall | null>(null);
const sfcWindowsDir = ref("C:\\Windows");
const results = ref<RepairResult[]>([]);
const activeTab = ref("repair");

const tabs = [
  { id: "repair",   label: "Réparation",  icon: Wrench },
  { id: "network",  label: "Réseau",      icon: Wifi },
  { id: "disk",     label: "Disques",     icon: DiskIcon },
  { id: "system",   label: "Système",     icon: Monitor },
  { id: "registry", label: "Registre",    icon: Archive },
  { id: "tools",    label: "Outils",      icon: Download },
];

function onResult(r: RepairResult) {
  results.value.unshift(r);
  if (results.value.length > 50) results.value = results.value.slice(0, 50);
}

const driveUsedPercent = (d: PeDrive) =>
  d.size_gb > 0 ? Math.round(((d.size_gb - d.free_gb) / d.size_gb) * 100) : 0;

async function loadInfo() {
  try {
    sysInfo.value = await invoke<PeSystemInfo>("get_pe_system_info");
    if (sysInfo.value) drives.value = sysInfo.value.drives;
  } catch { /* silent */ }
  try {
    winInstalls.value = await invoke<WindowsInstall[]>("detect_windows_installs");
    if (winInstalls.value.length >= 1) {
      selectedInstall.value = winInstalls.value[0];
      sfcWindowsDir.value = winInstalls.value[0].windows_dir;
    }
  } catch { /* silent */ }
}

onMounted(loadInfo);
</script>

<style scoped>
.winpe-page { display: flex; flex-direction: column; gap: 20px; max-width: 1400px; }

.winpe-banner {
  display: flex; align-items: center; gap: 16px; padding: 16px 20px;
  background: linear-gradient(135deg, rgba(255,140,0,.12), rgba(255,80,0,.06));
  border: 1px solid rgba(255,140,0,.3); border-radius: 12px;
}
.banner-icon { color: var(--accent-primary); }
.banner-content h1 { font-size: 1.1rem; font-weight: 700; margin: 0 0 4px; }
.banner-content p { font-size: 0.78rem; color: var(--text-muted); margin: 0; }
.loading-text { font-style: italic; }
.pe-badge { margin-left: auto; }

.section-card { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 12px; padding: 16px; }
.section-title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 700; color: var(--text-primary); margin-bottom: 12px; }
.empty-state { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 30px; color: var(--text-muted); font-size: 13px; }

.install-list { display: flex; flex-direction: column; gap: 8px; }
.install-card { display: flex; align-items: center; gap: 12px; padding: 10px 14px; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-primary); cursor: pointer; transition: all .15s; width: 100%; font-family: inherit; }
.install-card:hover { border-color: var(--accent-primary); }
.install-card.active { border-color: var(--accent-primary); background: rgba(255,140,0,.06); }
.install-info { display: flex; flex-direction: column; gap: 2px; flex: 1; text-align: left; }
.install-version { font-size: 13px; font-weight: 600; color: var(--text-primary); }
.install-path { font-size: 11px; color: var(--text-muted); font-family: monospace; }
.install-build { font-size: 11px; color: var(--text-muted); }
.check-icon { color: var(--accent-primary); flex-shrink: 0; }

.drives-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 8px; }
.drive-card { background: var(--bg-primary); border: 1px solid var(--border); border-radius: 8px; padding: 10px 12px; display: flex; flex-direction: column; gap: 6px; }
.drive-letter { font-size: 1.1rem; font-weight: 700; color: var(--accent-primary); }
.drive-info { display: flex; justify-content: space-between; font-size: 0.75rem; }
.drive-label { color: var(--text-secondary); }
.drive-fs { color: var(--text-muted); }
.drive-bar { height: 4px; background: var(--bg-tertiary); border-radius: 2px; overflow: hidden; }
.drive-bar-fill { height: 100%; background: var(--accent-primary); border-radius: 2px; transition: width .3s; }
.drive-bar-fill.danger { background: var(--danger); }
.drive-sizes { font-size: 0.7rem; color: var(--text-muted); }

/* Tabs */
.tab-bar { display: flex; gap: 6px; border-bottom: 1px solid var(--border); padding-bottom: 0; flex-wrap: wrap; }
.tab-btn {
  display: flex; align-items: center; gap: 7px; padding: 9px 16px;
  background: none; border: none; border-bottom: 2px solid transparent;
  cursor: pointer; font-family: inherit; font-size: 13px; font-weight: 600;
  color: var(--text-muted); transition: all .15s; margin-bottom: -1px;
}
.tab-btn:hover { color: var(--text-primary); }
.tab-btn.active { color: var(--accent-primary); border-bottom-color: var(--accent-primary); }

/* Results */
.results-panel { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 12px; padding: 16px; }
.results-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
.results-list { display: flex; flex-direction: column; gap: 8px; max-height: 400px; overflow-y: auto; }
.result-item { border-radius: 8px; padding: 10px 12px; border: 1px solid; }
.result-item.success { border-color: rgba(34,197,94,.3); background: rgba(34,197,94,.04); }
.result-item.error   { border-color: rgba(239,68,68,.3);  background: rgba(239,68,68,.04); }
.result-header { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
.result-icon.success { color: var(--success); }
.result-icon.error   { color: var(--danger); }
.result-cmd { font-size: 11px; font-family: monospace; color: var(--text-secondary); }
.result-output { font-size: 11px; font-family: "JetBrains Mono", monospace; color: var(--text-muted); margin: 0; white-space: pre-wrap; max-height: 160px; overflow-y: auto; }
</style>
