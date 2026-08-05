<script setup lang="ts">
import { ref, computed } from "vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import { invoke, invokeRaw } from "@/utils/invoke";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import { useNotificationStore } from "@/stores/notifications";
import { Search, Trash2, AlertTriangle } from "lucide-vue-next";

interface DllEntry {
  name: string;
  path: string;
  size: number;
  company: string;
  description: string;
  version: string;
  location: string;
  category: string;
}

const notify = useNotificationStore();
const dlls = ref<DllEntry[]>([]);
const loading = ref(false);
const search = ref("");
const filterCat = ref("all");
const deleting = ref<Set<string>>(new Set());
const selected = ref<Set<string>>(new Set());

const CATS = [
  { id: "all",            label: "Toutes",              color: "var(--text-muted)" },
  { id: "Système",        label: "Système (Microsoft)",  color: "var(--success)" },
  { id: "Tiers (System32)", label: "Tiers (System32)",  color: "var(--warning)" },
  { id: "Application",   label: "Applications",          color: "var(--accent-primary)" },
];

function catVariant(cat: string): "success" | "warning" | "neutral" | "danger" {
  if (cat === "Système")          return "success";
  if (cat === "Tiers (System32)") return "warning";
  return "neutral";
}

function formatSize(bytes: number): string {
  if (bytes > 1048576) return `${(bytes / 1048576).toFixed(1)} MB`;
  if (bytes > 1024)    return `${(bytes / 1024).toFixed(0)} KB`;
  return `${bytes} o`;
}

const filtered = computed(() => {
  let list = dlls.value;
  if (filterCat.value !== "all") list = list.filter(d => d.category === filterCat.value);
  if (search.value.trim()) {
    const q = search.value.toLowerCase();
    list = list.filter(d =>
      d.name.toLowerCase().includes(q) ||
      d.company.toLowerCase().includes(q) ||
      d.description.toLowerCase().includes(q)
    );
  }
  return list;
});

const catCounts = computed(() => {
  const counts: Record<string, number> = { all: dlls.value.length };
  for (const d of dlls.value) {
    counts[d.category] = (counts[d.category] ?? 0) + 1;
  }
  return counts;
});

async function scan() {
  loading.value = true;
  dlls.value = [];
  selected.value.clear();
  notify.info("Scanner DLL", "Analyse en cours (peut prendre 30-60s)…");
  try {
    dlls.value = await invokeRaw<DllEntry[]>("scan_dlls");
    notify.success("DLL scannées", `${dlls.value.length} DLL trouvées`);
  } catch (e: unknown) {
    notify.error("Erreur scan DLL", String(e));
  }
  loading.value = false;
}

async function deleteDllInner(path: string) {
  deleting.value = new Set(deleting.value).add(path);
  try {
    await invoke("delete_dll", { path });
    dlls.value = dlls.value.filter(d => d.path !== path);
    selected.value.delete(path);
    notify.success("DLL supprimée", path);
  } catch (e: unknown) {
    notify.error("Erreur suppression", String(e));
  }
  const next = new Set(deleting.value);
  next.delete(path);
  deleting.value = next;
}

async function deleteDll(path: string) {
  if (!(await confirm(`Supprimer ${path} ?\n\nAttention : la suppression d'une DLL système ou tierce peut déstabiliser le système.`, { title: "Nitrite", kind: "warning" }))) return;
  await deleteDllInner(path);
}

function toggleSelect(path: string) {
  const next = new Set(selected.value);
  if (next.has(path)) next.delete(path); else next.add(path);
  selected.value = next;
}

async function deleteSelected() {
  if (!selected.value.size) return;
  if (!(await confirm(`Supprimer ${selected.value.size} DLL(s) ?\n\nCette action est irréversible.`, { title: "Nitrite", kind: "warning" }))) return;
  // deleteDllInner (pas deleteDll) : la confirmation ci-dessus couvre déjà tout
  // le lot — reprompter par fichier forcerait un clic supplémentaire par DLL,
  // rendant la suppression groupée pire qu'une suppression une par une.
  for (const path of selected.value) {
    await deleteDllInner(path);
  }
}
</script>

