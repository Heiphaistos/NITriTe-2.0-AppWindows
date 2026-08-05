<script setup lang="ts">
import { ref, computed } from "vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import { csvCell } from "@/composables/useExportData";
import { invoke } from "@/utils/invoke";
import { Search, Package, Terminal, Plus, Trash2, Download, Loader2 } from "lucide-vue-next";
import NBadge from "@/components/ui/NBadge.vue";
import NButton from "@/components/ui/NButton.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import { useNotificationStore } from "@/stores/notifications";
import type { InstalledSoftware, EnvVar } from "@/types/diagnostic";

const notify = useNotificationStore();

const props = defineProps<{
  tab: string;
  softwareList: InstalledSoftware[];
  envVars: EnvVar[];
}>();

const emit = defineEmits<{ refresh: [] }>();

const softwareSearch = ref("");
const envSearch = ref("");
const SW_SORT_OPTS = [['name','Nom'],['date','Date'],['size','Taille'],['publisher','Éditeur']] as const;
const VAR_SCOPE_OPTS = ['Utilisateur','Système'] as const;
const sortBy = ref<"name"|"date"|"size"|"publisher">("name");

// Ajout de variable
const showAddForm = ref(false);
const newVarName = ref("");
const newVarValue = ref("");
const newVarScope = ref<"Système"|"Utilisateur">("Utilisateur");
const envMsg = ref("");
const envErr = ref(false);

function showEnvMsg(msg: string, err = false) {
  envMsg.value = msg; envErr.value = err;
  setTimeout(() => { envMsg.value = ""; }, 4000);
}

async function addEnvVar() {
  if (!newVarName.value.trim()) { showEnvMsg("Le nom est requis.", true); return; }
  try {
    const r = await invoke<string>("set_environment_variable", {
      name: newVarName.value.trim(),
      value: newVarValue.value,
      scope: newVarScope.value,
    });
    showEnvMsg(r);
    newVarName.value = ""; newVarValue.value = ""; showAddForm.value = false;
    emit("refresh");
  } catch (e: unknown) { showEnvMsg(String(e), true); }
}

async function deleteEnvVar(e: EnvVar) {
  if (!(await confirm(`Supprimer la variable "${e.name}" (${e.var_type}) ?`, { title: "Nitrite", kind: "warning" }))) return;
  try {
    const r = await invoke<string>("delete_environment_variable", { name: e.name, scope: e.var_type });
    showEnvMsg(r);
    emit("refresh");
  } catch (err: unknown) { showEnvMsg(String(err), true); }
}

const filteredSoftware = computed(() => {
  let list = [...props.softwareList];
  const q = softwareSearch.value.toLowerCase();
  if (q) list = list.filter(s => s.name.toLowerCase().includes(q) || (s.publisher || "").toLowerCase().includes(q));
  if (sortBy.value === "date") list.sort((a, b) => (b.install_date || "").localeCompare(a.install_date || ""));
  else if (sortBy.value === "size") list.sort((a, b) => (b.estimated_size_mb || 0) - (a.estimated_size_mb || 0));
  else if (sortBy.value === "publisher") list.sort((a, b) => (a.publisher || "").localeCompare(b.publisher || ""));
  else list.sort((a, b) => a.name.localeCompare(b.name));
  return list;
});

const filteredEnv = computed(() => {
  const q = envSearch.value.toLowerCase();
  if (!q) return props.envVars;
  return props.envVars.filter(e => e.name.toLowerCase().includes(q) || e.value.toLowerCase().includes(q));
});

function mbStr(v: number): string {
  return v >= 1024 ? `${(v / 1024).toFixed(1)} GB` : `${v.toFixed(0)} MB`;
}

const uninstallingName = ref<string | null>(null);
const confirmUninstall = ref<string | null>(null);

async function uninstallSoftware(name: string) {
  if (confirmUninstall.value !== name) {
    confirmUninstall.value = name;
    return;
  }
  confirmUninstall.value = null;
  uninstallingName.value = name;
  try {
    const r = await invoke<string>("quick_uninstall_software", { name });
    notify.success("Désinstallation lancée", r);
    emit("refresh");
  } catch (e: unknown) {
    notify.error("Erreur désinstallation", String(e));
  } finally {
    uninstallingName.value = null;
  }
}

