<script setup lang="ts">
defineOptions({ name: "MasterInstallPage" });
import { ref, computed, onMounted, type Component } from "vue";
import { invoke, invokeRaw } from "@/utils/invoke";
import { confirm } from "@tauri-apps/plugin-dialog";
import { cachedInvoke } from "@/composables/useCachedInvoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NProgress from "@/components/ui/NProgress.vue";
import NSearchBar from "@/components/ui/NSearchBar.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NTabs from "@/components/ui/NTabs.vue";
import NModal from "@/components/ui/NModal.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import { useNotificationStore } from "@/stores/notifications";
import {
  Download, CheckSquare, Square, Package,
  Globe, Shield, Code, Image, MessageSquare,
  FileText, Music, Video, Wrench,
  Cpu, Monitor, Printer, Archive,
  Bot, Users, Cloud, Star, Lock, Play,
  ChevronDown, ChevronRight, Layers, FileCode, Eye,
  RotateCcw, Trash2,
} from "lucide-vue-next";

const notifications = useNotificationStore();
const search = ref("");
const installing = ref(false);
const exportingScript = ref(false);

// ── Dry run ────────────────────────────────────────────────────
const showDryRun = ref(false);
const dryRunApps = computed(() => apps.value.filter(a => a.checked && !a.installed));

// ── Résumé installation ────────────────────────────────────────
interface InstallResult { name: string; success: boolean; message: string; manualUrl?: string }
const showSummary = ref(false);
const installResults = ref<InstallResult[]>([]);

// Profils prédéfinis
interface Profile { id: string; label: string; icon: Component; color: string; wingetIds: string[] }
const PROFILES: Profile[] = [
  { id: "essential", label: "Essentiels", icon: Star, color: "#f97316",
    wingetIds: ["7zip.7zip", "Google.Chrome", "Mozilla.Firefox", "Notepad++.Notepad++", "VideoLAN.VLC", "Microsoft.PowerShell"] },
  { id: "office", label: "Bureau", icon: FileText, color: "#3b82f6",
    wingetIds: ["Microsoft.Office", "Adobe.Acrobat.Reader.64-bit", "TheDocumentFoundation.LibreOffice", "Zoom.Zoom", "Microsoft.Teams"] },
  { id: "dev", label: "Dev", icon: Code, color: "#22c55e",
    wingetIds: ["Microsoft.VisualStudioCode", "Git.Git", "Python.Python.3.12", "OpenJS.NodeJS", "JetBrains.IntelliJIDEA.Community", "Docker.DockerDesktop"] },
  { id: "gaming", label: "Gaming", icon: Play, color: "#a855f7",
    wingetIds: ["Valve.Steam", "Discord.Discord", "EpicGames.EpicGamesLauncher", "Nvidia.GeForceExperience", "Parsec.Parsec"] },
  { id: "security", label: "Sécurité", icon: Shield, color: "#ef4444",
    wingetIds: ["Malwarebytes.Malwarebytes", "WiresharkFoundation.Wireshark", "KeePassXCTeam.KeePassXC", "Bitwarden.Bitwarden"] },
  { id: "creative", label: "Créatif", icon: Image, color: "#ec4899",
    wingetIds: ["Inkscape.Inkscape", "GIMP.GIMP", "HandBrake.HandBrake", "OBSProject.OBSStudio", "Audacity.Audacity", "Blender.Blender"] },
];
const currentApp = ref("");
const installProgress = ref(0);
const installTotal = ref(0);
const installIndex = ref(0);
const installStartTime = ref<number | null>(null);
const installEtaLabel = ref("");

function formatEta(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "";
  const m = Math.floor(seconds / 60);
  const s = Math.round(seconds % 60);
  return m > 0 ? `~${m} min ${s}s restantes` : `~${s}s restantes`;
}
const collapsedCategories = ref<Set<string>>(new Set());

interface AppItem {
  id: string;
  name: string;
  description: string;
  category: string;
  winget_id?: string | null;
  choco_id?: string | null;
  url?: string | null;
  icon?: string;
  checked: boolean;
  installed: boolean;
}

const apps = ref<AppItem[]>([]);

