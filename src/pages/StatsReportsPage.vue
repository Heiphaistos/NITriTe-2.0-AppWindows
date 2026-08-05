<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import type { SysInfo } from "@/types/diagnostic";
import { invoke, isTauriContext } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NProgress from "@/components/ui/NProgress.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NBadge from "@/components/ui/NBadge.vue";
import { useNotificationStore } from "@/stores/notifications";
import {
  BarChart3, Cpu, MemoryStick, HardDrive, Monitor,
  Clock, CheckCircle, AlertTriangle, XCircle,
  FileText, RefreshCw, Activity, FolderOpen,
  Settings, TrendingUp, TrendingDown, Printer, Trophy,
} from "lucide-vue-next";

const notifications = useNotificationStore();
const loading = ref(true);
const generating = ref(false);

const STORAGE_THRESHOLDS = "nitrite-stats-thresholds";
const STORAGE_SNAPSHOT   = "nitrite-stats-snapshot";

interface SysStats {
  osName: string; osVersion: string; cpuModel: string;
  cpuUsage: number; ramTotal: number; ramUsed: number; ramPercent: number;
  diskTotal: number; diskUsed: number; diskPercent: number; uptime: string;
}

interface Thresholds { cpu: number; ram: number; disk: number; }
interface SnapShot { cpu: number; ram: number; disk: number; ts: string; }

const stats = ref<SysStats>({
  osName: "", osVersion: "", cpuModel: "",
  cpuUsage: 0, ramTotal: 0, ramUsed: 0, ramPercent: 0,
  diskTotal: 0, diskUsed: 0, diskPercent: 0, uptime: "",
});

// Thresholds
const thresholds = ref<Thresholds>({ cpu: 80, ram: 80, disk: 85 });
const showThresholdPanel = ref(false);

function loadThresholds() {
  try {
    const raw = localStorage.getItem(STORAGE_THRESHOLDS);
    if (raw) thresholds.value = { ...thresholds.value, ...JSON.parse(raw) };
  } catch { /* keep defaults */ }
}

function saveThresholds() {
  localStorage.setItem(STORAGE_THRESHOLDS, JSON.stringify(thresholds.value));
  notifications.success("Seuils sauvegardés", "");
}

// Snapshot (comparaison session précédente)
const prevSnapshot = ref<SnapShot | null>(null);

function loadSnapshot() {
  try {
    const raw = localStorage.getItem(STORAGE_SNAPSHOT);
    if (raw) prevSnapshot.value = JSON.parse(raw);
  } catch { /* ignore */ }
}

function saveSnapshot() {
  const snap: SnapShot = {
    cpu: stats.value.cpuUsage,
    ram: stats.value.ramPercent,
    disk: stats.value.diskPercent,
    ts: new Date().toLocaleString("fr-FR"),
  };
  localStorage.setItem(STORAGE_SNAPSHOT, JSON.stringify(snap));
}

function delta(current: number, prev: number | undefined): string {
  if (prev === undefined) return "";
  const d = current - prev;
  if (Math.abs(d) < 1) return "";
  return (d > 0 ? "+" : "") + d.toFixed(1) + "%";
}

function deltaVariant(current: number, prev: number | undefined): "success" | "danger" | "neutral" {
  if (prev === undefined || Math.abs(current - prev) < 1) return "neutral";
  return current > prev ? "danger" : "success";
}

// ── Health ────────────────────────────────────────────────────────────────────
interface HealthItem { label: string; status: "ok" | "warn" | "critical"; detail: string; }
const healthChecks = ref<HealthItem[]>([]);

function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const parts: string[] = [];
  if (d > 0) parts.push(`${d}j`);
  if (h > 0) parts.push(`${h}h`);
  parts.push(`${m}min`);
  return parts.join(" ");
}

function statusIcon(status: string) {
  if (status === "ok") return CheckCircle;
  if (status === "warn") return AlertTriangle;
  return XCircle;
}

function statusColor(status: string): string {
  if (status === "ok") return "var(--success)";
  if (status === "warn") return "var(--warning)";
  return "var(--danger)";
}

function statusVariant(status: string): "success" | "warning" | "danger" {
  if (status === "ok") return "success";
  if (status === "warn") return "warning";
  return "danger";
}

