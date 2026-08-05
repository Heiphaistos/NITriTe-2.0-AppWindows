<script setup lang="ts">
import { ref } from "vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import { invokeRaw as invoke } from "@/utils/invoke";
import NButton from "@/components/ui/NButton.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import { useNotificationStore } from "@/stores/notifications";
import { GitCompare, FolderOpen, RotateCcw, Trash2, PlusCircle } from "lucide-vue-next";

const props = defineProps<{
  devicePath: string;
  subPath: string;
}>();

const notify = useNotificationStore();

interface ComparedFile {
  name: string; shadow_path: string; live_path: string; status: string;
  shadow_size: number; live_size: number; shadow_modified: string; live_modified: string;
}

const livePath = ref("");
const files = ref<ComparedFile[]>([]);
const loading = ref(false);
const done = ref(false);

async function pickLivePath() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const dir = await open({ directory: true, multiple: false, title: "Dossier actuel à comparer" });
  if (dir) livePath.value = dir as string;
}

async function compare() {
  if (!livePath.value.trim()) { notify.error("Chemin manquant", "Saisissez le dossier actuel à comparer."); return; }
  loading.value = true; files.value = []; done.value = false;
  try {
    files.value = await invoke<ComparedFile[]>("compare_shadow_with_current", {
      devicePath: props.devicePath,
      subPath: props.subPath,
      livePath: livePath.value,
    });
    done.value = true;
  } catch (e: unknown) {
    notify.error("Erreur comparaison", String(e));
  }
  loading.value = false;
}

async function restoreFile(f: ComparedFile) {
  if (!f.shadow_path) return;
  const targetDir = livePath.value || "C:\\NiTriTe\\Restaurés";
  // La restauration écrit avec -Force côté backend : pour un fichier "modifié",
  // la cible par défaut (targetDir = livePath, le dossier live comparé) écrase
  // silencieusement la version ACTUELLE par l'ancienne version du cliché, sans
  // aucune sauvegarde préalable — contrairement à "supprimé" (recrée un fichier
  // absent, rien à écraser) ce cas détruit réellement des données existantes.
  if (f.status === "modified") {
    const overwritesLive = targetDir === livePath.value;
    const msg = overwritesLive
      ? `Écraser la version actuelle de "${f.name}" par l'ancienne version du cliché ? La version actuelle sera perdue.`
      : `Restaurer l'ancienne version de "${f.name}" dans "${targetDir}" ?`;
    if (!(await confirm(msg, { title: "Nitrite", kind: "warning" }))) return;
  }
  try {
    const r = await invoke<{ success: boolean; message: string; restored_path: string }>("restore_from_shadow", {
      sourcePath: f.shadow_path,
      targetFolder: targetDir,
    });
    if (r.success) notify.success("Restauré", f.name);
    else notify.error("Échec", r.message);
  } catch (e: unknown) { notify.error("Erreur", String(e)); }
}

function formatSize(b: number) {
  if (!b) return "—";
  if (b < 1024) return `${b} o`;
  if (b < 1024 ** 2) return `${(b / 1024).toFixed(1)} Ko`;
  return `${(b / 1024 ** 2).toFixed(1)} Mo`;
}
function formatDate(s: string) { try { return new Date(s).toLocaleString("fr-FR"); } catch { return s || "—"; } }

const deleted  = () => files.value.filter(f => f.status === "deleted");
const modified = () => files.value.filter(f => f.status === "modified");
const added    = () => files.value.filter(f => f.status === "added");
</script>