// Categories avec icônes — correspondent aux valeurs exactes de programs.json
const CATEGORIES: { id: string; label: string; icon: Component }[] = [
  { id: "Outils Essentiels", label: "Outils Essentiels", icon: Star },
  { id: "Navigateurs", label: "Navigateurs", icon: Globe },
  { id: "Securite", label: "Sécurité", icon: Shield },
  { id: "Antivirus", label: "Antivirus", icon: Shield },
  { id: "Desinstallateurs Antivirus", label: "Désinstallateurs Antivirus", icon: Lock },
  { id: "Developpement", label: "Développement", icon: Code },
  { id: "Multimedia", label: "Multimédia", icon: Video },
  { id: "Streaming Video", label: "Streaming Vidéo", icon: Play },
  { id: "Streaming Audio", label: "Streaming Audio", icon: Music },
  { id: "Communication", label: "Communication", icon: MessageSquare },
  { id: "Reseaux Sociaux", label: "Réseaux Sociaux", icon: Users },
  { id: "Bureautique", label: "Bureautique", icon: FileText },
  { id: "PDF et Documents", label: "PDF & Documents", icon: FileText },
  { id: "Suites Professionnelles", label: "Suites Pro", icon: Cpu },
  { id: "Productivite", label: "Productivité", icon: CheckSquare },
  { id: "IA & Assistants", label: "IA & Assistants", icon: Bot },
  { id: "Utilitaires", label: "Utilitaires", icon: Wrench },
  { id: "Utilitaires Systeme", label: "Utilitaires Système", icon: Monitor },
  { id: "Stockage Cloud", label: "Stockage Cloud", icon: Cloud },
  { id: "Compression", label: "Compression", icon: Archive },
  { id: "Internet", label: "Internet", icon: Globe },
  { id: "Jeux", label: "Jeux", icon: Play },
  { id: "Imprimantes & Scan", label: "Imprimantes & Scan", icon: Printer },
  { id: "Services Apple", label: "Services Apple", icon: Package },
];

const categoryTabs = [
  { id: "all", label: "Tout" },
  ...CATEGORIES.map(c => ({ id: c.id, label: c.label })),
];

const activeCategory = ref("all");

function normalizeStr(s: string) {
  return s.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "");
}

const filteredApps = computed(() => {
  const q = search.value.toLowerCase();
  return apps.value.filter((a) => {
    const matchSearch = !q || a.name.toLowerCase().includes(q) || a.description.toLowerCase().includes(q);
    const matchCat = activeCategory.value === "all" || normalizeStr(a.category) === normalizeStr(activeCategory.value);
    return matchSearch && matchCat;
  });
});

const groupedApps = computed(() => {
  const groups: Record<string, AppItem[]> = {};
  const cats = activeCategory.value === "all"
    ? CATEGORIES.map(c => c.id)
    : [activeCategory.value];

  for (const catId of cats) {
    const items = filteredApps.value.filter((a) => normalizeStr(a.category) === normalizeStr(catId));
    if (items.length > 0) groups[catId] = items;
  }
  return groups;
});

const totalCount = computed(() => apps.value.length);
const selectedCount = computed(() => apps.value.filter((a) => a.checked).length);

function selectAll() {
  filteredApps.value.forEach((a) => (a.checked = true));
}

function deselectAll() {
  apps.value.forEach((a) => (a.checked = false));
}

function toggleApp(app: AppItem) {
  if (!app.installed) app.checked = !app.checked;
}

function toggleCategory(catId: string) {
  if (collapsedCategories.value.has(catId)) {
    collapsedCategories.value.delete(catId);
  } else {
    collapsedCategories.value.add(catId);
  }
  collapsedCategories.value = new Set(collapsedCategories.value);
}

function getCategoryInfo(id: string) {
  return CATEGORIES.find((c) => c.id === id) ?? { id, label: id, icon: Package };
}

function selectCategory(catId: string) {
  const catApps = apps.value.filter((a) => a.category === catId && !a.installed);
  const allChecked = catApps.every((a) => a.checked);
  catApps.forEach((a) => (a.checked = !allChecked));
}

