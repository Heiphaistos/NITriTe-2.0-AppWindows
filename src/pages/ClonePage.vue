<script setup lang="ts">
defineOptions({ name: "ClonePage" });
import { ref, computed, onMounted, watch, onUnmounted } from "vue";
import { invoke, invokeRaw } from "@/utils/invoke";
import NButton from "@/components/ui/NButton.vue";
import NProgress from "@/components/ui/NProgress.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NBadge from "@/components/ui/NBadge.vue";
import { useNotificationStore } from "@/stores/notifications";
import PartitionManagerTab from "@/components/recovery/PartitionManagerTab.vue";
import {
  Copy, HardDrive, RefreshCw, Play, AlertTriangle,
  CheckCircle, XCircle, Server, Cpu, Shield,
  Info, ChevronRight, Layers,
} from "lucide-vue-next";

const notify = useNotificationStore();

interface PartitionInfo {
  letter: string; label: string; size_gb: number; free_gb: number;
  file_system: string; is_system: boolean; is_boot: boolean;
}
interface DiskInfo {
  index: number; label: string; size_gb: number; disk_type: string;
  bus_type: string; partitions: PartitionInfo[];
}
interface CloneProgress { step: string; percent: number; message: string; }
interface CloneResult { success: boolean; method: string; message: string; duration_secs: number; }

type Tab = "image" | "robocopy" | "partitions";
const activeTab = ref<Tab>("image");
const disks = ref<DiskInfo[]>([]);
const loadingDisks = ref(false);
const sourceDrive = ref("");
const targetDrive = ref("");
const cloning = ref(false);
const progress = ref(0);
const progressMsg = ref("");
const progressStep = ref(0); // 0=idle 1=prep 2=copy 3=verify 4=done
const result = ref<CloneResult | null>(null);
const confirmed = ref(false);

watch(activeTab, () => {
  confirmed.value = false;
  targetDrive.value = "";
  result.value = null;
  progress.value = 0;
  progressStep.value = 0;
});

const drivesWithLetters = computed(() =>
  disks.value.flatMap(d =>
    d.partitions
      .filter(p => p.letter)
      .map(p => ({ ...p, disk_label: d.label, disk_index: d.index }))
  )
);
const availableTargets = computed(() =>
  drivesWithLetters.value.filter(p =>
    // p.letter garde le ':' (ex: "C:") alors que sourceDrive ne l'a pas (ex: "C",
    // assigné via .replace(":", "") ligne 72 plus bas) — la comparaison brute ne
    // matchait donc jamais et n'excluait jamais réellement la source par lettre.
    // Sans impact pratique aujourd'hui (is_system/is_boot exclut déjà le lecteur
    // système sur cet onglet), mais incohérent avec le reste du fichier qui
    // normalise systématiquement via .replace(':', '') avant de comparer.
    !p.is_system && !p.is_boot && p.letter.replace(':', '') !== sourceDrive.value
  )
);
const systemDrive = computed(() =>
  drivesWithLetters.value.find(p => p.is_boot || p.is_system)
);

async function loadDisks() {
  loadingDisks.value = true;
  disks.value = [];
  try {
    disks.value = await invoke<DiskInfo[]>("get_disks_for_clone");
    if (systemDrive.value) sourceDrive.value = systemDrive.value.letter.replace(":", "");
  } catch (e: unknown) {
    notify.error("Erreur", String(e));
  }
  loadingDisks.value = false;
}

let unlistenCloneProgress: (() => void) | null = null;

onMounted(async () => {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlistenCloneProgress = await listen<CloneProgress>("clone-progress", (ev) => {
      progress.value = ev.payload.percent;
      progressMsg.value = ev.payload.message;
      if (ev.payload.percent < 20) progressStep.value = 1;
      else if (ev.payload.percent < 85) progressStep.value = 2;
      else if (ev.payload.percent < 100) progressStep.value = 3;
      else progressStep.value = 4;
    });
  } catch (e) {
    notify.error("Monitoring clonage", "Impossible d'écouter les événements de progression.");
  }
  await loadDisks();
});

onUnmounted(() => {
  if (unlistenCloneProgress) { unlistenCloneProgress(); unlistenCloneProgress = null; }
});

async function startSystemImage() {
  if (!targetDrive.value || !confirmed.value) return;
  cloning.value = true; result.value = null; progress.value = 5; progressStep.value = 1;
  startTimer();
  progressMsg.value = "Lancement de Windows Backup (wbadmin)...";
  try {
    result.value = await invokeRaw<CloneResult>("start_system_image", { targetDrive: targetDrive.value });
    progressStep.value = 4;
    if (result.value.success) notify.success("Image système créée", result.value.message);
    else notify.error("Clonage échoué", result.value.message);
  } catch (e: unknown) {
    notify.error("Erreur", String(e));
  }
  stopTimer();
  cloning.value = false;
}