function exportSoftwareCsv() {
  const rows = [
    "#,Nom,Version,Éditeur,Date installation,Taille (MB)",
    ...filteredSoftware.value.map((s, i) =>
      [i+1, s.name, s.version||'', s.publisher||'', s.install_date||'', s.estimated_size_mb||0].map(csvCell).join(",")
    )
  ].join("\n");
  const blob = new Blob([rows], { type: "text/csv" });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = `logiciels_${new Date().toISOString().slice(0,10)}.csv`;
  a.click();
  notify.success("Export CSV", `${filteredSoftware.value.length} logiciels exportés`);
}

const topBySize = computed(() =>
  [...props.softwareList].filter(s => s.estimated_size_mb > 0)
    .sort((a, b) => b.estimated_size_mb - a.estimated_size_mb).slice(0, 10)
);

const byPublisher = computed(() => {
  const map = new Map<string, number>();
  for (const s of props.softwareList) {
    const p = s.publisher || "Inconnu";
    map.set(p, (map.get(p) || 0) + 1);
  }
  return [...map.entries()].sort((a, b) => b[1] - a[1]).slice(0, 10);
});
</script>

<template>
  <!-- Logiciels installés -->
  <template v-if="tab === 'software'">
    <div class="diag-tab-content">
      <DiagBanner :icon="Package" title="Logiciels Installés" desc="Applications, versions, éditeurs et taille" color="blue" />
      <div style="display:flex;gap:12px;align-items:center;flex-wrap:wrap;margin-bottom:12px">
        <div class="diag-search" style="flex:1;margin-bottom:0">
          <Search :size="14" />
          <input v-model="softwareSearch" placeholder="Rechercher un logiciel ou éditeur..." class="diag-search-input" />
          <span class="muted">{{ filteredSoftware.length }}/{{ softwareList.length }}</span>
        </div>
        <div style="display:flex;gap:4px">
          <NBadge v-for="s in SW_SORT_OPTS" :key="s[0]"
            :variant="sortBy === s[0] ? 'info' : 'default'"
            style="cursor:pointer" @click="sortBy = s[0]">{{ s[1] }}</NBadge>
        </div>
      </div>

      <div style="display:flex;gap:12px;flex-wrap:wrap;margin-bottom:16px">
        <div class="card-block" style="flex:1;min-width:180px;margin-bottom:0">
          <p class="diag-section-label" style="margin:0 0 4px 0">Total installés</p>
          <span style="font-size:28px;font-weight:700;color:var(--accent)">{{ softwareList.length }}</span>
        </div>
        <div class="card-block" style="flex:1;min-width:180px;margin-bottom:0">
          <p class="diag-section-label" style="margin:0 0 4px 0">Taille totale estimée</p>
          <span style="font-size:20px;font-weight:700">{{ mbStr(softwareList.reduce((a, s) => a + (s.estimated_size_mb || 0), 0)) }}</span>
        </div>
      </div>

      <p class="diag-section-label">Top 10 plus volumineux</p>
      <div v-for="(s, i) in topBySize" :key="`top-${s.name}-${s.publisher}`" class="list-row">
        <span class="muted" style="min-width:20px;font-size:11px">{{ i + 1 }}.</span>
        <span class="list-name">{{ s.name }}</span>
        <span class="muted" style="font-size:11px">{{ s.publisher || "—" }}</span>
        <NBadge variant="warning">{{ mbStr(s.estimated_size_mb) }}</NBadge>
      </div>

      <p class="diag-section-label" style="margin-top:16px">Top éditeurs ({{ byPublisher.length }})</p>
      <div style="display:flex;gap:6px;flex-wrap:wrap;margin-bottom:16px">
        <NBadge v-for="([pub, count]) in byPublisher" :key="pub" variant="default">
          {{ pub }} <span style="font-size:10px;margin-left:4px;opacity:0.7">×{{ count }}</span>
        </NBadge>
      </div>

      <!-- Confirmation désinstall banner -->
      <div v-if="confirmUninstall" class="uninstall-confirm-banner">
        <Trash2 :size="14" style="color:var(--danger);flex-shrink:0" />
        <span>Désinstaller <strong>{{ confirmUninstall }}</strong> ? Cette action est irréversible.</span>
        <button class="conf-btn conf-btn-danger" @click="uninstallSoftware(confirmUninstall)">Confirmer</button>
        <button class="conf-btn" @click="confirmUninstall = null">Annuler</button>
      </div>

      <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:6px">
        <p class="diag-section-label" style="margin:0">Liste complète</p>
        <button class="export-btn" @click="exportSoftwareCsv" title="Exporter CSV">
          <Download :size="13" /> CSV
        </button>
      </div>
      <div class="table-wrap">
        <table class="data-table">
          <thead><tr><th>#</th><th>Nom</th><th>Version</th><th>Éditeur</th><th>Date install.</th><th>Taille</th><th>Action</th></tr></thead>
          <tbody>
            <tr v-if="filteredSoftware.length === 0">
              <td colspan="7" class="muted" style="text-align:center;padding:16px 8px">
                {{ softwareList.length === 0 ? 'Aucun logiciel détecté.' : 'Aucun logiciel ne correspond à la recherche.' }}
              </td>
            </tr>
            <tr v-for="(s, i) in filteredSoftware.slice(0, 300)" :key="`sw-${s.name}-${s.publisher}`" :class="{ 'uninstall-row': confirmUninstall === s.name }">
              <td class="muted">{{ i + 1 }}</td>
              <td style="font-weight:500;max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ s.name }}</td>
              <td><code>{{ s.version || "—" }}</code></td>
              <td class="muted" style="max-width:140px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ s.publisher || "—" }}</td>
              <td class="muted">{{ s.install_date || "—" }}</td>
              <td>{{ s.estimated_size_mb > 0 ? mbStr(s.estimated_size_mb) : "—" }}</td>
              <td>
                <button
                  class="uninstall-btn"
                  :disabled="uninstallingName === s.name"
                  :title="confirmUninstall === s.name ? 'Cliquer pour confirmer' : 'Désinstaller'"
                  @click="uninstallSoftware(s.name)"
                >
                  <Loader2 v-if="uninstallingName === s.name" :size="11" class="spin" />
                  <Trash2 v-else :size="11" />
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </template>

  <!-- Variables d'environnement -->
  <template v-else-if="tab === 'env'">
    <div class="diag-tab-content">
      <DiagBanner :icon="Terminal" title="Variables d'Environnement" desc="Variables système et utilisateur" color="teal" />

      <!-- Toolbar -->
      <div style="display:flex;gap:8px;align-items:center;margin-bottom:10px;flex-wrap:wrap">
        <div class="diag-search" style="flex:1;margin:0">
          <Search :size="14" />
          <input v-model="envSearch" placeholder="Filtrer variable ou valeur..." class="diag-search-input" />
          <span class="muted">{{ filteredEnv.length }}/{{ envVars.length }}</span>
        </div>
        <NButton variant="primary" size="sm" @click="showAddForm = !showAddForm">
          <Plus :size="13" /> Ajouter
        </NButton>
      </div>

      <!-- Message feedback -->
      <div v-if="envMsg" :style="{ color: envErr ? 'var(--error)' : 'var(--success)', fontSize: '12px', marginBottom: '8px' }">
        {{ envMsg }}
      </div>

      <!-- Formulaire d'ajout -->
      <div v-if="showAddForm" class="card-block" style="margin-bottom:12px;display:flex;gap:8px;flex-wrap:wrap;align-items:flex-end">
        <div style="flex:1;min-width:140px">
          <p class="diag-section-label" style="margin:0 0 4px 0;font-size:11px">Nom</p>
          <input v-model="newVarName" placeholder="MY_VARIABLE" class="diag-search-input" style="width:100%;padding:5px 8px;border-radius:6px;border:1px solid var(--border);background:var(--bg-tertiary);color:var(--text);font-size:12px" />
        </div>
        <div style="flex:2;min-width:200px">
          <p class="diag-section-label" style="margin:0 0 4px 0;font-size:11px">Valeur</p>
          <input v-model="newVarValue" placeholder="valeur..." class="diag-search-input" style="width:100%;padding:5px 8px;border-radius:6px;border:1px solid var(--border);background:var(--bg-tertiary);color:var(--text);font-size:12px" />
        </div>
        <div>
          <p class="diag-section-label" style="margin:0 0 4px 0;font-size:11px">Portée</p>
          <div style="display:flex;gap:4px">
            <NBadge v-for="s in VAR_SCOPE_OPTS" :key="s"
              :variant="newVarScope === s ? 'info' : 'default'"
              style="cursor:pointer" @click="newVarScope = s">{{ s }}</NBadge>
          </div>
        </div>
        <div style="display:flex;gap:6px">
          <NButton variant="success" size="sm" @click="addEnvVar">Créer</NButton>
          <NButton variant="ghost" size="sm" @click="showAddForm = false">Annuler</NButton>
        </div>
      </div>

      <div class="table-wrap">
        <table class="data-table">
          <thead><tr><th>Variable</th><th>Valeur</th><th>Portée</th><th>Actions</th></tr></thead>
          <tbody>
            <tr v-if="filteredEnv.length === 0">
              <td colspan="4" class="muted" style="text-align:center;padding:16px 8px">
                {{ envVars.length === 0 ? 'Aucune variable d\'environnement détectée.' : 'Aucune variable ne correspond au filtre.' }}
              </td>
            </tr>
            <tr v-for="e in filteredEnv" :key="`env-${e.name}-${e.var_type}`">
              <td><code style="font-size:11px">{{ e.name }}</code></td>
              <td style="max-width:320px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-family:monospace;font-size:12px;color:var(--text-secondary)">
                {{ e.value }}
              </td>
              <td><NBadge :variant="e.var_type === 'Système' ? 'info' : 'default'" style="font-size:10px">{{ e.var_type }}</NBadge></td>
              <td>
                <button class="env-del-btn" title="Supprimer la variable" @click="deleteEnvVar(e)">
                  <Trash2 :size="11" />
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </template>
</template>

