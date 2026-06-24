<script setup lang="ts">
import { ref, computed, onMounted, defineAsyncComponent } from "vue";
import { invoke } from "@/utils/invoke";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import NButton from "@/components/ui/NButton.vue";
const DllScannerTab = defineAsyncComponent(() => import("@/components/uninstaller/DllScannerTab.vue"));
import NProgress from "@/components/ui/NProgress.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import { useNotificationStore } from "@/stores/notifications";
import {
  Trash2, Search, RefreshCw, CheckSquare, Square,
  CheckCircle, XCircle, AlertTriangle, Package,
  ChevronUp, ChevronDown, Eye, X, FolderOpen,
  RotateCcw, Archive, ShieldCheck, AlertOctagon,
  ExternalLink, Lightbulb, Library,
} from "lucide-vue-next";

const mainTab = ref<'apps' | 'dll'>('apps');

const notify = useNotificationStore();

interface InstalledApp {
  name: string; version: string; publisher: string;
  uninstall_string: string; source: string;
  install_size_kb: number; install_date: string;
  registry_key?: string;
}

interface UninstallJob {
  app: InstalledApp;
  status: "pending" | "running" | "done" | "error";
  message: string;
  residuals: string[];
  residualsHandled: boolean;
  keepAppData: boolean;
}

interface ResidualCleanResult {
  success: boolean; deleted_count: number; failed_count: number; message: string;
}

// ─── Restore point avant désinstall ─────────────────────────────────────────
const LS_RESTORE = "nitrite-uninstall-restore";
const createRestoreBeforeUninstall = ref(localStorage.getItem(LS_RESTORE) === "true");

function toggleRestore() {
  createRestoreBeforeUninstall.value = !createRestoreBeforeUninstall.value;
  localStorage.setItem(LS_RESTORE, String(createRestoreBeforeUninstall.value));
}

async function doCreateRestorePoint() {
  try {
    await invoke("run_system_command", {
      cmd: "powershell",
      args: ["-Command", "Checkpoint-Computer -Description 'NitritePreUninstall' -RestorePointType 'MODIFY_SETTINGS'"],
    });
    notify.success("Point de restauration créé", "NitritePreUninstall");
  } catch {
    notify.warning("Point de restauration", "Impossible de créer (droits admin requis ?)");
  }
}

// ─── Undo last uninstall ─────────────────────────────────────────────────────
const LS_LAST_UNINSTALL = "nitrite-last-uninstall";
interface LastUninstall { name: string; date: string; }
function loadLastUninstall(): LastUninstall | null {
  try { const v = localStorage.getItem(LS_LAST_UNINSTALL); return v ? JSON.parse(v) : null; }
  catch { return null; }
}
const lastUninstall = ref<LastUninstall | null>(loadLastUninstall());

function isUndoVisible(): boolean {
  if (!lastUninstall.value) return false;
  const diff = Date.now() - new Date(lastUninstall.value.date).getTime();
  return diff < 10 * 60 * 1000; // 10 minutes
}

function saveLastUninstall(name: string) {
  const entry: LastUninstall = { name, date: new Date().toISOString() };
  lastUninstall.value = entry;
  localStorage.setItem(LS_LAST_UNINSTALL, JSON.stringify(entry));
}

async function undoLastUninstall() {
  try {
    await invoke("run_system_command", { cmd: "cmd", args: ["/c", "start", "ms-settings:appsfeatures"] });
    notify.success("Paramètres ouverts", "Réinstallez l'application depuis le Microsoft Store ou son installeur d'origine.");
  } catch (e: any) {
    notify.error("Erreur", String(e));
  }
}

// ─── Force remove ────────────────────────────────────────────────────────────
const forceRemoveApp = ref<InstalledApp | null>(null);
const forceRemoveConfirm = ref(0); // 0 = idle, 1 = 1er clic, 2 = 2e clic = exécute
const forceRemoving = ref(false);

function startForceRemove(app: InstalledApp) {
  forceRemoveApp.value = app;
  forceRemoveConfirm.value = 1;
}

