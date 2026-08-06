<script setup lang="ts">
import { ref, computed, onUnmounted } from "vue";
import { invoke } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NProgress from "@/components/ui/NProgress.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import SparklineChart from "@/components/ui/SparklineChart.vue";
import { useNotificationStore } from "@/stores/notifications";
import { useExportData } from "@/composables/useExportData";
import {
  Activity, Cpu, MemoryStick, HardDrive, Wifi,
  Play, Pause, Trophy, Download, Monitor,
} from "lucide-vue-next";

const notify = useNotificationStore();
const { exportCSV } = useExportData();

interface PerfPoint {
  timestamp: string; cpu_percent: number; ram_used_mb: number; ram_total_mb: number;
  disk_read_mbps: number; disk_write_mbps: number; net_recv_mbps: number; net_send_mbps: number;
  gpu_usage?: number;
}
interface PerfHistory {
  points: PerfPoint[]; sample_interval_secs: number; duration_secs: number;
  avg_cpu: number; peak_cpu: number; avg_ram_mb: number; peak_ram_mb: number;
}
interface TopProcess { name: string; pid: number; cpu_percent: number; ram_mb: number; disk_mbps: number; }

const history  = ref<PerfHistory | null>(null);
const topProcs = ref<TopProcess[]>([]);
const loading  = ref(false);
const paused   = ref(false);
const samples  = ref(20);
const interval = ref(3);

// Tri des top processus
type SortKey = "cpu" | "ram" | "disk";
const sortKey = ref<SortKey>("cpu");

const sortedProcs = computed(() => {
  if (!topProcs.value.length) return [];
  return [...topProcs.value].sort((a, b) => {
    if (sortKey.value === "cpu")  return b.cpu_percent - a.cpu_percent;
    if (sortKey.value === "ram")  return b.ram_mb - a.ram_mb;
    if (sortKey.value === "disk") return b.disk_mbps - a.disk_mbps;
    return 0;
  });
});

const cpuHistory  = computed(() => history.value?.points.map(p => p.cpu_percent) ?? []);
const ramHistory  = computed(() => history.value?.points.map(p => Math.round(p.ram_used_mb / (p.ram_total_mb || 1) * 100)) ?? []);
const netHistory  = computed(() => history.value?.points.map(p => p.net_recv_mbps + p.net_send_mbps) ?? []);
const diskHistory = computed(() => history.value?.points.map(p => p.disk_read_mbps + p.disk_write_mbps) ?? []);
const gpuHistory  = computed(() => history.value?.points.map(p => p.gpu_usage ?? 0) ?? []);

const hasGpu = computed(() => gpuHistory.value.some(v => v > 0));

// Stats min/max/avg pour chaque série
function calcStats(arr: number[]): { min: number; max: number; avg: number } {
  if (!arr.length) return { min: 0, max: 0, avg: 0 };
  const min = Math.min(...arr);
  const max = Math.max(...arr);
  const avg = arr.reduce((a, b) => a + b, 0) / arr.length;
  return { min, max, avg };
}

const cpuStats  = computed(() => calcStats(cpuHistory.value));
const ramStats  = computed(() => calcStats(ramHistory.value));
const netStats  = computed(() => calcStats(netHistory.value));
const diskStats = computed(() => calcStats(diskHistory.value));
const gpuStats  = computed(() => calcStats(gpuHistory.value));

const ramPct = computed(() => {
  if (!history.value) return 0;
  const avg   = history.value.avg_ram_mb;
  const total = history.value.points[0]?.ram_total_mb || 1;
  return Math.round((avg / total) * 100);
});

const avgDiskMbps = computed(() => {
  const pts = history.value?.points;
  if (!pts?.length) return "0.0";
  return (pts.reduce((s, p) => s + p.disk_read_mbps + p.disk_write_mbps, 0) / pts.length).toFixed(1);
});

const avgNetMbps = computed(() => {
  const pts = history.value?.points;
  if (!pts?.length) return "0.00";
  return (pts.reduce((s, p) => s + p.net_recv_mbps + p.net_send_mbps, 0) / pts.length).toFixed(2);
});

let pollTimer: ReturnType<typeof setInterval> | null = null;
let aborted = false;

onUnmounted(() => {
  aborted = true;
  if (pollTimer !== null) clearInterval(pollTimer);
});

