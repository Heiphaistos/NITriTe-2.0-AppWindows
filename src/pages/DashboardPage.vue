<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, type Component } from "vue";
import { invoke, isTauriContext } from "@/utils/invoke";
import { confirm } from "@tauri-apps/plugin-dialog";
import { useNotificationStore } from "@/stores/notifications";
import { useRouter } from "vue-router";
import { useDiagnosticStore } from "@/stores/diagnosticStore";
import StatsCard from "@/components/shared/StatsCard.vue";
import SparklineChart from "@/components/ui/SparklineChart.vue";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NProgress from "@/components/ui/NProgress.vue";
import NBadge from "@/components/ui/NBadge.vue";
import AlertThresholdsModal from "@/components/ui/AlertThresholdsModal.vue";
import type { AlertThresholds } from "@/utils/alertThresholds";
import { normalizeThresholds } from "@/utils/alertThresholds";
import type { SystemMonitorPayload, SysInfo, AppLogEntry, SystemHistory } from "@/types/diagnostic";
import {
  Cpu, MemoryStick, HardDrive, Wifi,
  Stethoscope, Trash2, RefreshCw, Save, Shield,
  Clock, CheckCircle, AlertTriangle, Activity,
  ArrowUp, ArrowDown, X, Zap,
  Monitor, Thermometer, Flame, Pause, Play,
  Settings, XCircle, Lightbulb, ArrowRight,
} from "lucide-vue-next";

const router    = useRouter();
const diagStore = useDiagnosticStore();

// --- Interfaces ---
interface CoreUsage { id?: number; usage: number; }
interface GpuInfo { name: string; usage_percent: number; vram_used_mb: number; vram_total_mb: number; temperature_c: number; }
interface DiskTemp { name: string; temp_c: number; }
interface ProcessInfo { pid: number; name: string; cpu_percent: number; memory_mb: number; }
interface AlertItem { id: number; level: string; message: string; time: string; }

const notify = useNotificationStore();

// --- Seuils alertes ---
const thresholds = ref<AlertThresholds>({
  cpu_warn: 75, cpu_crit: 90,
  ram_warn: 75, ram_crit: 90,
  disk_warn: 85, disk_crit: 95,
});
const showThresholdModal = ref(false);

function loadThresholds() {
  try {
    const s = localStorage.getItem("nitrite_alert_thresholds");
    // normalizeThresholds() : filet de sécurité pour une valeur déjà persistée
    // par une version antérieure au fix (seuil critique < avertissement).
    if (s) thresholds.value = normalizeThresholds({ ...thresholds.value, ...JSON.parse(s) }, thresholds.value);
  } catch { /* ignore */ }
}

// --- Métriques ---
const cpuUsage = ref(0), ramUsage = ref(0), ramUsedGb = ref(0), ramTotalGb = ref(0);
const diskUsage = ref(0), networkDown = ref(0), networkUp = ref(0), healthScore = ref(0);
const loading = ref(true), isLive = ref(false), isSimulation = ref(false);
const cpuName = ref(""), gpuName = ref(""), diskModel = ref("");
const topProcesses = ref<ProcessInfo[]>([]);
const alerts = ref<AlertItem[]>([]);
let alertIdCounter = 0;

// Kill process
const killingPid = ref<number | null>(null);
const killError = ref<string | null>(null);

async function killProcess(pid: number, name: string) {
  if (!(await confirm(`Terminer le processus "${name}" (PID ${pid}) ?`, { title: "Nitrite", kind: "warning" }))) return;
  killingPid.value = pid;
  killError.value = null;
  try {
    await invoke("kill_process", { pid });
    topProcesses.value = topProcesses.value.filter(p => p.pid !== pid);
  } catch (e: unknown) {
    killError.value = `Impossible de tuer ${name}: ${(e instanceof Error ? e.message : String(e)).slice(0, 120)}`;
    setTimeout(() => { killError.value = null; }, 4000);
  } finally { killingPid.value = null; }
}

// --- Monitoring détaillé ---
const cpuCores = ref<CoreUsage[]>([]), gpuData = ref<GpuInfo[]>([]), diskTemps = ref<DiskTemp[]>([]);
const diskReadKbs = ref(0), diskWriteKbs = ref(0), networkDownKbs = ref(0), networkUpKbs = ref(0);
const paused = ref(false), cpuTemp = ref(0);
const cpuHistory = ref<number[]>([]), ramHistory = ref<number[]>([]);
const netHistory = ref<number[]>([]), diskHistory = ref<number[]>([]);
const MAX_HISTORY = 60;
function pushHistory(arr: typeof cpuHistory, val: number) {
  arr.value.push(val);
  if (arr.value.length > MAX_HISTORY) arr.value.shift();
}

import type { UnlistenFn } from "@tauri-apps/api/event";
let unlisten: UnlistenFn | null = null;
let devInterval: ReturnType<typeof setInterval> | null = null;
const recentActivity = ref<{ time: string; message: string; type: "info" | "warning" | "error" }[]>([{ time: "Maintenant", message: "Application démarrée", type: "info" }]);

// --- Infos système détaillées ---
const cpuFreqMhz = ref(0), cpuCoreCount = ref(0), cpuThreadCount = ref(0), cpuBaseGhz = ref(0);
const hostname = ref(""), osName = ref(""), osVersion = ref(""), osArch = ref("");
const uptimeH = ref(0), totalProcesses = ref(0), ramType = ref("");

const quickActions = [
  { label: "Diagnostic", icon: Stethoscope, route: "/diagnostic", grad: "linear-gradient(135deg,#f97316,#fb923c)" },
  { label: "Nettoyage", icon: Trash2, route: "/optimizations", grad: "linear-gradient(135deg,#22c55e,#16a34a)" },
  { label: "Mises à jour", icon: RefreshCw, route: "/updates", grad: "linear-gradient(135deg,#3b82f6,#2563eb)" },
  { label: "Sauvegarde", icon: Save, route: "/backup", grad: "linear-gradient(135deg,#eab308,#ca8a04)" },
  { label: "Scan Antivirus", icon: Shield, route: "/scanvirus", grad: "linear-gradient(135deg,#ef4444,#dc2626)" },
];