function computeHealth() {
  healthChecks.value = [
    {
      label: "CPU",
      status: stats.value.cpuUsage < 70 ? "ok" : stats.value.cpuUsage < 90 ? "warn" : "critical",
      detail: `Usage: ${stats.value.cpuUsage}%`,
    },
    {
      label: "RAM",
      status: stats.value.ramPercent < 75 ? "ok" : stats.value.ramPercent < 90 ? "warn" : "critical",
      detail: `${stats.value.ramUsed.toFixed(1)} / ${stats.value.ramTotal.toFixed(0)} GB (${stats.value.ramPercent}%)`,
    },
    {
      label: "Disque",
      status: stats.value.diskPercent < 80 ? "ok" : stats.value.diskPercent < 95 ? "warn" : "critical",
      detail: `${stats.value.diskUsed.toFixed(0)} / ${stats.value.diskTotal.toFixed(0)} GB (${stats.value.diskPercent}%)`,
    },
  ];
}

// ── Score système ─────────────────────────────────────────────────────────────
const systemScore = computed(() => {
  let score = 0;
  // CPU : <30% = 34pts, <60% = 17pts, sinon 0
  if (stats.value.cpuUsage < 30) score += 34;
  else if (stats.value.cpuUsage < 60) score += 17;
  // RAM libre : >50% = 33pts, >25% = 16pts
  const ramFree = 100 - stats.value.ramPercent;
  if (ramFree > 50) score += 33;
  else if (ramFree > 25) score += 16;
  // Disque : <80% = 33pts, <90% = 16pts
  if (stats.value.diskPercent < 80) score += 33;
  else if (stats.value.diskPercent < 90) score += 16;
  return score;
});

function scoreColor(s: number): string {
  if (s >= 80) return "var(--success)";
  if (s >= 50) return "var(--warning)";
  return "var(--danger)";
}

function scoreLabel(s: number): string {
  if (s >= 80) return "Excellent";
  if (s >= 60) return "Bon";
  if (s >= 40) return "Moyen";
  return "Critique";
}

// ── Threshold helpers ─────────────────────────────────────────────────────────
function exceeds(value: number, threshold: number): boolean {
  return value >= threshold;
}

// ── Load stats ────────────────────────────────────────────────────────────────
async function loadStats() {
  loading.value = true;
  try {
    const [info, sysHistory] = await Promise.all([
      invoke<SysInfo>("get_system_info"),
      invoke<{ current_uptime_hours: number }>("get_system_history").catch(() => null),
    ]);

    stats.value.osName    = info.os?.name ?? "Windows";
    stats.value.osVersion = info.os?.version ?? "11";
    stats.value.cpuModel  = info.cpu?.name ?? "Inconnu";
    stats.value.cpuUsage  = Math.round(info.cpu?.usage_percent ?? 0);
    stats.value.ramTotal  = info.ram?.total_gb ?? 0;
    stats.value.ramUsed   = info.ram?.used_gb ?? 0;
    stats.value.ramPercent = Math.round(info.ram?.usage_percent ?? 0);

    if ((info.disks?.length ?? 0) > 0 && (info.disks[0].partitions?.length ?? 0) > 0) {
      const p = info.disks[0].partitions[0];
      stats.value.diskTotal   = p.total_gb ?? 0;
      stats.value.diskUsed    = p.used_gb ?? 0;
      stats.value.diskPercent = Math.round(p.usage_percent ?? 0);
    }

    const uptimeHours = sysHistory?.current_uptime_hours ?? 0;
    stats.value.uptime = uptimeHours > 0 ? formatUptime(Math.round(uptimeHours * 3600)) : "N/A";
  } catch (e: unknown) {
    if (isTauriContext()) {
      notifications.error("Statistiques système", (e instanceof Error ? e.message : String(e)).slice(0, 120));
    } else {
      stats.value = {
        osName: "Windows", osVersion: "11 (26100)", cpuModel: "AMD Ryzen 7 5800X",
        cpuUsage: 23, ramTotal: 32, ramUsed: 14.4, ramPercent: 45,
        diskTotal: 931, diskUsed: 456, diskPercent: 49, uptime: "3j 14h 22min",
      };
    }
  }
  computeHealth();
  loading.value = false;
}

// ── Perf history & trends ─────────────────────────────────────────────────────
interface PerfPoint { timestamp: string; cpu_percent: number; ram_used_mb: number; ram_total_mb: number; }
interface PerfHistory { points: PerfPoint[]; avg_cpu: number; peak_cpu: number; avg_ram_mb: number; }

const perfHistory = ref<PerfHistory | null>(null);
const perfLoading = ref(false);

