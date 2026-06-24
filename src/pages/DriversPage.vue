<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NSearchBar from "@/components/ui/NSearchBar.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NTabs from "@/components/ui/NTabs.vue";
import DiagTabSysDrivers from "@/components/diagnostic/DiagTabSysDrivers.vue";
import { useNotificationStore } from "@/stores/notifications";
import {
  Cpu, RefreshCw, Download, FileSpreadsheet, FileText,
  FolderOpen, ExternalLink, CheckCircle, XCircle,
  AlertTriangle, ToggleLeft, ToggleRight, RotateCcw, Zap, ScanSearch,
} from "lucide-vue-next";

type MainTab = "list" | "diagnostics";
const mainTab = ref<MainTab>("list");

const notifications = useNotificationStore();
const loading = ref(true);
const search = ref("");
const activeDriverCategory = ref("all");

const driverCategoryTabs = [
  { id: "all", label: "Tous" },
  { id: "gpu", label: "GPU" },
  { id: "audio", label: "Audio" },
  { id: "usb", label: "USB" },
  { id: "wifi", label: "WiFi" },
  { id: "reseau", label: "Reseau" },
  { id: "bluetooth", label: "Bluetooth" },
  { id: "stockage", label: "Stockage" },
  { id: "systeme", label: "Systeme" },
  { id: "autre", label: "Autre" },
];

// Drivers considérés comme critiques (regex sur module + displayName)
const CRITICAL_PATTERN = /nvidia|geforce|radeon|amd.*video|ati.*display|nvlddmkm|tcpip|ndis|ethernet|lan|wifi|wlan|wireless|ntfs|disk|nvme|sata|ahci|acpi|hal\b|realtek.*audio|hdaudio/i;

function isCriticalDriver(module: string, displayName: string): boolean {
  return CRITICAL_PATTERN.test(module + " " + displayName);
}

function getDriverCategory(module: string, displayName: string): string {
  const m = (module + " " + displayName).toLowerCase();
  if (/nvidia|geforce|quadro|nvlddmkm|dxgkrnl|nvdmapx|amd.*video|radeon|ati.*display/.test(m)) return "gpu";
  if (/realtek.*audio|audio|sound|ac97|hdaudio|portcls|drmk|ks.*filter|splitter|wave/.test(m)) return "audio";
  if (/usb|usbhub|xhci|ehci|ohci|uhci|usbstor|usbvideo|usbprint|wusb/.test(m)) return "usb";
  if (/wifi|wlan|wireless|802\.11/.test(m)) return "wifi";
  if (/tcpip|ndis|ethernet|netbt|afd|lan|e1000|rtl8|rl8|broadcom.*net|intel.*net|marvell|bfe|nsi/.test(m)) return "reseau";
  if (/bluetooth|bth|rfcomm|bthport/.test(m)) return "bluetooth";
  if (/disk|nvme|sata|ahci|storahci|stornvme|ide|atapi|ntfs|fat|exfat|refs|cdrom|storport|raid/.test(m)) return "stockage";
  if (/acpi|pci|smbus|gpio|i2c|isa|wdf|clfs|hal\b|kernel|nt\b|wmi|power/.test(m)) return "systeme";
  return "autre";
}

interface DriverEntry {
  module: string;
  displayName: string;
  driverType: string;
  linkDate: string;
  state: string;
  provider?: string;
  instance_id?: string;
  device_id?: string;
}

const drivers = ref<DriverEntry[]>([]);
const togglingIds = ref<string[]>([]);
const rollingBackIds = ref<string[]>([]);

// Drivers problématiques (Error / Degraded)
const problematicDrivers = computed(() =>
  drivers.value.filter(d =>
    /error|degraded/i.test(d.state)
  )
);

const filteredDrivers = computed(() => {
  let result = drivers.value;
  if (activeDriverCategory.value !== "all") {
    result = result.filter(d => getDriverCategory(d.module, d.displayName) === activeDriverCategory.value);
  }
  const q = search.value.toLowerCase();
  if (q) {
    result = result.filter(d =>
      d.module.toLowerCase().includes(q) ||
      d.displayName.toLowerCase().includes(q) ||
      d.driverType.toLowerCase().includes(q) ||
      d.state.toLowerCase().includes(q) ||
      (d.provider ?? "").toLowerCase().includes(q)
    );
  }
  return result;
});

