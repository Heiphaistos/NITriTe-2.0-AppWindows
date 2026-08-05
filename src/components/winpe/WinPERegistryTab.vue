<script setup lang="ts">
import { ref } from "vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import { invoke } from "@/utils/invoke";
import NButton from "@/components/ui/NButton.vue";
import { Database, Shield, RefreshCw, CheckCircle, XCircle } from "lucide-vue-next";

interface RepairResult { success: boolean; output: string; command: string; }

const emit = defineEmits<{ (e: "result", r: RepairResult): void }>();

const output = ref("");
const isLoading = ref(false);
const lastSuccess = ref<boolean | null>(null);

// Chargement ruche hors ligne
const hiveAlias = ref("HKLM\\OfflineSystem");
const hivePath = ref("C:\\Windows\\System32\\config\\SYSTEM");
const regExportKey = ref("HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion");
const exportPath = ref("C:\\export_reg.reg");
const regKey = ref("");
const regValue = ref("");
const regData = ref("");
const regType = ref("REG_SZ");

async function runMaybeConfirmed(cmd: string, label: string, dangerous?: boolean) {
  // "Restaurer hive depuis fichier" est le seul item de cette liste qui
  // REMPLACE toute une ruche registre en direct (SYSTEM) depuis un fichier
  // figé — les 15 autres sont des lectures (reg query) ou des éditions
  // ciblées (une seule valeur ajoutée/supprimée), sans commune mesure de
  // gravité. Un backup obsolète ou absent peut rendre Windows inutilisable.
  if (dangerous && !(await confirm(
    `⚠️ IRRÉVERSIBLE : ${label} — remplace la ruche registre en direct par le contenu du fichier de sauvegarde. Continuer ?`,
    { title: "Nitrite", kind: "warning" }
  ))) return;
  await run(cmd, label);
}

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

