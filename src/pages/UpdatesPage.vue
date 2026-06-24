<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke, invokeRaw } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NTabs from "@/components/ui/NTabs.vue";
import NCollapse from "@/components/ui/NCollapse.vue";
import { useNotificationStore } from "@/stores/notifications";
import {
  RefreshCw, Download, Package,
  CheckCircle, AlertTriangle, ArrowUpCircle,
  Terminal, Shield, Calendar, Search, Play,
  EyeOff, RotateCcw, Clock, Settings, RotateCw, Zap,
} from "lucide-vue-next";
import { listen } from "@tauri-apps/api/event";

const notify = useNotificationStore();
const tabs = [
  { id: "winget", label: "WinGet" },
  { id: "chocolatey", label: "Chocolatey" },
  { id: "scoop", label: "Scoop" },
  { id: "windows", label: "Windows Update" },
  { id: "automation", label: "Automatisation" },
];
const activeTab = ref("winget");
// ─── Exclusions (persistées en localStorage) ────────────────────────────────
const LS_EXCLUDED = "nitrite-updates-excluded";
function parseLs<T>(key: string, fallback: T): T {
  try { const v = localStorage.getItem(key); return v ? JSON.parse(v) : fallback; }
  catch { return fallback; }
}
const excludedIds = ref<string[]>(parseLs<string[]>(LS_EXCLUDED, []));

function saveExclusions() {
  localStorage.setItem(LS_EXCLUDED, JSON.stringify(excludedIds.value));
}

function excludePkg(id: string) {
  if (!excludedIds.value.includes(id)) {
    excludedIds.value.push(id);
    saveExclusions();
  }
}

function reintegratePkg(id: string) {
  excludedIds.value = excludedIds.value.filter(x => x !== id);
  saveExclusions();
}

// ─── Schedule (persisté en localStorage) ────────────────────────────────────
const LS_SCHEDULE = "nitrite-updates-schedule";
interface UpdateSchedule { frequency: "off" | "daily" | "weekly"; time: string; }
const defaultSchedule: UpdateSchedule = { frequency: "off", time: "03:00" };
const schedule = ref<UpdateSchedule>(parseLs<UpdateSchedule>(LS_SCHEDULE, defaultSchedule));
const scheduleSaved = ref(false);

function saveSchedule() {
  localStorage.setItem(LS_SCHEDULE, JSON.stringify(schedule.value));
  scheduleSaved.value = true;
  setTimeout(() => (scheduleSaved.value = false), 2000);
  notify.success("Planning sauvegardé", `Fréquence : ${schedule.value.frequency} à ${schedule.value.time}`);
}

// ─── Backup avant MAJ ────────────────────────────────────────────────────────
const LS_BACKUP = "nitrite-updates-backup";
const backupBeforeUpdate = ref(localStorage.getItem(LS_BACKUP) === "true");

function toggleBackup() {
  backupBeforeUpdate.value = !backupBeforeUpdate.value;
  localStorage.setItem(LS_BACKUP, String(backupBeforeUpdate.value));
}

async function createRestorePoint(description: string) {
  try {
    await invoke("run_system_command", {
      cmd: "powershell",
      args: ["-Command", `Checkpoint-Computer -Description '${description}' -RestorePointType 'MODIFY_SETTINGS'`],
    });
    notify.success("Point de restauration créé", description);
  } catch (e: any) {
    notify.warning("Point de restauration", "Impossible de créer (droits admin ?)");
  }
}

// ─── Rollback dernière MAJ Windows ───────────────────────────────────────────
const rollbackConfirm = ref(false);
const rollbackRunning = ref(false);

async function rollbackLastWindowsUpdate() {
  if (!rollbackConfirm.value) {
    rollbackConfirm.value = true;
    setTimeout(() => (rollbackConfirm.value = false), 8000);
    return;
  }
  rollbackConfirm.value = false;
  rollbackRunning.value = true;
  try {
    await invoke("run_system_command", {
      cmd: "powershell",
      args: ["-Command", "Get-HotFix | Sort-Object InstalledOn -Descending | Select-Object -First 1 | ForEach-Object { wusa.exe /uninstall /kb:($_.HotFixID -replace 'KB','') /quiet /norestart }"],
    });
    notify.success("Rollback lancé", "Désinstallation de la dernière mise à jour Windows en cours");
  } catch (e: any) {
    notify.error("Rollback échoué", String(e));
  }
  rollbackRunning.value = false;
}

// ─── WinGet ──────────────────────────────────────────────────────────────────
interface WingetPackage { name: string; id: string; version: string; available: string; source: string; requires_restart?: boolean; }
const wingetOk = ref(false);
const wingetPkgs = ref<WingetPackage[]>([]);
const wingetStatus = ref<"idle" | "checking" | "updating" | "done" | "error">("idle");
const wingetLogs = ref<string[]>([]);
let unlistenLog: (() => void) | null = null;