function parseCSV(csv: string): DriverEntry[] {
  const lines = csv.trim().split("\n").filter((l) => l.trim());
  if (lines.length < 2) return [];

  const result: DriverEntry[] = [];
  for (let i = 1; i < lines.length; i++) {
    const cols = lines[i].match(/("([^"]*)"|[^,]+)/g)?.map((c) => c.replace(/^"|"$/g, "").trim()) ?? [];
    if (cols.length >= 5) {
      result.push({
        module: cols[0] ?? "",
        displayName: cols[1] ?? "",
        driverType: cols[3] ?? "",
        linkDate: cols[5] ?? cols[4] ?? "",
        state: cols[2] ?? "",
        provider: cols[6] ?? undefined,
      });
    }
  }
  return result;
}

async function loadDrivers() {
  loading.value = true;
  try {
    const result = await invoke<any>("run_system_command", {
      cmd: "driverquery",
      args: ["/FO", "CSV", "/V"],
    });
    const out = result?.stdout ?? result?.output ?? "";
    drivers.value = parseCSV(out);
  } catch {
    // Données de démo
    drivers.value = [
      { module: "1394ohci", displayName: "1394 OHCI Compliant Host Controller", driverType: "Kernel", linkDate: "21/06/2006", state: "Stopped" },
      { module: "3ware", displayName: "3ware", driverType: "Kernel", linkDate: "17/05/2015", state: "Stopped" },
      { module: "ACPI", displayName: "Microsoft ACPI Driver", driverType: "Kernel", linkDate: "21/06/2006", state: "Running", provider: "Microsoft" },
      { module: "AcpiDev", displayName: "ACPI Devices driver", driverType: "Kernel", linkDate: "21/06/2006", state: "Stopped", provider: "Microsoft" },
      { module: "acpiex", displayName: "Microsoft ACPIEx Driver", driverType: "Kernel", linkDate: "21/06/2006", state: "Running", provider: "Microsoft" },
      { module: "AFD", displayName: "Ancillary Function Driver", driverType: "Kernel", linkDate: "11/03/2024", state: "Running", provider: "Microsoft" },
      { module: "ahcache", displayName: "Application Compatibility Cache", driverType: "Kernel", linkDate: "21/06/2006", state: "Running" },
      { module: "amdgpio2", displayName: "AMD GPIO Client Driver", driverType: "Kernel", linkDate: "01/09/2023", state: "Running", provider: "AMD" },
      { module: "amdi2c", displayName: "AMD I2C Controller Driver", driverType: "Kernel", linkDate: "01/09/2023", state: "Stopped", provider: "AMD" },
      { module: "AmdK8", displayName: "AMD K8 Processor Driver", driverType: "Kernel", linkDate: "21/06/2006", state: "Stopped" },
      { module: "Beep", displayName: "Beep", driverType: "Kernel", linkDate: "21/06/2006", state: "Running" },
      { module: "BthA2dp", displayName: "Microsoft Bluetooth A2dp driver", driverType: "Kernel", linkDate: "21/06/2006", state: "Stopped", provider: "Microsoft" },
      { module: "CLFS", displayName: "Common Log (CLFS)", driverType: "Kernel", linkDate: "21/06/2006", state: "Running", provider: "Microsoft" },
      { module: "disk", displayName: "Disk Driver", driverType: "Kernel", linkDate: "21/06/2006", state: "Running", provider: "Microsoft" },
      { module: "HTTP", displayName: "HTTP Service", driverType: "Kernel", linkDate: "21/06/2006", state: "Running", provider: "Microsoft" },
      { module: "intelppm", displayName: "Intel Processor Driver", driverType: "Kernel", linkDate: "21/06/2006", state: "Stopped", provider: "Intel" },
      { module: "Ndu", displayName: "Network Data Usage Monitoring Driver", driverType: "Kernel", linkDate: "21/06/2006", state: "Running" },
      { module: "Ntfs", displayName: "Ntfs", driverType: "File System", linkDate: "21/06/2006", state: "Running", provider: "Microsoft" },
      { module: "nvlddmkm", displayName: "NVIDIA Windows Kernel Mode Driver", driverType: "Kernel", linkDate: "15/01/2025", state: "Running", provider: "NVIDIA" },
      { module: "Tcpip", displayName: "TCP/IP Protocol Driver", driverType: "Kernel", linkDate: "21/06/2006", state: "Running", provider: "Microsoft" },
    ];
  }
  loading.value = false;
}

async function getExportFolder(): Promise<string> {
  try {
    const { homeDir, join } = await import("@tauri-apps/api/path");
    return await join(await homeDir(), "Documents", "NiTriTe", "exports");
  } catch { return ""; }
}