<style scoped>
.uninstall-confirm-banner {
  display:flex; align-items:center; gap:10px; flex-wrap:wrap;
  padding:10px 14px; background:rgba(239,68,68,.08); border:1px solid rgba(239,68,68,.25);
  border-radius:var(--radius-md); font-size:12px; margin-bottom:12px;
}
.uninstall-confirm-banner span { flex:1; }
.conf-btn {
  padding:4px 12px; border:1px solid var(--border); border-radius:var(--radius-sm);
  background:var(--bg-tertiary); color:var(--text-secondary); font-size:11px; cursor:pointer;
}
.conf-btn-danger { background:rgba(239,68,68,.12); color:var(--danger); border-color:rgba(239,68,68,.3); }
.conf-btn-danger:hover { background:rgba(239,68,68,.2); }
.uninstall-row td { background:rgba(239,68,68,.04) !important; }
.uninstall-btn {
  display:inline-flex; align-items:center; justify-content:center;
  width:22px; height:22px; border-radius:4px; border:1px solid rgba(239,68,68,.3);
  background:rgba(239,68,68,.08); color:#ef4444; cursor:pointer; transition:all .15s;
}
.uninstall-btn:hover:not(:disabled) { background:rgba(239,68,68,.2); }
.uninstall-btn:disabled { opacity:.4; cursor:not-allowed; }
.export-btn {
  display:inline-flex; align-items:center; gap:5px; padding:3px 10px;
  border:1px solid var(--border); border-radius:var(--radius-sm); background:var(--bg-secondary);
  color:var(--text-muted); font-size:11px; cursor:pointer; transition:all .15s;
}
.export-btn:hover { color:var(--accent-primary); border-color:var(--accent-primary); }
@keyframes spin { to { transform:rotate(360deg); } }
.spin { animation:spin .8s linear infinite; }

.env-del-btn {
  display: inline-flex; align-items: center; justify-content: center;
  width: 20px; height: 20px; border-radius: 4px;
  border: 1px solid rgba(239,68,68,0.4);
  background: rgba(239,68,68,0.1); color: #ef4444;
  cursor: pointer; transition: all 0.15s;
}
.env-del-btn:hover { background: rgba(239,68,68,0.25); }
</style>
