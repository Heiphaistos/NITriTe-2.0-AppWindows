<template>
  <div class="bench-root">
    <!-- Banner -->
    <div class="bench-banner">
      <div class="bench-banner-icon"><Gauge :size="28" /></div>
      <div class="bench-banner-text">
        <div class="bench-banner-title">Benchmark Système</div>
        <div class="bench-banner-desc">Testez les performances CPU, RAM et stockage de votre machine</div>
      </div>
      <div class="bench-banner-actions">
        <button class="bench-btn bench-btn-glow" :disabled="anyRunning" @click="runAll">
          <Play :size="14" /> Tout tester
        </button>
        <button class="bench-btn" :disabled="anyRunning" @click="reset">Réinitialiser</button>
      </div>
    </div>

    <!-- Cards grid -->
    <div class="bench-grid">
      <!-- CPU -->
      <div class="bench-card bench-card-cpu">
        <div class="bench-card-header">
          <div class="bench-card-icon cpu-icon"><Cpu :size="18" /></div>
          <div>
            <div class="bench-card-title">CPU Single-thread</div>
            <div class="bench-card-sub">Calcul flottant sqrt/sin</div>
          </div>
          <span class="bench-tag">{{ cpu.unit }}</span>
        </div>
        <div v-if="cpu.loading" class="bench-running">
          <div class="bench-spinner cpu-spin" /> Test CPU en cours...
        </div>
        <div v-else-if="cpu.score !== null" class="bench-result">
          <div class="bench-score cpu-score">{{ cpu.score.toFixed(1) }}</div>
          <div class="bench-detail">{{ cpu.details }}</div>
          <div class="bench-bar-row">
            <div class="bench-bar-bg">
              <div class="bench-bar-fill" :style="{ width: barPct(cpu.score, 200) + '%', background: scoreGrad(cpu.score, 200) }" />
            </div>
            <span class="bench-rating" :style="{ color: scoreColor(cpu.score, 200) }">{{ rating(cpu.score, 200) }}</span>
          </div>
          <div class="bench-time">{{ cpu.duration_ms }}ms</div>
        </div>
        <div v-else class="bench-idle"><Activity :size="32" style="opacity:.15" /><span>En attente</span></div>
        <button class="bench-btn bench-btn-accent" :disabled="anyRunning" @click="runCpu">Lancer le test</button>
      </div>

      <!-- RAM -->
      <div class="bench-card bench-card-ram">
        <div class="bench-card-header">
          <div class="bench-card-icon ram-icon"><MemoryStick :size="18" /></div>
          <div>
            <div class="bench-card-title">RAM Write</div>
            <div class="bench-card-sub">Remplissage 64MB</div>
          </div>
          <span class="bench-tag">{{ ram.unit }}</span>
        </div>
        <div v-if="ram.loading" class="bench-running">
          <div class="bench-spinner ram-spin" /> Test RAM en cours...
        </div>
        <div v-else-if="ram.score !== null" class="bench-result">
          <div class="bench-score ram-score">{{ ram.score.toFixed(2) }}</div>
          <div class="bench-detail">{{ ram.details }}</div>
          <div class="bench-bar-row">
            <div class="bench-bar-bg">
              <div class="bench-bar-fill" :style="{ width: barPct(ram.score, 50) + '%', background: scoreGrad(ram.score, 50) }" />
            </div>
            <span class="bench-rating" :style="{ color: scoreColor(ram.score, 50) }">{{ rating(ram.score, 50) }}</span>
          </div>
          <div class="bench-time">{{ ram.duration_ms }}ms</div>
        </div>
        <div v-else class="bench-idle"><Activity :size="32" style="opacity:.15" /><span>En attente</span></div>
        <button class="bench-btn bench-btn-accent" :disabled="anyRunning" @click="runRam">Lancer le test</button>
      </div>

      <!-- Disk -->
      <div class="bench-card bench-card-disk" style="grid-column: 1 / -1">
        <div class="bench-card-header">
          <div class="bench-card-icon disk-icon"><HardDrive :size="18" /></div>
          <div>
            <div class="bench-card-title">Stockage</div>
            <div class="bench-card-sub">Séquentiel 128MB + Aléatoire 4K</div>
          </div>
          <div style="display:flex;align-items:center;gap:8px;margin-left:auto">
            <span style="font-size:12px;opacity:.6">Lecteur :</span>
            <input v-model="drive" class="bench-input" placeholder="C:" />
          </div>
        </div>
        <div v-if="disk.loading" class="bench-running">
          <div class="bench-spinner disk-spin" /> Test disque en cours (peut prendre 30s)...
        </div>
        <div v-else-if="disk.results.length > 0" class="bench-disk-results">
          <div v-for="r in disk.results" :key="r.name" class="bench-disk-row">
            <div class="bench-disk-label">
              <span class="bench-disk-name">{{ r.name }}</span>
              <code class="bench-disk-score">{{ r.score?.toFixed(0) ?? '—' }} {{ r.unit }}</code>
            </div>
            <div class="bench-bar-bg" style="flex:1">
              <div class="bench-bar-fill" :style="{ width: diskPct(r) + '%', background: diskGrad(r) }" />
            </div>
            <span class="bench-rating" :style="{ color: diskColor(r) }">{{ diskRating(r) }}</span>
            <span style="font-size:11px;opacity:.5">{{ r.duration_ms }}ms</span>
          </div>
        </div>
        <div v-else class="bench-idle"><HardDrive :size="32" style="opacity:.15" /><span>En attente</span></div>
        <button class="bench-btn bench-btn-accent" :disabled="anyRunning" @click="runDisk">Lancer le test disque</button>
      </div>
    </div>

    <!-- Summary table -->
    <div v-if="hasResults" class="bench-summary-box">
      <div class="bench-summary-title"><BarChart3 :size="14" /> Résumé des performances</div>
      <table class="bench-summary-table">
        <thead><tr><th>Test</th><th>Score</th><th>Unité</th><th>Durée</th><th>Note</th></tr></thead>
        <tbody>
          <tr v-if="cpu.score !== null">
            <td>CPU Single-thread</td>
            <td class="score-val">{{ cpu.score.toFixed(1) }}</td>
            <td>Kops/s</td>
            <td>{{ cpu.duration_ms }}ms</td>
            <td><span class="bench-grade" :style="{ background: scoreGrad(cpu.score, 200) }">{{ rating(cpu.score, 200) }}</span></td>
          </tr>
          <tr v-if="ram.score !== null">
            <td>RAM Write</td>
            <td class="score-val">{{ ram.score.toFixed(2) }}</td>
            <td>GB/s</td>
            <td>{{ ram.duration_ms }}ms</td>
            <td><span class="bench-grade" :style="{ background: scoreGrad(ram.score, 50) }">{{ rating(ram.score, 50) }}</span></td>
          </tr>
          <template v-for="r in disk.results" :key="r.name">
            <tr>
              <td>Disque — {{ r.name }}</td>
              <td class="score-val">{{ r.score?.toFixed(0) ?? '—' }}</td>
              <td>{{ r.unit }}</td>
              <td>{{ r.duration_ms }}ms</td>
              <td><span class="bench-grade" :style="{ background: diskGrad(r) }">{{ diskRating(r) }}</span></td>
            </tr>
          </template>
        </tbody>
      </table>
    </div>

    <!-- Historique des benchmarks -->
    <div class="bench-history">
      <div class="bench-history-header" @click="showHistory = !showHistory" style="cursor:pointer;display:flex;align-items:center;gap:8px;padding:12px 16px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:10px">
        <span style="font-size:13px;font-weight:600">Historique des tests ({{ benchHistory.length }})</span>
        <span style="margin-left:auto;font-size:12px;color:var(--text-muted)">{{ showHistory ? '▲' : '▼' }}</span>
        <button v-if="benchHistory.length > 0" @click.stop="clearHistory" style="font-size:11px;padding:3px 8px;border:1px solid var(--border);border-radius:4px;background:none;color:var(--text-muted);cursor:pointer">Effacer</button>
      </div>
      <div v-if="showHistory && benchHistory.length > 0" style="border:1px solid var(--border);border-top:none;border-radius:0 0 10px 10px;overflow:hidden">
        <table style="width:100%;border-collapse:collapse;font-size:11px">
          <thead>
            <tr style="background:var(--bg-tertiary)">
              <th style="padding:6px 10px;text-align:left;color:var(--text-muted)">Date</th>
              <th style="padding:6px 10px;text-align:right;color:var(--text-muted)">CPU (Mops/s)</th>
              <th style="padding:6px 10px;text-align:right;color:var(--text-muted)">RAM (GB/s)</th>
              <th style="padding:6px 10px;text-align:right;color:var(--text-muted)">Lect. Séq.</th>
              <th style="padding:6px 10px;text-align:right;color:var(--text-muted)">Écrit. Séq.</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(h, i) in [...benchHistory].reverse()" :key="i" style="border-bottom:1px solid var(--border)">
              <td style="padding:5px 10px;color:var(--text-secondary)">{{ h.date }}</td>
              <td style="padding:5px 10px;text-align:right;font-family:monospace">{{ h.cpu?.toFixed(1) ?? '—' }}</td>
              <td style="padding:5px 10px;text-align:right;font-family:monospace">{{ h.ram?.toFixed(2) ?? '—' }}</td>
              <td style="padding:5px 10px;text-align:right;font-family:monospace">{{ h.disk_seq_r?.toFixed(0) ?? '—' }} MB/s</td>
              <td style="padding:5px 10px;text-align:right;font-family:monospace">{{ h.disk_seq_w?.toFixed(0) ?? '—' }} MB/s</td>
            </tr>
          </tbody>
        </table>
      </div>
      <div v-else-if="showHistory && !benchHistory.length" style="padding:16px;text-align:center;font-size:12px;color:var(--text-muted);border:1px solid var(--border);border-top:none;border-radius:0 0 10px 10px">
        Aucun test enregistré — lancez "Tout tester" pour commencer
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from "@/utils/invoke";
import { Gauge, Cpu, MemoryStick, HardDrive, Play, Activity, BarChart3 } from 'lucide-vue-next'