function isValidRegPath(v: string) { return v.length > 0 && !/[";`$<>|]/.test(v); }
function isValidFilePath(v: string) { return v.length > 0 && !/[";`$<>|]/.test(v); }
// Donnée de registre passée à reg.exe entre guillemets : un backslash final
// (fréquent sur les chemins, ex. C:\App\) échapperait le guillemet fermant via
// le parser d'arguments MSVCRT. On double la série finale de backslashes pour
// que reg.exe stocke la donnée intacte.
function escapeTrailingBackslashes(v: string): string {
  const m = v.match(/\\+$/);
  return m ? v + "\\".repeat(m[0].length) : v;
}

async function loadHive() {
  if (!hiveAlias.value || !hivePath.value) return;
  // hiveAlias and hivePath are <select> elements with hardcoded values
  await run(`reg load "${hiveAlias.value}" "${hivePath.value}"`, `Charger ruche ${hivePath.value}`);
}

async function unloadHive() {
  if (!hiveAlias.value) return;
  await run(`reg unload "${hiveAlias.value}"`, `Décharger ${hiveAlias.value}`);
}

async function doExportKey() {
  if (!regExportKey.value || !exportPath.value) return;
  if (!isValidRegPath(regExportKey.value)) { output.value = "Chemin de clé invalide."; lastSuccess.value = false; return; }
  if (!isValidFilePath(exportPath.value)) { output.value = "Chemin de fichier invalide."; lastSuccess.value = false; return; }
  await run(`reg export "${regExportKey.value}" "${exportPath.value}" /y`, `Export ${regExportKey.value}`);
}

async function queryKey() {
  if (!regKey.value) return;
  if (!isValidRegPath(regKey.value)) { output.value = "Chemin de clé invalide."; lastSuccess.value = false; return; }
  await run(`reg query "${regKey.value}"`, `Query ${regKey.value}`);
}

async function setValue() {
  if (!regKey.value || !regValue.value) return;
  if (!isValidRegPath(regKey.value)) { output.value = "Chemin de clé invalide."; lastSuccess.value = false; return; }
  if (!isValidRegPath(regValue.value)) { output.value = "Nom de valeur invalide."; lastSuccess.value = false; return; }
  if (regData.value && /[";`$<>|]/.test(regData.value)) { output.value = "Données invalides — caractères interdits."; lastSuccess.value = false; return; }
  await run(`reg add "${regKey.value}" /v "${regValue.value}" /t ${regType.value} /d "${escapeTrailingBackslashes(regData.value)}" /f`,
    `Set ${regKey.value}\\${regValue.value}`);
}

const repairKeys = [
  { label: "Infos Windows (clé NT)",     cmd: `reg query "HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion" /v ProductName` },
  { label: "Clé de produit (brute)",     cmd: `reg query "HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion" /v DigitalProductId` },
  { label: "Apps au démarrage (User)",   cmd: `reg query "HKCU\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run"` },
  { label: "Apps au démarrage (Local)",  cmd: `reg query "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run"` },
  { label: "Pilotes installés",          cmd: `reg query "HKLM\\SYSTEM\\CurrentControlSet\\Services" /k` },
  { label: "Politique mot de passe",     cmd: `reg query "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Lsa"` },
  { label: "Réactivation Wi-Fi (reg)",   cmd: `reg add "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e972-e325-11ce-bfc1-08002be10318}\\0001" /v DisableDeviceOnSuspend /t REG_DWORD /d 0 /f` },
  { label: "Désactiver UAC (offline)",   cmd: `reg add "HKLM\\OfflineSystem\\Microsoft\\Windows\\CurrentVersion\\Policies\\System" /v EnableLUA /t REG_DWORD /d 0 /f` },
  { label: "Activer RDP (offline)",      cmd: `reg add "HKLM\\OfflineSystem\\CurrentControlSet\\Control\\Terminal Server" /v fDenyTSConnections /t REG_DWORD /d 0 /f` },
  { label: "Fix LSA (SecureBoot)",       cmd: `reg add "HKLM\\OfflineSystem\\CurrentControlSet\\Control\\Lsa" /v LsaCfgFlags /t REG_DWORD /d 0 /f` },
  { label: "Supprimer clé startup",      cmd: `reg delete "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run" /v Malware /f` },
  { label: "Lister hives chargées",      cmd: `reg query HKLM` },
  { label: "Type de démarrage services", cmd: `reg query "HKLM\\SYSTEM\\CurrentControlSet\\Services" /s /v Start` },
  { label: "Sauv. SYSTEM hive",         cmd: `reg save HKLM\\SYSTEM C:\\backup_SYSTEM.hiv /y` },
  { label: "Sauv. SAM hive",            cmd: `reg save HKLM\\SAM C:\\backup_SAM.hiv /y` },
  { label: "Restaurer hive depuis fichier", cmd: `reg restore HKLM\\SYSTEM C:\\backup_SYSTEM.hiv`, dangerous: true },
];
</script>

<template>
  <div class="reg-tab">
    <!-- Ruches hors ligne -->
    <div class="section-card">
      <h3 class="section-title"><Database :size="15" /> Montage de Ruches Hors Ligne</h3>
      <p class="hint">Charge une ruche Windows hors ligne (SAM, SYSTEM, SOFTWARE) pour la modifier depuis WinPE.</p>
      <div class="form-row">
        <div class="form-group">
          <label class="form-label">Alias (clé de montage)</label>
          <select v-model="hiveAlias" class="form-select">
            <option value="HKLM\OfflineSAM">HKLM\OfflineSAM (comptes)</option>
            <option value="HKLM\OfflineSystem">HKLM\OfflineSystem (système)</option>
            <option value="HKLM\OfflineSoftware">HKLM\OfflineSoftware (logiciels)</option>
            <option value="HKLM\OfflineDefault">HKLM\OfflineDefault (profil défaut)</option>
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">Chemin de la ruche</label>
          <select v-model="hivePath" class="form-select">
            <option value="C:\Windows\System32\config\SAM">SAM (C:\Windows\System32\config\SAM)</option>
            <option value="C:\Windows\System32\config\SYSTEM">SYSTEM</option>
            <option value="C:\Windows\System32\config\SOFTWARE">SOFTWARE</option>
            <option value="C:\Windows\System32\config\DEFAULT">DEFAULT</option>
            <option value="C:\Users\Default\NTUSER.DAT">NTUSER.DAT (Default User)</option>
          </select>
        </div>
      </div>
      <div class="tool-actions">
        <NButton variant="primary" size="sm" :disabled="!hiveAlias || !hivePath || isLoading" @click="loadHive">
          <Database :size="13" /> Charger ruche
        </NButton>
        <NButton variant="ghost" size="sm" :disabled="!hiveAlias || isLoading" @click="unloadHive">
          Décharger ruche
        </NButton>
      </div>
    </div>

    <!-- Export registre -->
    <div class="section-card">
      <h3 class="section-title"><RefreshCw :size="15" /> Export / Sauvegarde Registre</h3>
      <div class="form-row">
        <div class="form-group" style="flex:2">
          <label class="form-label">Clé à exporter</label>
          <input v-model="regExportKey" class="form-input" placeholder="Ex: HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion" />
        </div>
        <div class="form-group" style="flex:1">
          <label class="form-label">Fichier de destination (.reg)</label>
          <input v-model="exportPath" class="form-input" placeholder="C:\export.reg" />
        </div>
      </div>
      <div class="tool-actions">
        <NButton variant="primary" size="sm" :disabled="!regExportKey || !exportPath || isLoading" @click="doExportKey">
          Exporter
        </NButton>
      </div>
    </div>

    <!-- Query / Modifier une valeur -->
    <div class="section-card">
      <h3 class="section-title"><Shield :size="15" /> Lire / Modifier une Valeur</h3>
      <div class="form-row">
        <div class="form-group" style="flex:2">
          <label class="form-label">Chemin de clé</label>
          <input v-model="regKey" class="form-input" placeholder="Ex: HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Run" />
        </div>
        <div class="form-group">
          <label class="form-label">Nom de valeur</label>
          <input v-model="regValue" class="form-input" placeholder="NomValeur" />
        </div>
        <div class="form-group">
          <label class="form-label">Type</label>
          <select v-model="regType" class="form-select">
            <option value="REG_SZ">REG_SZ</option>
            <option value="REG_DWORD">REG_DWORD</option>
            <option value="REG_EXPAND_SZ">REG_EXPAND_SZ</option>
            <option value="REG_BINARY">REG_BINARY</option>
            <option value="REG_MULTI_SZ">REG_MULTI_SZ</option>
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">Données</label>
          <input v-model="regData" class="form-input" placeholder="Valeur" />
        </div>
      </div>
      <div class="tool-actions">
        <NButton variant="ghost" size="sm" :disabled="!regKey || isLoading" @click="queryKey">Lire</NButton>
        <NButton variant="primary" size="sm" :disabled="!regKey || !regValue || isLoading" @click="setValue">Écrire valeur</NButton>
      </div>
    </div>

    <!-- Réparations courantes -->
    <div class="section-card">
      <h3 class="section-title"><Shield :size="15" /> Réparations Registre Courantes</h3>
      <div class="cmd-grid">
        <button v-for="c in repairKeys" :key="c.label" class="cmd-btn" :class="{ 'cmd-btn-danger': c.dangerous }" :disabled="isLoading" @click="runMaybeConfirmed(c.cmd, c.label, c.dangerous)">
          {{ c.label }}
        </button>
      </div>
    </div>

    <!-- Output -->
    <div v-if="output" class="output-panel" :class="lastSuccess === false ? 'error' : lastSuccess ? 'success' : ''">
      <div class="output-header">
        <CheckCircle v-if="lastSuccess" :size="13" style="color:var(--success)" />
        <XCircle v-else-if="lastSuccess === false" :size="13" style="color:var(--danger)" />
        <span>Résultat Registre</span>
      </div>
      <pre class="output-pre">{{ output }}</pre>
    </div>
  </div>
</template>

<style scoped>
.reg-tab { display: flex; flex-direction: column; gap: 14px; }
.section-card { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius-xl); padding: 14px; display: flex; flex-direction: column; gap: 12px; }
.section-title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 700; color: var(--text-primary); }
.hint { font-size: 11px; color: var(--text-muted); line-height: 1.5; }
.form-row { display: flex; flex-wrap: wrap; gap: 10px; }
.form-group { display: flex; flex-direction: column; gap: 4px; flex: 1; min-width: 180px; }
.form-label { font-size: 11px; font-weight: 600; color: var(--text-muted); }
.form-input { background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-md); padding: 6px 10px; font-size: 12px; color: var(--text-primary); width: 100%; font-family: monospace; }
.form-select { background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-md); padding: 6px 8px; font-size: 12px; color: var(--text-primary); width: 100%; }
.tool-actions { display: flex; gap: 8px; flex-wrap: wrap; }
.cmd-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 6px; }
.cmd-btn { padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-md); font-size: 11px; color: var(--text-secondary); cursor: pointer; transition: all .15s; text-align: left; font-family: inherit; }
.cmd-btn:hover:not(:disabled) { border-color: var(--accent-primary); color: var(--accent-primary); }
.cmd-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.cmd-btn-danger { border-color: rgba(239,68,68,.35); color: var(--danger); }
.cmd-btn-danger:hover:not(:disabled) { border-color: var(--danger); color: var(--danger); }
.output-panel { background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-lg); padding: 12px; }
.output-panel.success { border-color: var(--success); }
.output-panel.error { border-color: var(--danger); }
.output-header { display: flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 600; color: var(--text-muted); margin-bottom: 8px; }
.output-pre { font-size: 11px; font-family: "JetBrains Mono", monospace; color: var(--text-secondary); white-space: pre-wrap; word-break: break-all; max-height: 320px; overflow-y: auto; }
</style>