function applyProfile(profile: Profile) {
  // Sélectionner les apps du profil qui ne sont pas encore installées
  let matched = 0;
  apps.value.forEach((a) => {
    if (profile.wingetIds.includes(a.winget_id ?? "") && !a.installed) {
      a.checked = true;
      matched++;
    }
  });
  if (matched === 0) notifications.warning(`Profil "${profile.label}" : aucune app correspondante trouvée`);
  else notifications.success(`Profil "${profile.label}" : ${matched} app(s) sélectionnée(s)`);
}

function allCatChecked(catId: string): boolean {
  const catApps = apps.value.filter(a => a.category === catId && !a.installed);
  return catApps.length > 0 && catApps.every(a => a.checked);
}

function toggleSelectCategory(catId: string) {
  const catApps = apps.value.filter(a => a.category === catId && !a.installed);
  const allChecked = catApps.every(a => a.checked);
  catApps.forEach(a => (a.checked = !allChecked));
}

async function exportDeployScript() {
  const selected = apps.value.filter((a) => a.checked && !a.installed && a.winget_id);
  if (!selected.length) { notifications.warning("Aucune app avec WinGet ID sélectionnée"); return; }
  exportingScript.value = true;
  const lines = [
    "@echo off",
    ":: Script de déploiement généré par NiTriTe",
    `:: ${new Date().toLocaleString("fr-FR")}`,
    "",
    ":: Vérification des droits administrateur",
    "NET SESSION >nul 2>&1",
    "IF %ERRORLEVEL% NEQ 0 (",
    "    echo ERREUR : Ce script doit etre execute en tant qu'administrateur.",
    "    echo Clic droit sur le fichier ^> Executer en tant qu'administrateur.",
    "    pause",
    "    exit /b 1",
    ")",
    "",
    "echo === Installation des logiciels ===",
    "",
  ];
  for (const app of selected) {
    lines.push(`echo Installation de ${app.name}...`);
    lines.push(`winget install --id ${app.winget_id} --silent --accept-package-agreements --accept-source-agreements`);
    lines.push("");
  }
  lines.push("echo === Terminé ===", "pause");
  const content = lines.join("\r\n");
  try {
    await invoke("save_export_file", { filename: "deploy_nitrite.bat", content });
    notifications.success("Script exporté", "deploy_nitrite.bat");
  } catch {
    try {
      await navigator.clipboard.writeText(content);
      notifications.info("Script copié dans le presse-papier");
    } catch { notifications.error("Export échoué"); }
  }
  exportingScript.value = false;
}

async function installSelection() {
  const selected = apps.value.filter((a) => a.checked && !a.installed);
  if (selected.length === 0) {
    notifications.warning("Aucune application sélectionnée");
    return;
  }

  installing.value = true;
  installTotal.value = selected.length;
  installIndex.value = 0;
  installResults.value = [];
  installStartTime.value = Date.now();
  installEtaLabel.value = "";

  for (const app of selected) {
    installIndex.value++;
    currentApp.value = app.name;
    installProgress.value = Math.round((installIndex.value / installTotal.value) * 100);
    if (installIndex.value > 1 && installStartTime.value) {
      const elapsedSec = (Date.now() - installStartTime.value) / 1000;
      const avgPerApp = elapsedSec / (installIndex.value - 1);
      const remaining = installTotal.value - installIndex.value + 1;
      installEtaLabel.value = formatEta(avgPerApp * remaining);
    }

    try {
      // install_app tente en cascade winget -> chocolatey -> scoop ->
      // telechargement direct (backend), pas seulement winget — couvre les
      // apps du catalogue sans winget_id (~48%, Office retail, portables...).
      // invokeRaw (pas de timeout 30s) : le bootstrap Chocolatey/Scoop seul
      // peut prendre plus d'une minute la premiere fois.
      const result = await invokeRaw<{ success: boolean; app_id: string; message: string }>("install_app", {
        appId: app.id,
        wingetId: app.winget_id ?? undefined,
      });
      installResults.value.push({ name: app.name, success: result.success, message: result.message, manualUrl: !result.success ? (app.url ?? undefined) : undefined });
      if (!result.success) {
        notifications.warning(`${app.name}: ${result.message}`);
      } else {
        notifications.success(`${app.name} installé`);
        // Ne marquer installé/décoché qu'en cas de succès réel : sinon une
        // installation échouée disparaît de la liste et ne peut plus être relancée.
        app.installed = true;
        app.checked = false;
      }
    } catch (e: unknown) {
      const errMsg = (e instanceof Error ? e.message : String(e)).slice(0, 120);
      installResults.value.push({ name: app.name, success: false, message: errMsg });
      notifications.error(`Échec: ${app.name}`, errMsg);
    }
  }

  installing.value = false;
  currentApp.value = "";
  installProgress.value = 0;
  installEtaLabel.value = "";
  installStartTime.value = null;
  showSummary.value = true;
}

