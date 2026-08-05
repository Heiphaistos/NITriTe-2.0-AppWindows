<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import { invoke, isTauriContext } from "@/utils/invoke";
import type { CommandResult } from "@/types/diagnostic";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NTabs from "@/components/ui/NTabs.vue";
import NCollapse from "@/components/ui/NCollapse.vue";
import { useNotificationStore } from "@/stores/notifications";
import {
  Container, RefreshCw, Play, Square, RotateCcw, Trash2,
  AlertTriangle, FileText, X, Info, BarChart2, Database, Scissors,
} from "lucide-vue-next";

const notify = useNotificationStore();

interface DockerContainer {
  id: string; name: string; image: string;
  status: string; ports: string; created: string;
}
interface DockerImage { id: string; repository: string; tag: string; size: string; created: string; }
interface DockerVolume { name: string; driver: string; mountpoint: string; created: string; }
interface DockerInfo {
  available: boolean; version: string;
  containers: DockerContainer[]; images: DockerImage[];
}

const info          = ref<DockerInfo | null>(null);
const loading       = ref(false);
const actionLoading = ref<string | null>(null);
const activeTab     = ref("containers");

// Logs
const logsContainer = ref<{ id: string; name: string } | null>(null);
const logsText      = ref("");
const logsLoading   = ref(false);
const logsLines     = ref(100);

// Inspect
const inspectData   = ref<Record<string, string | null>>({});  // containerid → JSON string | null
const inspectLoading = ref<string | null>(null);

// Stats
const statsData     = ref<Record<string, string | null>>({});
const statsLoading  = ref<string | null>(null);

// Volumes
const volumes       = ref<DockerVolume[]>([]);
const volumesLoading = ref(false);
const pruneVolLoading = ref(false);

// Prune images
const pruneImgLoading = ref(false);

// ── Helpers ──────────────────────────────────────────────────────────────────
function isRunning(c: DockerContainer): boolean {
  return c.status.toLowerCase().includes("up");
}

function statusVariant(c: DockerContainer): "success" | "warning" | "danger" | "neutral" {
  if (c.status.toLowerCase().includes("up")) return "success";
  if (c.status.toLowerCase().includes("exit")) return "danger";
  return "neutral";
}

/** Parse "0.0.0.0:8080->80/tcp, ..." → ["8080:80/tcp", ...] */
function parsePorts(raw: string): string[] {
  if (!raw || raw === "—") return [];
  return raw.split(",").map(s => {
    const m = s.trim().match(/:(\d+)->(\d+\/\w+)/);
    if (m) return `${m[1]}:${m[2]}`;
    return s.trim();
  }).filter(Boolean);
}

// ── Load ─────────────────────────────────────────────────────────────────────
async function load() {
  loading.value = true;
  try {
    info.value = await invoke<DockerInfo>("get_docker_info");
  } catch (e: unknown) {
    notify.error("Erreur Docker", String(e));
  } finally {
    loading.value = false;
  }
}

async function loadVolumes() {
  volumesLoading.value = true;
  try {
    const raw = await invoke<CommandResult>("run_system_command", {
      cmd: "docker", args: ["volume", "ls", "--format", "{{.Name}}|{{.Driver}}|{{.Mountpoint}}|{{.CreatedAt}}"],
    });
    volumes.value = (raw?.stdout ?? "").trim().split("\n").filter(Boolean).map(line => {
      const [name, driver, mountpoint, created] = line.split("|");
      return { name: name ?? "", driver: driver ?? "", mountpoint: mountpoint ?? "", created: created?.slice(0, 16) ?? "" };
    });
  } catch (e: unknown) {
    volumes.value = [];
    if (isTauriContext()) notify.warning("Volumes Docker", (e instanceof Error ? e.message : String(e)).slice(0, 120));
  } finally {
    volumesLoading.value = false;
  }
}