async function loadPerfHistory() {
  perfLoading.value = true;
  try {
    perfHistory.value = await invoke<PerfHistory>("get_perf_history", { samples: 30, intervalSecs: 2 });
  } catch (e: unknown) {
    if (isTauriContext()) {
      notifications.error("Historique perf", (e instanceof Error ? e.message : String(e)).slice(0, 120));
    } else {
      const pts: PerfPoint[] = Array.from({ length: 30 }, (_, i) => ({
        timestamp: `T-${30 - i}`,
        cpu_percent: 20 + Math.random() * 40,
        ram_used_mb: 8000 + Math.random() * 6000,
        ram_total_mb: 32768,
      }));
      perfHistory.value = { points: pts, avg_cpu: 35, peak_cpu: 60, avg_ram_mb: 11000 };
    }
  }
  perfLoading.value = false;
}

/** Calcule la tendance : (moy des 5 dernières - moy des 5 précédentes) / moy précédentes * 100 */
function trendPct(values: number[]): number | null {
  if (values.length < 10) return null;
  const last5 = values.slice(-5);
  const prev5 = values.slice(-10, -5);
  const avgLast = last5.reduce((a, b) => a + b, 0) / 5;
  const avgPrev = prev5.reduce((a, b) => a + b, 0) / 5;
  if (avgPrev === 0) return null;
  return ((avgLast - avgPrev) / avgPrev) * 100;
}

const cpuPoints  = computed(() => (perfHistory.value?.points ?? []).map(p => p.cpu_percent));
const ramPoints  = computed(() => (perfHistory.value?.points ?? []).map(p => (p.ram_used_mb / (p.ram_total_mb || 1)) * 100));

const cpuTrend  = computed(() => trendPct(cpuPoints.value));
const ramTrend  = computed(() => trendPct(ramPoints.value));

