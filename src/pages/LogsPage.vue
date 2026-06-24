<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from "vue";
import { invoke } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NSearchBar from "@/components/ui/NSearchBar.vue";
import NBadge from "@/components/ui/NBadge.vue";
import { useExportData } from "@/composables/useExportData";
import { logBuffer, logStats as sessionStats } from "@/utils/logger";
import type { LogEntry, LogLevel, LogSource } from "@/utils/logger";
import {
  ScrollText, RefreshCw, Trash2, ArrowDownToLine,
  Filter, Info, AlertTriangle, XCircle, Download,
  ChevronDown, ChevronRight, Zap, Bug, Database,
  FolderArchive, FileText,
} from "lucide-vue-next";

const { exportTXT } = useExportData();

// ── Onglets ───────────────────────────────────────────────────────────────────
type Tab = "session" | "file" | "windows";
const activeTab = ref<Tab>("session");

// ── Source logs session (in-memory) ──────────────────────────────────────────
const sessionLogs = computed(() => logBuffer.value);

// ── Logs fichier (Rust) ───────────────────────────────────────────────────────
interface FileLogStats { total: number; debug: number; info: number; warn: number; error: number; critical: number; file_size_kb: number; }
const fileLogs     = ref<LogEntry[]>([]);
const fileStats    = ref<FileLogStats | null>(null);
const fileLoading  = ref(false);
const fileArchives = ref<string[]>([]);

// ── Windows Events ────────────────────────────────────────────────────────────
interface WinEvent { id: number; level: string; source: string; timestamp: string; message: string; }
const winEvents       = ref<WinEvent[]>([]);
const winLogName      = ref("System");
const winLoading      = ref(false);

// ── Filtres communs ───────────────────────────────────────────────────────────
const search       = ref("");
const levelFilter  = ref<"ALL" | LogLevel>("ALL");
const sourceFilter = ref<"ALL" | LogSource>("ALL");
const startDate    = ref("");
const endDate      = ref("");

const LEVELS:  Array<"ALL" | LogLevel>  = ["ALL", "DEBUG", "INFO", "WARN", "ERROR", "CRITICAL"];
const SOURCES: Array<"ALL" | LogSource> = ["ALL", "VUE", "ROUTER", "TAURI", "UNCAUGHT", "PERF", "STORE", "SYSTEM", "UI"];

// ── UI state ──────────────────────────────────────────────────────────────────
const outputEl      = ref<HTMLDivElement | null>(null);
const autoScroll    = ref(true);
const liveMode      = ref(false);
const expandedIds   = ref<Set<string>>(new Set());
const followInterval = ref<ReturnType<typeof setInterval> | null>(null);

// ── Real-time Tauri event ─────────────────────────────────────────────────────
let unlistenLog: (() => void) | null = null;

async function startLiveListener() {
  if (unlistenLog) return;
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlistenLog = await listen<LogEntry>("log:entry", (ev) => {
      if (activeTab.value === "file") {
        fileLogs.value.push(ev.payload);
        if (fileLogs.value.length > 2000) fileLogs.value.splice(0, fileLogs.value.length - 2000);
        if (autoScroll.value) scrollBottom();
      }
    });
  } catch { /* dev — pas de Tauri */ }
}

function stopLiveListener() {
  if (unlistenLog) { unlistenLog(); unlistenLog = null; }
}

// ── Scroll ────────────────────────────────────────────────────────────────────
function scrollBottom() {
  nextTick(() => { if (outputEl.value) outputEl.value.scrollTop = outputEl.value.scrollHeight; });
}
watch(() => sessionLogs.value.length, () => { if (autoScroll.value && activeTab.value === "session") scrollBottom(); });

// ── Fonctions de chargement ───────────────────────────────────────────────────
async function loadFileLogs() {
  fileLoading.value = true;
  try {
    fileLogs.value  = await invoke<LogEntry[]>("get_recent_logs", { count: 1000 });
    fileStats.value = await invoke<FileLogStats>("get_log_stats");
    if (autoScroll.value) scrollBottom();
  } catch { fileLogs.value = []; } finally { fileLoading.value = false; }
}

