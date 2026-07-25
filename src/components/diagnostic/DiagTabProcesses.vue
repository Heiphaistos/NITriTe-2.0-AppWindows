<script setup lang="ts">
import { ref, computed } from "vue";
import { Search, RefreshCw, Cpu, Server, Play, Clock, X, SquarePlay, Square, RotateCcw, Trash2 } from "lucide-vue-next";
import { confirm } from "@tauri-apps/plugin-dialog";
import { invoke } from "@/utils/invoke";
import NBadge from "@/components/ui/NBadge.vue";
import NProgress from "@/components/ui/NProgress.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import NButton from "@/components/ui/NButton.vue";
import NCollapse from "@/components/ui/NCollapse.vue";
import { useExportData } from '@/composables/useExportData';
import type { ProcessInfo, ServiceInfo, StartupProgram, ScheduledTask } from "@/types/diagnostic";
const { exportCSV } = useExportData();

// Actions enrichies processus
async function openProcessLocation(path: string) {
  if (!path) return;
  try {
    const folder = path.includes('\\') ? path.substring(0, path.lastIndexOf('\\')) : path;
    await invoke('open_path', { path: folder });
  } catch {}
}

async function openVirusTotal(name: string) {
  try {
    await invoke('open_url', { url: `https://www.virustotal.com/gui/search/${encodeURIComponent(name)}` });
  } catch {}
}

async function searchProcess(name: string) {
  try {
    await invoke('open_url', { url: `https://www.google.com/search?q=${encodeURIComponent(name + ' process windows')}` });
  } catch {}
}

function doExportProcesses(processList: ProcessInfo[]) {
  exportCSV(processList.map(p => ({
    PID: p.pid, Nom: p.name, CPU: p.cpu_percent?.toFixed(1) + '%',
    RAM: p.memory_mb?.toFixed(0) + ' MB', Chemin: p.path || '',
    Statut: p.status || '',
  })), 'processus-' + new Date().toISOString().slice(0, 10));
}

const props = defineProps<{
  tab: string;
  processes: ProcessInfo[];
  services: ServiceInfo[];
  startupPrograms: StartupProgram[];
  tasks: ScheduledTask[];
  onRefresh: () => void;
}>();

const procSearch = ref("");
const PROC_SORT_OPTS = [['cpu','CPU'],['mem','RAM'],['name','Nom']] as const;
const procSort = ref<"cpu"|"mem"|"name">("cpu");
const svcSearch = ref("");
const svcFilter = ref("all");
const taskSearch = ref("");
const actionMsg = ref("");
const actionErr = ref(false);
const busyPid = ref<number | null>(null);
const busySvc = ref<string | null>(null);
const busyTask = ref<string | null>(null);

function showMsg(msg: string, err = false) {
  actionMsg.value = msg;
  actionErr.value = err;
  setTimeout(() => { actionMsg.value = ""; }, 4000);
}

async function killProc(pid: number, name: string) {
  if (!(await confirm(`Terminer le processus "${name}" (PID: ${pid}) ?`, { title: "Nitrite", kind: "warning" }))) return;
  busyPid.value = pid;
  try {
    const r = await invoke<string>("kill_process", { pid });
    showMsg(r);
    setTimeout(props.onRefresh, 800);
  } catch (e: unknown) {
    showMsg((e instanceof Error ? e.message : String(e)) || "Erreur", true);
  } finally { busyPid.value = null; }
}

async function ctrlService(name: string, action: string) {
  busySvc.value = name + action;
  try {
    const r = await invoke<string>("control_service", { name, action });
    showMsg(r);
    setTimeout(props.onRefresh, 1000);
  } catch (e: unknown) {
    showMsg((e instanceof Error ? e.message : String(e)) || "Erreur service", true);
  } finally { busySvc.value = null; }
}

