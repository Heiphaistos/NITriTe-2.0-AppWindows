<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import NButton from "@/components/ui/NButton.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import { useNotificationStore } from "@/stores/notifications";
import RecoveryTabDisk from "@/components/recovery/RecoveryTabDisk.vue";
import RecoveryShadowCompare from "@/components/recovery/RecoveryShadowCompare.vue";
import DiskImagerTab from "@/components/recovery/DiskImagerTab.vue";
import RecoveryTabMft from "@/components/recovery/RecoveryTabMft.vue";
import RecoveryTabBackup from "@/components/recovery/RecoveryTabBackup.vue";
import {
  Database, RefreshCw, RotateCcw, Clock, Trash2,
  FileText, CheckCircle, Folder, Search,
  Save, CheckSquare, Square, FolderOpen, HardDrive, GitCompare, Filter,
  Shield, Lightbulb, Plus,
} from "lucide-vue-next";

const notify = useNotificationStore();

// ── NTFS drives dynamiques ─────────────────────────────────────────────────────
const ntfsDrives = ref<string[]>([]);
async function loadNtfsDrives() {
  try { ntfsDrives.value = await invoke<string[]>("get_ntfs_drives"); }
  catch { notify.warning("Lecteurs NTFS", "Impossible de lister les lecteurs NTFS disponibles."); }
}

// ── Shadow Copy create/delete ──────────────────────────────────────────────────
const createShadowVolume = ref("C:");
const creatingShadow = ref(false);

async function createShadow() {
  creatingShadow.value = true;
  try {
    const id = await invoke<string>("create_shadow_copy_cmd", { volume: createShadowVolume.value });
    notify.success("Shadow copy créée", `ID : ${id}`);
    await loadShadows();
  } catch (e: any) { notify.error("Erreur création", String(e)); }
  creatingShadow.value = false;
}

async function deleteShadow(s: ShadowCopy) {
  try {
    await invoke<string>("delete_shadow_copy_cmd", { shadowId: s.id });
    notify.success("Shadow copy supprimée", "");
    shadows.value = shadows.value.filter(x => x.id !== s.id);
    if (selectedShadow.value?.id === s.id) { selectedShadow.value = null; shadowFiles.value = []; }
  } catch (e: any) { notify.error("Erreur suppression", String(e)); }
}

// ── Ouvrir dossier restauration ────────────────────────────────────────────────
async function openRestoreFolder(path: string) {
  try { await invoke("open_in_explorer", { path }); }
  catch (e: any) { notify.error("Erreur", String(e)); }
}

interface ShadowCopy {
  id: string; volume: string; creation_time: string;
  provider: string; device_path: string;
}
interface RecoveredFile {
  name: string; path: string; size_bytes: number; deleted_date: string; source: string; is_dir: boolean;
}
interface RestoreResult { success: boolean; message: string; restored_path: string; }
interface BatchRestoreResult { success: boolean; restored_count: number; failed_count: number; message: string; }

type Tab = "shadow" | "recycle" | "mft" | "backup" | "disk" | "image" | "rapport";
const activeTab = ref<Tab>("shadow");

// ── Image disque / Deep scan / Rapport ────────────────────────────────────────
interface DeepMftFile { name: string; path: string; size_bytes: number; modified: string; extension: string; is_deleted: boolean; source: string; }
const deepFiles = ref<DeepMftFile[]>([]);
const deepDrive = ref("C:");
const scanningDeep = ref(false);
const reportPath = ref("");
const generatingReport = ref(false);

async function runDeepScan() {
  scanningDeep.value = true; deepFiles.value = [];
  try {
    deepFiles.value = await invoke<DeepMftFile[]>("deep_mft_scan_advanced_cmd", { drive: deepDrive.value });
  } catch (e: any) { notify.error("Erreur scan MFT", String(e)); }
  scanningDeep.value = false;
}
async function makeReport() {
  if (deepFiles.value.length === 0) { notify.warning("Aucun fichier", "Lancez d'abord un scan."); return; }
  generatingReport.value = true;
  try {
    const outPath = `C:\\NiTriTe\\rapport_recovery_${Date.now()}.html`;
    const path = await invoke<string>("generate_recovery_report_cmd", {
      title: "Rapport de Récupération Nitrite",
      filesJson: JSON.stringify(deepFiles.value),
      outputPath: outPath,
    });
    reportPath.value = path;
    notify.success("Rapport généré", path);
  } catch (e: any) { notify.error("Erreur rapport", String(e)); }
  generatingReport.value = false;
}
async function openReport() {
  if (reportPath.value) await invoke("open_in_explorer", { path: reportPath.value.split("\\").slice(0, -1).join("\\") }).catch(() => {});
}

