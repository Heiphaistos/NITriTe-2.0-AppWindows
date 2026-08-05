<script setup lang="ts">
import { ref } from "vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import { invoke } from "@/utils/invoke";
import NButton from "@/components/ui/NButton.vue";
import { Cpu, Layers, Settings, Terminal, CheckCircle, XCircle } from "lucide-vue-next";

interface RepairResult { success: boolean; output: string; command: string; }

const emit = defineEmits<{ (e: "result", r: RepairResult): void }>();

const output = ref("");
const isLoading = ref(false);
const lastSuccess = ref<boolean | null>(null);
const killPid = ref("");
const svcName = ref("");

async function run(cmd: string, label?: string) {
  isLoading.value = true;
  try {
    const r = await invoke<RepairResult>("winpe_run_command", { command: cmd });
    output.value = r.output;
    lastSuccess.value = r.success;
    emit("result", { ...r, command: label || cmd });
  } catch (e) { output.value = String(e); lastSuccess.value = false; }
  finally { isLoading.value = false; }
}

const processCmds = [
  { label: "Lister tous les processus",  cmd: "tasklist /fo table" },
  { label: "Processus + PID + RAM",      cmd: "tasklist /fo csv | ConvertFrom-Csv | Select-Object 'Image Name','PID','Mem Usage' | Sort-Object 'Mem Usage' -Descending | Format-Table -AutoSize" },
  { label: "Processus en cours d'exec.", cmd: "Get-Process | Select-Object Name,Id,CPU,WorkingSet | Sort-Object CPU -Descending | Format-Table -AutoSize" },
  { label: "Services actifs",            cmd: "sc query state= all | findstr /i \"service_name state\"" },
  { label: "Services démarrés",          cmd: "Get-Service | Where-Object { $_.Status -eq 'Running' } | Select-Object Name,DisplayName | Format-Table -AutoSize" },
  { label: "Services stoppés",           cmd: "Get-Service | Where-Object { $_.Status -eq 'Stopped' } | Select-Object Name,DisplayName | Format-Table -AutoSize" },
  { label: "Infos système (systeminfo)", cmd: "systeminfo" },
  { label: "Uptime système",             cmd: "Get-WmiObject Win32_OperatingSystem | Select-Object LastBootUpTime,CSName,Caption | Format-List" },
  { label: "Utilisateurs connectés",     cmd: "query user" },
  { label: "Variables d'environnement",  cmd: "set" },
  { label: "Tâches planifiées",          cmd: "schtasks /query /fo list /v | findstr /i \"Task Name Status\"" },
  { label: "Programmes au démarrage",    cmd: "Get-CimInstance Win32_StartupCommand | Select-Object Name,Command,Location | Format-Table -AutoSize" },
  { label: "Pilotes actifs",             cmd: "driverquery /fo table" },
  { label: "Journal système (10 err.)",  cmd: "Get-EventLog -LogName System -Newest 10 -EntryType Error | Select-Object TimeGenerated,Source,Message | Format-List" },
  { label: "Journal applic. (10 err.)",  cmd: "Get-EventLog -LogName Application -Newest 10 -EntryType Error | Select-Object TimeGenerated,Source,Message | Format-List" },
  { label: "Dumps BSOD présents",        cmd: "Get-ChildItem C:\\Windows\\Minidump -ErrorAction SilentlyContinue | Select-Object Name,LastWriteTime | Format-Table" },
];

function isValidPid(v: string) { return /^\d{1,6}$/.test(v.trim()); }
function isValidSvcName(v: string) { return v.length > 0 && v.length <= 80 && /^[a-zA-Z0-9_\- ]+$/.test(v); }

async function killProcess() {
  if (!killPid.value) return;
  if (!isValidPid(String(killPid.value))) { output.value = "PID invalide — entrez un nombre."; lastSuccess.value = false; return; }
  // Même pattern que DashboardPage.vue/DiagTabProcesses.vue (confirm() déjà
  // présent sur leurs killProcess) — celui-ci n'en avait aucun.
  if (!(await confirm(`Terminer le processus PID ${killPid.value} ?`, { title: "Nitrite", kind: "warning" }))) return;
  await run(`taskkill /f /pid ${killPid.value}`, `Terminer PID ${killPid.value}`);
}

async function startService() {
  if (!svcName.value) return;
  if (!isValidSvcName(svcName.value)) { output.value = "Nom de service invalide."; lastSuccess.value = false; return; }
  await run(`net start "${svcName.value}"`, `Démarrer service ${svcName.value}`);
}

async function stopService() {
  if (!svcName.value) return;
  if (!isValidSvcName(svcName.value)) { output.value = "Nom de service invalide."; lastSuccess.value = false; return; }
  // Même pattern que DiagTabProcesses.vue::ctrlService (confirm() déjà présent
  // sur stop/restart, cycle 13) — celui-ci n'en avait aucun.
  if (!(await confirm(`Arrêter le service "${svcName.value}" ?`, { title: "Nitrite", kind: "warning" }))) return;
  await run(`net stop "${svcName.value}"`, `Arrêter service ${svcName.value}`);
}
</script>