async function run() {
  if (paused.value) return;
  aborted = false;
  loading.value = true;
  notify.info("Collecte en cours", `${samples.value} échantillons × ${interval.value}s = ${samples.value * interval.value}s`);
  try {
    const [hist, procs] = await Promise.all([
      invoke<PerfHistory>("get_perf_history", { samples: samples.value, intervalSecs: interval.value }),
      invoke<TopProcess[]>("get_top_processes_by_cpu", { limit: 10 }),
    ]);
    if (!aborted) {
      history.value = hist;
      topProcs.value = procs;
      notify.success("Analyse terminée", `${hist.points.length} points collectés`);
    }
  } catch (e: unknown) {
    if (!aborted) notify.error("Erreur", String(e));
  }
  if (!aborted) loading.value = false;
}

function togglePause() {
  paused.value = !paused.value;
  if (!paused.value) {
    // Reprend — relance immédiatement
    run();
  } else {
    notify.info("Collecte suspendue", "Cliquez sur Reprendre pour continuer.");
  }
}

function exportHistory() {
  if (!history.value?.points.length) {
    notify.warning("Export", "Aucune donnée à exporter.");
    return;
  }
  const rows = history.value.points.map(p => ({
    timestamp:       p.timestamp,
    cpu_percent:     p.cpu_percent,
    ram_used_mb:     p.ram_used_mb,
    ram_total_mb:    p.ram_total_mb,
    disk_read_mbps:  p.disk_read_mbps,
    disk_write_mbps: p.disk_write_mbps,
    net_recv_mbps:   p.net_recv_mbps,
    net_send_mbps:   p.net_send_mbps,
    gpu_usage:       p.gpu_usage ?? 0,
  }));
  exportCSV(rows, `perf-history-${new Date().toISOString().slice(0, 10)}`);
  notify.success("Export CSV", `${rows.length} points exportés`);
}
</script>

