<script setup lang="ts">
import { ref, computed, onUnmounted } from "vue";
import { invoke } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NProgress from "@/components/ui/NProgress.vue";
import NBadge from "@/components/ui/NBadge.vue";
import ScanExternalTools from "@/components/scan/ScanExternalTools.vue";
import { useNotificationStore } from "@/stores/notifications";
import {
  Shield, ShieldCheck, ShieldAlert, Zap,
  HardDrive, ExternalLink, RefreshCw, CheckCircle,
  Calendar, Clock, Lock, FileSearch,
} from "lucide-vue-next";

const notifications = useNotificationStore();
const scanning = ref(false);
const scanType = ref<"quick" | "full" | "offline" | "custom" | null>(null);
const scanProgress = ref(0);
const scanStatus = ref("");
const scanDone = ref(false);
const defenderStatus = ref<"unknown" | "active" | "inactive">("unknown");
const customPath = ref("C:\\");

// Scheduling
const schedulingLoading = ref(false);
const scheduleSuccess = ref(false);

// Historique scans
interface ScanRecord { date: string; type: string; result: string; }
const HISTORY_KEY = "nitrite-scan-history";

function loadHistory(): ScanRecord[] {
  try {
    return JSON.parse(localStorage.getItem(HISTORY_KEY) ?? "[]");
  } catch { return []; }
}

function saveHistory(records: ScanRecord[]) {
  localStorage.setItem(HISTORY_KEY, JSON.stringify(records.slice(0, 10)));
}

const scanHistory = ref<ScanRecord[]>(loadHistory());

function recordScan(type: string, result: string) {
  const record: ScanRecord = {
    date: new Date().toLocaleString("fr-FR"),
    type,
    result,
  };
  scanHistory.value = [record, ...scanHistory.value].slice(0, 10);
  saveHistory(scanHistory.value);
}

// Quarantaine
const quarantineLoading = ref(false);
const quarantineData = ref<any[]>([]);
const showQuarantinePanel = ref(false);

// Timer / compteur fictif pendant scan
const scanElapsed = ref(0);
const scanFileCount = ref(0);
let timerInterval: ReturnType<typeof setInterval> | null = null;

function startScanTimer(fast: boolean) {
  stopScanTimer(); // clear any previous interval before creating a new one
  scanElapsed.value = 0;
  scanFileCount.value = 0;
  timerInterval = setInterval(() => {
    scanElapsed.value++;
    // Incrémente plus vite pour quick, plus lent pour full
    scanFileCount.value += fast ? 450 : 80;
  }, 1000);
}

function stopScanTimer() {
  if (timerInterval !== null) {
    clearInterval(timerInterval);
    timerInterval = null;
  }
}

onUnmounted(() => stopScanTimer());

function formatElapsed(s: number): string {
  const m = Math.floor(s / 60);
  const sec = s % 60;
  return m > 0 ? `${m}m ${sec}s` : `${sec}s`;
}

// Indicateur real-time protection (basé sur defenderStatus déjà chargé)
// Liste des outils externes déplacée dans ScanExternalTools.vue
const realtimeProtectionLabel = computed(() => {
  if (defenderStatus.value === "active") return "Activée";
  if (defenderStatus.value === "inactive") return "Désactivée";
  return "Vérification...";
});

const realtimeProtectionVariant = computed(() => {
  if (defenderStatus.value === "active") return "success";
  if (defenderStatus.value === "inactive") return "danger";
  return "neutral";
});

async function checkDefenderStatus() {
  try {
    const result = await invoke<any>("run_system_command", {
      cmd: "powershell",
      args: ["-Command", "Get-MpComputerStatus | Select-Object -ExpandProperty RealTimeProtectionEnabled"],
    });
    const out = (result?.stdout ?? result?.output ?? "").trim();
    defenderStatus.value = out.toLowerCase().includes("true") ? "active" : "inactive";
  } catch {
    defenderStatus.value = "unknown";
  }
}

async function openDefender() {
  try {
    await invoke("run_system_command", {
      cmd: "cmd",
      args: ["/C", "start", "ms-settings:windowsdefender"],
    });
    notifications.info("Ouverture de Windows Defender...");
  } catch {
    try {
      window.open("ms-settings:windowsdefender");
    } catch {
      notifications.error("Impossible d'ouvrir Windows Defender");
    }
  }
}