const devWingetPkgs: WingetPackage[] = [
  { name: "Google Chrome", id: "Google.Chrome", version: "131.0", available: "132.0", source: "winget" },
  { name: "Visual Studio Code", id: "Microsoft.VisualStudioCode", version: "1.95.3", available: "1.96.1", source: "winget" },
  { name: "Git", id: "Git.Git", version: "2.43.0", available: "2.47.1", source: "winget", requires_restart: true },
];

// Packages visibles = non exclus
function visibleWinget() {
  return wingetPkgs.value.filter(p => !excludedIds.value.includes(p.id));
}
function excludedWinget() {
  return wingetPkgs.value.filter(p => excludedIds.value.includes(p.id));
}

async function checkWinget() {
  wingetStatus.value = "checking"; wingetPkgs.value = []; wingetLogs.value = [];
  try {
    wingetOk.value = await invoke<boolean>("check_winget");
    if (!wingetOk.value) { wingetStatus.value = "error"; return; }
    wingetPkgs.value = await invoke<WingetPackage[]>("list_upgradable");
    wingetStatus.value = wingetPkgs.value.length > 0 ? "idle" : "done";
  } catch {
    wingetOk.value = true;
    wingetPkgs.value = devWingetPkgs;
    wingetStatus.value = "idle";
  }
}

async function upgradeAllWinget() {
  if (backupBeforeUpdate.value) await createRestorePoint("NitritePreUpdate");
  wingetStatus.value = "updating"; wingetLogs.value = [];
  try {
    await invokeRaw("upgrade_all", { excluded_ids: excludedIds.value });
    wingetStatus.value = "done"; wingetPkgs.value = [];
    notify.success("WinGet", "Toutes les mises à jour installées");
  } catch {
    for (const pkg of wingetPkgs.value) {
      wingetLogs.value.push(`Mise à jour de ${pkg.name} (${pkg.version} → ${pkg.available})...`);
      await new Promise(r => setTimeout(r, 500));
    }
    wingetStatus.value = "done"; wingetPkgs.value = [];
  }
}

// ─── Chocolatey ──────────────────────────────────────────────────────────────
interface ChocoPackage { name: string; current_version: string; available_version: string; pinned: boolean; requires_restart?: boolean; }
const chocoOk = ref(false);
const chocoPkgs = ref<ChocoPackage[]>([]);
const chocoStatus = ref<"idle" | "checking" | "updating" | "done" | "error">("idle");
const chocoMsg = ref("");

function visibleChoco() { return chocoPkgs.value.filter(p => !excludedIds.value.includes(p.name)); }
function excludedChoco() { return chocoPkgs.value.filter(p => excludedIds.value.includes(p.name)); }

async function checkChocolatey() {
  chocoStatus.value = "checking"; chocoPkgs.value = []; chocoMsg.value = "";
  try {
    chocoOk.value = await invoke<boolean>("check_chocolatey");
    if (!chocoOk.value) { chocoStatus.value = "error"; chocoMsg.value = "Chocolatey non installé"; return; }
    chocoPkgs.value = await invoke<ChocoPackage[]>("list_chocolatey_upgrades");
    chocoStatus.value = chocoPkgs.value.length > 0 ? "idle" : "done";
    if (chocoPkgs.value.length === 0) chocoMsg.value = "Tous les paquets sont à jour";
  } catch {
    chocoOk.value = false; chocoStatus.value = "error";
    chocoMsg.value = "Chocolatey non disponible";
  }
}

async function upgradeAllChoco() {
  if (backupBeforeUpdate.value) await createRestorePoint("NitritePreUpdate");
  chocoStatus.value = "updating";
  try {
    const res = await invokeRaw<{ success: boolean; upgraded_count: number; message: string }>("upgrade_chocolatey_all");
    chocoStatus.value = "done"; chocoPkgs.value = [];
    chocoMsg.value = res.message;
    if (res.success) notify.success("Chocolatey", res.message);
    else notify.warning("Chocolatey", res.message);
  } catch (e: any) {
    chocoStatus.value = "error";
    chocoMsg.value = e?.toString() ?? "Erreur upgrade";
    notify.error("Chocolatey", chocoMsg.value);
  }
}

// ─── Scoop ───────────────────────────────────────────────────────────────────
interface ScoopPackage { name: string; installed: string; available: string; requires_restart?: boolean; }
const scoopOk = ref(false);
const scoopPkgs = ref<ScoopPackage[]>([]);
const scoopStatus = ref<"idle" | "checking" | "updating" | "done" | "error">("idle");
const scoopMsg = ref("");
const scoopLog = ref("");
const scoopInstalling = ref(false);
const scoopInstallMsg = ref("");
let unlistenScoop: (() => void) | null = null;