<template>
  <div class="perf-page">
    <div class="page-header">
      <div class="header-icon"><Activity :size="22" /></div>
      <div>
        <h1>Historique Performances</h1>
        <p class="subtitle">Analyse approfondie CPU, RAM, disque et réseau sur une période</p>
      </div>
    </div>

    <!-- Config -->
    <NCard>
      <template #header>
        <div class="section-header">
          <Play :size="14" /><span>Configuration</span>
          <!-- Boutons export + pause/resume -->
          <div style="margin-left:auto;display:flex;gap:8px;align-items:center">
            <NButton variant="secondary" size="sm" :disabled="!history" @click="exportHistory">
              <Download :size="13" /> Export CSV
            </NButton>
            <NButton :variant="paused ? 'success' : 'warning'" size="sm" :disabled="loading && !paused" @click="togglePause">
              <Pause v-if="!paused" :size="13" />
              <Play  v-else         :size="13" />
              {{ paused ? 'Reprendre' : 'Pause' }}
            </NButton>
          </div>
        </div>
      </template>
      <div class="config-row">
        <label class="config-item">
          <span class="config-label">Échantillons</span>
          <select v-model="samples" class="mini-select">
            <option :value="10">10</option><option :value="20">20</option>
            <option :value="30">30</option><option :value="60">60</option>
          </select>
        </label>
        <label class="config-item">
          <span class="config-label">Intervalle</span>
          <select v-model="interval" class="mini-select">
            <option :value="1">1s</option><option :value="2">2s</option>
            <option :value="3">3s</option><option :value="5">5s</option><option :value="10">10s</option>
          </select>
        </label>
        <span class="duration-badge">Durée : {{ samples * interval }}s</span>
        <NButton variant="primary" size="sm" :loading="loading" :disabled="loading || paused" @click="run">
          <Play :size="13" /> {{ loading ? 'En cours...' : 'Lancer l\'analyse' }}
        </NButton>
        <NBadge v-if="paused" variant="warning">Suspendu</NBadge>
      </div>
      <div v-if="loading" class="loading-hint">
        <NSpinner :size="14" /> Collecte des métriques en cours — veuillez patienter...
      </div>
    </NCard>

    <template v-if="history">
      <!-- Stats résumé -->
      <div class="stats-grid">
        <!-- CPU -->
        <div class="stat-card">
          <div class="stat-icon cpu"><Cpu :size="18" /></div>
          <div class="stat-info">
            <span class="stat-big">{{ history.avg_cpu.toFixed(1) }}%</span>
            <span class="stat-sub">CPU moyen</span>
            <span class="stat-peak">Pic : {{ history.peak_cpu.toFixed(1) }}%</span>
          </div>
          <div class="sparkline-col">
            <SparklineChart :data="cpuHistory" color="var(--accent-primary)" :height="40" :fill="true" label="cpu" />
            <div class="minmax-row">
              <span class="minmax-label">Min <strong>{{ cpuStats.min.toFixed(1) }}%</strong></span>
              <span class="minmax-label">Moy <strong>{{ cpuStats.avg.toFixed(1) }}%</strong></span>
              <span class="minmax-label">Max <strong>{{ cpuStats.max.toFixed(1) }}%</strong></span>
            </div>
          </div>
        </div>
        <!-- RAM -->
        <div class="stat-card">
          <div class="stat-icon ram"><MemoryStick :size="18" /></div>
          <div class="stat-info">
            <span class="stat-big">{{ ramPct }}%</span>
            <span class="stat-sub">RAM moyenne ({{ Math.round(history.avg_ram_mb / 1024) }} Go)</span>
            <span class="stat-peak">Pic : {{ Math.round(history.peak_ram_mb / 1024 * 10) / 10 }} Go</span>
          </div>
          <div class="sparkline-col">
            <SparklineChart :data="ramHistory" color="var(--success)" :height="40" :fill="true" label="ram" />
            <div class="minmax-row">
              <span class="minmax-label">Min <strong>{{ ramStats.min }}%</strong></span>
              <span class="minmax-label">Moy <strong>{{ ramStats.avg.toFixed(0) }}%</strong></span>
              <span class="minmax-label">Max <strong>{{ ramStats.max }}%</strong></span>
            </div>
          </div>
        </div>
        <!-- Disque -->
        <div class="stat-card">
          <div class="stat-icon disk"><HardDrive :size="18" /></div>
          <div class="stat-info">
            <span class="stat-big">{{ avgDiskMbps }} MB/s</span>
            <span class="stat-sub">Disque moyen</span>
          </div>
          <div class="sparkline-col">
            <SparklineChart :data="diskHistory" color="var(--warning)" :height="40" :fill="true" label="disk" />
            <div class="minmax-row">
              <span class="minmax-label">Min <strong>{{ diskStats.min.toFixed(1) }}</strong></span>
              <span class="minmax-label">Moy <strong>{{ diskStats.avg.toFixed(1) }}</strong></span>
              <span class="minmax-label">Max <strong>{{ diskStats.max.toFixed(1) }}</strong></span>
            </div>
          </div>
        </div>
        <!-- Réseau -->
        <div class="stat-card">
          <div class="stat-icon net"><Wifi :size="18" /></div>
          <div class="stat-info">
            <span class="stat-big">{{ avgNetMbps }} MB/s</span>
            <span class="stat-sub">Réseau moyen</span>
          </div>
          <div class="sparkline-col">
            <SparklineChart :data="netHistory" color="var(--info)" :height="40" :fill="true" label="net" />
            <div class="minmax-row">
              <span class="minmax-label">Min <strong>{{ netStats.min.toFixed(2) }}</strong></span>
              <span class="minmax-label">Moy <strong>{{ netStats.avg.toFixed(2) }}</strong></span>
              <span class="minmax-label">Max <strong>{{ netStats.max.toFixed(2) }}</strong></span>
            </div>
          </div>
        </div>
        <!-- GPU (si disponible) -->
        <div v-if="hasGpu" class="stat-card stat-card-gpu">
          <div class="stat-icon gpu"><Monitor :size="18" /></div>
          <div class="stat-info">
            <span class="stat-big">{{ gpuStats.avg.toFixed(1) }}%</span>
            <span class="stat-sub">GPU moyen</span>
            <span class="stat-peak">Pic : {{ gpuStats.max.toFixed(1) }}%</span>
          </div>
          <div class="sparkline-col">
            <SparklineChart :data="gpuHistory" color="var(--purple, #a855f7)" :height="40" :fill="true" label="gpu" />
            <div class="minmax-row">
              <span class="minmax-label">Min <strong>{{ gpuStats.min.toFixed(1) }}%</strong></span>
              <span class="minmax-label">Moy <strong>{{ gpuStats.avg.toFixed(1) }}%</strong></span>
              <span class="minmax-label">Max <strong>{{ gpuStats.max.toFixed(1) }}%</strong></span>
            </div>
          </div>
        </div>
      </div>

      <!-- Tableau des points -->
      <NCard>
        <template #header>
          <div class="section-header"><Activity :size="14" /><span>Timeline ({{ history.points.length }} points)</span></div>
        </template>
        <div class="timeline-wrap">
          <table class="timeline-table">
            <thead>
              <tr>
                <th>Heure</th><th>CPU %</th><th>RAM %</th><th>Disque R/W</th><th>Réseau ↓/↑</th>
                <th v-if="hasGpu">GPU %</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(p, i) in history.points" :key="i">
                <td class="mono">{{ p.timestamp }}</td>
                <td>
                  <div class="mini-bar">
                    <div class="mini-fill cpu-fill" :style="{ width: p.cpu_percent + '%' }" />
                    <span>{{ p.cpu_percent.toFixed(1) }}%</span>
                  </div>
                </td>
                <td>
                  <div class="mini-bar">
                    <div class="mini-fill ram-fill" :style="{ width: Math.round(p.ram_used_mb / (p.ram_total_mb || 1) * 100) + '%' }" />
                    <span>{{ Math.round(p.ram_used_mb / (p.ram_total_mb || 1) * 100) }}%</span>
                  </div>
                </td>
                <td class="mono">{{ p.disk_read_mbps.toFixed(1) }} / {{ p.disk_write_mbps.toFixed(1) }}</td>
                <td class="mono">{{ p.net_recv_mbps.toFixed(2) }} / {{ p.net_send_mbps.toFixed(2) }}</td>
                <td v-if="hasGpu" class="mono">{{ (p.gpu_usage ?? 0).toFixed(1) }}%</td>
              </tr>
            </tbody>
          </table>
        </div>
      </NCard>

      <!-- Top processus -->
      <NCard v-if="topProcs.length">
        <template #header>
          <div class="section-header">
            <Trophy :size="14" />
            <span>Top processus</span>
            <!-- Dropdown tri -->
            <div style="margin-left:auto;display:flex;gap:6px;align-items:center">
              <span style="font-size:11px;color:var(--text-muted)">Trier par</span>
              <select v-model="sortKey" class="mini-select">
                <option value="cpu">CPU</option>
                <option value="ram">RAM</option>
                <option value="disk">Disk I/O</option>
              </select>
            </div>
          </div>
        </template>
        <div class="procs-list">
          <div v-for="p in sortedProcs" :key="p.pid" class="proc-row">
            <span class="proc-name">{{ p.name }}</span>
            <span class="proc-pid mono">PID {{ p.pid }}</span>
            <div class="proc-bar-wrap">
              <div class="proc-bar-track">
                <div class="proc-bar-fill" :style="{ width: Math.min(p.cpu_percent, 100) + '%' }" />
              </div>
              <span class="proc-val">{{ p.cpu_percent.toFixed(1) }}%</span>
            </div>
            <span class="proc-ram">{{ p.ram_mb }} Mo</span>
            <span class="proc-ram mono">{{ p.disk_mbps.toFixed(1) }} MB/s</span>
          </div>
        </div>
      </NCard>
      <!-- get_top_processes_by_cpu() renvoie un Vec brut (pas de Result) : un
           echec interne PS/JSON retombe silencieusement sur [] sans exception ni
           toast (meme defaut deja corrige dans DiagTabPerfHistory.vue, cycle 116).
           history et topProcs se resolvent ensemble dans le meme Promise.all, donc
           si history est deja affiche, topProcs vide est definitif — sans ce
           v-else, toute la carte "Top processus" disparaissait sans explication. -->
      <NCard v-else>
        <template #header>
          <div class="section-header"><Trophy :size="14" /><span>Top processus</span></div>
        </template>
        <p style="font-size:12px;color:var(--text-muted);padding:8px 0">Aucun processus détecté. Relancez l'analyse pour réessayer.</p>
      </NCard>
    </template>
  </div>
