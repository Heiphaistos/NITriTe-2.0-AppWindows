<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import NButton from "@/components/ui/NButton.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NSearchBar from "@/components/ui/NSearchBar.vue";
import NTabs from "@/components/ui/NTabs.vue";
import NBadge from "@/components/ui/NBadge.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import {
  Wrench, Play, ExternalLink, Star,
  ShieldCheck, Stethoscope, Trash2,
  Wifi, Settings, Download, Headphones,
  ShieldAlert,
} from "lucide-vue-next";

interface ToolInfo {
  id: string;
  name: string;
  description: string;
  command: string;
  is_url: boolean;
  category: string;
  section?: string;
  icon?: string;
  requires_admin?: boolean;
}

const STORAGE_RECENT   = "nitrite-tools-recent";
const STORAGE_FAVORITES = "nitrite-tools-favorites";
const STORAGE_COUNTS   = "nitrite-tools-counts";

const loading     = ref(true);
const tools       = ref<ToolInfo[]>([]);
const searchQuery = ref("");
const activeCategory = ref("all");
const launchingId = ref<string | null>(null);

// Admin checkbox state per tool
const adminChecked = ref<Record<string, boolean>>({});

// Reactive localStorage state
const recentIds    = ref<string[]>([]);
const favoriteIds  = ref<Set<string>>(new Set());
const usageCounts  = ref<Record<string, number>>({});

// ── localStorage helpers ─────────────────────────────────────────────────────
function loadLocalState() {
  try {
    const r = localStorage.getItem(STORAGE_RECENT);
    recentIds.value = r ? JSON.parse(r) : [];
  } catch { recentIds.value = []; }

  try {
    const f = localStorage.getItem(STORAGE_FAVORITES);
    favoriteIds.value = new Set(f ? JSON.parse(f) : []);
  } catch { favoriteIds.value = new Set(); }

  try {
    const c = localStorage.getItem(STORAGE_COUNTS);
    usageCounts.value = c ? JSON.parse(c) : {};
  } catch { usageCounts.value = {}; }
}

function saveRecent() {
  localStorage.setItem(STORAGE_RECENT, JSON.stringify(recentIds.value));
}

function saveFavorites() {
  localStorage.setItem(STORAGE_FAVORITES, JSON.stringify([...favoriteIds.value]));
}

function saveCounts() {
  localStorage.setItem(STORAGE_COUNTS, JSON.stringify(usageCounts.value));
}

function trackUsage(id: string) {
  // Comptes
  usageCounts.value[id] = (usageCounts.value[id] ?? 0) + 1;
  saveCounts();

  // Récents (max 5, sans doublon)
  recentIds.value = [id, ...recentIds.value.filter(x => x !== id)].slice(0, 5);
  saveRecent();
}

function toggleFavorite(id: string) {
  if (favoriteIds.value.has(id)) {
    favoriteIds.value.delete(id);
  } else {
    favoriteIds.value.add(id);
  }
  // Force reactivity
  favoriteIds.value = new Set(favoriteIds.value);
  saveFavorites();
}

// ── Tool lists ───────────────────────────────────────────────────────────────
const categoryTabs = [
  { id: "all", label: "Tout" },
  { id: "reparation", label: "Réparation" },
  { id: "diagnostics", label: "Diagnostics" },
  { id: "nettoyage", label: "Nettoyage" },
  { id: "reseau", label: "Réseau" },
  { id: "parametres", label: "Paramètres" },
  { id: "telechargements", label: "Téléchargements" },
  { id: "fabricants", label: "Fabricants" },
  { id: "benchmark", label: "Benchmark" },
  { id: "fournisseurs", label: "Fournisseurs" },
  { id: "activation", label: "Activation" },
  { id: "winget", label: "WinGet" },
  { id: "documentation", label: "Docs" },
  { id: "drivers", label: "Drivers" },
];

const categoryIcons: Record<string, any> = {
  reparation: ShieldCheck,
  diagnostics: Stethoscope,
  nettoyage: Trash2,
  reseau: Wifi,
  parametres: Settings,
  telechargements: Download,
  fabricants: Headphones,
};

const filteredTools = computed(() => {
  let result = tools.value;
  if (activeCategory.value !== "all") {
    result = result.filter((t) => t.category === activeCategory.value);
  }
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase();
    result = result.filter(
      (t) =>
        t.name.toLowerCase().includes(q) ||
        t.description.toLowerCase().includes(q)
    );
  }
  return result;
});