function sparklinePath(values: number[], w = 200, h = 40): string {
  if (values.length < 2) return "";
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = (max - min) || 1;
  const pts = values.map((v, i) => {
    const x = (i / (values.length - 1)) * w;
    const y = h - ((v - min) / range) * (h - 4) - 2;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  return "M " + pts.join(" L ");
}

function sparklineArea(values: number[], w = 200, h = 40): string {
  if (!values.length) return "";
  return `${sparklinePath(values, w, h)} L ${w},${h} L 0,${h} Z`;
}

// ── Disk partitions ───────────────────────────────────────────────────────────
interface Partition { letter: string; total_gb: number; used_gb: number; usage_percent: number; name?: string; }
const partitions = ref<Partition[]>([]);

async function loadPartitions() {
  try {
    const info = await invoke<SysInfo>("get_system_info");
    const parts: Partition[] = [];
    for (const disk of (info.disks ?? [])) {
      for (const p of (disk.partitions ?? [])) {
        const letter = p.mount_point?.replace(/\\+$/, "") || "?";
        parts.push({ letter, total_gb: p.total_gb ?? 0, used_gb: p.used_gb ?? 0, usage_percent: Math.round(p.usage_percent ?? 0), name: disk.name });
      }
    }
    if (parts.length) partitions.value = parts;
    else partitions.value = [{ letter: "C:", total_gb: stats.value.diskTotal, used_gb: stats.value.diskUsed, usage_percent: stats.value.diskPercent }];
  } catch (e: unknown) {
    if (isTauriContext()) {
      notifications.error("Partitions disque", (e instanceof Error ? e.message : String(e)).slice(0, 120));
    } else {
      partitions.value = [
        { letter: "C:", total_gb: 931, used_gb: 456, usage_percent: 49, name: "SSD" },
        { letter: "D:", total_gb: 2000, used_gb: 1200, usage_percent: 60, name: "HDD" },
      ];
    }
  }
}

function partitionColor(pct: number): string {
  if (pct >= 90) return "var(--danger)";
  if (pct >= 75) return "var(--warning)";
  return "var(--success)";
}

// ── Reports ───────────────────────────────────────────────────────────────────
interface ReportFile { name: string; path: string; size_bytes: number; created: string; }
const reports = ref<ReportFile[]>([]);
const reportsLoading = ref(false);

async function loadReports() {
  reportsLoading.value = true;
  try {
    reports.value = await invoke<ReportFile[]>("list_reports");
  } catch (e: unknown) {
    if (isTauriContext()) {
      reports.value = [];
      notifications.error("Rapports", (e instanceof Error ? e.message : String(e)).slice(0, 120));
    } else {
      reports.value = [
        { name: "rapport-systeme-2026-03-01.txt", path: "C:\\NiTriTe\\rapports\\rapport-systeme-2026-03-01.txt", size_bytes: 4096, created: "2026-03-01 10:00" },
        { name: "backup-info-2026-02-28.txt", path: "C:\\NiTriTe\\rapports\\backup-info-2026-02-28.txt", size_bytes: 8192, created: "2026-02-28 14:30" },
      ];
    }
  } finally {
    reportsLoading.value = false;
  }
}

async function openReportFolder() {
  try {
    await invoke("run_system_command", { cmd: "explorer", args: ["%USERPROFILE%\\Documents\\NiTriTe"] });
  } catch (e: unknown) {
    if (!isTauriContext()) { notifications.info("Mode dev", "Ouverture dossier simulee"); }
    else { notifications.error("Ouverture dossier", (e instanceof Error ? e.message : String(e)).slice(0, 120)); }
  }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}

async function generateReport() {
  generating.value = true;
  try {
    notifications.info("Rapport", "Utilisez le bouton PDF pour exporter le rapport.");
    window.print();
  } finally {
    generating.value = false;
  }
}

function exportPDF() {
  window.print();
}

onMounted(async () => {
  loadThresholds();
  loadSnapshot();
  await loadStats();
  saveSnapshot();
  loadReports();
  loadPerfHistory();
  loadPartitions();
});
</script>

<template>
  <div class="stats-page print-area">
    <div class="page-header">
      <div>
        <h1>Statistiques & Rapports</h1>
        <p class="page-subtitle">Vue d'ensemble et generation de rapports systeme</p>
      </div>
      <div class="header-actions">
        <NButton variant="ghost" size="sm" @click="exportPDF" title="Imprimer / Exporter PDF">
          <Printer :size="14" />
          PDF
        </NButton>
        <NButton variant="secondary" size="sm" :loading="generating" @click="generateReport">
          <FileText :size="14" />
          Générer rapport
        </NButton>
        <NButton variant="primary" size="sm" :loading="loading" @click="loadStats">
          <RefreshCw :size="14" />
          Rafraîchir
        </NButton>
      </div>
    </div>

    <div v-if="loading" class="loading-state">
      <NSpinner :size="32" />
      <p>Collecte des statistiques...</p>
    </div>

    <template v-else>
      <!-- Score système -->
      <div class="score-banner" :style="{ borderColor: scoreColor(systemScore), boxShadow: `0 0 20px ${scoreColor(systemScore)}33` }">
        <Trophy :size="22" :style="{ color: scoreColor(systemScore) }" />
        <div>
          <div class="score-title">Score Système</div>
          <div class="score-sub">Basé sur CPU, RAM et espace disque</div>
        </div>
        <div class="score-value" :style="{ color: scoreColor(systemScore) }">
          {{ systemScore }}<span class="score-max">/100</span>
        </div>
        <NBadge :variant="systemScore >= 80 ? 'success' : systemScore >= 50 ? 'warning' : 'danger'" size="sm">
          {{ scoreLabel(systemScore) }}
        </NBadge>
        <!-- Comparaison session précédente -->
        <div v-if="prevSnapshot" class="score-delta">
          <span v-if="delta(stats.cpuUsage, prevSnapshot.cpu)" class="delta-chip" :class="deltaVariant(stats.cpuUsage, prevSnapshot.cpu)">
            CPU {{ delta(stats.cpuUsage, prevSnapshot.cpu) }}
          </span>
          <span v-if="delta(stats.ramPercent, prevSnapshot.ram)" class="delta-chip" :class="deltaVariant(stats.ramPercent, prevSnapshot.ram)">
            RAM {{ delta(stats.ramPercent, prevSnapshot.ram) }}
          </span>
          <span v-if="delta(stats.diskPercent, prevSnapshot.disk)" class="delta-chip" :class="deltaVariant(stats.diskPercent, prevSnapshot.disk)">
            Disk {{ delta(stats.diskPercent, prevSnapshot.disk) }}
          </span>
          <span class="prev-ts">vs session du {{ prevSnapshot.ts }}</span>
        </div>
      </div>

      <!-- Uptime -->
      <NCard>
        <div class="uptime-display">
          <Clock :size="20" style="color: var(--accent-primary)" />
          <div>
            <span class="uptime-label">Uptime systeme</span>
            <span class="uptime-value">{{ stats.uptime }}</span>
          </div>
        </div>
      </NCard>

      <!-- Summary Cards -->
      <div class="summary-grid">
        <NCard>
          <div class="summary-item">
            <div class="summary-icon" style="background: var(--accent-muted)">
              <Cpu :size="20" style="color: var(--accent-primary)" />
            </div>
            <div class="summary-info">
              <span class="summary-label">Processeur</span>
              <span class="summary-value">{{ stats.cpuModel }}</span>
              <span class="summary-detail" :class="{ 'over-threshold': exceeds(stats.cpuUsage, thresholds.cpu) }">
                Usage: {{ stats.cpuUsage }}%
              </span>
            </div>
          </div>
        </NCard>

        <NCard>
          <div class="summary-item">
            <div class="summary-icon" style="background: var(--info-muted)">
              <MemoryStick :size="20" style="color: var(--info)" />
            </div>
            <div class="summary-info">
              <span class="summary-label">Memoire RAM</span>
              <span class="summary-value">{{ stats.ramTotal.toFixed(0) }} GB</span>
              <span class="summary-detail" :class="{ 'over-threshold': exceeds(stats.ramPercent, thresholds.ram) }">
                {{ stats.ramUsed.toFixed(1) }} GB utilises
              </span>
            </div>
          </div>
        </NCard>

        <NCard>
          <div class="summary-item">
            <div class="summary-icon" style="background: var(--warning-muted)">
              <HardDrive :size="20" style="color: var(--warning)" />
            </div>
            <div class="summary-info">
              <span class="summary-label">Stockage</span>
              <span class="summary-value">{{ stats.diskTotal.toFixed(0) }} GB</span>
              <span class="summary-detail" :class="{ 'over-threshold': exceeds(stats.diskPercent, thresholds.disk) }">
                {{ stats.diskUsed.toFixed(0) }} GB utilises
              </span>
            </div>
          </div>
        </NCard>

        <NCard>
          <div class="summary-item">
            <div class="summary-icon" style="background: var(--success-muted)">
              <Monitor :size="20" style="color: var(--success)" />
            </div>
            <div class="summary-info">
              <span class="summary-label">Systeme</span>
              <span class="summary-value">{{ stats.osName }}</span>
              <span class="summary-detail">{{ stats.osVersion }}</span>
            </div>
          </div>
        </NCard>
      </div>

      <!-- Health Check -->
      <NCard>
        <template #header>
          <div class="section-header">
            <Activity :size="16" />
            <span>Verification de sante</span>
          </div>
        </template>
        <div class="health-list">
          <div v-for="check in healthChecks" :key="check.label" class="health-row">
            <div class="health-left">
              <component :is="statusIcon(check.status)" :size="16" :style="{ color: statusColor(check.status) }" />
              <span class="health-name">{{ check.label }}</span>
            </div>
            <span class="health-detail">{{ check.detail }}</span>
            <NBadge :variant="statusVariant(check.status)">
              {{ check.status === "ok" ? "OK" : check.status === "warn" ? "Attention" : "Critique" }}
            </NBadge>
          </div>
        </div>
      </NCard>

      <!-- Usage bars + seuils -->
      <NCard>
        <template #header>
          <div class="section-header">
            <BarChart3 :size="16" />
            <span>Utilisation des ressources</span>
            <button class="threshold-btn" @click="showThresholdPanel = !showThresholdPanel" title="Configurer les seuils">
              <Settings :size="14" />
            </button>
          </div>
        </template>

        <!-- Panel seuils inline -->
        <div v-if="showThresholdPanel" class="threshold-panel">
          <div class="threshold-title">Seuils d'alerte (%)</div>
          <div class="threshold-fields">
            <label class="threshold-field">
              <span>CPU</span>
              <input type="number" v-model.number="thresholds.cpu" min="1" max="100" class="threshold-input" />
            </label>
            <label class="threshold-field">
              <span>RAM</span>
              <input type="number" v-model.number="thresholds.ram" min="1" max="100" class="threshold-input" />
            </label>
            <label class="threshold-field">
              <span>Disque</span>
              <input type="number" v-model.number="thresholds.disk" min="1" max="100" class="threshold-input" />
            </label>
            <NButton variant="primary" size="sm" @click="saveThresholds">Sauvegarder</NButton>
          </div>
        </div>

        <div class="usage-bars">
          <div class="usage-row">
            <span class="usage-label">CPU</span>
            <NProgress :value="stats.cpuUsage" showLabel />
            <div class="trend-badge" v-if="cpuTrend !== null">
              <TrendingUp v-if="cpuTrend > 0" :size="12" class="trend-up" />
              <TrendingDown v-else :size="12" class="trend-down" />
              <span :class="cpuTrend > 0 ? 'trend-up' : 'trend-down'">{{ Math.abs(cpuTrend).toFixed(1) }}%</span>
            </div>
            <NBadge v-if="exceeds(stats.cpuUsage, thresholds.cpu)" variant="danger" size="sm">
              Seuil !
            </NBadge>
          </div>
          <div class="usage-row">
            <span class="usage-label">RAM</span>
            <NProgress :value="stats.ramPercent" showLabel />
            <div class="trend-badge" v-if="ramTrend !== null">
              <TrendingUp v-if="ramTrend > 0" :size="12" class="trend-up" />
              <TrendingDown v-else :size="12" class="trend-down" />
              <span :class="ramTrend > 0 ? 'trend-up' : 'trend-down'">{{ Math.abs(ramTrend).toFixed(1) }}%</span>
            </div>
            <NBadge v-if="exceeds(stats.ramPercent, thresholds.ram)" variant="danger" size="sm">
              Seuil !
            </NBadge>
          </div>
          <div class="usage-row">
            <span class="usage-label">Disque</span>
            <NProgress :value="stats.diskPercent" showLabel />
            <NBadge v-if="exceeds(stats.diskPercent, thresholds.disk)" variant="danger" size="sm">
              Seuil !
            </NBadge>
          </div>
        </div>
      </NCard>

      <!-- Sparklines historique -->
      <NCard v-if="perfHistory || perfLoading">
        <template #header>
          <div class="section-header">
            <Activity :size="16" />
            <span>Historique des 2 dernières minutes</span>
          </div>
        </template>
        <div v-if="perfLoading" style="text-align:center;padding:20px;color:var(--text-muted);font-size:13px">Collecte des donnees...</div>
        <div v-else-if="perfHistory" class="sparklines-grid">
          <div class="sparkline-card">
            <div class="sparkline-label">
              CPU — moy. {{ perfHistory.avg_cpu.toFixed(1) }}% · pic {{ perfHistory.peak_cpu.toFixed(1) }}%
              <span v-if="cpuTrend !== null" class="trend-inline" :class="cpuTrend > 0 ? 'trend-up' : 'trend-down'">
                <TrendingUp v-if="cpuTrend > 0" :size="11" />
                <TrendingDown v-else :size="11" />
                {{ Math.abs(cpuTrend).toFixed(1) }}%
              </span>
            </div>
            <svg viewBox="0 0 200 40" class="spark-svg">
              <defs>
                <linearGradient id="cpuGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stop-color="var(--accent-primary)" stop-opacity="0.4"/>
                  <stop offset="100%" stop-color="var(--accent-primary)" stop-opacity="0.02"/>
                </linearGradient>
              </defs>
              <path :d="sparklineArea(cpuPoints)" fill="url(#cpuGrad)" />
              <path :d="sparklinePath(cpuPoints)" fill="none" stroke="var(--accent-primary)" stroke-width="1.5" stroke-linecap="round" />
            </svg>
          </div>
          <div class="sparkline-card">
            <div class="sparkline-label">
              RAM — moy. {{ (perfHistory.avg_ram_mb / 1024).toFixed(1) }} GB
              <span v-if="ramTrend !== null" class="trend-inline" :class="ramTrend > 0 ? 'trend-up' : 'trend-down'">
                <TrendingUp v-if="ramTrend > 0" :size="11" />
                <TrendingDown v-else :size="11" />
                {{ Math.abs(ramTrend).toFixed(1) }}%
              </span>
            </div>
            <svg viewBox="0 0 200 40" class="spark-svg">
              <defs>
                <linearGradient id="ramGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stop-color="var(--info)" stop-opacity="0.4"/>
                  <stop offset="100%" stop-color="var(--info)" stop-opacity="0.02"/>
                </linearGradient>
              </defs>
              <path :d="sparklineArea(ramPoints)" fill="url(#ramGrad)" />
              <path :d="sparklinePath(ramPoints)" fill="none" stroke="var(--info)" stroke-width="1.5" stroke-linecap="round" />
            </svg>
          </div>
        </div>
      </NCard>

      <!-- Breakdown disques -->
      <NCard v-if="partitions.length">
        <template #header>
          <div class="section-header">
            <HardDrive :size="16" /><span>Partitions disques ({{ partitions.length }})</span>
          </div>
        </template>
        <div class="partitions-list">
          <div v-for="p in partitions" :key="p.letter" class="partition-row">
            <div class="part-letter">{{ p.letter }}</div>
            <div class="part-bar-wrap">
              <div class="part-bar-track">
                <div class="part-bar-fill" :style="{ width: `${p.usage_percent}%`, background: partitionColor(p.usage_percent) }" />
              </div>
            </div>
            <div class="part-info">
              <span :style="{ color: partitionColor(p.usage_percent) }">{{ p.usage_percent }}%</span>
              <span class="part-detail">{{ p.used_gb.toFixed(0) }} / {{ p.total_gb.toFixed(0) }} GB</span>
            </div>
          </div>
        </div>
      </NCard>

      <!-- Reports listing -->
      <NCard>
        <template #header>
          <div class="section-header">
            <FileText :size="16" />
            <span>Rapports generes ({{ reports.length }})</span>
            <NButton variant="secondary" size="sm" @click="openReportFolder" style="margin-left: auto">
              <FolderOpen :size="14" /> Ouvrir dossier
            </NButton>
          </div>
        </template>
        <div v-if="reportsLoading" class="loading-state" style="padding: 24px">
          <NSpinner :size="20" />
        </div>
        <div v-else-if="reports.length === 0" class="reports-empty">
          Aucun rapport genere pour le moment.
        </div>
        <div v-else class="reports-list">
          <div v-for="r in reports" :key="r.path" class="report-row">
            <FileText :size="14" style="color: var(--text-muted); flex-shrink: 0" />
            <div class="report-info">
              <span class="report-name">{{ r.name }}</span>
              <span class="report-meta">{{ r.created }} — {{ formatBytes(r.size_bytes) }}</span>
            </div>
          </div>
        </div>
      </NCard>
    </template>
  </div>
</template>

<style scoped>
.stats-page { display:flex; flex-direction:column; gap:16px; }

.page-header { display:flex; justify-content:space-between; align-items:flex-start; flex-wrap:wrap; gap:12px; }
.page-header h1 { font-size:24px; font-weight:700; }
.page-subtitle { color:var(--text-muted); font-size:13px; margin-top:2px; }
.header-actions { display:flex; gap:8px; flex-wrap:wrap; align-items:center; }
.format-selector { display:flex; gap:2px; background:var(--bg3); border:1px solid var(--border); border-radius:6px; padding:2px; }
.fmt-btn { padding:3px 9px; font-size:11px; font-weight:600; border:none; background:transparent; color:var(--text2); cursor:pointer; border-radius:4px; transition:all .15s; }
.fmt-btn:hover { color:var(--text); background:var(--bg2); }
.fmt-btn.active { background:var(--accent); color:#fff; }

.section-header { display:flex; align-items:center; gap:8px; width:100%; }

.loading-state { display:flex; flex-direction:column; align-items:center; gap:12px; padding:60px; color:var(--text-muted); }

/* Score banner */
.score-banner {
  display: flex; align-items: center; gap: 16px; flex-wrap: wrap;
  background: var(--bg-secondary); border: 1px solid var(--border);
  border-radius: 14px; padding: 16px 20px;
  transition: box-shadow .3s ease;
}
.score-title { font-size: 14px; font-weight: 700; color: var(--text-primary); }
.score-sub { font-size: 11px; color: var(--text-muted); }
.score-value { font-size: 36px; font-weight: 900; font-family: "JetBrains Mono", monospace; margin-left: auto; }
.score-max { font-size: 16px; font-weight: 400; color: var(--text-muted); }
.score-delta { display:flex; align-items:center; gap:8px; flex-wrap:wrap; margin-left:8px; }
.delta-chip { font-size: 11px; font-weight: 600; padding: 2px 7px; border-radius: 99px; }
.delta-chip.success { background: rgba(34,197,94,.15); color: var(--success); }
.delta-chip.danger  { background: rgba(239,68,68,.15); color: var(--danger); }
.delta-chip.neutral { background: var(--bg-tertiary); color: var(--text-muted); }
.prev-ts { font-size: 10px; color: var(--text-muted); }

/* Threshold */
.threshold-btn { background:none; border:1px solid var(--border); border-radius:6px; padding:3px 6px; color:var(--text-muted); cursor:pointer; display:flex; align-items:center; margin-left:auto; transition:all var(--transition-fast); }
.threshold-btn:hover { color:var(--text-primary); border-color:var(--accent-primary); }
.threshold-panel { background:var(--bg-tertiary); border:1px solid var(--border); border-radius:10px; padding:12px 16px; margin-bottom:12px; }
.threshold-title { font-size:11px; font-weight:700; text-transform:uppercase; letter-spacing:.05em; color:var(--text-muted); margin-bottom:10px; }
.threshold-fields { display:flex; align-items:center; gap:16px; flex-wrap:wrap; }
.threshold-field { display:flex; align-items:center; gap:8px; font-size:13px; color:var(--text-secondary); }
.threshold-input { width:64px; padding:4px 8px; border:1px solid var(--border); border-radius:6px; background:var(--bg-secondary); color:var(--text-primary); font-size:13px; }
.threshold-input:focus { outline:none; border-color:var(--accent-primary); }

/* Usage rows */
.usage-bars { display:flex; flex-direction:column; gap:12px; }
.usage-row { display:flex; align-items:center; gap:12px; }
.usage-label { min-width:60px; font-size:13px; font-weight:500; color:var(--text-secondary); }

/* Trend */
.trend-badge { display:flex; align-items:center; gap:3px; font-size:11px; font-weight:600; white-space:nowrap; }
.trend-inline { display:inline-flex; align-items:center; gap:3px; font-size:10px; font-weight:600; margin-left:8px; }
.trend-up { color:var(--danger); }
.trend-down { color:var(--success); }

/* Over threshold coloring */
.over-threshold { color: var(--danger) !important; font-weight: 700; }

/* Uptime */
.uptime-display { display:flex; align-items:center; gap:16px; }
.uptime-label { display:block; font-size:12px; color:var(--text-muted); text-transform:uppercase; letter-spacing:.5px; }
.uptime-value { display:block; font-size:22px; font-weight:700; font-family:"JetBrains Mono",monospace; color:var(--text-primary); }

/* Summary */
.summary-grid { display:grid; grid-template-columns:repeat(4,1fr); gap:16px; }
@media(max-width:1200px){ .summary-grid{ grid-template-columns:repeat(2,1fr); } }
@media(max-width:600px){ .summary-grid{ grid-template-columns:1fr; } }
.summary-item { display:flex; align-items:flex-start; gap:12px; }
.summary-icon { width:40px; height:40px; border-radius:var(--radius-md); display:flex; align-items:center; justify-content:center; flex-shrink:0; }
.summary-info { display:flex; flex-direction:column; gap:2px; }
.summary-label { font-size:12px; color:var(--text-muted); }
.summary-value { font-size:14px; font-weight:600; color:var(--text-primary); }
.summary-detail { font-size:12px; color:var(--text-secondary); }

/* Health */
.health-list { display:flex; flex-direction:column; gap:8px; }
.health-row { display:flex; align-items:center; gap:12px; padding:8px 0; border-bottom:1px solid var(--border); }
.health-row:last-child { border-bottom:none; }
.health-left { display:flex; align-items:center; gap:8px; min-width:100px; }
.health-name { font-size:13px; font-weight:500; color:var(--text-primary); }
.health-detail { flex:1; font-size:12px; color:var(--text-secondary); font-family:"JetBrains Mono",monospace; }

/* Sparklines */
.sparklines-grid { display:grid; grid-template-columns:1fr 1fr; gap:16px; }
@media(max-width:700px){ .sparklines-grid{ grid-template-columns:1fr; } }
.sparkline-card { display:flex; flex-direction:column; gap:6px; background:var(--bg-secondary); border:1px solid var(--border); border-radius:8px; padding:12px; }
.sparkline-label { font-size:11px; color:var(--text-muted); font-weight:500; display:flex; align-items:center; }
.spark-svg { width:100%; height:48px; }

/* Partitions */
.partitions-list { display:flex; flex-direction:column; gap:10px; }
.partition-row { display:flex; align-items:center; gap:12px; }
.part-letter { font-family:"JetBrains Mono",monospace; font-size:13px; font-weight:700; color:var(--text-primary); min-width:40px; }
.part-bar-wrap { flex:1; }
.part-bar-track { height:8px; border-radius:99px; background:var(--bg-elevated); border:1px solid var(--border); overflow:hidden; }
.part-bar-fill { height:100%; border-radius:99px; transition:width .4s ease; }
.part-info { display:flex; flex-direction:column; align-items:flex-end; min-width:100px; }
.part-info span:first-child { font-size:13px; font-weight:700; font-family:"JetBrains Mono",monospace; }
.part-detail { font-size:11px; color:var(--text-muted); }

/* Reports */
.reports-empty { text-align:center; color:var(--text-muted); padding:24px; font-size:13px; }
.reports-list { display:flex; flex-direction:column; gap:4px; }
.report-row { display:flex; align-items:center; gap:10px; padding:8px 10px; border-radius:var(--radius-md); transition:background var(--transition-fast); }
.report-row:hover { background:var(--bg-tertiary); }
.report-info { display:flex; flex-direction:column; gap:2px; }
.report-name { font-size:13px; font-weight:500; color:var(--text-primary); font-family:"JetBrains Mono",monospace; }
.report-meta { font-size:11px; color:var(--text-muted); }

/* Print */
@media print {
  .header-actions, .threshold-btn, .threshold-panel { display:none !important; }
  .stats-page { gap:10px; }
}
</style>