async function ensureExportDir(folder: string) {
  if (!folder) return;
  try { const { mkdir } = await import("@tauri-apps/plugin-fs"); await mkdir(folder, { recursive: true }); } catch { /* ignore */ }
}

async function openExportFolder() {
  const folder = await getExportFolder();
  if (!folder) return;
  await ensureExportDir(folder);
  try {
    await invoke("open_path", { path: folder });
  } catch { notifications.error("Impossible d'ouvrir le dossier"); }
}

async function writeToExports(filename: string, content: string) {
  const folder = await getExportFolder();
  try {
    await ensureExportDir(folder);
    const { writeTextFile } = await import("@tauri-apps/plugin-fs");
    const { join } = await import("@tauri-apps/api/path");
    const fullPath = folder ? await join(folder, filename) : filename;
    await writeTextFile(fullPath, content);
    notifications.success(`Export ${filename} sauvegardé`);
  } catch {
    navigator.clipboard.writeText(content);
    notifications.info("Copié dans le presse-papier", "Export fichier indisponible en mode dev");
  }
}

async function exportCSV() {
  const header = "Module,Nom,Type,Date,Etat,Provider";
  const rows = filteredDrivers.value.map(
    (d) => `"${d.module}","${d.displayName}","${d.driverType}","${d.linkDate}","${d.state}","${d.provider ?? ""}"`
  );
  await writeToExports("drivers.csv", [header, ...rows].join("\n"));
}

async function exportTxt() {
  const col = (s: string, w: number) => s.padEnd(w).slice(0, w);
  const header = `${col("Module", 25)} ${col("Nom", 40)} ${col("Type", 15)} ${col("Date", 12)} ${col("Etat", 12)} Provider`;
  const sep = "-".repeat(120);
  const rows = filteredDrivers.value.map(d =>
    `${col(d.module, 25)} ${col(d.displayName, 40)} ${col(d.driverType, 15)} ${col(d.linkDate, 12)} ${col(d.state, 12)} ${d.provider ?? ""}`
  );
  await writeToExports("drivers.txt", [header, sep, ...rows].join("\n"));
}

function stateVariant(state: string): "success" | "warning" | "neutral" | "danger" {
  if (state.toLowerCase().includes("running")) return "success";
  if (state.toLowerCase().includes("stopped")) return "neutral";
  if (/error|degraded/i.test(state)) return "danger";
  return "warning";
}

function parseDriverDate(dateStr: string): Date | null {
  const m = dateStr.match(/(\d{1,2})\/(\d{1,2})\/(\d{4})/);
  if (!m) return null;
  return new Date(parseInt(m[3]), parseInt(m[2]) - 1, parseInt(m[1]));
}

function driverAgeInfo(dateStr: string): { label: string; variant: "success" | "warning" | "danger" | "neutral" } {
  const date = parseDriverDate(dateStr);
  if (!date) return { label: "?", variant: "neutral" };
  const ageYears = (Date.now() - date.getTime()) / (1000 * 60 * 60 * 24 * 365.25);
  if (ageYears < 1) return { label: "A jour", variant: "success" };
  if (ageYears < 2) return { label: "Ancien", variant: "warning" };
  return { label: "Obsolete", variant: "danger" };
}

function driverKey(d: DriverEntry): string {
  return d.instance_id ?? d.module;
}

function isRunning(d: DriverEntry): boolean {
  return d.state.toLowerCase().includes("running");
}

async function rollbackDriver(d: DriverEntry) {
  const confirmed = window.confirm(
    `Rollback du driver "${d.displayName}" ?\n\nLe Gestionnaire de périphériques va s'ouvrir. Sélectionnez le driver > Propriétés > Pilote > Restaurer.`
  );
  if (!confirmed) return;

  rollingBackIds.value = [...rollingBackIds.value, driverKey(d)];
  try {
    await invoke("run_system_command", {
      cmd: "cmd",
      args: ["/c", "start", "devmgmt.msc"],
    });
    notifications.info(`Gestionnaire de périphériques ouvert`, `Trouvez "${d.displayName}" et utilisez Propriétés > Pilote > Restaurer`);
  } catch (e) {
    notifications.error(`Erreur ouverture Gestionnaire de périphériques`, String(e));
  }
  rollingBackIds.value = rollingBackIds.value.filter(x => x !== driverKey(d));
}

