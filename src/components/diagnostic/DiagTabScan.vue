<script setup lang="ts">
import { computed, ref } from "vue";
import {
  CheckCircle, AlertTriangle, RefreshCw, ScanLine, Lock, LockOpen,
  Key, HardDrive, Battery,
  FileDown, FileText, FileCode, Wrench, Cpu, MemoryStick,
} from "lucide-vue-next";
import NProgress from "@/components/ui/NProgress.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import { invoke } from "@/utils/invoke";
import { useNotificationStore } from "@/stores/notifications";
import { useScanExport } from "@/composables/useScanExport";
import ScanHealthScore from "@/components/diagnostic/ScanHealthScore.vue";
import ScanProgressBar from "@/components/diagnostic/ScanProgressBar.vue";
import ScanChoiceCards from "@/components/diagnostic/ScanChoiceCards.vue";
import ScanSectionHardware from "@/components/diagnostic/ScanSectionHardware.vue";

interface BatteryInfo {
  name: string; status: string; estimated_charge_remaining: number; estimated_run_time: string;
  design_capacity: number; full_charge_capacity: number; battery_health_percent: number;
  chemistry: string; cycle_count: number;
}
interface WingetUpgrade { name: string; id: string; current_version: string; available_version: string; }

const props = defineProps<{
  scanning: boolean;
  scanProgress: number;
  scanStep: string;
  scanResult: any; // ScanResult — any car défini dans DiagnosticPage
  scanProblems: string[];
  batteries: BatteryInfo[];
  onRunScan: () => void;
  onLaunchTotal: (formats: Set<string>) => void;
}>();


// ── Helpers statut DISM / SFC ────────────────────────────────────────────────
// Approach inversée : ne retourne false QUE sur les statuts connus comme mauvais.
// Ceci évite de fausses alarmes pour "OK", "DISM non disponible", "Vérification complète", etc.
function isDismHealthy(status: string | undefined | null): boolean {
  if (!status) return true;
  const s = status.toLowerCase();
  return !['avertissement', 'erreur', 'corrupt'].some(w => s.includes(w));
}
function isSfcIntegre(status: string | undefined | null): boolean {
  if (!status) return true;
  const s = status.toLowerCase();
  return !['corrompus', 'corrupt', 'fichiers corr'].some(w => s.includes(w))
      && !(s.includes('avertissement') && !s.includes('non vérifié'));
}

// ── Solutions recommandées ────────────────────────────────────────────────────
interface Solution { problem: string; action: string; repairKey?: string; severity: "critical" | "warning" | "info" }

const scanSolutions = computed<Solution[]>(() => {
  if (!props.scanResult) return [];
  const sr = props.scanResult;
  const sol: Solution[] = [];
  if (!sr.firewall_enabled) sol.push({ problem: "Pare-feu désactivé", action: "Activer le pare-feu (tous profils)", repairKey: "enable_firewall", severity: "critical" });
  if (!sr.defender_enabled) sol.push({ problem: "Defender (temps réel) inactif", action: "Réactiver via Paramètres → Sécurité Windows", severity: "critical" });
  if (sr.smbv1_enabled) sol.push({ problem: "SMBv1 activé (vulnérable)", action: "Désactiver SMBv1 via PowerShell", repairKey: "disable_smb1", severity: "critical" });
  if (sr.wmi_subscriptions > 0) sol.push({ problem: `${sr.wmi_subscriptions} abonnement(s) WMI suspect(s)`, action: "Nettoyer les abonnements WMI (indicateur malware)", repairKey: "wmi_cleanup", severity: "critical" });
  if (sr.pending_reboot) sol.push({ problem: "Redémarrage requis", action: "Redémarrer le PC pour appliquer les mises à jour", severity: "warning" });
  if (sr.last_update_days > 60) sol.push({ problem: `Dernière MAJ il y a ${sr.last_update_days} jours`, action: "Lancer Windows Update", repairKey: "wu_usoclient", severity: "warning" });
  if (sr.defender_definition_age_days > 7) sol.push({ problem: `Définitions Defender datant de ${sr.defender_definition_age_days} jours`, action: "Mettre à jour les signatures Defender", repairKey: "defender_update", severity: "warning" });
  if (sr.dism_status && !isDismHealthy(sr.dism_status)) sol.push({ problem: "Composant Windows corrompu (DISM)", action: "Lancer DISM /RestoreHealth", repairKey: "dism_restore", severity: "critical" });
  if (sr.sfc_status && sr.sfc_status.toLowerCase().includes("corrupt")) sol.push({ problem: "Fichiers système corrompus (SFC)", action: "Exécuter SFC /scannow", repairKey: "sfc", severity: "critical" });
  if (sr.temp_folder_size_mb > 2048) sol.push({ problem: `Fichiers temp volumineux (${(sr.temp_folder_size_mb/1024).toFixed(1)} GB)`, action: "Nettoyer %TEMP%", repairKey: "temp_cleanup", severity: "warning" });
  if (sr.disk_usage?.some((d: any) => d.used_percent > 90)) sol.push({ problem: "Disque(s) à plus de 90% de capacité", action: "Nettoyer les fichiers temporaires et le cache", repairKey: "diskcleanup", severity: "critical" });
  if (sr.disk_usage?.some((d: any) => d.used_percent > 80)) sol.push({ problem: "Disque(s) à plus de 80% de capacité", action: "Activer Storage Sense", repairKey: "storage_sense", severity: "warning" });
  if (!sr.tpm_present) sol.push({ problem: "TPM absent ou désactivé", action: "Activer le TPM dans le BIOS (requis pour Windows 11)", severity: "warning" });
  if (!sr.secure_boot) sol.push({ problem: "Secure Boot désactivé", action: "Activer Secure Boot dans le BIOS/UEFI", severity: "warning" });
  if (sr.rdp_enabled) sol.push({ problem: "Bureau à distance (RDP) activé", action: "Désactiver si non nécessaire (Paramètres → Système → Bureau à distance)", severity: "info" });
  if (sr.guest_enabled) sol.push({ problem: "Compte Invité activé", action: "Désactiver le compte Invité", repairKey: "disable_guest", severity: "warning" });
  if (sr.suspicious_processes?.length > 0) sol.push({ problem: `${sr.suspicious_processes.length} processus hors chemins sécurisés`, action: "Vérifier manuellement ces processus avec Process Explorer", severity: "warning" });
  if (!sr.network_ok) sol.push({ problem: "Pas de connectivité Internet", action: "Réinitialiser la pile réseau (Winsock + IP)", repairKey: "net_reset_all", severity: "critical" });
  if (sr.winget_upgradable?.length > 5) sol.push({ problem: `${sr.winget_upgradable.length} logiciels obsolètes`, action: "Mettre à jour via WinGet (onglet Mises à jour)", severity: "info" });
  return sol;
});

