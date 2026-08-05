<script setup lang="ts">
defineOptions({ name: "DuplicateFinderPage" });
import { ref, computed } from "vue";
import { csvCell } from "@/composables/useExportData";
import { invoke, invokeRaw } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import { useNotificationStore } from "@/stores/notifications";
import { Copy, Trash2, FolderOpen, Search, Download, RotateCcw } from "lucide-vue-next";

const notify = useNotificationStore();

interface DuplicateGroup { hash: string; size_bytes: number; count: number; files: string[]; wasted_bytes: number; }

const groups      = ref<DuplicateGroup[]>([]);
const loading     = ref(false);
const hasScanned  = ref(false);
const deleting    = ref<Set<string>>(new Set());
const scanPath    = ref("C:\\Users");
const minSizeKb   = ref(100);
const confirmAll  = ref(false);
const scanMode    = ref<"hash" | "name">("hash");
const creatingRP  = ref(false);
const displayCount = ref(20);
const totalWasted = computed(() => groups.value.reduce((s, g) => s + g.wasted_bytes, 0));
const dupeFiles   = computed(() => groups.value.reduce((s, g) => s + g.files.length - 1, 0));
const visibleGroups = computed(() => groups.value.slice(0, displayCount.value));

const scanModes = [
  { value: "hash" as const, label: "Hash", icon: "🔑", desc: "Fichiers identiques bit à bit (MD5)" },
  { value: "name" as const, label: "Nom",  icon: "📝", desc: "Même nom de fichier (tailles différentes possibles)" },
];