// ── Container actions ────────────────────────────────────────────────────────
async function containerAction(id: string, action: string, name: string) {
  if (action === "rm" && !(await confirm(`Supprimer definitivement le container "${name}" ? Cette action est irreversible.`, { title: "Nitrite", kind: "warning" }))) return;
  actionLoading.value = id + action;
  try {
    await invoke("docker_container_action", { containerId: id, action });
    notify.success(`Container ${action}`, name);
    await load();
  } catch (e: unknown) {
    notify.error(`Erreur ${action}`, String(e));
  } finally {
    actionLoading.value = null;
  }
}

async function removeImage(id: string, repo: string) {
  if (!(await confirm(`Supprimer definitivement l'image "${repo}" ? Cette action est irreversible.`, { title: "Nitrite", kind: "warning" }))) return;
  actionLoading.value = id;
  try {
    await invoke("docker_image_remove", { imageId: id });
    notify.success("Image supprimee", repo);
    await load();
  } catch (e: unknown) {
    notify.error("Erreur suppression", String(e));
  } finally {
    actionLoading.value = null;
  }
}

// ── Logs ─────────────────────────────────────────────────────────────────────
async function openLogs(c: DockerContainer) {
  logsContainer.value = { id: c.id, name: c.name };
  logsText.value = "";
  logsLoading.value = true;
  try {
    logsText.value = await invoke<string>("docker_container_logs", { containerId: c.id, lines: logsLines.value });
  } catch (e: unknown) {
    logsText.value = `Erreur : ${String(e)}`;
  } finally {
    logsLoading.value = false;
  }
}

function closeLogs() { logsContainer.value = null; logsText.value = ""; }

// ── Inspect ──────────────────────────────────────────────────────────────────
async function inspectContainer(c: DockerContainer) {
  if (inspectData.value[c.id] !== undefined) {
    // toggle off
    const copy = { ...inspectData.value };
    delete copy[c.id];
    inspectData.value = copy;
    return;
  }
  inspectLoading.value = c.id;
  try {
    const raw = await invoke<CommandResult>("run_system_command", {
      cmd: "docker", args: ["inspect", c.id],
    });
    inspectData.value = { ...inspectData.value, [c.id]: raw?.stdout ?? "" };
  } catch (e: unknown) {
    inspectData.value = { ...inspectData.value, [c.id]: `Erreur : ${String(e)}` };
  } finally {
    inspectLoading.value = null;
  }
}

// ── Stats ────────────────────────────────────────────────────────────────────
async function fetchStats(c: DockerContainer) {
  statsLoading.value = c.id;
  try {
    const raw = await invoke<CommandResult>("run_system_command", {
      cmd: "docker",
      args: ["stats", c.id, "--no-stream", "--format", "{{.CPUPerc}} {{.MemUsage}}"],
    });
    statsData.value = { ...statsData.value, [c.id]: (raw?.stdout ?? "").trim() };
  } catch (e: unknown) {
    statsData.value = { ...statsData.value, [c.id]: `Erreur : ${String(e)}` };
  } finally {
    statsLoading.value = null;
  }
}

function parseStats(raw: string): { cpu: string; mem: string } {
  const parts = raw.split(" ");
  return { cpu: parts[0] ?? "—", mem: parts.slice(1).join(" ") || "—" };
}

// ── Prune images ─────────────────────────────────────────────────────────────
async function pruneImages() {
  if (!(await confirm("Supprimer toutes les images orphelines (dangling) ? Cette action est irreversible.", { title: "Nitrite", kind: "warning" }))) return;
  pruneImgLoading.value = true;
  try {
    const result = await invoke<CommandResult>("run_system_command", {
      cmd: "docker", args: ["image", "prune", "-f"],
    });
    notify.success("Nettoyage OK", (result?.stdout ?? "").trim().slice(0, 80) || "Images orphelines supprimees");
    await load();
  } catch (e: unknown) {
    notify.error("Erreur prune", String(e));
  } finally {
    pruneImgLoading.value = false;
  }
}