const healthScore = computed(() => {
  if (!props.scanResult) return null;
  const sr = props.scanResult;
  let score = 100;
  if (!sr.firewall_enabled) score -= 20;
  if (!sr.defender_enabled) score -= 20;
  if (sr.smbv1_enabled) score -= 15;
  if (sr.wmi_subscriptions > 0) score -= 15;
  if (sr.pending_reboot) score -= 5;
  if (sr.last_update_days > 60) score -= 10;
  if (sr.last_update_days > 30) score -= 5;
  if (sr.defender_definition_age_days > 7) score -= 5;
  if (sr.dism_status && !isDismHealthy(sr.dism_status)) score -= 15;
  if (sr.sfc_status && sr.sfc_status.toLowerCase().includes('corrupt')) score -= 15;
  if (sr.temp_folder_size_mb > 2048) score -= 3;
  if (sr.disk_usage?.some((d: any) => d.used_percent > 90)) score -= 10;
  if (!sr.tpm_present) score -= 5;
  if (!sr.secure_boot) score -= 5;
  if (sr.rdp_enabled) score -= 3;
  if (sr.guest_enabled) score -= 5;
  if (sr.suspicious_processes?.length > 0) score -= 10;
  if (!sr.network_ok) score -= 10;
  return Math.max(0, Math.min(100, score));
});

const scoreVariant = computed(() => {
  if (healthScore.value === null) return 'neutral';
  if (healthScore.value >= 80) return 'success';
  if (healthScore.value >= 60) return 'warning';
  return 'danger';
});

const scoreLabel = computed(() => {
  if (healthScore.value === null) return '';
  if (healthScore.value >= 90) return 'Excellent';
  if (healthScore.value >= 80) return 'Bon';
  if (healthScore.value >= 60) return 'Moyen';
  if (healthScore.value >= 40) return 'Faible';
  return 'Critique';
});

const applyingFix = ref<string | null>(null);
const fixResults = ref<Record<string, boolean>>({});

async function applyFix(repairKey: string) {
  applyingFix.value = repairKey;
  try {
    await invoke('run_repair_command', { repairType: repairKey });
    fixResults.value[repairKey] = true;
  } catch (err) {
    fixResults.value[repairKey] = false;
    useNotificationStore().error("Réparation échouée", `${repairKey}: ${String(err)}`);
  }
  applyingFix.value = null;
}

const { exportScanTxt, exportScanHtml, exportScanMd, exportScanJson } = useScanExport();

// NOTE: exportScanTxt/Html/Md/Json are now in useScanExport composable
// Template wrappers that pass props to the composable functions:
function doExportTxt()  { exportScanTxt(props.scanResult,  props.scanProblems, props.batteries, scanSolutions.value); }
function doExportHtml() { exportScanHtml(props.scanResult, props.scanProblems, props.batteries, scanSolutions.value); }
function doExportMd()   { exportScanMd(props.scanResult,   props.scanProblems, props.batteries, scanSolutions.value); }
function doExportJson() { exportScanJson(props.scanResult, props.scanProblems, props.batteries, scanSolutions.value); }

// keep fullRegPath locally — used in template (autorun section)
function fullRegPath(location: string, name?: string): string {
  let p = location
    .replace(/^HKCU(\\|$)/, "HKEY_CURRENT_USER$1")
    .replace(/^HKLM(\\|$)/, "HKEY_LOCAL_MACHINE$1")
    .replace(/^HKCR(\\|$)/, "HKEY_CLASSES_ROOT$1")
    .replace(/^HKU(\\|$)/, "HKEY_USERS$1");
  if (name) p = p + (p.endsWith("\\") ? "" : "\\") + name;
  return p;
}


async function copyRegPath(path: string) {
  try {
    await navigator.clipboard.writeText(path);
    useNotificationStore().success("Chemin copié", path.slice(0, 60) + (path.length > 60 ? "…" : ""));
  } catch { useNotificationStore().error("Copie échouée"); }
}

async function openRegedit(location: string) {
  try {
    await invoke("open_in_regedit", { keyPath: location });
  } catch (e: any) { useNotificationStore().error("Impossible d'ouvrir Regedit", String(e)); }
}

function actTypeVariant(t: string): 'success'|'neutral'|'warning'|'danger' {
  if (t.includes('MAS') || t.includes('tiers') || t.includes('Non activé')) return 'danger';
  if (t.includes('Retail')) return 'success';
  if (t.includes('OEM')) return 'neutral';
  if (t.includes('KMS') || t.includes('Volume')) return 'warning';
  return 'neutral';
}

async function runRepairCommand(type: "sfc" | "dism") {
  const notif = useNotificationStore();
  try {
    if (type === "sfc") {
      await invoke("run_system_command", { cmd: "cmd", args: ["/c", "start", "cmd", "/k", "sfc /scannow"] });
      notif.success("SFC lancé", "Une fenêtre cmd s'est ouverte avec sfc /scannow");
    } else {
      await invoke("run_system_command", { cmd: "cmd", args: ["/c", "start", "cmd", "/k", "DISM /Online /Cleanup-Image /RestoreHealth"] });
      notif.success("DISM RestoreHealth lancé", "Réparation des composants Windows en cours");
    }
  } catch (e: any) {
    notif.error("Erreur lancement commande", String(e));
  }
}

</script>