// ── Shadow Copies ─────────────────────────────────────────────
const shadows = ref<ShadowCopy[]>([]);
const loadingShadows = ref(false);
const selectedShadow = ref<ShadowCopy | null>(null);
const shadowFiles = ref<RecoveredFile[]>([]);
const browsingPath = ref(""); // chemin UNC complet courant, ou "" pour la racine
const pathStack = ref<string[]>([]); // chemins UNC complets pour le fil d'Ariane
// Affichage lisible du chemin (sans le préfixe \\?\GLOBALROOT\Device\ShadowCopyX)
interface UserFolder { name: string; path: string; shadow_relative: string }
const userFolders = ref<UserFolder[]>([]);

const browsingDisplay = computed(() => {
  if (!browsingPath.value) return "Racine";
  const parts = browsingPath.value.split("\\").filter(Boolean);
  return parts.slice(4).join("\\") || "Racine";
});
const loadingBrowse = ref(false);
const restoreTarget = ref("C:\\NiTriTe\\Restaurés");
const selectedFiles = ref<Set<string>>(new Set());
const searchQuery = ref("");
const isSearching = ref(false);
const searchMode = ref(false);
const batchRestoring = ref(false);
const extFilter = ref(""); // E — filtre par extension (ex: ".jpg,.png")
const showCompare = ref(false); // F — panneau comparaison
const showDeletedOnly = ref(false); // G — seulement fichiers supprimés
const shadowSortKey = ref<"name" | "size" | "date">("name");
const shadowSortDir = ref<"asc" | "desc">("asc");

function setShadowSort(key: "name" | "size" | "date") {
  if (shadowSortKey.value === key) shadowSortDir.value = shadowSortDir.value === "asc" ? "desc" : "asc";
  else { shadowSortKey.value = key; shadowSortDir.value = "asc"; }
}

// E — fichiers filtrés par extension + supprimés seulement
const filteredShadowFiles = computed(() => {
  let files = shadowFiles.value;
  if (showDeletedOnly.value) files = files.filter(f => f.is_dir || f.source === "deleted" || f.deleted_date);
  if (!extFilter.value.trim()) return files;
  const exts = extFilter.value.toLowerCase().split(",").map(e => e.trim()).filter(Boolean);
  return files.filter(f => {
    if (f.is_dir) return true;
    const dot = f.name.lastIndexOf(".");
    const ext = dot >= 0 ? f.name.slice(dot).toLowerCase() : "";
    return exts.some(e => e === ext || e === ext.slice(1));
  });
});

// Tri des fichiers
const sortedShadowFiles = computed(() => {
  const list = [...filteredShadowFiles.value];
  const dir = shadowSortDir.value === "asc" ? 1 : -1;
  list.sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
    switch (shadowSortKey.value) {
      case "name": return dir * a.name.localeCompare(b.name);
      case "size": return dir * (a.size_bytes - b.size_bytes);
      case "date": return dir * a.deleted_date.localeCompare(b.deleted_date);
    }
  });
  return list;
});

// Taille totale sélectionnée
const totalSelectedBytes = computed(() =>
  shadowFiles.value.filter(f => selectedFiles.value.has(f.path)).reduce((a, f) => a + f.size_bytes, 0)
);
function formatSizeShort(b: number) {
  if (b >= 1_073_741_824) return `${(b / 1_073_741_824).toFixed(1)} Go`;
  if (b >= 1_048_576) return `${(b / 1_048_576).toFixed(1)} Mo`;
  if (b >= 1024) return `${(b / 1024).toFixed(0)} Ko`;
  return `${b} o`;
}

async function loadShadows() {
  loadingShadows.value = true; shadows.value = [];
  try {
    shadows.value = await invoke<ShadowCopy[]>("list_shadow_copies");
  } catch (e: any) {
    notify.error("Erreur Shadow Copy", String(e));
  }
  loadingShadows.value = false;
}