// ── Vérifier MAJ / Désinstaller (par app) ───────────────────────
const checkingUpdateIds = ref<Set<string>>(new Set());
const uninstallingIds = ref<Set<string>>(new Set());

async function checkAppUpdate(app: AppItem) {
  if (!app.winget_id) { notifications.warning(`Vérification indisponible pour ${app.name} (pas d'ID WinGet)`); return; }
  checkingUpdateIds.value = new Set([...checkingUpdateIds.value, app.id]);
  try {
    const result = await invoke<{ stdout?: string }>("run_system_command", {
      cmd: "winget",
      args: ["upgrade", "--id", app.winget_id],
    });
    const out = result?.stdout ?? "";
    const low = out.toLowerCase();
    // winget renvoie un exit code non-zéro même quand l'app est déjà à jour
    // (quirk winget confirmé en direct sur cette machine : "upgrade --id Git.Git"
    // à jour → exit -1978335189) — seul le texte distingue les 3 issues réelles.
    // Sur Windows FR, winget répond "Mise à niveau disponible introuvable." /
    // "Aucune version de package plus récente n'est disponible...", pas la string
    // anglaise "No applicable upgrade" attendue jusqu'ici (jamais présente en FR
    // → cette branche succès n'était jamais atteinte sur un poste francophone).
    const upToDate = out.includes("No applicable upgrade")
      || low.includes("disponible introuvable")
      || low.includes("n'est disponible à partir des sources");
    const notFound = low.includes("ne correspond aux critères") || low.includes("no installed package found");
    if (upToDate) {
      notifications.success(`${app.name} est à jour`);
    } else if (notFound) {
      notifications.error(`${app.name} introuvable via WinGet`, "L'ID WinGet ne correspond à aucun package installé.");
    } else if (out.trim()) {
      notifications.info(`MAJ disponible pour ${app.name}`, out.split("\n").slice(0, 3).join(" "));
    } else {
      notifications.info(`Vérification terminée pour ${app.name}`);
    }
  } catch (e: unknown) {
    notifications.error(`Impossible de vérifier MAJ pour ${app.name}`, String(e));
  }
  checkingUpdateIds.value = new Set([...checkingUpdateIds.value].filter((id) => id !== app.id));
}

async function uninstallApp(app: AppItem) {
  const confirmed = await confirm(`Désinstaller ${app.name} ?\n\nCette action supprimera l'application de votre système.`, { title: "Nitrite", kind: "warning" });
  if (!confirmed) return;
  uninstallingIds.value = new Set([...uninstallingIds.value, app.id]);
  try {
    // Même cascade que l'install (winget -> chocolatey -> scoop), vérifiée
    // pour de vrai côté backend (le registre ne doit plus contenir l'app).
    const result = await invokeRaw<{ success: boolean; message: string }>("uninstall_app", {
      appId: app.id,
      wingetId: app.winget_id ?? undefined,
    });
    if (!result.success) throw new Error(result.message);
    app.installed = false;
    notifications.success(`${app.name} désinstallé`);
  } catch (e: unknown) {
    notifications.error(`Erreur désinstallation ${app.name}`, String(e));
  }
  uninstallingIds.value = new Set([...uninstallingIds.value].filter((id) => id !== app.id));
}

onMounted(async () => {
  try {
    const result = await cachedInvoke<Omit<AppItem, 'checked' | 'installed'>[]>("get_apps");
    apps.value = result.map((a) => ({ ...a, checked: false, installed: false }));
  } catch {
    notifications.warning("Impossible de charger la base de données");
  }
});
</script>

