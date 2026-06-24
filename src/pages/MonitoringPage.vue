<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { invoke } from "@/utils/invoke";
import StatsCard from "@/components/shared/StatsCard.vue";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NProgress from "@/components/ui/NProgress.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NBadge from "@/components/ui/NBadge.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import { useNotificationStore } from "@/stores/notifications";
import {
  Cpu, MemoryStick, HardDrive, Wifi,
  Activity, AlertTriangle, RefreshCw,
  ArrowUp, ArrowDown, Pause, Play, Monitor,
  Thermometer, Flame, Dumbbell,
  Download, PenLine, Circle, Square, Tag,
} from "lucide-vue-next";

const notify = useNotificationStore();

interface CoreUsage { id: number; usage: number; }
interface ProcessInfo { name: string; pid: number; cpu_percent: number; ram_percent: number; }
interface GpuInfo { name: string; usage_percent: number; vram_used_mb: number; vram_total_mb: number; temperature_c: number; }
interface DiskTemp { name: string; temp_c: number; }
interface MonitorData {
  cpu_usage: number; cpu_name?: string; cpu_temp_c?: number; cpu_cores: CoreUsage[];
  ram_used_gb: number; ram_total_gb: number; ram_percent: number;
  disk_percent?: number; disk_read_kbs: number; disk_write_kbs: number; disk_temps?: DiskTemp[];
  net_upload_kbs: number; net_download_kbs: number;
  gpu_data: GpuInfo[]; top_processes: ProcessInfo[];
}
interface SystemAlert { level: "warning" | "danger" | "info"; message: string; time: string; }

// --- Annotation ---
interface Annotation { id: number; time: string; text: string; color: string; }
const annotations = ref<Annotation[]>([]);
let annotationCounter = 0;
const annotationInput = ref("");
const annotationColor = ref("#f97316");
const showAnnotationForm = ref(false);
function addAnnotation() {
  if (!annotationInput.value.trim()) return;
  const now = new Date().toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  annotations.value.unshift({ id: ++annotationCounter, time: now, text: annotationInput.value.trim(), color: annotationColor.value });
  annotationInput.value = "";
  showAnnotationForm.value = false;
}
function removeAnnotation(id: number) { annotations.value = annotations.value.filter(a => a.id !== id); }

// --- Session Recording ---
interface SessionFrame {
  ts: string;
  cpu: number; ram: number; gpu: number;
  net_down: number; net_up: number;
  disk_read: number; disk_write: number;
  annotation?: string;
}
const recording = ref(false);
const sessionFrames = ref<SessionFrame[]>([]);
function clearSession() { sessionFrames.value = []; }
const sessionStart = ref<Date | null>(null);
const sessionDuration = computed(() => {
  if (!sessionStart.value) return "00:00";
  const s = Math.floor((Date.now() - sessionStart.value.getTime()) / 1000);
  return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
});
let durationTick: ReturnType<typeof setInterval> | null = null;
const durationDisplay = ref("00:00");

