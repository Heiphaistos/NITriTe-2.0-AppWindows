<template>
  <div class="ph-root">
    <!-- Banner -->
    <div class="ph-banner">
      <div class="ph-banner-icon"><Activity :size="26" /></div>
      <div class="ph-banner-text">
        <div class="ph-banner-title">Historique des Performances</div>
        <div class="ph-banner-desc">Collecte CPU, RAM, Disque et Réseau sur une période définie</div>
      </div>
      <div class="ph-banner-controls">
        <div class="ph-control-group">
          <label class="ph-ctrl-label">Échantillons</label>
          <select v-model.number="samples" class="ph-select">
            <option :value="10">10</option><option :value="20">20</option>
            <option :value="30">30</option><option :value="60">60</option>
          </select>
        </div>
        <div class="ph-control-group">
          <label class="ph-ctrl-label">Intervalle</label>
          <select v-model.number="interval" class="ph-select">
            <option :value="1">1 sec</option><option :value="2">2 sec</option><option :value="5">5 sec</option>
          </select>
        </div>
        <button class="ph-btn ph-btn-primary" :disabled="loading" @click="runHistory">
          <Play :size="13" /> Lancer ({{ samples * interval }}s)
        </button>
      </div>
    </div>

    <!-- Loading progress -->
    <div v-if="loading" class="ph-progress">
      <div class="ph-progress-bar-wrap">
        <div class="ph-progress-bar" :style="{ width: (progressPts / samples * 100) + '%' }" />
      </div>
      <span class="ph-progress-text"><div class="ph-spinner" /> Collecte {{ progressPts }}/{{ samples }} échantillons...</span>
    </div>

    <div v-if="history">
      <!-- Summary stats -->
      <div class="ph-stats">
        <div class="ph-stat" :class="history.avg_cpu > 80 ? 'st-red' : history.avg_cpu > 50 ? 'st-orange' : 'st-green'">
          <div class="ph-stat-icon"><Cpu :size="16" /></div>
          <div class="ph-stat-val">{{ history.avg_cpu.toFixed(1) }}%</div>
          <div class="ph-stat-lbl">CPU Moyen</div>
        </div>
        <div class="ph-stat" :class="history.peak_cpu > 80 ? 'st-red' : history.peak_cpu > 50 ? 'st-orange' : 'st-green'">
          <div class="ph-stat-icon"><TrendingUp :size="16" /></div>
          <div class="ph-stat-val">{{ history.peak_cpu.toFixed(1) }}%</div>
          <div class="ph-stat-lbl">CPU Pic</div>
        </div>
        <div class="ph-stat st-blue">
          <div class="ph-stat-icon"><MemoryStick :size="16" /></div>
          <div class="ph-stat-val">{{ fmtMb(history.avg_ram_mb) }}</div>
          <div class="ph-stat-lbl">RAM Moyenne</div>
        </div>
        <div class="ph-stat st-purple">
          <div class="ph-stat-icon"><MemoryStick :size="16" /></div>
          <div class="ph-stat-val">{{ fmtMb(history.peak_ram_mb) }}</div>
          <div class="ph-stat-lbl">RAM Pic</div>
        </div>
        <div class="ph-stat st-gray">
          <div class="ph-stat-icon"><Clock :size="16" /></div>
          <div class="ph-stat-val">{{ history.duration_secs }}s</div>
          <div class="ph-stat-lbl">Durée</div>
        </div>
      </div>

      <!-- CPU Chart -->
      <div class="ph-chart-wrap">
        <div class="ph-chart-header">
          <span class="ph-chart-title"><Cpu :size="13" /> CPU %</span>
          <span class="ph-chart-avg">moy. {{ history.avg_cpu.toFixed(1) }}% · pic {{ history.peak_cpu.toFixed(1) }}%</span>
        </div>
        <div class="ph-chart">
          <div class="ph-grid">
            <div v-for="l in [75,50,25]" :key="l" class="ph-grid-line" :style="{bottom: l+'%'}">
              <span class="ph-grid-lbl">{{ l }}%</span>
            </div>
          </div>
          <div class="ph-bars">
            <div v-for="(p, i) in history.points" :key="i" class="ph-bar-col" :title="`${p.timestamp} — CPU: ${p.cpu_percent.toFixed(1)}%`">
              <div class="ph-bar" :style="{height: Math.max(2,p.cpu_percent)+'%', background: pctGrad(p.cpu_percent)}" />
            </div>
          </div>
        </div>
        <div class="ph-xaxis">
          <span v-for="(p, i) in history.points" :key="i"
                v-show="i % Math.max(1, Math.floor(history.points.length / 8)) === 0"
                class="ph-xlabel">{{ p.timestamp }}</span>
        </div>
      </div>

      <!-- RAM Chart -->
      <div class="ph-chart-wrap">
        <div class="ph-chart-header">
          <span class="ph-chart-title"><MemoryStick :size="13" /> RAM utilisée</span>
          <span class="ph-chart-avg">moy. {{ fmtMb(history.avg_ram_mb) }} · pic {{ fmtMb(history.peak_ram_mb) }}</span>
        </div>
        <div class="ph-chart">
          <div class="ph-grid">
            <div v-for="l in [75,50,25]" :key="l" class="ph-grid-line" :style="{bottom: l+'%'}">
              <span class="ph-grid-lbl">{{ fmtMb(Math.round(maxRam * l / 100)) }}</span>
            </div>
          </div>
          <div class="ph-bars">
            <div v-for="(p, i) in history.points" :key="i" class="ph-bar-col"
                 :title="`${p.timestamp} — RAM: ${p.ram_used_mb}MB / ${p.ram_total_mb}MB`">
              <div class="ph-bar" :style="{height: Math.max(2, p.ram_used_mb/maxRam*100)+'%', background: 'linear-gradient(to top,#1d4ed8,#3b82f6)'}" />
            </div>
          </div>
        </div>
      </div>

      <!-- Raw data toggle -->
      <details class="ph-raw-details">
        <summary class="ph-raw-summary"><Table :size="13" /> Données brutes ({{ history.points.length }} points)</summary>
        <div class="ph-raw-table-wrap">
          <table class="ph-table">
            <thead><tr><th>Heure</th><th>CPU %</th><th>RAM MB</th><th>D.Lect</th><th>D.Écr</th><th>Net↓</th><th>Net↑</th></tr></thead>
            <tbody>
              <tr v-for="(p, i) in history.points" :key="i">
                <td>{{ p.timestamp }}</td>
                <td :style="{color: pctColor(p.cpu_percent)}">{{ p.cpu_percent.toFixed(1) }}%</td>
                <td>{{ p.ram_used_mb }}</td>
                <td>{{ p.disk_read_mbps.toFixed(2) }}</td>
                <td>{{ p.disk_write_mbps.toFixed(2) }}</td>
                <td>{{ p.net_recv_mbps.toFixed(3) }}</td>
                <td>{{ p.net_send_mbps.toFixed(3) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </details>
    </div>

    <!-- Top Processes -->
    <div class="ph-top-section">
      <div class="ph-top-header">
        <div class="ph-top-title"><Cpu :size="14" /> Top Processus par CPU</div>
        <button class="ph-btn" :disabled="topLoading" @click="loadTop"><RefreshCw :size="12" /> Actualiser</button>
      </div>
      <div v-if="topLoading" class="ph-top-loading"><div class="ph-spinner" /> Chargement...</div>
      <div v-else-if="topProcs.length > 0" class="ph-top-list">
        <div v-for="(p, i) in topProcs" :key="p.pid" class="ph-top-proc">
          <span class="ph-proc-rank" :class="i < 3 ? 'rank-top' : ''">#{{ i + 1 }}</span>
          <div class="ph-proc-info">
            <div class="ph-proc-name">{{ p.name }}</div>
            <div class="ph-proc-pid">PID {{ p.pid }}</div>
          </div>
          <div class="ph-proc-bars">
            <div class="ph-proc-bar-wrap">
              <div class="ph-proc-bar cpu-bar" :style="{ width: Math.min(100, p.cpu_percent / (topProcs[0]?.cpu_percent || 1) * 100) + '%' }" />
            </div>
            <span class="ph-proc-cpu" :style="{color: pctColor(p.cpu_percent)}">{{ p.cpu_percent.toFixed(1) }}s</span>
          </div>
          <span class="ph-proc-ram">{{ p.ram_mb }} MB</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke, invokeRaw } from "@/utils/invoke";
import { Activity, Play, Cpu, RefreshCw, TrendingUp, Clock, HardDrive as MemoryStick, List as Table } from 'lucide-vue-next'

interface PerfPoint {
  timestamp: string; cpu_percent: number; ram_used_mb: number; ram_total_mb: number
  disk_read_mbps: number; disk_write_mbps: number; net_recv_mbps: number; net_send_mbps: number
}
interface PerfHistory {
  points: PerfPoint[]; sample_interval_secs: number; duration_secs: number
  avg_cpu: number; peak_cpu: number; avg_ram_mb: number; peak_ram_mb: number
}
interface TopProcess { name: string; pid: number; cpu_percent: number; ram_mb: number; disk_mbps: number }

const loading = ref(false); const history = ref<PerfHistory | null>(null)
const samples = ref(20); const interval = ref(2); const progressPts = ref(0)
const topLoading = ref(false); const topProcs = ref<TopProcess[]>([])

const maxRam = computed(() => {
  if (!history.value) return 1
  return Math.max(...history.value.points.map(p => p.ram_total_mb), 1)
})

async function runHistory() {
  loading.value = true; history.value = null; progressPts.value = 0
  try { history.value = await invokeRaw<PerfHistory>('get_perf_history', { samples: samples.value, intervalSecs: interval.value }) }
  finally { loading.value = false }
}

async function loadTop() {
  topLoading.value = true
  try { topProcs.value = await invoke<TopProcess[]>('get_top_processes_by_cpu', { limit: 20 }) }
  finally { topLoading.value = false }
}

function pctColor(v: number): string { return v > 80 ? '#ef4444' : v > 50 ? '#f59e0b' : '#22c55e' }
function pctGrad(v: number): string { return v > 80 ? 'linear-gradient(to top,#b91c1c,#ef4444)' : v > 50 ? 'linear-gradient(to top,#b45309,#f59e0b)' : 'linear-gradient(to top,#15803d,#22c55e)' }
function fmtMb(mb: number): string { return mb >= 1024 ? (mb / 1024).toFixed(1) + ' GB' : mb + ' MB' }

onMounted(loadTop)
</script>

<style scoped>
.ph-root { display: flex; flex-direction: column; gap: 14px; }

/* Banner */
.ph-banner { display: flex; align-items: center; gap: 16px; padding: 18px 22px;
  background: linear-gradient(135deg, rgba(20,184,166,.13), rgba(6,182,212,.07));
  border: 1px solid rgba(20,184,166,.3); border-radius: 14px; flex-wrap: wrap; }
.ph-banner-icon { width: 48px; height: 48px; border-radius: 12px;
  background: linear-gradient(135deg,#14b8a6,#0891b2); display: flex; align-items: center;
  justify-content: center; color: #fff; flex-shrink: 0; box-shadow: 0 4px 14px rgba(20,184,166,.4); }
.ph-banner-text { flex: 1; min-width: 160px; }
.ph-banner-title { font-size: 17px; font-weight: 700; margin-bottom: 3px; }
.ph-banner-desc { font-size: 12px; opacity: .7; }
.ph-banner-controls { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.ph-control-group { display: flex; flex-direction: column; gap: 2px; }
.ph-ctrl-label { font-size: 9px; opacity: .5; text-transform: uppercase; letter-spacing: .04em; }
.ph-select { background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 7px; padding: 6px 10px; color: var(--text-primary); font-size: 12px; outline: none; }

/* Progress */
.ph-progress { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 12px; padding: 16px; display: flex; flex-direction: column; gap: 10px; }
.ph-progress-bar-wrap { background: var(--bg-tertiary); border-radius: 4px; height: 6px; overflow: hidden; }
.ph-progress-bar { height: 100%; background: linear-gradient(90deg,#14b8a6,#0891b2); border-radius: 4px; transition: width .5s ease; }
.ph-progress-text { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--text-muted); }
.ph-spinner { width: 13px; height: 13px; border: 2px solid rgba(255,255,255,.15); border-top-color: #14b8a6; border-radius: 50%; animation: spin .8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

/* Stats */
.ph-stats { display: grid; grid-template-columns: repeat(5,1fr); gap: 10px; }
.ph-stat { border-radius: 12px; padding: 14px; text-align: center; border: 1px solid transparent; position: relative; }
.st-red    { background: rgba(239,68,68,.1);  border-color: rgba(239,68,68,.25); }
.st-orange { background: rgba(249,115,22,.1); border-color: rgba(249,115,22,.25); }
.st-green  { background: rgba(34,197,94,.1);  border-color: rgba(34,197,94,.25); }
.st-blue   { background: rgba(59,130,246,.1); border-color: rgba(59,130,246,.25); }
.st-purple { background: rgba(124,58,237,.1); border-color: rgba(124,58,237,.25); }
.st-gray   { background: var(--bg-secondary); border-color: var(--border); }
.ph-stat-icon { position: absolute; top: 10px; right: 10px; opacity: .3; }
.ph-stat-val { font-size: 20px; font-weight: 700; margin-bottom: 3px; }
.ph-stat-lbl { font-size: 10px; opacity: .5; text-transform: uppercase; }

/* Charts */
.ph-chart-wrap { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 12px; overflow: hidden; }
.ph-chart-header { display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; border-bottom: 1px solid var(--border); background: var(--bg-tertiary); }
.ph-chart-title { display: flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 600; opacity: .8; }
.ph-chart-avg { font-size: 11px; opacity: .45; }
.ph-chart { position: relative; height: 140px; padding: 8px 8px 4px; background: var(--bg-secondary); }
.ph-grid { position: absolute; inset: 0; pointer-events: none; }
.ph-grid-line { position: absolute; left: 0; right: 0; border-top: 1px dashed rgba(255,255,255,.06); }
.ph-grid-lbl { font-size: 9px; opacity: .35; margin-left: 4px; }
.ph-bars { display: flex; align-items: flex-end; height: 100%; gap: 1px; }
.ph-bar-col { flex: 1; height: 100%; display: flex; align-items: flex-end; min-width: 2px; cursor: default; }
.ph-bar { width: 100%; border-radius: 2px 2px 0 0; min-height: 2px; }
.ph-xaxis { display: flex; justify-content: space-between; padding: 4px 8px 8px; }
.ph-xlabel { font-size: 9px; opacity: .35; }

/* Raw details */
.ph-raw-details { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 12px; overflow: hidden; }
.ph-raw-summary { display: flex; align-items: center; gap: 8px; padding: 10px 14px; font-size: 12px; font-weight: 600; opacity: .7; cursor: pointer; list-style: none; background: var(--bg-tertiary); }
.ph-raw-summary::-webkit-details-marker { display: none; }
.ph-raw-table-wrap { max-height: 220px; overflow-y: auto; }
.ph-table { width: 100%; border-collapse: collapse; font-size: 11px; }
.ph-table th { padding: 6px 10px; text-align: left; color: var(--text-muted); font-size: 10px; text-transform: uppercase; letter-spacing: .04em; border-bottom: 1px solid var(--border); background: var(--bg-secondary); position: sticky; top: 0; }
.ph-table td { padding: 6px 10px; border-bottom: 1px solid var(--border); font-family: 'JetBrains Mono',monospace; }
.ph-table tbody tr:hover td { background: var(--bg-tertiary); }

/* Top processes */
.ph-top-section { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 14px; overflow: hidden; }
.ph-top-header { display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; border-bottom: 1px solid var(--border); background: var(--bg-tertiary); }
.ph-top-title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 600; opacity: .8; }
.ph-top-loading { display: flex; align-items: center; gap: 8px; padding: 16px; font-size: 12px; color: var(--text-muted); }
.ph-top-list { display: flex; flex-direction: column; }
.ph-top-proc { display: flex; align-items: center; gap: 12px; padding: 10px 16px; border-bottom: 1px solid var(--border); }
.ph-top-proc:last-child { border-bottom: none; }
.ph-top-proc:hover { background: var(--bg-tertiary); }
.ph-proc-rank { font-size: 12px; font-weight: 700; min-width: 28px; opacity: .4; }
.rank-top { color: #f59e0b; opacity: 1; }
.ph-proc-info { min-width: 140px; }
.ph-proc-name { font-size: 12px; font-weight: 500; }
.ph-proc-pid { font-size: 10px; opacity: .4; font-family: 'JetBrains Mono',monospace; }
.ph-proc-bars { flex: 1; display: flex; align-items: center; gap: 8px; }
.ph-proc-bar-wrap { flex: 1; height: 5px; background: var(--bg-tertiary); border-radius: 3px; overflow: hidden; }
.ph-proc-bar { height: 100%; border-radius: 3px; transition: width .3s; }
.cpu-bar { background: linear-gradient(90deg,#14b8a6,#0891b2); }
.ph-proc-cpu { font-size: 12px; font-weight: 600; min-width: 50px; text-align: right; }
.ph-proc-ram { font-size: 11px; opacity: .5; min-width: 55px; text-align: right; }

/* Buttons */
.ph-btn { display: inline-flex; align-items: center; gap: 5px; padding: 7px 13px; border-radius: 8px;
  border: 1px solid var(--border); background: var(--bg-secondary); color: var(--text-secondary);
  font-size: 12px; cursor: pointer; transition: all 150ms; font-family: inherit; }
.ph-btn:disabled { opacity: .4; cursor: not-allowed; }
.ph-btn-primary { background: rgba(20,184,166,.15); color: #2dd4bf; border-color: rgba(20,184,166,.3); }
.ph-btn-primary:hover:not(:disabled) { background: rgba(20,184,166,.25); }
</style>