function formatSize(bytes: number): string {
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function filename(path: string): string {
  return path.split("\\").pop() || path;
}

async function createRestorePoint() {
  creatingRP.value = true;
  try {
    await invoke("create_restore_point_cmd", { description: "Avant suppression doublons Nitrite" });
    notify.success("Point de restauration créé", "Vous pouvez annuler la suppression depuis la restauration système");
  } catch (e: unknown) {
    notify.error("Erreur", "Impossible de créer le point de restauration : " + String(e));
  } finally {
    creatingRP.value = false;
  }
}

async function scan() {
  loading.value = true; hasScanned.value = true; groups.value = []; confirmAll.value = false; displayCount.value = 20;
  try {
    groups.value = await invokeRaw<DuplicateGroup[]>("find_duplicates", {
      path: scanPath.value,
      minSizeKb: minSizeKb.value,
      mode: scanMode.value,
    });
    if (groups.value.length === 0) notify.info("Aucun doublon", "Aucun fichier dupliqué trouvé");
    else notify.success("Scan terminé", `${groups.value.length} groupe(s) — ${formatSize(totalWasted.value)} récupérable(s)`);
  } catch (e: unknown) {
    notify.error("Erreur scan", String(e));
  } finally {
    loading.value = false;
  }
}

async function deleteFile(path: string, group: DuplicateGroup) {
  deleting.value.add(path);
  const gi = groups.value.indexOf(group);
  if (gi < 0) { deleting.value.delete(path); return; }
  try {
    await invoke("trash_file", { path });
    groups.value[gi].files = groups.value[gi].files.filter(f => f !== path);
    groups.value[gi].wasted_bytes -= groups.value[gi].size_bytes;
    if (groups.value[gi].files.length <= 1) groups.value.splice(gi, 1);
    notify.success("Envoyé à la corbeille", filename(path));
  } catch (_) {
    try {
      await invoke("delete_file", { path });
      groups.value[gi].files = groups.value[gi].files.filter(f => f !== path);
      groups.value[gi].wasted_bytes -= groups.value[gi].size_bytes;
      if (groups.value[gi].files.length <= 1) groups.value.splice(gi, 1);
      notify.success("Supprimé", filename(path));
    } catch (e: unknown) {
      notify.error("Erreur suppression", String(e));
    }
  } finally {
    deleting.value.delete(path);
  }
}

async function deleteAllDupes() {
  let count = 0; let freed = 0;
  for (let gi = groups.value.length - 1; gi >= 0; gi--) {
    const g = groups.value[gi];
    for (let fi = g.files.length - 1; fi >= 1; fi--) {
      try {
        try { await invoke("trash_file", { path: g.files[fi] }); }
        catch { await invoke("delete_file", { path: g.files[fi] }); }
        freed += g.size_bytes; count++;
        g.files.splice(fi, 1);
      } catch (e: unknown) {
        notify.error("Suppression échouée", String(e));
      }
    }
    if (g.files.length <= 1) groups.value.splice(gi, 1);
  }
  confirmAll.value = false;
  notify.success("Nettoyage terminé", `${count} fichier(s) envoyé(s) à la corbeille — ${formatSize(freed)} libéré(s)`);
}

function exportCsv() {
  const rows = ["Groupe,Hash,Taille,Chemin,Type,Mode"];
  groups.value.forEach((g, gi) => g.files.forEach((f, fi) =>
    rows.push([gi + 1, g.hash, g.size_bytes, f, fi === 0 ? "Original" : "Doublon", scanMode.value].map(csvCell).join(","))
  ));
  const blob = new Blob([rows.join("\n")], { type: "text/csv" });
  const a = document.createElement("a"); a.href = URL.createObjectURL(blob);
  a.download = `doublons_${new Date().toISOString().slice(0,10)}.csv`; a.click();
}

function copyPath(path: string) {
  navigator.clipboard.writeText(path).then(() => notify.success("Copié", "Chemin copié"));
}

async function openFile(path: string) {
  try { await invoke("open_path", { path: path.substring(0, path.lastIndexOf("\\")) }); } catch {}
}
</script>

<template>
  <div class="dup-page">
    <div class="page-header">
      <div>
        <h1>Détecteur de Doublons</h1>
        <p class="page-subtitle">Trouvez et supprimez les fichiers dupliqués — par hash (MD5) ou par nom</p>
      </div>
      <div v-if="groups.length" style="display:flex;gap:8px">
        <NButton variant="ghost" size="sm" @click="exportCsv"><Download :size="14" /> CSV</NButton>
        <NButton variant="danger" size="sm" @click="confirmAll = !confirmAll">
          <Trash2 :size="14" /> Supprimer tous les doublons
        </NButton>
      </div>
    </div>

    <!-- Confirmation masse -->
    <div v-if="confirmAll" class="confirm-banner">
      <span>⚠ Envoyer <strong>{{ dupeFiles }} doublons</strong> ({{ formatSize(totalWasted) }}) à la corbeille ? Les originaux seront conservés.</span>
      <div style="display:flex;gap:8px">
        <NButton variant="ghost" size="sm" :loading="creatingRP" @click="createRestorePoint">
          <RotateCcw :size="12" /> Point de restauration
        </NButton>
        <NButton variant="danger" size="sm" @click="deleteAllDupes">Confirmer</NButton>
        <NButton variant="ghost" size="sm" @click="confirmAll = false">Annuler</NButton>
      </div>
    </div>

    <NCard>
      <!-- Mode scan -->
      <div class="mode-row">
        <span style="font-size:12px;color:var(--text-muted);white-space:nowrap">Mode :</span>
        <div class="mode-tabs">
          <button
            v-for="m in scanModes" :key="m.value"
            class="mode-btn" :class="{ active: scanMode === m.value }"
            @click="scanMode = m.value"
            :title="m.desc"
          >
            {{ m.icon }} {{ m.label }}
          </button>
        </div>
        <span class="mode-desc">{{ scanModes.find(m => m.value === scanMode)?.desc }}</span>
      </div>

      <div class="scan-controls">
        <input v-model="scanPath" class="path-input" placeholder="C:\Users" @keyup.enter="scan" />
        <div style="display:flex;align-items:center;gap:8px">
          <span style="font-size:12px;color:var(--text-muted);white-space:nowrap">Taille min :</span>
          <select v-model="minSizeKb" class="size-select">
            <option :value="10">10 KB</option>
            <option :value="100">100 KB</option>
            <option :value="1024">1 MB</option>
            <option :value="10240">10 MB</option>
            <option :value="102400">100 MB</option>
          </select>
        </div>
        <NButton variant="ghost" size="sm" :loading="creatingRP" @click="createRestorePoint" title="Créer un point de restauration Windows avant de scanner">
          <RotateCcw :size="14" />
        </NButton>
        <NButton variant="primary" :loading="loading" @click="scan">
          <Search :size="14" /> Analyser
        </NButton>
      </div>
    </NCard>

    <NCard v-if="loading" style="text-align:center;padding:48px">
      <NSpinner :size="28" />
      <p style="margin-top:12px;color:var(--text-muted)">Recherche des doublons... (peut prendre plusieurs minutes)</p>
    </NCard>

    <template v-if="!loading && groups.length > 0">
      <div class="summary-bar">
        <NBadge variant="warning">{{ groups.length }} groupes</NBadge>
        <NBadge variant="danger">{{ dupeFiles }} fichiers en double</NBadge>
        <NBadge variant="danger">{{ formatSize(totalWasted) }} gaspillé</NBadge>
        <NBadge variant="neutral">Mode : {{ scanMode }}</NBadge>
        <span style="font-size:12px;color:var(--text-muted)">📄 = original  📋 = doublon (corbeille)</span>
      </div>

      <NCard v-for="g in visibleGroups" :key="g.hash">
        <template #header>
          <div style="display:flex;align-items:center;gap:8px;width:100%">
            <Copy :size="14" style="color:var(--warning)" />
            <span>{{ g.count }} fichiers identiques</span>
            <NBadge variant="neutral">{{ formatSize(g.size_bytes) }} chacun</NBadge>
            <NBadge variant="warning">{{ formatSize(g.wasted_bytes) }} gaspillé</NBadge>
            <code style="margin-left:auto;font-size:10px;color:var(--text-muted)">{{ g.hash.slice(0,16) }}...</code>
          </div>
        </template>
        <div class="files-list">
          <div v-for="(f, fi) in g.files" :key="f" class="file-row" :class="{ original: fi === 0 }">
            <span class="file-icon">{{ fi === 0 ? '📄' : '📋' }}</span>
            <div class="file-info">
              <span class="file-name">{{ filename(f) }}</span>
              <span class="file-path">{{ f }}</span>
            </div>
            <div class="file-actions">
              <button class="icon-btn" @click="copyPath(f)" title="Copier chemin"><Copy :size="12" /></button>
              <button class="icon-btn" @click="openFile(f)" title="Ouvrir dossier"><FolderOpen :size="12" /></button>
              <button
                v-if="fi > 0"
                class="icon-btn danger"
                :class="{ spinning: deleting.has(f) }"
                :disabled="deleting.has(f)"
                @click="deleteFile(f, g)"
                title="Envoyer à la corbeille"
              >
                <NSpinner v-if="deleting.has(f)" :size="10" />
                <Trash2 v-else :size="12" />
              </button>
            </div>
          </div>
        </div>
      </NCard>
      <div v-if="displayCount < groups.length" style="text-align:center;padding:12px">
        <NButton variant="ghost" size="sm" @click="displayCount += 20">
          Charger plus ({{ groups.length - displayCount }} restants)
        </NButton>
      </div>
    </template>

    <div v-if="!loading && hasScanned && groups.length === 0" style="text-align:center;padding:40px;color:var(--text-muted)">
      Aucun doublon trouvé pour ce chemin/ces critères.
    </div>
  </div>
</template>

<style scoped>
.dup-page { display:flex; flex-direction:column; gap:16px; }
.page-header { display:flex; justify-content:space-between; align-items:flex-start; flex-wrap:wrap; gap:12px; }
.page-header h1 { font-size:24px; font-weight:700; }
.page-subtitle { color:var(--text-muted); font-size:13px; margin-top:2px; }

.confirm-banner {
  display:flex; align-items:center; justify-content:space-between; gap:12px; flex-wrap:wrap;
  padding:12px 16px; background:rgba(239,68,68,.1); border:1px solid rgba(239,68,68,.3);
  border-radius:var(--radius-md); font-size:13px; color:var(--text-primary);
}

.mode-row { display:flex; align-items:center; gap:10px; margin-bottom:12px; flex-wrap:wrap; }
.mode-tabs { display:flex; gap:4px; }
.mode-btn {
  padding:4px 12px; border:1px solid var(--border); border-radius:var(--radius-sm);
  background:var(--bg-tertiary); color:var(--text-secondary); cursor:pointer; font-size:12px;
  transition:all .15s;
}
.mode-btn.active { border-color:var(--accent-primary); color:var(--accent-primary); background:var(--accent-muted); }
.mode-btn:hover:not(.active) { border-color:var(--border-hover); color:var(--text-primary); }
.mode-desc { font-size:11px; color:var(--text-muted); font-style:italic; }

.scan-controls { display:flex; gap:8px; flex-wrap:wrap; align-items:center; }
.path-input {
  flex:1; min-width:200px; padding:8px 12px; border:1px solid var(--border); border-radius:var(--radius-md);
  background:var(--bg-tertiary); color:var(--text-primary); font-family:monospace; font-size:13px; outline:none;
}
.path-input:focus { border-color:var(--accent-primary); }
.size-select { padding:6px 8px; border:1px solid var(--border); border-radius:var(--radius-sm); background:var(--bg-tertiary); color:var(--text-secondary); font-size:12px; cursor:pointer; }

.summary-bar { display:flex; align-items:center; gap:8px; flex-wrap:wrap; }

.files-list { display:flex; flex-direction:column; gap:4px; }
.file-row { display:flex; align-items:center; gap:10px; padding:8px 10px; border-radius:var(--radius-sm); background:var(--bg-secondary); }
.file-row.original { background:var(--success-muted, rgba(34,197,94,0.08)); }
.file-icon { font-size:14px; flex-shrink:0; }
.file-info { flex:1; min-width:0; display:flex; flex-direction:column; gap:2px; }
.file-name { font-size:13px; font-weight:500; color:var(--text-primary); }
.file-path { font-size:10px; color:var(--text-muted); font-family:monospace; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.file-actions { display:flex; gap:4px; flex-shrink:0; }
.icon-btn { background:none; border:none; color:var(--text-muted); cursor:pointer; padding:4px; border-radius:4px; transition:all .15s; display:flex; align-items:center; }
.icon-btn:hover { color:var(--text-primary); background:var(--bg-tertiary); }
.icon-btn.danger:hover { color:var(--danger); }
.icon-btn:disabled { opacity:.4; cursor:default; }
</style>