async function startRobocopy() {
  if (!sourceDrive.value || !targetDrive.value || !confirmed.value) return;
  if (sourceDrive.value === targetDrive.value) { notify.error("Clonage impossible", "Source et destination identiques."); return; }
  cloning.value = true; result.value = null; progress.value = 5; progressStep.value = 1;
  startTimer();
  progressMsg.value = "Démarrage Robocopy...";
  try {
    result.value = await invokeRaw<CloneResult>("start_robocopy_clone", {
      sourceDrive: sourceDrive.value,
      targetDrive: targetDrive.value,
    });
    progressStep.value = 4;
    if (result.value.success) notify.success("Clonage terminé", result.value.message);
    else notify.error("Erreur Robocopy", result.value.message);
  } catch (e: unknown) {
    notify.error("Erreur", String(e));
  }
  stopTimer();
  cloning.value = false;
}

function formatSize(gb: number) {
  return gb >= 1000 ? `${(gb / 1000).toFixed(1)} To` : `${gb.toFixed(0)} Go`;
}

const stepLabels = ["Préparation", "Copie", "Vérification", "Terminé"];

// Chronomètre
const elapsedSecs = ref(0);
let elapsedTimer: ReturnType<typeof setInterval> | null = null;
function startTimer() { elapsedSecs.value = 0; elapsedTimer = setInterval(() => elapsedSecs.value++, 1000); }
function stopTimer() { if (elapsedTimer) { clearInterval(elapsedTimer); elapsedTimer = null; } }
function formatElapsed(s: number) { return `${Math.floor(s / 60)}min ${s % 60}s`; }
onUnmounted(stopTimer);

// ETA & débit estimé (basé sur progression % / temps écoulé)
const etaSecs = computed((): number | null => {
  if (!cloning.value || progress.value <= 0 || elapsedSecs.value < 3) return null;
  const rate = progress.value / elapsedSecs.value; // % par seconde
  if (rate <= 0) return null;
  const remaining = 100 - progress.value;
  return Math.round(remaining / rate);
});

const speedStr = computed((): string | null => {
  if (!cloning.value || progress.value <= 0 || elapsedSecs.value < 3) return null;
  // Estimer la taille totale d'après la partition source
  const srcPart = activeTab.value === 'robocopy'
    ? drivesWithLetters.value.find(p => p.letter.replace(':', '') === sourceDrive.value)
    : drivesWithLetters.value.find(p => p.is_boot || p.is_system);
  if (!srcPart) return null;
  const usedGb = srcPart.size_gb - srcPart.free_gb;
  if (usedGb <= 0) return null;
  const copiedGb = (progress.value / 100) * usedGb;
  const mbPerSec = (copiedGb * 1024) / elapsedSecs.value;
  return mbPerSec >= 1 ? `${mbPerSec.toFixed(0)} MB/s` : `${(mbPerSec * 1024).toFixed(0)} KB/s`;
});

function formatEta(s: number): string {
  if (s < 60) return `${s}s`;
  return `${Math.floor(s / 60)}min ${s % 60}s`;
}

// Validation source ≠ cible (lettre identique)
const sameLetterError = computed((): string | null => {
  if (!sourceDrive.value || !targetDrive.value) return null;
  if (sourceDrive.value.toUpperCase() === targetDrive.value.toUpperCase()) {
    return "La source et la destination ne peuvent pas être le même lecteur.";
  }
  return null;
});

// Erreur espace insuffisant
const spaceError = computed((): string | null => {
  if (!spaceCheck.value) return null;
  if (!spaceCheck.value.ok) {
    return `Espace insuffisant : besoin de ${spaceCheck.value.usedGb.toFixed(1)} Go, seulement ${spaceCheck.value.freeGb.toFixed(1)} Go disponibles sur la cible.`;
  }
  return null;
});

