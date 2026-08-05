<script setup lang="ts">
import { ref } from "vue";
import { invoke, invokeRaw } from "@/utils/invoke";
import NButton from "@/components/ui/NButton.vue";
import NModal from "@/components/ui/NModal.vue";
import { Wrench, Search, Shield, KeyRound, Trash2, AlertTriangle, Lock } from "lucide-vue-next";

interface RepairResult { success: boolean; output: string; command: string; }
interface PeDrive { letter: string; label: string; size_gb: number; free_gb: number; fs: string; is_system: boolean; }
interface OfflineUser { username: string; full_name: string; account_type: string; enabled: boolean; }
interface BitLockerStatus { drive: string; encrypted: boolean; locked: boolean; status_text: string; }

const props = defineProps<{
  drives: PeDrive[];
  sfcWindowsDir: string;
}>();
const emit = defineEmits<{ (e: "result", r: RepairResult): void }>();

const chkdskDrive = ref("");
const chkdskFix = ref(true);
const sfcDir = ref(props.sfcWindowsDir || "C:\\Windows");
const offlineUsers = ref<OfflineUser[]>([]);
const selectedUser = ref("");
const newPassword = ref("");
const bitlockerDrive = ref("");
const bitlockerKey = ref("");
const bitlockerStatus = ref<BitLockerStatus | null>(null);
const wipeDiskIndex = ref<number | null>(null);
const wipeMethod = ref("quick");
const showWipeConfirm = ref(false);
const wipeConfirmText = ref("");

const loading = ref({
  mbr: false, boot: false, bcd: false, scanos: false,
  chkdsk: false, sfc: false, dism: false,
  listUsers: false, resetPwd: false, wipe: false,
  clearPwd: false, enableAccount: false,
  bitlockerCheck: false, bitlockerUnlock: false,
});

async function runRepair(type: string) {
  const key = type as keyof typeof loading.value;
  if (loading.value[key]) return;
  loading.value[key] = true;
  try {
    let res: RepairResult;
    if (type === "mbr") res = await invoke<RepairResult>("repair_mbr");
    else if (type === "boot") res = await invoke<RepairResult>("repair_boot");
    else if (type === "bcd") res = await invoke<RepairResult>("rebuild_bcd");
    else res = await invoke<RepairResult>("scan_os_installations");
    emit("result", res);
  } catch (e) { emit("result", { success: false, output: String(e), command: type }); }
  finally { loading.value[key] = false; }
}

async function runChkdsk() {
  loading.value.chkdsk = true;
  try {
    const res = await invokeRaw<RepairResult>("run_chkdsk_pe", { drive: chkdskDrive.value, fix: chkdskFix.value });
    emit("result", res);
  } catch (e) { emit("result", { success: false, output: String(e), command: "chkdsk" }); }
  finally { loading.value.chkdsk = false; }
}

async function runSfc() {
  loading.value.sfc = true;
  try {
    const res = await invokeRaw<RepairResult>("run_sfc_offline", { windowsDir: sfcDir.value });
    emit("result", res);
  } catch (e) { emit("result", { success: false, output: String(e), command: "sfc" }); }
  finally { loading.value.sfc = false; }
}

async function runDism() {
  loading.value.dism = true;
  try {
    const res = await invokeRaw<RepairResult>("run_dism_offline_repair", { windowsDir: sfcDir.value });
    emit("result", res);
  } catch (e) { emit("result", { success: false, output: String(e), command: "dism" }); }
  finally { loading.value.dism = false; }
}

async function loadOfflineUsers() {
  loading.value.listUsers = true;
  try {
    offlineUsers.value = await invoke<OfflineUser[]>("list_offline_users", { windowsDir: sfcDir.value });
  } catch { /* silent */ }
  finally { loading.value.listUsers = false; }
}

async function runResetPassword() {
  loading.value.resetPwd = true;
  try {
    const res = await invoke<RepairResult>("reset_user_password", { windowsDir: sfcDir.value, username: selectedUser.value, newPassword: newPassword.value });
    emit("result", res);
  } catch (e) { emit("result", { success: false, output: String(e), command: "reset password" }); }
  finally { loading.value.resetPwd = false; }
}

