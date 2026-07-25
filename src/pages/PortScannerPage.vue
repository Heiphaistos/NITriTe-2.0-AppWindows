<script setup lang="ts">
defineOptions({ name: "PortScannerPage" });
import { ref, computed, onUnmounted } from "vue";
import { csvCell } from "@/composables/useExportData";
import { invoke } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import { useNotificationStore } from "@/stores/notifications";
import { Network, Search, Download, Play, Square } from "lucide-vue-next";

const notify = useNotificationStore();

interface OpenPort { protocol: string; local_address: string; local_port: number; remote_address: string; state: string; pid: number; process_name: string; }

const ports       = ref<OpenPort[]>([]);
const prevPorts   = ref<Set<string>>(new Set());
const newPorts    = ref<Set<string>>(new Set());
const loading     = ref(false);
const autoRefresh = ref(false);
const countdown   = ref(5);
const searchQuery = ref("");
const PROTO_OPTS = ['ALL','TCP','UDP'] as const;
const filterProto = ref<"ALL"|"TCP"|"UDP">("ALL");
const filterState = ref("ALL");
let   timer: ReturnType<typeof setInterval> | null = null;
let   cdTimer: ReturnType<typeof setInterval> | null = null;

const states = computed(() => {
  const s = new Set(ports.value.map(p => p.state).filter(Boolean));
  return ["ALL", ...Array.from(s)];
});

const filtered = computed(() => ports.value.filter(p => {
  if (filterProto.value !== "ALL" && p.protocol !== filterProto.value) return false;
  if (filterState.value !== "ALL" && p.state !== filterState.value) return false;
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase();
    return p.process_name.toLowerCase().includes(q) || String(p.local_port).includes(q) || p.local_address.toLowerCase().includes(q);
  }
  return true;
}));

const stats = computed(() => ({
  total:       ports.value.length,
  listening:   ports.value.filter(p => p.state === "LISTENING").length,
  established: ports.value.filter(p => p.state === "ESTABLISHED").length,
  newCount:    newPorts.value.size,
}));

function portKey(p: OpenPort) { return `${p.protocol}-${p.local_port}-${p.pid}`; }

function stateVariant(state: string): "success"|"warning"|"danger"|"neutral" {
  if (state === "LISTENING")  return "success";
  if (state === "ESTABLISHED") return "warning";
  if (state === "TIME_WAIT" || state === "CLOSE_WAIT") return "danger";
  return "neutral";
}

function knownService(port: number): string {
  const map: Record<number, string> = {
    80:"HTTP", 443:"HTTPS", 22:"SSH", 21:"FTP", 25:"SMTP", 53:"DNS", 3389:"RDP",
    3306:"MySQL", 5432:"PostgreSQL", 27017:"MongoDB", 6379:"Redis", 8080:"HTTP-Alt",
    8443:"HTTPS-Alt", 1433:"MSSQL", 5985:"WinRM", 445:"SMB", 135:"RPC", 139:"NetBIOS",
    5900:"VNC", 3000:"Dev-HTTP", 4200:"Angular", 5173:"Vite", 9229:"Node-Debug",
    5000:"Flask", 8000:"Django", 27015:"Steam", 25565:"Minecraft",
  };
  return map[port] || "";
}

async function scan(silent = false) {
  if (!silent) loading.value = true;
  try {
    const fresh = await invoke<OpenPort[]>("get_local_ports");
    // Detect new ports
    const freshKeys = new Set(fresh.map(portKey));
    if (prevPorts.value.size > 0) {
      newPorts.value = new Set([...freshKeys].filter(k => !prevPorts.value.has(k)));
      if (newPorts.value.size > 0 && silent)
        notify.info("Nouveaux ports", `${newPorts.value.size} nouveau(x) port(s) détecté(s)`);
    }
    prevPorts.value = freshKeys;
    ports.value = fresh;
  } catch (e: unknown) {
    if (!silent) notify.error("Erreur scan", String(e));
  } finally {
    if (!silent) loading.value = false;
  }
}

function toggleAutoRefresh() {
  if (autoRefresh.value) {
    stopAuto();
  } else {
    autoRefresh.value = true;
    countdown.value = 5;
    scan(true);
    timer = setInterval(() => scan(true), 5000);
    cdTimer = setInterval(() => {
      countdown.value = countdown.value <= 1 ? 5 : countdown.value - 1;
    }, 1000);
  }
}