<template>
  <div class="sys-tab">
    <!-- Commandes rapides -->
    <div class="section-card">
      <h3 class="section-title"><Terminal :size="15" /> Processus, Services &amp; Système</h3>
      <div class="cmd-grid">
        <button v-for="c in processCmds" :key="c.label" class="cmd-btn" :disabled="isLoading" @click="run(c.cmd, c.label)">
          {{ c.label }}
        </button>
      </div>
    </div>

    <!-- Kill process -->
    <div class="section-card">
      <h3 class="section-title"><Cpu :size="15" /> Terminer un Processus</h3>
      <div class="row-form">
        <input v-model="killPid" class="form-input" placeholder="PID du processus à terminer" type="number" />
        <NButton variant="danger" size="sm" :disabled="!killPid || isLoading" @click="killProcess">
          Terminer
        </NButton>
      </div>
    </div>

    <!-- Start/Stop service -->
    <div class="section-card">
      <h3 class="section-title"><Settings :size="15" /> Gérer un Service Windows</h3>
      <div class="row-form">
        <input v-model="svcName" class="form-input" placeholder="Nom du service (ex: wuauserv)" />
        <NButton variant="primary" size="sm" :disabled="!svcName || isLoading" @click="startService">Démarrer</NButton>
        <NButton variant="secondary" size="sm" :disabled="!svcName || isLoading" @click="stopService">Arrêter</NButton>
      </div>
    </div>

    <!-- Outils système graphiques -->
    <div class="section-card">
      <h3 class="section-title"><Layers :size="15" /> Outils Graphiques WinPE</h3>
      <div class="cmd-grid">
        <button class="cmd-btn" :disabled="isLoading" @click="run('regedit', 'Regedit')">Éditeur registre</button>
        <button class="cmd-btn" :disabled="isLoading" @click="run('explorer', 'Explorateur')">Explorateur fichiers</button>
        <button class="cmd-btn" :disabled="isLoading" @click="run('taskmgr', 'Gestionnaire tâches')">Gestionnaire tâches</button>
        <button class="cmd-btn" :disabled="isLoading" @click="run('msconfig', 'MSConfig')">MSConfig</button>
        <button class="cmd-btn" :disabled="isLoading" @click="run('eventvwr', 'Observateur events')">Observateur événements</button>
        <button class="cmd-btn" :disabled="isLoading" @click="run('compmgmt.msc', 'Gestion ordinateur')">Gestion ordinateur</button>
        <button class="cmd-btn" :disabled="isLoading" @click="run('diskmgmt.msc', 'Gestion des disques')">Gestion des disques</button>
        <button class="cmd-btn" :disabled="isLoading" @click="run('services.msc', 'Services')">Services</button>
        <button class="cmd-btn" :disabled="isLoading" @click="run('notepad', 'Notepad')">Bloc-notes</button>
        <button class="cmd-btn" :disabled="isLoading" @click="run('cmd', 'CMD')">Invite de commandes</button>
        <button class="cmd-btn" :disabled="isLoading" @click="run('powershell', 'PowerShell')">PowerShell</button>
        <button class="cmd-btn" :disabled="isLoading" @click="run('mstsc', 'Bureau à distance')">Bureau à distance</button>
      </div>
    </div>

    <!-- Output -->
    <div v-if="output" class="output-panel" :class="lastSuccess === false ? 'error' : lastSuccess ? 'success' : ''">
      <div class="output-header">
        <CheckCircle v-if="lastSuccess" :size="13" style="color:var(--success)" />
        <XCircle v-else-if="lastSuccess === false" :size="13" style="color:var(--danger)" />
        <span>Résultat</span>
      </div>
      <pre class="output-pre">{{ output }}</pre>
    </div>
  </div>
</template>

<style scoped>
.sys-tab { display: flex; flex-direction: column; gap: 14px; }
.section-card { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius-xl); padding: 14px; display: flex; flex-direction: column; gap: 12px; }
.section-title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 700; color: var(--text-primary); }
.cmd-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 6px; }
.cmd-btn { padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-md); font-size: 11px; color: var(--text-secondary); cursor: pointer; transition: all .15s; text-align: left; font-family: inherit; }
.cmd-btn:hover:not(:disabled) { border-color: var(--accent-primary); color: var(--accent-primary); }
.cmd-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.row-form { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.form-input { background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-md); padding: 6px 10px; font-size: 12px; color: var(--text-primary); flex: 1; min-width: 160px; font-family: monospace; }
.output-panel { background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-lg); padding: 12px; }
.output-panel.success { border-color: var(--success); }
.output-panel.error { border-color: var(--danger); }
.output-header { display: flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 600; color: var(--text-muted); margin-bottom: 8px; }
.output-pre { font-size: 11px; font-family: "JetBrains Mono", monospace; color: var(--text-secondary); white-space: pre-wrap; word-break: break-all; max-height: 320px; overflow-y: auto; }
</style>