function visibleScoop() { return scoopPkgs.value.filter(p => !excludedIds.value.includes(p.name)); }
function excludedScoop() { return scoopPkgs.value.filter(p => excludedIds.value.includes(p.name)); }

async function checkScoop() {
  scoopStatus.value = "checking"; scoopPkgs.value = []; scoopMsg.value = ""; scoopLog.value = "";
  try {
    scoopOk.value = await invoke<boolean>("check_scoop");
    if (!scoopOk.value) { scoopStatus.value = "error"; return; }
    scoopPkgs.value = await invoke<ScoopPackage[]>("list_scoop_upgrades");
    scoopStatus.value = scoopPkgs.value.length > 0 ? "idle" : "done";
    if (scoopPkgs.value.length === 0) scoopMsg.value = "Tous les paquets sont à jour";
  } catch {
    scoopOk.value = false; scoopStatus.value = "error";
    scoopMsg.value = "Scoop non disponible";
  }
}

async function upgradeAllScoop() {
  if (backupBeforeUpdate.value) await createRestorePoint("NitritePreUpdate");
  scoopStatus.value = "updating"; scoopLog.value = ""; scoopMsg.value = "Mise à jour en cours...";
  if (unlistenScoop) unlistenScoop();
  unlistenScoop = await listen<string>("scoop-upgrade-done", (e) => {
    scoopLog.value = e.payload || "";
    scoopMsg.value = "Mise à jour Scoop terminée ✓";
    scoopStatus.value = "done";
    scoopPkgs.value = [];
  });
  try {
    await invokeRaw("upgrade_scoop_all");
  } catch (e: any) {
    scoopMsg.value = "Erreur : " + e;
    scoopStatus.value = "error";
  }
}

async function installScoop() {
  scoopInstalling.value = true;
  scoopInstallMsg.value = "Installation Scoop en cours (30-60s)...";
  try {
    const msg = await invokeRaw<string>("install_package_manager", { manager: "scoop" });
    scoopInstallMsg.value = msg || "Scoop installé — rechargez pour détecter.";
    setTimeout(() => checkScoop(), 6000);
  } catch (e: any) {
    scoopInstallMsg.value = "Erreur : " + String(e);
    notify.error("Scoop", String(e));
  }
  scoopInstalling.value = false;
}

// ─── Windows Update ───────────────────────────────────────────────────────────
interface WinUpdate { hotfix_id: string; description: string; installed_on: string; }
interface WuPendingUpdate { title: string; kbs: string; severity: string; size_mb: number; category: string; requires_restart?: boolean; }

const winUpdates = ref<WinUpdate[]>([]);
const winUpdateStatus = ref<"idle" | "checking" | "done" | "error">("idle");
const wuPending = ref<WuPendingUpdate[]>([]);
const wuPendingStatus = ref<"idle" | "searching" | "done" | "error">("idle");
const wuLogs = ref<string[]>([]);
const wuInstalling = ref(false);
let unlistenWu: (() => void) | null = null;

function visibleWuPending() { return wuPending.value.filter(u => !excludedIds.value.includes(u.kbs || u.title)); }
function excludedWuPending() { return wuPending.value.filter(u => excludedIds.value.includes(u.kbs || u.title)); }

async function checkWindowsUpdates() {
  winUpdateStatus.value = "checking"; winUpdates.value = [];
  try {
    winUpdates.value = await invoke<WinUpdate[]>("check_windows_updates");
    winUpdateStatus.value = "done";
  } catch {
    winUpdateStatus.value = "done";
    winUpdates.value = [
      { hotfix_id: "KB5046617", description: "Mise à jour de sécurité", installed_on: "2024-11-12" },
      { hotfix_id: "KB5044285", description: "Mise à jour cumulative", installed_on: "2024-10-08" },
      { hotfix_id: "KB890830", description: "Outil de suppression des logiciels malveillants", installed_on: "2024-10-08" },
    ];
  }
}

async function searchPendingUpdates() {
  wuPendingStatus.value = "searching"; wuPending.value = []; wuLogs.value = [];
  try {
    wuPending.value = await invoke<WuPendingUpdate[]>("search_pending_updates");
    wuPendingStatus.value = "done";
    if (wuPending.value.length === 0) notify.success("Windows Update", "Système à jour !");
  } catch (e: any) {
    wuPendingStatus.value = "error";
    notify.error("Windows Update", e?.toString() ?? "Erreur de recherche");
  }
}

