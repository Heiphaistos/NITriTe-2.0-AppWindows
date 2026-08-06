<script setup lang="ts">
defineOptions({ name: "DiagnosticPage" });
import { ref, watch, onMounted, defineAsyncComponent } from "vue";
import { invoke } from "@/utils/invoke";
import { useRoute } from "vue-router";
import { cachedInvoke, refreshCached } from "@/composables/useCachedInvoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import DiagnosticToolsGrid from "@/components/shared/DiagnosticToolsGrid.vue";
import { useLayoutStore } from "@/stores/layoutStore";
import { useDiagnosticExport } from "@/composables/useDiagnosticExport";
import { TABS, GROUPS } from "@/data/diagnosticTabs";
import type {
  SysInfo, BiosInfo, MoboDetailed, GpuDetailed, RamDetailed, CpuCache,
  StoragePhysical, NetworkAdapter, MonitorDetail, AudioDevice, UsbDevice,
  BatteryDetailed, PowerPlan, PrinterDetail, InstalledSoftware,
  EnvVar, StartupProgram, InstalledUpdate, WinLicense,
  CpuExtendedInfo, FolderEntry, ProcessInfo, ServiceInfo, ScheduledTask,
  VolumeInfo, SmartDiskInfo, TcpConnection, WifiInfo, SecurityStatus,
  OsExtended, BiosExtended, MoboExtended,
} from "@/types/diagnostic";
import {
  FileDown, FolderOpen, ScanLine, RefreshCw, Search,
} from "lucide-vue-next";

// ── Lazy-loading onglets diagnostics ─────────────────────────────────────────
const DiagTabSystem       = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabSystem.vue"));
const DiagTabCPU          = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabCPU.vue"));
const DiagTabGPU          = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabGPU.vue"));
const DiagTabRAM          = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabRAM.vue"));
const DiagTabStorage      = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabStorage.vue"));
const DiagTabNetwork      = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabNetwork.vue"));
const DiagTabDevices      = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabDevices.vue"));
const DiagTabSoftware     = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabSoftware.vue"));
const DiagTabProcesses    = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabProcesses.vue"));
const DiagTabSecurity     = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabSecurity.vue"));
const DiagTabUpdates      = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabUpdates.vue"));
const DiagTabActivation   = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabActivation.vue"));
const DiagTabFolders      = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabFolders.vue"));
const DiagTabAccounts     = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabAccounts.vue"));
const DiagTabFirewall     = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabFirewall.vue"));
const DiagTabShares       = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabShares.vue"));
const DiagTabRegistry     = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabRegistry.vue"));
const DiagTabHistory      = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabHistory.vue"));
const DiagTabSysDrivers   = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabSysDrivers.vue"));
const DiagTabCertificates = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabCertificates.vue"));
const DiagTabPerf         = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabPerf.vue"));
const DiagTabNetTools     = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabNetTools.vue"));
const DiagTabRepair       = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabRepair.vue"));
const DiagTabBenchmark    = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabBenchmark.vue"));
const DiagTabCleaner      = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabCleaner.vue"));
const DiagTabBsod         = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabBsod.vue"));
const DiagTabHosts        = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabHosts.vue"));
const DiagTabBoot         = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabBoot.vue"));
const DiagTabWsl          = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabWsl.vue"));
const DiagTabBluetooth    = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabBluetooth.vue"));
const DiagTabPerfHistory  = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabPerfHistory.vue"));
const DiagTabScan         = defineAsyncComponent(() => import("@/components/diagnostic/DiagTabScan.vue"));

// ── Stores ────────────────────────────────────────────────────────────────────
import { useDiagnosticStore } from "@/stores/diagnosticStore";
const layoutStore = useLayoutStore();
const diagStore   = useDiagnosticStore();
const route       = useRoute();

// ── invokeCached local avec support force-refresh ─────────────────────────────
async function invokeCached<T = unknown>(cmd: string, args?: Record<string, unknown>, force = false): Promise<T> {
  if (force) return refreshCached<T>(cmd, args);
  return cachedInvoke<T>(cmd, args);
}