async function confirmForceRemove() {
  if (!forceRemoveApp.value) return;
  if (forceRemoveConfirm.value === 1) {
    forceRemoveConfirm.value = 2;
    return;
  }
  const app = forceRemoveApp.value;
  forceRemoving.value = true;
  const rawKey = app.registry_key ?? app.name;
  const regKey = rawKey.replace(/'/g, "''").replace(/[`$()]/g, "");
  try {
    await invoke("run_system_command", {
      cmd: "powershell",
      args: ["-Command", `Remove-Item -Path (Get-ItemProperty 'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${regKey}').PSPath -Recurse -Force -ErrorAction SilentlyContinue; Remove-Item -Path 'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${regKey}' -Recurse -Force -ErrorAction SilentlyContinue`],
    });
    notify.success("Suppression forcée", `${app.name} retiré du registre`);
    apps.value = apps.value.filter(a => a.name !== app.name);
    saveLastUninstall(app.name);
  } catch (e: any) {
    notify.error("Suppression forcée échouée", String(e));
  }
  forceRemoving.value = false;
  forceRemoveApp.value = null;
  forceRemoveConfirm.value = 0;
}

function cancelForceRemove() {
  forceRemoveApp.value = null;
  forceRemoveConfirm.value = 0;
}

// ─── Post-uninstall tips ─────────────────────────────────────────────────────
const showCleanupTips = ref(false);
const cleanupTipsApp = ref("");

function showTips(appName: string) {
  cleanupTipsApp.value = appName;
  showCleanupTips.value = true;
}

// ─── App list & state ────────────────────────────────────────────────────────
const apps = ref<InstalledApp[]>([]);
const loading = ref(false);
const search = ref("");
const selected = ref<Set<string>>(new Set());
const filterPublisher = ref("");
const filterCat = ref<string>("all");

type AppCat = "system" | "microsoft" | "driver" | "redist" | "user";
const CAT_LABELS: Record<AppCat, string> = { system:"Système Windows", microsoft:"Microsoft", driver:"Pilotes", redist:"Redistributables", user:"Applications" };
const CAT_COLORS: Record<AppCat, string> = { system:"#ef4444", microsoft:"#3b82f6", driver:"#f59e0b", redist:"#8b5cf6", user:"#22c55e" };

const SYSTEM_PUBS = ["microsoft windows","windows embedded","microsoft corporation"];
const DRIVER_PUBS = ["intel","amd","nvidia","realtek","qualcomm","broadcom","marvell","mediatek","asus","dell","hp","lenovo","gigabyte","asrock","logitech","corsair","razer","steelseries","creative","sound blaster","epson","canon","brother","sharp","konica"];
const REDIST_NAMES = ["visual c++","microsoft .net","directx","windows sdk","vcredist","msvc","java ","jre ","jdk ","openal","webview2","redistributable","runtime","framework","cumulative"];
const SYSTEM_NAMES = ["windows ","microsoft windows","windows media","windows powershell","microsoft edge","cortana","xbox","microsoft store","windows security","windows defender"];

function appCategory(app: InstalledApp): AppCat {
  const n = app.name.toLowerCase();
  const p = (app.publisher || "").toLowerCase();
  if (SYSTEM_NAMES.some(s => n.startsWith(s))) return "system";
  if (REDIST_NAMES.some(s => n.includes(s))) return "redist";
  if (SYSTEM_PUBS.includes(p) && (n.includes("windows") || n.includes("microsoft"))) return "system";
  if (DRIVER_PUBS.some(s => p.includes(s)) && (n.includes("driver") || n.includes("pilote") || n.includes("firmware") || n.includes("chipset"))) return "driver";
  if (p.includes("microsoft") || p === "microsoft corporation") return "microsoft";
  return "user";
}

// Per-app keep-appdata preference (map: app.name → boolean)
const keepAppDataMap = ref<Record<string, boolean>>({});
function toggleKeepAppData(name: string) {
  keepAppDataMap.value[name] = !keepAppDataMap.value[name];
}

type SortKey = "name" | "publisher" | "size" | "date" | "version";
const sortKey = ref<SortKey>("name");
const sortDir = ref<"asc" | "desc">("asc");
function setSort(key: SortKey) {
  if (sortKey.value === key) sortDir.value = sortDir.value === "asc" ? "desc" : "asc";
  else { sortKey.value = key; sortDir.value = "asc"; }
}

const previewApp = ref<InstalledApp | null>(null);
const previewItems = ref<string[]>([]);
const loadingPreview = ref(false);
async function showPreview(app: InstalledApp) {
  previewApp.value = app; previewItems.value = []; loadingPreview.value = true;
  try {
    previewItems.value = await invoke<string[]>("preview_residuals", { appName: app.name, publisher: app.publisher });
  } catch (e: any) { notify.error("Erreur preview", String(e)); }
  loadingPreview.value = false;
}

const jobs = ref<UninstallJob[]>([]);
const uninstalling = ref(false);
const progress = ref(0);
const currentStep = ref("");
const showResults = ref(false);
const handlingResiduals = ref<Record<string, boolean>>({});
const extractTarget = ref("C:\\NiTriTe\\Résidus");
const minSizeMb = ref(0);

const filtered = computed(() => {
  const q = search.value.toLowerCase();
  const pub = filterPublisher.value.toLowerCase();
  const minKb = minSizeMb.value * 1024;
  return apps.value.filter(a =>
    a.name.toLowerCase().includes(q) &&
    (pub === "" || a.publisher.toLowerCase().includes(pub)) &&
    (minKb === 0 || (a.install_size_kb ?? 0) >= minKb) &&
    (filterCat.value === "all" || appCategory(a) === filterCat.value)
  );
});

const totalSelectedSizeMb = computed(() => {
  const total = apps.value
    .filter(a => selected.value.has(a.name))
    .reduce((acc, a) => acc + (a.install_size_kb ?? 0), 0);
  return total >= 1024 ? `${(total / 1024).toFixed(0)} Mo` : `${total} Ko`;
});

const sorted = computed(() => {
  const list = [...filtered.value];
  const dir = sortDir.value === "asc" ? 1 : -1;
  list.sort((a, b) => {
    switch (sortKey.value) {
      case "name":      return dir * a.name.localeCompare(b.name);
      case "publisher": return dir * a.publisher.localeCompare(b.publisher);
      case "version":   return dir * a.version.localeCompare(b.version);
      case "size":      return dir * (a.install_size_kb - b.install_size_kb);
      case "date":      return dir * a.install_date.localeCompare(b.install_date);
    }
  });
  return list;
});

const selectedCount = computed(() => selected.value.size);
const allChecked = computed(() =>
  filtered.value.length > 0 && filtered.value.every(a => selected.value.has(a.name))
);
const pendingResiduals = computed(() => jobs.value.filter(j => j.status === "done" && !j.residualsHandled && j.residuals.length > 0));

function toggleSelect(name: string) {
  if (selected.value.has(name)) selected.value.delete(name);
  else selected.value.add(name);
  selected.value = new Set(selected.value);
}
function toggleAll() {
  if (allChecked.value) filtered.value.forEach(a => selected.value.delete(a.name));
  else filtered.value.forEach(a => selected.value.add(a.name));
  selected.value = new Set(selected.value);
}

async function loadApps() {
  loading.value = true; apps.value = []; selected.value = new Set();
  try {
    apps.value = await invoke<InstalledApp[]>("list_installed_apps_for_uninstall");
  } catch (e: any) { notify.error("Chargement échoué", String(e)); }
  loading.value = false;
}

async function startUninstall() {
  const toUninstall = apps.value.filter(a => selected.value.has(a.name));
  if (!toUninstall.length) return;

  if (createRestoreBeforeUninstall.value) await doCreateRestorePoint();

  jobs.value = toUninstall.map(a => ({
    app: a, status: "pending", message: "", residuals: [], residualsHandled: false,
    keepAppData: keepAppDataMap.value[a.name] ?? false,
  }));
  uninstalling.value = true;
  progress.value = 0;
  showResults.value = false;
  previewApp.value = null;

  for (let i = 0; i < jobs.value.length; i++) {
    const job = jobs.value[i];
    job.status = "running";
    currentStep.value = `(${i + 1}/${jobs.value.length}) Désinstallation de ${job.app.name}...`;
    try {
      const result = await invoke<{ success: boolean; message: string; residuals_found: string[] }>("uninstall_app_clean", {
        appName: job.app.name,
        uninstallString: job.app.uninstall_string,
        publisher: job.app.publisher,
      });
      job.status = result.success ? "done" : "error";
      job.message = result.message;

      // Filtrer les résidus AppData si l'utilisateur veut les conserver
      const rawResiduals = result.residuals_found ?? [];
      job.residuals = job.keepAppData
        ? rawResiduals.filter(r => !r.toLowerCase().includes("appdata"))
        : rawResiduals;

      if (result.success) {
        apps.value = apps.value.filter(a => a.name !== job.app.name);
        selected.value.delete(job.app.name);
        saveLastUninstall(job.app.name);
        showTips(job.app.name);
      }
    } catch (e: any) {
      job.status = "error";
      job.message = String(e);
    }
    progress.value = Math.round(((i + 1) / jobs.value.length) * 100);
  }

  const doneCount  = jobs.value.filter(j => j.status === "done").length;
  const errCount   = jobs.value.filter(j => j.status === "error").length;
  const residCount = jobs.value.filter(j => j.residuals.length > 0).length;

  if (errCount === 0) notify.success("Désinstallation terminée", `${doneCount} app(s) supprimée(s)`);
  else notify.warning("Partiellement terminé", `${doneCount} succès, ${errCount} échec(s)`);

  uninstalling.value = false;
  currentStep.value = "";
  showResults.value = doneCount > 0 || errCount > 0;

  if (residCount > 0) notify.warning(`${residCount} app(s) avec résidus`, "Consultez le panneau Résultats pour nettoyer.");
}

async function deleteJobResiduals(job: UninstallJob) {
  handlingResiduals.value[job.app.name] = true;
  try {
    const r = await invoke<ResidualCleanResult>("delete_residuals", { paths: job.residuals });
    if (r.success) notify.success("Résidus supprimés", r.message);
    else notify.error("Suppression partielle", r.message);
    job.residualsHandled = true;
  } catch (e: any) { notify.error("Erreur", String(e)); }
  handlingResiduals.value[job.app.name] = false;
}

async function extractJobResiduals(job: UninstallJob) {
  handlingResiduals.value[job.app.name] = true;
  try {
    const r = await invoke<ResidualCleanResult>("extract_residuals", {
      paths: job.residuals,
      target: extractTarget.value,
    });
    if (r.success) notify.success("Résidus extraits + supprimés", r.message);
    else notify.error("Extraction partielle", r.message);
    job.residualsHandled = true;
  } catch (e: any) { notify.error("Erreur", String(e)); }
  handlingResiduals.value[job.app.name] = false;
}

async function pickExtractTarget() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const dir = await open({ directory: true, multiple: false, title: "Dossier d'extraction" });
  if (dir) extractTarget.value = dir as string;
}