function computeHealthScore() {
  let score = 100;
  const t = thresholds.value;
  if (cpuUsage.value >= t.cpu_crit) score -= 20; else if (cpuUsage.value >= t.cpu_warn) score -= 10;
  if (ramUsage.value >= t.ram_crit) score -= 25; else if (ramUsage.value >= t.ram_warn) score -= 15;
  if (diskUsage.value >= t.disk_crit) score -= 25; else if (diskUsage.value >= t.disk_warn) score -= 5;
  healthScore.value = Math.max(0, score);
}
function healthColor() { return healthScore.value >= 80 ? "var(--success)" : healthScore.value >= 50 ? "var(--warning)" : "var(--danger)"; }
function healthLabel() { return healthScore.value >= 80 ? "Excellent" : healthScore.value >= 60 ? "Bon" : healthScore.value >= 40 ? "Moyen" : "Critique"; }

// ─── Suggestions Engine ──────────────────────────────────────────────────────
interface Suggestion { id: string; icon: Component; title: string; description: string; route: string; level: 'info' | 'warning' | 'critical'; }

const suggestions = computed<Suggestion[]>(() => {
  const list: Suggestion[] = [];
  const t = thresholds.value;
  if (ramUsage.value >= t.ram_crit)
    list.push({ id: 'ram-crit', icon: MemoryStick, title: "RAM critique", description: `${ramUsage.value}% utilisée — fermez des applications ou activez le Mode Turbo`, route: "/optimizations", level: 'critical' });
  else if (ramUsage.value >= t.ram_warn)
    list.push({ id: 'ram-warn', icon: MemoryStick, title: "RAM élevée", description: `${ramUsage.value}% utilisée — Mode Turbo recommandé pour libérer de la mémoire`, route: "/optimizations", level: 'warning' });
  if (diskUsage.value >= t.disk_crit)
    list.push({ id: 'disk-crit', icon: HardDrive, title: "Disque presque plein", description: `${diskUsage.value}% occupé — lancez le Nettoyeur ou supprimez de gros fichiers`, route: "/cleaner", level: 'critical' });
  else if (diskUsage.value >= t.disk_warn)
    list.push({ id: 'disk-warn', icon: HardDrive, title: "Disque presque plein", description: `${diskUsage.value}% — pensez à nettoyer les fichiers temporaires`, route: "/cleaner", level: 'warning' });
  if (cpuUsage.value >= t.cpu_crit)
    list.push({ id: 'cpu-crit', icon: Cpu, title: "CPU surchargé", description: `${cpuUsage.value}% — identifiez et terminez les processus gourmands`, route: "/diagnostic", level: 'critical' });
  if (healthScore.value < 50)
    list.push({ id: 'health-low', icon: Stethoscope, title: "Santé système faible", description: `Score ${healthScore.value}/100 — lancez un diagnostic complet`, route: "/diagnostic", level: 'warning' });
  if (cpuTemp.value > 85)
    list.push({ id: 'temp-high', icon: Thermometer, title: "Température CPU critique", description: `${cpuTemp.value}°C — vérifiez le refroidissement et la pâte thermique`, route: "/temperatures", level: 'critical' });
  else if (cpuTemp.value > 75)
    list.push({ id: 'temp-warn', icon: Thermometer, title: "Température CPU élevée", description: `${cpuTemp.value}°C — nettoyez les ventilateurs si possible`, route: "/temperatures", level: 'warning' });
  if (list.length === 0 && healthScore.value >= 80)
    list.push({ id: 'all-ok', icon: CheckCircle, title: "Système en bonne santé", description: "Aucun problème détecté. Pensez à faire une sauvegarde régulière.", route: "/backup", level: 'info' });
  return list;
});

function checkThresholdAlerts(cpu: number, ram: number, disk: number) {
  const t = thresholds.value;
  const now = new Date().toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
  const add = (level: string, msg: string) => {
    if (!alerts.value.find(a => a.message === msg))
      alerts.value.unshift({ id: ++alertIdCounter, level, message: msg, time: now });
  };
  if (cpu >= t.cpu_crit) add("critical", `CPU critique : ${cpu}% (seuil: ${t.cpu_crit}%)`);
  else if (cpu >= t.cpu_warn) add("warning", `CPU élevé : ${cpu}% (seuil: ${t.cpu_warn}%)`);
  if (ram >= t.ram_crit) add("critical", `RAM critique : ${ram}% (seuil: ${t.ram_crit}%)`);
  else if (ram >= t.ram_warn) add("warning", `RAM élevée : ${ram}% (seuil: ${t.ram_warn}%)`);
  if (disk >= t.disk_crit) add("critical", `Disque critique : ${disk}% (seuil: ${t.disk_crit}%)`);
  else if (disk >= t.disk_warn) add("warning", `Disque élevé : ${disk}% (seuil: ${t.disk_warn}%)`);
}