// Checklist pré-lancement
const preflight = computed(() => {
  const tgtPartImg = drivesWithLetters.value.find(p => p.letter.replace(':', '') === targetDrive.value);
  const srcPartRob = drivesWithLetters.value.find(p => p.letter.replace(':', '') === sourceDrive.value);
  const tgtPartRob = drivesWithLetters.value.find(p => p.letter.replace(':', '') === targetDrive.value);

  const ntfsOk = activeTab.value === 'image'
    ? (tgtPartImg?.file_system === 'NTFS')
    : (tgtPartRob?.file_system === 'NTFS');

  const diffDisk = activeTab.value === 'robocopy'
    ? (srcPartRob?.disk_index !== tgtPartRob?.disk_index)
    : true; // wbadmin gère les partitions du même disque

  return [
    { label: "Droits administrateur", ok: true, always: true },
    { label: "Cible format NTFS", ok: !!ntfsOk, na: !targetDrive.value },
    { label: "Source ≠ Cible (disque physique différent)", ok: !!diffDisk, na: activeTab.value !== 'robocopy' || !sourceDrive.value || !targetDrive.value },
    { label: "Espace disque suffisant", ok: spaceCheck.value?.ok ?? false, na: !spaceCheck.value },
  ];
});

// Vérification espace pré-lancement
const spaceCheck = computed(() => {
  if (activeTab.value === 'image') {
    const src = drivesWithLetters.value.find(p => p.is_boot || p.is_system);
    const tgt = drivesWithLetters.value.find(p => p.letter.replace(':', '') === targetDrive.value);
    if (!src || !tgt) return null;
    const usedGb = src.size_gb - src.free_gb;
    return { ok: tgt.free_gb >= usedGb, usedGb, freeGb: tgt.free_gb };
  }
  if (activeTab.value === 'robocopy') {
    const src = drivesWithLetters.value.find(p => p.letter.replace(':', '') === sourceDrive.value);
    const tgt = drivesWithLetters.value.find(p => p.letter.replace(':', '') === targetDrive.value);
    if (!src || !tgt) return null;
    const usedGb = src.size_gb - src.free_gb;
    return { ok: tgt.free_gb >= usedGb, usedGb, freeGb: tgt.free_gb };
  }
  return null;
});
</script>