<template>
  <div class="dll-scanner">
    <div class="dll-toolbar">
      <NButton variant="primary" size="sm" :loading="loading" @click="scan">
        <Search :size="13" /> {{ loading ? 'Analyse…' : 'Scanner les DLL' }}
      </NButton>
      <div v-if="selected.size > 0" style="display:flex;align-items:center;gap:8px">
        <span class="muted" style="font-size:12px">{{ selected.size }} sélectionnée(s)</span>
        <NButton variant="danger" size="sm" @click="deleteSelected">
          <Trash2 :size="13" /> Supprimer la sélection
        </NButton>
      </div>
      <div class="dll-search-wrap">
        <Search :size="13" class="search-icon" />
        <input class="dll-search" v-model="search" placeholder="Rechercher…" />
      </div>
    </div>

    <!-- Avertissement -->
    <div class="dll-warning">
      <AlertTriangle :size="14" style="color:var(--warning);flex-shrink:0" />
      <span>Les DLL <strong>Tiers (System32)</strong> ont été ajoutées par des pilotes ou applications tierces. Ne supprimez que ce dont vous êtes certain·e — une DLL manquante peut planter des logiciels ou Windows.</span>
    </div>

    <!-- Filtres -->
    <div class="dll-filters">
      <button v-for="c in CATS" :key="c.id"
        class="dll-cat-pill" :class="{ active: filterCat === c.id }"
        @click="filterCat = c.id">
        <span class="cat-dot" :style="{ background: c.color }"></span>
        {{ c.label }}
        <span class="cat-count">{{ catCounts[c.id] ?? 0 }}</span>
      </button>
    </div>

    <!-- Aucune donnée -->
    <div v-if="!loading && dlls.length === 0" class="dll-empty">
      Cliquez sur "Scanner les DLL" pour analyser les bibliothèques installées.
    </div>

    <!-- Tableau -->
    <div v-if="filtered.length > 0" class="dll-table-wrap">
      <table class="dll-table">
        <thead>
          <tr>
            <th style="width:30px"></th>
            <th>Nom</th>
            <th>Description</th>
            <th>Éditeur</th>
            <th>Emplacement</th>
            <th>Version</th>
            <th>Taille</th>
            <th>Catégorie</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="d in filtered" :key="d.path"
              :class="{ 'row-selected': selected.has(d.path) }">
            <td><input type="checkbox" :checked="selected.has(d.path)" @change="toggleSelect(d.path)" /></td>
            <td class="dll-name" :title="d.path">{{ d.name }}</td>
            <td class="dll-desc muted">{{ d.description || '—' }}</td>
            <td class="muted">{{ d.company || '—' }}</td>
            <td><span class="location-badge">{{ d.location }}</span></td>
            <td class="mono muted small">{{ d.version || '—' }}</td>
            <td class="mono muted small">{{ formatSize(d.size) }}</td>
            <td><NBadge :variant="catVariant(d.category)" size="sm">{{ d.category }}</NBadge></td>
            <td>
              <NButton variant="ghost" size="sm" :loading="deleting.has(d.path)"
                @click="deleteDll(d.path)"
                :title="`Supprimer ${d.name}`">
                <Trash2 :size="12" />
              </NButton>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <div v-else-if="dlls.length > 0 && filtered.length === 0" class="dll-empty">Aucun résultat pour ce filtre.</div>
  </div>
</template>

<style scoped>
.dll-scanner { display: flex; flex-direction: column; gap: 12px; }
.dll-toolbar { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.dll-search-wrap { position: relative; margin-left: auto; }
.search-icon { position: absolute; left: 9px; top: 50%; transform: translateY(-50%); color: var(--text-muted); pointer-events: none; }
.dll-search { padding: 6px 10px 6px 30px; border: 1px solid var(--border); border-radius: var(--radius-lg); background: var(--bg-secondary); color: var(--text-primary); font-size: 12px; width: 200px; }
.dll-warning { display: flex; gap: 8px; align-items: flex-start; padding: 10px 14px; background: rgba(255,193,7,.08); border: 1px solid rgba(255,193,7,.25); border-radius: var(--radius-lg); font-size: 12px; line-height: 1.6; }
.dll-filters { display: flex; gap: 6px; flex-wrap: wrap; }
.dll-cat-pill { display: flex; align-items: center; gap: 6px; padding: 4px 12px; border-radius: 999px; border: 1px solid var(--border); background: transparent; cursor: pointer; font-size: 12px; color: var(--text-secondary); transition: all .15s; }
.dll-cat-pill:hover { border-color: var(--border-hover); }
.dll-cat-pill.active { background: var(--accent-primary); border-color: var(--accent-primary); color: #fff; }
.cat-dot { width: 7px; height: 7px; border-radius: 50%; }
.cat-count { font-size: 10px; background: rgba(255,255,255,.15); padding: 1px 5px; border-radius: 999px; }
.dll-empty { text-align: center; padding: 32px; color: var(--text-muted); font-size: 13px; }
.dll-table-wrap { overflow-x: auto; max-height: 520px; overflow-y: auto; }
.dll-table { width: 100%; border-collapse: collapse; font-size: 12px; }
.dll-table th { position: sticky; top: 0; z-index: 1; padding: 7px 10px; text-align: left; font-size: 10px; text-transform: uppercase; letter-spacing: .05em; color: var(--text-muted); background: var(--bg-primary); border-bottom: 1px solid var(--border); font-weight: 700; }
.dll-table td { padding: 6px 10px; border-bottom: 1px solid var(--border); vertical-align: middle; }
.dll-table tr:hover td { background: var(--bg-tertiary); }
.row-selected td { background: rgba(var(--accent-rgb,255,152,0),.06) !important; }
.dll-name { font-weight: 600; max-width: 180px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-family: "JetBrains Mono", monospace; font-size: 11px; }
.dll-desc { max-width: 200px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.location-badge { font-size: 10px; padding: 2px 7px; border-radius: 4px; background: var(--bg-tertiary); color: var(--text-secondary); font-family: "JetBrains Mono", monospace; }
.mono { font-family: "JetBrains Mono", monospace; }
.muted { color: var(--text-muted); }
.small { font-size: 11px; }
</style>