async function toggleStartup(item: StartupProgram, enable: boolean) {
  try {
    const r = await invoke<string>("toggle_startup_program", {
      name: item.name,
      location: item.location,
      command: item.command,
      enable
    });
    showMsg(r);
    setTimeout(props.onRefresh, 500);
  } catch (e: unknown) {
    showMsg((e instanceof Error ? e.message : String(e)) || "Erreur démarrage", true);
  }
}

async function removeStartup(item: StartupProgram) {
  if (!(await confirm(`Supprimer "${item.name}" du démarrage ?`, { title: "Nitrite", kind: "warning" }))) return;
  try {
    const r = await invoke<string>("remove_startup_program", { name: item.name, location: item.location });
    showMsg(r);
    setTimeout(props.onRefresh, 500);
  } catch (e: unknown) {
    showMsg((e instanceof Error ? e.message : String(e)) || "Erreur", true);
  }
}

async function deleteTask(t: ScheduledTask) {
  if (!(await confirm(`Supprimer la tâche "${t.name}" ?`, { title: "Nitrite", kind: "warning" }))) return;
  busyTask.value = t.name;
  try {
    const r = await invoke<string>("delete_scheduled_task", { taskName: t.name, taskPath: t.path || "" });
    showMsg(r);
    setTimeout(props.onRefresh, 800);
  } catch (e: unknown) {
    showMsg((e instanceof Error ? e.message : String(e)) || "Erreur", true);
  } finally { busyTask.value = null; }
}

async function runTaskNow(t: ScheduledTask) {
  busyTask.value = t.name + "run";
  try {
    const fullName = t.path && t.path !== "\\" ? t.path.replace(/\\$/, "") + "\\" + t.name : t.name;
    const r = await invoke<string>("run_scheduled_task_now", { taskName: fullName });
    showMsg(r);
  } catch (e: unknown) {
    showMsg((e instanceof Error ? e.message : String(e)) || "Erreur", true);
  } finally { busyTask.value = null; }
}

const filteredProcs = computed(() => {
  let list = [...props.processes];
  const q = procSearch.value.toLowerCase();
  if (q) list = list.filter(p => p.name.toLowerCase().includes(q) || (p.path || "").toLowerCase().includes(q));
  if (procSort.value === "cpu") list.sort((a, b) => b.cpu_percent - a.cpu_percent);
  else if (procSort.value === "mem") list.sort((a, b) => b.memory_mb - a.memory_mb);
  else list.sort((a, b) => a.name.localeCompare(b.name));
  return list;
});

const filteredSvcs = computed(() => {
  let list = [...props.services];
  const q = svcSearch.value.toLowerCase();
  if (q) list = list.filter(s => s.display_name.toLowerCase().includes(q) || s.name.toLowerCase().includes(q));
  if (svcFilter.value !== "all") list = list.filter(s => s.state === svcFilter.value);
  return list;
});

const filteredTasks = computed(() => {
  const q = taskSearch.value.toLowerCase();
  if (!q) return props.tasks;
  return props.tasks.filter(t => t.name.toLowerCase().includes(q) || t.path.toLowerCase().includes(q));
});

const totalCpuUsage = computed(() => {
  const sum = props.processes.reduce((a, p) => a + p.cpu_percent, 0);
  return Math.min(100, Math.round(sum * 10) / 10);
});
const totalRamMb = computed(() => props.processes.reduce((a, p) => a + p.memory_mb, 0));
const runningCount = computed(() => props.services.filter(s => s.state === "Running").length);
</script>