<template>
  <div class="master-install">
    <!-- Header -->
    <div class="page-header">
      <div>
        <h1>Master Install</h1>
        <p class="page-subtitle">
          Base de données de <strong>{{ totalCount }}</strong> applications — installation groupée via WinGet
        </p>
      </div>
      <div class="header-actions">
        <NButton variant="ghost" size="sm" @click="selectAll">
          <CheckSquare :size="14" />
          Tout sélectionner
        </NButton>
        <NButton variant="ghost" size="sm" @click="deselectAll">
          <Square :size="14" />
          Tout déselectionner
        </NButton>
        <NButton
          variant="ghost"
          size="sm"
          :disabled="selectedCount === 0"
          @click="showDryRun = true"
        >
          <Eye :size="14" />
          Prévisualiser
        </NButton>
        <NButton
          variant="ghost"
          size="sm"
          :loading="exportingScript"
          :disabled="selectedCount === 0"
          @click="exportDeployScript"
        >
          <FileCode :size="14" />
          Export .bat
        </NButton>
        <NButton
          variant="primary"
          size="sm"
          :loading="installing"
          :disabled="selectedCount === 0"
          @click="installSelection"
        >
          <Download :size="14" />
          Installer ({{ selectedCount }})
        </NButton>
      </div>
    </div>

    <!-- Profils prédéfinis -->
    <NCard>
      <template #header>
        <div style="display:flex;align-items:center;gap:8px">
          <Layers :size="16" />
          <span>Profils Prédéfinis</span>
          <span style="font-size:12px;color:var(--text-muted);margin-left:4px">— Sélection rapide par usage</span>
        </div>
      </template>
      <div class="profiles-grid">
        <button
          v-for="profile in PROFILES"
          :key="profile.id"
          class="profile-card"
          :style="{ '--p-color': profile.color }"
          @click="applyProfile(profile)"
        >
          <component :is="profile.icon" :size="20" :style="{ color: profile.color }" />
          <span class="profile-label">{{ profile.label }}</span>
          <span class="profile-count">{{ profile.wingetIds.length }} apps</span>
        </button>
      </div>
    </NCard>

    <!-- Search -->
    <NSearchBar v-model="search" placeholder="Rechercher une application..." />

    <!-- Barre progression globale visible pendant install -->
    <NCard v-if="installing" class="progress-card">
      <div class="install-progress-global">
        <div class="install-status-row">
          <NSpinner :size="14" />
          <span class="install-label">
            Installation de <strong>{{ currentApp }}</strong>
          </span>
          <NBadge variant="info">{{ installIndex }}/{{ installTotal }}</NBadge>
          <NBadge v-if="installEtaLabel" variant="neutral">{{ installEtaLabel }}</NBadge>
        </div>
        <NProgress :value="installProgress" :max="100" size="lg" :show-label="true" :glow="true" />
      </div>
    </NCard>

    <!-- Category Tabs -->
    <NTabs :tabs="categoryTabs" v-model="activeCategory" wrap>
      <template #default>
        <div v-if="Object.keys(groupedApps).length === 0" class="empty-state">
          <Package :size="40" class="empty-icon" />
          <p>Aucune application trouvée</p>
        </div>

        <div v-else class="categories-list">
          <template v-for="(catApps, catId) in groupedApps" :key="catId">
            <NCard>
              <template #header>
                <div class="section-header" @click="toggleCategory(catId as string)">
                  <component :is="getCategoryInfo(catId as string).icon" :size="16" />
                  <span>{{ getCategoryInfo(catId as string).label }}</span>
                  <NBadge variant="neutral">{{ catApps.length }}</NBadge>
                  <span class="spacer" />
                  <button class="select-cat-btn" @click.stop="toggleSelectCategory(catId as string)">
                    {{ allCatChecked(catId as string) ? 'Tout déselectionner' : 'Tout sélectionner' }}
                  </button>
                  <component
                    :is="collapsedCategories.has(catId as string) ? ChevronRight : ChevronDown"
                    :size="14"
                    class="collapse-icon"
                  />
                </div>
              </template>

              <div v-if="!collapsedCategories.has(catId as string)" class="apps-grid">
                <div
                  v-for="app in catApps"
                  :key="app.id"
                  class="app-item"
                  :class="{ 'app-item--checked': app.checked, 'app-item--installed': app.installed }"
                  @click="toggleApp(app)"
                >
                  <div class="app-checkbox">
                    <CheckSquare v-if="app.checked || app.installed" :size="18" class="check-on" />
                    <Square v-else :size="18" class="check-off" />
                  </div>
                  <div class="app-info">
                    <span class="app-name">{{ app.name }}</span>
                    <span class="app-desc">{{ app.description }}</span>
                  </div>
                  <template v-if="app.installed">
                    <NBadge variant="success">Installé</NBadge>
                    <button
                      class="app-action-btn"
                      title="Vérifier MAJ"
                      :disabled="checkingUpdateIds.has(app.id)"
                      @click.stop="checkAppUpdate(app)"
                    >
                      <NSpinner v-if="checkingUpdateIds.has(app.id)" :size="12" />
                      <RotateCcw v-else :size="12" />
                    </button>
                    <button
                      class="app-action-btn app-action-btn--danger"
                      title="Désinstaller"
                      :disabled="uninstallingIds.has(app.id)"
                      @click.stop="uninstallApp(app)"
                    >
                      <NSpinner v-if="uninstallingIds.has(app.id)" :size="12" />
                      <Trash2 v-else :size="12" />
                    </button>
                  </template>
                  <NBadge v-else-if="app.winget_id" variant="info" class="winget-badge">WinGet</NBadge>
                  <NBadge v-else-if="app.url" variant="warning" class="winget-badge">URL</NBadge>
                </div>
              </div>
            </NCard>
          </template>
        </div>
      </template>
    </NTabs>
  </div>

  <!-- Modal Dry Run -->
  <NModal :open="showDryRun" @close="showDryRun = false" title="Prévisualisation — Apps sélectionnées">
    <div v-if="dryRunApps.length === 0" style="text-align:center;padding:24px;color:var(--text-muted);font-size:13px">
      Aucune application sélectionnée.
    </div>
    <div v-else style="display:flex;flex-direction:column;gap:6px;max-height:420px;overflow-y:auto">
      <div v-for="app in dryRunApps" :key="app.id" class="dryrun-item">
        <div class="dryrun-name">{{ app.name }}</div>
        <code v-if="app.winget_id" class="dryrun-cmd">
          winget install --id {{ app.winget_id }} --silent ...
        </code>
        <span v-else class="dryrun-nowinget">Pas de WinGet ID — sera ignoré</span>
      </div>
    </div>
    <template #footer>
      <NBadge variant="neutral" style="margin-right:auto">{{ dryRunApps.length }} app(s)</NBadge>
      <NButton variant="ghost" @click="showDryRun = false">Fermer</NButton>
    </template>
  </NModal>

  <!-- Modal Résumé installation -->
  <NModal :open="showSummary" @close="showSummary = false" title="Résumé de l'installation">
    <div style="display:flex;flex-direction:column;gap:6px;max-height:420px;overflow-y:auto">
      <div v-for="r in installResults" :key="r.name" class="summary-item" :class="r.success ? 'summary-ok' : 'summary-fail'">
        <span class="summary-status">{{ r.success ? '✓' : '✗' }}</span>
        <div class="summary-info">
          <span class="summary-name">{{ r.name }}</span>
          <span v-if="!r.success" class="summary-msg">{{ r.message }}</span>
        </div>
        <a v-if="r.manualUrl" :href="r.manualUrl" target="_blank" class="summary-manual-link">Télécharger</a>
      </div>
    </div>
    <template #footer>
      <NBadge variant="success" style="margin-right:auto">
        {{ installResults.filter(r => r.success).length }} succès
      </NBadge>
      <NBadge v-if="installResults.some(r => !r.success)" variant="danger">
        {{ installResults.filter(r => !r.success).length }} échec(s)
      </NBadge>
      <NButton variant="primary" @click="showSummary = false" style="margin-left:8px">Fermer</NButton>
    </template>
  </NModal>