async function runClearPassword() {
  loading.value.clearPwd = true;
  try {
    const res = await invoke<RepairResult>("clear_offline_password", { windowsDir: sfcDir.value, username: selectedUser.value });
    emit("result", res);
  } catch (e) { emit("result", { success: false, output: String(e), command: "clear password" }); }
  finally { loading.value.clearPwd = false; }
}

async function runEnableAccount() {
  loading.value.enableAccount = true;
  try {
    const res = await invoke<RepairResult>("enable_offline_account", { windowsDir: sfcDir.value, username: selectedUser.value });
    emit("result", res);
  } catch (e) { emit("result", { success: false, output: String(e), command: "enable account" }); }
  finally { loading.value.enableAccount = false; }
}

async function checkBitlocker() {
  loading.value.bitlockerCheck = true;
  try {
    bitlockerStatus.value = await invoke<BitLockerStatus>("get_bitlocker_status", { drive: bitlockerDrive.value });
  } catch { /* silent */ }
  finally { loading.value.bitlockerCheck = false; }
}

async function runUnlockBitlocker() {
  loading.value.bitlockerUnlock = true;
  try {
    const res = await invoke<RepairResult>("unlock_bitlocker", { drive: bitlockerDrive.value, recoveryKey: bitlockerKey.value });
    emit("result", res);
  } catch (e) { emit("result", { success: false, output: String(e), command: "unlock bitlocker" }); }
  finally { loading.value.bitlockerUnlock = false; }
}

function confirmWipe() { wipeConfirmText.value = ""; showWipeConfirm.value = true; }
async function runWipe() {
  // disk_wipe est l'action la plus destructrice de toute l'app (efface un
  // disque ENTIER, contrairement à format/supprimer/initialiser une simple
  // partition dans PartitionManagerTab.vue qui exigent déjà une saisie exacte)
  // — sans liste de disques réels affichée ici (juste un index numéroté au
  // clavier), une saisie exacte du numéro re-tapé est le minimum pour donner
  // à l'utilisateur une chance de remarquer une faute de frappe avant l'effacement.
  if (wipeConfirmText.value.trim() !== String(wipeDiskIndex.value)) return;
  showWipeConfirm.value = false;
  loading.value.wipe = true;
  try {
    const res = await invoke<RepairResult>("disk_wipe", { diskIndex: wipeDiskIndex.value, method: wipeMethod.value });
    emit("result", res);
  } catch (e) { emit("result", { success: false, output: String(e), command: "wipe disk" }); }
  finally { loading.value.wipe = false; }
}
</script>