async function browseShadow(shadow: ShadowCopy, path: string = "") {
  selectedShadow.value = shadow;
  selectedFiles.value = new Set();
  searchMode.value = false;
  searchQuery.value = "";
  // Normalise vers chemin UNC complet (shadow_path() côté Rust gère les deux formes)
  const devicePath = shadow.device_path.replace(/\\+$/, "");
  let fullPath: string;
  if (!path) {
    fullPath = "";
  } else if (path.startsWith("\\\\?\\") || path.startsWith("\\\\")) {
    fullPath = path.replace(/\\+$/, "");
  } else {
    fullPath = devicePath + "\\" + path.replace(/^\\+/, "");
  }
  // Reconstruire le fil d'Ariane avec chemins UNC complets
  if (!fullPath) {
    pathStack.value = [];
  } else {
    const rel = fullPath.startsWith(devicePath)
      ? fullPath.slice(devicePath.length).replace(/^\\+/, "")
      : fullPath.split("\\").filter(Boolean).slice(4).join("\\");
    let acc = devicePath;
    pathStack.value = rel.split("\\").filter(Boolean).map(part => {
      acc = acc + "\\" + part;
      return acc;
    });
  }
  browsingPath.value = fullPath;
  loadingBrowse.value = true;
  try {
    shadowFiles.value = await invoke<RecoveredFile[]>("browse_shadow_copy", {
      devicePath: shadow.device_path,
      relativePath: fullPath,
    });
  } catch (e: any) {
    notify.error("Erreur navigation", String(e));
  }
  loadingBrowse.value = false;
}

async function navigateInto(file: RecoveredFile) {
  if (!selectedShadow.value || !file.is_dir) return;
  // file.path de PowerShell (FullName) est déjà le chemin UNC complet
  await browseShadow(selectedShadow.value, file.path);
}

async function navigateTo(path: string) {
  if (!selectedShadow.value) return;
  await browseShadow(selectedShadow.value, path);
}

async function searchInShadow() {
  if (!selectedShadow.value || !searchQuery.value.trim()) return;
  searchMode.value = true;
  isSearching.value = true;
  selectedFiles.value = new Set();
  try {
    shadowFiles.value = await invoke<RecoveredFile[]>("search_shadow_copy", {
      devicePath: selectedShadow.value.device_path,
      query: searchQuery.value.trim(),
      basePath: browsingPath.value,
    });
  } catch (e: any) {
    notify.error("Erreur recherche", String(e));
  }
  isSearching.value = false;
}

function toggleFile(path: string) {
  const s = new Set(selectedFiles.value);
  if (s.has(path)) s.delete(path); else s.add(path);
  selectedFiles.value = s;
}
function toggleAllFiles() {
  const files = filteredShadowFiles.value.filter(f => !f.is_dir);
  if (selectedFiles.value.size === files.length) {
    selectedFiles.value = new Set();
  } else {
    selectedFiles.value = new Set(files.map(f => f.path));
  }
}

async function batchRestore() {
  if (selectedFiles.value.size === 0) return;
  batchRestoring.value = true;
  try {
    const result = await invoke<BatchRestoreResult>("restore_files_batch", {
      files: Array.from(selectedFiles.value),
      targetFolder: restoreTarget.value,
    });
    if (result.success) notify.success(`${result.restored_count} fichier(s) restauré(s)`, result.message);
    else notify.error(`Échec (${result.failed_count} erreur(s))`, result.message);
    selectedFiles.value = new Set();
  } catch (e: any) {
    notify.error("Erreur restauration", String(e));
  }
  batchRestoring.value = false;
}

async function restoreFromShadow(file: RecoveredFile) {
  try {
    const result = await invoke<RestoreResult>("restore_from_shadow", {
      sourcePath: file.path,
      targetFolder: restoreTarget.value,
    });
    if (result.success) notify.success("Restauré", `→ ${result.restored_path}`);
    else notify.error("Échec restauration", result.message);
  } catch (e: any) {
    notify.error("Erreur", String(e));
  }
}

// ── Corbeille ─────────────────────────────────────────────────
const recycleFiles = ref<RecoveredFile[]>([]);
const loadingRecycle = ref(false);