async function loadArchives() {
  try { fileArchives.value = await invoke<string[]>("list_log_archives"); } catch { fileArchives.value = []; }
}

async function loadWinEvents() {
  winLoading.value = true;
  try { winEvents.value = await invoke<WinEvent[]>("get_event_logs", { logName: winLogName.value, count: 200 }); }
  catch { winEvents.value = []; } finally { winLoading.value = false; }
}

async function clearFileLogs() {
  if (!confirm("Archiver et vider le fichier de log ?")) return;
  try { await invoke("clear_logs"); await loadFileLogs(); } catch { /* ignore */ }
}

async function openLogFile() {
  try {
    const path = await invoke<string>("get_log_file_path");
    await invoke("open_path", { path });
  } catch { /* ignore */ }
}

// ── Follow mode fichier ───────────────────────────────────────────────────────
function toggleLive() {
  liveMode.value = !liveMode.value;
  if (liveMode.value) {
    autoScroll.value = true;
    if (activeTab.value === "file") {
      followInterval.value = setInterval(loadFileLogs, 3000);
    }
  } else {
    if (followInterval.value) { clearInterval(followInterval.value); followInterval.value = null; }
  }
}

// ── Expand/collapse détails ───────────────────────────────────────────────────
function toggleExpand(id: string) {
  if (expandedIds.value.has(id)) expandedIds.value.delete(id);
  else expandedIds.value.add(id);
}

// ── Filtrage ──────────────────────────────────────────────────────────────────
function applyFilters(entries: LogEntry[]): LogEntry[] {
  let result = entries;
  if (levelFilter.value  !== "ALL") result = result.filter(e => e.level  === levelFilter.value);
  if (sourceFilter.value !== "ALL") result = result.filter(e => e.source === sourceFilter.value);
  if (startDate.value) {
    const from = new Date(startDate.value);
    result = result.filter(e => new Date(e.timestamp) >= from);
  }
  if (endDate.value) {
    const to = new Date(endDate.value); to.setHours(23,59,59,999);
    result = result.filter(e => new Date(e.timestamp) <= to);
  }
  const q = search.value.toLowerCase();
  if (q) result = result.filter(e => e.message.toLowerCase().includes(q) || (e.details ?? "").toLowerCase().includes(q) || e.source.toLowerCase().includes(q));
  return result;
}

const filteredSession = computed(() => applyFilters([...sessionLogs.value].reverse()));
const filteredFile    = computed(() => applyFilters([...fileLogs.value].reverse()));
const filteredWin     = computed(() => {
  let r = winEvents.value;
  const q = search.value.toLowerCase();
  if (q) r = r.filter(e => e.message.toLowerCase().includes(q) || e.source.toLowerCase().includes(q));
  return r;
});

// ── Styles ────────────────────────────────────────────────────────────────────
const LEVEL_VARIANT: Record<string, "info" | "warning" | "danger" | "success" | "default"> = {
  DEBUG: "default", INFO: "info", WARN: "warning", ERROR: "danger", CRITICAL: "danger",
};
function levelVariant(l: string) { return LEVEL_VARIANT[l] ?? "default"; }
function winVariant(l: string) { return l === "Error" || l === "Critical" ? "danger" : l === "Warning" ? "warning" : "info"; }

function formatTime(ts: string): string {
  try { return new Date(ts).toLocaleTimeString("fr-FR", { hour12: false }); } catch { return ts; }
}
function formatDateTime(ts: string): string {
  try { return new Date(ts).toLocaleString("fr-FR"); } catch { return ts; }
}