function applyMonitorData(raw: SystemMonitorPayload) {
  cpuUsage.value = Math.round(raw.cpu_percent ?? raw.cpu_usage ?? 0);
  const rU = raw.ram_used_gb ?? 0, rT = raw.ram_total_gb ?? 0;
  ramUsedGb.value = Math.round(rU * 10) / 10; ramTotalGb.value = Math.round(rT * 10) / 10;
  ramUsage.value = Math.round(raw.ram_percent ?? (rT > 0 ? (rU / rT) * 100 : 0));
  diskUsage.value = Math.round(raw.disk_percent ?? 0);
  networkDown.value = Math.round(raw.network_down_kbs ?? raw.net_download_kbs ?? 0);
  networkUp.value = Math.round(raw.network_up_kbs ?? raw.net_upload_kbs ?? 0);
  cpuCores.value = (raw.cpu_per_core ?? raw.cpu_cores ?? []).map((u: number | CoreUsage, i: number) =>
    typeof u === "number" ? { id: i, usage: u } : u);
  if (Array.isArray(raw.gpu_data) && raw.gpu_data.length > 0) gpuData.value = raw.gpu_data;
  if (Array.isArray(raw.disk_temps) && raw.disk_temps.length > 0) diskTemps.value = raw.disk_temps;
  else if (!Array.isArray(raw.disk_temps)) diskTemps.value = [];
  diskReadKbs.value = raw.disk_read_kbs ?? 0; diskWriteKbs.value = raw.disk_write_kbs ?? 0;
  cpuTemp.value = raw.cpu_temp_c ?? 0;
  if ((raw.cpu_freq_mhz ?? 0) > 0) cpuFreqMhz.value = raw.cpu_freq_mhz!;
  if ((raw.process_count ?? 0) > 0) totalProcesses.value = raw.process_count!;
  networkDownKbs.value = raw.network_down_kbs ?? raw.net_download_kbs ?? 0;
  networkUpKbs.value = raw.network_up_kbs ?? raw.net_upload_kbs ?? 0;
  topProcesses.value = (raw.top_processes ?? []).slice(0, 8).map(p => ({
    pid: p.pid, name: p.name,
    cpu_percent: Math.round(p.cpu_percent * 10) / 10,
    memory_mb: Math.round(p.memory_mb ?? 0),
  }));
  if (Array.isArray(raw.alerts)) {
    for (const a of raw.alerts) {
      if (!alerts.value.find(x => x.message === a.message))
        alerts.value.unshift({ id: ++alertIdCounter, level: a.level, message: a.message,
          time: new Date().toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" }) });
    }
    if (alerts.value.length > 10) alerts.value = alerts.value.slice(0, 10);
  }
  checkThresholdAlerts(cpuUsage.value, ramUsage.value, diskUsage.value);
  pushHistory(cpuHistory, cpuUsage.value); pushHistory(ramHistory, ramUsage.value);
  pushHistory(netHistory, Math.min(100, (networkDownKbs.value + networkUpKbs.value) / 100));
  pushHistory(diskHistory, diskUsage.value);
  computeHealthScore();
}

function dismissAlert(id: number) { alerts.value = alerts.value.filter(a => a.id !== id); }
function clearAlerts() { alerts.value = []; }
function formatSpeed(kbs: number) { return kbs >= 1024 ? `${(kbs / 1024).toFixed(1)} MB/s` : `${Math.round(kbs)} KB/s`; }
function coreBarColor(u: number) { return u > 90 ? "var(--danger)" : u > 70 ? "var(--warning)" : "var(--accent-primary)"; }
function tempColor(t: number) { return t > 85 ? "var(--danger)" : t > 70 ? "var(--warning)" : "var(--success)"; }
const networkStatus = computed(() => {
  const tot = networkDown.value + networkUp.value;
  return tot === 0 ? "Inactif" : `↓ ${formatSpeed(networkDown.value)} ↑ ${formatSpeed(networkUp.value)}`;
});

onMounted(async () => {
  loadThresholds();
  try {
    const logs = await invoke<AppLogEntry[]>("get_app_logs");
    if (logs?.length > 0) recentActivity.value = logs.slice(0, 5).map(l => ({
      time: l.timestamp?.substring(11, 16) || "—", message: l.message || "—",
      type: (l.level === "ERROR" ? "error" : l.level === "WARN" ? "warning" : "info") as "error" | "warning" | "info",
    }));
  } catch { /* pas de logs */ }

  try {
    const { listen } = await import("@tauri-apps/api/event");
    isLive.value = true;
    unlisten = await listen<SystemMonitorPayload>("system-monitor", (e) => { applyMonitorData(e.payload); });
    const info = await diagStore.fetchSysInfo() as SysInfo | null;
    cpuUsage.value = Math.round(info?.cpu?.usage_percent ?? 0);
    ramUsedGb.value = Math.round((info?.ram?.used_gb ?? 0) * 10) / 10;
    ramTotalGb.value = Math.round((info?.ram?.total_gb ?? 0) * 10) / 10;
    ramUsage.value = Math.round(info?.ram?.usage_percent ?? 0);
    if ((info?.disks?.length ?? 0) > 0 && (info!.disks[0].partitions?.length ?? 0) > 0)
      diskUsage.value = Math.round(info!.disks[0].partitions[0].usage_percent);
    cpuName.value = info?.cpu?.name ?? "";
    diskModel.value = info?.disks?.[0]?.model ?? info?.disks?.[0]?.name ?? "";
    if ((info?.gpus?.length ?? 0) > 0) gpuName.value = info!.gpus[0].name ?? "";
    cpuCoreCount.value = info?.cpu?.cores ?? 0;
    cpuThreadCount.value = info?.cpu?.threads ?? 0;
    cpuBaseGhz.value = (info?.cpu?.base_speed_mhz ?? 0) / 1000;
    hostname.value = info?.os?.hostname ?? "";
    osName.value = info?.os?.name ?? "";
    osVersion.value = info?.os?.version ?? "";
    osArch.value = info?.os?.architecture ?? "";
    if ((info?.ram?.modules?.length ?? 0) > 0) ramType.value = info!.ram.modules![0]?.memory_type ?? "";
    computeHealthScore();
    try {
      const hist = await invoke<SystemHistory>("get_system_history");
      uptimeH.value = hist?.current_uptime_hours ?? 0;
    } catch { /* uptime: non critique */ }
  } catch (e: unknown) {
    isSimulation.value = true;
    if (isTauriContext()) {
      notify.error("Dashboard", (e instanceof Error ? e.message : String(e)).slice(0, 120));
    }
    cpuUsage.value = 23; ramUsage.value = 45; ramUsedGb.value = 14.4;
    ramTotalGb.value = 32; diskUsage.value = 49; networkDown.value = 120;
    topProcesses.value = [
      { pid: 1234, name: "chrome.exe", cpu_percent: 12.5, memory_mb: 480 },
      { pid: 5678, name: "code.exe", cpu_percent: 8.3, memory_mb: 350 },
      { pid: 2345, name: "explorer.exe", cpu_percent: 2.1, memory_mb: 120 },
    ];
    // Mode simulation: valeurs statiques, pas de données aléatoires
    devInterval = setInterval(() => {
      pushHistory(cpuHistory, cpuUsage.value); pushHistory(ramHistory, ramUsage.value);
      pushHistory(netHistory, Math.min(100, networkDown.value / 20)); pushHistory(diskHistory, diskUsage.value);
      checkThresholdAlerts(cpuUsage.value, ramUsage.value, diskUsage.value);
      computeHealthScore();
    }, 2000);
    computeHealthScore();
  }
  loading.value = false;
});

onUnmounted(() => {
  if (unlisten) unlisten();
  if (devInterval) clearInterval(devInterval);
});
</script>

<template>
  <div class="dashboard">
    <!-- Modal seuils -->
    <AlertThresholdsModal :open="showThresholdModal" v-model="thresholds" @close="showThresholdModal = false" />

    <!-- Welcome -->
    <div class="welcome">
      <div>
        <h1 class="welcome-title">Tableau de bord</h1>
        <p class="welcome-date">{{ new Date().toLocaleDateString("fr-FR", { weekday: "long", year: "numeric", month: "long", day: "numeric" }) }}</p>
      </div>
      <div style="display:flex;align-items:center;gap:8px">
        <NButton variant="ghost" size="sm" @click="showThresholdModal = true" title="Configurer les seuils d'alerte">
          <Settings :size="14" /> Seuils
        </NButton>
        <NBadge :variant="isLive ? 'success' : isSimulation ? 'warning' : 'neutral'" class="live-badge">
          <span v-if="isLive" class="live-dot"></span>
          {{ isLive ? "Monitoring actif" : isSimulation ? "⚠ Simulation" : "Mode statique" }}
        </NBadge>
      </div>
    </div>

    <!-- Kill error -->
    <div v-if="killError" class="kill-error-bar"><XCircle :size="14" /> {{ killError }}</div>

    <!-- Alertes actives -->
    <div v-if="alerts.length > 0" class="alerts-bar">
      <div v-for="alert in alerts" :key="alert.id" class="alert-item" :class="`alert-${alert.level}`">
        <AlertTriangle :size="14" /><span>{{ alert.message }}</span>
        <span class="alert-time">{{ alert.time }}</span>
        <button class="alert-dismiss" @click="dismissAlert(alert.id)"><X :size="12" /></button>
      </div>
      <button v-if="alerts.length > 1" class="clear-alerts" @click="clearAlerts">Effacer tout</button>
    </div>

    <!-- Stats Cards -->
    <div class="stats-grid stagger-children">
      <StatsCard title="CPU" :subtitle="cpuName || 'Processeur'" :value="`${cpuUsage}%`" :icon="Cpu" :progress="cpuUsage" color="accent" />
      <StatsCard title="RAM" :subtitle="`${ramUsedGb} / ${ramTotalGb} GB`" :value="`${ramUsage}%`" :icon="MemoryStick" :progress="ramUsage" color="info" />
      <StatsCard title="Disque" :subtitle="diskModel || 'Partition C:'" :value="`${diskUsage}%`" :icon="HardDrive" :progress="diskUsage" color="warning" />
      <StatsCard title="Réseau" :subtitle="networkStatus" :value="formatSpeed(networkDown)" :icon="Wifi" color="success" />
    </div>

    <!-- Infos Système -->
    <div v-if="hostname || osName" class="sysinfo-bar">
      <div class="sysinfo-item" v-if="osName">
        <Monitor :size="12" class="sysinfo-icon" />
        <span class="sysinfo-label">OS</span>
        <span class="sysinfo-val">{{ osName }} {{ osVersion }}</span>
      </div>
      <div class="sysinfo-item" v-if="hostname">
        <span class="sysinfo-label">Machine</span>
        <span class="sysinfo-val font-mono">{{ hostname }}</span>
      </div>
      <div class="sysinfo-item" v-if="osArch">
        <span class="sysinfo-label">Arch</span>
        <span class="sysinfo-val">{{ osArch }}</span>
      </div>
      <div class="sysinfo-item" v-if="cpuCoreCount > 0">
        <Cpu :size="12" class="sysinfo-icon" />
        <span class="sysinfo-label">CPU</span>
        <span class="sysinfo-val">{{ cpuCoreCount }}C / {{ cpuThreadCount }}T
          <template v-if="cpuFreqMhz > 0"> @ {{ (cpuFreqMhz / 1000).toFixed(1) }} GHz</template>
          <template v-else-if="cpuBaseGhz > 0"> @ {{ cpuBaseGhz.toFixed(1) }} GHz</template>
        </span>
      </div>
      <div class="sysinfo-item" v-if="ramType">
        <MemoryStick :size="12" class="sysinfo-icon" />
        <span class="sysinfo-label">RAM</span>
        <span class="sysinfo-val">{{ ramTotalGb }} GB {{ ramType }}</span>
      </div>
      <div class="sysinfo-item" v-if="uptimeH > 0">
        <Clock :size="12" class="sysinfo-icon" />
        <span class="sysinfo-label">Uptime</span>
        <span class="sysinfo-val">{{ uptimeH >= 24 ? `${Math.floor(uptimeH / 24)}j ${Math.floor(uptimeH % 24)}h` : `${uptimeH.toFixed(0)}h` }}</span>
      </div>
      <div class="sysinfo-item" v-if="totalProcesses > 0">
        <Activity :size="12" class="sysinfo-icon" />
        <span class="sysinfo-label">Processus</span>
        <span class="sysinfo-val">{{ totalProcesses }}</span>
      </div>
    </div>

    <!-- Sparklines -->
    <div v-if="cpuHistory.length >= 3" class="sparklines-row">
      <div class="sparkline-card">
        <span class="sparkline-label">CPU</span>
        <SparklineChart :data="cpuHistory" color="var(--accent-primary)" :height="36" :fill="true" label="cpu" />
        <span class="sparkline-val">{{ cpuUsage }}%</span>
      </div>
      <div class="sparkline-card">
        <span class="sparkline-label">RAM</span>
        <SparklineChart :data="ramHistory" color="var(--info, #60a5fa)" :height="36" :fill="true" label="ram" />
        <span class="sparkline-val">{{ ramUsage }}%</span>
      </div>
      <div class="sparkline-card">
        <span class="sparkline-label">Disque</span>
        <SparklineChart :data="diskHistory" color="var(--warning)" :height="36" :fill="true" label="disk" />
        <span class="sparkline-val">{{ diskUsage }}%</span>
      </div>
      <div class="sparkline-card">
        <span class="sparkline-label">Réseau</span>
        <SparklineChart :data="netHistory" color="var(--success)" :height="36" :fill="true" label="net" />
        <span class="sparkline-val">{{ formatSpeed(networkDown) }}</span>
      </div>
    </div>

    <!-- Health + Quick Actions -->
    <div class="dashboard-grid">
      <NCard>
        <template #header><div class="card-header-row"><Shield :size="16" /><span>Santé Système</span></div></template>
        <div class="health-section">
          <div class="health-score" :style="{ color: healthColor() }">{{ healthScore }}<span class="health-max">/100</span></div>
          <div class="health-label" :style="{ color: healthColor() }">{{ healthLabel() }}</div>
          <NProgress :value="healthScore" size="lg" />
          <div class="health-details">
            <div class="health-item">
              <CheckCircle v-if="cpuUsage < thresholds.cpu_warn" :size="14" style="color:var(--success)" />
              <AlertTriangle v-else :size="14" style="color:var(--warning)" />
              <span>CPU: {{ cpuUsage }}% {{ cpuUsage < thresholds.cpu_warn ? "✓" : cpuUsage >= thresholds.cpu_crit ? "— Critique" : "— Élevé" }}</span>
            </div>
            <div class="health-item">
              <CheckCircle v-if="ramUsage < thresholds.ram_warn" :size="14" style="color:var(--success)" />
              <AlertTriangle v-else :size="14" style="color:var(--danger)" />
              <span>RAM: {{ ramUsage }}% {{ ramUsage < thresholds.ram_warn ? "✓" : ramUsage >= thresholds.ram_crit ? "— Critique" : "— Élevée" }}</span>
            </div>
            <div class="health-item">
              <CheckCircle v-if="diskUsage < thresholds.disk_warn" :size="14" style="color:var(--success)" />
              <AlertTriangle v-else :size="14" style="color:var(--danger)" />
              <span>Disque: {{ diskUsage }}% {{ diskUsage < thresholds.disk_warn ? "✓" : diskUsage >= thresholds.disk_crit ? "— Critique" : "— Élevé" }}</span>
            </div>
            <div class="health-item"><ArrowDown :size="14" style="color:var(--success)" /><span>↓ {{ formatSpeed(networkDown) }} ↑ {{ formatSpeed(networkUp) }}</span></div>
            <div v-if="gpuName" class="health-item">
              <CheckCircle :size="14" style="color:var(--success)" />
              <span class="muted" style="font-size:11px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:200px">{{ gpuName }}</span>
            </div>
          </div>
        </div>
      </NCard>

      <NCard>
        <template #header><div class="card-header-row"><Zap :size="16" /><span>Actions Rapides</span></div></template>
        <div class="quick-actions">
          <button v-for="a in quickActions" :key="a.label" class="action-btn" @click="router.push(a.route)">
            <div class="action-icon" :style="{ background: a.grad }"><component :is="a.icon" :size="16" style="color:white" /></div>
            <span>{{ a.label }}</span>
          </button>
        </div>
      </NCard>
    </div>

    <!-- Suggestions Engine -->
    <div v-if="suggestions.length > 0" class="suggestions-bar">
      <div class="suggestions-title"><Lightbulb :size="13" /> Recommandations</div>
      <div v-for="s in suggestions" :key="s.id" class="suggestion-item" :class="`suggestion-${s.level}`">
        <component :is="s.icon" :size="14" class="suggestion-icon" />
        <div class="suggestion-body">
          <span class="suggestion-title">{{ s.title }}</span>
          <span class="suggestion-desc">{{ s.description }}</span>
        </div>
        <RouterLink :to="s.route" class="suggestion-link"><ArrowRight :size="12" /></RouterLink>
      </div>
    </div>

    <!-- Top Processus + Kill -->
    <NCard v-if="topProcesses.length > 0">
      <template #header>
        <div class="card-header-row"><Activity :size="16" /><span>Top Processus</span><NBadge variant="neutral">{{ topProcesses.length }}</NBadge></div>
      </template>
      <div class="processes-table">
        <div class="process-header"><span>Processus</span><span>PID</span><span>CPU</span><span>RAM</span><span></span></div>
        <div v-for="proc in topProcesses" :key="proc.pid" class="process-row">
          <span class="proc-name">{{ proc.name }}</span>
          <span class="proc-pid font-mono">{{ proc.pid }}</span>
          <div class="proc-cpu">
            <span class="proc-val">{{ proc.cpu_percent.toFixed(1) }}%</span>
            <div class="proc-bar"><div class="proc-bar-fill proc-bar-cpu" :style="{ width: `${Math.min(100, proc.cpu_percent * 2)}%` }"></div></div>
          </div>
          <div class="proc-ram">
            <span class="proc-val">{{ proc.memory_mb >= 1024 ? `${(proc.memory_mb/1024).toFixed(1)} GB` : `${proc.memory_mb} MB` }}</span>
          </div>
          <button class="kill-btn" :class="{ 'kill-loading': killingPid === proc.pid }"
            :disabled="killingPid !== null" @click="killProcess(proc.pid, proc.name)" title="Terminer le processus">
            <XCircle :size="13" />
          </button>
        </div>
      </div>
    </NCard>

    <!-- Monitoring Temps Réel Header -->
    <div class="monitoring-header" v-if="isLive">
      <h2 class="section-title-mon"><Activity :size="16" /> Monitoring Temps Réel</h2>
      <div style="display:flex;align-items:center;gap:8px">
        <NBadge :variant="paused ? 'warning' : 'success'">{{ paused ? 'Pause' : 'Live' }}</NBadge>
        <NButton variant="ghost" size="sm" @click="paused = !paused">
          <Pause v-if="!paused" :size="13" /><Play v-else :size="13" />
        </NButton>
      </div>
    </div>

    <!-- GPU Cards -->
    <div class="gpu-cards" v-if="gpuData.length && isLive && !paused">
      <NCard v-for="(gpu, i) in gpuData" :key="i">
        <template #header><div class="card-header-row"><Monitor :size="14" /><span>{{ gpu.name }}</span></div></template>
        <div class="gpu-stat-row">
          <span class="gpu-stat-label">Utilisation</span>
          <div class="mon-bar-track" style="flex:1"><div class="mon-bar-fill" :style="{ width: `${gpu.usage_percent}%`, background: coreBarColor(gpu.usage_percent) }" /></div>
          <span class="mon-val">{{ Math.round(gpu.usage_percent) }}%</span>
        </div>
        <div class="gpu-stat-row" v-if="gpu.vram_total_mb > 0">
          <span class="gpu-stat-label">VRAM</span>
          <div class="mon-bar-track" style="flex:1"><div class="mon-bar-fill" :style="{ width: `${(gpu.vram_used_mb/gpu.vram_total_mb)*100}%`, background: 'var(--info)' }" /></div>
          <span class="mon-val">{{ gpu.vram_used_mb }} / {{ gpu.vram_total_mb }} MB</span>
        </div>
        <div class="gpu-stat-row" v-if="gpu.temperature_c > 0">
          <span class="gpu-stat-label">Temp.</span>
          <span class="mon-val" :style="{ color: tempColor(gpu.temperature_c) }">{{ gpu.temperature_c }}°C</span>
        </div>
      </NCard>
    </div>

    <!-- Températures -->
    <NCard v-if="isLive && !paused && (cpuTemp > 0 || gpuData.some(g => g.temperature_c > 0) || diskTemps.length > 0)">
      <template #header>
        <div class="card-header-row"><Thermometer :size="16" /><span>Températures</span>
          <Flame v-if="cpuTemp > 80 || gpuData.some(g => g.temperature_c > 85) || diskTemps.some(d => d.temp_c > 55)"
            :size="14" style="color:var(--danger);margin-left:4px" />
        </div>
      </template>
      <div class="temps-grid">
        <div v-if="cpuTemp > 0" class="temp-item">
          <Cpu :size="14" /><span class="temp-label">CPU</span>
          <div class="temp-bar-track" style="flex:1"><div class="temp-bar-fill" :style="{ width: `${Math.min((cpuTemp/100)*100,100)}%`, background: tempColor(cpuTemp) }" /></div>
          <span class="temp-value" :style="{ color: tempColor(cpuTemp) }">{{ cpuTemp }}°C</span>
        </div>
        <div v-for="(gpu, i) in gpuData.filter(g => g.temperature_c > 0)" :key="`gt-${i}`" class="temp-item">
          <Monitor :size="14" /><span class="temp-label">{{ gpu.name.split(' ').slice(0,2).join(' ') }}</span>
          <div class="temp-bar-track" style="flex:1"><div class="temp-bar-fill" :style="{ width: `${Math.min((gpu.temperature_c/100)*100,100)}%`, background: tempColor(gpu.temperature_c) }" /></div>
          <span class="temp-value" :style="{ color: tempColor(gpu.temperature_c) }">{{ gpu.temperature_c }}°C</span>
        </div>
        <div v-for="(disk, i) in diskTemps" :key="`dt-${i}`" class="temp-item">
          <HardDrive :size="14" /><span class="temp-label">{{ disk.name.length > 16 ? disk.name.slice(0,16)+'…' : disk.name }}</span>
          <div class="temp-bar-track" style="flex:1"><div class="temp-bar-fill" :style="{ width: `${Math.min((disk.temp_c/80)*100,100)}%`, background: tempColor(disk.temp_c-10) }" /></div>
          <span class="temp-value" :style="{ color: tempColor(disk.temp_c-10) }">{{ disk.temp_c }}°C</span>
        </div>
      </div>
    </NCard>

    <!-- CPU Cores + Réseau/Disque -->
    <div class="monitor-grid" v-if="isLive && !paused && cpuCores.length > 0">
      <NCard>
        <template #header><div class="card-header-row"><Cpu :size="16" /><span>Usage par Cœur</span></div></template>
        <div class="cores-grid">
          <div v-for="core in cpuCores" :key="core.id" class="core-item">
            <span class="core-label">C{{ core.id }}</span>
            <div class="mon-bar-track"><div class="mon-bar-fill" :style="{ width: `${core.usage}%`, background: coreBarColor(core.usage) }" /></div>
            <span class="mon-val">{{ core.usage }}%</span>
          </div>
        </div>
      </NCard>
      <NCard>
        <template #header><div class="card-header-row"><Wifi :size="16" /><span>Débits Réseau &amp; Disque</span></div></template>
        <div class="net-speeds">
          <div class="net-item"><div class="net-icon net-download"><ArrowDown :size="15" /></div><div><div class="net-lbl">Download</div><div class="net-val">{{ formatSpeed(networkDownKbs) }}</div></div></div>
          <div class="net-item"><div class="net-icon net-upload"><ArrowUp :size="15" /></div><div><div class="net-lbl">Upload</div><div class="net-val">{{ formatSpeed(networkUpKbs) }}</div></div></div>
          <div class="net-sep" />
          <div class="net-item"><div class="net-icon net-read"><HardDrive :size="15" /></div><div><div class="net-lbl">Lecture Disque</div><div class="net-val">{{ formatSpeed(diskReadKbs) }}</div></div></div>
          <div class="net-item"><div class="net-icon net-write"><HardDrive :size="15" /></div><div><div class="net-lbl">Écriture Disque</div><div class="net-val">{{ formatSpeed(diskWriteKbs) }}</div></div></div>
        </div>
      </NCard>
    </div>

    <!-- Recent Activity -->
    <NCard>
      <template #header><div class="card-header-row"><Clock :size="16" /><span>Activité Récente</span></div></template>
      <div class="activity-list">
        <div v-for="(item, i) in recentActivity" :key="i" class="activity-item">
          <div class="activity-dot" :class="`dot-${item.type}`"></div>
          <span class="activity-time">{{ item.time }}</span>
          <span class="activity-msg">{{ item.message }}</span>
        </div>
        <div v-if="recentActivity.length === 1" class="activity-empty">Aucune activité récente. Lancez un diagnostic ou une optimisation.</div>
      </div>
    </NCard>
  </div>
</template>

<style scoped>
.dashboard { display: flex; flex-direction: column; gap: 20px; }
.welcome { display: flex; justify-content: space-between; align-items: flex-start; }
.welcome-title {
  font-size: 26px; font-weight: 800;
  background: linear-gradient(135deg, var(--text-primary) 40%, var(--accent-primary));
  -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; line-height: 1.2;
}
.welcome-date { color: var(--text-secondary); font-size: 13px; margin-top: 3px; }
.live-badge { display: flex; align-items: center; gap: 6px; }
.live-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--success); animation: pulse 2s ease-in-out infinite; }
@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }
.kill-error-bar { display: flex; align-items: center; gap: 8px; padding: 8px 12px; background: var(--danger-muted); color: var(--danger); border: 1px solid var(--danger); border-radius: var(--radius-md); font-size: 12px; font-weight: 500; }
.alerts-bar { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
.alert-item { display: flex; align-items: center; gap: 6px; padding: 6px 10px; border-radius: var(--radius-md); font-size: 12px; font-weight: 500; }
.alert-warning { background: rgba(var(--warning-rgb, 245, 158, 11), 0.15); color: var(--warning); border: 1px solid var(--warning); }
.alert-critical { background: rgba(var(--danger-rgb, 239, 68, 68), 0.15); color: var(--danger); border: 1px solid var(--danger); }
.alert-info { background: var(--accent-muted); color: var(--accent-primary); border: 1px solid var(--accent-primary); }
.alert-time { color: inherit; opacity: 0.6; font-size: 11px; }
.alert-dismiss { background: none; border: none; cursor: pointer; color: inherit; opacity: 0.6; padding: 0; display: flex; }
.alert-dismiss:hover { opacity: 1; }
.clear-alerts { font-size: 11px; color: var(--text-muted); background: none; border: none; cursor: pointer; padding: 4px 8px; border-radius: var(--radius-sm); }
.clear-alerts:hover { color: var(--text-secondary); background: var(--bg-tertiary); }
.stats-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; }
@media (max-width: 1200px) { .stats-grid { grid-template-columns: repeat(2, 1fr); } }
.sparklines-row { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; }
@media (max-width: 1200px) { .sparklines-row { grid-template-columns: repeat(2, 1fr); } }
.sparkline-card { display: flex; align-items: center; gap: 10px; padding: 10px 14px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius-lg); }
.sparkline-label { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: .06em; color: var(--text-muted); width: 36px; flex-shrink: 0; }
.sparkline-val { font-family: "JetBrains Mono", monospace; font-size: 12px; font-weight: 700; color: var(--text-primary); width: 52px; text-align: right; flex-shrink: 0; }
.dashboard-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
@media (max-width: 1000px) { .dashboard-grid { grid-template-columns: 1fr; } }
.card-header-row { display: flex; align-items: center; gap: 8px; }
.health-section { text-align: center; display: flex; flex-direction: column; align-items: center; gap: 8px; }
.health-score { font-size: 48px; font-weight: 800; font-family: "JetBrains Mono", monospace; line-height: 1; }
.health-max { font-size: 20px; opacity: 0.4; }
.health-label { font-size: 14px; font-weight: 600; }
.health-details { display: flex; flex-direction: column; gap: 6px; margin-top: 8px; width: 100%; }
.health-item { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--text-secondary); }
.quick-actions { display: flex; flex-direction: column; gap: 6px; }
.action-btn { display: flex; align-items: center; gap: 12px; padding: 10px 12px; border: 1px solid transparent; border-radius: var(--radius-md); background: transparent; cursor: pointer; font-family: inherit; font-size: 13px; color: var(--text-primary); transition: all var(--transition-fast); text-align: left; width: 100%; font-weight: 500; }
.action-btn:hover { background: var(--bg-tertiary); border-color: var(--border); transform: translateX(3px); }
.action-icon { width: 34px; height: 34px; border-radius: var(--radius-md); display: flex; align-items: center; justify-content: center; flex-shrink: 0; box-shadow: 0 2px 8px rgba(0,0,0,.3); }
.processes-table { display: flex; flex-direction: column; gap: 4px; }
.process-header { display: grid; grid-template-columns: 1fr 70px 140px 100px 32px; gap: 8px; padding: 6px 8px; font-size: 11px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; border-bottom: 1px solid var(--border); }
.process-row { display: grid; grid-template-columns: 1fr 70px 140px 100px 32px; gap: 8px; padding: 6px 8px; border-radius: var(--radius-sm); font-size: 12px; align-items: center; transition: background var(--transition-fast); }
.process-row:hover { background: var(--bg-tertiary); }
.proc-name { font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.proc-pid { color: var(--text-muted); font-size: 11px; }
.proc-cpu, .proc-ram { display: flex; align-items: center; gap: 6px; }
.proc-val { font-family: "JetBrains Mono", monospace; font-size: 11px; min-width: 42px; }
.proc-bar { flex: 1; height: 5px; background: var(--bg-elevated); border: 1px solid var(--border); border-radius: 99px; overflow: hidden; }
.proc-bar-fill { height: 100%; border-radius: 99px; transition: width 0.5s ease; }
.proc-bar-cpu { background: linear-gradient(90deg, var(--accent-primary), var(--accent-hover)); }
.kill-btn { display: flex; align-items: center; justify-content: center; width: 26px; height: 26px; border-radius: var(--radius-sm); background: transparent; border: 1px solid transparent; cursor: pointer; color: var(--text-muted); transition: all var(--transition-fast); padding: 0; }
.kill-btn:hover:not(:disabled) { background: var(--danger-muted); color: var(--danger); border-color: var(--danger); }
.kill-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.kill-btn.kill-loading { animation: pulse 0.8s ease-in-out infinite; }
.activity-list { display: flex; flex-direction: column; gap: 8px; }
.activity-item { display: flex; align-items: center; gap: 10px; font-size: 13px; }
.activity-dot { width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; }
.dot-info { background: var(--info); } .dot-success { background: var(--success); } .dot-warning { background: var(--warning); } .dot-error { background: var(--danger); }
.activity-time { color: var(--text-muted); min-width: 50px; font-family: "JetBrains Mono", monospace; font-size: 11px; }
.activity-msg { color: var(--text-secondary); }
.activity-empty { color: var(--text-muted); font-size: 13px; text-align: center; padding: 12px; }
.font-mono { font-family: "JetBrains Mono", monospace; }
.monitoring-header { display: flex; justify-content: space-between; align-items: center; }
.section-title-mon { display: flex; align-items: center; gap: 8px; font-size: 15px; font-weight: 700; color: var(--text-primary); }
.gpu-cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 16px; }
.gpu-stat-row { display: flex; align-items: center; gap: 10px; margin-top: 8px; }
.gpu-stat-label { font-size: 12px; color: var(--text-secondary); min-width: 75px; }
.monitor-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
@media (max-width: 900px) { .monitor-grid { grid-template-columns: 1fr; } }
.temps-grid { display: flex; flex-direction: column; gap: 10px; }
.temp-item { display: flex; align-items: center; gap: 8px; }
.temp-label { font-size: 12px; color: var(--text-secondary); min-width: 80px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.temp-bar-track { height: 6px; border-radius: 99px; background: var(--bg-elevated); border: 1px solid var(--border); overflow: hidden; }
.temp-bar-fill { height: 100%; border-radius: 99px; transition: width 400ms ease, background 400ms ease; }
.temp-value { font-size: 12px; font-weight: 600; font-family: "JetBrains Mono", monospace; min-width: 44px; text-align: right; }
.cores-grid { display: flex; flex-direction: column; gap: 6px; }
.core-item { display: flex; align-items: center; gap: 8px; }
.core-label { font-size: 11px; color: var(--text-secondary); min-width: 22px; font-family: "JetBrains Mono", monospace; }
.mon-bar-track { flex: 1; height: 6px; border-radius: 99px; background: var(--bg-elevated); border: 1px solid var(--border); overflow: hidden; }
.mon-bar-fill { height: 100%; border-radius: 99px; background: linear-gradient(90deg, var(--accent-primary), var(--accent-hover)); transition: width 300ms ease; }
.mon-val { font-size: 11px; font-weight: 500; color: var(--text-secondary); font-family: "JetBrains Mono", monospace; min-width: 36px; text-align: right; }
.net-speeds { display: flex; flex-direction: column; gap: 10px; }
.net-item { display: flex; align-items: center; gap: 10px; }
.net-icon { width: 32px; height: 32px; border-radius: var(--radius-md); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.net-download { background: linear-gradient(135deg, rgba(34,197,94,.25), rgba(34,197,94,.08)); color: var(--success); }
.net-upload { background: linear-gradient(135deg, rgba(59,130,246,.25), rgba(59,130,246,.08)); color: var(--info); }
.net-read { background: var(--info-muted); color: var(--info); }
.net-write { background: var(--warning-muted); color: var(--warning); }
.net-lbl { font-size: 11px; color: var(--text-secondary); }
.net-val { font-size: 15px; font-weight: 600; color: var(--text-primary); font-family: "JetBrains Mono", monospace; }
.net-sep { height: 1px; background: var(--border); margin: 2px 0; }
.sysinfo-bar { display: flex; flex-wrap: wrap; gap: 6px; padding: 10px 14px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius-lg); }

/* Suggestions engine */
.suggestions-bar { display: flex; flex-direction: column; gap: 6px; }
.suggestions-title { display: flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: .06em; color: var(--text-muted); margin-bottom: 2px; }
.suggestion-item { display: flex; align-items: center; gap: 10px; padding: 10px 14px; border-radius: var(--radius-lg); border: 1px solid var(--border); background: var(--bg-secondary); transition: border-color .15s; }
.suggestion-item:hover { border-color: var(--border-hover); }
.suggestion-info .suggestion-icon { color: var(--accent-primary); }
.suggestion-warning { border-color: rgba(245,158,11,.3); background: rgba(245,158,11,.05); }
.suggestion-warning .suggestion-icon { color: var(--warning, #f59e0b); }
.suggestion-critical { border-color: rgba(239,68,68,.35); background: rgba(239,68,68,.05); }
.suggestion-critical .suggestion-icon { color: var(--danger); }
.suggestion-body { display: flex; flex-direction: column; gap: 2px; flex: 1; min-width: 0; }
.suggestion-title { font-size: 12px; font-weight: 700; color: var(--text-primary); }
.suggestion-desc { font-size: 11px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.suggestion-link { display: flex; align-items: center; justify-content: center; width: 24px; height: 24px; border-radius: 6px; background: var(--bg-tertiary); color: var(--text-muted); flex-shrink: 0; text-decoration: none; transition: all .15s; }
.suggestion-link:hover { background: var(--accent-primary); color: #fff; }
.sysinfo-item { display: flex; align-items: center; gap: 5px; padding: 3px 10px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-md); }
.sysinfo-icon { color: var(--accent-primary); flex-shrink: 0; }
.sysinfo-label { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: .05em; color: var(--text-muted); }
.sysinfo-val { font-size: 11px; color: var(--text-secondary); font-family: "JetBrains Mono", monospace; }
</style>