const recentTools = computed(() =>
  recentIds.value
    .map(id => tools.value.find(t => t.id === id))
    .filter((t): t is ToolInfo => !!t)
);

const favoriteTools = computed(() =>
  tools.value.filter(t => favoriteIds.value.has(t.id))
);

const devTools: ToolInfo[] = [
  { id: "1", name: "SFC /scannow", description: "Verification des fichiers systeme Windows", command: "sfc /scannow", is_url: false, category: "reparation", requires_admin: true },
  { id: "2", name: "DISM Repair", description: "Reparation de l'image systeme Windows", command: "DISM /Online /Cleanup-Image /RestoreHealth", is_url: false, category: "reparation", requires_admin: true },
  { id: "3", name: "chkdsk", description: "Verification et reparation du disque dur", command: "chkdsk C: /f /r", is_url: false, category: "reparation", requires_admin: true },
  { id: "4", name: "Moniteur de ressources", description: "Surveillance detaillee des ressources systeme", command: "resmon", is_url: false, category: "diagnostics" },
  { id: "5", name: "Informations systeme", description: "Details complets du systeme", command: "msinfo32", is_url: false, category: "diagnostics" },
  { id: "6", name: "Gestionnaire de peripheriques", description: "Gestion des pilotes et peripheriques", command: "devmgmt.msc", is_url: false, category: "diagnostics" },
  { id: "7", name: "Observateur d'evenements", description: "Journaux d'evenements Windows", command: "eventvwr.msc", is_url: false, category: "diagnostics" },
  { id: "8", name: "Nettoyage de disque", description: "Supprimer les fichiers temporaires", command: "cleanmgr", is_url: false, category: "nettoyage" },
  { id: "9", name: "Prefetch cleanup", description: "Nettoyer le dossier Prefetch", command: "del /q /s %SystemRoot%\\Prefetch\\*", is_url: false, category: "nettoyage", requires_admin: true },
  { id: "10", name: "Temp cleanup", description: "Nettoyer les fichiers temporaires", command: "del /q /s %TEMP%\\*", is_url: false, category: "nettoyage" },
  { id: "11", name: "Reset Winsock", description: "Reinitialiser le catalogue Winsock", command: "netsh winsock reset", is_url: false, category: "reseau", requires_admin: true },
  { id: "12", name: "Flush DNS", description: "Vider le cache DNS", command: "ipconfig /flushdns", is_url: false, category: "reseau" },
  { id: "13", name: "Reset IP", description: "Reinitialiser la configuration TCP/IP", command: "netsh int ip reset", is_url: false, category: "reseau", requires_admin: true },
  { id: "14", name: "Windows Update", description: "Parametres de mise a jour Windows", command: "ms-settings:windowsupdate", is_url: true, category: "parametres" },
  { id: "15", name: "Applications par defaut", description: "Configurer les applications par defaut", command: "ms-settings:defaultapps", is_url: true, category: "parametres" },
  { id: "16", name: "Systeme", description: "Parametres systeme generaux", command: "ms-settings:about", is_url: true, category: "parametres" },
  { id: "17", name: "Reseau & Internet", description: "Parametres reseau", command: "ms-settings:network", is_url: true, category: "parametres" },
  { id: "18", name: "CrystalDiskInfo (web)", description: "Site officiel CrystalDiskInfo", command: "https://crystalmark.info/en/software/crystaldiskinfo/", is_url: true, category: "telechargements" },
  { id: "19", name: "HWMonitor (web)", description: "Site officiel HWMonitor", command: "https://www.cpuid.com/softwares/hwmonitor.html", is_url: true, category: "telechargements" },
  { id: "20", name: "CPU-Z (web)", description: "Site officiel CPU-Z", command: "https://www.cpuid.com/softwares/cpu-z.html", is_url: true, category: "telechargements" },
  { id: "25", name: "MajorGeeks", description: "Base de logiciels systeme et utilitaires", command: "https://www.majorgeeks.com/", is_url: true, category: "telechargements" },
  { id: "26", name: "Malekal", description: "Guides securite, drivers et Windows", command: "https://www.malekal.com/", is_url: true, category: "telechargements" },
  { id: "28", name: "La Cale", description: "Portail de telechargement", command: "https://la-cale.space/login", is_url: true, category: "telechargements" },
  { id: "29", name: "Gemini Tracker", description: "Tracker torrent prive", command: "https://gemini-tracker.org/login", is_url: true, category: "telechargements" },
  { id: "30", name: "C411 Tracker", description: "Portail torrent communautaire", command: "https://staging-68d548c5bd4.c411.org/login", is_url: true, category: "telechargements" },
  { id: "p1", name: "Autoruns64 (Sysinternals)", description: "Gestion avancee des entrees de demarrage Windows", command: "https://learn.microsoft.com/en-us/sysinternals/downloads/autoruns", is_url: true, category: "diagnostics" },
  { id: "p3", name: "GetDataBack (web)", description: "Recuperation de donnees avancee depuis des disques endommages", command: "https://www.runtime.org/data-recovery-software.htm", is_url: true, category: "diagnostics" },
  { id: "p6", name: "CPU-Z Portable (web)", description: "Informations detaillees sur le CPU, RAM et carte mere", command: "https://www.cpuid.com/softwares/cpu-z.html", is_url: true, category: "diagnostics" },
  { id: "p7", name: "CrystalDiskInfo (web)", description: "Analyse S.M.A.R.T. et sante des disques durs/SSD", command: "https://crystalmark.info/en/software/crystaldiskinfo/", is_url: true, category: "diagnostics" },
  { id: "p8", name: "CrystalDiskMark (web)", description: "Benchmark de vitesse des disques (lecture/ecriture)", command: "https://crystalmark.info/en/software/crystaldiskmark/", is_url: true, category: "benchmark" },
  { id: "p9", name: "HWiNFO (web)", description: "Informations materielles completes et monitoring temps reel", command: "https://www.hwinfo.com/download/", is_url: true, category: "diagnostics" },
  { id: "p10", name: "HWMonitor (web)", description: "Surveillance des temperatures, voltages et ventilateurs", command: "https://www.cpuid.com/softwares/hwmonitor.html", is_url: true, category: "diagnostics" },
  { id: "p11", name: "Hard Disk Sentinel (web)", description: "Surveillance avancee de la sante des disques (HDD/SSD/NVMe)", command: "https://www.hdsentinel.com/", is_url: true, category: "diagnostics" },
  { id: "p12", name: "Process Explorer (Sysinternals)", description: "Gestionnaire de processus avance (Sysinternals)", command: "https://learn.microsoft.com/en-us/sysinternals/downloads/process-explorer", is_url: true, category: "diagnostics" },
  { id: "p2", name: "Bulk Crap Uninstaller (web)", description: "Desinstalleur massif d'applications avec nettoyage des residus", command: "https://www.bcuninstaller.com/", is_url: true, category: "nettoyage" },
  { id: "p14", name: "AdwCleaner (web)", description: "Suppression des adwares, PUP et toolbars indesirables", command: "https://www.malwarebytes.com/adwcleaner", is_url: true, category: "nettoyage" },
  { id: "p15", name: "WiseCare 365 (web)", description: "Optimisation et nettoyage systeme complet", command: "https://www.wisecleaner.com/wise-care-365.html", is_url: true, category: "nettoyage" },
  { id: "p4", name: "DDU (Display Driver Uninstaller)", description: "Desinstallation propre et complete des pilotes graphiques", command: "https://www.guru3d.com/download/display-driver-uninstaller-download/", is_url: true, category: "drivers" },
  // Activation
  { id: "act1", name: "Statut activation Windows",      description: "Vérifie si Windows est activé (slmgr /xpr)", command: "slmgr /xpr", is_url: false, category: "activation" },
  { id: "act2", name: "Infos licence détaillées",        description: "Affiche toutes les infos de licence (slmgr /dlv)", command: "slmgr /dlv", is_url: false, category: "activation" },
  { id: "act3", name: "Statut licence court",            description: "Résumé rapide de la licence (slmgr /dli)", command: "slmgr /dli", is_url: false, category: "activation" },
  { id: "act4", name: "Activer Windows en ligne",        description: "Tente l'activation via Internet (slmgr /ato)", command: "slmgr /ato", is_url: false, category: "activation", requires_admin: true },
  { id: "act5", name: "Désinstaller clé produit",        description: "Retire la clé installée (slmgr /upk)", command: "slmgr /upk", is_url: false, category: "activation", requires_admin: true },
  { id: "act6", name: "Réinitialiser période d'essai",   description: "Rearm Windows – repousse l'expiration (slmgr /rearm)", command: "slmgr /rearm", is_url: false, category: "activation", requires_admin: true },
  { id: "act7", name: "Paramètres Activation (GUI)",     description: "Ouvre les paramètres d'activation Windows", command: "ms-settings:activation", is_url: true, category: "activation" },
  { id: "act8", name: "Clé produit (OA3 PS)",           description: "Lit la clé OEM embarquée dans le BIOS", command: "powershell (Get-WmiObject -query 'select * from SoftwareLicensingService').OA3xOriginalProductKey", is_url: false, category: "activation", requires_admin: true },
  { id: "act9", name: "Edition & Version Windows",       description: "Affiche l'édition, version et build (WMIC)", command: "wmic os get caption,version,buildnumber,osarchitecture", is_url: false, category: "activation" },
  { id: "act10", name: "MS Activation Scripts (web)",    description: "Scripts d'activation open-source de massgravel", command: "https://massgrave.dev/", is_url: true, category: "activation" },
  { id: "act11", name: "Office Tool Plus (web)",         description: "Outil d'installation et activation Office", command: "https://otp.landian.vip/", is_url: true, category: "activation" },
  { id: "act12", name: "KMS Server local",               description: "Définir un serveur KMS (remplacer l'adresse)", command: "slmgr /skms kms.example.com", is_url: false, category: "activation", requires_admin: true },
  // Drivers
  { id: "p5", name: "WinDirStat", description: "Analyse visuelle de l'espace disque (a telecharger)", command: "https://windirstat.net/download.html", is_url: true, category: "diagnostics" },
  { id: "21", name: "Dell Support", description: "Support et pilotes Dell", command: "https://www.dell.com/support", is_url: true, category: "fabricants" },
  { id: "22", name: "HP Support", description: "Support et pilotes HP", command: "https://support.hp.com", is_url: true, category: "fabricants" },
  { id: "23", name: "Lenovo Support", description: "Support et pilotes Lenovo", command: "https://support.lenovo.com", is_url: true, category: "fabricants" },
  { id: "24", name: "ASUS Support", description: "Support et pilotes ASUS", command: "https://www.asus.com/support/", is_url: true, category: "fabricants" },
];