<template>
  <div class="clone-page">

    <!-- Header Premium -->
    <div class="clone-hero">
      <div class="hero-icon-wrap">
        <div class="hero-icon"><Copy :size="28" /></div>
      </div>
      <div class="hero-text">
        <h1 class="hero-title">Clonage &amp; Sauvegarde Système</h1>
        <p class="hero-desc">Image système complète ou copie disque-à-disque — migration Windows 10/11 sécurisée</p>
      </div>
      <NBadge variant="info" style="flex-shrink:0">Admin requis</NBadge>
    </div>

    <!-- Carte prérequis -->
    <div class="prereq-card">
      <div class="prereq-title"><Shield :size="14" /> Prérequis</div>
      <div class="prereq-list">
        <div class="prereq-item"><CheckCircle :size="13" style="color:var(--success)" /> Droits administrateur Windows</div>
        <div class="prereq-item"><CheckCircle :size="13" style="color:var(--success)" /> Disque cible en format NTFS</div>
        <div class="prereq-item"><CheckCircle :size="13" style="color:var(--success)" /> Espace disque cible ≥ espace utilisé source</div>
        <div class="prereq-item"><Info :size="13" style="color:var(--info)" /> Recommandé : fermer toutes les applications avant de lancer</div>
      </div>
    </div>

    <!-- Méthodes recommandées -->
    <div class="methods-row">
      <div class="method-card" :class="{ selected: activeTab === 'image' }" @click="activeTab = 'image'">
        <div class="method-icon method-icon-blue"><Server :size="20" /></div>
        <div class="method-info">
          <span class="method-name">Image Système <span class="method-badge">Recommandé</span></span>
          <span class="method-desc">wbadmin — récupération complète OS depuis le démarrage</span>
        </div>
        <ChevronRight :size="14" class="method-arrow" :class="{ active: activeTab === 'image' }" />
      </div>
      <div class="method-card" :class="{ selected: activeTab === 'robocopy' }" @click="activeTab = 'robocopy'">
        <div class="method-icon method-icon-orange"><Copy :size="20" /></div>
        <div class="method-info">
          <span class="method-name">Clone Robocopy</span>
          <span class="method-desc">Migration rapide des fichiers et données vers un nouveau disque</span>
        </div>
        <ChevronRight :size="14" class="method-arrow" :class="{ active: activeTab === 'robocopy' }" />
      </div>
      <div class="method-card" :class="{ selected: activeTab === 'partitions' }" @click="activeTab = 'partitions'">
        <div class="method-icon method-icon-green"><Layers :size="20" /></div>
        <div class="method-info">
          <span class="method-name">Partitions &amp; SMART</span>
          <span class="method-desc">Gestionnaire de partitions — format, création, santé disque</span>
        </div>
        <ChevronRight :size="14" class="method-arrow" :class="{ active: activeTab === 'partitions' }" />
      </div>
    </div>

    <!-- Toolbar -->
    <div class="toolbar">
      <NButton variant="ghost" size="sm" :loading="loadingDisks" @click="loadDisks">
        <RefreshCw :size="13" /> Actualiser les disques
      </NButton>
      <span v-if="!loadingDisks" class="count-badge">{{ disks.length }} disque(s) détecté(s)</span>
    </div>

    <!-- Loading -->
    <div v-if="loadingDisks" class="loading-state"><NSpinner :size="22" /><p>Détection des disques...</p></div>

    <template v-else>

      <!-- ══ IMAGE SYSTÈME ══ -->
      <template v-if="activeTab === 'image'">
        <div class="info-banner warning">
          <AlertTriangle :size="14" />
          <span><strong>Droits administrateur obligatoires.</strong> Clic droit → "Exécuter en tant qu'administrateur" si l'opération échoue.</span>
        </div>
        <div class="info-banner info">
          <Info :size="14" />
          <span>wbadmin crée une Image Système Windows complète (OS + données) récupérable depuis le menu de démarrage Windows.</span>
        </div>

        <div class="config-grid">
          <div class="config-card">
            <p class="config-label"><Cpu :size="13" /> Système à capturer</p>
            <div class="drive-badge system">
              <HardDrive :size="14" />
              <div>
                <span>C:\ — Windows (système actuel)</span>
                <span class="badge-sub">Toutes les partitions critiques</span>
              </div>
            </div>
          </div>

          <div class="config-card">
            <p class="config-label"><HardDrive :size="13" /> Destination de l'image</p>
            <select v-model="targetDrive" class="drive-select">
              <option value="">— Choisir un lecteur cible —</option>
              <option v-for="p in availableTargets" :key="p.letter" :value="p.letter.replace(':', '')">
                {{ p.letter }} — {{ p.disk_label }} — {{ formatSize(p.size_gb) }} ({{ formatSize(p.free_gb) }} libres)
              </option>
            </select>
            <p class="config-hint">⚠ Le lecteur cible doit avoir plus d'espace que le système source</p>
          </div>
        </div>

        <!-- Checklist pré-lancement -->
        <div v-if="targetDrive" class="preflight-list">
          <div v-for="c in preflight" :key="c.label" class="preflight-item" :class="c.na ? 'na' : c.ok ? 'ok' : 'nok'">
            <span class="pf-dot">{{ c.na ? '○' : c.ok ? '✓' : '✗' }}</span>
            <span>{{ c.label }}</span>
          </div>
        </div>

        <div class="confirm-row">
          <label class="confirm-label">
            <input type="checkbox" v-model="confirmed" />
            Je confirme que le lecteur cible peut être écrasé et que j'ai vérifié l'espace disponible
          </label>
        </div>

        <div v-if="spaceError" class="validation-error">
          <AlertTriangle :size="13" /> {{ spaceError }}
        </div>

        <NButton variant="primary" :disabled="!targetDrive || !confirmed || cloning || spaceCheck?.ok === false" :loading="cloning" @click="startSystemImage">
          <Play :size="14" /> Créer l'Image Système
        </NButton>
      </template>

      <!-- ══ CLONE ROBOCOPY ══ -->
      <template v-if="activeTab === 'robocopy'">
        <div class="info-banner info">
          <Info :size="14" />
          <span>Robocopy /MIR clone tous les fichiers avec attributs et permissions. Idéal pour migrer des données. Ne copie pas le secteur de démarrage.</span>
        </div>

        <div class="config-grid">
          <div class="config-card">
            <p class="config-label"><HardDrive :size="13" /> Source</p>
            <select v-model="sourceDrive" class="drive-select">
              <option value="">— Choisir la source —</option>
              <option v-for="p in drivesWithLetters" :key="p.letter" :value="p.letter.replace(':', '')">
                {{ p.letter }} — {{ p.label || p.disk_label }} — {{ formatSize(p.size_gb) }}
                {{ p.is_system ? '(Système)' : '' }}
              </option>
            </select>
          </div>

          <div class="config-card">
            <p class="config-label"><HardDrive :size="13" /> Destination</p>
            <select v-model="targetDrive" class="drive-select">
              <option value="">— Choisir la destination —</option>
              <option
                v-for="p in drivesWithLetters.filter(p => p.letter.replace(':', '') !== sourceDrive)"
                :key="p.letter"
                :value="p.letter.replace(':', '')"
              >
                {{ p.letter }} — {{ p.label || p.disk_label }} — {{ formatSize(p.size_gb) }} ({{ formatSize(p.free_gb) }} libres)
              </option>
            </select>
          </div>
        </div>

        <!-- Checklist pré-lancement -->
        <div v-if="sourceDrive && targetDrive" class="preflight-list">
          <div v-for="c in preflight" :key="c.label" class="preflight-item" :class="c.na ? 'na' : c.ok ? 'ok' : 'nok'">
            <span class="pf-dot">{{ c.na ? '○' : c.ok ? '✓' : '✗' }}</span>
            <span>{{ c.label }}</span>
          </div>
        </div>

        <div class="confirm-row">
          <label class="confirm-label">
            <input type="checkbox" v-model="confirmed" />
            Je confirme que la destination peut être écrasée (les fichiers existants seront supprimés avec /MIR)
          </label>
        </div>

        <div v-if="sameLetterError" class="validation-error">
          <AlertTriangle :size="13" /> {{ sameLetterError }}
        </div>
        <div v-else-if="spaceError" class="validation-error">
          <AlertTriangle :size="13" /> {{ spaceError }}
        </div>

        <NButton variant="primary" :disabled="!sourceDrive || !targetDrive || !confirmed || cloning || !!sameLetterError || spaceCheck?.ok === false" :loading="cloning" @click="startRobocopy">
          <Play :size="14" /> Lancer le Clone
        </NButton>
      </template>

      <!-- Progression avec étapes visuelles -->
      <div v-if="cloning || progress > 0" class="progress-section">
        <!-- Étapes -->
        <div class="step-bar">
          <div v-for="(label, i) in stepLabels" :key="i" class="step-item" :class="{ done: progressStep > i+1, active: progressStep === i+1 }">
            <div class="step-dot">{{ progressStep > i+1 ? '✓' : i+1 }}</div>
            <span class="step-label">{{ label }}</span>
          </div>
        </div>
        <div class="progress-header">
          <NSpinner v-if="cloning" :size="14" />
          <span>{{ progressMsg }}</span>
          <span v-if="cloning && elapsedSecs > 0" class="elapsed-badge">⏱ {{ formatElapsed(elapsedSecs) }}</span>
          <span v-if="cloning && speedStr" class="speed-badge">{{ speedStr }}</span>
          <span v-if="cloning && etaSecs !== null" class="eta-badge">ETA : {{ formatEta(etaSecs as number) }}</span>
        </div>
        <NProgress :value="progress" showLabel size="lg" />
        <p class="progress-note">Cette opération peut prendre plusieurs minutes selon la taille du disque.</p>
      </div>

      <!-- Résultat -->
      <div v-if="result" class="result-card" :class="result.success ? 'success' : 'error'">
        <CheckCircle v-if="result.success" :size="22" />
        <XCircle v-else :size="22" />
        <div>
          <p class="result-title">{{ result.success ? '✓ Opération réussie' : '✕ Opération échouée' }}</p>
          <p class="result-method">Méthode : {{ result.method }}</p>
          <p class="result-msg">{{ result.message }}</p>
          <p class="result-duration">Durée : {{ Math.floor(result.duration_secs / 60) }}min {{ result.duration_secs % 60 }}s</p>
        </div>
      </div>

      <!-- ══ PARTITIONS & SMART ══ -->
      <template v-if="activeTab === 'partitions'">
        <PartitionManagerTab />
      </template>

      <!-- Avertissements & Bonnes Pratiques -->
      <div v-if="activeTab !== 'partitions'" class="checklist-card">
        <p class="checklist-title">⚡ Bonnes Pratiques</p>
        <div class="checklist">
          <div class="checklist-item">☑ Fermez toutes les applications avant de lancer le clonage</div>
          <div class="checklist-item">☑ Vérifiez que le disque cible est vide ou que les données peuvent être perdues</div>
          <div class="checklist-item">☑ Pour une migration OS, préférez l'Image Système (wbadmin) à Robocopy</div>
          <div class="checklist-item">☑ Conservez l'ancienne installation jusqu'à validation du nouveau disque</div>
          <div class="checklist-item">☑ Pour un clonage secteur par secteur, envisagez Clonezilla ou Macrium Reflect</div>
        </div>
      </div>

      <!-- Disques détectés -->
      <div v-if="disks.length > 0 && activeTab !== 'partitions'" class="disks-section">
        <p class="section-label">Disques détectés</p>
        <div v-for="disk in disks" :key="disk.index" class="disk-card">
          <div class="disk-header">
            <div class="disk-icon"><HardDrive :size="16" /></div>
            <div class="disk-info">
              <span class="disk-name">Disque {{ disk.index }} — {{ disk.label }}</span>
              <span class="disk-meta">{{ formatSize(disk.size_gb) }} · {{ disk.disk_type }} · {{ disk.bus_type }}</span>
            </div>
          </div>
          <div class="partitions-list">
            <div v-for="p in disk.partitions" :key="p.letter" class="part-row">
              <span class="part-letter">{{ p.letter || '—' }}</span>
              <span class="part-label">{{ p.label || '—' }}</span>
              <span class="part-fs">{{ p.file_system }}</span>
              <span class="part-size">{{ formatSize(p.size_gb) }}</span>
              <div class="part-bar-wrap">
                <div class="part-bar-track">
                  <div class="part-bar-fill" :style="{
                    width: `${Math.round(((p.size_gb - p.free_gb) / p.size_gb) * 100)}%`,
                    background: (p.free_gb / p.size_gb) < 0.1 ? 'var(--danger)' : 'var(--accent-primary)'
                  }" />
                </div>
              </div>
              <span class="part-free" :style="{ color: (p.free_gb / p.size_gb) < 0.1 ? 'var(--danger)' : 'var(--success)' }">
                {{ formatSize(p.free_gb) }} libres
              </span>
              <span v-if="p.is_system" class="badge-sys">SYSTÈME</span>
              <span v-if="p.is_boot" class="badge-boot">BOOT</span>
            </div>
          </div>
        </div>
      </div>

    </template>
  </div>