async function toggleDriver(d: DriverEntry) {
  const action = isRunning(d) ? "désactiver" : "activer";
  const confirmed = window.confirm(
    `Voulez-vous ${action} le driver "${d.displayName}" ?\n\n${isRunning(d) ? "ATTENTION : Désactiver un driver critique peut rendre le système instable." : ""}`
  );
  if (!confirmed) return;

  const key = driverKey(d);
  togglingIds.value = [...togglingIds.value, key];
  const instanceId = d.instance_id;
  if (!instanceId || !/^[A-Z]+\\/i.test(instanceId)) {
    notifications.error(`Identifiant PnP non disponible`, `Le driver "${d.displayName}" ne peut pas être activé/désactivé automatiquement.`);
    togglingIds.value = togglingIds.value.filter(x => x !== key);
    return;
  }
  try {
    const safeId = instanceId.replace(/'/g, "''");
    const psCmd = isRunning(d)
      ? `Disable-PnpDevice -InstanceId '${safeId}' -Confirm:$false`
      : `Enable-PnpDevice -InstanceId '${safeId}' -Confirm:$false`;

    await invoke("run_system_command", {
      cmd: "powershell",
      args: ["-Command", psCmd],
    });

    // Mettre à jour l'état local optimistement
    const idx = drivers.value.findIndex(x => driverKey(x) === key);
    if (idx !== -1) {
      drivers.value[idx] = {
        ...drivers.value[idx],
        state: isRunning(d) ? "Stopped" : "Running",
      };
    }
    const newState = isRunning(d) ? "désactivé" : "activé";
    notifications.success(`Driver "${d.displayName}" ${newState}`);
  } catch (e) {
    notifications.error(`Erreur toggle driver "${d.displayName}"`, String(e));
  }
  togglingIds.value = togglingIds.value.filter(x => x !== key);
}

// Recommended drivers
interface RecommendedDriver {
  driver: {
    id: string;
    name: string;
    description: string;
    category: string;
    url: string;
  };
  installed: boolean;
}

const recommended = ref<RecommendedDriver[]>([]);
const recommendedLoading = ref(true);

async function loadRecommended() {
  recommendedLoading.value = true;
  try {
    recommended.value = await invoke<RecommendedDriver[]>("get_recommended_drivers");
  } catch {
    recommended.value = [
      { driver: { id: "vcredist-2015-2022", name: "Visual C++ 2015-2022", description: "Runtime requis par la plupart des applications", category: "Runtime", url: "https://aka.ms/vs/17/release/vc_redist.x64.exe" }, installed: true },
      { driver: { id: "dotnet-8", name: ".NET 8 Runtime", description: "Runtime .NET 8 LTS", category: "Runtime", url: "#" }, installed: false },
      { driver: { id: "nvidia-drivers", name: "NVIDIA GeForce Drivers", description: "Pilotes GPU NVIDIA", category: "GPU", url: "#" }, installed: true },
    ];
  } finally {
    recommendedLoading.value = false;
  }
}

async function launchSdi() {
  try {
    (window as any).__nitrite_sdi_active = true;
    setTimeout(() => { (window as any).__nitrite_sdi_active = false; }, 60000);
    await invoke("launch_sdi");
    notifications.success("Snappy Driver Installer lancé");
  } catch (e) {
    (window as any).__nitrite_sdi_active = false;
    notifications.error("SDI introuvable", String(e));
  }
}

async function openDriverUrl(url: string) {
  try {
    await invoke("open_url", { url });
  } catch {
    window.open(url, "_blank");
  }
}

const recommendedCategories = computed(() => {
  const cats = new Map<string, RecommendedDriver[]>();
  for (const r of recommended.value) {
    const cat = r.driver.category;
    if (!cats.has(cat)) cats.set(cat, []);
    cats.get(cat)!.push(r);
  }
  return cats;
});

onMounted(() => {
  loadDrivers();
  loadRecommended();
});
</script>

<template>
  <div class="drivers-page">
    <div class="page-header">
      <div>
        <h1>Drivers</h1>
        <p class="page-subtitle">Liste des pilotes systeme installes</p>
      </div>
      <div class="header-actions">
        <div class="page-tabs">
          <button class="page-tab" :class="{ active: mainTab === 'list' }" @click="mainTab = 'list'"><Cpu :size="13" /> Pilotes</button>
          <button class="page-tab" :class="{ active: mainTab === 'diagnostics' }" @click="mainTab = 'diagnostics'"><ScanSearch :size="13" /> Diagnostics</button>
        </div>
        <NButton variant="primary" size="sm" @click="launchSdi"><Zap :size="14" /> Snappy Driver</NButton>
        <NButton variant="ghost" size="sm" @click="openExportFolder"><FolderOpen :size="14" /> Exports</NButton>
        <NButton variant="secondary" size="sm" @click="exportTxt"><FileText :size="14" /> TXT</NButton>
        <NButton variant="secondary" size="sm" @click="exportCSV">
          <FileSpreadsheet :size="14" />
          CSV
        </NButton>
        <NButton variant="primary" size="sm" :loading="loading" @click="loadDrivers">
          <RefreshCw :size="14" />
          Rafraichir
        </NButton>
      </div>
    </div>

    <!-- Onglet Diagnostics -->
    <DiagTabSysDrivers v-if="mainTab === 'diagnostics'" />

    <!-- Onglet Pilotes (liste) -->
    <template v-if="mainTab === 'list'">

    <!-- Bandeau d'alerte drivers problématiques -->
    <div v-if="!loading && problematicDrivers.length > 0" class="alert-banner">
      <AlertTriangle :size="16" class="alert-icon" />
      <span>
        <strong>{{ problematicDrivers.length }} driver(s) problématique(s) détecté(s)</strong>
        — {{ problematicDrivers.map(d => d.displayName).join(", ") }}
      </span>
    </div>

    <!-- Recommended Drivers -->
    <NCard>
      <template #header>
        <div class="section-header">
          <Download :size="16" />
          <span>Telechargements recommandes</span>
          <NButton variant="secondary" size="sm" :loading="recommendedLoading" @click="loadRecommended" style="margin-left: auto">
            <RefreshCw :size="14" />
          </NButton>
        </div>
      </template>
      <div v-if="recommendedLoading" class="loading-state"><NSpinner :size="24" /><p>Detection...</p></div>
      <div v-else class="rec-drivers">
        <div v-for="[cat, items] in recommendedCategories" :key="cat" class="rec-category">
          <h4 class="rec-cat-title">{{ cat }}</h4>
          <div class="rec-list">
            <div v-for="r in items" :key="r.driver.id" class="rec-item">
              <div class="rec-status">
                <CheckCircle v-if="r.installed" :size="16" style="color: var(--success)" />
                <XCircle v-else :size="16" style="color: var(--text-muted)" />
              </div>
              <div class="rec-info">
                <span class="rec-name">{{ r.driver.name }}</span>
                <span class="rec-desc">{{ r.driver.description }}</span>
              </div>
              <NBadge v-if="r.installed" variant="success">Installe</NBadge>
              <NButton v-else variant="primary" size="sm" @click="openDriverUrl(r.driver.url)">
                <ExternalLink :size="12" /> Telecharger
              </NButton>
            </div>
          </div>
        </div>
      </div>
    </NCard>

    <NSearchBar v-model="search" placeholder="Rechercher un pilote..." />

    <div v-if="loading" class="loading-state">
      <NSpinner :size="32" />
      <p>Chargement des pilotes...</p>
    </div>

    <template v-else>
      <NTabs :tabs="driverCategoryTabs" v-model="activeDriverCategory">
        <template #default>
          <NCard>
            <template #header>
              <div class="section-header">
                <Cpu :size="16" />
                <span>Pilotes ({{ filteredDrivers.length }})</span>
              </div>
            </template>
            <div class="table-wrapper">
              <table class="drivers-table">
                <thead>
                  <tr>
                    <th>Module</th>
                    <th>Nom / Provider</th>
                    <th>Type</th>
                    <th>Date</th>
                    <th>Age</th>
                    <th>Etat</th>
                    <th>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="d in filteredDrivers"
                    :key="d.module"
                    :class="`age-${driverAgeInfo(d.linkDate).variant}`"
                  >
                    <td class="font-mono">
                      {{ d.module }}
                      <!-- Badge CRITIQUE -->
                      <NBadge v-if="isCriticalDriver(d.module, d.displayName)" variant="danger" class="critical-badge">
                        CRITIQUE
                      </NBadge>
                    </td>
                    <td>
                      <div class="driver-name-cell">
                        <span>{{ d.displayName }}</span>
                        <span v-if="d.provider" class="driver-provider">{{ d.provider }}</span>
                      </div>
                    </td>
                    <td>{{ d.driverType }}</td>
                    <td class="font-mono">{{ d.linkDate }}</td>
                    <td><NBadge :variant="driverAgeInfo(d.linkDate).variant">{{ driverAgeInfo(d.linkDate).label }}</NBadge></td>
                    <td><NBadge :variant="stateVariant(d.state)">{{ d.state }}</NBadge></td>
                    <td>
                      <div class="action-cell">
                        <!-- Rollback -->
                        <NButton
                          variant="secondary"
                          size="sm"
                          :loading="rollingBackIds.includes(driverKey(d))"
                          :title="`Rollback : ${d.displayName}`"
                          @click="rollbackDriver(d)"
                        >
                          <RotateCcw :size="12" />
                          Rollback
                        </NButton>
                        <!-- Enable / Disable -->
                        <NButton
                          :variant="isRunning(d) ? 'warning' : 'success'"
                          size="sm"
                          :loading="togglingIds.includes(driverKey(d))"
                          :title="isRunning(d) ? 'Désactiver le driver' : 'Activer le driver'"
                          @click="toggleDriver(d)"
                        >
                          <ToggleLeft v-if="isRunning(d)" :size="12" />
                          <ToggleRight v-else :size="12" />
                          {{ isRunning(d) ? "Désactiver" : "Activer" }}
                        </NButton>
                      </div>
                    </td>
                  </tr>
                  <tr v-if="filteredDrivers.length === 0">
                    <td colspan="7" class="empty-row">Aucun pilote dans cette categorie</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </NCard>
        </template>
      </NTabs>
    </template>
    </template><!-- end list tab -->
  </div>
</template>

<style scoped>
.drivers-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.page-tabs {
  display: flex;
  gap: 3px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 3px;
}
.page-tab {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 5px 14px;
  border-radius: calc(var(--radius-md) - 2px);
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  transition: all .15s;
}
.page-tab:hover { color: var(--text-primary); }
.page-tab.active { background: var(--accent-muted); color: var(--accent-primary); font-weight: 600; }

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.page-header h1 { font-size: 24px; font-weight: 700; }
.page-subtitle { color: var(--text-muted); font-size: 13px; margin-top: 2px; }
.header-actions { display: flex; gap: 8px; }

.section-header { display: flex; align-items: center; gap: 8px; }

/* Bandeau alerte drivers problématiques */
.alert-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  background: color-mix(in srgb, var(--danger) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--danger) 35%, transparent);
  border-radius: var(--radius-md);
  font-size: 13px;
  color: var(--text-primary);
}