<template>
  <div class="diag-tab-content">
    <DiagBanner :icon="ScanLine" title="Scan Complet du Système" desc="Analyse approfondie : sécurité, performances, licences et intégrité" color="emerald" />

    <ScanHealthScore v-if="healthScore !== null"
      :health-score="healthScore"
      :score-variant="scoreVariant"
      :score-label="scoreLabel"
      :solutions-count="scanSolutions.length"
      :critical-count="scanSolutions.filter(s => s.severity === 'critical').length"
      @export="doExportTxt"
    />
    <ScanProgressBar v-if="scanning" :step="scanStep" :progress="scanProgress" />
    <ScanChoiceCards v-else-if="!scanResult" @launch-total="onLaunchTotal" />

    <!-- Résultats -->
    <div v-else style="display:flex;flex-direction:column;gap:14px">

      <!-- ===== BILAN ===== -->
      <div class="diag-section" :style="{borderLeft: `3px solid ${scanProblems.length ? 'var(--warning)' : 'var(--success)'}`}">
        <p class="diag-section-label" style="margin:0 0 8px 0">
          {{ scanProblems.length ? `⚠ ${scanProblems.length} problème(s) détecté(s)` : '✓ Aucun problème critique' }}
        </p>
        <div v-if="!scanProblems.length" style="color:var(--success);font-size:13px">Système en bonne santé</div>
        <div v-for="(p, i) in scanProblems" :key="i"
          style="display:flex;align-items:center;gap:8px;padding:5px 0;border-bottom:1px solid var(--border);font-size:13px">
          <AlertTriangle :size="13" class="ic-warn" />{{ p }}
        </div>
      </div>

      <!-- ===== SOLUTIONS RECOMMANDÉES ===== -->
      <div v-if="scanSolutions.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 10px 0">
          <Wrench :size="13" style="display:inline;margin-right:6px;color:var(--accent-primary)" />
          Solutions Recommandées ({{ scanSolutions.length }})
        </p>
        <div v-for="(s, i) in scanSolutions" :key="i" class="solution-row"
          :class="`solution-${s.severity}`">
          <div class="solution-sev">
            <span v-if="s.severity === 'critical'" style="color:var(--danger)">🔴</span>
            <span v-else-if="s.severity === 'warning'" style="color:var(--warning)">🟡</span>
            <span v-else style="color:var(--info)">🔵</span>
          </div>
          <div style="flex:1;min-width:0">
            <div style="font-size:13px;font-weight:500;color:var(--text-primary)">{{ s.problem }}</div>
            <div style="font-size:12px;color:var(--text-secondary);margin-top:2px">→ {{ s.action }}</div>
            <NButton v-if="s.repairKey && !fixResults[s.repairKey]"
              variant="primary" size="sm"
              :disabled="applyingFix !== null"
              @click="applyFix(s.repairKey)"
              style="margin-top:6px">
              <NSpinner v-if="applyingFix === s.repairKey" :size="12" />
              {{ applyingFix === s.repairKey ? 'Application...' : 'Appliquer la correction' }}
            </NButton>
            <span v-else-if="s.repairKey && fixResults[s.repairKey]" style="color:var(--success);font-size:12px;margin-top:4px;display:block">✓ Correction appliquée</span>
          </div>
          <NBadge v-if="s.severity === 'critical'" variant="danger" style="flex-shrink:0">Critique</NBadge>
          <NBadge v-else-if="s.severity === 'warning'" variant="warning" style="flex-shrink:0">Attention</NBadge>
          <NBadge v-else variant="info" style="flex-shrink:0">Info</NBadge>
        </div>
      </div>

      <!-- ===== SYSTÈME GÉNÉRAL ===== -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Résumé Système</p>
        <div class="info-grid">
          <div class="info-row"><span>Windows</span><span>{{ scanResult.windows_version }}</span></div>
          <div class="info-row"><span>Activation</span>
            <NBadge :variant="scanResult.windows_activation === 'Activé' || scanResult.windows_activation === 'Licencié' ? 'success' : 'danger'">
              {{ scanResult.windows_activation || "Inconnu" }}
            </NBadge>
          </div>
          <div class="info-row"><span>Uptime</span>
            <span>{{ scanResult.uptime_hours >= 24 ? `${(scanResult.uptime_hours/24).toFixed(1)} j` : `${scanResult.uptime_hours.toFixed(1)} h` }}</span>
          </div>
          <div class="info-row"><span>Redémarrage requis</span>
            <NBadge :variant="scanResult.pending_reboot ? 'warning' : 'success'">{{ scanResult.pending_reboot ? "Oui" : "Non" }}</NBadge>
          </div>
          <div class="info-row"><span>Démarrage auto</span><span>{{ scanResult.startup_count }} programmes</span></div>
          <div class="info-row"><span>Fichiers %TEMP%</span>
            <NBadge :variant="scanResult.temp_folder_size_mb > 2048 ? 'danger' : scanResult.temp_folder_size_mb > 512 ? 'warning' : 'success'">
              {{ scanResult.temp_folder_size_mb >= 1024 ? (scanResult.temp_folder_size_mb/1024).toFixed(1)+' GB' : scanResult.temp_folder_size_mb.toFixed(0)+' MB' }}
            </NBadge>
          </div>
          <div class="info-row"><span>Logiciels installés</span><span>{{ scanResult.installed_software_count }}</span></div>
          <div class="info-row"><span>Services actifs / arrêtés</span>
            <span><span style="color:var(--success)">{{ scanResult.services_running }}</span> / <span class="muted">{{ scanResult.services_stopped }}</span></span>
          </div>
          <div class="info-row"><span>Mém. virtuelle</span>
            <span>{{ scanResult.virtual_memory_available_mb > 0 ? (scanResult.virtual_memory_available_mb/1024).toFixed(1)+'GB libres / '+(scanResult.virtual_memory_total_mb/1024).toFixed(1)+'GB' : 'N/A' }}</span>
          </div>
          <div class="info-row" v-if="scanResult.plan_alimentation || scanResult.power_plan">
            <span>Plan d'alimentation</span><span>{{ scanResult.power_plan }}</span>
          </div>
        </div>
      </div>

      <!-- ===== IDENTITÉ SYSTÈME & BIOS ===== -->
      <div v-if="scanResult.system_manufacturer || scanResult.bios_manufacturer" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Identité Système & BIOS</p>
        <div class="info-grid">
          <div v-if="scanResult.system_manufacturer" class="info-row"><span>Fabricant</span><span>{{ scanResult.system_manufacturer }}</span></div>
          <div v-if="scanResult.system_model" class="info-row"><span>Modèle</span><span>{{ scanResult.system_model }}</span></div>
          <div v-if="scanResult.system_serial && scanResult.system_serial !== 'N/A'" class="info-row">
            <span>N° Série carte mère</span><code style="font-size:11px">{{ scanResult.system_serial }}</code>
          </div>
          <div v-if="scanResult.bios_manufacturer" class="info-row"><span>BIOS Fabricant</span><span>{{ scanResult.bios_manufacturer }}</span></div>
          <div v-if="scanResult.bios_version" class="info-row"><span>BIOS Version</span><code style="font-size:11px">{{ scanResult.bios_version }}</code></div>
          <div v-if="scanResult.bios_date" class="info-row"><span>BIOS Date</span><span>{{ scanResult.bios_date }}</span></div>
          <div v-if="scanResult.license_type" class="info-row"><span>Type de licence Windows</span>
            <NBadge :variant="scanResult.license_type === 'OEM' ? 'info' : scanResult.license_type === 'Retail' ? 'success' : 'neutral'">
              {{ scanResult.license_type }}
            </NBadge>
          </div>
          <div v-if="scanResult.activation_type" class="info-row"><span>Mode d'activation Windows</span>
            <NBadge :variant="actTypeVariant(scanResult.activation_type)">{{ scanResult.activation_type }}</NBadge>
          </div>
          <div v-if="scanResult.office_activation_type" class="info-row"><span>Mode d'activation Office</span>
            <NBadge :variant="actTypeVariant(scanResult.office_activation_type)">{{ scanResult.office_activation_type }}</NBadge>
          </div>
        </div>
      </div>

      <!-- ===== SÉCURITÉ AVANCÉE ===== -->
      <div v-if="scanResult.tpm_present !== undefined" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Sécurité Avancée</p>
        <div class="info-grid">
          <div class="info-row"><span>TPM</span>
            <NBadge :variant="scanResult.tpm_present ? 'success' : 'warning'">
              {{ scanResult.tpm_present ? (scanResult.tpm_enabled ? 'Présent & Activé' : 'Présent (désactivé)') : 'Absent' }}
            </NBadge>
          </div>
          <div v-if="scanResult.tpm_present && scanResult.tpm_version" class="info-row">
            <span>Version TPM</span><span>{{ scanResult.tpm_version }}</span>
          </div>
          <div class="info-row"><span>Secure Boot</span>
            <NBadge :variant="scanResult.secure_boot ? 'success' : 'warning'">{{ scanResult.secure_boot ? 'Activé' : 'Désactivé' }}</NBadge>
          </div>
          <div class="info-row"><span>Niveau UAC</span><span>{{ scanResult.uac_level || 'Inconnu' }}</span></div>
          <div class="info-row"><span>Bureau à distance (RDP)</span>
            <NBadge :variant="scanResult.rdp_enabled ? 'warning' : 'success'">{{ scanResult.rdp_enabled ? 'Activé' : 'Désactivé' }}</NBadge>
          </div>
          <div class="info-row"><span>SMBv1 (obsolète)</span>
            <div style="display:flex;align-items:center;gap:6px">
              <NBadge :variant="scanResult.smbv1_enabled ? 'danger' : 'success'">{{ scanResult.smbv1_enabled ? 'Activé ⚠' : 'Désactivé' }}</NBadge>
              <NButton v-if="scanResult.smbv1_enabled && !fixResults['disable_smb1']" variant="danger" size="sm"
                style="font-size:10px;padding:2px 6px" :loading="applyingFix === 'disable_smb1'" @click="applyFix('disable_smb1')">Désactiver</NButton>
              <span v-if="fixResults['disable_smb1']" style="font-size:11px;color:var(--success)">✓</span>
            </div>
          </div>
          <div class="info-row"><span>Abonnements WMI (indicateur malware)</span>
            <NBadge :variant="scanResult.wmi_subscriptions > 0 ? 'danger' : 'success'">{{ scanResult.wmi_subscriptions ?? 0 }}</NBadge>
          </div>
          <div class="info-row"><span>Compte Invité</span>
            <div style="display:flex;align-items:center;gap:6px">
              <NBadge :variant="scanResult.guest_enabled ? 'warning' : 'success'">{{ scanResult.guest_enabled ? 'Activé ⚠' : 'Désactivé' }}</NBadge>
              <NButton v-if="scanResult.guest_enabled && !fixResults['disable_guest']" variant="warning" size="sm"
                style="font-size:10px;padding:2px 6px" :loading="applyingFix === 'disable_guest'" @click="applyFix('disable_guest')">Désactiver</NButton>
              <span v-if="fixResults['disable_guest']" style="font-size:11px;color:var(--success)">✓</span>
            </div>
          </div>
          <div v-if="scanResult.pending_updates_cached >= 0" class="info-row">
            <span>MAJ Windows en attente (cache)</span>
            <NBadge :variant="scanResult.pending_updates_cached > 10 ? 'danger' : scanResult.pending_updates_cached > 0 ? 'warning' : 'success'">
              {{ scanResult.pending_updates_cached }}
            </NBadge>
          </div>
          <div v-if="scanResult.last_restore_point" class="info-row">
            <span>Dernier point de restauration</span>
            <NBadge :variant="scanResult.last_restore_point.includes('Aucun') ? 'warning' : 'success'">
              {{ scanResult.last_restore_point }}
            </NBadge>
          </div>
        </div>
        <div v-if="scanResult.local_admins?.length" style="margin-top:8px">
          <p style="font-size:12px;color:var(--text-secondary);margin:0 0 4px 0">Administrateurs locaux ({{ scanResult.local_admins.length }})</p>
          <div style="display:flex;gap:6px;flex-wrap:wrap">
            <code v-for="(a, i) in scanResult.local_admins" :key="i"
              style="font-size:11px;background:var(--bg-secondary);padding:2px 6px;border-radius:4px">{{ a }}</code>
          </div>
        </div>
      </div>

      <!-- ===== COMPOSANTS MATÉRIELS ===== -->
      <ScanSectionHardware :scan-result="scanResult" />

      <!-- ===== STOCKAGE PHYSIQUE ===== -->
      <div v-if="scanResult.storage_items?.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Stockage physique ({{ scanResult.storage_items.length }} disque(s))</p>
        <div v-for="(s, i) in scanResult.storage_items" :key="i"
          style="padding:8px 0;border-bottom:1px solid var(--border);font-size:12px">
          <div style="display:flex;align-items:center;gap:10px;flex-wrap:wrap">
            <HardDrive :size="13" style="color:var(--accent);flex-shrink:0" />
            <span style="font-weight:500;flex:1;min-width:160px">{{ s.model || "Disque inconnu" }}</span>
            <NBadge :variant="s.media_type === 'SSD' || s.media_type === 'NVMe' ? 'info' : 'default'">{{ s.media_type }}</NBadge>
            <NBadge variant="neutral">{{ s.interface_type }}</NBadge>
            <span class="muted">{{ s.size_gb > 0 ? s.size_gb + ' GB' : '—' }}</span>
            <NBadge :variant="s.health === 'Healthy' || s.health === 'Sain' ? 'success' : s.health ? 'warning' : 'neutral'">
              {{ s.health || "—" }}
            </NBadge>
          </div>
          <div v-if="s.power_on_hours > 0 || s.power_on_count > 0 || s.rpm > 0"
            style="display:flex;gap:16px;margin-top:5px;padding-left:23px;flex-wrap:wrap">
            <span v-if="s.power_on_hours > 0" class="muted" style="font-size:11px">
              ⏱ {{ s.power_on_hours >= 8760 ? (s.power_on_hours/8760).toFixed(1)+' ans' : s.power_on_hours+' h' }} allumé
            </span>
            <span v-if="s.power_on_count > 0" class="muted" style="font-size:11px">
              🔁 {{ s.power_on_count }} démarrages
            </span>
            <span v-if="s.rpm > 0" class="muted" style="font-size:11px">{{ s.rpm }} RPM</span>
          </div>
        </div>
      </div>

      <!-- ===== ESPACE DISQUE ===== -->
      <div v-if="scanResult.disk_usage?.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Espace disque (volumes)</p>
        <div v-for="d in scanResult.disk_usage" :key="d.drive"
          style="display:flex;align-items:center;gap:12px;margin-bottom:8px;font-size:13px">
          <code style="min-width:40px">{{ d.drive }}</code>
          <NProgress :value="d.used_percent"
            :variant="d.used_percent > 90 ? 'danger' : d.used_percent > 80 ? 'warning' : 'default'"
            size="sm" showLabel style="flex:1" />
          <span class="muted" style="min-width:130px;text-align:right;font-size:11px;font-family:monospace">
            {{ d.free_gb.toFixed(0) }} GB libres / {{ d.total_gb.toFixed(0) }} GB
          </span>
        </div>
      </div>

      <!-- ===== LICENCES & CHIFFREMENT ===== -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Licences & Chiffrement</p>
        <div class="info-grid">
          <!-- Clé Windows -->
          <div class="info-row info-full">
            <span style="display:flex;align-items:center;gap:4px"><Key :size="12" /> Clé Windows</span>
            <code v-if="scanResult.windows_product_key" style="color:var(--accent);font-size:12px">
              {{ scanResult.windows_product_key }}
            </code>
            <span v-else class="muted" style="font-size:11px">Non disponible (BIOS/UEFI sans clé embarquée)</span>
          </div>
          <!-- Clé Office -->
          <div v-if="scanResult.office_name || scanResult.office_product_key" class="info-row info-full">
            <span style="display:flex;align-items:center;gap:4px"><Key :size="12" /> {{ scanResult.office_name || "Office" }}</span>
            <code v-if="scanResult.office_product_key" style="color:var(--success);font-size:12px">
              {{ scanResult.office_product_key }}
            </code>
            <span v-else class="muted" style="font-size:11px">Clé non disponible dans le registre</span>
          </div>
          <div v-if="scanResult.office_activation_type" class="info-row">
            <span>Activation Office</span>
            <NBadge :variant="actTypeVariant(scanResult.office_activation_type)">{{ scanResult.office_activation_type }}</NBadge>
          </div>
        </div>

        <!-- BitLocker -->
        <div v-if="scanResult.bitlocker_volumes?.length" style="margin-top:10px">
          <p style="font-size:12px;font-weight:600;margin-bottom:6px;color:var(--text-secondary)">BitLocker</p>
          <div v-for="(bv, i) in scanResult.bitlocker_volumes" :key="i"
            style="border:1px solid var(--border);border-radius:6px;padding:10px;margin-bottom:8px">
            <div style="display:flex;align-items:center;gap:8px;margin-bottom:6px">
              <component :is="bv.protection_status === 'On' || bv.protection_status === '1' ? Lock : LockOpen"
                :size="14" :style="{ color: bv.protection_status === 'On' || bv.protection_status === '1' ? 'var(--success)' : 'var(--warning)' }" />
              <strong>{{ bv.drive }}</strong>
              <NBadge :variant="bv.protection_status === 'On' || bv.protection_status === '1' ? 'success' : 'warning'">
                {{ bv.protection_status === 'On' || bv.protection_status === '1' ? 'Protégé' : 'Non protégé' }}
              </NBadge>
              <span v-if="bv.encryption_percent < 100 && bv.encryption_percent > 0" class="muted" style="font-size:11px">
                {{ bv.encryption_percent }}% chiffré
              </span>
              <div style="display:flex;gap:4px;flex-wrap:wrap">
                <NBadge v-for="(p, pi) in bv.protectors" :key="pi" variant="neutral" style="font-size:10px">{{ p }}</NBadge>
              </div>
            </div>
            <div v-if="bv.recovery_password" class="info-row" style="background:var(--bg-secondary);padding:6px 8px;border-radius:4px">
              <span style="font-size:12px;color:var(--text-secondary)">Clé de récupération</span>
              <code style="font-size:11px;color:var(--warning);word-break:break-all">{{ bv.recovery_password }}</code>
            </div>
            <div v-else class="muted" style="font-size:11px">Aucune clé de récupération disponible</div>
          </div>
        </div>
        <div v-else class="muted" style="font-size:12px;margin-top:8px">
          BitLocker non configuré (ou non détectable sans droits admin)
        </div>
      </div>

      <!-- ===== BATTERIE ===== -->
      <div v-if="batteries?.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Batterie</p>
        <div v-for="(b, i) in (batteries as BatteryInfo[])" :key="i">
          <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px">
            <Battery :size="14" style="color:var(--accent)" />
            <strong>{{ b.name }}</strong>
            <NBadge :variant="b.battery_health_percent > 80 ? 'success' : b.battery_health_percent > 50 ? 'warning' : 'danger'">
              {{ b.battery_health_percent.toFixed(0) }}% santé
            </NBadge>
            <NBadge :variant="b.status === 'Charging' || b.status === 'En charge' ? 'success' : 'info'">
              {{ b.status || "—" }}
            </NBadge>
          </div>
          <div class="info-grid">
            <div class="info-row"><span>Charge actuelle</span>
              <NBadge :variant="b.estimated_charge_remaining < 20 ? 'danger' : b.estimated_charge_remaining < 40 ? 'warning' : 'success'">
                {{ b.estimated_charge_remaining }}%
              </NBadge>
            </div>
            <div class="info-row"><span>Autonomie estimée</span><span>{{ b.estimated_run_time || "—" }}</span></div>
            <div class="info-row"><span>Cycles de charge</span>
              <NBadge :variant="b.cycle_count > 500 ? 'danger' : b.cycle_count > 300 ? 'warning' : 'success'">
                {{ b.cycle_count > 0 ? b.cycle_count : 'N/A' }}
              </NBadge>
            </div>
            <div class="info-row"><span>Capacité de conception</span>
              <span>{{ b.design_capacity > 0 ? b.design_capacity + ' mWh' : 'N/A' }}</span>
            </div>
            <div class="info-row"><span>Capacité actuelle</span>
              <span>{{ b.full_charge_capacity > 0 ? b.full_charge_capacity + ' mWh' : 'N/A' }}</span>
            </div>
            <div v-if="b.design_capacity > 0 && b.full_charge_capacity > 0" class="info-row">
              <span>Usure capacité</span>
              <code style="color:var(--warning)">
                -{{ ((1 - b.full_charge_capacity / b.design_capacity) * 100).toFixed(1) }}%
              </code>
            </div>
            <div class="info-row"><span>Chimie</span><span>{{ b.chemistry || "N/A" }}</span></div>
          </div>
        </div>
      </div>

      <!-- ===== SÉCURITÉ ===== -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Sécurité</p>
        <div v-for="[ok, label, val] in [
          [scanResult.firewall_enabled, 'Pare-feu Windows', scanResult.firewall_enabled ? 'Activé' : 'DÉSACTIVÉ ⚠'],
          [scanResult.defender_enabled, 'Defender (temps réel)', scanResult.defender_enabled ? 'Actif' : 'INACTIF ⚠'],
          [scanResult.network_ok, 'Connectivité Internet (8.8.8.8)', scanResult.network_ok ? 'OK' : 'Hors ligne'],
          [scanResult.suspicious_processes?.length === 0, 'Processus suspects', scanResult.suspicious_processes?.length === 0 ? 'Aucun' : `${scanResult.suspicious_processes?.length} détecté(s)`],
        ]" :key="label" style="display:flex;align-items:center;gap:8px;padding:6px 0;border-bottom:1px solid var(--border);font-size:13px">
          <component :is="ok ? CheckCircle : AlertTriangle" :size="14" :class="ok ? 'ic-ok' : 'ic-warn'" />
          <span style="flex:1">{{ label }}</span>
          <span class="mono">{{ val }}</span>
        </div>
        <div v-if="scanResult.open_ports?.length" style="display:flex;align-items:center;gap:8px;padding:6px 0;font-size:13px">
          <AlertTriangle :size="14" class="ic-warn" />
          <span style="flex:1">Ports en écoute globale</span>
          <code class="mono" style="font-size:11px">{{ scanResult.open_ports.join(", ") }}</code>
        </div>
        <div style="display:flex;align-items:center;gap:8px;padding:6px 0;border-bottom:1px solid var(--border);font-size:13px">
          <component :is="!scanResult.last_bsod || scanResult.last_bsod.includes('Aucun') ? CheckCircle : AlertTriangle"
            :size="14" :class="!scanResult.last_bsod || scanResult.last_bsod.includes('Aucun') ? 'ic-ok' : 'ic-warn'" />
          <span style="flex:1">Dernier BSOD</span>
          <span class="mono">{{ scanResult.last_bsod || "Aucun" }}</span>
        </div>
      </div>

      <!-- ===== ANTIVIRUS ===== -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Antivirus & Protection</p>
        <div class="info-grid">
          <div class="info-row"><span>Antivirus tiers</span><span>{{ scanResult.antivirus_installed || "Aucun (Defender)" }}</span></div>
          <div class="info-row"><span>Définitions Defender</span>
            <NBadge :variant="scanResult.defender_definition_age_days < 0 ? 'neutral' : scanResult.defender_definition_age_days <= 3 ? 'success' : scanResult.defender_definition_age_days <= 7 ? 'warning' : 'danger'">
              {{ scanResult.defender_definition_age_days >= 0 ? scanResult.defender_definition_age_days + ' j' : 'N/A' }}
            </NBadge>
          </div>
          <div class="info-row"><span>Dernier KB Windows</span>
            <NBadge :variant="scanResult.last_update_days < 0 ? 'neutral' : scanResult.last_update_days <= 30 ? 'success' : scanResult.last_update_days <= 60 ? 'warning' : 'danger'">
              {{ scanResult.last_update_days >= 0 ? 'il y a ' + scanResult.last_update_days + ' j' : 'N/A' }}
            </NBadge>
          </div>
        </div>
      </div>

      <!-- ===== INTÉGRITÉ WINDOWS ===== -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Intégrité Windows</p>

        <!-- DISM row -->
        <div style="padding:6px 0;border-bottom:1px solid var(--border)">
          <div style="display:flex;align-items:center;gap:8px;font-size:13px">
            <component :is="isDismHealthy(scanResult.dism_status) ? CheckCircle : AlertTriangle"
              :size="14" :class="isDismHealthy(scanResult.dism_status) ? 'ic-ok' : 'ic-warn'" />
            <span style="flex:1">DISM (Health Store)</span>
            <span class="mono">{{ scanResult.dism_status }}</span>
            <NButton v-if="!isDismHealthy(scanResult.dism_status)" size="sm" variant="danger"
              style="font-size:10px;padding:2px 8px" @click="runRepairCommand('dism')">
              Réparer (DISM)
            </NButton>
          </div>
          <pre v-if="scanResult.dism_details && !isDismHealthy(scanResult.dism_status)"
            class="scan-details-pre">{{ scanResult.dism_details }}</pre>
        </div>

        <!-- SFC row -->
        <div style="padding:6px 0;border-bottom:1px solid var(--border)">
          <div style="display:flex;align-items:center;gap:8px;font-size:13px">
            <component :is="isSfcIntegre(scanResult.sfc_status) ? CheckCircle : AlertTriangle"
              :size="14" :class="isSfcIntegre(scanResult.sfc_status) ? 'ic-ok' : 'ic-warn'" />
            <span style="flex:1">SFC (System File Checker)</span>
            <span class="mono">{{ scanResult.sfc_status }}</span>
            <NButton size="sm" variant="ghost" style="font-size:10px;padding:2px 8px" @click="runRepairCommand('sfc')">
              Lancer SFC
            </NButton>
          </div>
          <pre v-if="scanResult.sfc_details" class="scan-details-pre">{{ scanResult.sfc_details }}</pre>
        </div>

        <!-- WinGet row -->
        <div style="padding:6px 0">
          <div style="display:flex;align-items:center;gap:8px;font-size:13px">
            <component :is="scanResult.winget_upgradable?.length === 0 ? CheckCircle : AlertTriangle"
              :size="14" :class="scanResult.winget_upgradable?.length === 0 ? 'ic-ok' : 'ic-warn'" />
            <span style="flex:1">WinGet — mises à jour disponibles</span>
            <span class="mono">{{ scanResult.winget_upgradable?.length || 0 }} logiciel(s)</span>
          </div>
          <div v-if="scanResult.winget_upgradable?.length" style="margin-top:8px">
            <div v-for="(u, i) in (scanResult.winget_upgradable as WingetUpgrade[])" :key="i"
              style="display:flex;align-items:center;gap:8px;padding:4px 0;border-bottom:1px solid var(--border);font-size:12px">
              <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-weight:500">{{ u.name }}</span>
              <code class="muted" style="font-size:10px;min-width:160px">{{ u.id }}</code>
              <NBadge variant="warning" style="font-size:10px;flex-shrink:0">{{ u.current_version }}</NBadge>
              <span style="color:var(--success);font-size:10px;flex-shrink:0">→ {{ u.available_version }}</span>
            </div>
          </div>
        </div>

        <!-- Windows Update row -->
        <div style="padding:6px 0;border-bottom:1px solid var(--border)">
          <div style="display:flex;align-items:center;gap:8px;font-size:13px">
            <component :is="(scanResult.pending_updates_cached ?? 0) === 0 ? CheckCircle : AlertTriangle"
              :size="14" :class="(scanResult.pending_updates_cached ?? 0) > 0 ? 'ic-warn' : 'ic-ok'" />
            <span style="flex:1">Windows Update — mises à jour en attente</span>
            <span class="mono">{{ scanResult.pending_updates_cached >= 0 ? scanResult.pending_updates_cached : '?' }}</span>
            <NButton variant="ghost" size="sm" style="font-size:10px;padding:2px 6px" :loading="applyingFix === 'wu_usoclient'" @click="applyFix('wu_usoclient')">Forcer WU</NButton>
          </div>
          <div v-if="scanResult.windows_updates_pending?.length" style="margin-top:6px">
            <div v-for="(u, i) in scanResult.windows_updates_pending" :key="i"
              style="padding:2px 0;font-size:11px;color:var(--text-secondary);border-bottom:1px solid var(--border)">{{ u }}</div>
          </div>
        </div>

        <!-- Chocolatey row -->
        <div style="padding:6px 0;border-bottom:1px solid var(--border)">
          <div style="display:flex;align-items:center;gap:8px;font-size:13px">
            <component :is="scanResult.choco_upgradable?.length ? AlertTriangle : CheckCircle"
              :size="14" :class="scanResult.choco_upgradable?.length ? 'ic-warn' : 'ic-ok'" />
            <span style="flex:1">Chocolatey — mises à jour disponibles</span>
            <span class="mono">{{ scanResult.choco_upgradable?.length || 0 }} paquet(s)</span>
          </div>
          <div v-if="scanResult.choco_upgradable?.length" style="margin-top:6px">
            <div v-for="(s, i) in scanResult.choco_upgradable" :key="i"
              style="padding:2px 0;font-size:11px;color:var(--text-secondary);font-family:monospace;border-bottom:1px solid var(--border)">{{ s }}</div>
          </div>
        </div>
        <!-- Scoop row -->
        <div style="padding:6px 0">
          <div style="display:flex;align-items:center;gap:8px;font-size:13px">
            <component :is="scanResult.scoop_upgradable?.length ? AlertTriangle : CheckCircle"
              :size="14" :class="scanResult.scoop_upgradable?.length ? 'ic-warn' : 'ic-ok'" />
            <span style="flex:1">Scoop — mises à jour disponibles</span>
            <span class="mono">{{ scanResult.scoop_upgradable?.length || 0 }} paquet(s)</span>
          </div>
          <div v-if="scanResult.scoop_upgradable?.length" style="margin-top:6px">
            <div v-for="(s, i) in scanResult.scoop_upgradable" :key="i"
              style="padding:2px 0;font-size:11px;color:var(--text-secondary);font-family:monospace;border-bottom:1px solid var(--border)">{{ s }}</div>
          </div>
        </div>
      </div>

      <!-- ===== ABONNEMENTS WMI SUSPECTS ===== -->
      <div v-if="scanResult.wmi_subscriptions > 0" class="diag-section" style="border-left:3px solid var(--danger)">
        <p class="diag-section-label" style="margin:0 0 8px 0;color:var(--danger)">
          <AlertTriangle :size="13" style="display:inline;margin-right:6px" />
          Abonnements WMI suspects ({{ scanResult.wmi_subscriptions }}) — Indicateur malware potentiel
        </p>
        <div v-for="(sub, i) in scanResult.wmi_subscription_details" :key="i"
          style="padding:6px 0;border-bottom:1px solid var(--border)">
          <div style="display:flex;align-items:center;gap:8px;font-size:12px;flex-wrap:wrap">
            <code style="font-weight:600;color:var(--danger)">{{ sub.name || "(sans nom)" }}</code>
            <NBadge variant="danger" style="font-size:10px">{{ sub.consumer_type }}</NBadge>
            <button class="reg-action-btn" title="Copier le chemin WMI" @click="copyRegPath(sub.path)">📋</button>
          </div>
          <div class="muted" style="font-size:10px;margin-top:2px;font-family:monospace;word-break:break-all">{{ sub.path }}</div>
        </div>
        <div style="margin-top:8px;display:flex;align-items:center;gap:10px;flex-wrap:wrap">
          <NButton variant="danger" size="sm" :loading="applyingFix === 'wmi_cleanup'"
            :disabled="!!fixResults['wmi_cleanup']" @click="applyFix('wmi_cleanup')">
            🧹 Nettoyer les abonnements WMI
          </NButton>
          <span v-if="fixResults['wmi_cleanup']" style="font-size:11px;color:var(--success)">✓ Nettoyé — redémarrez pour confirmer</span>
          <button class="reg-action-btn" title="Copier la commande PS" @click="copyRegPath('Get-WmiObject -Namespace root\\subscription -Class __EventFilter | Remove-WmiObject -EA SilentlyContinue; Get-WmiObject -Namespace root\\subscription -Class __EventConsumer | Remove-WmiObject -EA SilentlyContinue; Get-WmiObject -Namespace root\\subscription -Class __FilterToConsumerBinding | Remove-WmiObject -EA SilentlyContinue')">📋</button>
        </div>
      </div>

      <!-- ===== PROCESSUS SUSPECTS ===== -->
      <div v-if="scanResult.suspicious_processes?.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Processus hors chemins sécurisés ({{ scanResult.suspicious_processes.length }})</p>
        <div v-for="p in scanResult.suspicious_processes" :key="p.pid" class="list-row">
          <code class="list-name">{{ p.name }}</code>
          <NBadge variant="warning" style="flex-shrink:0">{{ p.reason }}</NBadge>
          <div class="muted" style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:11px">{{ p.path }}</div>
        </div>
      </div>

      <!-- ===== SERVICES TIERS ===== -->
      <div v-if="scanResult.suspicious_services?.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Services tiers actifs ({{ scanResult.suspicious_services.length }})</p>
        <div v-for="(s, i) in scanResult.suspicious_services.slice(0, 15)" :key="i" class="list-row">
          <code class="list-name" style="min-width:130px">{{ s.name }}</code>
          <div class="muted" style="flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ s.display_name }}</div>
          <div class="muted" style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:11px">{{ s.path }}</div>
        </div>
      </div>

      <!-- ===== AUTORUNS ===== -->
      <div v-if="scanResult.autorun_entries?.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Entrées Autorun tiers ({{ scanResult.autorun_entries.length }})</p>
        <div v-for="(a, i) in scanResult.autorun_entries.slice(0, 20)" :key="i" class="list-row autorun-row">
          <code class="list-name" style="min-width:120px;flex-shrink:0">{{ a.name }}</code>
          <div class="autorun-reg-path" :title="fullRegPath(a.location, a.name)">
            <span class="reg-path-text">{{ fullRegPath(a.location, a.name) }}</span>
          </div>
          <div class="muted" style="flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:11px" :title="a.path">{{ a.path }}</div>
          <div class="autorun-actions">
            <button class="reg-action-btn" title="Copier le chemin de registre" @click="copyRegPath(fullRegPath(a.location, a.name))">📋</button>
            <button class="reg-action-btn" title="Ouvrir dans Regedit" @click="openRegedit(a.location)">🔑</button>
          </div>
        </div>
      </div>

      <!-- ===== ÉVÉNEMENTS D'ERREUR ===== -->
      <div v-if="scanResult.recent_errors?.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">
          Événements d'erreur récents (48h) — {{ scanResult.recent_errors.length }}
        </p>
        <div v-for="(e, i) in scanResult.recent_errors.slice(0, 25)" :key="i" class="list-row">
          <code class="muted" style="min-width:110px;font-size:10px">{{ e.time }}</code>
          <NBadge :variant="e.level?.toLowerCase().includes('critical') || e.level?.toLowerCase().includes('critique') ? 'danger' : 'warning'"
            style="flex-shrink:0;font-size:10px">{{ e.level }}</NBadge>
          <div style="flex:1;min-width:0">
            <span style="font-size:12px;font-weight:500">{{ e.source }}</span>
            <div class="muted" style="font-size:11px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">{{ e.message }}</div>
          </div>
        </div>
      </div>

      <!-- ===== TOP PROCESSUS ===== -->
      <div v-if="scanResult.top_cpu?.length || scanResult.top_ram?.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Top 5 Processus (snapshot)</p>
        <div style="display:flex;gap:16px;flex-wrap:wrap">
          <div v-if="scanResult.top_cpu?.length" style="flex:1;min-width:200px">
            <p style="font-size:11px;font-weight:600;color:var(--text-secondary);margin:0 0 6px 0"><Cpu :size="11" style="display:inline;margin-right:4px" />CPU — temps cumulé (sec.)</p>
            <div v-for="(p, i) in scanResult.top_cpu" :key="i"
              style="display:flex;align-items:center;gap:8px;padding:4px 0;border-bottom:1px solid var(--border);font-size:12px">
              <code class="muted" style="min-width:34px;font-size:10px">{{ p.pid }}</code>
              <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ p.name }}</span>
              <code style="color:var(--accent);font-size:11px;flex-shrink:0">{{ p.value }}s</code>
            </div>
          </div>
          <div v-if="scanResult.top_ram?.length" style="flex:1;min-width:200px">
            <p style="font-size:11px;font-weight:600;color:var(--text-secondary);margin:0 0 6px 0"><MemoryStick :size="11" style="display:inline;margin-right:4px" />RAM (MB)</p>
            <div v-for="(p, i) in scanResult.top_ram" :key="i"
              style="display:flex;align-items:center;gap:8px;padding:4px 0;border-bottom:1px solid var(--border);font-size:12px">
              <code class="muted" style="min-width:34px;font-size:10px">{{ p.pid }}</code>
              <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ p.name }}</span>
              <code style="color:var(--success);font-size:11px;flex-shrink:0">{{ p.value }} MB</code>
            </div>
          </div>
        </div>
      </div>

      <!-- ===== TÂCHES PLANIFIÉES SUSPECTES ===== -->
      <div v-if="scanResult.susp_tasks?.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Tâches planifiées suspectes ({{ scanResult.susp_tasks_count }})</p>
        <div v-for="(t, i) in scanResult.susp_tasks" :key="i" class="list-row">
          <code class="list-name" style="min-width:140px">{{ t.name }}</code>
          <NBadge variant="warning" style="flex-shrink:0;font-size:10px">{{ t.path }}</NBadge>
          <div class="muted" style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:11px">{{ t.exec }}</div>
        </div>
      </div>

      <!-- Export scan -->
      <div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center;padding-top:4px">
        <NButton variant="ghost" size="sm" @click="onRunScan"><RefreshCw :size="12" /> Relancer Scan</NButton>
        <NButton variant="ghost" size="sm" @click="doExportTxt"><FileText :size="12" /> Export .txt</NButton>
        <NButton variant="ghost" size="sm" @click="doExportHtml"><FileCode :size="12" /> Export .html</NButton>
        <NButton variant="ghost" size="sm" @click="doExportMd"><FileDown :size="12" /> Export .md</NButton>
        <NButton variant="ghost" size="sm" @click="doExportJson"><FileDown :size="12" /> Export .json</NButton>
      </div>
    </div>
  </div>
</template>