interface BenchResult { name: string; score: number; unit: string; duration_ms: number; details: string }
interface BenchState { loading: boolean; score: number | null; unit: string; duration_ms: number; details: string }

const drive = ref('C:')
const cpu = ref<BenchState>({ loading: false, score: null, unit: 'Kops/s', duration_ms: 0, details: '' })
const ram = ref<BenchState>({ loading: false, score: null, unit: 'GB/s', duration_ms: 0, details: '' })
const disk = ref<{ loading: boolean; results: BenchResult[] }>({ loading: false, results: [] })

const anyRunning = computed(() => cpu.value.loading || ram.value.loading || disk.value.loading)
const hasResults = computed(() => cpu.value.score !== null || ram.value.score !== null || disk.value.results.length > 0)

const BENCH_STORAGE_KEY = 'nitrite-bench-history';

interface BenchRecord {
  date: string;
  cpu: number | null;
  ram: number | null;
  disk_seq_r: number | null;
  disk_seq_w: number | null;
}

const benchHistory = ref<BenchRecord[]>([]);
const showHistory = ref(false);

function loadHistory() {
  try {
    const raw = localStorage.getItem(BENCH_STORAGE_KEY);
    benchHistory.value = raw ? JSON.parse(raw) : [];
  } catch { benchHistory.value = []; }
}