// ── Prune volumes ─────────────────────────────────────────────────────────────
async function pruneVolumes() {
  if (!(await confirm("Supprimer tous les volumes inutilises ? Cette action est irreversible.", { title: "Nitrite", kind: "warning" }))) return;
  pruneVolLoading.value = true;
  try {
    await invoke<CommandResult>("run_system_command", { cmd: "docker", args: ["volume", "prune", "-f"] });
    notify.success("Volumes pruned", "Volumes inutilises supprimes");
    await loadVolumes();
  } catch (e: unknown) {
    notify.error("Erreur prune volumes", String(e));
  } finally {
    pruneVolLoading.value = false;
  }
}

const tabs = computed(() => [
  { id: "containers", label: "Containers" },
  { id: "images", label: "Images" },
  { id: "volumes", label: "Volumes" },
]);

onMounted(async () => {
  await load();
  // loadVolumes() spawne "docker volume ls" directement — inutile et source d'un
  // warning technique brut ("Erreur lancement docker: program not found") si
  // Docker n'est pas installé, alors que l'état "Docker non détecté" ci-dessus
  // l'explique déjà proprement.
  if (info.value?.available) await loadVolumes();
});
</script>

<template>
  <div class="docker-page">
    <div class="page-header">
      <div>
        <h1>Docker Manager</h1>
        <p class="page-subtitle">Gerez vos containers et images Docker</p>
      </div>
      <NButton variant="ghost" size="sm" :loading="loading" @click="load">
        <RefreshCw :size="14" />
      </NButton>
    </div>

    <div v-if="loading" style="display:flex;justify-content:center;padding:40px"><NSpinner :size="24" /></div>

    <NCard v-else-if="info && !info.available">
      <div style="display:flex;flex-direction:column;align-items:center;gap:16px;padding:32px;text-align:center">
        <AlertTriangle :size="40" style="color:var(--warning)" />
        <div>
          <p style="font-size:16px;font-weight:600;margin-bottom:8px">Docker non detecte</p>
          <p style="font-size:13px;color:var(--text-muted);line-height:1.6">
            Docker Desktop n'est pas installe ou n'est pas demarre.<br>
            Installez Docker Desktop depuis <strong>docker.com</strong> et lancez-le.
          </p>
        </div>
      </div>
    </NCard>

    <template v-else-if="info">
      <!-- Stats bar -->
      <div class="stats-bar">
        <div class="stat-pill">
          <span class="stat-val">{{ info.version }}</span>
          <span class="stat-label">Docker</span>
        </div>
        <div class="stat-pill">
          <span class="stat-val success">{{ info.containers.filter(c => isRunning(c)).length }}</span>
          <span class="stat-label">Running</span>
        </div>
        <div class="stat-pill">
          <span class="stat-val muted">{{ info.containers.filter(c => !isRunning(c)).length }}</span>
          <span class="stat-label">Stopped</span>
        </div>
        <div class="stat-pill">
          <span class="stat-val">{{ info.images.length }}</span>
          <span class="stat-label">Images</span>
        </div>
        <div class="stat-pill">
          <span class="stat-val">{{ volumes.length }}</span>
          <span class="stat-label">Volumes</span>
        </div>
      </div>

      <NTabs v-model="activeTab" :tabs="tabs">
        <!-- ── Containers ─────────────────────────────────────────────── -->
        <template #containers>
          <NCard>
            <div v-if="info.containers.length === 0" class="empty-state">
              <Container :size="28" style="color:var(--text-muted);opacity:.3" />
              <p>Aucun container</p>
            </div>
            <table v-else class="docker-table">
              <thead>
                <tr>
                  <th>Nom</th><th>Image</th><th>Status</th><th>Ports</th><th>Cree</th><th>Actions</th>
                </tr>
              </thead>
              <tbody>
                <template v-for="c in info.containers" :key="c.id">
                  <tr :class="{ 'log-active': logsContainer?.id === c.id }">
                    <td class="bold">{{ c.name }}</td>
                    <td class="mono muted">{{ c.image }}</td>
                    <td><NBadge :variant="statusVariant(c)" size="sm">{{ c.status.slice(0,25) }}</NBadge></td>
                    <td class="ports-cell">
                      <template v-if="parsePorts(c.ports).length">
                        <span v-for="p in parsePorts(c.ports)" :key="p" class="port-pill">{{ p }}</span>
                      </template>
                      <span v-else class="muted">—</span>
                    </td>
                    <td class="small muted">{{ c.created?.slice(0,16) }}</td>
                    <td>
                      <div class="action-btns">
                        <button v-if="!isRunning(c)" class="act-btn green" @click="containerAction(c.id,'start',c.name)" :disabled="actionLoading === c.id+'start'" title="Demarrer"><Play :size="12" /></button>
                        <button v-if="isRunning(c)" class="act-btn orange" @click="containerAction(c.id,'stop',c.name)" :disabled="actionLoading === c.id+'stop'" title="Arreter"><Square :size="12" /></button>
                        <button class="act-btn blue" @click="containerAction(c.id,'restart',c.name)" :disabled="!!actionLoading" title="Redemarrer"><RotateCcw :size="12" /></button>
                        <button class="act-btn purple" @click="openLogs(c)" title="Logs"><FileText :size="12" /></button>
                        <button
                          class="act-btn teal"
                          @click="inspectContainer(c)"
                          :disabled="inspectLoading === c.id"
                          title="Details"
                        >
                          <Info :size="12" />
                        </button>
                        <button
                          class="act-btn indigo"
                          @click="fetchStats(c)"
                          :disabled="statsLoading === c.id || !!statsLoading"
                          title="Stats CPU/RAM"
                        >
                          <BarChart2 :size="12" />
                        </button>
                        <button class="act-btn red" @click="containerAction(c.id,'rm',c.name)" :disabled="!!actionLoading || isRunning(c)" title="Supprimer"><Trash2 :size="12" /></button>
                      </div>
                    </td>
                  </tr>

                  <!-- Stats row -->
                  <tr v-if="statsData[c.id] !== undefined" class="detail-row">
                    <td colspan="6">
                      <div class="stats-inline">
                        <BarChart2 :size="13" style="color:var(--info)" />
                        <span class="stats-item">CPU : <strong>{{ parseStats(statsData[c.id]!).cpu }}</strong></span>
                        <span class="stats-sep">|</span>
                        <span class="stats-item">MEM : <strong>{{ parseStats(statsData[c.id]!).mem }}</strong></span>
                      </div>
                    </td>
                  </tr>

                  <!-- Inspect row -->
                  <tr v-if="inspectData[c.id] !== undefined" class="detail-row">
                    <td colspan="6">
                      <NCollapse title="docker inspect" :defaultOpen="true" variant="subtle">
                        <pre class="inspect-pre">{{ inspectData[c.id] }}</pre>
                      </NCollapse>
                    </td>
                  </tr>
                </template>
              </tbody>
            </table>
          </NCard>

          <!-- Logs panel -->
          <NCard v-if="logsContainer">
            <template #header>
              <div style="display:flex;align-items:center;gap:8px;width:100%">
                <FileText :size="15" />
                <span>Logs — <strong>{{ logsContainer.name }}</strong></span>
                <select v-model="logsLines" class="lines-select" @change="openLogs(info!.containers.find(c=>c.id===logsContainer!.id)!)">
                  <option :value="50">50 lignes</option>
                  <option :value="100">100 lignes</option>
                  <option :value="200">200 lignes</option>
                  <option :value="500">500 lignes</option>
                </select>
                <NButton variant="ghost" size="sm" @click="openLogs(info!.containers.find(c=>c.id===logsContainer!.id)!)" :loading="logsLoading" style="margin-left:auto">
                  <RefreshCw :size="13" />
                </NButton>
                <button class="close-btn" @click="closeLogs"><X :size="14" /></button>
              </div>
            </template>
            <div v-if="logsLoading" style="display:flex;justify-content:center;padding:20px"><NSpinner /></div>
            <pre v-else class="logs-pre">{{ logsText || '(aucun log)' }}</pre>
          </NCard>
        </template>

        <!-- ── Images ──────────────────────────────────────────────────── -->
        <template #images>
          <div class="images-toolbar">
            <NButton variant="danger" size="sm" :loading="pruneImgLoading" @click="pruneImages">
              <Scissors :size="13" />
              Nettoyer images orphelines
            </NButton>
          </div>
          <NCard>
            <div v-if="info.images.length === 0" class="empty-state">
              <Container :size="28" style="color:var(--text-muted);opacity:.3" />
              <p>Aucune image</p>
            </div>
            <table v-else class="docker-table">
              <thead>
                <tr><th>Repository</th><th>Tag</th><th>Taille</th><th>Cree</th><th>Actions</th></tr>
              </thead>
              <tbody>
                <tr v-for="img in info.images" :key="img.id">
                  <td class="bold">{{ img.repository }}</td>
                  <td><NBadge variant="accent" size="sm">{{ img.tag }}</NBadge></td>
                  <td class="mono">{{ img.size }}</td>
                  <td class="small muted">{{ img.created?.slice(0,16) }}</td>
                  <td><button class="act-btn red" @click="removeImage(img.id, img.repository)" :disabled="!!actionLoading" title="Supprimer"><Trash2 :size="12" /></button></td>
                </tr>
              </tbody>
            </table>
          </NCard>
        </template>

        <!-- ── Volumes ─────────────────────────────────────────────────── -->
        <template #volumes>
          <div class="images-toolbar">
            <NButton variant="ghost" size="sm" :loading="volumesLoading" @click="loadVolumes">
              <RefreshCw :size="13" />
              Rafraichir
            </NButton>
            <NButton variant="danger" size="sm" :loading="pruneVolLoading" @click="pruneVolumes">
              <Trash2 :size="13" />
              Pruner volumes inutilises
            </NButton>
          </div>
          <NCard>
            <div v-if="volumesLoading" style="display:flex;justify-content:center;padding:24px"><NSpinner /></div>
            <div v-else-if="volumes.length === 0" class="empty-state">
              <Database :size="28" style="color:var(--text-muted);opacity:.3" />
              <p>Aucun volume</p>
            </div>
            <table v-else class="docker-table">
              <thead>
                <tr><th>Nom</th><th>Driver</th><th>Mountpoint</th><th>Cree</th></tr>
              </thead>
              <tbody>
                <tr v-for="v in volumes" :key="v.name">
                  <td class="bold mono">{{ v.name }}</td>
                  <td><NBadge variant="neutral" size="sm">{{ v.driver }}</NBadge></td>
                  <td class="mono small muted">{{ v.mountpoint }}</td>
                  <td class="small muted">{{ v.created }}</td>
                </tr>
              </tbody>
            </table>
          </NCard>
        </template>
      </NTabs>
    </template>
  </div>