</template>

<style scoped>
.master-install {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 12px;
}

.page-header h1 { font-size: 24px; font-weight: 700; }
.page-subtitle { color: var(--text-muted); font-size: 13px; margin-top: 2px; }
.header-actions { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }

.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  user-select: none;
}

.spacer { flex: 1; }

.select-cat-btn {
  font-size: 11px;
  color: var(--accent-primary);
  background: none;
  border: none;
  cursor: pointer;
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  transition: background var(--transition-fast);
}
.select-cat-btn:hover { background: var(--accent-muted); }

.collapse-icon { color: var(--text-muted); }

.install-progress {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.install-status {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-secondary);
}

.spin-icon { animation: spin 1s linear infinite; }

@keyframes spin { to { transform: rotate(360deg); } }

.categories-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.apps-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 4px;
}

.app-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.app-item:hover { background: var(--bg-tertiary); }
.app-item--checked { background: var(--accent-muted); }
.app-item--installed { cursor: default; }
.app-item--installed .app-name, .app-item--installed .app-desc { opacity: 0.6; }

.app-checkbox { flex-shrink: 0; display: flex; }
.check-on { color: var(--accent-primary); }
.check-off { color: var(--text-muted); }

.app-info { display: flex; flex-direction: column; gap: 1px; flex: 1; min-width: 0; }