function stopAuto() {
  autoRefresh.value = false;
  if (timer)   { clearInterval(timer);   timer = null; }
  if (cdTimer) { clearInterval(cdTimer); cdTimer = null; }
  newPorts.value.clear();
}

function exportCsv() {
  const rows = ["Protocole,Port,Service,Adresse locale,Adresse distante,État,PID,Processus"];
  for (const p of ports.value)
    // csvCell : échappe virgules/guillemets et neutralise l'injection de formule
    // (un process_name commençant par =,+,-,@ pourrait s'exécuter dans Excel).
    rows.push([p.protocol, p.local_port, knownService(p.local_port), p.local_address, p.remote_address || "", p.state, p.pid, p.process_name].map(csvCell).join(","));
  const blob = new Blob([rows.join("\n")], { type: "text/csv" });
  const a = document.createElement("a"); a.href = URL.createObjectURL(blob);
  a.download = `ports_${new Date().toISOString().slice(0,10)}.csv`; a.click();
}

onUnmounted(stopAuto);
</script>

<template>
  <div class="port-page">
    <div class="page-header">
      <div>
        <h1>Scanner de Ports Local</h1>
        <p class="page-subtitle">Ports ouverts sur ce système avec processus associés</p>
      </div>
      <div style="display:flex;gap:8px;align-items:center">
        <NButton v-if="ports.length" variant="ghost" size="sm" @click="exportCsv" title="Exporter CSV">
          <Download :size="14" />
        </NButton>
        <NButton :variant="autoRefresh ? 'warning' : 'secondary'" size="sm" @click="toggleAutoRefresh">
          <component :is="autoRefresh ? Square : Play" :size="14" />
          {{ autoRefresh ? `Auto (${countdown}s)` : 'Auto' }}
        </NButton>
        <NButton variant="primary" :loading="loading" @click="scan()">
          <Network :size="14" /> Scanner
        </NButton>
      </div>
    </div>

    <!-- Stats -->
    <div v-if="ports.length" class="stats-row">
      <div class="stat-pill"><span class="stat-num">{{ stats.total }}</span> Total</div>
      <div class="stat-pill ok"><span class="stat-num">{{ stats.listening }}</span> LISTENING</div>
      <div class="stat-pill warn"><span class="stat-num">{{ stats.established }}</span> ESTABLISHED</div>
      <div v-if="stats.newCount" class="stat-pill new"><span class="stat-num">{{ stats.newCount }}</span> Nouveaux</div>
    </div>

    <NCard>
      <template #header>
        <div style="display:flex;align-items:center;gap:8px;width:100%;flex-wrap:wrap">
          <Network :size="16" />
          <span>Ports ouverts</span>
          <NBadge v-if="ports.length" variant="neutral">{{ filtered.length }} / {{ ports.length }}</NBadge>
          <div style="margin-left:auto;display:flex;gap:8px;flex-wrap:wrap">
            <div class="filter-group">
              <button v-for="p in PROTO_OPTS" :key="p" class="filter-btn" :class="{active: filterProto===p}" @click="filterProto = p">{{ p }}</button>
            </div>
            <select class="state-select" v-model="filterState">
              <option v-for="s in states" :key="s" :value="s">{{ s === 'ALL' ? 'Tous états' : s }}</option>
            </select>
            <div class="search-wrap">
              <Search :size="13" class="search-icon" />
              <input v-model="searchQuery" class="search-input" placeholder="Process, port..." />
            </div>
          </div>
        </div>
      </template>

      <div v-if="loading" style="display:flex;justify-content:center;padding:40px"><NSpinner :size="24" /></div>

      <div v-else-if="ports.length === 0" class="empty-state">
        <Network :size="32" style="color:var(--text-muted);opacity:.2" />
        <p>Cliquez sur "Scanner" pour analyser les ports</p>
      </div>

      <div v-else class="table-wrap">
        <table class="ports-table">
          <thead>
            <tr>
              <th>Proto</th><th>Port</th><th>Service</th><th>Adresse locale</th>
              <th>Adresse distante</th><th>État</th><th>PID</th><th>Processus</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="p in filtered" :key="portKey(p)" :class="{ 'row-new': newPorts.has(portKey(p)) }">
              <td><NBadge :variant="p.protocol === 'TCP' ? 'accent' : 'warning'" size="sm">{{ p.protocol }}</NBadge></td>
              <td class="port-num">{{ p.local_port }}</td>
              <td><span v-if="knownService(p.local_port)" class="service-tag">{{ knownService(p.local_port) }}</span></td>
              <td class="mono">{{ p.local_address }}</td>
              <td class="mono">{{ p.remote_address || '—' }}</td>
              <td><NBadge :variant="stateVariant(p.state)" size="sm">{{ p.state || 'N/A' }}</NBadge></td>
              <td class="mono pid">{{ p.pid || '—' }}</td>
              <td class="proc-name">
                <span v-if="newPorts.has(portKey(p))" class="new-badge">NEW</span>
                {{ p.process_name || '—' }}
              </td>
            </tr>
            <tr v-if="!filtered.length">
              <td colspan="8" style="text-align:center;color:var(--text-muted);padding:24px;font-size:13px">Aucun port ne correspond aux filtres</td>
            </tr>
          </tbody>
        </table>
      </div>
    </NCard>
  </div>