// ── Accordion nav ─────────────────────────────────────────────────────────────
const collapsedGroups = ref<Set<string>>(new Set());
function toggleGroup(groupId: string) {
  if (collapsedGroups.value.has(groupId)) collapsedGroups.value.delete(groupId);
  else collapsedGroups.value.add(groupId);
  collapsedGroups.value = new Set(collapsedGroups.value);
}

// ── Navigation state ──────────────────────────────────────────────────────────
const activeTab   = ref("os");
const activeGroup = ref("hardware");
const navFilter   = ref("");
const loadedTabs  = ref<Set<string>>(new Set());
const tabLoadTime = ref<Record<string, number>>({});
const loadingTab  = ref<string | null>(null);
const tabError    = ref<Record<string, string>>({});
const TAB_TTL_MS  = 5 * 60 * 1000;

function visibleTabsForGroup(groupId: string) {
  const f = navFilter.value.toLowerCase().trim();
  return TABS.filter(t => t.groupId === groupId && (!f || t.label.toLowerCase().includes(f)));
}
function isGroupActive(groupId: string) {
  return TABS.find(t => t.id === activeTab.value)?.groupId === groupId;
}

// ── Data refs ─────────────────────────────────────────────────────────────────
const sysInfo         = ref<SysInfo | null>(null);
const biosInfo        = ref<BiosInfo | null>(null);
const biosExtended    = ref<BiosExtended | null>(null);
const moboInfo        = ref<MoboDetailed | null>(null);
const moboExtended    = ref<MoboExtended | null>(null);
const cpuCache        = ref<CpuCache | null>(null);
const cpuExtended     = ref<CpuExtendedInfo | null>(null);
const osExtended      = ref<OsExtended | null>(null);
const gpuList         = ref<GpuDetailed[]>([]);
const ramData         = ref<RamDetailed | null>(null);
const storageList     = ref<StoragePhysical[]>([]);
const volumes         = ref<VolumeInfo[]>([]);
const networkAdapters = ref<NetworkAdapter[]>([]);
const connections     = ref<TcpConnection[]>([]);
const wifiInfo        = ref<WifiInfo | null>(null);
const monitors        = ref<MonitorDetail[]>([]);
const audioDevices    = ref<AudioDevice[]>([]);
const usbDevices      = ref<UsbDevice[]>([]);
const batteries       = ref<BatteryDetailed[]>([]);
const powerPlans      = ref<PowerPlan[]>([]);
const printers        = ref<PrinterDetail[]>([]);
const softwareList    = ref<InstalledSoftware[]>([]);
const envVars         = ref<EnvVar[]>([]);
const processes       = ref<ProcessInfo[]>([]);
const services        = ref<ServiceInfo[]>([]);
const startupPrograms = ref<StartupProgram[]>([]);
const scheduledTasks  = ref<ScheduledTask[]>([]);
const securityInfo    = ref<SecurityStatus | null>(null);
const licenseInfo     = ref<WinLicense | null>(null);
const updatesHistory  = ref<InstalledUpdate[]>([]);
const folders         = ref<FolderEntry[]>([]);
const smartData       = ref<SmartDiskInfo[]>([]);
const smartLoading    = ref(false);
const scanResult      = ref(null as import("@/types/diagnostic").ScanResult | null);
const scanProblems    = ref<string[]>([]);

// ── Composable export/scan ────────────────────────────────────────────────────
function navigateToScan() {
  activeTab.value = "scan";
  activeGroup.value = "tools";
  if (collapsedGroups.value.has("tools")) {
    collapsedGroups.value.delete("tools");
    collapsedGroups.value = new Set(collapsedGroups.value);
  }
}

const diagExport = useDiagnosticExport({
  sysInfo, biosInfo, moboInfo, cpuCache, gpuList, ramData,
  storageList, networkAdapters, monitors, audioDevices, usbDevices,
  batteries, powerPlans, printers, softwareList, envVars, startupPrograms,
  updatesHistory, licenseInfo, volumes, scanResult, scanProblems, loadedTabs,
}, navigateToScan);