</template>

<style scoped>
.clone-page { display: flex; flex-direction: column; gap: 16px; }

/* Hero */
.clone-hero {
  display: flex; align-items: center; gap: 16px;
  padding: 20px 24px;
  background: linear-gradient(135deg, var(--bg-secondary) 0%, color-mix(in srgb, var(--accent-primary) 6%, var(--bg-secondary)) 100%);
  border: 1px solid var(--border); border-radius: var(--radius-xl);
  position: relative; overflow: hidden;
}
.clone-hero::before {
  content: ''; position: absolute; top: -40px; right: -40px;
  width: 160px; height: 160px; border-radius: 50%;
  background: radial-gradient(circle, color-mix(in srgb, var(--accent-primary) 12%, transparent), transparent 70%);
  pointer-events: none;
}
.hero-icon-wrap { flex-shrink: 0; }
.hero-icon {
  width: 52px; height: 52px; border-radius: var(--radius-lg);
  background: linear-gradient(135deg, var(--accent-primary), var(--accent-hover));
  display: flex; align-items: center; justify-content: center; color: white;
  box-shadow: 0 4px 16px color-mix(in srgb, var(--accent-primary) 40%, transparent);
  animation: float 3s ease-in-out infinite;
}
@keyframes float { 0%,100%{transform:translateY(0)} 50%{transform:translateY(-4px)} }
.hero-text { flex: 1; }
.hero-title { font-size: 22px; font-weight: 800; color: var(--text-primary); }
.hero-desc { font-size: 13px; color: var(--text-secondary); margin-top: 4px; }

