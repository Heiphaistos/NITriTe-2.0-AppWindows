<script setup lang="ts">
defineOptions({ name: "BigFilesFinderPage" });
import { ref, computed } from "vue";
import { csvCell } from "@/composables/useExportData";
import { invoke, invokeRaw } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import { useNotificationStore } from "@/stores/notifications";
import { FileSearch, FolderOpen, Copy, Trash2, Download, Plus, X } from "lucide-vue-next";

const notify = useNotificationStore();

interface BigFile { name: string; path: string; size_bytes: number; extension: string; modified: string; }

const files      = ref<BigFile[]>([]);
const loading    = ref(false);
const scanPath   = ref("C:\\");
const countLimit = ref(50);
const filterExt  = ref("");
const deleting   = ref<string | null>(null);
const confirmDelete = ref<string | null>(null);
const trashMode  = ref(true);
const hasScanned = ref(false);

// Exclusions de dossiers
const excludedFolders = ref<string[]>(["C:\\Windows", "C:\\$Recycle.Bin"]);
const newExclusion    = ref("");

function addExclusion() {
  const v = newExclusion.value.trim();
  if (v && !excludedFolders.value.includes(v)) {
    excludedFolders.value.push(v);
    newExclusion.value = "";
  }
}
function removeExclusion(folder: string) {
  excludedFolders.value = excludedFolders.value.filter(f => f !== folder);
}

// Filtre extensions (multi, séparées par des virgules). On normalise chaque
// entrée sans le point de tête ("png" ou ".png" → "png") pour une comparaison
// EXACTE : un includes() ferait matcher « png » sur « .apng » et « zip » sur
// « .zip.tmp », sur-incluant des fichiers non voulus.
const filterExts = computed(() => {
  if (!filterExt.value.trim()) return [];
  return filterExt.value.split(",")
    .map(e => e.trim().toLowerCase().replace(/^\./, ""))
    .filter(Boolean);
});

const displayed = computed(() => {
  if (filterExts.value.length === 0) return files.value;
  return files.value.filter(f => {
    const ext = f.extension.toLowerCase().replace(/^\./, "");
    return filterExts.value.includes(ext);
  });
});

const totalSize = computed(() =>
  files.value.reduce((acc, f) => acc + f.size_bytes, 0)
);
const displayedSize = computed(() =>
  displayed.value.reduce((acc, f) => acc + f.size_bytes, 0)
);