</template>

<style scoped>
.docker-page { display:flex; flex-direction:column; gap:16px; }
.page-header { display:flex; justify-content:space-between; align-items:flex-start; }
.page-header h1 { font-size:24px; font-weight:700; }
.page-subtitle { color:var(--text-muted); font-size:13px; margin-top:2px; }

.stats-bar { display:flex; gap:12px; flex-wrap:wrap; }
.stat-pill { background:var(--bg-secondary); border:1px solid var(--border); border-radius:var(--radius-md); padding:10px 20px; display:flex; flex-direction:column; align-items:center; gap:2px; }
.stat-val { font-size:18px; font-weight:700; color:var(--text-primary); }
.stat-val.success { color:var(--success); }
.stat-val.muted { color:var(--text-muted); }
.stat-label { font-size:11px; color:var(--text-muted); }

.docker-table { width:100%; border-collapse:collapse; font-size:12px; }
.docker-table th { padding:8px 12px; text-align:left; color:var(--text-muted); font-size:10px; font-weight:700; text-transform:uppercase; letter-spacing:0.06em; border-bottom:1px solid var(--border); }
.docker-table td { padding:8px 12px; border-bottom:1px solid var(--border); vertical-align:middle; }
.docker-table tr:last-child td { border-bottom:none; }
.docker-table tr:hover td { background:var(--bg-tertiary); }
.docker-table tr.log-active td { background:rgba(139,92,246,.06); }
.docker-table tr.detail-row td { background:var(--bg-secondary); padding:4px 12px 8px; }