<template>
  <!-- Processus actifs -->
  <template v-if="tab === 'processes'">
    <div class="diag-tab-content">
      <DiagBanner :icon="Cpu" title="Processus Actifs" desc="CPU, mémoire et tous les processus en cours d'exécution" color="red" />
      <div style="display:flex;gap:12px;flex-wrap:wrap;margin-bottom:12px">
        <div class="card-block" style="flex:1;min-width:140px;margin-bottom:0">
          <p class="diag-section-label" style="margin:0 0 4px 0">Processus actifs</p>
          <span style="font-size:24px;font-weight:700;color:var(--accent)">{{ processes.length }}</span>
        </div>
        <div class="card-block" style="flex:1;min-width:140px;margin-bottom:0">
          <p class="diag-section-label" style="margin:0 0 4px 0">CPU total</p>
          <span style="font-size:24px;font-weight:700" :class="totalCpuUsage > 80 ? 'text-err' : totalCpuUsage > 50 ? 'text-warn' : 'ic-ok'">{{ totalCpuUsage }}%</span>
        </div>
        <div class="card-block" style="flex:1;min-width:140px;margin-bottom:0">
          <p class="diag-section-label" style="margin:0 0 4px 0">RAM allouée</p>
          <span style="font-size:24px;font-weight:700">{{ (totalRamMb / 1024).toFixed(1) }} GB</span>
        </div>
      </div>

      <div style="display:flex;gap:8px;align-items:center;margin-bottom:12px;flex-wrap:wrap">
        <div class="diag-search" style="flex:1;margin:0">
          <Search :size="14" />
          <input v-model="procSearch" placeholder="Filtrer par nom ou chemin..." class="diag-search-input" />
          <span class="muted">{{ filteredProcs.length }}/{{ processes.length }}</span>
        </div>
        <div style="display:flex;gap:4px">
          <NBadge v-for="[k,v] in PROC_SORT_OPTS" :key="k"
            :variant="procSort === k ? 'info' : 'default'" style="cursor:pointer" @click="procSort = k">{{ v }}</NBadge>
        </div>
        <button class="diag-search" style="padding:6px 10px;cursor:pointer;border:none" @click="onRefresh">
          <RefreshCw :size="13" style="color:var(--text-secondary)" />
        </button>
        <button @click="doExportProcesses(filteredProcs)"
          style="font-size:11px;padding:4px 10px;border:1px solid var(--border);border-radius:6px;background:var(--bg-secondary);color:var(--text-secondary);cursor:pointer">
          ↓ CSV
        </button>
      </div>

      <!-- Toast global -->
      <teleport to="body">
        <div v-if="actionMsg" :class="['action-toast', actionErr ? 'action-toast-err' : 'action-toast-ok']">{{ actionMsg }}</div>
      </teleport>

      <div class="table-wrap">
        <table class="data-table">
          <thead>
            <tr><th>PID</th><th>Nom</th><th>CPU %</th><th>RAM (MB)</th><th>GPU %</th><th>État</th><th>Actions</th></tr>
          </thead>
          <tbody>
            <tr v-for="p in filteredProcs.slice(0, 300)" :key="p.pid">
              <td class="muted">{{ p.pid }}</td>
              <td style="font-weight:500;max-width:160px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap" :title="p.path || p.name">{{ p.name }}</td>
              <td>
                <div style="display:flex;align-items:center;gap:6px;min-width:80px">
                  <span style="min-width:36px" :class="p.cpu_percent > 20 ? 'text-warn' : ''">{{ p.cpu_percent.toFixed(1) }}%</span>
                  <div style="flex:1;min-width:40px"><NProgress :value="p.cpu_percent" size="sm" :variant="p.cpu_percent > 20 ? 'warning' : 'default'" /></div>
                </div>
              </td>
              <td :class="p.memory_mb > 500 ? 'text-warn' : ''">{{ p.memory_mb.toFixed(0) }}</td>
              <td :class="(p.gpu_percent || 0) > 10 ? 'text-warn' : 'muted'">
                {{ p.gpu_percent != null ? p.gpu_percent.toFixed(1) + '%' : '—' }}
              </td>
              <td><NBadge :variant="p.status === 'Running' ? 'success' : p.status === 'Sleeping' ? 'default' : 'warning'" style="font-size:10px">{{ p.status }}</NBadge></td>
              <td>
                <div style="display:flex;gap:3px;align-items:center">
                  <button
                    class="kill-btn"
                    :disabled="busyPid === p.pid"
                    @click="killProc(p.pid, p.name)"
                    title="Terminer le processus"
                  >
                    <X :size="12" />
                  </button>
                  <button @click="openProcessLocation(p.path)" :disabled="!p.path"
                    style="font-size:10px;padding:2px 6px;border:1px solid var(--border);border-radius:3px;background:none;color:var(--text-muted);cursor:pointer;opacity:0.7"
                    :style="{ opacity: p.path ? '0.85' : '0.3', cursor: p.path ? 'pointer' : 'not-allowed' }"
                    title="Ouvrir emplacement">📂</button>
                  <button @click="openVirusTotal(p.name)"
                    style="font-size:10px;padding:2px 6px;border:1px solid var(--border);border-radius:3px;background:none;color:var(--text-muted);cursor:pointer;opacity:0.7"
                    title="Vérifier VirusTotal">🛡</button>
                  <button @click="searchProcess(p.name)"
                    style="font-size:10px;padding:2px 6px;border:1px solid var(--border);border-radius:3px;background:none;color:var(--text-muted);cursor:pointer;opacity:0.7"
                    title="Rechercher sur Google">🔍</button>
                </div>
              </td>
            </tr>
            <tr v-if="!filteredProcs.length">
              <td colspan="7" style="text-align:center;color:var(--text-muted);padding:24px;font-size:13px">Aucun processus ne correspond à la recherche</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </template>

  <!-- Services Windows -->
  <template v-else-if="tab === 'services'">
    <div class="diag-tab-content">
      <DiagBanner :icon="Server" title="Services Windows" desc="Services système actifs, arrêtés et désactivés" color="blue" />
      <div style="display:flex;gap:12px;flex-wrap:wrap;margin-bottom:12px">
        <div class="card-block" style="flex:1;min-width:120px;margin-bottom:0">
          <p class="diag-section-label" style="margin:0 0 4px 0">Total services</p>
          <span style="font-size:24px;font-weight:700">{{ services.length }}</span>
        </div>
        <div class="card-block" style="flex:1;min-width:120px;margin-bottom:0">
          <p class="diag-section-label" style="margin:0 0 4px 0">En cours</p>
          <span style="font-size:24px;font-weight:700;color:var(--success)">{{ runningCount }}</span>
        </div>
        <div class="card-block" style="flex:1;min-width:120px;margin-bottom:0">
          <p class="diag-section-label" style="margin:0 0 4px 0">Arrêtés</p>
          <span style="font-size:24px;font-weight:700;color:var(--text-secondary)">{{ services.length - runningCount }}</span>
        </div>
      </div>

      <div style="display:flex;gap:8px;margin-bottom:12px;flex-wrap:wrap">
        <div class="diag-search" style="flex:1;margin:0">
          <Search :size="14" />
          <input v-model="svcSearch" placeholder="Rechercher un service..." class="diag-search-input" />
          <span class="muted">{{ filteredSvcs.length }}</span>
        </div>
        <div style="display:flex;gap:4px;flex-wrap:wrap">
          <NBadge v-for="[k,v] in [['all','Tous'],['Running','En cours'],['Stopped','Arrêtés']]" :key="k"
            :variant="svcFilter === k ? 'info' : 'default'" style="cursor:pointer" @click="svcFilter = k">{{ v }}</NBadge>
        </div>
      </div>

      <div class="table-wrap">
        <table class="data-table">
          <thead>
            <tr><th>Nom</th><th>Nom affiché</th><th>État</th><th>Démarrage</th><th>Compte</th><th>Actions</th></tr>
          </thead>
          <tbody>
            <tr v-for="(s, i) in filteredSvcs.slice(0, 400)" :key="i">
              <td><code style="font-size:11px">{{ s.name }}</code></td>
              <td style="font-weight:500">{{ s.display_name }}</td>
              <td>
                <NBadge
                  :variant="s.state === 'Running' ? 'success' : s.state === 'Stopped' ? 'default' : 'warning'"
                  style="font-size:10px">{{ s.state }}</NBadge>
              </td>
              <td>
                <NBadge
                  :variant="s.start_mode === 'Auto' ? 'info' : s.start_mode === 'Disabled' ? 'danger' : 'default'"
                  style="font-size:10px">{{ s.start_mode }}</NBadge>
              </td>
              <td class="muted" style="font-size:11px;max-width:100px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ s.account || "—" }}</td>
              <td style="white-space:nowrap">
                <div style="display:flex;gap:4px">
                  <button v-if="s.state !== 'Running'" class="svc-btn svc-start"
                    :disabled="busySvc === s.name + 'start'"
                    title="Démarrer" @click="ctrlService(s.name, 'start')">
                    <SquarePlay :size="12" />
                  </button>
                  <button v-if="s.state === 'Running'" class="svc-btn svc-stop"
                    :disabled="busySvc === s.name + 'stop'"
                    title="Arrêter" @click="ctrlService(s.name, 'stop')">
                    <Square :size="12" />
                  </button>
                  <button v-if="s.state === 'Running'" class="svc-btn svc-restart"
                    :disabled="busySvc === s.name + 'restart'"
                    title="Redémarrer" @click="ctrlService(s.name, 'restart')">
                    <RotateCcw :size="12" />
                  </button>
                </div>
              </td>
            </tr>
            <tr v-if="!filteredSvcs.length">
              <td colspan="6" style="text-align:center;color:var(--text-muted);padding:24px;font-size:13px">Aucun service ne correspond à la recherche</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </template>

  <!-- Démarrage -->
  <template v-else-if="tab === 'startup'">
    <div class="diag-tab-content">
      <DiagBanner :icon="Play" title="Programmes de Démarrage" desc="Applications lancées automatiquement au démarrage" color="orange" />
      <NCollapse :title="'Programmes au démarrage — ' + startupPrograms.length" storageKey="diag-startup-programs" :defaultOpen="true">
        <div v-if="!startupPrograms.length" class="diag-empty">Aucun programme au démarrage trouvé</div>
        <div class="table-wrap">
          <table class="data-table">
            <thead><tr><th>Nom</th><th>Catégorie</th><th>Source</th><th>Portée</th><th>Commande</th><th>Actions</th></tr></thead>
            <tbody>
              <tr v-for="(s, i) in startupPrograms" :key="i">
                <td style="font-weight:500">{{ s.name }}</td>
                <td>
                  <NBadge
                    :variant="s.category === 'Microsoft / Windows' ? 'info' : s.category === 'Sécurité' ? 'success' : s.category === 'Tiers' ? 'warning' : 'default'"
                    style="font-size:10px">{{ s.category }}</NBadge>
                </td>
                <td class="muted" style="font-size:11px">{{ s.location }}</td>
                <td><NBadge :variant="s.user === 'Tous les utilisateurs' ? 'neutral' : 'default'" style="font-size:10px">{{ s.user }}</NBadge></td>
                <td class="muted" style="font-size:10px;max-width:160px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap" :title="s.command">{{ s.command }}</td>
                <td style="white-space:nowrap">
                  <div style="display:flex;gap:4px">
                    <button class="svc-btn svc-restart" title="Désactiver" @click="toggleStartup(s, false)">
                      <Square :size="12" />
                    </button>
                    <button class="svc-btn svc-stop" title="Supprimer" @click="removeStartup(s)">
                      <Trash2 :size="12" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </NCollapse>
    </div>
  </template>

  <!-- Tâches planifiées -->
  <template v-else-if="tab === 'tasks'">
    <div class="diag-tab-content">
      <DiagBanner :icon="Clock" title="Tâches Planifiées" desc="Tâches planifiées Windows et leur état" color="purple" />
      <div class="diag-search">
        <Search :size="14" />
        <input v-model="taskSearch" placeholder="Rechercher une tâche..." class="diag-search-input" />
        <span class="muted">{{ filteredTasks.length }}/{{ tasks.length }}</span>
      </div>
      <div class="table-wrap">
        <table class="data-table">
          <thead><tr><th>Nom</th><th>Chemin</th><th>État</th><th>Déclencheur</th><th>Dernier exec.</th><th>Résultat</th><th>Actions</th></tr></thead>
          <tbody>
            <tr v-for="(t, i) in filteredTasks.slice(0, 300)" :key="i">
              <td style="font-weight:500;max-width:140px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ t.name }}</td>
              <td class="muted" style="font-size:10px;max-width:100px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ t.path }}</td>
              <td>
                <NBadge
                  :variant="t.state === 'Ready' ? 'success' : t.state === 'Disabled' ? 'default' : t.state === 'Running' ? 'info' : 'warning'"
                  style="font-size:10px">{{ t.state }}</NBadge>
              </td>
              <td class="muted" style="font-size:11px">{{ t.trigger || "—" }}</td>
              <td class="muted" style="font-size:11px">{{ t.last_run_time }}</td>
              <td>
                <NBadge :variant="t.last_task_result === 0 ? 'success' : 'danger'" style="font-size:10px">
                  {{ t.last_task_result === 0 ? 'OK' : '0x' + t.last_task_result.toString(16).toUpperCase() }}
                </NBadge>
              </td>
              <td style="white-space:nowrap">
                <div style="display:flex;gap:4px">
                  <button class="svc-btn svc-start" title="Exécuter maintenant"
                    :disabled="busyTask === t.name + 'run'"
                    @click="runTaskNow(t)">
                    <SquarePlay :size="12" />
                  </button>
                  <button class="svc-btn svc-stop" title="Supprimer la tâche"
                    :disabled="busyTask === t.name"
                    @click="deleteTask(t)">
                    <Trash2 :size="12" />
                  </button>
                </div>
              </td>
            </tr>
            <tr v-if="!filteredTasks.length">
              <td colspan="7" style="text-align:center;color:var(--text-muted);padding:24px;font-size:13px">Aucune tâche ne correspond à la recherche</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </template>