</template>

<style scoped>
.perf-page { display: flex; flex-direction: column; gap: 14px; }
.page-header { display: flex; align-items: center; gap: 12px; }
.header-icon { width: 42px; height: 42px; border-radius: var(--radius-lg); background: var(--accent-muted); display: flex; align-items: center; justify-content: center; color: var(--accent-primary); flex-shrink: 0; }
h1 { font-size: 22px; font-weight: 700; }
.subtitle { font-size: 12px; color: var(--text-muted); }
.section-header { display: flex; align-items: center; gap: 8px; width: 100%; }
.config-row { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; }
.config-item { display: flex; align-items: center; gap: 8px; }
.config-label { font-size: 12px; color: var(--text-muted); }
.mini-select { padding: 5px 9px; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-primary); font-size: 12px; cursor: pointer; }
.duration-badge { font-size: 12px; color: var(--accent-primary); background: var(--accent-muted); padding: 4px 10px; border-radius: 99px; font-family: monospace; }
.loading-hint { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--text-muted); margin-top: 10px; }

.stats-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; }
@media (max-width: 900px) { .stats-grid { grid-template-columns: 1fr 1fr; } }

.stat-card { display: flex; align-items: flex-start; gap: 12px; padding: 14px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius-xl); flex-direction: column; }
.stat-card-gpu { grid-column: 1 / -1; flex-direction: row; flex-wrap: wrap; }