function formatSize(kb: number) {
  if (!kb) return "—";
  if (kb < 1024) return `${kb} Ko`;
  return `${(kb / 1024).toFixed(1)} Mo`;
}

function sourceLabel(src: string) {
  if (!src) return "";
  if (src.toLowerCase().includes("hkcu")) return "User";
  if (src.toLowerCase().includes("store") || src.toLowerCase().includes("appx")) return "Store";
  return "Sys";
}

function selectByPublisher(pub: string) {
  if (!pub) return;
  filtered.value.filter(a => a.publisher === pub).forEach(a => selected.value.add(a.name));
  selected.value = new Set(selected.value);
}

async function exportCsv() {
  const rows = ["Nom,Version,Editeur,Taille,Installé le,Source"];
  for (const a of sorted.value) {
    const esc = (s: string) => `"${(s || "").replace(/"/g, '""')}"`;
    rows.push([esc(a.name), esc(a.version), esc(a.publisher), formatSize(a.install_size_kb), esc(a.install_date), esc(a.source)].join(","));
  }
  const csv = rows.join("\r\n");
  try {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const { writeTextFile } = await import("@tauri-apps/plugin-fs");
    const path = await save({ filters: [{ name: "CSV", extensions: ["csv"] }], defaultPath: "applications.csv" });
    if (path) { await writeTextFile(path, csv); notify.success("Export CSV", path); }
  } catch {
    await navigator.clipboard.writeText(csv);
    notify.success("Copié", "CSV copié dans le presse-papier");
  }
}