const {
  scanning, scanProgress, scanStep,
  showExportModal, exportFormats, exportRunning, modalScanMode,
  openExportModal, confirmScanLaunch, toggleExportFormat, runExportSelected,
  runTotalScan, launchScanWithFormats,
  openExportFolder,
} = diagExport;

// ── Tab loader ────────────────────────────────────────────────────────────────
async function loadTab(tab: string, force = false) {
  const now = Date.now();
  const lastLoad = tabLoadTime.value[tab] ?? 0;
  const expired = now - lastLoad > TAB_TTL_MS;
  if (!force && loadedTabs.value.has(tab) && !expired) return;
  loadedTabs.value.add(tab);
  loadingTab.value = tab;
  tabError.value[tab] = "";
  try {
    switch (tab) {
      case "os":
        if (!sysInfo.value || force) sysInfo.value = await invokeCached<SysInfo>("get_system_info", undefined, force);
        if (!osExtended.value || force) osExtended.value = await invokeCached<OsExtended>("get_os_extended", undefined, force).catch(() => null);
        break;
      case "bios":
        [biosInfo.value, biosExtended.value] = await Promise.all([
          invokeCached<BiosInfo>("get_bios_info", undefined, force),
          invokeCached<BiosExtended>("get_bios_extended", undefined, force).catch(() => null),
        ]); break;
      case "mobo":
        [moboInfo.value, moboExtended.value] = await Promise.all([
          invokeCached<MoboDetailed>("get_motherboard_detailed", undefined, force),
          invokeCached<MoboExtended>("get_motherboard_extended", undefined, force).catch(() => null),
        ]); break;
      case "cpu":
        if (!sysInfo.value || force) sysInfo.value = await invokeCached<SysInfo>("get_system_info", undefined, force);
        [cpuCache.value, cpuExtended.value] = await Promise.all([
          invokeCached<CpuCache>("get_cpu_cache_info", undefined, force),
          invokeCached<CpuExtendedInfo>("get_cpu_extended", undefined, force).catch(() => null),
        ]); break;
      case "gpu":         gpuList.value       = await invokeCached<GpuDetailed[]>("get_gpu_detailed", undefined, force); break;
      case "ram":         ramData.value        = await invokeCached<RamDetailed>("get_ram_detailed", undefined, force); break;
      case "disks":
        storageList.value = await invokeCached<StoragePhysical[]>("get_storage_physical_info", undefined, force).catch(() => []);
        smartLoading.value = true;
        // smartLoading distingue "requête encore en vol" de "terminée mais vide" —
        // sans lui, DiagTabStorage.vue affichait "SMART en cours de chargement..."
        // indéfiniment dès que get_smart_info échouait ou revenait vide, un message
        // devenu factuellement faux (rien ne charge plus) qui laissait croire que
        // les données allaient encore arriver.
        invokeCached<SmartDiskInfo[]>("get_smart_info", undefined, force)
          .then(v => { smartData.value = v; })
          .catch(() => {})
          .finally(() => { smartLoading.value = false; });
        invokeCached<VolumeInfo[]>("get_logical_volumes", undefined, force).then(v => { volumes.value = v; loadedTabs.value.add("volumes"); }).catch(() => {});
        break;
      case "network":
        networkAdapters.value = await invokeCached<NetworkAdapter[]>("get_network_adapters_detailed", undefined, force);
        invokeCached<WifiInfo>("get_wifi_status").then(v => { wifiInfo.value = v; }).catch(() => {});
        break;
      case "connections":
        connections.value = await invokeCached<TcpConnection[]>("get_active_connections", undefined, force);
        wifiInfo.value = await invokeCached<WifiInfo>("get_wifi_status", undefined, force).catch(() => null);
        break;
      case "monitors": {
        monitors.value = await invokeCached<MonitorDetail[]>("get_monitor_info", undefined, force);
        invokeCached<unknown>("get_monitor_refresh_rates", undefined, force).then(rateData => {
          const rates: unknown[] = Array.isArray(rateData) ? rateData : (rateData ? [rateData] : []);
          if (!rates.length) return;
          monitors.value = (monitors.value as unknown[]).map((m, i) => {
            const rate = (rates[i] || rates[0]) as Record<string, number> | undefined;
            const mon = m as Record<string, unknown>;
            if (rate !== undefined && rate.refresh_rate_hz > 0 && (!mon.refresh_rate_hz || mon.refresh_rate_hz === 0))
              return { ...mon, refresh_rate_hz: rate.refresh_rate_hz };
            return mon;
          }) as unknown as MonitorDetail[];
        }).catch(() => {});
        break;
      }
      case "audio":    audioDevices.value  = await invokeCached<AudioDevice[]>("get_audio_devices", undefined, force); break;
      case "usb":      usbDevices.value    = await invokeCached<UsbDevice[]>("get_usb_devices", undefined, force); break;
      case "battery":  batteries.value     = await invokeCached<BatteryDetailed[]>("get_battery_detailed", undefined, force); break;
      case "power":    powerPlans.value    = await invokeCached<PowerPlan[]>("get_power_plans", undefined, force); break;
      case "printers": printers.value      = await invokeCached<PrinterDetail[]>("get_printers", undefined, force); break;
      case "software": softwareList.value  = await invokeCached<InstalledSoftware[]>("get_installed_software", undefined, force); break;
      case "env":      envVars.value       = await invokeCached<EnvVar[]>("get_environment_variables", undefined, force); break;
      case "processes":processes.value     = await invokeCached<ProcessInfo[]>("get_running_processes", undefined, force); break;
      case "services": services.value      = await invokeCached<ServiceInfo[]>("get_windows_services", undefined, force); break;
      case "startup":  startupPrograms.value = await invokeCached<StartupProgram[]>("get_startup_programs_detailed", undefined, force); break;
      case "tasks":    scheduledTasks.value = await invokeCached<ScheduledTask[]>("get_scheduled_tasks", undefined, force); break;
      case "security": securityInfo.value  = await invokeCached<SecurityStatus>("get_security_status", undefined, force).catch(() => null); break;
      case "license":  licenseInfo.value   = await invokeCached<WinLicense>("get_windows_license", undefined, force); break;
      case "updates":  updatesHistory.value = await invokeCached<InstalledUpdate[]>("get_installed_updates", undefined, force); break;
      case "folders":  folders.value       = await invoke<FolderEntry[]>("get_folder_sizes_detailed"); break;
      // Ces onglets chargent leurs données eux-mêmes
      case "activation": case "comptes": case "parefeu": case "partages":
      case "registre": case "historique": case "pilotes": case "certificats":
      case "performances": case "outils-reseau": case "reparation": case "benchmark":
      case "nettoyeur": case "bsod": case "hosts": case "boot": case "wsl":
      case "bluetooth": case "perf-history": break;
    }
    tabLoadTime.value[tab] = Date.now();
  } catch (e: unknown) {
    tabError.value[tab] = String(e) ?? "Erreur inconnue";
    loadedTabs.value.delete(tab);
  } finally { loadingTab.value = null; }
}