function startRecording() {
  // Nettoyer un éventuel interval précédent avant d'en créer un nouveau
  if (durationTick !== null) { clearInterval(durationTick); durationTick = null; }
  sessionFrames.value = [];
  sessionStart.value = new Date();
  recording.value = true;
  durationDisplay.value = "00:00";
  durationTick = setInterval(() => {
    if (!sessionStart.value) return;
    const s = Math.floor((Date.now() - sessionStart.value.getTime()) / 1000);
    durationDisplay.value = `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
  }, 1000);
}
function stopRecording() {
  recording.value = false;
  if (durationTick) { clearInterval(durationTick); durationTick = null; }
}
function recordFrame(d: MonitorData) {
  if (!recording.value) return;
  sessionFrames.value.push({
    ts: new Date().toISOString(),
    cpu: Math.round(d.cpu_usage),
    ram: Math.round(d.ram_percent),
    gpu: d.gpu_data.length ? Math.round(d.gpu_data[0].usage_percent) : 0,
    net_down: d.net_download_kbs,
    net_up: d.net_upload_kbs,
    disk_read: d.disk_read_kbs,
    disk_write: d.disk_write_kbs,
  });
}

// --- CSV Export ---
function exportCSV() {
  const src = sessionFrames.value.length > 0 ? sessionFrames.value : buildCurrentFrame();
  if (src.length === 0) { notify.warning("Aucune donnée", "Lancez un enregistrement de session."); return; }
  const header = "Timestamp,CPU%,RAM%,GPU%,Download KB/s,Upload KB/s,Disk Read KB/s,Disk Write KB/s";
  const rows = src.map(f =>
    `${f.ts},${f.cpu},${f.ram},${f.gpu},${f.net_down},${f.net_up},${f.disk_read},${f.disk_write}`
  );
  const csv = [header, ...rows].join("\n");
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `nitrite_monitoring_${new Date().toISOString().slice(0,19).replace(/:/g, "-")}.csv`;
  a.click();
  URL.revokeObjectURL(url);
}
function buildCurrentFrame(): SessionFrame[] {
  if (!data.value.cpu_usage) return [];
  return [{
    ts: new Date().toISOString(),
    cpu: Math.round(data.value.cpu_usage),
    ram: Math.round(data.value.ram_percent),
    gpu: data.value.gpu_data.length ? Math.round(data.value.gpu_data[0].usage_percent) : 0,
    net_down: data.value.net_download_kbs,
    net_up: data.value.net_upload_kbs,
    disk_read: data.value.disk_read_kbs,
    disk_write: data.value.disk_write_kbs,
  }];
}

// --- Core state ---
const loading = ref(true);
const paused = ref(false);
const data = ref<MonitorData>({
  cpu_usage: 0, cpu_cores: [], ram_used_gb: 0, ram_total_gb: 0, ram_percent: 0,
  disk_read_kbs: 0, disk_write_kbs: 0, net_upload_kbs: 0, net_download_kbs: 0,
  gpu_data: [], top_processes: [],
});
const alerts = ref<SystemAlert[]>([]);
let unlisten: (() => void) | null = null;
let devInterval: ReturnType<typeof setInterval> | null = null;

function checkAlerts(d: MonitorData) {
  const now = new Date().toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  if (d.cpu_usage > 90 && !alerts.value.find(a => a.message.includes("CPU") && a.level === "danger"))
    alerts.value.unshift({ level: "danger", message: `CPU critique : ${Math.round(d.cpu_usage)}%`, time: now });
  else if (d.cpu_usage > 75 && !alerts.value.find(a => a.message.includes("CPU") && a.level === "warning"))
    alerts.value.unshift({ level: "warning", message: `CPU eleve : ${Math.round(d.cpu_usage)}%`, time: now });
  if (d.ram_percent > 90 && !alerts.value.find(a => a.message.includes("RAM") && a.level === "danger"))
    alerts.value.unshift({ level: "danger", message: `RAM critique : ${Math.round(d.ram_percent)}%`, time: now });
  if (alerts.value.length > 20) alerts.value = alerts.value.slice(0, 20);
}

function tempColor(t: number): string {
  if (t > 85) return "var(--danger)";
  if (t > 70) return "var(--warning)";
  return "var(--success)";
}

async function launchOcct() {
  try { await invoke("execute_tool", { command: "OCCT.exe", isUrl: false }); }
  catch { window.open("https://www.occt.eu/download", "_blank"); }
}

function generateDevData(): MonitorData {
  const cores: CoreUsage[] = Array.from({ length: 8 }, (_, i) => ({ id: i, usage: Math.round(10 + Math.random() * 60) }));
  const cpuAvg = Math.round(cores.reduce((s, c) => s + c.usage, 0) / cores.length);
  return {
    cpu_usage: cpuAvg, cpu_temp_c: Math.round(42 + Math.random() * 30), cpu_cores: cores,
    ram_used_gb: Math.round((12 + Math.random() * 8) * 10) / 10, ram_total_gb: 32, ram_percent: 0,
    disk_read_kbs: Math.round(Math.random() * 50000), disk_write_kbs: Math.round(Math.random() * 30000),
    net_upload_kbs: Math.round(Math.random() * 5000), net_download_kbs: Math.round(Math.random() * 15000),
    gpu_data: [{ name: "NVIDIA RTX 3070", usage_percent: Math.round(Math.random() * 60), vram_used_mb: 3200, vram_total_mb: 8192, temperature_c: Math.round(52 + Math.random() * 28) }],
    top_processes: [
      { name: "chrome.exe", pid: 1234, cpu_percent: Math.round(Math.random() * 15), ram_percent: Math.round(2 + Math.random() * 6) },
      { name: "code.exe", pid: 5678, cpu_percent: Math.round(Math.random() * 10), ram_percent: Math.round(3 + Math.random() * 5) },
      { name: "explorer.exe", pid: 2345, cpu_percent: Math.round(Math.random() * 5), ram_percent: Math.round(1 + Math.random() * 3) },
      { name: "Tauri.exe", pid: 3456, cpu_percent: Math.round(Math.random() * 8), ram_percent: Math.round(1 + Math.random() * 4) },
      { name: "svchost.exe", pid: 800, cpu_percent: Math.round(Math.random() * 3), ram_percent: Math.round(1 + Math.random() * 2) },
    ],
  };
}

function formatSpeed(kbs: number): string {
  if (kbs >= 1024) return `${(kbs / 1024).toFixed(1)} MB/s`;
  return `${Math.round(kbs)} KB/s`;
}

function coreBarColor(usage: number): string {
  if (usage > 90) return "var(--danger)";
  if (usage > 70) return "var(--warning)";
  return "var(--accent-primary)";
}

function togglePause() { paused.value = !paused.value; }

onMounted(async () => {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    await invoke("start_monitoring");
    unlisten = await listen<any>("system-monitor", (event) => {
      if (paused.value) return;
      const raw = event.payload;
      const d: MonitorData = {
        cpu_usage: raw.cpu_percent ?? raw.cpu_usage ?? 0,
        cpu_name: raw.cpu_name ?? undefined,
        cpu_temp_c: raw.cpu_temp_c ?? undefined,
        cpu_cores: (raw.cpu_per_core ?? raw.cpu_cores ?? []).map((u: number | { usage: number }, i: number) =>
          typeof u === "number" ? { id: i, usage: u } : u
        ),
        ram_used_gb: raw.ram_used_gb ?? 0,
        ram_total_gb: raw.ram_total_gb ?? 0,
        ram_percent: raw.ram_percent ?? (raw.ram_total_gb > 0 ? (raw.ram_used_gb / raw.ram_total_gb) * 100 : 0),
        disk_percent: raw.disk_percent ?? undefined,
        disk_read_kbs: raw.disk_read_kbs ?? 0,
        disk_write_kbs: raw.disk_write_kbs ?? 0,
        disk_temps: raw.disk_temps ?? [],
        net_upload_kbs: raw.network_up_kbs ?? raw.net_upload_kbs ?? 0,
        net_download_kbs: raw.network_down_kbs ?? raw.net_download_kbs ?? 0,
        gpu_data: raw.gpu_data ?? [],
        top_processes: (raw.top_processes ?? []).map((p: any) => ({
          name: p.name, pid: p.pid, cpu_percent: p.cpu_percent,
          ram_percent: p.ram_percent ?? 0,
        })),
      };
      data.value = d;
      checkAlerts(d);
      recordFrame(d);
    });
  } catch {
    devInterval = setInterval(() => {
      if (paused.value) return;
      const d = generateDevData();
      d.ram_percent = (d.ram_used_gb / d.ram_total_gb) * 100;
      data.value = d;
      checkAlerts(d);
      recordFrame(d);
    }, 3000);
    const d = generateDevData();
    d.ram_percent = (d.ram_used_gb / d.ram_total_gb) * 100;
    data.value = d;
  }
  loading.value = false;
});

onUnmounted(() => {
  if (unlisten) unlisten();
  if (devInterval) clearInterval(devInterval);
  if (durationTick) clearInterval(durationTick);
  // stop_monitoring retiré — le service de monitoring global est géré par App.vue
});
</script>

<template>
  <div class="monitoring">
    <DiagBanner :icon="Activity" title="Monitoring Temps Réel" desc="CPU, RAM, GPU, réseau et processus en direct" color="blue" />

    <!-- Header -->
    <div class="page-header">
      <div>
        <h1>Monitoring Temps Réel</h1>
        <p class="page-subtitle">Surveillance en direct des ressources système</p>
      </div>
      <div class="header-actions">
        <NBadge :variant="paused ? 'warning' : 'success'">{{ paused ? "En pause" : "En direct" }}</NBadge>

        <!-- Session Recording -->
        <NButton v-if="!recording" variant="ghost" size="sm" @click="startRecording">
          <Circle :size="12" style="color:var(--danger)" />
          Enregistrer
        </NButton>
        <div v-else class="rec-badge">
          <span class="rec-dot"></span>
          <span class="rec-time">{{ durationDisplay }}</span>
          <span style="font-size:11px;color:var(--text-muted)">({{ sessionFrames.length }} pts)</span>
          <NButton variant="ghost" size="sm" @click="stopRecording"><Square :size="11" /> Stop</NButton>
        </div>

        <!-- Annotation -->
        <NButton variant="ghost" size="sm" @click="showAnnotationForm = !showAnnotationForm">
          <PenLine :size="14" />
          Annoter
        </NButton>

        <!-- Export CSV -->
        <NButton variant="ghost" size="sm" @click="exportCSV">
          <Download :size="14" />
          Export CSV
        </NButton>

        <NButton variant="ghost" size="sm" @click="launchOcct">
          <Dumbbell :size="14" />
          Stress Test
        </NButton>
        <NButton variant="secondary" size="sm" @click="togglePause">
          <Pause v-if="!paused" :size="14" /><Play v-else :size="14" />
          {{ paused ? "Reprendre" : "Pause" }}
        </NButton>
      </div>
    </div>

    <!-- Annotation form inline -->
    <div v-if="showAnnotationForm" class="annotation-form">
      <Tag :size="14" style="color:var(--accent-primary);flex-shrink:0" />
      <input
        v-model="annotationInput"
        class="annotation-input"
        placeholder="Note rapide (ex: pic CPU — lancement application)..."
        @keydown.enter="addAnnotation"
        @keydown.escape="showAnnotationForm = false"
        autofocus
      />
      <input type="color" v-model="annotationColor" class="annotation-color" title="Couleur" />
      <NButton variant="primary" size="sm" @click="addAnnotation">Ajouter</NButton>
      <NButton variant="ghost" size="sm" @click="showAnnotationForm = false">✕</NButton>
    </div>

    <!-- Annotations list -->
    <div v-if="annotations.length > 0" class="annotations-bar">
      <div v-for="ann in annotations" :key="ann.id" class="annotation-item" :style="{ borderLeftColor: ann.color }">
        <span class="ann-time">{{ ann.time }}</span>
        <span class="ann-text">{{ ann.text }}</span>
        <button class="ann-remove" @click="removeAnnotation(ann.id)">✕</button>
      </div>
    </div>

    <!-- Session info (terminée) -->
    <div v-if="!recording && sessionFrames.length > 0" class="session-info-bar">
      <Activity :size="14" />
      Session enregistrée : <strong>{{ sessionFrames.length }} points</strong>
      — durée {{ durationDisplay }}
      <NButton variant="ghost" size="sm" @click="exportCSV"><Download :size="12" /> Télécharger CSV</NButton>
      <button class="ann-remove" @click="clearSession()">✕ Effacer</button>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="loading-state">
      <NSpinner :size="32" />
      <p>Démarrage du monitoring...</p>
    </div>

    <template v-else>
      <!-- Stats Cards -->
      <div class="stats-grid stagger-children">
        <StatsCard
          :title="data.cpu_name ? data.cpu_name.split(' ').slice(0,3).join(' ') : 'CPU'"
          :subtitle="data.cpu_temp_c ? `${data.cpu_temp_c}°C — Processeur` : 'Processeur'"
          :value="`${Math.round(data.cpu_usage)}%`"
          :icon="Cpu" :progress="data.cpu_usage" color="accent"
        />
        <StatsCard
          title="RAM"
          :subtitle="`${data.ram_used_gb.toFixed(1)} / ${data.ram_total_gb.toFixed(0)} GB`"
          :value="`${Math.round(data.ram_percent)}%`"
          :icon="MemoryStick" :progress="data.ram_percent" color="info"
        />
        <StatsCard
          v-if="data.gpu_data.length"
          :title="data.gpu_data[0].name.split(' ').slice(-2).join(' ')"
          :subtitle="data.gpu_data[0].temperature_c > 0 ? `${data.gpu_data[0].temperature_c}°C` : 'Carte graphique'"
          :value="`${Math.round(data.gpu_data[0].usage_percent)}%`"
          :icon="Monitor" :progress="data.gpu_data[0].usage_percent" color="warning"
        />
        <StatsCard title="Disque I/O" :subtitle="`R: ${formatSpeed(data.disk_read_kbs)}`"
          :value="formatSpeed(data.disk_write_kbs)" :icon="HardDrive" color="warning" />
        <StatsCard title="Réseau" :subtitle="`Up: ${formatSpeed(data.net_upload_kbs)}`"
          :value="formatSpeed(data.net_download_kbs)" :icon="Wifi" color="success" />
      </div>

      <!-- Températures -->
      <NCard v-if="data.cpu_temp_c || data.gpu_data.some(g => g.temperature_c > 0) || (data.disk_temps && data.disk_temps.length > 0)">
        <template #header>
          <div class="section-header">
            <Thermometer :size="16" />
            <span>Températures Composants</span>
            <Flame v-if="(data.cpu_temp_c ?? 0) > 80 || data.gpu_data.some(g => g.temperature_c > 85) || (data.disk_temps ?? []).some(d => d.temp_c > 55)"
              :size="14" style="color: var(--danger); margin-left: 4px;" />
          </div>
        </template>
        <div class="temps-grid">
          <div v-if="data.cpu_temp_c" class="temp-item">
            <Cpu :size="16" class="temp-icon" />
            <span class="temp-label" :title="data.cpu_name">CPU{{ data.cpu_name ? ` — ${data.cpu_name.split(' ').slice(0,2).join(' ')}` : '' }}</span>
            <div class="temp-bar-wrap">
              <div class="temp-bar-track">
                <div class="temp-bar-fill" :style="{ width: `${Math.min((data.cpu_temp_c / 100) * 100, 100)}%`, background: tempColor(data.cpu_temp_c) }" />
              </div>
            </div>
            <span class="temp-value" :style="{ color: tempColor(data.cpu_temp_c) }">{{ data.cpu_temp_c }}°C</span>
          </div>
          <div v-for="(gpu, i) in data.gpu_data.filter(g => g.temperature_c > 0)" :key="`gpu-${i}`" class="temp-item">
            <Monitor :size="16" class="temp-icon" />
            <span class="temp-label">{{ gpu.name.split(' ').slice(0,3).join(' ') || `GPU #${i+1}` }}</span>
            <div class="temp-bar-wrap">
              <div class="temp-bar-track">
                <div class="temp-bar-fill" :style="{ width: `${Math.min((gpu.temperature_c / 100) * 100, 100)}%`, background: tempColor(gpu.temperature_c) }" />
              </div>
            </div>
            <span class="temp-value" :style="{ color: tempColor(gpu.temperature_c) }">{{ gpu.temperature_c }}°C</span>
          </div>
          <div v-for="(disk, i) in (data.disk_temps ?? [])" :key="`disk-${i}`" class="temp-item">
            <HardDrive :size="16" class="temp-icon" />
            <span class="temp-label">{{ disk.name.length > 20 ? disk.name.slice(0,20)+'…' : disk.name }}</span>
            <div class="temp-bar-wrap">
              <div class="temp-bar-track">
                <div class="temp-bar-fill" :style="{ width: `${Math.min((disk.temp_c / 80) * 100, 100)}%`, background: tempColor(disk.temp_c - 10) }" />
              </div>
            </div>
            <span class="temp-value" :style="{ color: tempColor(disk.temp_c - 10) }">{{ disk.temp_c }}°C</span>
          </div>
        </div>
      </NCard>

      <!-- GPU Cards -->
      <div v-if="data.gpu_data.length" class="gpu-grid">
        <NCard v-for="(gpu, i) in data.gpu_data" :key="i">
          <template #header>
            <div class="section-header">
              <Monitor :size="16" />
              <span>GPU {{ data.gpu_data.length > 1 ? `#${i + 1}` : '' }} — {{ gpu.name }}</span>
            </div>
          </template>
          <div class="gpu-stats">
            <div class="gpu-stat-row">
              <span class="gpu-stat-label">Utilisation</span>
              <div class="gpu-stat-bar">
                <div class="core-bar-track" style="flex:1">
                  <div class="core-bar-fill" :style="{ width: `${gpu.usage_percent}%`, background: coreBarColor(gpu.usage_percent) }" />
                </div>
                <span class="core-value">{{ Math.round(gpu.usage_percent) }}%</span>
              </div>
            </div>
            <div v-if="gpu.vram_total_mb > 0" class="gpu-stat-row">
              <span class="gpu-stat-label">VRAM</span>
              <div class="gpu-stat-bar">
                <div class="core-bar-track" style="flex:1">
                  <div class="core-bar-fill" :style="{ width: `${(gpu.vram_used_mb / gpu.vram_total_mb) * 100}%`, background: 'var(--info)' }" />
                </div>
                <span class="core-value">{{ gpu.vram_used_mb > 0 ? `${gpu.vram_used_mb} / ${gpu.vram_total_mb} MB` : `${gpu.vram_total_mb} MB` }}</span>
              </div>
            </div>
            <div v-if="gpu.temperature_c > 0" class="gpu-stat-row">
              <span class="gpu-stat-label">Température</span>
              <span class="core-value" :style="{ color: gpu.temperature_c > 85 ? 'var(--danger)' : gpu.temperature_c > 70 ? 'var(--warning)' : 'var(--success)' }">
                {{ gpu.temperature_c }}°C
              </span>
            </div>
          </div>
        </NCard>
      </div>

      <div class="monitor-grid">
        <!-- CPU Cores -->
        <NCard>
          <template #header>
            <div class="section-header"><Cpu :size="16" /><span>Usage par Cœur</span></div>
          </template>
          <div class="cores-grid">
            <div v-for="core in data.cpu_cores" :key="core.id" class="core-item">
              <div class="core-label">Core {{ core.id }}</div>
              <div class="core-bar-track">
                <div class="core-bar-fill" :style="{ width: `${core.usage}%`, background: coreBarColor(core.usage) }" />
              </div>
              <div class="core-value">{{ core.usage }}%</div>
            </div>
          </div>
        </NCard>

        <!-- Network speeds -->
        <NCard>
          <template #header>
            <div class="section-header"><Wifi :size="16" /><span>Débit Réseau</span></div>
          </template>
          <div class="net-speeds">
            <div class="net-item">
              <div class="net-icon net-download"><ArrowDown :size="16" /></div>
              <div class="net-info"><div class="net-label">Download</div><div class="net-value">{{ formatSpeed(data.net_download_kbs) }}</div></div>
            </div>
            <div class="net-item">
              <div class="net-icon net-upload"><ArrowUp :size="16" /></div>
              <div class="net-info"><div class="net-label">Upload</div><div class="net-value">{{ formatSpeed(data.net_upload_kbs) }}</div></div>
            </div>
            <div class="net-separator" />
            <div class="net-item">
              <div class="net-icon net-read"><HardDrive :size="16" /></div>
              <div class="net-info"><div class="net-label">Lecture Disque</div><div class="net-value">{{ formatSpeed(data.disk_read_kbs) }}</div></div>
            </div>
            <div class="net-item">
              <div class="net-icon net-write"><HardDrive :size="16" /></div>
              <div class="net-info"><div class="net-label">Écriture Disque</div><div class="net-value">{{ formatSpeed(data.disk_write_kbs) }}</div></div>
            </div>
          </div>
        </NCard>
      </div>

      <!-- Top Processes -->
      <NCard>
        <template #header>
          <div class="section-header"><Activity :size="16" /><span>Processus les plus actifs</span></div>
        </template>
        <div class="table-wrap">
          <table class="data-table">
            <thead>
              <tr><th>Processus</th><th>PID</th><th>CPU</th><th>RAM</th></tr>
            </thead>
            <tbody>
              <tr v-for="proc in data.top_processes" :key="proc.pid">
                <td class="process-name">{{ proc.name }}</td>
                <td class="font-mono">{{ proc.pid }}</td>
                <td>
                  <div class="usage-cell">
                    <NProgress :value="proc.cpu_percent" size="sm" />
                    <span class="usage-val">{{ proc.cpu_percent }}%</span>
                  </div>
                </td>
                <td>
                  <div class="usage-cell">
                    <NProgress :value="proc.ram_percent" size="sm" color="accent" />
                    <span class="usage-val">{{ proc.ram_percent }}%</span>
                  </div>
                </td>
              </tr>
              <tr v-if="!data.top_processes.length">
                <td colspan="4" class="empty-row">Aucun processus détecté</td>
              </tr>
            </tbody>
          </table>
        </div>
      </NCard>

      <!-- Alerts -->
      <NCard>
        <template #header>
          <div class="section-header">
            <AlertTriangle :size="16" />
            <span>Alertes Système</span>
            <NBadge v-if="alerts.length" variant="danger" style="margin-left: auto;">{{ alerts.length }}</NBadge>
          </div>
        </template>
        <div v-if="alerts.length" class="alerts-list">
          <div v-for="(alert, i) in alerts" :key="i" class="alert-item" :class="`alert-${alert.level}`">
            <AlertTriangle :size="14" />
            <span class="alert-time">{{ alert.time }}</span>
            <span class="alert-msg">{{ alert.message }}</span>
          </div>
        </div>
        <div v-else class="empty-state">Aucune alerte. Tout fonctionne normalement.</div>
      </NCard>
    </template>
  </div>
</template>

<style scoped>
.monitoring { display: flex; flex-direction: column; gap: 16px; }
.page-header { display: flex; justify-content: space-between; align-items: flex-start; }
.page-header h1 { font-size: 24px; font-weight: 700; }
.page-subtitle { color: var(--text-secondary); font-size: 13px; margin-top: 2px; }
.header-actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.stats-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 16px; }

/* Session recording badge */
.rec-badge {
  display: flex; align-items: center; gap: 6px; padding: 4px 10px;
  background: rgba(239,68,68,.12); border: 1px solid var(--danger);
  border-radius: var(--radius-md); font-size: 12px;
}
.rec-dot {
  width: 8px; height: 8px; border-radius: 50%; background: var(--danger);
  animation: pulse-rec 1s ease-in-out infinite; flex-shrink: 0;
}
@keyframes pulse-rec { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }
.rec-time { font-family: "JetBrains Mono", monospace; font-size: 12px; font-weight: 700; color: var(--danger); min-width: 38px; }

/* Annotation */
.annotation-form {
  display: flex; align-items: center; gap: 8px; padding: 10px 14px;
  background: var(--bg-secondary); border: 1px solid var(--accent-primary);
  border-radius: var(--radius-md);
}
.annotation-input {
  flex: 1; background: var(--bg-tertiary); border: 1px solid var(--border);
  border-radius: var(--radius-sm); padding: 5px 10px; font-size: 13px;
  color: var(--text-primary); font-family: inherit;
}
.annotation-input:focus { outline: none; border-color: var(--accent-primary); }
.annotation-color { width: 28px; height: 28px; border: none; border-radius: var(--radius-sm); cursor: pointer; background: none; padding: 0; }
.annotations-bar { display: flex; flex-direction: column; gap: 4px; }
.annotation-item {
  display: flex; align-items: center; gap: 8px; padding: 6px 12px;
  background: var(--bg-secondary); border-left: 3px solid var(--accent-primary);
  border-radius: 0 var(--radius-md) var(--radius-md) 0; font-size: 12px;
}
.ann-time { font-family: "JetBrains Mono", monospace; font-size: 11px; color: var(--text-muted); min-width: 65px; }
.ann-text { flex: 1; color: var(--text-primary); font-weight: 500; }
.ann-remove { background: none; border: none; cursor: pointer; color: var(--text-muted); font-size: 12px; padding: 0 4px; }
.ann-remove:hover { color: var(--danger); }

/* Session info bar */
.session-info-bar {
  display: flex; align-items: center; gap: 8px; padding: 8px 14px;
  background: var(--bg-secondary); border: 1px solid var(--border);
  border-radius: var(--radius-md); font-size: 12px; color: var(--text-secondary);
}
.session-info-bar strong { color: var(--text-primary); }

/* Températures */
.temps-grid { display: flex; flex-direction: column; gap: 10px; }
.temp-item { display: flex; align-items: center; gap: 10px; }
.temp-icon { width: 28px; height: 28px; border-radius: 8px; display: flex; align-items: center; justify-content: center; background: linear-gradient(135deg, rgba(239,68,68,.25), rgba(239,68,68,.08)); box-shadow: 0 4px 12px rgba(239,68,68,.3); color: var(--danger); flex-shrink: 0; }
.temp-label { font-size: 12px; color: var(--text-secondary); min-width: 48px; font-family: "JetBrains Mono", monospace; }
.temp-bar-wrap { flex: 1; }
.temp-bar-track { height: 6px; border-radius: 99px; background: var(--bg-elevated); border: 1px solid var(--border); overflow: hidden; }
.temp-bar-fill { height: 100%; border-radius: 99px; transition: width 400ms ease, background 400ms ease; }
.temp-value { font-size: 13px; font-weight: 600; font-family: "JetBrains Mono", monospace; min-width: 44px; text-align: right; }

.loading-state { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; padding: 60px; color: var(--text-secondary); }

.section-header { display: flex; align-items: center; gap: 8px; border-left: 3px solid var(--accent-primary); padding-left: 10px; font-weight: 700; color: var(--text-primary); }
.monitor-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
@media (max-width: 1000px) { .monitor-grid { grid-template-columns: 1fr; } }

/* GPU */
.gpu-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(340px, 1fr)); gap: 16px; }
.gpu-stats { display: flex; flex-direction: column; gap: 10px; }
.gpu-stat-row { display: flex; align-items: center; gap: 10px; }
.gpu-stat-label { font-size: 12px; color: var(--text-secondary); min-width: 90px; }
.gpu-stat-bar { flex: 1; display: flex; align-items: center; gap: 8px; }

