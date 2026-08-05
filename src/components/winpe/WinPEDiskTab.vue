<script setup lang="ts">
import { ref } from "vue";
import { confirm, message } from "@tauri-apps/plugin-dialog";
import { invoke } from "@/utils/invoke";
import NButton from "@/components/ui/NButton.vue";
import NModal from "@/components/ui/NModal.vue";
import { HardDrive, Terminal, Activity, Trash2, CheckCircle, XCircle, AlertTriangle } from "lucide-vue-next";

interface RepairResult { success: boolean; output: string; command: string; }

const emit = defineEmits<{ (e: "result", r: RepairResult): void }>();

const output = ref("");
const isLoading = ref(false);
const lastSuccess = ref<boolean | null>(null);

const formatDrive = ref("");
const formatFs = ref("ntfs");
const formatLabel = ref("");
const cloneSrc = ref("");
const cloneDst = ref("");
const cleanDiskIndex = ref("");
const showCleanConfirm = ref(false);
const cleanConfirmText = ref("");

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

const diskpartCmds = [
  { label: "Lister tous les disques",     cmd: "echo list disk | diskpart" },
  { label: "Lister les volumes",           cmd: "echo list volume | diskpart" },
  { label: "Partitions disk 0",           cmd: "(echo select disk 0 & echo list partition) | diskpart" },
  { label: "Infos SMART (WMI)",            cmd: "Get-WmiObject -Namespace 'root/WMI' -Class MSStorageDriver_FailurePredictStatus | Select-Object InstanceName,PredictFailure,Reason | Format-Table -AutoSize" },
  { label: "État des disques physiques",   cmd: "Get-PhysicalDisk | Select-Object FriendlyName,MediaType,OperationalStatus,HealthStatus,Size | Format-Table -AutoSize" },
  { label: "Partitions Windows",           cmd: "Get-Partition | Select-Object DiskNumber,PartitionNumber,DriveLetter,Size,Type | Format-Table -AutoSize" },
  { label: "Volumes montés",              cmd: "Get-Volume | Select-Object DriveLetter,FileSystem,HealthStatus,SizeRemaining,Size | Format-Table -AutoSize" },
  { label: "Débit disque (3s)",           cmd: "winsat disk -ran -read -drive c" },
  { label: "Test vitesse lecture",         cmd: "winsat disk -read -drive c" },
  { label: "Infos S.M.A.R.T. détaillées", cmd: "Get-WmiObject -Class MSStorageDriver_FailurePredictData -Namespace 'root/WMI' | Select-Object InstanceName,VendorSpecific | Format-List" },
  { label: "Attribuer lettre D: au vol 1", cmd: "(echo select volume 1 & echo assign letter=D) | diskpart" },
  { label: "Enlever lettre D:",           cmd: "(echo select volume 1 & echo remove letter=D) | diskpart" },
  { label: "Étendre partition (disk 0 part 1)", cmd: "(echo select disk 0 & echo select partition 1 & echo extend) | diskpart" },
  { label: "Marquer partition active",    cmd: "(echo select disk 0 & echo select partition 1 & echo active) | diskpart" },
];