async function installAllUpdates() {
  if (wuInstalling.value) return;
  if (backupBeforeUpdate.value) await createRestorePoint("NitritePreUpdate");
  wuInstalling.value = true; wuLogs.value = [];
  try {
    const { listen: listenEvt } = await import("@tauri-apps/api/event");
    unlistenWu = await listenEvt<string>("wu-log", (e) => { wuLogs.value.push(e.payload); });
    const ok = await invokeRaw<boolean>("install_windows_updates");
    if (ok) { notify.success("Windows Update", "Installation terminée"); wuPending.value = []; }
    else notify.warning("Windows Update", "Installation partielle ou erreur");
  } catch {
    wuLogs.value.push("[SIMULATION] Chrome: 131.0 → 132.0 ✓");
    wuLogs.value.push("[SIMULATION] KB5046617 installé ✓");
    wuLogs.value.push("[TERMINE] Simulation");
  } finally {
    wuInstalling.value = false;
    if (unlistenWu) { unlistenWu(); unlistenWu = null; }
  }
}

async function openWindowsUpdate() {
  try {
    await invoke("execute_tool", { command: "ms-settings:windowsupdate", isUrl: true });
  } catch { window.open("ms-settings:windowsupdate", "_blank"); }
}

function severityClass(s: string) {
  if (s === "Critical") return "sev-critical";
  if (s === "Important") return "sev-important";
  if (s === "Moderate") return "sev-moderate";
  return "sev-other";
}

onMounted(async () => {
  try {
    const { listen: listenEvt } = await import("@tauri-apps/api/event");
    unlistenLog = await listenEvt<string>("upgrade-log", (e) => { wingetLogs.value.push(e.payload); });
  } catch { /* dev */ }
  await checkWinget();
});

onUnmounted(() => {
  if (unlistenLog) unlistenLog();
  if (unlistenWu) unlistenWu();
  if (unlistenScoop) unlistenScoop();
});

async function launchSdi() {
  try {
    (window as any).__nitrite_sdi_active = true;
    setTimeout(() => { (window as any).__nitrite_sdi_active = false; }, 60000);
    await invoke("launch_sdi");
    notify.success("Snappy Driver Installer lancé");
  } catch (e) {
    (window as any).__nitrite_sdi_active = false;
    notify.error("SDI introuvable", String(e));
  }
}
</script>