function saveHistory() {
  const diskResults = Array.isArray(disk.value.results) ? disk.value.results : [];
  const seqR = diskResults.find(r => r.name.toLowerCase().includes('séq') && r.name.toLowerCase().includes('lect'))?.score
    ?? diskResults.find(r => r.name.toLowerCase().includes('seq') && r.name.toLowerCase().includes('read'))?.score
    ?? diskResults[0]?.score ?? null;
  const seqW = diskResults.find(r => r.name.toLowerCase().includes('séq') && r.name.toLowerCase().includes('écrit'))?.score
    ?? diskResults.find(r => r.name.toLowerCase().includes('seq') && r.name.toLowerCase().includes('write'))?.score
    ?? diskResults[1]?.score ?? null;
  const record: BenchRecord = {
    date: new Date().toLocaleString('fr-FR'),
    cpu: cpu.value.score,
    ram: ram.value.score,
    disk_seq_r: seqR,
    disk_seq_w: seqW,
  };
  const history = [...benchHistory.value, record].slice(-20);
  benchHistory.value = history;
  localStorage.setItem(BENCH_STORAGE_KEY, JSON.stringify(history));
}

function clearHistory() {
  benchHistory.value = [];
  localStorage.removeItem(BENCH_STORAGE_KEY);
}