<template>
  <div class="repair-tab">
    <!-- Répertoire Windows -->
    <div class="form-group">
      <label class="form-label">Répertoire Windows cible :</label>
      <input v-model="sfcDir" class="form-input" placeholder="Ex: C:\Windows" />
      <small class="hint">Chemin vers l'OS à réparer (utilisé par SFC, DISM, réinitialisation MDP)</small>
    </div>

    <div class="tools-grid">
      <!-- Réparation Boot -->
      <div class="tool-card">
        <div class="tool-header"><Wrench :size="18" /><h3>Réparation Démarrage</h3></div>
        <p class="tool-desc">Répare MBR, secteur boot et reconstruit le BCD.</p>
        <div class="tool-actions">
          <NButton size="sm" variant="secondary" :loading="loading.mbr" @click="runRepair('mbr')">Réparer MBR</NButton>
          <NButton size="sm" variant="secondary" :loading="loading.boot" @click="runRepair('boot')">Réparer Boot</NButton>
          <NButton size="sm" variant="primary" :loading="loading.bcd" @click="runRepair('bcd')">Reconstruire BCD</NButton>
          <NButton size="sm" variant="ghost" :loading="loading.scanos" @click="runRepair('scanos')">Scanner OS</NButton>
        </div>
      </div>

      <!-- ChkDsk -->
      <div class="tool-card">
        <div class="tool-header"><Search :size="18" /><h3>Vérification Disque</h3></div>
        <p class="tool-desc">Analyse et corrige les erreurs de système de fichiers.</p>
        <div class="tool-form">
          <select v-model="chkdskDrive" class="form-select">
            <option value="">Sélectionner un disque</option>
            <option v-for="d in drives" :key="d.letter" :value="d.letter">{{ d.letter }} ({{ d.label || d.fs }})</option>
          </select>
          <label class="checkbox-label"><input type="checkbox" v-model="chkdskFix" /><span>Corriger les erreurs (/f)</span></label>
        </div>
        <div class="tool-actions">
          <NButton size="sm" variant="primary" :loading="loading.chkdsk" :disabled="!chkdskDrive" @click="runChkdsk">Lancer ChkDsk</NButton>
        </div>
      </div>

      <!-- SFC Offline -->
      <div class="tool-card">
        <div class="tool-header"><Shield :size="18" /><h3>SFC / DISM Offline</h3></div>
        <p class="tool-desc">Répare les fichiers système corrompus depuis WinPE.</p>
        <div class="tool-actions">
          <NButton size="sm" variant="primary" :loading="loading.sfc" :disabled="!sfcDir" @click="runSfc">SFC /scannow</NButton>
          <NButton size="sm" variant="secondary" :loading="loading.dism" :disabled="!sfcDir" @click="runDism">DISM Repair</NButton>
        </div>
      </div>

      <!-- Reset mot de passe -->
      <div class="tool-card">
        <div class="tool-header"><KeyRound :size="18" /><h3>Réinitialisation Mot de Passe</h3></div>
        <p class="tool-desc">Réinitialise le MDP d'un compte Windows hors ligne.</p>
        <div class="tool-form">
          <NButton size="sm" variant="ghost" :loading="loading.listUsers" @click="loadOfflineUsers">Charger les utilisateurs</NButton>
          <select v-if="offlineUsers.length" v-model="selectedUser" class="form-select">
            <option value="">Sélectionner un utilisateur</option>
            <option v-for="u in offlineUsers" :key="u.username" :value="u.username">
              {{ u.username }}{{ !u.enabled ? ' (désactivé)' : '' }}
            </option>
          </select>
          <input v-if="selectedUser" v-model="newPassword" type="password" placeholder="Nouveau mot de passe (vide = supprimer)" class="form-input" />
        </div>
        <div class="tool-actions">
          <NButton size="sm" variant="danger" :loading="loading.resetPwd" :disabled="!selectedUser || !newPassword || !sfcDir" @click="runResetPassword">Définir MDP</NButton>
          <NButton size="sm" variant="secondary" :loading="loading.clearPwd" :disabled="!selectedUser || !sfcDir" @click="runClearPassword">Supprimer MDP</NButton>
          <NButton size="sm" variant="ghost" :loading="loading.enableAccount" :disabled="!selectedUser || !sfcDir" @click="runEnableAccount">Activer compte</NButton>
        </div>
        <p class="tool-desc" style="margin-top:6px;font-size:0.72rem">💡 <strong>Comptes Microsoft / PIN</strong> : utilisez Jayro's Lockpick (onglet Outils) pour les cas avancés.</p>
      </div>

      <!-- BitLocker -->
      <div class="tool-card">
        <div class="tool-header"><Lock :size="18" /><h3>Déverrouillage BitLocker</h3></div>
        <p class="tool-desc">Vérifiez et déverrouillez un volume chiffré BitLocker.</p>
        <div class="tool-form">
          <div style="display:flex;gap:6px;align-items:center">
            <select v-model="bitlockerDrive" class="form-select" style="flex:1">
              <option value="">Sélectionner un disque</option>
              <option v-for="d in drives" :key="d.letter" :value="d.letter">{{ d.letter }}</option>
            </select>
            <NButton size="sm" variant="ghost" :loading="loading.bitlockerCheck" :disabled="!bitlockerDrive" @click="checkBitlocker">Vérifier</NButton>
          </div>
          <div v-if="bitlockerStatus" class="bl-status" :class="bitlockerStatus.locked ? 'locked' : 'ok'">
            {{ bitlockerStatus.status_text }}
          </div>
          <template v-if="bitlockerStatus?.locked">
            <input v-model="bitlockerKey" class="form-input" placeholder="Clé de récupération 48 chiffres" />
          </template>
        </div>
        <div class="tool-actions">
          <NButton size="sm" variant="primary" :loading="loading.bitlockerUnlock" :disabled="!bitlockerDrive || !bitlockerKey || !bitlockerStatus?.locked" @click="runUnlockBitlocker">Déverrouiller</NButton>
        </div>
      </div>

      <!-- Effacement disque -->
      <div class="tool-card danger-card">
        <div class="tool-header"><Trash2 :size="18" /><h3>Effacement Disque</h3></div>
        <p class="tool-desc">Efface complètement un disque (IRRÉVERSIBLE).</p>
        <div class="tool-form">
          <input v-model.number="wipeDiskIndex" type="number" min="0" placeholder="Index disque (0, 1, 2...)" class="form-input" />
          <select v-model="wipeMethod" class="form-select">
            <option value="quick">Rapide (clean)</option>
            <option value="secure">Sécurisé (zeros - lent)</option>
          </select>
        </div>
        <div class="tool-actions">
          <NButton size="sm" variant="danger" :loading="loading.wipe" :disabled="wipeDiskIndex === null" @click="confirmWipe">
            Effacer le disque {{ wipeDiskIndex }}
          </NButton>
        </div>
      </div>
    </div>

    <!-- Confirm wipe -->
    <NModal :open="showWipeConfirm" title="Confirmer l'effacement" @close="showWipeConfirm = false">
      <div style="display:flex;flex-direction:column;align-items:center;gap:12px;padding:8px">
        <AlertTriangle :size="32" style="color:var(--danger)" />
        <p>Effacement <strong>IRRÉVERSIBLE</strong> du Disque {{ wipeDiskIndex }} en mode <strong>{{ wipeMethod === 'secure' ? 'Sécurisé' : 'Rapide' }}</strong>.</p>
        <div class="form-group" style="width:100%">
          <label class="form-label">Retapez <strong>{{ wipeDiskIndex }}</strong> pour confirmer :</label>
          <input v-model="wipeConfirmText" class="form-input" :placeholder="String(wipeDiskIndex)" />
        </div>
      </div>
      <template #footer>
        <NButton variant="ghost" @click="showWipeConfirm = false">Annuler</NButton>
        <NButton variant="danger" :disabled="wipeConfirmText.trim() !== String(wipeDiskIndex)" @click="runWipe">Confirmer</NButton>
      </template>
    </NModal>
  </div>