</template>

<style scoped>
.action-toast {
  position: fixed; bottom: 24px; right: 24px;
  padding: 12px 20px; border-radius: 8px; font-size: 13px; font-weight: 600;
  z-index: 9999; box-shadow: 0 4px 16px rgba(0,0,0,0.3);
}
.action-toast-ok { background: #1a4a1a; border: 1px solid #22c55e; color: #86efac; }
.action-toast-err { background: #4a1a1a; border: 1px solid #ef4444; color: #fca5a5; }

.kill-btn {
  display: inline-flex; align-items: center; justify-content: center;
  width: 22px; height: 22px; border-radius: 4px; border: 1px solid rgba(239,68,68,0.4);
  background: rgba(239,68,68,0.12); color: #ef4444; cursor: pointer;
  transition: all 0.15s;
}
.kill-btn:hover:not(:disabled) { background: rgba(239,68,68,0.3); }
.kill-btn:disabled { opacity: 0.4; cursor: not-allowed; }

.svc-btn {
  display: inline-flex; align-items: center; justify-content: center;
  width: 22px; height: 22px; border-radius: 4px; border: 1px solid rgba(255,255,255,0.1);
  background: rgba(255,255,255,0.05); cursor: pointer; transition: all 0.15s;
}
.svc-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.svc-start { border-color: rgba(34,197,94,0.4); color: #22c55e; }
.svc-start:hover:not(:disabled) { background: rgba(34,197,94,0.2); }
.svc-stop { border-color: rgba(239,68,68,0.4); color: #ef4444; }
.svc-stop:hover:not(:disabled) { background: rgba(239,68,68,0.2); }
.svc-restart { border-color: rgba(234,179,8,0.4); color: #eab308; }
.svc-restart:hover:not(:disabled) { background: rgba(234,179,8,0.2); }
</style>