function isValidDriveLetter(v: string) { return /^[A-Za-z]:?$/.test(v.trim()); }
function isValidVolumeLabel(v: string) { return v.length <= 32 && /^[A-Za-z0-9_ -]*$/.test(v); }
function isValidPath(v: string) { return v.length > 0 && !/[";`$<>|]/.test(v); }
// Un backslash final devant le guillemet fermant l'échapperait (`"D:\"` → arg cassé).
// On le retire ; pour une racine (`D:`), `\.` référence le même répertoire sans ce piège.
function safeRoboPath(v: string): string {
  let p = v.trim().replace(/[\\/]+$/, "");
  if (/^[A-Za-z]:$/.test(p)) p += "\\.";
  return p;
}
function isValidDiskIndex(v: string) { return /^\d{1,2}$/.test(v.trim()); }

async function formatVolume() {
  if (!formatDrive.value) return;
  if (!isValidDriveLetter(formatDrive.value)) { output.value = "Lettre de lecteur invalide (ex: C ou C:)."; lastSuccess.value = false; return; }
  const letter = formatDrive.value.replace(":", "").trim().toUpperCase();
  const label = (formatLabel.value || "VOLUME").trim();
  if (!isValidVolumeLabel(label)) { output.value = "Étiquette invalide — alphanumériques, espaces, tirets, underscores uniquement."; lastSuccess.value = false; return; }
  // Formater un volume est tout aussi irréversible que cleanDisk() ci-dessous
  // (efface toutes les données du volume) mais n'avait aucune confirmation du tout.
  if (!(await confirm(`⚠️ IRRÉVERSIBLE : formater le volume ${letter}: (toutes les données seront effacées) ?`, { title: "Nitrite", kind: "warning" }))) return;
  // Label quoté : un espace non quoté serait parsé comme paramètre séparé par format.com
  const cmd = `format ${letter}: /fs:${formatFs.value} /v:"${label}" /q /y`;
  await run(cmd, `Formater ${letter}: en ${formatFs.value}`);
}

async function cloneDisk() {
  if (!cloneSrc.value || !cloneDst.value) return;
  if (!isValidPath(cloneSrc.value)) { output.value = "Chemin source invalide."; lastSuccess.value = false; return; }
  if (!isValidPath(cloneDst.value)) { output.value = "Chemin destination invalide."; lastSuccess.value = false; return; }
  const src = safeRoboPath(cloneSrc.value);
  const dst = safeRoboPath(cloneDst.value);
  const cmd = `robocopy "${src}" "${dst}" /E /COPYALL /R:0 /W:0 /LOG+:robocopy_log.txt`;
  await run(cmd, `Clone ${src} → ${dst}`);
}

async function requestCleanDisk() {
  const idx = cleanDiskIndex.value;
  if (!idx) return;
  if (!isValidDiskIndex(idx)) { await message("Index invalide — entrez un nombre entre 0 et 99.", { title: "Nitrite", kind: "error" }); return; }
  cleanConfirmText.value = "";
  showCleanConfirm.value = true;
}
async function cleanDisk() {
  // disk_wipe est le même appel que WinPERepairTab.vue::runWipe() (l'action la
  // plus destructrice de l'app) — même exigence de re-saisie exacte de l'index
  // pour rester cohérent entre les deux points d'entrée de cette commande.
  if (cleanConfirmText.value.trim() !== cleanDiskIndex.value.trim()) return;
  const diskIndex = parseInt(cleanDiskIndex.value, 10);
  showCleanConfirm.value = false;
  isLoading.value = true;
  try {
    const r = await invoke<RepairResult>("disk_wipe", { diskIndex, method: "quick" });
    output.value = r.output;
    lastSuccess.value = r.success;
    emit("result", { ...r, command: `Clean disk ${diskIndex}` });
  } catch (e) { output.value = String(e); lastSuccess.value = false; }
  finally { isLoading.value = false; }
}
</script>

<template>
  <div class="disk-tab">
    <!-- Commandes rapides -->
    <div class="section-card">
      <h3 class="section-title"><Terminal :size="15" /> Commandes Disques / Partitions</h3>
      <div class="cmd-grid">
        <button v-for="c in diskpartCmds" :key="c.label" class="cmd-btn" :disabled="isLoading" @click="run(c.cmd, c.label)">
          {{ c.label }}
        </button>
      </div>
    </div>

    <!-- Formatage -->
    <div class="section-card">
      <h3 class="section-title"><HardDrive :size="15" /> Formater un Volume</h3>
      <div class="form-row">
        <div class="form-group">
          <label class="form-label">Lettre de lecteur</label>
          <input v-model="formatDrive" class="form-input" placeholder="Ex: D:" />
        </div>
        <div class="form-group">
          <label class="form-label">Système de fichiers</label>
          <select v-model="formatFs" class="form-select">
            <option value="ntfs">NTFS</option>
            <option value="fat32">FAT32</option>
            <option value="exfat">exFAT</option>
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">Étiquette de volume</label>
          <input v-model="formatLabel" class="form-input" placeholder="Ex: BACKUP" />
        </div>
      </div>
      <div class="tool-actions">
        <NButton variant="danger" size="sm" :disabled="!formatDrive || isLoading" @click="formatVolume">
          Formater {{ formatDrive }}
        </NButton>
      </div>
    </div>

    <!-- Clone avec robocopy -->
    <div class="section-card">
      <h3 class="section-title"><Activity :size="15" /> Clone Robocopy (Fichiers)</h3>
      <p class="hint">Copie tous les fichiers d'un volume vers un autre avec Robocopy.</p>
      <div class="form-row">
        <div class="form-group">
          <label class="form-label">Source</label>
          <input v-model="cloneSrc" class="form-input" placeholder="Ex: C:\" />
        </div>
        <div class="form-group">
          <label class="form-label">Destination</label>
          <input v-model="cloneDst" class="form-input" placeholder="Ex: D:\" />
        </div>
      </div>
      <div class="tool-actions">
        <NButton variant="primary" size="sm" :disabled="!cloneSrc || !cloneDst || isLoading" @click="cloneDisk">
          Lancer la copie
        </NButton>
      </div>
    </div>

    <!-- Nettoyage disque dangereux -->
    <div class="section-card danger-card">
      <h3 class="section-title"><Trash2 :size="15" /> Clean Disk (Effacer partitions)</h3>
      <p class="hint" style="color:var(--danger)">⚠️ Supprime toutes les partitions du disque sélectionné — IRRÉVERSIBLE.</p>
      <div class="form-row">
        <div class="form-group">
          <label class="form-label">Index du disque</label>
          <input v-model="cleanDiskIndex" class="form-input" placeholder="Ex: 0" />
        </div>
      </div>
      <div class="tool-actions">
        <NButton variant="danger" size="sm" :disabled="!cleanDiskIndex || isLoading" @click="requestCleanDisk">Clean disk {{ cleanDiskIndex }}</NButton>
      </div>
    </div>

    <!-- Confirm clean disk -->
    <NModal :open="showCleanConfirm" title="Confirmer l'effacement" @close="showCleanConfirm = false">
      <div style="display:flex;flex-direction:column;align-items:center;gap:12px;padding:8px">
        <AlertTriangle :size="32" style="color:var(--danger)" />
        <p>Effacement <strong>IRRÉVERSIBLE</strong> de toutes les partitions du disque {{ cleanDiskIndex }}.</p>
        <div class="form-group" style="width:100%">
          <label class="form-label">Retapez <strong>{{ cleanDiskIndex }}</strong> pour confirmer :</label>
          <input v-model="cleanConfirmText" class="form-input" :placeholder="cleanDiskIndex" />
        </div>
      </div>
      <template #footer>
        <NButton variant="ghost" @click="showCleanConfirm = false">Annuler</NButton>
        <NButton variant="danger" :disabled="cleanConfirmText.trim() !== cleanDiskIndex" @click="cleanDisk">Confirmer</NButton>
      </template>
    </NModal>

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
.disk-tab { display: flex; flex-direction: column; gap: 14px; }
.section-card { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius-xl); padding: 14px; display: flex; flex-direction: column; gap: 12px; }
.danger-card { border-color: rgba(239,68,68,.35); }
.section-title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 700; color: var(--text-primary); }
.cmd-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 6px; }
.cmd-btn { padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-md); font-size: 11px; color: var(--text-secondary); cursor: pointer; transition: all .15s; text-align: left; font-family: inherit; }
.cmd-btn:hover:not(:disabled) { border-color: var(--accent-primary); color: var(--accent-primary); }
.cmd-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.form-row { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 10px; }
.form-group { display: flex; flex-direction: column; gap: 4px; }
.form-label { font-size: 11px; font-weight: 600; color: var(--text-muted); }
.form-input { background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-md); padding: 6px 10px; font-size: 12px; color: var(--text-primary); width: 100%; font-family: monospace; }
.form-select { background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-md); padding: 6px 8px; font-size: 12px; color: var(--text-primary); width: 100%; }
.tool-actions { display: flex; gap: 8px; flex-wrap: wrap; }
.hint { font-size: 11px; color: var(--text-muted); }
.output-panel { background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-lg); padding: 12px; }
.output-panel.success { border-color: var(--success); }
.output-panel.error { border-color: var(--danger); }
.output-header { display: flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 600; color: var(--text-muted); margin-bottom: 8px; }
.output-pre { font-size: 11px; font-family: "JetBrains Mono", monospace; color: var(--text-secondary); white-space: pre-wrap; word-break: break-all; max-height: 320px; overflow-y: auto; }
</style>