// Mots-clés dont le nom indique un outil d'activation Windows/Office
const ACTIVATION_NAME_KEYWORDS = [
  "activation script", "ms activation", "kms tool", "office tool plus",
  "windows toolkit", "windows loader", "slmgr", "kmsauto", "kmspico",
  "office activat", "windows activat", "hwid", "massgrave",
];

function normalizeCategory(section: string, name: string = ""): string {
  const s = section.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "");
  const n = name.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "");

  // Priorité : si le nom cible explicitement l'activation → activation
  if (ACTIVATION_NAME_KEYWORDS.some(k => n.includes(k))) return "activation";

  if (s.includes("reparation")) return "reparation";
  if (s.includes("maintenance") || s.includes("nettoyage")) return "nettoyage";
  if (s.includes("diagnostic") || s.includes("info")) return "diagnostics";
  if (s.includes("reseau") || s.includes("internet") || s.includes("network")) return "reseau";
  if (s.includes("parametre") || s.includes("ms-settings") || s.includes("windows setting")) return "parametres";
  if (s.includes("telechargement") || s.includes("download")) return "telechargements";
  if (s.includes("activation")) return "activation";
  if (s.includes("fabricant") || s.includes("support")) return "fabricants";
  if (s.includes("benchmark") || s.includes("test") || s.includes("performa")) return "benchmark";
  if (s.includes("fournisseur") || s.includes("achat")) return "fournisseurs";
  if (s.includes("winget") || s.includes("package")) return "winget";
  if (s.includes("documentation") || s.includes("doc")) return "documentation";
  if (s.includes("driver") || s.includes("pilote")) return "drivers";
  return "diagnostics";
}