async function loadRecycleBin() {
  loadingRecycle.value = true; recycleFiles.value = [];
  try {
    recycleFiles.value = await invoke<RecoveredFile[]>("scan_recycle_bin");
  } catch (e: any) {
    notify.error("Erreur Corbeille", String(e));
  }
  loadingRecycle.value = false;
}

async function restoreRecycle(file: RecoveredFile) {
  try {
    const result = await invoke<RestoreResult>("restore_recycle_bin_item", {
      originalPath: file.path,
    });
    if (result.success) {
      notify.success("Restauré", file.name);
      recycleFiles.value = recycleFiles.value.filter(f => f.path !== file.path);
    } else {
      notify.error("Échec", result.message);
    }
  } catch (e: any) {
    notify.error("Erreur", String(e));
  }
}



function formatDate(raw: string) {
  if (!raw) return "—";
  try { return new Date(raw).toLocaleString("fr-FR"); } catch { return raw; }
}
function formatSize(bytes: number) {
  if (!bytes) return "—";
  if (bytes < 1024) return `${bytes} o`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} Ko`;
  return `${(bytes / 1024 / 1024).toFixed(1)} Mo`;
}

onMounted(async () => {
  loadShadows();
  loadNtfsDrives();
  try {
    userFolders.value = await invoke<UserFolder[]>("get_user_profile_folders");
  } catch {
    userFolders.value = [];
  }
});
</script>

<template>
  <div class="recovery-page">

    <!-- Header Premium -->
    <div class="recovery-hero">
      <div class="hero-icon-wrap">
        <div class="hero-icon"><Database :size="26" /></div>
      </div>
      <div class="hero-text">
        <h1 class="hero-title">Récupération de Données</h1>
        <p class="hero-desc">Restaurez vos fichiers depuis les Shadow Copies, la Corbeille, le journal NTFS ou un disque</p>
      </div>
    </div>

    <!-- Carte "Comment ça marche" -->
    <div class="how-card">
      <div class="how-title"><Lightbulb :size="14" /> Comment ça marche — 5 méthodes de récupération</div>
      <div class="how-grid">
        <div class="how-item"><span class="how-num">1</span><div><strong>Shadow Copy</strong> — Points de restauration VSS de Windows (le plus fiable)</div></div>
        <div class="how-item"><span class="how-num">2</span><div><strong>Corbeille</strong> — Fichiers récemment supprimés mais non vidés</div></div>
        <div class="how-item"><span class="how-num">3</span><div><strong>Journal NTFS</strong> — USN Journal : suppressions récentes sur volumes NTFS</div></div>
        <div class="how-item"><span class="how-num">4</span><div><strong>Sauvegarde Profil</strong> — Copie via Robocopy de vos dossiers utilisateur</div></div>
        <div class="how-item"><span class="how-num">5</span><div><strong>Récupération Disque</strong> — Scan bas niveau via outils tiers (Recuva/TestDisk)</div></div>
      </div>
    </div>

    <!-- Onglets améliorés -->
    <div class="tabs">
      <button class="tab-btn" :class="{ active: activeTab === 'shadow' }" @click="activeTab = 'shadow'; loadShadows()">
        <Clock :size="14" /> Points de Restauration
        <span v-if="shadows.length > 0" class="tab-badge">{{ shadows.length }}</span>
      </button>
      <button class="tab-btn" :class="{ active: activeTab === 'recycle' }" @click="activeTab = 'recycle'; loadRecycleBin()">
        <Trash2 :size="14" /> Corbeille
        <span v-if="recycleFiles.length > 0" class="tab-badge tab-badge-warn">{{ recycleFiles.length }}</span>
      </button>
      <button class="tab-btn" :class="{ active: activeTab === 'mft' }" @click="activeTab = 'mft'">
        <Search :size="14" /> Journal NTFS
      </button>
      <button class="tab-btn" :class="{ active: activeTab === 'backup' }" @click="activeTab = 'backup'">
        <Save :size="14" /> Sauvegarde Profil
      </button>
      <button class="tab-btn" :class="{ active: activeTab === 'disk' }" @click="activeTab = 'disk'">
        <HardDrive :size="14" /> Récupération Disque
      </button>
      <button class="tab-btn" :class="{ active: activeTab === 'image' }" @click="activeTab = 'image'">
        <Database :size="14" /> Image Disque
      </button>
      <button class="tab-btn" :class="{ active: activeTab === 'rapport' }" @click="activeTab = 'rapport'">
        <FileText :size="14" /> Rapport
      </button>
    </div>

    <!-- ══ SHADOW COPIES ══ -->
    <div v-if="activeTab === 'shadow'" class="tab-content">
      <div class="toolbar">
        <NButton variant="ghost" size="sm" :loading="loadingShadows" @click="loadShadows">
          <RefreshCw :size="13" /> Actualiser
        </NButton>
        <select v-model="createShadowVolume" class="drive-select-sm">
          <option v-for="d in ntfsDrives" :key="d" :value="d">{{ d }}</option>
          <option v-if="ntfsDrives.length === 0" value="C:">C:</option>
        </select>
        <NButton variant="ghost" size="sm" :loading="creatingShadow" @click="createShadow">
          <Plus :size="13" /> Créer snapshot
        </NButton>
        <span class="count">{{ shadows.length }} point(s) de restauration trouvé(s)</span>
      </div>

      <div v-if="loadingShadows" class="loading-state"><NSpinner :size="20" /><span>Recherche des Shadow Copies...</span></div>
      <div v-else-if="shadows.length === 0" class="empty">
        <Clock :size="28" />
        <p>Aucun point de restauration VSS trouvé</p>
        <p class="hint">Activez la protection du système dans Propriétés Système → Protection du système</p>
      </div>
      <div v-else>
        <div class="shadows-list">
          <div
            v-for="s in shadows" :key="s.id"
            class="shadow-card"
            :class="{ selected: selectedShadow?.id === s.id }"
          >
            <div class="shadow-header">
              <div class="shadow-icon"><Clock :size="16" /></div>
              <div class="shadow-info">
                <span class="shadow-date">{{ formatDate(s.creation_time) }}</span>
                <div class="shadow-meta-row">
                  <span class="shadow-vol">Volume : {{ s.volume }}</span>
                  <span v-if="s.provider" class="shadow-provider">· {{ s.provider }}</span>
                </div>
              </div>
              <NButton variant="primary" size="sm" @click="browseShadow(s)">
                <Folder :size="12" /> Parcourir
              </NButton>
              <NButton variant="ghost" size="sm" style="color:var(--danger)" title="Supprimer" @click.stop="deleteShadow(s)">
                <Trash2 :size="12" />
              </NButton>
            </div>
          </div>
        </div>

        <!-- Navigateur de fichiers de la shadow copy -->
        <div v-if="selectedShadow" class="file-browser">
          <!-- En-tête : destination + recherche -->
          <div class="browser-header">
            <div class="restore-target-row">
              <label class="target-label">Restaurer vers :</label>
              <input v-model="restoreTarget" class="target-input" />
            </div>
            <div class="search-row">
              <input
                v-model="searchQuery"
                class="search-input"
                placeholder="Rechercher un fichier..."
                @keydown.enter="searchInShadow"
              />
              <NButton variant="primary" size="sm" :loading="isSearching" @click="searchInShadow">
                <Search :size="12" /> Chercher
              </NButton>
              <NButton v-if="searchMode" variant="ghost" size="sm" @click="browseShadow(selectedShadow!, browsingPath)">
                ✕ Effacer
              </NButton>
              <!-- G — Supprimés seulement -->
              <button class="ext-preset-btn" :class="{ active: showDeletedOnly }"
                @click="showDeletedOnly = !showDeletedOnly" title="Afficher seulement les fichiers supprimés">🗑</button>
              <!-- E — Filtre extension + raccourcis type -->
              <div class="ext-filter-wrap">
                <Filter :size="12" class="ext-icon" />
                <input v-model="extFilter" class="ext-input" placeholder=".jpg,.docx…" title="Filtrer par extension" />
              </div>
              <div class="ext-presets">
                <button class="ext-preset-btn" :class="{ active: extFilter === '.jpg,.jpeg,.png,.gif,.bmp,.webp,.heic' }"
                  @click="extFilter = extFilter === '.jpg,.jpeg,.png,.gif,.bmp,.webp,.heic' ? '' : '.jpg,.jpeg,.png,.gif,.bmp,.webp,.heic'" title="Images">🖼</button>
                <button class="ext-preset-btn" :class="{ active: extFilter === '.mp4,.mkv,.avi,.mov,.wmv,.m4v' }"
                  @click="extFilter = extFilter === '.mp4,.mkv,.avi,.mov,.wmv,.m4v' ? '' : '.mp4,.mkv,.avi,.mov,.wmv,.m4v'" title="Vidéos">🎬</button>
                <button class="ext-preset-btn" :class="{ active: extFilter === '.mp3,.flac,.wav,.ogg,.aac,.m4a' }"
                  @click="extFilter = extFilter === '.mp3,.flac,.wav,.ogg,.aac,.m4a' ? '' : '.mp3,.flac,.wav,.ogg,.aac,.m4a'" title="Musique">🎵</button>
                <button class="ext-preset-btn" :class="{ active: extFilter === '.doc,.docx,.pdf,.xls,.xlsx,.ppt,.pptx,.txt,.odt' }"
                  @click="extFilter = extFilter === '.doc,.docx,.pdf,.xls,.xlsx,.ppt,.pptx,.txt,.odt' ? '' : '.doc,.docx,.pdf,.xls,.xlsx,.ppt,.pptx,.txt,.odt'" title="Documents">📄</button>
                <button class="ext-preset-btn" :class="{ active: extFilter === '.zip,.rar,.7z,.tar,.gz' }"
                  @click="extFilter = extFilter === '.zip,.rar,.7z,.tar,.gz' ? '' : '.zip,.rar,.7z,.tar,.gz'" title="Archives">📦</button>
                <button v-if="extFilter" class="ext-preset-btn ext-clear" @click="extFilter = ''" title="Effacer filtre">✕</button>
              </div>
              <!-- F — Comparaison -->
              <NButton variant="ghost" size="sm" @click="showCompare = !showCompare">
                <GitCompare :size="12" /> {{ showCompare ? 'Fermer' : 'Comparer' }}
              </NButton>
            </div>
          </div>

          <!-- Fil d'Ariane (breadcrumb) -->
          <div class="breadcrumb" v-if="!searchMode">
            <button class="crumb" :class="{ current: pathStack.length === 0 }" @click="pathStack.length > 0 && navigateTo('')">🖥 Racine</button>
            <template v-for="(seg, i) in pathStack" :key="seg">
              <span class="crumb-sep">/</span>
              <button
                class="crumb"
                :class="{ current: i === pathStack.length - 1 }"
                @click="i < pathStack.length - 1 && navigateTo(seg)"
              >{{ seg.split('\\').filter((s: string) => s).pop() }}</button>
            </template>
          </div>
          <div v-else class="search-info">
            🔍 Résultats pour "<strong>{{ searchQuery }}</strong>" dans {{ browsingDisplay }}
            — {{ shadowFiles.length }} fichier(s) trouvé(s)
          </div>

          <!-- F — Comparaison -->
          <RecoveryShadowCompare
            v-if="showCompare && selectedShadow && !searchMode"
            :device-path="selectedShadow.device_path"
            :sub-path="browsingPath"
          />

          <!-- Actions batch -->
          <div class="batch-bar" v-if="sortedShadowFiles.length > 0">
            <label class="check-all" @click="toggleAllFiles">
              <CheckSquare v-if="selectedFiles.size > 0 && selectedFiles.size === filteredShadowFiles.filter(f => !f.is_dir).length" :size="14" />
              <Square v-else :size="14" />
              <span>{{ selectedFiles.size > 0 ? `${selectedFiles.size} sélectionné(s)` : 'Tout sélectionner' }}</span>
            </label>
            <span v-if="selectedFiles.size > 0" class="total-sel-size">{{ formatSizeShort(totalSelectedBytes) }}</span>
            <NButton
              v-if="selectedFiles.size > 0"
              variant="primary"
              size="sm"
              :loading="batchRestoring"
              @click="batchRestore"
            >
              <RotateCcw :size="12" /> Restaurer ({{ selectedFiles.size }})
            </NButton>
            <NButton variant="ghost" size="sm" @click="openRestoreFolder(restoreTarget)">
              <FolderOpen :size="12" /> Ouvrir dossier
            </NButton>
            <span v-if="extFilter || showDeletedOnly" class="count">{{ filteredShadowFiles.length }} / {{ shadowFiles.length }} fichier(s)</span>
          </div>

          <div v-if="loadingBrowse || isSearching" class="loading-state"><NSpinner :size="16" /><span>{{ isSearching ? 'Recherche...' : 'Chargement...' }}</span></div>
          <div v-else-if="filteredShadowFiles.length === 0" class="empty-small">
            {{ extFilter ? `Aucun fichier avec ces extensions` : searchMode ? 'Aucun fichier trouvé pour cette recherche' : 'Dossier vide ou inaccessible' }}
          </div>
          <div v-else class="files-table">
            <div class="file-row header-row">
              <span></span>
              <span class="sortable-col" @click="setShadowSort('name')">
                Nom <span class="sort-arrow">{{ shadowSortKey==='name' ? (shadowSortDir==='asc'?'↑':'↓') : '' }}</span>
              </span>
              <span class="sortable-col" @click="setShadowSort('size')">
                Taille <span class="sort-arrow">{{ shadowSortKey==='size' ? (shadowSortDir==='asc'?'↑':'↓') : '' }}</span>
              </span>
              <span class="sortable-col" @click="setShadowSort('date')">
                Modifié <span class="sort-arrow">{{ shadowSortKey==='date' ? (shadowSortDir==='asc'?'↑':'↓') : '' }}</span>
              </span>
              <span></span>
            </div>
            <div
              v-for="f in sortedShadowFiles" :key="f.path"
              class="file-row"
              :class="{ 'is-dir': f.is_dir, 'is-selected': selectedFiles.has(f.path) }"
            >
              <span class="file-check">
                <template v-if="!f.is_dir">
                  <CheckSquare v-if="selectedFiles.has(f.path)" :size="13" class="chk on" @click.stop="toggleFile(f.path)" />
                  <Square v-else :size="13" class="chk" @click.stop="toggleFile(f.path)" />
                </template>
              </span>
              <span
                class="file-name"
                :style="f.is_dir ? 'cursor:pointer;color:var(--accent-primary)' : ''"
                @click="f.is_dir ? navigateInto(f) : undefined"
              >
                <Folder v-if="f.is_dir" :size="12" style="color:var(--accent-primary)" />
                <FileText v-else :size="12" />
                {{ f.name }}
              </span>
              <span class="file-size">{{ f.is_dir ? '—' : formatSize(f.size_bytes) }}</span>
              <span class="file-date">{{ formatDate(f.deleted_date) }}</span>
              <NButton v-if="!f.is_dir" variant="ghost" size="sm" @click="restoreFromShadow(f)">
                <RotateCcw :size="11" /> Restaurer
              </NButton>
              <NButton v-else variant="ghost" size="sm" @click="navigateInto(f)">
                → Ouvrir
              </NButton>
            </div>
          </div>

          <div class="browse-shortcuts">
            <NButton variant="ghost" size="sm" @click="browseShadow(selectedShadow!, '')">📁 Racine</NButton>
            <NButton variant="ghost" size="sm" @click="browseShadow(selectedShadow!, 'Users')">📁 Utilisateurs</NButton>
            <template v-for="f in userFolders.filter(x => ['Documents','Bureau','Images','Telechargements'].includes(x.name))" :key="f.path">
              <NButton variant="ghost" size="sm" @click="browseShadow(selectedShadow!, f.shadow_relative)">
                📂 {{ f.name }}
              </NButton>
            </template>
          </div>
        </div>
      </div>
    </div>

    <!-- ══ CORBEILLE ══ -->
    <div v-if="activeTab === 'recycle'" class="tab-content">
      <div class="toolbar">
        <NButton variant="ghost" size="sm" :loading="loadingRecycle" @click="loadRecycleBin">
          <RefreshCw :size="13" /> Actualiser
        </NButton>
        <span class="count">{{ recycleFiles.length }} fichier(s) dans la corbeille</span>
      </div>

      <div v-if="loadingRecycle" class="loading-state"><NSpinner :size="20" /><span>Lecture de la Corbeille...</span></div>
      <div v-else-if="recycleFiles.length === 0" class="empty">
        <Trash2 :size="28" />
        <p>La Corbeille est vide</p>
        <p class="hint">Aucun fichier supprimé récemment à restaurer</p>
      </div>
      <div v-else>
        <div class="recycle-info-banner">
          <Shield :size="13" />
          <span>{{ recycleFiles.length }} fichier(s) supprimé(s) récupérables. Cliquez "Restaurer" pour les remettre à leur emplacement d'origine.</span>
        </div>
        <div class="files-table files-table-simple">
          <div class="file-row-simple header-row">
            <span>Nom</span><span>Taille</span><span>Supprimé le</span><span></span>
          </div>
          <div v-for="f in recycleFiles" :key="f.path" class="file-row-simple">
            <span class="file-name"><FileText :size="12" /> {{ f.name }}</span>
            <span class="file-size">{{ formatSize(f.size_bytes) }}</span>
            <span class="file-date">{{ formatDate(f.deleted_date) }}</span>
            <NButton variant="ghost" size="sm" @click="restoreRecycle(f)">
              <RotateCcw :size="11" /> Restaurer
            </NButton>
          </div>
        </div>
      </div>
    </div>

    <!-- ══ JOURNAL MFT ══ -->
    <div v-if="activeTab === 'mft'" class="tab-content">
      <RecoveryTabMft />
    </div>

    <!-- ══ SAUVEGARDE PROFIL ══ -->
    <div v-if="activeTab === 'backup'" class="tab-content">
      <RecoveryTabBackup />
    </div>

    <!-- ══ RÉCUPÉRATION DISQUE ══ -->
    <div v-if="activeTab === 'disk'" class="tab-content">
      <RecoveryTabDisk />
    </div>

    <!-- ══ IMAGE DISQUE ══ -->
    <div v-if="activeTab === 'image'" class="tab-content">
      <DiskImagerTab />
    </div>

    <!-- ══ RAPPORT ══ -->
    <div v-if="activeTab === 'rapport'" class="tab-content">
      <div class="rapport-toolbar">
        <select v-model="deepDrive" class="drive-select-sm">
          <option v-for="d in ntfsDrives" :key="d" :value="d">{{ d }}</option>
          <option v-if="ntfsDrives.length === 0" value="C:">C:</option>
        </select>
        <NButton variant="primary" size="sm" :loading="scanningDeep" @click="runDeepScan">
          <Search :size="13" /> Scanner fichiers supprimés
        </NButton>
        <NButton variant="secondary" size="sm" :loading="generatingReport" :disabled="deepFiles.length === 0" @click="makeReport">
          <FileText :size="13" /> Générer rapport HTML
        </NButton>
        <NButton v-if="reportPath" variant="ghost" size="sm" @click="openReport">
          <FolderOpen :size="13" /> Ouvrir
        </NButton>
        <span class="count" v-if="deepFiles.length > 0">{{ deepFiles.length }} fichier(s) trouvé(s)</span>
      </div>

      <div v-if="scanningDeep" class="loading-state"><NSpinner :size="20" /><span>Scan MFT en cours...</span></div>
      <div v-else-if="deepFiles.length === 0" class="empty">
        <FileText :size="28" />
        <p>Aucun fichier scanné</p>
        <p class="hint">Sélectionnez un volume et lancez le scan pour détecter les fichiers supprimés</p>
      </div>
      <div v-else class="mft-table">
        <div class="mft-row hdr">
          <span>Nom</span><span>Chemin</span><span>Taille</span><span>Source</span><span>Statut</span>
        </div>
        <div v-for="f in deepFiles" :key="f.path" class="mft-row">
          <span class="mft-name">{{ f.name }}</span>
          <span class="mft-path">{{ f.path }}</span>
          <span class="mft-size">{{ f.size_bytes > 0 ? formatSize(f.size_bytes) : '—' }}</span>
          <span class="mft-source">{{ f.source }}</span>
          <span :style="{ color: f.is_deleted ? 'var(--danger)' : 'var(--success)' }">{{ f.is_deleted ? 'Supprimé' : 'Actif' }}</span>
        </div>
      </div>
      <div v-if="reportPath" class="report-path-box">
        <CheckCircle :size="13" style="color:var(--success)" />
        Rapport généré : <code>{{ reportPath }}</code>
      </div>
    </div>
  </div>
</template>

<style scoped src="./DataRecoveryPage.css"></style>