.app-name {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.app-desc {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.winget-badge { flex-shrink: 0; font-size: 10px; }

.app-action-btn {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.app-action-btn:hover { background: var(--bg-tertiary); color: var(--text-primary); }
.app-action-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.app-action-btn--danger:hover { color: var(--danger); border-color: var(--danger); }

.empty-state {
  text-align: center;
  padding: 48px;
  color: var(--text-muted);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.empty-icon { opacity: 0.3; }

/* ── Progression globale ──────────────────────────── */
.progress-card { border-color: var(--accent-muted); }
.install-progress-global { display: flex; flex-direction: column; gap: 10px; }
.install-status-row { display: flex; align-items: center; gap: 8px; font-size: 13px; }
.install-label { flex: 1; color: var(--text-secondary); }

/* ── Dry run ──────────────────────────────────────── */
.dryrun-item { padding: 8px 10px; border: 1px solid var(--border); border-radius: var(--radius-md); background: var(--bg-tertiary); }
.dryrun-name { font-size: 12px; font-weight: 600; color: var(--text-primary); margin-bottom: 4px; }
.dryrun-cmd { font-size: 11px; color: var(--accent-primary); font-family: "JetBrains Mono", monospace; display: block; white-space: pre-wrap; word-break: break-all; }
.dryrun-nowinget { font-size: 11px; color: var(--text-muted); font-style: italic; }

/* ── Résumé ───────────────────────────────────────── */
.summary-item { display: flex; align-items: flex-start; gap: 10px; padding: 8px 10px; border-radius: var(--radius-md); border: 1px solid var(--border); }
.summary-ok { background: color-mix(in srgb, var(--success) 8%, var(--bg-secondary)); border-color: color-mix(in srgb, var(--success) 30%, var(--border)); }
.summary-fail { background: color-mix(in srgb, var(--danger) 8%, var(--bg-secondary)); border-color: color-mix(in srgb, var(--danger) 30%, var(--border)); }
.summary-status { font-size: 14px; font-weight: 700; flex-shrink: 0; }
.summary-ok .summary-status { color: var(--success); }
.summary-fail .summary-status { color: var(--danger); }
.summary-info { display: flex; flex-direction: column; gap: 2px; }
.summary-name { font-size: 12px; font-weight: 500; color: var(--text-primary); }
.summary-msg { font-size: 11px; color: var(--danger); font-family: "JetBrains Mono", monospace; }
.summary-manual-link { margin-left: auto; align-self: center; font-size: 11px; color: var(--primary); text-decoration: underline; white-space: nowrap; flex-shrink: 0; }

.profiles-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
  gap: 10px;
}
.profile-card {
  display: flex; flex-direction: column; align-items: center; gap: 6px;
  padding: 14px 10px; border-radius: var(--radius-lg);
  border: 1px solid var(--border); background: var(--bg-tertiary);
  cursor: pointer; transition: all var(--transition-fast);
}
.profile-card:hover {
  border-color: var(--p-color, var(--accent-primary));
  background: var(--bg-secondary);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(0,0,0,0.15);
}
.profile-label { font-size: 13px; font-weight: 600; color: var(--text-primary); }
.profile-count { font-size: 11px; color: var(--text-muted); }
</style>