async function refreshTab() {
  const tab = activeTab.value;
  loadedTabs.value.delete(tab);
  tabLoadTime.value[tab] = 0;
  await loadTab(tab, true);
}

onMounted(() => {
  // Précharge les données partagées (cache utilisé par Dashboard, etc.)
  diagStore.prefetchAll().catch(() => {});

  const tabParam = route.query.tab as string | undefined;
  if (tabParam && TABS.find(t => t.id === tabParam)) {
    const grp = TABS.find(t => t.id === tabParam)?.groupId;
    if (grp) activeGroup.value = grp;
    activeTab.value = tabParam;
    loadTab(tabParam);
  } else {
    loadTab("os");
  }
});

watch(() => route.query.tab, (tabParam) => {
  if (typeof tabParam === "string" && TABS.find(t => t.id === tabParam)) {
    const grp = TABS.find(t => t.id === tabParam)?.groupId;
    if (grp) activeGroup.value = grp;
    activeTab.value = tabParam;
    loadTab(tabParam);
  }
});

watch(activeTab, (tab) => {
  if (tab !== "tools" && tab !== "scan") loadTab(tab);
});
</script>

<template>
  <div class="page-content">
    <!-- Header -->
    <div class="diag-page-header">
      <div class="diag-page-title">
        <h2>Diagnostic Système</h2>
        <span class="diag-page-subtitle">{{ TABS.length }} analyses disponibles</span>
      </div>
      <div class="diag-exports">
        <NButton variant="ghost" size="sm" @click="openExportModal(null)"><FileDown :size="13" />Exporter</NButton>
        <NButton variant="ghost" size="sm" @click="openExportFolder"><FolderOpen :size="13" />Exports</NButton>
        <NButton variant="primary" size="sm" :loading="scanning" @click="() => { navigateToScan(); runTotalScan(); }">
          <ScanLine :size="13" />Scan PC
        </NButton>
      </div>
    </div>

    <!-- Layout principal -->
    <div class="diag-layout" :data-nav="layoutStore.state.groupNavStyle">

      <!-- Panneau navigation latéral -->
      <nav class="diag-sidenav">
        <div class="diag-sidenav-filter">
          <Search :size="12" class="sidenav-filter-icon" />
          <input v-model="navFilter" class="sidenav-filter-input" placeholder="Filtrer les onglets…" />
          <button v-if="navFilter" class="sidenav-filter-clear" @click="navFilter = ''">✕</button>
        </div>
        <div class="sidenav-scroll">
          <div
            v-for="group in GROUPS" :key="group.id"
            class="sidenav-group"
            :class="{
              'sidenav-group--active': isGroupActive(group.id),
              'sidenav-group--collapsed': collapsedGroups.has(group.id),
            }"
          >
            <template v-if="visibleTabsForGroup(group.id).length > 0">
              <div class="sidenav-group-label" @click="toggleGroup(group.id)">
                <component :is="group.icon" :size="11" />
                {{ group.label }}
                <span class="sidenav-group-count">{{ visibleTabsForGroup(group.id).length }}</span>
              </div>
              <div class="sidenav-items-wrapper">
                <button
                  v-for="tab in visibleTabsForGroup(group.id)" :key="tab.id"
                  class="sidenav-item" :class="{ active: activeTab === tab.id }"
                  :title="tab.label" @click="activeTab = tab.id"
                >
                  <component :is="tab.icon" :size="14" class="sidenav-item-icon" />
                  <span class="sidenav-item-label">{{ tab.label }}</span>
                </button>
              </div>
            </template>
          </div>
        </div>
      </nav>

      <!-- Zone de contenu -->
      <div class="diag-content-area">
        <div class="diag-tab-breadcrumb">
          <span class="breadcrumb-group">{{ GROUPS.find(g => g.id === TABS.find(t => t.id === activeTab)?.groupId)?.label }}</span>
          <span class="breadcrumb-sep">›</span>
          <span class="breadcrumb-tab">{{ TABS.find(t => t.id === activeTab)?.label }}</span>
          <button class="breadcrumb-refresh" @click="refreshTab" title="Rafraîchir"><RefreshCw :size="12" /></button>
        </div>

        <NCard style="padding:16px">
          <div v-if="loadingTab === activeTab" style="display:flex;align-items:center;gap:10px;padding:24px 0;color:var(--text-secondary)">
            <NSpinner :size="18" /><span>Chargement {{ activeTab }}...</span>
          </div>
          <div v-else-if="tabError[activeTab]" style="color:var(--error);padding:16px 0;font-size:13px">
            ⚠ {{ tabError[activeTab] }}
            <NButton variant="ghost" size="sm" style="margin-left:12px" @click="refreshTab">Réessayer</NButton>
          </div>
          <template v-else>
            <DiagTabSystem
              v-if="activeTab === 'os' || activeTab === 'bios' || activeTab === 'mobo'"
              :tab="activeTab" :sysInfo="sysInfo" :biosInfo="biosInfo"
              :moboInfo="moboInfo" :osExtended="osExtended"
              :biosExtended="biosExtended" :moboExtended="moboExtended"
            />
            <DiagTabCPU    v-else-if="activeTab === 'cpu'"   :sysInfo="sysInfo" :cpuCache="cpuCache" :cpuExtended="cpuExtended" />
            <DiagTabGPU    v-else-if="activeTab === 'gpu'"   :gpuList="gpuList" />
            <DiagTabRAM    v-else-if="activeTab === 'ram'"   :ramData="ramData" :sysInfo="sysInfo" />
            <DiagTabStorage
              v-else-if="activeTab === 'disks'"
              :tab="activeTab" :storageList="storageList" :volumes="volumes" :smartData="smartData"
              :smartLoading="smartLoading"
            />
            <DiagTabNetwork
              v-else-if="activeTab === 'network' || activeTab === 'connections'"
              :tab="activeTab" :networkAdapters="networkAdapters" :connections="connections" :wifiInfo="wifiInfo"
            />
            <DiagTabDevices
              v-else-if="['monitors','audio','usb','battery','power','printers'].includes(activeTab)"
              :tab="activeTab" :monitors="monitors" :audioDevices="audioDevices"
              :usbDevices="usbDevices" :batteries="batteries" :powerPlans="powerPlans" :printers="printers"
            />
            <DiagTabSoftware
              v-else-if="activeTab === 'software' || activeTab === 'env'"
              :tab="activeTab" :softwareList="softwareList" :envVars="envVars" @refresh="refreshTab"
            />
            <DiagTabProcesses
              v-else-if="['processes','services','startup','tasks'].includes(activeTab)"
              :tab="activeTab" :processes="processes" :services="services"
              :startupPrograms="startupPrograms" :tasks="scheduledTasks" :onRefresh="refreshTab"
            />
            <DiagTabSecurity
              v-else-if="activeTab === 'security' || activeTab === 'license'"
              :tab="activeTab" :securityInfo="securityInfo" :licenseInfo="licenseInfo" :updatesHistory="updatesHistory"
            />
            <DiagTabUpdates      v-else-if="activeTab === 'updates'"      :updatesHistory="updatesHistory" />
            <DiagTabActivation   v-else-if="activeTab === 'activation'" />
            <DiagTabAccounts     v-else-if="activeTab === 'comptes'" />
            <DiagTabFirewall     v-else-if="activeTab === 'parefeu'" />
            <DiagTabShares       v-else-if="activeTab === 'partages'" />
            <DiagTabRegistry     v-else-if="activeTab === 'registre'" />
            <DiagTabHistory      v-else-if="activeTab === 'historique'" />
            <DiagTabSysDrivers   v-else-if="activeTab === 'pilotes'" />
            <DiagTabCertificates v-else-if="activeTab === 'certificats'" />
            <DiagTabPerf         v-else-if="activeTab === 'performances'" />
            <DiagTabNetTools     v-else-if="activeTab === 'outils-reseau'" />
            <DiagTabRepair       v-else-if="activeTab === 'reparation'" />
            <DiagTabBenchmark    v-else-if="activeTab === 'benchmark'" />
            <DiagTabCleaner      v-else-if="activeTab === 'nettoyeur'" />
            <DiagTabBsod         v-else-if="activeTab === 'bsod'" />
            <DiagTabHosts        v-else-if="activeTab === 'hosts'" />
            <DiagTabBoot         v-else-if="activeTab === 'boot'" />
            <DiagTabWsl          v-else-if="activeTab === 'wsl'" />
            <DiagTabBluetooth    v-else-if="activeTab === 'bluetooth'" />
            <DiagTabPerfHistory  v-else-if="activeTab === 'perf-history'" />
            <DiagTabFolders      v-else-if="activeTab === 'folders'" :folders="folders" />
            <DiagnosticToolsGrid v-else-if="activeTab === 'tools'" />
            <DiagTabScan
              v-else-if="activeTab === 'scan'"
              :scanning="scanning" :scanProgress="scanProgress" :scanStep="scanStep"
              :scanResult="scanResult" :scanProblems="scanProblems" :batteries="batteries"
              :onRunScan="runTotalScan"
              :onLaunchTotal="(fmts: Set<string>) => launchScanWithFormats(fmts)"
            />
          </template>
        </NCard>
      </div>
    </div>

    <!-- Modal Export -->
    <Teleport to="body">
      <div v-if="showExportModal" class="export-modal-backdrop" @click.self="showExportModal = false">
        <div class="export-modal">
          <div class="export-modal-header">
            <h3>{{ modalScanMode === 'total' ? 'Scan Complet du Système' : 'Exporter le rapport' }}</h3>
            <button class="export-modal-close" @click="showExportModal = false">✕</button>
          </div>
          <p class="export-modal-desc">{{ modalScanMode ? "Formats d'export automatique après le scan." : "Choisissez un ou plusieurs formats." }}</p>
          <div class="export-modal-formats">
            <button
              v-for="fmt in ['html', 'txt', 'md', 'json'] as const" :key="fmt"
              class="export-fmt-btn" :class="{ selected: exportFormats.has(fmt) }"
              @click="toggleExportFormat(fmt)"
            >
              <span class="export-fmt-icon">{{ fmt === 'html' ? '🌐' : fmt === 'txt' ? '📄' : fmt === 'md' ? '📝' : '{ }' }}</span>
              <span class="export-fmt-label">{{ fmt.toUpperCase() }}</span>
              <span v-if="exportFormats.has(fmt)" class="export-fmt-check">✓</span>
            </button>
          </div>
          <div class="export-modal-footer">
            <NButton variant="ghost" size="sm" @click="openExportFolder"><FolderOpen :size="13" />Voir les exports</NButton>
            <NButton v-if="!modalScanMode" variant="primary" size="sm" :loading="exportRunning" @click="runExportSelected">
              <FileDown :size="13" />Exporter {{ exportFormats.size > 1 ? `(${exportFormats.size} formats)` : '' }}
            </NButton>
            <NButton v-else variant="primary" size="sm" :loading="scanning" @click="confirmScanLaunch">
              <ScanLine :size="13" />Lancer Scan Complet{{ exportFormats.size > 1 ? ` — ${exportFormats.size} formats` : '' }}
            </NButton>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.export-modal-backdrop {
  position: fixed; inset: 0; background: rgba(0,0,0,0.6); backdrop-filter: blur(4px);
  display: flex; align-items: center; justify-content: center; z-index: 9999;
}
.export-modal {
  background: var(--surface-2, #161625); border: 1px solid var(--border, #2e2e33);
  border-radius: 12px; padding: 24px; width: 380px; max-width: 90vw;
  box-shadow: 0 20px 60px rgba(0,0,0,0.5);
}
.export-modal-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }
.export-modal-header h3 { color: var(--text-primary, #e2e8f0); font-size: 15px; font-weight: 600; margin: 0; }
.export-modal-close { background: none; border: none; color: var(--text-muted, #64748b); cursor: pointer; font-size: 16px; padding: 2px 6px; border-radius: 4px; }
.export-modal-close:hover { background: var(--surface-3, #1e1e35); }
.export-modal-desc { color: var(--text-muted, #64748b); font-size: 12px; margin: 0 0 16px; }
.export-modal-formats { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-bottom: 20px; }
.export-fmt-btn {
  display: flex; align-items: center; gap: 8px; padding: 12px 14px;
  background: var(--surface-1, #0d0d1a); border: 1px solid var(--border, #2e2e33);
  border-radius: 8px; cursor: pointer; color: var(--text-secondary, #94a3b8);
  font-size: 13px; font-weight: 500; transition: all 0.15s;
}
.export-fmt-btn:hover { border-color: var(--accent-primary, #7c9af5); color: var(--text-primary, #e2e8f0); }
.export-fmt-btn.selected { border-color: var(--accent-primary, #7c9af5); background: rgba(124,154,245,0.08); color: var(--text-primary, #e2e8f0); }
.export-fmt-icon { font-size: 16px; }
.export-fmt-label { flex: 1; font-family: 'Consolas', monospace; }
.export-fmt-check { color: var(--accent-primary, #7c9af5); font-weight: 700; }
.export-modal-footer { display: flex; gap: 8px; justify-content: flex-end; }
</style>