onMounted(loadApps);
</script>

<template>
  <div class="uninstaller-page">
    <DiagBanner :icon="Trash2" title="Désinstallateur Propre"
      desc="Désinstallation silencieuse automatique — registre nettoyé, fichiers purgés" color="red" />

    <!-- ═══ Sélecteur d'onglet principal ═══ -->
    <div class="main-tabs">
      <button class="main-tab" :class="{ active: mainTab === 'apps' }" @click="mainTab = 'apps'">
        <Package :size="14" /> Applications
      </button>
      <button class="main-tab" :class="{ active: mainTab === 'dll' }" @click="mainTab = 'dll'">
        <Library :size="14" /> DLL / Bibliothèques
      </button>
    </div>

    <!-- ═══ Onglet DLL ═══ -->
    <Suspense v-if="mainTab === 'dll'"><DllScannerTab /></Suspense>

    <!-- ═══ Contenu onglet Applications ═══ -->
    <template v-if="mainTab === 'apps'">
    <!-- Bandeau options sécurité -->
    <div class="safety-bar">
      <!-- Restore point -->
      <label class="checkbox-row" @click="toggleRestore">
        <span class="custom-check" :class="{ checked: createRestoreBeforeUninstall }">
          <svg v-if="createRestoreBeforeUninstall" width="10" height="10" viewBox="0 0 12 12">
            <path d="M2 6l3 3 5-5" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </span>
        <ShieldCheck :size="13" style="color:var(--success);flex-shrink:0" />
        <span class="checkbox-label">Créer un point de restauration avant désinstallation</span>
      </label>

      <!-- Undo last uninstall -->
      <div v-if="isUndoVisible()" class="undo-banner">
        <RotateCcw :size="13" style="color:var(--accent-primary)" />
        <span>Dernière désinstallation : <strong>{{ lastUninstall?.name }}</strong></span>
        <NButton variant="ghost" size="sm" @click="undoLastUninstall">
          <ExternalLink :size="12" /> Annuler (ouvrir Paramètres)
        </NButton>
      </div>
    </div>

    <!-- Force Remove Dialog -->
    <div v-if="forceRemoveApp" class="force-remove-dialog">
      <div class="force-remove-inner">
        <AlertOctagon :size="18" style="color:var(--danger);flex-shrink:0" />
        <div class="force-remove-content">
          <p class="force-remove-title">Suppression forcée — <strong>{{ forceRemoveApp.name }}</strong></p>
          <p class="force-remove-desc">
            Cette action supprime directement la clé de registre Windows sans passer par le désinstalleur officiel.
            L'application peut laisser des fichiers orphelins. <strong>Non réversible.</strong>
          </p>
          <div class="force-remove-actions">
            <NButton v-if="forceRemoveConfirm < 2" variant="danger" size="sm" @click="confirmForceRemove" :loading="forceRemoving">
              {{ forceRemoveConfirm === 1 ? "Confirmer — supprimer définitivement ?" : "Forcer la suppression" }}
            </NButton>
            <NButton v-else variant="danger" size="sm" :loading="forceRemoving" @click="confirmForceRemove">
              Exécuter maintenant
            </NButton>
            <NButton variant="ghost" size="sm" @click="cancelForceRemove"><X :size="12" /> Annuler</NButton>
          </div>
        </div>
      </div>
    </div>

    <!-- Post-uninstall Tips -->
    <div v-if="showCleanupTips" class="tips-panel">
      <div class="tips-header">
        <Lightbulb :size="14" style="color:#f59e0b" />
        <span class="tips-title">Nettoyage recommandé après désinstallation de <strong>{{ cleanupTipsApp }}</strong></span>
        <button class="close-btn" @click="showCleanupTips = false"><X :size="14" /></button>
      </div>
      <ul class="tips-list">
        <li><CheckCircle :size="11" class="tip-ic" /> <strong>Redémarrez</strong> le PC pour finaliser la suppression et libérer les DLL verrouillées.</li>
        <li><CheckCircle :size="11" class="tip-ic" /> <strong>Vérifiez les résidus</strong> dans <code>%AppData%</code>, <code>%LocalAppData%</code> et <code>%ProgramData%</code>.</li>
        <li><CheckCircle :size="11" class="tip-ic" /> <strong>Videz la Corbeille</strong> pour récupérer l'espace disque définitivement.</li>
        <li><CheckCircle :size="11" class="tip-ic" /> Utilisez l'onglet <strong>Nettoyage</strong> pour purger les fichiers temporaires restants.</li>
      </ul>
    </div>

    <!-- Toolbar -->
    <div class="toolbar">
      <div class="toolbar-left">
        <div class="search-wrap">
          <Search :size="14" class="search-icon" />
          <input v-model="search" placeholder="Rechercher une application..." class="search-input" />
        </div>
        <input v-model="filterPublisher" placeholder="Filtrer par éditeur..." class="filter-input" />
      </div>
      <div class="toolbar-right">
        <span class="apps-count">{{ apps.length }} apps</span>
        <div class="size-filter-wrap">
          <span class="size-filter-label">Min</span>
          <select v-model.number="minSizeMb" class="size-filter-select">
            <option :value="0">Toutes</option>
            <option :value="10">10 Mo+</option>
            <option :value="50">50 Mo+</option>
            <option :value="100">100 Mo+</option>
            <option :value="500">500 Mo+</option>
            <option :value="1024">1 Go+</option>
          </select>
        </div>
        <NButton variant="ghost" size="sm" @click="exportCsv" title="Exporter la liste en CSV">
          ↓ CSV
        </NButton>
        <NButton variant="ghost" size="sm" :loading="loading" @click="loadApps">
          <RefreshCw :size="13" /> Actualiser
        </NButton>
        <NButton variant="danger" size="sm" :disabled="selectedCount === 0 || uninstalling" @click="startUninstall"
          :title="`Libérer ~${totalSelectedSizeMb}`">
          <Trash2 :size="13" /> Désinstaller ({{ selectedCount }}) · {{ totalSelectedSizeMb }}
        </NButton>
      </div>
    </div>

    <!-- Filtres catégories -->
    <div class="cat-filter-row">
      <button class="cat-pill" :class="{active: filterCat==='all'}" @click="filterCat='all'">
        Tout ({{ apps.length }})
      </button>
      <button v-for="(label, cat) in CAT_LABELS" :key="cat"
        class="cat-pill" :class="{active: filterCat===cat}"
        :style="filterCat===cat ? {borderColor: CAT_COLORS[cat as AppCat], color: CAT_COLORS[cat as AppCat], background: CAT_COLORS[cat as AppCat]+'18'} : {}"
        @click="filterCat = cat"
        :title="cat==='system'?'⚠ Applications système Windows — ne pas désinstaller':cat==='microsoft'?'Applications Microsoft':cat==='driver'?'Pilotes matériels':cat==='redist'?'Redistribuables (C++, .NET, Java...)':'Applications utilisateur'"
      >
        <span v-if="cat==='system'" style="font-size:10px">⚠</span>
        {{ label }} ({{ apps.filter(a => appCategory(a) === cat).length }})
      </button>
    </div>

    <!-- Progress -->
    <div v-if="uninstalling" class="progress-bar">
      <div class="progress-header"><NSpinner :size="14" /><span>{{ currentStep }}</span></div>
      <NProgress :value="progress" showLabel size="sm" />
    </div>

    <!-- ══ RÉSULTATS + RÉSIDUS ══ -->
    <div v-if="showResults && !uninstalling" class="results-panel">
      <div class="results-header">
        <span class="results-title">Résultats de désinstallation</span>
        <button class="close-btn" @click="showResults = false; jobs = []"><X :size="14" /></button>
      </div>

      <div v-if="pendingResiduals.length > 0" class="extract-global-row">
        <label class="small-label">Dossier d'extraction :</label>
        <input v-model="extractTarget" class="extract-input" />
        <NButton variant="ghost" size="sm" @click="pickExtractTarget"><FolderOpen :size="12" /></NButton>
      </div>

      <div v-for="job in jobs" :key="job.app.name" class="job-block" :class="job.status">
        <div class="job-row">
          <CheckCircle v-if="job.status === 'done'"  :size="14" class="ic-success" />
          <XCircle     v-else-if="job.status === 'error'" :size="14" class="ic-error" />
          <NSpinner    v-else :size="14" />
          <span class="job-name">{{ job.app.name }}</span>
          <span class="job-msg">{{ job.message }}</span>
          <span v-if="job.keepAppData" class="keep-appdata-tag">AppData conservée</span>
        </div>

        <div v-if="job.residuals.length > 0 && !job.residualsHandled" class="residuals-block">
          <div class="residuals-header">
            <AlertTriangle :size="12" style="color:#d97706;flex-shrink:0" />
            <span class="res-label">{{ job.residuals.length }} résidu(s) détecté(s) :</span>
            <div class="res-actions">
              <NButton variant="danger" size="sm" :loading="!!handlingResiduals[job.app.name]" @click="deleteJobResiduals(job)">
                <Trash2 :size="11" /> Supprimer définitivement
              </NButton>
              <NButton variant="ghost" size="sm" :loading="!!handlingResiduals[job.app.name]" @click="extractJobResiduals(job)">
                <Archive :size="11" /> Extraire puis supprimer
              </NButton>
              <NButton variant="ghost" size="sm" @click="job.residualsHandled = true">Ignorer</NButton>
            </div>
          </div>
          <div class="residuals-list">
            <div v-for="r in job.residuals" :key="r" class="residual-item">{{ r }}</div>
          </div>
        </div>
        <div v-else-if="job.residuals.length > 0 && job.residualsHandled" class="res-done">
          <CheckCircle :size="12" class="ic-success" /> Résidus traités
        </div>
        <div v-else-if="job.status === 'done'" class="res-done">
          <CheckCircle :size="12" class="ic-success" /> Aucun résidu détecté
        </div>
      </div>
    </div>

    <!-- C — Preview résidus (avant uninstall) -->
    <div v-if="previewApp" class="preview-panel">
      <div class="preview-header">
        <Eye :size="14" style="color:#d97706" />
        <span class="preview-title">Résidus potentiels — <strong>{{ previewApp.name }}</strong></span>
        <button class="close-btn" @click="previewApp = null"><X :size="14" /></button>
      </div>
      <div v-if="loadingPreview" class="preview-loading"><NSpinner :size="14" /><span>Analyse...</span></div>
      <div v-else-if="previewItems.length === 0" class="preview-empty">Aucun résidu détecté.</div>
      <div v-else class="preview-list">
        <div v-for="item in previewItems" :key="item" class="residual-item">{{ item }}</div>
      </div>
      <p class="preview-hint">Ces éléments seront proposés à la suppression après désinstallation.</p>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="loading-state"><NSpinner :size="24" /><p>Lecture du registre Windows...</p></div>

    <!-- Table -->
    <div v-else-if="apps.length > 0" class="apps-table-wrap">
      <table class="apps-table">
        <thead>
          <tr>
            <th class="col-check">
              <button class="check-all" @click="toggleAll">
                <CheckSquare v-if="allChecked" :size="15" style="color:var(--accent-primary)" />
                <Square v-else :size="15" />
              </button>
            </th>
            <th class="col-name sortable"      @click="setSort('name')">
              Application
              <ChevronUp v-if="sortKey==='name'&&sortDir==='asc'" :size="11" class="sort-ic"/>
              <ChevronDown v-if="sortKey==='name'&&sortDir==='desc'" :size="11" class="sort-ic"/>
            </th>
            <th class="col-version sortable"   @click="setSort('version')">
              Version
              <ChevronUp v-if="sortKey==='version'&&sortDir==='asc'" :size="11" class="sort-ic"/>
              <ChevronDown v-if="sortKey==='version'&&sortDir==='desc'" :size="11" class="sort-ic"/>
            </th>
            <th class="col-publisher sortable" @click="setSort('publisher')">
              Éditeur
              <ChevronUp v-if="sortKey==='publisher'&&sortDir==='asc'" :size="11" class="sort-ic"/>
              <ChevronDown v-if="sortKey==='publisher'&&sortDir==='desc'" :size="11" class="sort-ic"/>
            </th>
            <th class="col-size sortable"      @click="setSort('size')">
              Taille
              <ChevronUp v-if="sortKey==='size'&&sortDir==='asc'" :size="11" class="sort-ic"/>
              <ChevronDown v-if="sortKey==='size'&&sortDir==='desc'" :size="11" class="sort-ic"/>
            </th>
            <th class="col-date sortable"      @click="setSort('date')">
              Installé le
              <ChevronUp v-if="sortKey==='date'&&sortDir==='asc'" :size="11" class="sort-ic"/>
              <ChevronDown v-if="sortKey==='date'&&sortDir==='desc'" :size="11" class="sort-ic"/>
            </th>
            <th class="col-source">Source</th>
            <th class="col-keep">AppData</th>
            <th class="col-preview"></th>
            <th class="col-force"></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="app in sorted" :key="app.name"
            class="app-row" :class="{ selected: selected.has(app.name) }"
            @click="toggleSelect(app.name)"
          >
            <td class="col-check">
              <CheckSquare v-if="selected.has(app.name)" :size="14" style="color:var(--accent-primary)" />
              <Square v-else :size="14" style="color:var(--text-muted)" />
            </td>
            <td class="col-name">
              <span class="app-icon"><Package :size="13" /></span>
              {{ app.name }}
              <span class="cat-badge" :style="{ background: CAT_COLORS[appCategory(app)]+'22', color: CAT_COLORS[appCategory(app)], borderColor: CAT_COLORS[appCategory(app)]+'55' }"
                :title="appCategory(app)==='system'?'⚠ Composant système — ne pas désinstaller':appCategory(app)==='redist'?'Redistribuable (C++, .NET, Java...)':CAT_LABELS[appCategory(app)]">
                {{ appCategory(app) === 'system' ? '⚠ Système' : appCategory(app) === 'redist' ? 'Redist' : CAT_LABELS[appCategory(app)] }}
              </span>
            </td>
            <td class="col-version">{{ app.version || '—' }}</td>
            <td class="col-publisher" @click.stop="selectByPublisher(app.publisher)" :title="`Sélectionner tous : ${app.publisher}`" style="cursor:pointer">{{ app.publisher || '—' }}</td>
            <td class="col-size">{{ formatSize(app.install_size_kb) }}</td>
            <td class="col-date">{{ app.install_date || '—' }}</td>
            <td class="col-source" @click.stop>
              <span v-if="app.source" class="src-badge" :class="sourceLabel(app.source).toLowerCase()">{{ sourceLabel(app.source) }}</span>
            </td>
            <!-- Conserver AppData -->
            <td class="col-keep" @click.stop>
              <button
                class="keep-btn"
                :class="{ active: keepAppDataMap[app.name] }"
                :title="keepAppDataMap[app.name] ? 'AppData conservée' : 'Conserver les données utilisateur'"
                @click="toggleKeepAppData(app.name)"
              >
                <Archive :size="12" />
              </button>
            </td>
            <td class="col-preview" @click.stop>
              <button class="preview-btn" @click="showPreview(app)" title="Voir les résidus potentiels">
                <Eye :size="13" />
              </button>
            </td>
            <!-- Force remove -->
            <td class="col-force" @click.stop>
              <button
                v-if="!app.uninstall_string"
                class="force-btn"
                title="Forcer la suppression (pas de désinstalleur détecté)"
                @click="startForceRemove(app)"
              >
                <AlertOctagon :size="12" />
              </button>
            </td>
          </tr>
        </tbody>
      </table>
      <p v-if="sorted.length === 0" class="no-results">Aucun résultat pour « {{ search }} »</p>
    </div>

    <!-- Empty -->
    <div v-else class="empty-state">
      <AlertTriangle :size="32" style="color:var(--text-muted)" />
      <p>Aucune application détectée</p>
      <NButton variant="ghost" @click="loadApps"><RefreshCw :size="13" /> Réessayer</NButton>
    </div>
    </template><!-- fin apps -->
  </div>
</template>

<style scoped src="./UninstallerPage.css"></style>