async function loadTools() {
  loading.value = true;
  try {
    const raw = await invoke<any[]>("get_tools");
    tools.value = raw.map((t, i) => ({
      id: String(i),
      name: t.name,
      description: t.description || "",
      command: t.command,
      is_url: t.is_url,
      category: normalizeCategory(t.section || t.category || "", t.name || ""),
      requires_admin: t.requires_admin ?? false,
    }));
  } catch {
    tools.value = devTools;
  }
  loading.value = false;
}

async function launchTool(tool: ToolInfo) {
  launchingId.value = tool.id;
  trackUsage(tool.id);

  try {
    const useAdmin = tool.requires_admin && (adminChecked.value[tool.id] ?? false);
    if (useAdmin && !tool.is_url) {
      // Ouvre un CMD élevé visible qui reste ouvert après la commande
      await invoke("run_system_command", {
        cmd: "powershell",
        args: ["-NoProfile", "-Command",
               `Start-Process cmd -ArgumentList '/K ${tool.command}' -Verb RunAs`],
      });
    } else {
      await invoke("execute_tool", { command: tool.command, isUrl: tool.is_url });
    }
  } catch (err) {
    if (tool.is_url && tool.command.startsWith("http")) {
      window.open(tool.command, "_blank");
    } else {
      alert(`Erreur lors du lancement de "${tool.name}":\n${err}`);
    }
  }
  setTimeout(() => { launchingId.value = null; }, 500);
}