.bold { font-weight:600; color:var(--text-primary); }
.mono { font-family:monospace; color:var(--text-secondary); }
.small { font-size:11px; }
.muted { color:var(--text-muted); }

/* Ports */
.ports-cell { display:flex; flex-wrap:wrap; gap:4px; align-items:center; }
.port-pill { font-family:monospace; font-size:10px; background:rgba(59,130,246,.12); color:#3b82f6; border:1px solid rgba(59,130,246,.25); border-radius:4px; padding:1px 5px; white-space:nowrap; }

/* Action buttons */
.action-btns { display:flex; gap:4px; flex-wrap:wrap; }
.act-btn { padding:4px 6px; border:1px solid var(--border); border-radius:4px; cursor:pointer; display:flex; align-items:center; transition:all var(--transition-fast); background:transparent; }
.act-btn:disabled { opacity:.4; cursor:not-allowed; }
.act-btn.green  { background:rgba(34,197,94,.1);  color:var(--success); border-color:rgba(34,197,94,.3); }
.act-btn.green:hover:not(:disabled)  { background:rgba(34,197,94,.2); }
.act-btn.orange { background:rgba(249,115,22,.1); color:var(--warning); border-color:rgba(249,115,22,.3); }
.act-btn.orange:hover:not(:disabled) { background:rgba(249,115,22,.2); }
.act-btn.blue   { background:rgba(59,130,246,.1); color:#3b82f6; border-color:rgba(59,130,246,.3); }
.act-btn.blue:hover:not(:disabled)   { background:rgba(59,130,246,.2); }
.act-btn.purple { background:rgba(139,92,246,.1); color:#8b5cf6; border-color:rgba(139,92,246,.3); }
.act-btn.purple:hover:not(:disabled) { background:rgba(139,92,246,.2); }
.act-btn.teal   { background:rgba(20,184,166,.1); color:#14b8a6; border-color:rgba(20,184,166,.3); }
.act-btn.teal:hover:not(:disabled)   { background:rgba(20,184,166,.2); }
.act-btn.indigo { background:rgba(99,102,241,.1); color:#6366f1; border-color:rgba(99,102,241,.3); }
.act-btn.indigo:hover:not(:disabled) { background:rgba(99,102,241,.2); }
.act-btn.red    { background:rgba(239,68,68,.1);  color:var(--danger); border-color:rgba(239,68,68,.3); }
.act-btn.red:hover:not(:disabled)    { background:rgba(239,68,68,.2); }

/* Stats inline */
.stats-inline { display:flex; align-items:center; gap:8px; padding:6px 4px; font-size:12px; color:var(--text-secondary); }
.stats-item strong { color:var(--text-primary); font-family:monospace; }
.stats-sep { color:var(--border); }

/* Inspect */
.inspect-pre { font-family:"JetBrains Mono",monospace; font-size:11px; color:var(--text-secondary); background:var(--bg-tertiary); border-radius:var(--radius-md); padding:10px; white-space:pre-wrap; word-break:break-all; max-height:300px; overflow-y:auto; margin:0; }

/* Toolbar above images/volumes */
.images-toolbar { display:flex; gap:8px; margin-bottom:4px; }

/* Logs */
.lines-select { padding:3px 8px; border:1px solid var(--border); border-radius:var(--radius-sm); background:var(--bg-tertiary); color:var(--text-secondary); font-size:11px; cursor:pointer; margin-left:auto; }
.close-btn { background:none; border:none; color:var(--text-muted); cursor:pointer; padding:4px; border-radius:4px; display:flex; align-items:center; }
.close-btn:hover { color:var(--text-primary); background:var(--bg-tertiary); }
.logs-pre { font-family:"JetBrains Mono",monospace; font-size:11px; color:var(--text-secondary); background:var(--bg-tertiary); border-radius:var(--radius-md); padding:12px; white-space:pre-wrap; word-break:break-all; max-height:400px; overflow-y:auto; line-height:1.6; margin:0; }
.empty-state { display:flex; flex-direction:column; align-items:center; gap:8px; padding:40px; color:var(--text-muted); font-size:13px; }
</style>