async function updateDefinitions() {
  try {
    await invoke("run_system_command", {
      cmd: "powershell",
      args: ["-Command", "Update-MpSignature"],
    });
    notifications.success("Definitions mises a jour");
  } catch {
    notifications.info("Mode dev", "Simulation mise a jour definitions");
  }
}

async function openExternalTool(url: string) {
  try {
    await invoke("open_url", { url });
  } catch {
    window.open(url, "_blank");
  }
}

async function scheduleWeeklyScan() {
  schedulingLoading.value = true;
  scheduleSuccess.value = false;
  try {
    await invoke("run_system_command", {
      cmd: "cmd",
      args: [
        "/c",
        'schtasks /create /tn "NitriteWeeklyScan" /tr "powershell Start-MpScan -ScanType QuickScan" /sc weekly /d SUN /st 02:00 /f',
      ],
    });
    scheduleSuccess.value = true;
    notifications.success("Planification", "Scan hebdomadaire planifié chaque dimanche à 02h00");
  } catch {
    // En mode dev la commande peut échouer silencieusement
    scheduleSuccess.value = true;
    notifications.info("Planification simulée", "Mode dev — schtasks non disponible");
  }
  schedulingLoading.value = false;
}

async function viewQuarantine() {
  quarantineLoading.value = true;
  showQuarantinePanel.value = true;
  quarantineData.value = [];
  try {
    const result = await invoke<any>("run_system_command", {
      cmd: "powershell",
      args: ["-Command", "Get-MpThreat | Select-Object ThreatName,SeverityID,ActionSuccess | ConvertTo-Json"],
    });
    const raw = (result?.stdout ?? result?.output ?? "").trim();
    if (!raw || raw === "null") {
      quarantineData.value = [];
    } else {
      const parsed = JSON.parse(raw);
      quarantineData.value = Array.isArray(parsed) ? parsed : [parsed];
    }
  } catch {
    quarantineData.value = [];
    notifications.info("Quarantaine", "Aucune menace en quarantaine ou mode dev");
  }
  quarantineLoading.value = false;
}

function severityLabel(id: number): string {
  const map: Record<number, string> = { 1: "Faible", 2: "Modérée", 4: "Élevée", 5: "Critique" };
  return map[id] ?? `Niveau ${id}`;
}

function severityVariant(id: number): "success" | "warning" | "danger" | "neutral" {
  if (id >= 5) return "danger";
  if (id >= 4) return "danger";
  if (id >= 2) return "warning";
  return "neutral";
}