// ── Export ────────────────────────────────────────────────────────────────────
function exportSessionTXT() {
  const lines = filteredSession.value.map(e => `[${e.timestamp}] [${e.level}] [${e.source}] ${e.message}`);
  exportTXT(lines, `nitrite-session-${new Date().toISOString().slice(0,10)}`);
}
function exportSessionJSON() {
  const blob = new Blob([JSON.stringify(filteredSession.value, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a"); a.href = url;
  a.download = `nitrite-session-${new Date().toISOString().slice(0,10)}.json`; a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
function exportFileTXT() {
  const lines = filteredFile.value.map(e => `[${e.timestamp}] [${e.level}] [${e.source}] ${e.message}`);
  exportTXT(lines, `nitrite-logs-${new Date().toISOString().slice(0,10)}`);
}

// ── Lifecycle ─────────────────────────────────────────────────────────────────
onMounted(async () => {
  await startLiveListener();
  await loadFileLogs();
  await loadArchives();
});

onUnmounted(() => {
  stopLiveListener();
  if (followInterval.value) clearInterval(followInterval.value);
});
</script>

<template>
  <div class="logs-page">

    <!-- ── Header ─────────────────────────────────────────────────────────── -->
    <div class="page-header">
      <div>
        <h1>Logs NiTriTe</h1>
        <p class="page-subtitle">Observabilité complète — session + fichier + Windows</p>
      </div>
      <div class="header-actions">
        <NButton :variant="liveMode ? 'primary' : 'secondary'" size="sm" @click="toggleLive">
          <span v-if="liveMode" class="live-dot" />
          {{ liveMode ? 'LIVE' : 'Live' }}
        </NButton>
        <NButton variant="secondary" size="sm" @click="autoScroll = !autoScroll" :class="{ 'btn-active': autoScroll }">
          <ArrowDownToLine :size="14" /> Auto-scroll
        </NButton>
        <template v-if="activeTab === 'session'">
          <NButton variant="secondary" size="sm" @click="exportSessionTXT"><Download :size="14" /> TXT</NButton>
          <NButton variant="secondary" size="sm" @click="exportSessionJSON"><FileText :size="14" /> JSON</NButton>
        </template>
        <template v-if="activeTab === 'file'">
          <NButton variant="secondary" size="sm" @click="openLogFile"><FolderArchive :size="14" /> Ouvrir</NButton>
          <NButton variant="secondary" size="sm" @click="exportFileTXT"><Download :size="14" /> Export</NButton>
          <NButton variant="secondary" size="sm" @click="clearFileLogs"><Trash2 :size="14" /> Vider</NButton>
          <NButton variant="secondary" size="sm" :loading="fileLoading" @click="loadFileLogs"><RefreshCw :size="14" /></NButton>
        </template>
        <template v-if="activeTab === 'windows'">
          <NButton variant="secondary" size="sm" :loading="winLoading" @click="loadWinEvents"><RefreshCw :size="14" /></NButton>
        </template>
      </div>
    </div>

    <!-- ── Tabs ───────────────────────────────────────────────────────────── -->
    <div class="tab-row">
      <button class="tab-btn" :class="{ active: activeTab === 'session' }" @click="activeTab = 'session'">
        <Zap :size="13" /> Session ({{ sessionLogs.length }})
      </button>
      <button class="tab-btn" :class="{ active: activeTab === 'file' }" @click="activeTab = 'file'; loadFileLogs()">
        <Database :size="13" /> Fichier
      </button>
      <button class="tab-btn" :class="{ active: activeTab === 'windows' }" @click="activeTab = 'windows'; if (!winEvents.length) loadWinEvents()">
        <Bug :size="13" /> Windows Events
      </button>
    </div>

    <!-- ── Stats band ─────────────────────────────────────────────────────── -->
    <div class="stats-band">
      <template v-if="activeTab === 'session'">
        <div class="stat-item"><span class="stat-n">{{ sessionStats.debug }}</span><span class="stat-l">DEBUG</span></div>
        <div class="stat-sep" />
        <div class="stat-item"><span class="stat-n info-c">{{ sessionStats.info }}</span><span class="stat-l">INFO</span></div>
        <div class="stat-sep" />
        <div class="stat-item"><span class="stat-n warn-c">{{ sessionStats.warn }}</span><span class="stat-l">WARN</span></div>
        <div class="stat-sep" />
        <div class="stat-item"><span class="stat-n error-c">{{ sessionStats.error }}</span><span class="stat-l">ERROR</span></div>
        <div class="stat-sep" />
        <div class="stat-item"><span class="stat-n crit-c">{{ sessionStats.critical }}</span><span class="stat-l">CRITICAL</span></div>
        <div class="stat-sep" />
        <div class="stat-item stat-muted">{{ filteredSession.length }} affichés</div>
      </template>
      <template v-else-if="activeTab === 'file' && fileStats">
        <div class="stat-item"><span class="stat-n info-c">{{ fileStats.info }}</span><span class="stat-l">INFO</span></div>
        <div class="stat-sep" />
        <div class="stat-item"><span class="stat-n warn-c">{{ fileStats.warn }}</span><span class="stat-l">WARN</span></div>
        <div class="stat-sep" />
        <div class="stat-item"><span class="stat-n error-c">{{ fileStats.error + fileStats.critical }}</span><span class="stat-l">ERRORS</span></div>
        <div class="stat-sep" />
        <div class="stat-item stat-muted">{{ fileStats.total }} total · {{ fileStats.file_size_kb }} KB</div>
        <div class="stat-sep" />
        <div class="stat-item stat-muted">{{ fileArchives.length }} archive(s)</div>
      </template>
      <template v-else-if="activeTab === 'windows'">
        <div class="stat-item stat-muted">{{ winEvents.length }} événements chargés</div>
      </template>
    </div>

    <!-- ── Filtres ─────────────────────────────────────────────────────────── -->
    <div class="filters-row">
      <NSearchBar v-model="search" placeholder="Rechercher message, source, détails…" style="flex:1;min-width:200px" />

      <!-- Plage de dates -->
      <div class="date-range">
        <input type="date" v-model="startDate" class="date-input" title="Depuis" />
        <span class="date-sep">→</span>
        <input type="date" v-model="endDate" class="date-input" title="Jusqu'à" />
      </div>

      <!-- Windows log selector -->
      <select v-if="activeTab === 'windows'" v-model="winLogName" @change="loadWinEvents" class="log-select">
        <option value="System">System</option>
        <option value="Application">Application</option>
        <option value="Security">Security</option>
        <option value="Setup">Setup</option>
      </select>
    </div>

    <!-- Filtres niveau + source (session/file) -->
    <div v-if="activeTab !== 'windows'" class="filter-chips-row">
      <button
        v-for="lv in LEVELS" :key="lv"
        class="chip" :class="{ active: levelFilter === lv, [`chip-${lv.toLowerCase()}`]: true }"
        @click="levelFilter = lv"
      >
        <Filter v-if="lv === 'ALL'" :size="11" />
        {{ lv === 'ALL' ? 'Tous niveaux' : lv }}
      </button>
      <div class="chip-sep" />
      <button
        v-for="src in SOURCES" :key="src"
        class="chip chip-src" :class="{ active: sourceFilter === src }"
        @click="sourceFilter = src"
      >{{ src === 'ALL' ? 'Toutes sources' : src }}</button>
    </div>

    <!-- ── TAB: Session ───────────────────────────────────────────────────── -->
    <NCard v-if="activeTab === 'session'" padding="none">
      <template #header>
        <div class="card-hdr">
          <Zap :size="15" /> <span>Logs session ({{ filteredSession.length }})</span>
          <div v-if="liveMode" class="live-badge"><span class="live-dot" /> LIVE</div>
        </div>
      </template>
      <div class="logs-output" ref="outputEl">
        <div
          v-for="entry in filteredSession" :key="entry.id"
          class="log-row" :class="`log-${entry.level.toLowerCase()}`"
        >
          <button class="expand-btn" @click="toggleExpand(entry.id)">
            <component :is="expandedIds.has(entry.id) ? ChevronDown : ChevronRight" :size="12" />
          </button>
          <span class="log-time">{{ formatTime(entry.timestamp) }}</span>
          <NBadge :variant="levelVariant(entry.level)" class="log-badge">{{ entry.level }}</NBadge>
          <NBadge variant="default" class="log-src-badge">{{ entry.source }}</NBadge>
          <span class="log-msg">{{ entry.message }}</span>
          <span v-if="entry.duration_ms" class="log-dur">{{ entry.duration_ms }}ms</span>
        </div>

        <!-- Panneau détails expandé -->
        <template v-for="entry in filteredSession" :key="`d-${entry.id}`">
          <div v-if="expandedIds.has(entry.id) && entry.details" class="log-details">
            <div class="log-details-meta">
              {{ formatDateTime(entry.timestamp) }} · session {{ entry.session_id }}
              <span v-if="entry.duration_ms"> · {{ entry.duration_ms }}ms</span>
            </div>
            <pre class="log-details-body">{{ entry.details }}</pre>
          </div>
        </template>

        <div v-if="filteredSession.length === 0" class="logs-empty">
          <Info :size="24" style="opacity:.3;margin-bottom:8px" />
          <div>Aucun log en session</div>
          <div style="font-size:11px;margin-top:4px;opacity:.6">Les logs apparaissent ici au fur et à mesure des actions</div>
        </div>
      </div>
    </NCard>

    <!-- ── TAB: Fichier ───────────────────────────────────────────────────── -->
    <NCard v-if="activeTab === 'file'" padding="none">
      <template #header>
        <div class="card-hdr">
          <Database :size="15" /> <span>Logs fichier ({{ filteredFile.length }})</span>
          <div v-if="liveMode" class="live-badge"><span class="live-dot" /> LIVE</div>
        </div>
      </template>
      <div v-if="fileLoading" class="logs-empty">Chargement…</div>
      <div v-else class="logs-output" ref="outputEl">
        <div
          v-for="entry in filteredFile" :key="entry.id"
          class="log-row" :class="`log-${entry.level.toLowerCase()}`"
        >
          <button class="expand-btn" @click="toggleExpand(entry.id)">
            <component :is="expandedIds.has(entry.id) ? ChevronDown : ChevronRight" :size="12" />
          </button>
          <span class="log-time">{{ formatTime(entry.timestamp) }}</span>
          <NBadge :variant="levelVariant(entry.level)" class="log-badge">{{ entry.level }}</NBadge>
          <NBadge variant="default" class="log-src-badge">{{ entry.source }}</NBadge>
          <span class="log-msg">{{ entry.message }}</span>
          <span v-if="entry.duration_ms" class="log-dur">{{ entry.duration_ms }}ms</span>
        </div>
        <template v-for="entry in filteredFile" :key="`fd-${entry.id}`">
          <div v-if="expandedIds.has(entry.id) && entry.details" class="log-details">
            <div class="log-details-meta">{{ formatDateTime(entry.timestamp) }} · session {{ entry.session_id }}</div>
            <pre class="log-details-body">{{ entry.details }}</pre>
          </div>
        </template>
        <div v-if="filteredFile.length === 0" class="logs-empty">
          <Database :size="24" style="opacity:.3;margin-bottom:8px" />
          <div>Aucun log persisté</div>
        </div>
      </div>
    </NCard>

    <!-- ── TAB: Windows Events ────────────────────────────────────────────── -->
    <NCard v-if="activeTab === 'windows'" padding="none">
      <template #header>
        <div class="card-hdr">
          <Bug :size="15" /> <span>Windows Events — {{ winLogName }} ({{ filteredWin.length }})</span>
        </div>
      </template>
      <div v-if="winLoading" class="logs-empty">Chargement…</div>
      <div v-else class="logs-output">
        <div
          v-for="(ev, i) in filteredWin" :key="i"
          class="log-row"
          :class="ev.level === 'Error' || ev.level === 'Critical' ? 'log-error' : ev.level === 'Warning' ? 'log-warn' : 'log-info'"
        >
          <span class="log-time">{{ ev.timestamp.slice(0,19).replace("T"," ") }}</span>
          <NBadge :variant="winVariant(ev.level)" class="log-badge">{{ ev.level }}</NBadge>
          <span class="log-src-text">[{{ ev.source }}]</span>
          <span class="log-msg">{{ ev.message }}</span>
        </div>
        <div v-if="filteredWin.length === 0 && !winLoading" class="logs-empty">
          <Bug :size="24" style="opacity:.3;margin-bottom:8px" />
          <div>Aucun événement</div>
        </div>
      </div>
    </NCard>

  </div>
</template>

<style scoped>
.logs-page { display:flex; flex-direction:column; gap:16px; }

/* Header */
.page-header { display:flex; justify-content:space-between; align-items:flex-start; flex-wrap:wrap; gap:12px; }
.page-header h1 { font-size:24px; font-weight:700; }
.page-subtitle { color:var(--text-muted); font-size:13px; margin-top:2px; }
.header-actions { display:flex; gap:8px; flex-wrap:wrap; align-items:center; }
.btn-active { border-color:var(--accent-primary) !important; color:var(--accent-primary) !important; }

/* Tabs */
.tab-row { display:flex; gap:2px; border-bottom:1px solid var(--border); }
.tab-btn { display:flex; align-items:center; gap:6px; padding:8px 16px; border:none; border-bottom:2px solid transparent; background:none; color:var(--text-muted); font-family:inherit; font-size:13px; font-weight:500; cursor:pointer; transition:all 150ms ease; }
.tab-btn:hover { color:var(--text-primary); }
.tab-btn.active { color:var(--accent-primary); border-bottom-color:var(--accent-primary); }

/* Stats */
.stats-band { display:flex; align-items:center; gap:0; padding:8px 14px; background:var(--bg-secondary); border:1px solid var(--border); border-radius:var(--radius-md); font-size:12px; font-weight:600; flex-wrap:wrap; gap:0; }
.stat-item { display:flex; align-items:center; gap:5px; padding:0 12px; }
.stat-item:first-child { padding-left:0; }
.stat-n { font-size:15px; font-weight:700; }
.stat-l { color:var(--text-muted); font-size:10px; font-weight:500; text-transform:uppercase; letter-spacing:.05em; }
.stat-sep { width:1px; height:18px; background:var(--border); flex-shrink:0; }
.stat-muted { color:var(--text-muted); font-weight:400; }
.info-c { color:#60a5fa; }
.warn-c { color:var(--warning); }
.error-c { color:var(--danger); }
.crit-c { color:#f43f5e; }

/* Filtres */
.filters-row { display:flex; gap:10px; align-items:center; flex-wrap:wrap; }
.filter-chips-row { display:flex; gap:4px; flex-wrap:wrap; align-items:center; }
.chip { display:flex; align-items:center; gap:3px; padding:4px 10px; border:1px solid var(--border); border-radius:99px; background:var(--bg-secondary); color:var(--text-muted); font-family:inherit; font-size:11px; font-weight:500; cursor:pointer; transition:all 120ms ease; white-space:nowrap; }
.chip:hover { background:var(--bg-tertiary); color:var(--text-primary); }
.chip.active { background:var(--accent-muted); color:var(--accent-primary); border-color:var(--accent-primary); }
.chip-debug.active  { background:rgba(100,100,120,.2); color:#a1a1aa; border-color:#71717a; }
.chip-info.active   { background:rgba(96,165,250,.15); color:#60a5fa; border-color:#3b82f6; }
.chip-warn.active   { background:rgba(251,191,36,.12); color:var(--warning); border-color:var(--warning); }
.chip-error.active  { background:rgba(239,68,68,.12); color:var(--danger); border-color:var(--danger); }
.chip-critical.active { background:rgba(244,63,94,.15); color:#f43f5e; border-color:#f43f5e; }
.chip-sep { width:1px; height:16px; background:var(--border); margin:0 4px; flex-shrink:0; }
.chip-src { font-family:"JetBrains Mono",monospace; font-size:10px; }

.date-range { display:flex; align-items:center; gap:6px; background:var(--bg-tertiary); border:1px solid var(--border); border-radius:var(--radius-md); padding:5px 10px; }
.date-input { background:transparent; border:none; color:var(--text-primary); font-family:inherit; font-size:12px; outline:none; cursor:pointer; min-width:110px; }
.date-input::-webkit-calendar-picker-indicator { filter:invert(.6); cursor:pointer; }
.date-sep { font-size:11px; color:var(--text-muted); }
.log-select { background:var(--bg-tertiary); border:1px solid var(--border); border-radius:var(--radius-md); color:var(--text-primary); font-family:inherit; font-size:12px; padding:6px 8px; }

/* Card header */
.card-hdr { display:flex; align-items:center; gap:8px; }
.live-badge { display:flex; align-items:center; gap:5px; font-size:11px; font-weight:700; color:var(--success); background:rgba(34,197,94,.1); padding:2px 8px; border-radius:99px; margin-left:auto; }
.live-dot { display:inline-block; width:7px; height:7px; border-radius:50%; background:var(--success); animation:pulse-live 1.2s ease-in-out infinite; flex-shrink:0; }
@keyframes pulse-live { 0%,100%{opacity:1;transform:scale(1)} 50%{opacity:.4;transform:scale(.75)} }

/* Log list */
.logs-output { max-height:520px; overflow-y:auto; font-family:"JetBrains Mono",monospace; font-size:12px; padding:4px; }
.log-row { display:flex; align-items:center; gap:7px; padding:4px 6px; border-radius:4px; transition:background 120ms; min-height:28px; }
.log-row:hover { background:var(--bg-tertiary); }
.log-debug { opacity:.6; }
.log-warn  { background:color-mix(in srgb,var(--warning) 5%,transparent); }
.log-warn:hover { background:color-mix(in srgb,var(--warning) 10%,transparent); }
.log-error { background:color-mix(in srgb,var(--danger) 6%,transparent); }
.log-error:hover { background:color-mix(in srgb,var(--danger) 12%,transparent); }
.log-critical { background:color-mix(in srgb,#f43f5e 10%,transparent); border-left:3px solid #f43f5e; }
.log-critical:hover { background:color-mix(in srgb,#f43f5e 16%,transparent); }

.expand-btn { background:none; border:none; color:var(--text-muted); cursor:pointer; padding:0; flex-shrink:0; display:flex; align-items:center; opacity:.5; }
.expand-btn:hover { opacity:1; }

.log-time { color:var(--text-muted); font-size:10.5px; flex-shrink:0; min-width:62px; }
.log-badge { flex-shrink:0; font-size:10px !important; padding:1px 6px !important; }
.log-src-badge { flex-shrink:0; font-size:10px !important; padding:1px 6px !important; background:var(--bg-elevated) !important; color:var(--text-muted) !important; }
.log-src-text { color:var(--text-muted); font-size:10.5px; flex-shrink:0; }
.log-msg { color:var(--text-secondary); word-break:break-word; flex:1; }
.log-error .log-msg, .log-critical .log-msg { color:var(--danger); }
.log-warn .log-msg { color:var(--warning); }
.log-dur { color:var(--text-muted); font-size:10px; flex-shrink:0; margin-left:auto; }

/* Détails */
.log-details { margin:2px 0 4px 24px; border-left:2px solid var(--border); padding-left:12px; }
.log-details-meta { font-size:10px; color:var(--text-muted); margin-bottom:4px; font-family:inherit; }
.log-details-body { font-size:11px; color:var(--text-secondary); white-space:pre-wrap; word-break:break-all; background:var(--bg-elevated); border-radius:6px; padding:8px 10px; max-height:200px; overflow-y:auto; margin:0; }

.logs-empty { display:flex; flex-direction:column; align-items:center; justify-content:center; padding:40px; color:var(--text-muted); font-size:13px; font-family:inherit; text-align:center; }
</style>