</template>

<style scoped>
.port-page { display:flex; flex-direction:column; gap:16px; }
.page-header { display:flex; justify-content:space-between; align-items:flex-start; flex-wrap:wrap; gap:12px; }
.page-header h1 { font-size:24px; font-weight:700; }
.page-subtitle { color:var(--text-muted); font-size:13px; margin-top:2px; }

.stats-row { display:flex; gap:8px; flex-wrap:wrap; }
.stat-pill { display:flex; align-items:center; gap:6px; padding:6px 14px; background:var(--bg-secondary); border:1px solid var(--border); border-radius:99px; font-size:12px; color:var(--text-secondary); }
.stat-pill.ok   { border-color:rgba(34,197,94,.3);  color:var(--success); }
.stat-pill.warn { border-color:rgba(234,179,8,.3);  color:var(--warning); }
.stat-pill.new  { border-color:rgba(249,115,22,.4); color:var(--accent-primary); }
.stat-num { font-weight:700; font-size:14px; }

.filter-group { display:flex; gap:2px; }
.filter-btn { padding:4px 10px; border:1px solid var(--border); background:var(--bg-tertiary); color:var(--text-secondary); font-size:12px; cursor:pointer; font-family:inherit; }
.filter-btn:first-child { border-radius: var(--radius-sm) 0 0 var(--radius-sm); }
.filter-btn:last-child  { border-radius: 0 var(--radius-sm) var(--radius-sm) 0; }
.filter-btn.active { background:var(--accent-muted); border-color:var(--accent-primary); color:var(--accent-primary); }

.state-select { padding:4px 8px; border:1px solid var(--border); border-radius:var(--radius-sm); background:var(--bg-tertiary); color:var(--text-secondary); font-size:12px; cursor:pointer; font-family:inherit; }

.search-wrap { position:relative; display:flex; align-items:center; }
.search-icon { position:absolute; left:8px; color:var(--text-muted); }
.search-input { padding:4px 8px 4px 26px; border:1px solid var(--border); border-radius:var(--radius-sm); background:var(--bg-tertiary); color:var(--text-primary); font-size:12px; outline:none; }
.search-input:focus { border-color:var(--accent-primary); }

.table-wrap { overflow-x:auto; }
.ports-table { width:100%; border-collapse:collapse; font-size:12px; }
.ports-table th { padding:8px 12px; text-align:left; color:var(--text-muted); font-size:10px; font-weight:700; text-transform:uppercase; letter-spacing:0.06em; border-bottom:1px solid var(--border); white-space:nowrap; }
.ports-table td { padding:6px 12px; border-bottom:1px solid var(--border); }
.ports-table tr:hover td { background:var(--bg-tertiary); }
.row-new td { background:rgba(249,115,22,0.06) !important; }
.port-num { font-family:monospace; font-weight:700; color:var(--accent-primary); }
.mono { font-family:monospace; color:var(--text-secondary); font-size:11px; }
.pid  { color:var(--text-muted); }
.proc-name { font-weight:500; color:var(--text-primary); display:flex; align-items:center; gap:6px; }
.service-tag { font-size:10px; background:var(--accent-muted); color:var(--accent-primary); border-radius:4px; padding:1px 6px; }
.new-badge { font-size:9px; font-weight:800; background:var(--accent-primary); color:#fff; border-radius:4px; padding:1px 5px; letter-spacing:.04em; }

.empty-state { display:flex; flex-direction:column; align-items:center; gap:8px; padding:48px; color:var(--text-muted); font-size:13px; }
</style>