function getCategoryIcon(category: string) {
  return categoryIcons[category] || Wrench;
}

onMounted(() => {
  loadLocalState();
  loadTools();
});
</script>

<template>
  <div class="tools-page">
    <DiagBanner :icon="Wrench" title="Boîte à Outils" desc="Outils Windows intégrés et utilitaires système" color="teal" />

    <div class="page-header">
      <div>
        <h1>Outils Systeme</h1>
        <p class="page-subtitle">Lancer des outils de maintenance et diagnostic Windows</p>
      </div>
    </div>

    <div v-if="loading" class="loading-state">
      <NSpinner :size="32" />
      <p>Chargement des outils...</p>
    </div>

    <template v-else>
      <div class="toolbar">
        <NSearchBar v-model="searchQuery" placeholder="Rechercher un outil..." />
      </div>

      <!-- Section Favoris -->
      <div v-if="favoriteTools.length" class="quick-section">
        <div class="quick-section-title">
          <Star :size="14" class="star-icon filled" />
          Favoris
        </div>
        <div class="tools-grid">
          <div v-for="tool in favoriteTools" :key="'fav-' + tool.id" class="tool-card">
            <div class="tool-icon-wrap">
              <component :is="getCategoryIcon(tool.category)" :size="20" />
            </div>
            <div class="tool-info">
              <div class="tool-name">
                {{ tool.name }}
                <ExternalLink v-if="tool.is_url" :size="12" class="tool-url-icon" />
                <NBadge v-if="(usageCounts[tool.id] ?? 0) > 5" variant="warning" size="sm" style="margin-left:4px">
                  {{ usageCounts[tool.id] }}x
                </NBadge>
              </div>
              <div class="tool-desc">{{ tool.description }}</div>
              <label v-if="tool.requires_admin" class="admin-check">
                <input type="checkbox" v-model="adminChecked[tool.id]" />
                <ShieldAlert :size="11" />
                Lancer en admin
              </label>
            </div>
            <div class="tool-actions">
              <button class="fav-btn active" @click="toggleFavorite(tool.id)" title="Retirer des favoris">
                <Star :size="13" />
              </button>
              <NButton variant="secondary" size="sm" :loading="launchingId === tool.id" @click="launchTool(tool)">
                <Play :size="14" />
                Lancer
              </NButton>
            </div>
          </div>
        </div>
      </div>

      <!-- Section Récemment utilisés -->
      <div v-if="recentTools.length" class="quick-section">
        <div class="quick-section-title">
          <Play :size="14" />
          Récemment utilisés
        </div>
        <div class="tools-grid">
          <div v-for="tool in recentTools" :key="'rec-' + tool.id" class="tool-card">
            <div class="tool-icon-wrap">
              <component :is="getCategoryIcon(tool.category)" :size="20" />
            </div>
            <div class="tool-info">
              <div class="tool-name">
                {{ tool.name }}
                <ExternalLink v-if="tool.is_url" :size="12" class="tool-url-icon" />
                <NBadge v-if="(usageCounts[tool.id] ?? 0) > 5" variant="warning" size="sm" style="margin-left:4px">
                  {{ usageCounts[tool.id] }}x
                </NBadge>
              </div>
              <div class="tool-desc">{{ tool.description }}</div>
              <label v-if="tool.requires_admin" class="admin-check">
                <input type="checkbox" v-model="adminChecked[tool.id]" />
                <ShieldAlert :size="11" />
                Lancer en admin
              </label>
            </div>
            <div class="tool-actions">
              <button
                class="fav-btn"
                :class="{ active: favoriteIds.has(tool.id) }"
                @click="toggleFavorite(tool.id)"
                title="Ajouter aux favoris"
              >
                <Star :size="13" />
              </button>
              <NButton variant="secondary" size="sm" :loading="launchingId === tool.id" @click="launchTool(tool)">
                <Play :size="14" />
                Lancer
              </NButton>
            </div>
          </div>
        </div>
      </div>

      <!-- Tous les outils par catégorie -->
      <NTabs :tabs="categoryTabs" v-model="activeCategory">
        <template #default>
          <div v-if="filteredTools.length" class="tools-grid">
            <div v-for="tool in filteredTools" :key="tool.id" class="tool-card">
              <div class="tool-icon-wrap">
                <component :is="getCategoryIcon(tool.category)" :size="20" />
              </div>
              <div class="tool-info">
                <div class="tool-name">
                  {{ tool.name }}
                  <ExternalLink v-if="tool.is_url" :size="12" class="tool-url-icon" />
                  <NBadge v-if="(usageCounts[tool.id] ?? 0) > 5" variant="warning" size="sm" style="margin-left:4px">
                    {{ usageCounts[tool.id] }}x
                  </NBadge>
                </div>
                <div class="tool-desc">{{ tool.description }}</div>
                <label v-if="tool.requires_admin" class="admin-check">
                  <input type="checkbox" v-model="adminChecked[tool.id]" />
                  <ShieldAlert :size="11" />
                  Lancer en admin
                </label>
              </div>
              <div class="tool-actions">
                <button
                  class="fav-btn"
                  :class="{ active: favoriteIds.has(tool.id) }"
                  @click="toggleFavorite(tool.id)"
                  title="Favoris"
                >
                  <Star :size="13" />
                </button>
                <NButton
                  variant="secondary"
                  size="sm"
                  :loading="launchingId === tool.id"
                  @click="launchTool(tool)"
                >
                  <Play :size="14" />
                  Lancer
                </NButton>
              </div>
            </div>
          </div>

          <div v-else class="empty-state">
            Aucun outil trouve pour cette recherche.
          </div>
        </template>
      </NTabs>
    </template>
  </div>