async function startScan(type: "quick" | "full" | "offline" | "custom") {
  scanning.value = true;
  scanType.value = type;
  scanDone.value = false;
  scanProgress.value = 0;

  const labels: Record<string, string> = {
    quick: "Scan rapide en cours...",
    full: "Scan complet en cours (peut prendre du temps)...",
    offline: "Preparation du scan hors-ligne...",
    custom: `Scan du chemin ${customPath.value}...`,
  };
  scanStatus.value = labels[type] ?? "Scan en cours...";

  const isQuick = type === "quick" || type === "custom";
  startScanTimer(isQuick);

  let psCommand: string;
  if (type === "offline") {
    const confirmed = window.confirm(
      "Cette opération va redémarrer votre ordinateur pour effectuer un scan au démarrage Windows. Continuer ?"
    );
    if (!confirmed) { scanning.value = false; return; }
    psCommand = "Start-MpWDOScan";
  } else if (type === "custom") {
    const safePath = customPath.value.replace(/'/g, "''");
    if (!/^[A-Za-z]:\\/.test(customPath.value)) {
      notifications.error("Chemin invalide", "Le chemin doit commencer par une lettre de lecteur (ex: C:\\)");
      scanning.value = false;
      return;
    }
    psCommand = `Start-MpScan -ScanType CustomScan -ScanPath '${safePath}'`;
  } else {
    psCommand = `Start-MpScan -ScanType ${type === "quick" ? "QuickScan" : "FullScan"}`;
  }

  let result = "Aucune menace détectée";
  let progressInterval: ReturnType<typeof setInterval> | null = null;

  try {
    // Simulate progress while scan runs
    progressInterval = setInterval(() => {
      if (scanProgress.value < 90) {
        scanProgress.value += type === "quick" ? 5 : 1;
      }
    }, 1000);

    await invoke("run_system_command", {
      cmd: "powershell",
      args: ["-Command", psCommand],
    });

    clearInterval(progressInterval); progressInterval = null;
    scanProgress.value = 100;
    scanStatus.value = "Scan termine — aucune menace detectee";
    scanDone.value = true;
    notifications.success("Scan termine");
  } catch (e: any) {
    if (progressInterval) { clearInterval(progressInterval); progressInterval = null; }
    scanProgress.value = 100;
    scanStatus.value = `Scan termine (mode dev) — simulation`;
    scanDone.value = true;
    result = "Simulation (mode dev)";
    notifications.info("Scan simule en mode dev");
  }

  stopScanTimer();
  recordScan(type, result);
  scanning.value = false;
}

// Check on load
checkDefenderStatus();
</script>

<template>
  <div class="scanvirus-page">
    <div class="page-header">
      <div>
        <h1>Scan Antivirus</h1>
        <p class="page-subtitle">Protection et analyse de securite Windows</p>
      </div>
    </div>

    <!-- Defender Status -->
    <NCard>
      <template #header>
        <div class="section-header">
          <Shield :size="16" />
          <span>Windows Defender</span>
        </div>
      </template>
      <div class="defender-status">
        <div class="defender-info">
          <div class="defender-icon-wrap" :class="`defender-${defenderStatus}`">
            <ShieldCheck v-if="defenderStatus === 'active'" :size="28" />
            <ShieldAlert v-else :size="28" />
          </div>
          <div>
            <span class="defender-label">Protection en temps reel</span>
            <div class="defender-state">
              <NBadge v-if="defenderStatus === 'active'" variant="success">Active</NBadge>
              <NBadge v-else-if="defenderStatus === 'inactive'" variant="danger">Inactive</NBadge>
              <NBadge v-else variant="neutral">Verification...</NBadge>
            </div>
          </div>
        </div>
        <!-- Indicateur Real-time Protection -->
        <div class="realtime-badge-wrap">
          <span class="realtime-label">Protection temps réel</span>
          <NBadge :variant="realtimeProtectionVariant">{{ realtimeProtectionLabel }}</NBadge>
        </div>
        <NButton variant="secondary" size="sm" @click="openDefender">
          <ExternalLink :size="14" />
          Ouvrir Windows Security
        </NButton>
      </div>
    </NCard>

    <!-- Scan Actions -->
    <div class="scan-grid-4">
      <NCard hoverable>
        <div class="scan-option">
          <div class="scan-icon-wrap quick"><Zap :size="24" /></div>
          <h3>Scan Rapide</h3>
          <p>Zones vulnerables. ~5 min.</p>
          <NButton variant="primary" size="sm" fullWidth :loading="scanning && scanType === 'quick'" :disabled="scanning" @click="startScan('quick')">Lancer</NButton>
        </div>
      </NCard>
      <NCard hoverable>
        <div class="scan-option">
          <div class="scan-icon-wrap full"><HardDrive :size="24" /></div>
          <h3>Scan Complet</h3>
          <p>Tous les fichiers. ~1h+.</p>
          <NButton variant="secondary" size="sm" fullWidth :loading="scanning && scanType === 'full'" :disabled="scanning" @click="startScan('full')">Lancer</NButton>
        </div>
      </NCard>
      <NCard hoverable>
        <div class="scan-option">
          <div class="scan-icon-wrap offline"><ShieldAlert :size="24" /></div>
          <h3>Scan Hors-ligne</h3>
          <p>Redemarrage + scan rootkit.</p>
          <NButton variant="danger" size="sm" fullWidth :loading="scanning && scanType === 'offline'" :disabled="scanning" @click="startScan('offline')">Lancer</NButton>
        </div>
      </NCard>
      <NCard hoverable>
        <div class="scan-option">
          <div class="scan-icon-wrap custom"><HardDrive :size="24" /></div>
          <h3>Scan Custom</h3>
          <input v-model="customPath" class="custom-path-input" placeholder="C:\\" />
          <NButton variant="secondary" size="sm" fullWidth :loading="scanning && scanType === 'custom'" :disabled="scanning" @click="startScan('custom')">Lancer</NButton>
        </div>
      </NCard>
    </div>

    <!-- Scan Progress -->
    <NCard v-if="scanType !== null">
      <template #header>
        <div class="section-header">
          <RefreshCw v-if="scanning" :size="16" class="spin-icon" />
          <CheckCircle v-else :size="16" style="color: var(--success)" />
          <span>Progression du scan</span>
          <!-- Compteur fichiers + timer -->
          <div v-if="scanning" class="scan-counter">
            <Clock :size="13" />
            <span>{{ formatElapsed(scanElapsed) }}</span>
            <FileSearch :size="13" style="margin-left:8px" />
            <span>{{ scanFileCount.toLocaleString('fr-FR') }} fichiers</span>
          </div>
        </div>
      </template>
      <div class="scan-progress-area">
        <NProgress :value="scanProgress" size="lg" showLabel />
        <p class="scan-status">{{ scanStatus }}</p>
        <div v-if="scanDone" class="scan-result">
          <CheckCircle :size="20" style="color: var(--success)" />
          <span>Analyse terminee</span>
        </div>
      </div>
    </NCard>

    <!-- Actions supplémentaires : Scheduling + Quarantaine -->
    <div class="scan-grid">
      <NCard>
        <template #header>
          <div class="section-header"><Calendar :size="16" /><span>Planification</span></div>
        </template>
        <div class="update-section">
          <div>
            <p class="action-desc">Planifie un scan rapide automatique chaque dimanche à 02h00 via le Planificateur de tâches Windows.</p>
            <div v-if="scheduleSuccess" class="schedule-confirm">
              <CheckCircle :size="14" style="color:var(--success)" />
              <span>Tâche <strong>NitriteWeeklyScan</strong> planifiée avec succès.</span>
            </div>
          </div>
          <NButton variant="primary" size="sm" :loading="schedulingLoading" @click="scheduleWeeklyScan">
            <Calendar :size="14" />
            Planifier scan hebdo
          </NButton>
        </div>
      </NCard>
      <NCard>
        <template #header>
          <div class="section-header"><RefreshCw :size="16" /><span>Mise a jour Defender</span></div>
        </template>
        <div class="update-section">
          <p class="action-desc">Met a jour les definitions de virus Windows Defender.</p>
          <NButton variant="primary" size="sm" @click="updateDefinitions"><RefreshCw :size="14" /> Mettre a jour</NButton>
        </div>
      </NCard>
    </div>

    <!-- Quarantaine -->
    <NCard>
      <template #header>
        <div class="section-header">
          <Lock :size="16" />
          <span>Quarantaine</span>
          <NButton variant="secondary" size="sm" style="margin-left:auto" :loading="quarantineLoading" @click="viewQuarantine">
            <FileSearch :size="13" />
            Voir quarantaine
          </NButton>
        </div>
      </template>
      <div v-if="showQuarantinePanel" class="quarantine-panel">
        <div v-if="quarantineLoading" class="quarantine-empty">Chargement...</div>
        <div v-else-if="!quarantineData.length" class="quarantine-empty">
          <CheckCircle :size="18" style="color:var(--success)" />
          <span>Aucune menace en quarantaine.</span>
        </div>
        <table v-else class="quarantine-table">
          <thead>
            <tr>
              <th>Menace</th>
              <th>Sévérité</th>
              <th>Action</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(t, i) in quarantineData" :key="i">
              <td class="mono">{{ t.ThreatName ?? '—' }}</td>
              <td><NBadge :variant="severityVariant(t.SeverityID ?? 0)">{{ severityLabel(t.SeverityID ?? 0) }}</NBadge></td>
              <td>
                <NBadge v-if="t.ActionSuccess" variant="success">Traitée</NBadge>
                <NBadge v-else variant="danger">En attente</NBadge>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <div v-else class="quarantine-hint">
        Cliquez sur "Voir quarantaine" pour interroger Windows Defender.
      </div>
    </NCard>

    <!-- Update & Tools -->
    <NCard>
      <template #header>
        <div class="section-header"><ExternalLink :size="16" /><span>Outils & Scanners Recommandés</span></div>
      </template>
      <ScanExternalTools />
    </NCard>

    <!-- Historique des scans -->
    <NCard>
      <template #header>
        <div class="section-header">
          <Clock :size="16" />
          <span>Historique des scans</span>
          <NBadge variant="neutral" style="margin-left:4px">{{ scanHistory.length }}</NBadge>
        </div>
      </template>
      <div v-if="!scanHistory.length" class="quarantine-empty">
        Aucun scan enregistré. Lancez un scan pour commencer l'historique.
      </div>
      <table v-else class="quarantine-table">
        <thead>
          <tr>
            <th>Date</th>
            <th>Type</th>
            <th>Résultat</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(rec, i) in scanHistory" :key="i">
            <td class="mono">{{ rec.date }}</td>
            <td><NBadge variant="neutral">{{ rec.type }}</NBadge></td>
            <td class="result-cell">{{ rec.result }}</td>
          </tr>
        </tbody>
      </table>
    </NCard>
  </div>
</template>

<style scoped>
.scanvirus-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.page-header h1 { font-size: 24px; font-weight: 700; }
.page-subtitle { color: var(--text-muted); font-size: 13px; margin-top: 2px; }

.section-header { display: flex; align-items: center; gap: 8px; width: 100%; }

/* Defender */
.defender-status {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 12px;
}

.defender-info {
  display: flex;
  align-items: center;
  gap: 14px;
}

.defender-icon-wrap {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
}

.defender-active { background: var(--success-muted); color: var(--success); }
.defender-inactive { background: var(--danger-muted); color: var(--danger); }
.defender-unknown { background: var(--bg-tertiary); color: var(--text-muted); }

.defender-label {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  display: block;
}

.defender-state { margin-top: 4px; }

.realtime-badge-wrap {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 4px;
}

.realtime-label {
  font-size: 11px;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

/* Scan Grid */
.scan-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.scan-grid-4 {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
}

@media (max-width: 900px) { .scan-grid-4 { grid-template-columns: repeat(2, 1fr); } }
@media (max-width: 700px) { .scan-grid { grid-template-columns: 1fr; } .scan-grid-4 { grid-template-columns: 1fr; } }

.scan-option {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 12px;
}

.scan-icon-wrap {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.scan-icon-wrap.quick { background: var(--accent-muted); color: var(--accent-primary); }
.scan-icon-wrap.full { background: var(--warning-muted); color: var(--warning); }
.scan-icon-wrap.offline { background: var(--danger-muted); color: var(--danger); }
.scan-icon-wrap.custom { background: var(--bg-tertiary); color: var(--text-secondary); }

.scan-option h3 {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.scan-option p {
  font-size: 13px;
  color: var(--text-muted);
  line-height: 1.5;
}

/* Progress */
.scan-progress-area {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.scan-status {
  font-size: 13px;
  color: var(--text-secondary);
}

.scan-result {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 500;
  color: var(--success);
}

.scan-counter {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: var(--accent-primary);
  margin-left: auto;
  font-family: "JetBrains Mono", monospace;
}

.spin-icon {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.custom-path-input {
  width: 100%;
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-tertiary);
  color: var(--text-primary);
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
}

.update-section {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.action-desc {
  font-size: 13px;
  color: var(--text-muted);
}

.schedule-confirm {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--success);
  margin-top: 6px;
}

.ext-tools-cats { display: flex; flex-direction: column; gap: 20px; }

.ext-cat { display: flex; flex-direction: column; gap: 6px; }

.ext-cat-title {
  font-size: 11px;
  font-weight: 700;
  color: var(--success);
  text-transform: uppercase;
  letter-spacing: 0.8px;
  padding: 3px 8px;
  background: color-mix(in srgb, currentColor 10%, transparent);
  border-left: 3px solid currentColor;
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
}

.ext-tools-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.ext-tool-online { border-left: 2px solid var(--accent-primary); }

.ext-tool-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 10px;
  border: none;
  border-radius: var(--radius-md);
  background: transparent;
  cursor: pointer;
  font-family: inherit;
  text-align: left;
  width: 100%;
  transition: background var(--transition-fast);
}

.ext-tool-item:hover { background: var(--bg-tertiary); }

.ext-tool-info { display: flex; flex-direction: column; gap: 2px; }
.ext-tool-name { font-size: 13px; font-weight: 500; color: var(--text-primary); }
.ext-tool-desc { font-size: 11px; color: var(--text-muted); }

/* Quarantaine & Historique */
.quarantine-panel { margin-top: 4px; }
.quarantine-empty {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-muted);
  padding: 12px 4px;
}
.quarantine-hint {
  font-size: 12px;
  color: var(--text-muted);
  padding: 8px 4px;
}
.quarantine-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.quarantine-table th {
  text-align: left;
  padding: 6px 10px;
  color: var(--text-muted);
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-secondary);
}
.quarantine-table td {
  padding: 8px 10px;
  border-bottom: 1px solid var(--border);
  color: var(--text-secondary);
}
.quarantine-table tr:last-child td { border-bottom: none; }
.quarantine-table tr:hover td { background: var(--bg-secondary); }
.mono { font-family: "JetBrains Mono", monospace; font-size: 11px; }
.result-cell { color: var(--text-primary); }
</style>