</template>

<style scoped>
.repair-tab { display: flex; flex-direction: column; gap: 16px; }
.form-group { display: flex; flex-direction: column; gap: 6px; }
.form-label { font-size: 12px; font-weight: 600; color: var(--text-secondary); }
.tools-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 14px; }
.tool-card { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius-xl); padding: 14px; display: flex; flex-direction: column; gap: 10px; }
.danger-card { border-color: rgba(239,68,68,.35); }
.tool-header { display: flex; align-items: center; gap: 10px; color: var(--accent-primary); }
.tool-header h3 { font-size: 13px; font-weight: 700; color: var(--text-primary); }
.tool-desc { font-size: 12px; color: var(--text-muted); line-height: 1.5; }
.tool-form { display: flex; flex-direction: column; gap: 8px; }
.tool-actions { display: flex; flex-wrap: wrap; gap: 6px; }
.form-select { background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-md); padding: 6px 8px; font-size: 12px; color: var(--text-primary); width: 100%; }
.form-input { background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-md); padding: 6px 10px; font-size: 12px; color: var(--text-primary); width: 100%; font-family: monospace; }
.checkbox-label { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--text-secondary); cursor: pointer; }
.bl-status { padding: 6px 10px; border-radius: var(--radius-md); font-size: 12px; font-weight: 600; }
.bl-status.locked { background: rgba(239,68,68,.15); color: var(--danger); border: 1px solid var(--danger); }
.bl-status.ok { background: rgba(34,197,94,.15); color: var(--success); border: 1px solid var(--success); }
.hint { font-size: 11px; color: var(--text-muted); }
</style>