</template>

<style scoped>
.tools-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.page-header h1 { font-size: 24px; font-weight: 700; }
.page-subtitle { color: var(--text-secondary); font-size: 13px; margin-top: 2px; }

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 60px;
  color: var(--text-secondary);
}

.toolbar { max-width: 400px; }

/* Quick sections */
.quick-section {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 12px 16px;
}

.quick-section-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-muted);
  margin-bottom: 10px;
}

.star-icon.filled { color: #f59e0b; }

/* Tools Grid */
.tools-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 10px;
}

.tool-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 14px;
  transition: all var(--transition-normal);
}

.quick-section .tool-card {
  background: var(--bg-tertiary);
}

.tool-card:hover {
  border-color: var(--accent-primary);
  box-shadow: 0 4px 20px rgba(249,115,22,.15);
  transform: translateY(-2px);
}

.tool-icon-wrap {
  width: 40px;
  height: 40px;
  border-radius: var(--radius-md);
  background: linear-gradient(135deg, var(--accent-muted), transparent);
  box-shadow: 0 3px 10px rgba(249,115,22,.2);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--accent-primary);
}

.tool-info { flex: 1; min-width: 0; }

.tool-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  display: flex;
  align-items: center;
  gap: 6px;
}

.tool-url-icon { color: var(--text-muted); }

.tool-desc {
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Admin checkbox */
.admin-check {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: var(--warning);
  margin-top: 4px;
  cursor: pointer;
  user-select: none;
}

.admin-check input[type="checkbox"] {
  width: 12px;
  height: 12px;
  accent-color: var(--warning);
  cursor: pointer;
}

/* Actions */
.tool-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.fav-btn {
  width: 28px;
  height: 28px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--transition-fast);
  flex-shrink: 0;
}

.fav-btn:hover { border-color: #f59e0b; color: #f59e0b; background: rgba(245,158,11,.1); }
.fav-btn.active { border-color: #f59e0b; color: #f59e0b; background: rgba(245,158,11,.15); }

.empty-state {
  text-align: center;
  color: var(--text-secondary);
  font-size: 13px;
  padding: 40px;
}
</style>