/* Cores */
.cores-grid { display: flex; flex-direction: column; gap: 8px; }
.core-item { display: flex; align-items: center; gap: 10px; }
.core-label { font-size: 12px; color: var(--text-secondary); min-width: 50px; font-family: "JetBrains Mono", monospace; }
.core-bar-track { flex: 1; height: 6px; border-radius: 99px; background: var(--bg-elevated); border: 1px solid var(--border); overflow: hidden; }
.core-bar-fill { height: 100%; border-radius: 99px; background: linear-gradient(90deg, var(--accent-primary), var(--accent-hover)); transition: width 300ms ease; }
.core-value { font-size: 12px; font-weight: 500; color: var(--text-secondary); font-family: "JetBrains Mono", monospace; min-width: 36px; text-align: right; }

/* Net speeds */
.net-speeds { display: flex; flex-direction: column; gap: 12px; }
.net-item { display: flex; align-items: center; gap: 12px; }
.net-icon { width: 36px; height: 36px; border-radius: var(--radius-md); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.net-download { background: linear-gradient(135deg, rgba(34,197,94,.25), rgba(34,197,94,.08)); color: var(--success); }
.net-upload { background: linear-gradient(135deg, rgba(59,130,246,.25), rgba(59,130,246,.08)); color: var(--info); }
.net-read { background: var(--info-muted); color: var(--info); }
.net-write { background: var(--warning-muted); color: var(--warning); }
.net-label { font-size: 12px; color: var(--text-secondary); }
.net-value { font-size: 16px; font-weight: 600; color: var(--text-primary); font-family: "JetBrains Mono", monospace; }
.net-separator { height: 1px; background: var(--border); margin: 4px 0; }

/* Table */
.table-wrap { overflow-x: auto; }
.data-table { width: 100%; border-collapse: collapse; font-size: 13px; }
.data-table th { text-align: left; padding: 8px 12px; color: var(--text-secondary); font-weight: 700; font-size: 11px; text-transform: uppercase; letter-spacing: .06em; border-bottom: 1px solid var(--border); background: var(--bg-elevated); }
.data-table td { padding: 8px 12px; color: var(--text-secondary); border-bottom: 1px solid var(--border); }
.data-table tbody tr:hover { background: rgba(249,115,22,.06); }
.process-name { font-weight: 500; color: var(--text-primary) !important; }
.font-mono { font-family: "JetBrains Mono", monospace; font-size: 12px; }
.usage-cell { display: flex; align-items: center; gap: 8px; min-width: 120px; }
.usage-val { font-family: "JetBrains Mono", monospace; font-size: 12px; min-width: 36px; text-align: right; }
.empty-row { text-align: center; color: var(--text-secondary) !important; padding: 20px !important; }

/* Alerts */
.alerts-list { display: flex; flex-direction: column; gap: 6px; max-height: 200px; overflow-y: auto; }
.alert-item { display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-radius: var(--radius-md); font-size: 13px; }
.alert-warning { background: var(--warning-muted); color: var(--warning); border: 1px solid var(--warning-muted); }
.alert-danger { background: var(--danger-muted); color: var(--danger); border: 1px solid var(--danger-muted); }
.alert-info { background: var(--info-muted); color: var(--info); }
.alert-time { font-family: "JetBrains Mono", monospace; font-size: 11px; opacity: 0.8; min-width: 70px; }
.alert-msg { font-weight: 500; }
.empty-state { text-align: center; color: var(--text-secondary); font-size: 13px; padding: 20px; }
</style>