<template>
  <div class="updates-page">
    <div class="page-header">
      <div>
        <h1>Mises à Jour</h1>
        <p class="page-subtitle">Gestion des mises à jour — WinGet, Chocolatey, Windows</p>
      </div>
      <NButton variant="secondary" size="sm" @click="launchSdi" title="Lancer Snappy Driver Installer pour mettre à jour les pilotes">
        <Zap :size="14" /> Snappy Driver
      </NButton>
    </div>

    <NTabs :tabs="tabs" v-model="activeTab">
      <template #default>

        <!-- ═══════════════════════ WinGet ═══════════════════════ -->
        <template v-if="activeTab === 'winget'">
          <div class="tab-actions">
            <NBadge :variant="wingetOk ? 'success' : 'neutral'"><Package :size="12" /> WinGet {{ wingetOk ? "OK" : "---" }}</NBadge>
            <NButton variant="secondary" size="sm" :loading="wingetStatus === 'checking'" :disabled="wingetStatus === 'updating'" @click="checkWinget">
              <RefreshCw :size="14" /> Scanner
            </NButton>
            <NButton v-if="visibleWinget().length" variant="primary" size="sm" :loading="wingetStatus === 'updating'" @click="upgradeAllWinget">
              <Download :size="14" /> Tout mettre à jour ({{ visibleWinget().length }})
            </NButton>
          </div>

          <div v-if="wingetStatus === 'checking'" class="loading-state"><NSpinner :size="24" /><p>Analyse en cours...</p></div>
          <div v-else-if="wingetStatus === 'error'" class="error-state"><AlertTriangle :size="24" /><p>WinGet non disponible</p></div>
          <div v-else-if="wingetStatus === 'done' && !wingetPkgs.length && !wingetLogs.length" class="done-state"><CheckCircle :size="32" class="ok" /><p>Tout est à jour</p></div>

          <NCard v-if="visibleWinget().length">
            <template #header>
              <div class="section-header"><ArrowUpCircle :size="16" /><span>Mises à jour disponibles</span><NBadge variant="warning" style="margin-left:auto">{{ visibleWinget().length }}</NBadge></div>
            </template>
            <div class="table-wrap">
              <table class="data-table">
                <thead><tr><th>Application</th><th>ID</th><th>Actuelle</th><th>Disponible</th><th>Statut</th><th></th></tr></thead>
                <tbody>
                  <tr v-for="pkg in visibleWinget()" :key="pkg.id">
                    <td class="pkg-name">{{ pkg.name }}</td>
                    <td class="font-mono">{{ pkg.id }}</td>
                    <td class="font-mono ver-old">{{ pkg.version }}</td>
                    <td class="font-mono ver-new">{{ pkg.available }}</td>
                    <td>
                      <NBadge v-if="pkg.requires_restart" variant="warning" class="restart-badge">
                        <Clock :size="10" /> Redémarrage requis
                      </NBadge>
                    </td>
                    <td class="col-action">
                      <button class="ignore-btn" title="Ignorer ce package" @click="excludePkg(pkg.id)">
                        <EyeOff :size="12" /> Ignorer
                      </button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </NCard>

          <!-- Section Ignorés -->
          <NCollapse v-if="excludedWinget().length" title="Ignorés" :count="excludedWinget().length" :default-open="false" variant="subtle" storage-key="updates-winget-excluded">
            <div class="table-wrap">
              <table class="data-table">
                <thead><tr><th>Application</th><th>ID</th><th>Actuelle</th><th>Disponible</th><th></th></tr></thead>
                <tbody>
                  <tr v-for="pkg in excludedWinget()" :key="pkg.id" class="excluded-row">
                    <td class="pkg-name muted">{{ pkg.name }}</td>
                    <td class="font-mono muted">{{ pkg.id }}</td>
                    <td class="font-mono ver-old">{{ pkg.version }}</td>
                    <td class="font-mono ver-old">{{ pkg.available }}</td>
                    <td class="col-action">
                      <button class="reintegrate-btn" @click="reintegratePkg(pkg.id)">
                        <RotateCcw :size="12" /> Réintégrer
                      </button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </NCollapse>

          <NCard v-if="wingetLogs.length">
            <template #header><div class="section-header"><Terminal :size="16" /><span>Journal</span></div></template>
            <div class="log-output">
              <div v-for="(l, i) in wingetLogs" :key="i" class="log-line">{{ l }}</div>
              <div v-if="wingetStatus === 'updating'" class="log-cursor" />
            </div>
          </NCard>
        </template>

        <!-- ═══════════════════════ Chocolatey ═══════════════════════ -->
        <template v-else-if="activeTab === 'chocolatey'">
          <div class="tab-actions">
            <NBadge :variant="chocoOk ? 'success' : 'neutral'"><Package :size="12" /> Chocolatey {{ chocoOk ? "OK" : "absent" }}</NBadge>
            <NButton variant="secondary" size="sm" :loading="chocoStatus === 'checking'" @click="checkChocolatey">
              <RefreshCw :size="14" /> Scanner
            </NButton>
            <NButton v-if="visibleChoco().length" variant="primary" size="sm" :loading="chocoStatus === 'updating'" @click="upgradeAllChoco">
              <Download :size="14" /> Tout mettre à jour ({{ visibleChoco().length }})
            </NButton>
          </div>

          <div v-if="chocoStatus === 'checking'" class="loading-state"><NSpinner :size="24" /><p>Scan Chocolatey...</p></div>
          <div v-else-if="chocoStatus === 'error'" class="error-state"><AlertTriangle :size="24" /><p>{{ chocoMsg }}</p></div>
          <div v-else-if="chocoStatus === 'done' && !chocoPkgs.length" class="done-state"><CheckCircle :size="32" class="ok" /><p>{{ chocoMsg || "Tout est à jour" }}</p></div>

          <NCard v-if="visibleChoco().length">
            <template #header>
              <div class="section-header"><ArrowUpCircle :size="16" /><span>Paquets Chocolatey à mettre à jour</span><NBadge variant="warning" style="margin-left:auto">{{ visibleChoco().length }}</NBadge></div>
            </template>
            <div class="table-wrap">
              <table class="data-table">
                <thead><tr><th>Paquet</th><th>Installé</th><th>Disponible</th><th>Pinné</th><th>Statut</th><th></th></tr></thead>
                <tbody>
                  <tr v-for="pkg in visibleChoco()" :key="pkg.name">
                    <td class="pkg-name">{{ pkg.name }}</td>
                    <td class="font-mono ver-old">{{ pkg.current_version }}</td>
                    <td class="font-mono ver-new">{{ pkg.available_version }}</td>
                    <td><NBadge v-if="pkg.pinned" variant="neutral">Pinné</NBadge></td>
                    <td>
                      <NBadge v-if="pkg.requires_restart" variant="warning" class="restart-badge">
                        <Clock :size="10" /> Redémarrage requis
                      </NBadge>
                    </td>
                    <td class="col-action">
                      <button class="ignore-btn" @click="excludePkg(pkg.name)"><EyeOff :size="12" /> Ignorer</button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </NCard>

          <NCollapse v-if="excludedChoco().length" title="Ignorés" :count="excludedChoco().length" :default-open="false" variant="subtle" storage-key="updates-choco-excluded">
            <div class="table-wrap">
              <table class="data-table">
                <thead><tr><th>Paquet</th><th>Installé</th><th>Disponible</th><th></th></tr></thead>
                <tbody>
                  <tr v-for="pkg in excludedChoco()" :key="pkg.name" class="excluded-row">
                    <td class="pkg-name muted">{{ pkg.name }}</td>
                    <td class="font-mono ver-old">{{ pkg.current_version }}</td>
                    <td class="font-mono ver-old">{{ pkg.available_version }}</td>
                    <td class="col-action">
                      <button class="reintegrate-btn" @click="reintegratePkg(pkg.name)"><RotateCcw :size="12" /> Réintégrer</button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </NCollapse>
        </template>

        <!-- ═══════════════════════ Scoop ═══════════════════════ -->
        <template v-else-if="activeTab === 'scoop'">
          <div class="tab-actions">
            <NBadge :variant="scoopOk ? 'success' : 'neutral'"><Package :size="12" /> Scoop {{ scoopOk ? "OK" : "absent" }}</NBadge>
            <NButton variant="secondary" size="sm" :loading="scoopStatus === 'checking'" @click="checkScoop">
              <RefreshCw :size="14" /> Scanner
            </NButton>
            <NButton v-if="visibleScoop().length" variant="primary" size="sm" :loading="scoopStatus === 'updating'" @click="upgradeAllScoop">
              <Download :size="14" /> Tout mettre à jour ({{ visibleScoop().length }})
            </NButton>
          </div>

          <div v-if="scoopStatus === 'checking'" class="loading-state"><NSpinner :size="24" /><p>Détection Scoop...</p></div>

          <!-- Non installé -->
          <div v-else-if="scoopStatus === 'error' && !scoopOk" style="display:flex;flex-direction:column;gap:12px;padding:20px 0">
            <div style="display:flex;align-items:center;gap:8px;font-size:14px;color:var(--text-secondary)">
              <AlertTriangle :size="16" style="color:var(--warning)" /> Scoop n'est pas installé sur ce PC.
            </div>
            <div style="font-size:12px;color:var(--text-muted);padding:10px 14px;background:var(--bg-secondary);border-radius:6px;border:1px solid var(--border)">
              Scoop est un gestionnaire de paquets en ligne de commande pour Windows.
              L'installation se fait <strong>sans droits administrateur</strong>.
            </div>
            <div style="display:flex;gap:8px;flex-wrap:wrap">
              <NButton variant="primary" size="sm" :loading="scoopInstalling" @click="installScoop">
                <Download :size="14" /> {{ scoopInstalling ? 'Installation (30-60s)...' : 'Installer Scoop' }}
              </NButton>
              <NButton variant="ghost" size="sm" @click="checkScoop"><RefreshCw :size="13" /> Vérifier à nouveau</NButton>
            </div>
            <div v-if="scoopInstallMsg" style="font-size:12px;padding:8px 12px;border-radius:5px;border-left:3px solid var(--accent-primary);background:var(--bg-secondary)">
              {{ scoopInstallMsg }}
            </div>
          </div>

          <!-- Installé -->
          <template v-else>
            <div v-if="scoopStatus === 'done' && !scoopPkgs.length && !scoopLog" class="done-state">
              <CheckCircle :size="32" class="ok" /><p>{{ scoopMsg || "Tous les paquets Scoop sont à jour" }}</p>
            </div>
            <span v-if="scoopMsg && scoopStatus !== 'done'" class="muted" style="font-size:12px;display:block;margin-bottom:8px">{{ scoopMsg }}</span>

            <NCard v-if="visibleScoop().length">
              <template #header>
                <div class="section-header"><ArrowUpCircle :size="16" /><span>Mises à jour disponibles</span><NBadge variant="warning" style="margin-left:auto">{{ visibleScoop().length }}</NBadge></div>
              </template>
              <div class="table-wrap">
                <table class="data-table">
                  <thead><tr><th>Paquet</th><th>Installé</th><th>Disponible</th><th>Statut</th><th></th></tr></thead>
                  <tbody>
                    <tr v-for="pkg in visibleScoop()" :key="pkg.name">
                      <td class="pkg-name"><Package :size="12" style="margin-right:5px;opacity:.5" />{{ pkg.name }}</td>
                      <td class="font-mono ver-old">{{ pkg.installed || "—" }}</td>
                      <td class="font-mono ver-new">{{ pkg.available || "—" }}</td>
                      <td>
                        <NBadge v-if="pkg.requires_restart" variant="warning" class="restart-badge">
                          <Clock :size="10" /> Redémarrage requis
                        </NBadge>
                      </td>
                      <td class="col-action">
                        <button class="ignore-btn" @click="excludePkg(pkg.name)"><EyeOff :size="12" /> Ignorer</button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </NCard>

            <NCollapse v-if="excludedScoop().length" title="Ignorés" :count="excludedScoop().length" :default-open="false" variant="subtle" storage-key="updates-scoop-excluded">
              <div class="table-wrap">
                <table class="data-table">
                  <thead><tr><th>Paquet</th><th>Installé</th><th>Disponible</th><th></th></tr></thead>
                  <tbody>
                    <tr v-for="pkg in excludedScoop()" :key="pkg.name" class="excluded-row">
                      <td class="pkg-name muted">{{ pkg.name }}</td>
                      <td class="font-mono ver-old">{{ pkg.installed || "—" }}</td>
                      <td class="font-mono ver-old">{{ pkg.available || "—" }}</td>
                      <td class="col-action">
                        <button class="reintegrate-btn" @click="reintegratePkg(pkg.name)"><RotateCcw :size="12" /> Réintégrer</button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </NCollapse>

            <NCard v-if="scoopLog">
              <template #header><div class="section-header"><Terminal :size="16" /><span>Résultat</span></div></template>
              <div class="log-output">{{ scoopLog }}</div>
            </NCard>
            <div v-if="scoopStatus === 'updating'" class="loading-state"><NSpinner :size="24" /><p>Mise à jour Scoop en cours...</p></div>
          </template>
        </template>

        <!-- ═══════════════════════ Windows Update ═══════════════════════ -->
        <template v-else-if="activeTab === 'windows'">
          <!-- Actions -->
          <div class="tab-actions">
            <NButton variant="primary" size="sm" :loading="wuPendingStatus === 'searching'" :disabled="wuInstalling" @click="searchPendingUpdates">
              <Search :size="14" /> Rechercher les mises à jour
            </NButton>
            <NButton v-if="visibleWuPending().length" variant="success" size="sm" :loading="wuInstalling" @click="installAllUpdates">
              <Play :size="14" /> Installer tout ({{ visibleWuPending().length }})
            </NButton>
            <NButton variant="secondary" size="sm" :loading="winUpdateStatus === 'checking'" @click="checkWindowsUpdates">
              <Calendar :size="14" /> Historique KB
            </NButton>
            <NButton variant="ghost" size="sm" @click="openWindowsUpdate">
              <Shield :size="14" /> Ouvrir Windows Update
            </NButton>
            <!-- Rollback -->
            <NButton
              :variant="rollbackConfirm ? 'danger' : 'ghost'"
              size="sm"
              :loading="rollbackRunning"
              @click="rollbackLastWindowsUpdate"
              :title="rollbackConfirm ? 'Cliquer à nouveau pour confirmer' : 'Annuler la dernière MAJ Windows'"
            >
              <RotateCw :size="14" />
              {{ rollbackConfirm ? "Confirmer annulation ?" : "Annuler dernière MAJ" }}
            </NButton>
          </div>

          <!-- Pending search states -->
          <div v-if="wuPendingStatus === 'searching'" class="loading-state"><NSpinner :size="24" /><p>Recherche des mises à jour en attente...</p></div>
          <div v-else-if="wuPendingStatus === 'error'" class="error-state"><AlertTriangle :size="24" /><p>Erreur — WUA COM non disponible (droits admin requis)</p></div>
          <div v-else-if="wuPendingStatus === 'done' && !wuPending.length" class="done-state"><CheckCircle :size="32" class="ok" /><p>Système à jour</p></div>

          <!-- Pending updates list -->
          <NCard v-if="visibleWuPending().length">
            <template #header>
              <div class="section-header">
                <ArrowUpCircle :size="16" /><span>Mises à jour disponibles</span>
                <NBadge variant="warning" style="margin-left:auto">{{ visibleWuPending().length }}</NBadge>
              </div>
            </template>
            <div class="table-wrap">
              <table class="data-table">
                <thead><tr><th>Titre</th><th>KB</th><th>Catégorie</th><th>Sévérité</th><th>Taille</th><th>Statut</th><th></th></tr></thead>
                <tbody>
                  <tr v-for="u in visibleWuPending()" :key="u.title">
                    <td class="pkg-name wu-title">{{ u.title }}</td>
                    <td class="font-mono">{{ u.kbs || "—" }}</td>
                    <td>{{ u.category }}</td>
                    <td><span :class="['sev-badge', severityClass(u.severity)]">{{ u.severity }}</span></td>
                    <td class="font-mono">{{ u.size_mb > 0 ? u.size_mb + ' MB' : '—' }}</td>
                    <td>
                      <NBadge v-if="u.requires_restart" variant="warning" class="restart-badge">
                        <Clock :size="10" /> Redémarrage requis
                      </NBadge>
                    </td>
                    <td class="col-action">
                      <button class="ignore-btn" @click="excludePkg(u.kbs || u.title)"><EyeOff :size="12" /> Ignorer</button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </NCard>

          <!-- WU Ignorés -->
          <NCollapse v-if="excludedWuPending().length" title="Mises à jour ignorées" :count="excludedWuPending().length" :default-open="false" variant="subtle" storage-key="updates-wu-excluded">
            <div class="table-wrap">
              <table class="data-table">
                <thead><tr><th>Titre</th><th>KB</th><th></th></tr></thead>
                <tbody>
                  <tr v-for="u in excludedWuPending()" :key="u.title" class="excluded-row">
                    <td class="pkg-name muted wu-title">{{ u.title }}</td>
                    <td class="font-mono muted">{{ u.kbs || "—" }}</td>
                    <td class="col-action">
                      <button class="reintegrate-btn" @click="reintegratePkg(u.kbs || u.title)"><RotateCcw :size="12" /> Réintégrer</button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </NCollapse>

          <!-- Install log -->
          <NCard v-if="wuLogs.length">
            <template #header><div class="section-header"><Terminal :size="16" /><span>Journal d'installation</span></div></template>
            <div class="log-output">
              <div v-for="(l, i) in wuLogs" :key="i" :class="['log-line', l.startsWith('[ERREUR]') ? 'log-err' : l.startsWith('[OK]') || l.startsWith('[TERMINE]') ? 'log-ok' : '']">{{ l }}</div>
              <div v-if="wuInstalling" class="log-cursor" />
            </div>
          </NCard>

          <!-- Hotfix history -->
          <NCard v-if="winUpdates.length">
            <template #header>
              <div class="section-header"><Calendar :size="16" /><span>Historique (30 derniers KB)</span></div>
            </template>
            <div class="table-wrap">
              <table class="data-table">
                <thead><tr><th>KB ID</th><th>Description</th><th>Date</th></tr></thead>
                <tbody>
                  <tr v-for="u in winUpdates" :key="u.hotfix_id">
                    <td class="font-mono pkg-name">{{ u.hotfix_id }}</td>
                    <td>{{ u.description }}</td>
                    <td class="font-mono ver-old">{{ u.installed_on }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </NCard>
          <div v-else-if="winUpdateStatus === 'checking'" class="loading-state"><NSpinner :size="24" /><p>Récupération de l'historique...</p></div>
        </template>

        <!-- ═══════════════════════ Automatisation ═══════════════════════ -->
        <template v-else-if="activeTab === 'automation'">
          <div class="automation-grid">

            <!-- Schedule -->
            <NCard>
              <template #header>
                <div class="section-header"><Clock :size="16" /><span>Planning de mise à jour</span></div>
              </template>
              <div class="automation-form">
                <div class="form-row">
                  <label class="form-label">Fréquence</label>
                  <select v-model="schedule.frequency" class="form-select">
                    <option value="off">Désactivé</option>
                    <option value="daily">Quotidien</option>
                    <option value="weekly">Hebdomadaire</option>
                  </select>
                </div>
                <div class="form-row" v-if="schedule.frequency !== 'off'">
                  <label class="form-label">Heure</label>
                  <input v-model="schedule.time" type="time" class="form-input" />
                </div>
                <div class="form-row" v-if="schedule.frequency !== 'off'">
                  <p class="schedule-info">
                    <Clock :size="12" />
                    Nitrite vérifiera les mises à jour chaque
                    {{ schedule.frequency === 'daily' ? 'jour' : 'semaine' }} à {{ schedule.time }}.
                    <br /><span class="muted" style="font-size:11px">Note : l'application doit être ouverte à l'heure planifiée.</span>
                  </p>
                </div>
                <div class="form-actions">
                  <NButton variant="primary" size="sm" @click="saveSchedule">
                    <Settings :size="13" /> Sauvegarder planning
                  </NButton>
                  <NBadge v-if="scheduleSaved" variant="success">Sauvegardé</NBadge>
                </div>
              </div>
            </NCard>

            <!-- Options -->
            <NCard>
              <template #header>
                <div class="section-header"><Shield :size="16" /><span>Options de sécurité</span></div>
              </template>
              <div class="automation-form">
                <!-- Backup avant MAJ -->
                <label class="checkbox-row" @click="toggleBackup">
                  <span class="custom-check" :class="{ checked: backupBeforeUpdate }">
                    <svg v-if="backupBeforeUpdate" width="10" height="10" viewBox="0 0 12 12"><path d="M2 6l3 3 5-5" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>
                  </span>
                  <span class="checkbox-label">Créer un point de restauration avant les mises à jour</span>
                </label>
                <p class="checkbox-hint">Un point de restauration système sera créé avant chaque batch de mises à jour (WinGet, Choco, Scoop, Windows Update).</p>

                <div class="form-divider" />

                <!-- Rollback explication -->
                <div class="rollback-info">
                  <RotateCw :size="14" style="color:var(--warning);flex-shrink:0" />
                  <div>
                    <p class="rollback-title">Annuler la dernière MAJ Windows</p>
                    <p class="rollback-desc">Disponible dans l'onglet <strong>Windows Update</strong>, cette action désinstalle silencieusement le dernier hotfix KB installé via <code>wusa.exe</code>. Un redémarrage sera nécessaire.</p>
                  </div>
                </div>
              </div>
            </NCard>

          </div>
        </template>

      </template>
    </NTabs>
  </div>
</template>

<style scoped src="./UpdatesPage.css"></style>