/* Prérequis */
.prereq-card {
  padding: 14px 16px; background: var(--bg-secondary); border: 1px solid var(--border);
  border-radius: var(--radius-lg); border-left: 3px solid var(--info);
}
.prereq-title { font-size: 12px; font-weight: 700; color: var(--info); text-transform: uppercase; letter-spacing: .05em; display: flex; align-items: center; gap: 6px; margin-bottom: 10px; }
.prereq-list { display: flex; flex-direction: column; gap: 6px; }
.prereq-item { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--text-secondary); }

/* Méthodes */
.methods-row { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px; }
@media (max-width: 700px) { .methods-row { grid-template-columns: 1fr; } }
.method-card {
  display: flex; align-items: center; gap: 14px; padding: 14px 16px;
  border: 1.5px solid var(--border); border-radius: var(--radius-xl);
  background: var(--bg-secondary); cursor: pointer;
  transition: all 0.2s; position: relative; overflow: hidden;
}
.method-card:hover { border-color: var(--text-muted); background: var(--bg-tertiary); transform: translateY(-1px); }
.method-card.selected { border-color: var(--accent-primary); background: color-mix(in srgb, var(--accent-primary) 5%, var(--bg-secondary)); }
.method-card.selected::after {
  content: ''; position: absolute; top: 0; left: 0; right: 0; height: 2px;
  background: linear-gradient(90deg, var(--accent-primary), var(--accent-hover));
}
.method-icon {
  width: 44px; height: 44px; border-radius: var(--radius-lg);
  display: flex; align-items: center; justify-content: center; flex-shrink: 0; color: white;
}
.method-icon-blue { background: linear-gradient(135deg, #3b82f6, #2563eb); box-shadow: 0 4px 12px rgba(59,130,246,.35); }
.method-icon-orange { background: linear-gradient(135deg, #f97316, #ea580c); box-shadow: 0 4px 12px rgba(249,115,22,.35); }
.method-icon-green  { background: linear-gradient(135deg, #22c55e, #16a34a); box-shadow: 0 4px 12px rgba(34,197,94,.35); }
.method-info { flex: 1; display: flex; flex-direction: column; gap: 3px; }
.method-name { font-size: 14px; font-weight: 700; color: var(--text-primary); display: flex; align-items: center; gap: 8px; }
.method-badge { font-size: 10px; font-weight: 700; padding: 1px 6px; border-radius: 99px; background: var(--success-muted); color: var(--success); }
.method-desc { font-size: 12px; color: var(--text-secondary); }
.method-arrow { color: var(--text-muted); transition: color 0.15s, transform 0.15s; }
.method-arrow.active { color: var(--accent-primary); transform: translateX(3px); }

/* Toolbar */
.toolbar { display: flex; align-items: center; gap: 10px; }
.count-badge { font-size: 11px; color: var(--text-muted); font-family: monospace; }

/* Info banners */
.info-banner {
  display: flex; gap: 10px; align-items: flex-start; padding: 12px 16px;
  border-radius: var(--radius-lg); font-size: 12px; line-height: 1.6;
}
.info-banner.info    { background: var(--info-muted);    color: var(--info);    border: 1px solid color-mix(in srgb, var(--info) 30%, transparent); }
.info-banner.warning { background: var(--warning-muted); color: var(--warning); border: 1px solid color-mix(in srgb, var(--warning) 30%, transparent); }

/* Config */
.config-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
@media (max-width: 700px) { .config-grid { grid-template-columns: 1fr; } }
.config-card {
  display: flex; flex-direction: column; gap: 10px; padding: 16px;
  background: var(--bg-secondary); border-radius: var(--radius-xl);
  border: 1px solid var(--border); transition: border-color 0.15s;
}
.config-card:hover { border-color: var(--text-muted); }
.config-label { font-size: 12px; font-weight: 700; color: var(--text-secondary); display: flex; align-items: center; gap: 6px; }
.drive-badge {
  display: flex; align-items: center; gap: 10px; padding: 12px 14px;
  background: var(--bg-tertiary); border-radius: var(--radius-md);
  border: 1px solid var(--border); font-size: 13px;
}
.drive-badge.system { border-color: var(--accent-primary); color: var(--accent-primary); }
.badge-sub { display: block; font-size: 11px; color: var(--text-muted); margin-top: 2px; }
.drive-select {
  padding: 9px 12px; border: 1px solid var(--border); border-radius: var(--radius-md);
  background: var(--bg-tertiary); color: var(--text-primary); font-family: inherit; font-size: 12px;
  outline: none; cursor: pointer; transition: border-color 0.15s;
}
.drive-select:focus { border-color: var(--accent-primary); }
.config-hint { font-size: 11px; color: var(--warning); }

/* Confirm */
.confirm-row { padding: 8px 0; }
.confirm-label { display: flex; align-items: center; gap: 10px; font-size: 13px; color: var(--text-secondary); cursor: pointer; }
.confirm-label input { cursor: pointer; accent-color: var(--accent-primary); width: 15px; height: 15px; }

/* Progression */
.progress-section { display: flex; flex-direction: column; gap: 12px; padding: 16px; background: var(--bg-secondary); border-radius: var(--radius-xl); border: 1px solid var(--border); }
.step-bar { display: flex; align-items: center; gap: 0; }
.step-item { display: flex; flex-direction: column; align-items: center; gap: 4px; flex: 1; position: relative; }
.step-item:not(:last-child)::after { content:''; position: absolute; top: 14px; left: 50%; width: 100%; height: 2px; background: var(--border); z-index: 0; }
.step-item.done::after { background: var(--success); }
.step-item.active::after { background: linear-gradient(90deg, var(--accent-primary), var(--border)); }
.step-dot {
  width: 28px; height: 28px; border-radius: 50%; display: flex; align-items: center; justify-content: center;
  font-size: 11px; font-weight: 700; border: 2px solid var(--border); background: var(--bg-tertiary);
  color: var(--text-muted); z-index: 1; transition: all 0.3s;
}
.step-item.done .step-dot { background: var(--success); border-color: var(--success); color: white; }
.step-item.active .step-dot { background: var(--accent-primary); border-color: var(--accent-primary); color: white; animation: pulse-dot 1s ease-in-out infinite; }
@keyframes pulse-dot { 0%,100%{box-shadow:0 0 0 0 color-mix(in srgb,var(--accent-primary) 40%,transparent)} 50%{box-shadow:0 0 0 6px transparent} }
.step-label { font-size: 10px; color: var(--text-muted); text-align: center; }
.step-item.done .step-label, .step-item.active .step-label { color: var(--text-primary); }
.progress-header { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--text-secondary); }
.progress-note { font-size: 11px; color: var(--text-muted); }

/* Résultat */
.result-card { display: flex; align-items: flex-start; gap: 14px; padding: 16px 20px; border-radius: var(--radius-xl); border: 1px solid; }
.result-card.success { background: var(--success-muted); border-color: var(--success); color: var(--success); }
.result-card.error { background: var(--danger-muted); border-color: var(--danger); color: var(--danger); }
.result-title { font-weight: 800; font-size: 15px; }
.result-method, .result-msg, .result-duration { font-size: 12px; color: var(--text-secondary); margin-top: 4px; }

/* Bonnes pratiques */
.checklist-card {
  padding: 14px 16px; background: var(--bg-secondary); border: 1px solid var(--border);
  border-radius: var(--radius-lg); border-left: 3px solid var(--warning);
}
.checklist-title { font-size: 12px; font-weight: 700; color: var(--warning); text-transform: uppercase; letter-spacing: .05em; margin-bottom: 10px; }
.checklist { display: flex; flex-direction: column; gap: 6px; }
.checklist-item { font-size: 12px; color: var(--text-secondary); display: flex; align-items: flex-start; gap: 6px; }

/* Disques */
.disks-section { display: flex; flex-direction: column; gap: 10px; }
.section-label { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: .06em; color: var(--text-muted); }
.disk-card { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius-xl); overflow: hidden; transition: border-color 0.15s; }
.disk-card:hover { border-color: var(--text-muted); }
.disk-header { display: flex; align-items: center; gap: 12px; padding: 12px 16px; background: var(--bg-tertiary); }
.disk-icon { width: 34px; height: 34px; border-radius: var(--radius-md); background: var(--accent-muted); display: flex; align-items: center; justify-content: center; color: var(--accent-primary); flex-shrink: 0; }
.disk-info { flex: 1; display: flex; flex-direction: column; gap: 2px; }
.disk-name { font-size: 13px; font-weight: 700; color: var(--text-primary); }
.disk-meta { font-size: 11px; color: var(--text-muted); font-family: monospace; }
.partitions-list { padding: 0; }
.part-row { display: grid; grid-template-columns: 40px 1fr 60px 70px 80px 90px auto auto; align-items: center; gap: 8px; padding: 7px 16px; border-top: 1px solid var(--border); font-size: 12px; }
.part-letter { font-family: monospace; font-weight: 700; color: var(--accent-primary); }
.part-label { color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.part-fs { color: var(--text-muted); }
.part-size { color: var(--text-secondary); font-family: monospace; text-align: right; }
.part-bar-wrap { min-width: 60px; }
.part-bar-track { height: 5px; border-radius: 99px; background: var(--bg-elevated); border: 1px solid var(--border); overflow: hidden; }
.part-bar-fill { height: 100%; border-radius: 99px; transition: width 0.5s; }
.part-free { font-family: monospace; font-size: 11px; text-align: right; }
.badge-sys { font-size: 10px; font-weight: 700; padding: 2px 6px; border-radius: 4px; background: var(--accent-muted); color: var(--accent-primary); }
.badge-boot { font-size: 10px; font-weight: 700; padding: 2px 6px; border-radius: 4px; background: var(--warning-muted); color: var(--warning); }

.loading-state { display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 40px; color: var(--text-muted); font-size: 13px; }

.preflight-list { display: flex; flex-direction: column; gap: 5px; padding: 10px 14px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius-lg); }
.preflight-item { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--text-secondary); }
.preflight-item.ok  { color: var(--success); }
.preflight-item.nok { color: var(--danger); }
.preflight-item.na  { color: var(--text-muted); opacity: 0.6; }
.pf-dot { font-weight: 700; font-size: 13px; width: 16px; text-align: center; flex-shrink: 0; }

.elapsed-badge { margin-left: auto; font-size: 11px; color: var(--accent-primary); font-family: monospace; background: var(--accent-muted); padding: 2px 8px; border-radius: 99px; }
.speed-badge { font-size: 11px; color: var(--info); font-family: monospace; background: var(--info-muted); padding: 2px 8px; border-radius: 99px; }
.eta-badge { font-size: 11px; color: var(--warning); font-family: monospace; background: var(--warning-muted); padding: 2px 8px; border-radius: 99px; }
.validation-error { display: flex; align-items: center; gap: 8px; padding: 10px 14px; background: var(--danger-muted); border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent); border-radius: var(--radius-md); font-size: 12px; color: var(--danger); }
</style>