async function runCpu() {
  cpu.value.loading = true
  try { const r = await invoke<BenchResult>('run_cpu_bench'); cpu.value = { loading: false, ...r } }
  catch { cpu.value.loading = false }
}
async function runRam() {
  ram.value.loading = true
  try { const r = await invoke<BenchResult>('run_ram_bench'); ram.value = { loading: false, ...r } }
  catch { ram.value.loading = false }
}
async function runDisk() {
  disk.value = { loading: true, results: [] }
  try { disk.value.results = await invoke<BenchResult[]>('run_disk_bench', { drive: drive.value }) }
  catch {} finally { disk.value.loading = false }
}
async function runAll() {
  await Promise.all([runCpu(), runRam(), runDisk()]);
  saveHistory();
}
function reset() {
  cpu.value = { loading: false, score: null, unit: 'Kops/s', duration_ms: 0, details: '' }
  ram.value = { loading: false, score: null, unit: 'GB/s', duration_ms: 0, details: '' }
  disk.value = { loading: false, results: [] }
}

onMounted(() => {
  loadHistory();
})

function barPct(v: number, max: number) { return Math.min(v / max * 100, 100) }
function scoreColor(v: number, max: number) { const p = v/max; return p > .7 ? '#22c55e' : p > .4 ? '#f59e0b' : '#ef4444' }
function scoreGrad(v: number, max: number) { const p = v/max; return p > .7 ? 'linear-gradient(90deg,#16a34a,#22c55e)' : p > .4 ? 'linear-gradient(90deg,#d97706,#f59e0b)' : 'linear-gradient(90deg,#dc2626,#ef4444)' }
function rating(v: number, max: number) { const p = v/max; return p > .7 ? 'Excellent' : p > .4 ? 'Moyen' : 'Faible' }
function diskPct(r: BenchResult) { return r.unit === 'MB/s' ? Math.min(r.score/5000*100, 100) : Math.min(r.score/100000*100, 100) }
function diskGrad(r: BenchResult) { return scoreGrad(r.score, r.unit === 'MB/s' ? 5000 : 100000) }
function diskColor(r: BenchResult) { return scoreColor(r.score, r.unit === 'MB/s' ? 5000 : 100000) }
function diskRating(r: BenchResult) { return rating(r.score, r.unit === 'MB/s' ? 5000 : 100000) }
</script>

<style scoped>
.bench-root { display: flex; flex-direction: column; gap: 16px; }

/* Banner */
.bench-banner { display: flex; align-items: center; gap: 16px; padding: 20px 24px;
  background: linear-gradient(135deg, rgba(124,58,237,.2), rgba(59,130,246,.1));
  border: 1px solid rgba(124,58,237,.3); border-radius: 14px; }