.alert-icon {
  color: var(--danger);
  flex-shrink: 0;
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 60px;
  color: var(--text-muted);
}

.table-wrapper {
  overflow-x: auto;
}

.drivers-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.drivers-table th {
  text-align: left;
  padding: 10px 12px;
  color: var(--text-muted);
  font-weight: 500;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--border);
  white-space: nowrap;
}

.drivers-table td {
  padding: 8px 12px;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border);
  vertical-align: middle;
}

.drivers-table tbody tr:hover {
  background: var(--bg-tertiary);
}

.drivers-table tbody tr.age-warning {
  border-left: 3px solid var(--warning);
}

.drivers-table tbody tr.age-danger {
  border-left: 3px solid var(--danger);
}

.drivers-table tbody tr.age-success {
  border-left: 3px solid transparent;
}

.font-mono {
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
}

.empty-row {
  text-align: center;
  color: var(--text-muted);
  padding: 24px !important;
}

/* Driver name + provider */
.driver-name-cell {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.driver-provider {
  font-size: 11px;
  color: var(--text-muted);
  font-style: italic;
}

/* Badge critique */
.critical-badge {
  margin-left: 6px;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.5px;
  vertical-align: middle;
}

/* Actions column */
.action-cell {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

/* Recommended Drivers */
.rec-drivers { display: flex; flex-direction: column; gap: 16px; }
.rec-cat-title {
  font-size: 12px;
  font-weight: 700;
  color: var(--accent-primary);
  text-transform: uppercase;
  letter-spacing: 0.8px;
  margin-bottom: 6px;
  padding: 4px 8px;
  background: color-mix(in srgb, var(--accent-primary) 10%, transparent);
  border-left: 3px solid var(--accent-primary);
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
}
.rec-list { display: flex; flex-direction: column; gap: 2px; }

.rec-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 10px;
  border-radius: var(--radius-md);
  transition: background var(--transition-fast);
}
.rec-item:hover { background: var(--bg-tertiary); }

.rec-status { flex-shrink: 0; }
.rec-info { flex: 1; display: flex; flex-direction: column; gap: 2px; }
.rec-name { font-size: 13px; font-weight: 500; color: var(--text-primary); }
.rec-desc { font-size: 11px; color: var(--text-muted); }
</style>