<template>
  <div class="compare-panel">
    <div class="compare-header">
      <GitCompare :size="15" style="color:var(--accent-primary)" />
      <span class="compare-title">Comparaison avec le système actuel</span>
    </div>

    <div class="compare-form">
      <label class="cmp-label">Dossier actuel :</label>
      <input v-model="livePath" class="cmp-input" placeholder="Ex: C:\Users\John\Documents" />
      <NButton variant="ghost" size="sm" @click="pickLivePath"><FolderOpen :size="13" /></NButton>
      <NButton variant="primary" size="sm" :loading="loading" @click="compare">
        <GitCompare :size="13" /> Comparer
      </NButton>
    </div>

    <div v-if="loading" class="cmp-loading"><NSpinner :size="16" /><span>Analyse en cours...</span></div>

    <div v-else-if="done">
      <div v-if="files.length === 0" class="cmp-empty">Aucune différence trouvée — dossiers identiques.</div>
      <div v-else class="cmp-summary">
        <span class="badge deleted">🗑 {{ deleted().length }} supprimé(s)</span>
        <span class="badge modified">✏ {{ modified().length }} modifié(s)</span>
        <span class="badge added">+ {{ added().length }} ajouté(s)</span>
      </div>

      <div class="cmp-table" v-if="files.length > 0">
        <div class="cmp-row header-row">
          <span>Statut</span><span>Nom</span><span>Taille ancienne → actuelle</span><span>Modifié</span><span></span>
        </div>
        <div v-for="f in files" :key="f.name + f.status" class="cmp-row" :class="f.status">
          <span class="status-badge">
            <Trash2   v-if="f.status === 'deleted'"  :size="12" class="ic-del" />
            <span     v-else-if="f.status === 'modified'" class="ic-mod">✏</span>
            <PlusCircle v-else                         :size="12" class="ic-add" />
            {{ f.status === 'deleted' ? 'Supprimé' : f.status === 'modified' ? 'Modifié' : 'Ajouté' }}
          </span>
          <span class="cmp-name">{{ f.name }}</span>
          <span class="cmp-size">
            {{ f.status === 'added' ? '—' : formatSize(f.shadow_size) }}
            <template v-if="f.status === 'modified'"> → {{ formatSize(f.live_size) }}</template>
          </span>
          <span class="cmp-date">{{ formatDate(f.shadow_modified || f.live_modified) }}</span>
          <NButton
            v-if="f.status !== 'added' && f.shadow_path"
            variant="ghost" size="sm"
            @click="restoreFile(f)"
          >
            <RotateCcw :size="11" /> Restaurer
          </NButton>
          <span v-else></span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.compare-panel {
  display: flex; flex-direction: column; gap: 10px;
  border: 1px solid var(--accent-primary); border-radius: var(--radius-lg);
  padding: 12px 14px; background: var(--bg-secondary); margin-top: 6px;
}
.compare-header { display: flex; align-items: center; gap: 8px; }
.compare-title { font-size: 13px; font-weight: 700; color: var(--accent-primary); }

.compare-form { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.cmp-label { font-size: 11px; color: var(--text-muted); flex-shrink: 0; }
.cmp-input {
  flex: 1; min-width: 200px; padding: 5px 9px; font-size: 12px;
  background: var(--bg-tertiary); border: 1px solid var(--border);
  border-radius: var(--radius-sm); color: var(--text-primary); font-family: monospace; outline: none;
  transition: border-color 0.15s;
}
.cmp-input:focus { border-color: var(--accent-primary); }

.cmp-loading { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--text-muted); padding: 8px 0; }
.cmp-empty   { font-size: 12px; color: var(--text-muted); padding: 8px 0; }

.cmp-summary { display: flex; gap: 10px; flex-wrap: wrap; }
.badge { font-size: 11px; padding: 3px 9px; border-radius: 10px; font-weight: 600; }
.badge.deleted  { background: color-mix(in srgb, var(--danger) 12%, transparent); color: var(--danger); }
.badge.modified { background: color-mix(in srgb, #f59e0b 12%, transparent); color: #d97706; }
.badge.added    { background: color-mix(in srgb, var(--success) 12%, transparent); color: var(--success); }

.cmp-table { display: flex; flex-direction: column; border: 1px solid var(--border); border-radius: var(--radius-md); overflow: hidden; max-height: 300px; overflow-y: auto; }
.cmp-row { display: grid; grid-template-columns: 110px 1fr 160px 140px 90px; align-items: center; gap: 8px; padding: 5px 10px; border-bottom: 1px solid var(--border); font-size: 12px; }
.cmp-row:last-child { border-bottom: none; }
.header-row { background: var(--bg-tertiary); font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: .05em; position: sticky; top: 0; }
.cmp-row.deleted  { background: color-mix(in srgb, var(--danger) 5%, transparent); }
.cmp-row.modified { background: color-mix(in srgb, #f59e0b 5%, transparent); }
.cmp-row.added    { background: color-mix(in srgb, var(--success) 5%, transparent); }

.status-badge { display: flex; align-items: center; gap: 4px; font-size: 11px; font-weight: 600; }
.ic-del { color: var(--danger); }
.ic-mod { color: #d97706; font-size: 11px; }
.ic-add { color: var(--success); }
.cmp-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-primary); }
.cmp-size, .cmp-date { color: var(--text-muted); font-family: monospace; font-size: 11px; }
</style>