.bench-banner-icon { width: 52px; height: 52px; border-radius: 12px;
  background: linear-gradient(135deg,#7c3aed,#3b82f6); display: flex; align-items: center;
  justify-content: center; color: #fff; flex-shrink: 0; box-shadow: 0 4px 16px rgba(124,58,237,.4); }
.bench-banner-text { flex: 1; }
.bench-banner-title { font-size: 18px; font-weight: 700; margin-bottom: 2px; }
.bench-banner-desc { font-size: 12px; opacity: .6; }
.bench-banner-actions { display: flex; gap: 8px; }

/* Grid */
.bench-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }

/* Card */
.bench-card { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 14px; padding: 18px; display: flex; flex-direction: column; gap: 14px; }
.bench-card-cpu  { border-top: 3px solid #7c3aed; }
.bench-card-ram  { border-top: 3px solid #3b82f6; }
.bench-card-disk { border-top: 3px solid #f59e0b; }
.bench-card-header { display: flex; align-items: center; gap: 12px; }
.bench-card-icon { width: 40px; height: 40px; border-radius: 10px; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.cpu-icon  { background: rgba(124,58,237,.15); color: #7c3aed; }
.ram-icon  { background: rgba(59,130,246,.15);  color: #3b82f6; }
.disk-icon { background: rgba(245,158,11,.15);  color: #f59e0b; }
.bench-card-title { font-size: 14px; font-weight: 600; }
.bench-card-sub { font-size: 11px; opacity: .5; margin-top: 1px; }
.bench-tag { margin-left: auto; font-size: 10px; opacity: .5; background: var(--bg-tertiary); padding: 2px 7px; border-radius: 5px; }

/* Results */
.bench-result { display: flex; flex-direction: column; gap: 8px; }
.bench-score { font-size: 40px; font-weight: 800; line-height: 1; }
.cpu-score  { background: linear-gradient(135deg,#7c3aed,#a78bfa); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; }
.ram-score  { background: linear-gradient(135deg,#3b82f6,#60a5fa); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; }
.bench-detail { font-size: 11px; opacity: .5; }
.bench-bar-row { display: flex; align-items: center; gap: 8px; }
.bench-bar-bg { flex: 1; height: 8px; background: var(--bg-tertiary); border-radius: 4px; overflow: hidden; }
.bench-bar-fill { height: 100%; border-radius: 4px; transition: width .6s ease; }
.bench-rating { font-size: 11px; font-weight: 600; min-width: 60px; text-align: right; }
.bench-time { font-size: 10px; opacity: .4; }

/* Disk results */
.bench-disk-results { display: flex; flex-direction: column; gap: 10px; }
.bench-disk-row { display: flex; align-items: center; gap: 10px; }
.bench-disk-label { display: flex; align-items: center; gap: 8px; min-width: 150px; }
.bench-disk-name { font-size: 12px; font-weight: 500; }
.bench-disk-score { font-family: 'JetBrains Mono', monospace; font-size: 11px; color: var(--text-muted); }

/* Idle */
.bench-idle { display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 16px; color: var(--text-muted); font-size: 12px; }

/* Running */
.bench-running { display: flex; align-items: center; gap: 10px; font-size: 12px; opacity: .7; padding: 8px 0; }
.bench-spinner { width: 16px; height: 16px; border: 2px solid rgba(255,255,255,.15); border-radius: 50%; animation: spin .8s linear infinite; flex-shrink: 0; }
.cpu-spin  { border-top-color: #7c3aed; }
.ram-spin  { border-top-color: #3b82f6; }
.disk-spin { border-top-color: #f59e0b; }
@keyframes spin { to { transform: rotate(360deg); } }

/* Input */
.bench-input { background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 6px; padding: 4px 8px; color: var(--text-primary); font-size: 12px; width: 70px; outline: none; }

/* Buttons */
.bench-btn { display: inline-flex; align-items: center; gap: 6px; padding: 8px 16px; border-radius: 8px;
  border: 1px solid var(--border); background: var(--bg-tertiary); color: var(--text-secondary);
  font-size: 12px; cursor: pointer; transition: all 150ms; font-family: inherit; }
.bench-btn:disabled { opacity: .4; cursor: not-allowed; }
.bench-btn:hover:not(:disabled) { color: var(--text-primary); }
.bench-btn-accent { background: rgba(124,58,237,.15); color: #a78bfa; border-color: rgba(124,58,237,.3); }
.bench-btn-accent:hover:not(:disabled) { background: rgba(124,58,237,.25); }
.bench-btn-glow { background: linear-gradient(135deg,#7c3aed,#3b82f6); color: #fff; border: none; box-shadow: 0 2px 12px rgba(124,58,237,.4); }
.bench-btn-glow:hover:not(:disabled) { opacity: .85; }

/* Summary */
.bench-summary-box { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 14px; padding: 16px; }
.bench-summary-title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 600; margin-bottom: 12px; opacity: .8; }
.bench-summary-table { width: 100%; border-collapse: collapse; font-size: 12px; }
.bench-summary-table th { padding: 6px 10px; text-align: left; font-size: 10px; text-transform: uppercase; color: var(--text-muted); border-bottom: 1px solid var(--border); }
.bench-summary-table td { padding: 7px 10px; border-bottom: 1px solid var(--border); }
.bench-summary-table tbody tr:last-child td { border-bottom: none; }
.score-val { font-weight: 700; font-family: 'JetBrains Mono', monospace; }
.bench-grade { padding: 2px 10px; border-radius: 5px; font-size: 10px; font-weight: 700; color: #fff; }
</style>