function formatSize(bytes: number): string {
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function barWidth(bytes: number): string {
  const max = files.value[0]?.size_bytes || 1;
  return `${(bytes / max) * 100}%`;
}

function extColor(ext: string): string {
  const map: Record<string, string> = {
    ".mp4":"#ef4444", ".mkv":"#ef4444", ".avi":"#f97316", ".mov":"#f97316",
    ".iso":"#8b5cf6", ".zip":"#3b82f6", ".rar":"#3b82f6", ".7z":"#3b82f6",
    ".exe":"#22c55e", ".msi":"#22c55e", ".vhd":"#eab308", ".vmdk":"#eab308",
    ".pdf":"#f97316", ".docx":"#3b82f6", ".xlsx":"#22c55e", ".psd":"#8b5cf6",
    ".ai":"#f59e0b", ".blend":"#f97316",
  };
  return map[ext.toLowerCase()] || "#6b7280";
}

async function scan() {
  const trimmedPath = scanPath.value.trim();
  if (!trimmedPath) {
    notify.error("Chemin invalide", "Veuillez saisir un chemin de scan valide.");
    return;
  }
  loading.value = true;
  hasScanned.value = true;
  files.value = [];
  confirmDelete.value = null;
  try {
    files.value = await invokeRaw<BigFile[]>("get_big_files", {
      path: trimmedPath,
      count: countLimit.value,
      excludedFolders: Array.from(excludedFolders.value),
    });
    if (files.value.length === 0) notify.info("Aucun fichier", "Aucun gros fichier trouvé");
    else notify.success("Scan terminé", `${files.value.length} fichier(s) — ${formatSize(totalSize.value)} au total`);
  } catch (e: unknown) {
    notify.error("Erreur scan", String(e));
  } finally {
    loading.value = false;
  }
}

async function openFolder(path: string) {
  const folder = path.substring(0, path.lastIndexOf("\\"));
  try { await invoke("open_path", { path: folder }); } catch {}
}

function copyPath(path: string) {
  navigator.clipboard.writeText(path).then(() => notify.success("Copié", "Chemin copié"));
}

async function deleteFile(f: BigFile) {
  if (confirmDelete.value !== f.path) {
    confirmDelete.value = f.path;
    return;
  }
  confirmDelete.value = null;
  deleting.value = f.path;
  try {
    if (trashMode.value) {
      try {
        await invoke("trash_file", { path: f.path });
        notify.success("Corbeille", f.name);
      } catch {
        // trash_file non supporté — suppression définitive
        notify.warning("Corbeille indisponible — suppression définitive", f.name);
        await invoke("delete_file", { path: f.path });
        notify.success("Supprimé définitivement", f.name);
      }
    } else {
      await invoke("delete_file", { path: f.path });
      notify.success("Supprimé définitivement", f.name);
    }
    files.value = files.value.filter(x => x.path !== f.path);
  } catch (e: unknown) {
    notify.error("Erreur suppression", String(e));
  } finally {
    deleting.value = null;
  }
}

function cancelDelete() { confirmDelete.value = null; }

function exportCsv() {
  const rows = ["Rang,Nom,Chemin,Taille (octets),Taille lisible,Extension,Modifié",
    ...displayed.value.map((f, i) =>
      [i+1, f.name, f.path, f.size_bytes, formatSize(f.size_bytes), f.extension, f.modified].map(csvCell).join(",")
    )
  ].join("\n");
  const blob = new Blob([rows], { type: "text/csv" });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = `big_files_${new Date().toISOString().slice(0,10)}.csv`;
  a.click();
}
</script>

<template>
  <div class="bigfiles-page">
    <div class="page-header">
      <div>
        <h1>Chercheur de Gros Fichiers</h1>
        <p class="page-subtitle">Top {{ countLimit }} fichiers les plus volumineux — avec exclusions et filtre d'extension</p>
      </div>
      <div v-if="files.length" style="display:flex;gap:8px">
        <NButton variant="ghost" size="sm" @click="exportCsv"><Download :size="14" /> CSV</NButton>
      </div>
    </div>

    <NCard>
      <div class="controls">
        <input v-model="scanPath" class="path-input" placeholder="C:\\" @keyup.enter="scan" />
        <select v-model="countLimit" class="count-select">
          <option :value="20">Top 20</option>
          <option :value="50">Top 50</option>
          <option :value="100">Top 100</option>
          <option :value="200">Top 200</option>
        </select>
        <NButton variant="primary" :loading="loading" @click="scan">
          <FileSearch :size="14" /> Analyser
        </NButton>
      </div>

      <!-- Exclusions dossiers -->
      <div class="exclusions-section">
        <div class="exclusions-header">
          <span class="field-label">Dossiers exclus</span>
          <span class="exclusions-hint">Ces dossiers seront ignorés lors du scan</span>
        </div>
        <div class="exclusions-list">
          <div v-for="folder in excludedFolders" :key="folder" class="exclusion-tag">
            <span>{{ folder }}</span>
            <button class="exclusion-remove" @click="removeExclusion(folder)" title="Retirer"><X :size="10" /></button>
          </div>
        </div>
        <div class="exclusion-add-row">
          <input
            v-model="newExclusion"
            class="exclusion-input"
            placeholder="Ex: C:\Users\Momo\Téléchargements"
            @keyup.enter="addExclusion"
          />
          <NButton variant="ghost" size="sm" @click="addExclusion"><Plus :size="13" /> Ajouter</NButton>
        </div>
      </div>

      <!-- Mode suppression -->
      <div class="trash-mode-row">
        <label class="toggle-label">
          <input type="checkbox" v-model="trashMode" class="toggle-check" />
          <span class="toggle-text">Envoyer à la corbeille</span>
          <span class="toggle-hint">(décoché = suppression définitive)</span>
        </label>
      </div>
    </NCard>

    <!-- Confirmation suppression -->
    <div v-if="confirmDelete" class="confirm-banner">
      <Trash2 :size="14" style="color:var(--danger)" />
      <span>
        Confirmer {{ trashMode ? 'l\'envoi à la corbeille' : 'la suppression définitive' }} de
        <strong>{{ confirmDelete.split('\\').pop() }}</strong> ?
      </span>
      <NButton variant="danger" size="sm" @click="deleteFile(files.find(f=>f.path===confirmDelete)!)">
        {{ trashMode ? 'Corbeille' : 'Supprimer définitivement' }}
      </NButton>
      <NButton variant="ghost" size="sm" @click="cancelDelete">Annuler</NButton>
    </div>

    <NCard v-if="files.length > 0">
      <template #header>
        <div style="display:flex;align-items:center;gap:8px;width:100%;flex-wrap:wrap">
          <FileSearch :size="16" />
          <span>
            {{ displayed.length }} / {{ files.length }} fichiers ·
            <strong>{{ formatSize(displayedSize) }}</strong>
            <span v-if="filterExts.length" style="color:var(--text-muted);font-weight:400"> (filtré)</span>
          </span>
          <div class="ext-filter-wrap">
            <input
              v-model="filterExt"
              class="ext-filter"
              placeholder=".mp4, .iso, .zip"
              title="Filtrer par extension(s), séparées par virgule"
            />
          </div>
          <NButton variant="ghost" size="sm" @click="exportCsv" title="Exporter CSV">
            <Download :size="13" />
          </NButton>
        </div>
      </template>

      <div class="files-list">
        <div v-for="(f, i) in displayed" :key="f.path" class="file-row" :class="{ danger: confirmDelete === f.path }">
          <div class="rank">{{ i + 1 }}</div>
          <div class="ext-badge" :style="{ background: extColor(f.extension) + '22', color: extColor(f.extension), borderColor: extColor(f.extension) + '44' }">
            {{ f.extension || '??' }}
          </div>
          <div class="file-main">
            <div class="file-name">{{ f.name }}</div>
            <div class="bar-wrap">
              <div class="bar-fill" :style="{ width: barWidth(f.size_bytes), background: extColor(f.extension) }"></div>
            </div>
            <div class="file-path">{{ f.path }}</div>
          </div>
          <div class="file-meta">
            <span class="file-size">{{ formatSize(f.size_bytes) }}</span>
            <span class="file-date">{{ f.modified }}</span>
          </div>
          <div class="file-actions">
            <button class="icon-btn" @click="copyPath(f.path)" title="Copier chemin"><Copy :size="12" /></button>
            <button class="icon-btn" @click="openFolder(f.path)" title="Ouvrir dossier"><FolderOpen :size="12" /></button>
            <button
              class="icon-btn danger"
              :disabled="deleting === f.path"
              @click="deleteFile(f)"
              :title="confirmDelete === f.path ? 'Cliquer pour confirmer' : (trashMode ? 'Corbeille' : 'Supprimer')"
            >
              <NSpinner v-if="deleting === f.path" :size="10" />
              <Trash2 v-else :size="12" />
            </button>
          </div>
        </div>
      </div>
    </NCard>

    <div v-if="!loading && hasScanned && files.length === 0" style="text-align:center;padding:40px;color:var(--text-muted)">
      Aucun gros fichier trouvé pour ce chemin/ces critères.
    </div>

    <div v-if="loading" style="display:flex;justify-content:center;padding:40px;gap:12px;align-items:center">
      <NSpinner :size="24" />
      <span style="color:var(--text-muted)">Analyse récursive en cours (dossiers exclus : {{ excludedFolders.length }})...</span>
    </div>
  </div>
</template>

<style scoped>
.bigfiles-page { display:flex; flex-direction:column; gap:16px; }
.page-header { display:flex; justify-content:space-between; align-items:flex-start; flex-wrap:wrap; gap:12px; }
.page-header h1 { font-size:24px; font-weight:700; }
.page-subtitle { color:var(--text-muted); font-size:13px; margin-top:2px; }

.controls { display:flex; gap:8px; flex-wrap:wrap; align-items:center; }
.path-input {
  flex:1; min-width:200px; padding:8px 12px; border:1px solid var(--border); border-radius:var(--radius-md);
  background:var(--bg-tertiary); color:var(--text-primary); font-family:monospace; font-size:13px; outline:none;
}
.path-input:focus { border-color:var(--accent-primary); }
.count-select {
  padding:8px 10px; border:1px solid var(--border); border-radius:var(--radius-md);
  background:var(--bg-tertiary); color:var(--text-secondary); font-size:12px; cursor:pointer;
}

/* Exclusions */
.exclusions-section { margin-top:14px; padding-top:14px; border-top:1px solid var(--border); }
.exclusions-header { display:flex; align-items:baseline; gap:8px; margin-bottom:8px; }
.field-label { font-size:11px; color:var(--text-muted); font-weight:600; text-transform:uppercase; letter-spacing:.06em; }
.exclusions-hint { font-size:11px; color:var(--text-muted); font-style:italic; }
.exclusions-list { display:flex; flex-wrap:wrap; gap:6px; margin-bottom:8px; }
.exclusion-tag {
  display:inline-flex; align-items:center; gap:4px; padding:2px 8px 2px 10px;
  background:var(--bg-tertiary); border:1px solid var(--border); border-radius:99px;
  font-size:11px; font-family:monospace; color:var(--text-secondary);
}
.exclusion-remove {
  background:none; border:none; cursor:pointer; color:var(--text-muted); padding:1px;
  display:flex; align-items:center; border-radius:99px;
}
.exclusion-remove:hover { color:var(--danger); background:rgba(239,68,68,.1); }
.exclusion-add-row { display:flex; gap:6px; align-items:center; }
.exclusion-input {
  flex:1; padding:5px 10px; border:1px solid var(--border); border-radius:var(--radius-sm);
  background:var(--bg-tertiary); color:var(--text-primary); font-size:12px; outline:none; font-family:monospace;
}
.exclusion-input:focus { border-color:var(--accent-primary); }

/* Mode corbeille */
.trash-mode-row { margin-top:12px; padding-top:12px; border-top:1px solid var(--border); }
.toggle-label { display:inline-flex; align-items:center; gap:8px; cursor:pointer; font-size:12px; }
.toggle-check { cursor:pointer; accent-color:var(--accent-primary); width:14px; height:14px; }
.toggle-text { color:var(--text-secondary); font-weight:500; }
.toggle-hint { color:var(--text-muted); font-style:italic; }

.ext-filter-wrap { margin-left:auto; }
.ext-filter {
  padding:4px 10px; border:1px solid var(--border); border-radius:var(--radius-sm);
  background:var(--bg-tertiary); color:var(--text-primary); font-size:12px; outline:none; width:200px;
}
.ext-filter:focus { border-color:var(--accent-primary); }

.confirm-banner {
  display:flex; align-items:center; gap:10px; flex-wrap:wrap;
  padding:10px 16px; background:rgba(239,68,68,.08); border:1px solid rgba(239,68,68,.25);
  border-radius:var(--radius-md); font-size:13px; color:var(--text-primary);
}
.confirm-banner span { flex:1; }

.files-list { display:flex; flex-direction:column; gap:2px; }
.file-row {
  display:flex; align-items:center; gap:10px; padding:8px 10px;
  border-radius:var(--radius-sm); transition:background var(--transition-fast);
}
.file-row:hover { background:var(--bg-tertiary); }
.file-row.danger { background:rgba(239,68,68,.06); }
.rank { font-size:11px; color:var(--text-muted); font-family:monospace; width:24px; text-align:right; flex-shrink:0; }
.ext-badge { font-size:10px; padding:2px 6px; border:1px solid; border-radius:4px; font-family:monospace; font-weight:700; flex-shrink:0; min-width:40px; text-align:center; }
.file-main { flex:1; min-width:0; display:flex; flex-direction:column; gap:2px; }
.file-name { font-size:13px; font-weight:500; color:var(--text-primary); overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.bar-wrap { height:3px; background:var(--bg-secondary); border-radius:2px; overflow:hidden; }
.bar-fill { height:100%; border-radius:2px; transition:width var(--transition-fast); }
.file-path { font-size:10px; color:var(--text-muted); font-family:monospace; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.file-meta { display:flex; flex-direction:column; align-items:flex-end; gap:2px; flex-shrink:0; }
.file-size { font-size:13px; font-weight:700; color:var(--text-primary); font-family:monospace; }
.file-date { font-size:10px; color:var(--text-muted); }
.file-actions { display:flex; gap:4px; flex-shrink:0; }
.icon-btn { background:none; border:none; color:var(--text-muted); cursor:pointer; padding:4px; border-radius:4px; display:flex; align-items:center; }
.icon-btn:hover { color:var(--text-primary); background:var(--bg-secondary); }
.icon-btn.danger:hover { color:var(--danger); background:rgba(239,68,68,.1); }
.icon-btn:disabled { opacity:.4; cursor:not-allowed; }
</style>