.stat-icon { width: 38px; height: 38px; border-radius: var(--radius-md); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.stat-icon.cpu { background: var(--accent-muted); color: var(--accent-primary); }
.stat-icon.ram { background: var(--success-muted); color: var(--success); }
.stat-icon.disk { background: var(--warning-muted); color: var(--warning); }
.stat-icon.net { background: color-mix(in srgb, var(--info) 15%, transparent); color: var(--info); }
.stat-icon.gpu { background: color-mix(in srgb, #a855f7 15%, transparent); color: #a855f7; }

.stat-info { flex: 1; display: flex; flex-direction: column; gap: 1px; }
.stat-big { font-size: 18px; font-weight: 800; color: var(--text-primary); }
.stat-sub { font-size: 11px; color: var(--text-muted); }
.stat-peak { font-size: 10px; color: var(--text-muted); }

.sparkline-col { display: flex; flex-direction: column; gap: 4px; width: 100%; }

.minmax-row { display: flex; gap: 10px; justify-content: space-between; }
.minmax-label { font-size: 10px; color: var(--text-muted); }
.minmax-label strong { color: var(--text-secondary); font-weight: 600; }

.timeline-wrap { max-height: 360px; overflow-y: auto; border: 1px solid var(--border); border-radius: var(--radius-md); }
.timeline-table { width: 100%; border-collapse: collapse; font-size: 12px; }
.timeline-table th { position: sticky; top: 0; background: var(--bg-tertiary); padding: 7px 10px; text-align: left; font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: .06em; color: var(--text-muted); border-bottom: 1px solid var(--border); }
.timeline-table td { padding: 6px 10px; border-bottom: 1px solid var(--border); color: var(--text-secondary); }
.timeline-table tr:hover td { background: var(--bg-tertiary); }
.mini-bar { display: flex; align-items: center; gap: 6px; }
.mini-fill { height: 6px; border-radius: 99px; flex-shrink: 0; transition: width .3s; }
.cpu-fill { background: var(--accent-primary); width: 0; }
.ram-fill { background: var(--success); width: 0; }
.mono { font-family: "JetBrains Mono", monospace; }

.procs-list { display: flex; flex-direction: column; gap: 4px; }
.proc-row { display: grid; grid-template-columns: 180px 80px 1fr 70px 80px; align-items: center; gap: 10px; padding: 7px 10px; border-bottom: 1px solid var(--border); font-size: 12px; }
.proc-row:last-child { border-bottom: none; }
.proc-name { font-weight: 600; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.proc-pid { color: var(--text-muted); font-size: 11px; }
.proc-bar-wrap { display: flex; align-items: center; gap: 8px; }
.proc-bar-track { flex: 1; height: 6px; background: var(--bg-elevated); border-radius: 99px; overflow: hidden; }
.proc-bar-fill { height: 100%; background: var(--accent-primary); border-radius: 99px; }
.proc-val { font-family: monospace; font-size: 11px; color: var(--text-muted); width: 50px; text-align: right; flex-shrink: 0; }
.proc-ram { font-family: monospace; font-size: 11px; color: var(--text-muted); text-align: right; }
</style>
