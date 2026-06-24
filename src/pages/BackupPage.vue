<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke, invokeRaw } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NProgress from "@/components/ui/NProgress.vue";
import NBadge from "@/components/ui/NBadge.vue";
import { useNotificationStore } from "@/stores/notifications";
import {
  Save, FolderArchive, CheckSquare, Square,
  RefreshCw, CheckCircle, Clock, Download,
  AlertTriangle, FolderOpen, Lock,
} from "lucide-vue-next";

const SENSITIVE_ITEMS = new Set(["wifi_passwords","bitlocker_keys","windows_license","ssh_keys","office_license"]);

const notify = useNotificationStore();

// --- Format d'export ---
type ExportFormat = 'json' | 'txt' | 'html' | 'md';
const exportFormat = ref<ExportFormat>('txt');

// --- Emplacement personnalisé ---
const customBackupPath = ref('');
const useCustomPath = ref(false);

async function pickBackupFolder() {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const dir = await open({ directory: true, title: 'Choisir le dossier de sauvegarde' });
    if (dir && typeof dir === 'string') {
      customBackupPath.value = dir;
      useCustomPath.value = true;
    }
  } catch (e: any) {
    notify.error('Erreur sélection dossier', String(e));
  }
}

async function openSaveFolder() {
  try {
    if (customBackupPath.value) {
      await invoke('open_path', { path: customBackupPath.value });
    } else {
      const { homeDir, join } = await import('@tauri-apps/api/path');
      const defaultFolder = await join(await homeDir(), 'Documents', 'NiTriTe', 'backups');
      await invoke('open_path', { path: defaultFolder });
    }
  } catch { notify.error("Impossible d'ouvrir le dossier"); }
}

// --- Backup items ---
interface BackupItem {
  id: string;
  label: string;
  description: string;
  checked: boolean;
  category: string;
}

const backupItems = ref<BackupItem[]>([
  // Logiciels & systeme
  { id: "installed_apps",      label: "Apps installees",       description: "Liste complete de toutes les applications installees",                     checked: true,  category: "Logiciels & Système" },
  { id: "winget_export",       label: "WinGet JSON",           description: "Export winget packages.json reinstallable avec winget import",              checked: true,  category: "Logiciels & Système" },
  { id: "drivers",             label: "Drivers",               description: "Sauvegarde des pilotes du systeme via driverquery",                        checked: true,  category: "Logiciels & Système" },
  { id: "windows_features",    label: "Features Windows",      description: "Fonctionnalites Windows activees/desactivees",                              checked: false, category: "Logiciels & Système" },
  { id: "installed_fonts",     label: "Polices installees",    description: "Liste de toutes les polices du systeme",                                   checked: false, category: "Logiciels & Système" },
  // Reseau & securite
  { id: "network_config",      label: "Config reseau",         description: "Configuration IP, DNS, Wi-Fi, passerelle",                                  checked: true,  category: "Réseau & Sécurité" },
  { id: "wifi_passwords",      label: "Mots de passe WiFi",    description: "Profils WiFi avec mots de passe en clair",                                  checked: true,  category: "Réseau & Sécurité" },
  { id: "firewall_rules",      label: "Regles firewall",       description: "Export des regles du pare-feu Windows",                                    checked: false, category: "Réseau & Sécurité" },
  { id: "network_shares",      label: "Partages reseau",       description: "Lecteurs reseau mappes et partages SMB",                                   checked: false, category: "Réseau & Sécurité" },
  { id: "hosts_file",          label: "Fichier hosts",         description: "Copie du fichier C:\\Windows\\System32\\drivers\\etc\\hosts",              checked: false, category: "Réseau & Sécurité" },
  // Licences
  { id: "windows_license",     label: "Cle Windows",           description: "Cle de produit Windows extraite en clair via WMI/registry",               checked: true,  category: "Licences" },
  { id: "bitlocker_keys",      label: "Cles BitLocker",        description: "Cle(s) de recuperation BitLocker en clair via PowerShell",                 checked: true,  category: "Licences" },
  { id: "office_license",      label: "Cle Office",            description: "Cle de produit Microsoft Office extraite en clair via OSPP.vbs",           checked: true,  category: "Licences" },
  // Navigateurs
  { id: "chrome_bookmarks",    label: "Favoris Chrome",        description: "Favoris Google Chrome (Bookmarks JSON)",                                   checked: false, category: "Navigateurs" },
  { id: "edge_bookmarks",      label: "Favoris Edge",          description: "Favoris Microsoft Edge (Bookmarks JSON)",                                  checked: false, category: "Navigateurs" },
  { id: "brave_bookmarks",     label: "Favoris Brave",         description: "Favoris navigateur Brave (Bookmarks JSON)",                                checked: false, category: "Navigateurs" },
  // Demarrage & taches
  { id: "startup_programs",    label: "Programmes demarrage",  description: "Liste des programmes au demarrage (HKCU + HKLM Run)",                      checked: true,  category: "Démarrage & Tâches" },
  { id: "scheduled_tasks",     label: "Taches planifiees",     description: "Taches planifiees Windows actives (Get-ScheduledTask)",                     checked: false, category: "Démarrage & Tâches" },
  { id: "registry_export",     label: "Export registre Run",   description: "Export partiel registre utilisateur (Run, Shell, Winlogon)",                checked: false, category: "Démarrage & Tâches" },
  // Dev
  { id: "env_variables",       label: "Variables env.",        description: "Variables d'environnement systeme et utilisateur",                          checked: true,  category: "Développeur" },
  { id: "ssh_keys",            label: "Cles SSH",              description: "Fichiers ~/.ssh/ (id_rsa, id_ed25519, known_hosts, config)",               checked: false, category: "Développeur" },
  { id: "pip_packages",        label: "Packages Python",       description: "pip freeze > requirements.txt",                                            checked: false, category: "Développeur" },
  { id: "vscode_extensions",   label: "Extensions VSCode",     description: "code --list-extensions",                                                  checked: false, category: "Développeur" },
  { id: "wsl_config",          label: "Config WSL",            description: "Liste distros WSL + copie de .wslconfig",                                  checked: false, category: "Développeur" },
  { id: "powershell_profile",  label: "Profil PowerShell",     description: "Copie du profil PowerShell utilisateur ($PROFILE)",                       checked: false, category: "Développeur" },
  // Matériel
  { id: "system_components",   label: "Composants PC",         description: "CPU, GPU, RAM, SSD/HDD, Carte mère, BIOS — rapport complet",              checked: true,  category: "Matériel" },
  { id: "power_plans",         label: "Plans d'alimentation",  description: "Export de tous les plans d'alimentation (powercfg /export)",               checked: false, category: "Matériel" },
  { id: "printer_config",      label: "Imprimantes",           description: "Liste des imprimantes installes et leurs pilotes",                         checked: false, category: "Matériel" },
  // Divers
  { id: "folder_sizes",        label: "Tailles dossiers",      description: "Top 30 dossiers les plus volumineux sur C:",                               checked: false, category: "Divers" },
  { id: "desktop_files",       label: "Fichiers Bureau",       description: "Liste des fichiers presents sur le Bureau",                                checked: false, category: "Divers" },
  { id: "suspicious_processes",label: "Processus suspects",    description: "Processus hors dossiers systeme standards (hors System32/PF)",             checked: false, category: "Divers" },
]);

// Groupement par catégorie
const groupedItems = computed(() => {
  const groups: Record<string, BackupItem[]> = {};
  for (const item of backupItems.value) {
    if (!groups[item.category]) groups[item.category] = [];
    groups[item.category].push(item);
  }
  return groups;
});

// Presets
function applyPreset(preset: 'essential' | 'full' | 'dev') {
  const essential = new Set(["installed_apps","winget_export","drivers","network_config","wifi_passwords","windows_license","bitlocker_keys","office_license","startup_programs","env_variables","system_components"]);
  const dev = new Set([...essential, "ssh_keys","pip_packages","vscode_extensions","wsl_config","powershell_profile","firewall_rules","scheduled_tasks","registry_export"]);
  backupItems.value.forEach(item => {
    if (preset === 'essential') item.checked = essential.has(item.id);
    else if (preset === 'dev') item.checked = dev.has(item.id);
    else item.checked = true;
  });
}

function toggleItem(id: string) {
  const item = backupItems.value.find((i) => i.id === id);
  if (item) item.checked = !item.checked;
}

function selectAll() {
  backupItems.value.forEach((i) => (i.checked = true));
}

function selectNone() {
  backupItems.value.forEach((i) => (i.checked = false));
}

const selectedCount = computed(() => backupItems.value.filter((i) => i.checked).length);

// Estimation taille backup (heuristique)
const ESTIMATES: Record<string, number> = {
  installed_apps: 80,    winget_export: 30,    drivers: 100,
  windows_features: 20, installed_fonts: 15,  network_config: 20,
  wifi_passwords: 5,    firewall_rules: 30,   network_shares: 10,
  hosts_file: 5,        windows_license: 5,   bitlocker_keys: 5,
  office_license: 5,    chrome_bookmarks: 100, edge_bookmarks: 100,
  brave_bookmarks: 80,  startup_programs: 15,  scheduled_tasks: 25,
  registry_export: 10,  env_variables: 10,    ssh_keys: 20,
  pip_packages: 15,     vscode_extensions: 10, wsl_config: 10,
  powershell_profile: 5, system_components: 50, power_plans: 40,
  printer_config: 15,   folder_sizes: 20,     desktop_files: 10,
  suspicious_processes: 25,
};

const estimatedSizeKb = computed(() => {
  return backupItems.value
    .filter(i => i.checked)
    .reduce((acc, i) => acc + (ESTIMATES[i.id] ?? 10), 0);
});

const estimatedSizeLabel = computed(() => {
  const kb = estimatedSizeKb.value;
  if (kb < 1024) return `~${kb} KB`;
  return `~${(kb / 1024).toFixed(1)} MB`;
});

// --- Backup creation ---
const backupInProgress = ref(false);
const backupProgress = ref(0);
const backupStatus = ref("");
const backupResult = ref<{ path: string; items: string[] } | null>(null);

async function createBackup() {
  const selected = backupItems.value.filter((i) => i.checked).map((i) => i.id);
  if (selected.length === 0) {
    notify.warning("Aucun element", "Selectionnez au moins un element a sauvegarder.");
    return;
  }

  backupInProgress.value = true;
  backupProgress.value = 0;
  backupResult.value = null;
  backupStatus.value = "Initialisation...";

  try {

    // Simulate progress steps
    for (let i = 0; i < selected.length; i++) {
      const label = backupItems.value.find((b) => b.id === selected[i])?.label ?? selected[i];
      backupStatus.value = `Sauvegarde : ${label}...`;
      backupProgress.value = Math.round(((i + 1) / selected.length) * 100);
      await new Promise((r) => setTimeout(r, 200));
    }

    const result = await invokeRaw<{ path: string; total_items: number }>("create_backup", {
      items: selected,
      format: exportFormat.value,
      customPath: useCustomPath.value ? customBackupPath.value : undefined,
    });
    backupResult.value = { path: result.path, items: selected };
    notify.success(
      `Backup créé — ${selected.length} élément(s) sauvegardé(s)`,
      result.path
    );
  } catch (e: any) {
    notify.error("Sauvegarde échouée", String(e));
    backupProgress.value = 0;
  } finally {
    backupInProgress.value = false;
    backupStatus.value = "";
  }
}

// --- Previous backups ---
interface BackupEntry {
  filename: string;
  date: string;
  size: string;
  items_count: number;
}

const previousBackups = ref<BackupEntry[]>([]);
const backupsLoading = ref(true);

async function loadBackups() {
  backupsLoading.value = true;
  try {
    previousBackups.value = await invoke<BackupEntry[]>("list_backups");
  } catch {
    previousBackups.value = [];
  } finally {
    backupsLoading.value = false;
  }
}

async function openEntryFolder(filename: string) {
  try {
    const folder = backupResult.value?.path
      ? backupResult.value.path.substring(0, backupResult.value.path.lastIndexOf("\\"))
      : null;
    if (folder) {
      await invoke("open_path", { path: folder });
      return;
    }
    const { homeDir, join } = await import("@tauri-apps/api/path");
    const defaultFolder = await join(await homeDir(), "Documents", "NiTriTe", "backups");
    await invoke("open_path", { path: defaultFolder });
  } catch {
    notify.error(`Impossible d'ouvrir le dossier pour ${filename}`);
  }
}

async function openBackupFolder() {
  try {
    // Essaye d'abord le chemin retourné par le dernier backup
    const folder = backupResult.value?.path
      ? backupResult.value.path.substring(0, backupResult.value.path.lastIndexOf("\\"))
      : null;
    if (folder) {
      await invoke("open_path", { path: folder });
      return;
    }
    // Sinon dossier par défaut NiTriTe/backups
    const { homeDir, join } = await import("@tauri-apps/api/path");
    const defaultFolder = await join(await homeDir(), "Documents", "NiTriTe", "backups");
    await invoke("open_path", { path: defaultFolder });
  } catch {
    notify.error("Impossible d'ouvrir le dossier de sauvegarde");
  }
}

onMounted(loadBackups);
</script>

<template>
  <div class="backup">
    <!-- Header -->
    <div class="page-header">
      <div>
        <h1>Sauvegarde</h1>
        <p class="page-subtitle">Sauvegardez la configuration de votre systeme</p>
      </div>
      <NButton variant="ghost" size="sm" @click="openBackupFolder">
        <FolderOpen :size="14" />
        Ouvrir le dossier
      </NButton>
    </div>

    <div class="backup-grid">
      <!-- Backup items selection -->
      <NCard>
        <template #header>
          <div class="section-header">
            <Save :size="16" />
            <span>Elements a sauvegarder</span>
            <div class="header-btns">
              <div class="preset-btns">
                <button class="preset-btn preset-essential" @click="applyPreset('essential')" title="Sélection recommandée">Essentiel</button>
                <button class="preset-btn preset-dev"       @click="applyPreset('dev')"       title="Profil développeur">Dev</button>
                <button class="preset-btn preset-full"      @click="applyPreset('full')"      title="Tout sélectionner">Complet</button>
              </div>
              <span class="separator">·</span>
              <button class="link-btn" @click="selectAll">Tout</button>
              <span class="separator">|</span>
              <button class="link-btn" @click="selectNone">Aucun</button>
            </div>
          </div>
        </template>

        <div class="items-list">
          <template v-for="(items, category) in groupedItems" :key="category">
            <div class="category-header">
              <span class="cat-label">{{ category }}</span>
              <span class="cat-count">{{ items.filter(i => i.checked).length }}/{{ items.length }}</span>
            </div>
            <button
              v-for="item in items"
              :key="item.id"
              class="backup-item"
              :class="{ checked: item.checked }"
              @click="toggleItem(item.id)"
            >
              <component :is="item.checked ? CheckSquare : Square" :size="16" class="check-icon" />
              <div class="item-info">
                <div style="display:flex;align-items:center;gap:6px">
                  <span class="item-label">{{ item.label }}</span>
                  <span v-if="SENSITIVE_ITEMS.has(item.id)" class="sensitive-badge" title="Ce contenu est exporté en clair — conservez le fichier en lieu sûr">
                    <Lock :size="9" /> Sensible
                  </span>
                </div>
                <span class="item-desc">{{ item.description }}</span>
              </div>
            </button>
          </template>
        </div>

        <!-- Format d'export -->
        <div class="export-format-row">
          <span style="font-size:12px;color:var(--text-muted)">Format :</span>
          <div class="format-tabs">
            <button v-for="fmt in ['txt','html','md','json']" :key="fmt"
              class="fmt-btn" :class="{ active: exportFormat === fmt }"
              @click="exportFormat = fmt as ExportFormat">
              .{{ fmt }}
            </button>
          </div>
        </div>

        <!-- Emplacement de sauvegarde -->
        <div class="path-row">
          <span style="font-size:12px;color:var(--text-muted);flex-shrink:0">Dossier :</span>
          <code class="path-code">{{ customBackupPath || '~/Documents/NiTriTe/backups (défaut)' }}</code>
          <NButton variant="ghost" size="sm" @click="pickBackupFolder"><FolderOpen :size="12" /> Choisir</NButton>
          <NButton variant="ghost" size="sm" @click="openSaveFolder"><FolderOpen :size="12" /> Ouvrir</NButton>
          <button v-if="useCustomPath" class="link-btn" @click="useCustomPath = false; customBackupPath = ''">Réinitialiser</button>
        </div>

        <div class="backup-actions">
          <span class="selected-count">{{ selectedCount }} / {{ backupItems.length }} selectionne(s) <span style="color:var(--text-muted);font-size:11px">· {{ estimatedSizeLabel }}</span></span>
          <NButton
            variant="primary"
            :loading="backupInProgress"
            :disabled="backupInProgress || selectedCount === 0"
            @click="createBackup"
          >
            <Save :size="14" />
            Creer la sauvegarde
          </NButton>
        </div>

        <!-- Progress -->
        <div v-if="backupInProgress" class="backup-progress">
          <p class="progress-status">{{ backupStatus }}</p>
          <NProgress :value="backupProgress" size="lg" showLabel />
        </div>

        <!-- Result -->
        <div v-if="backupResult" class="backup-result">
          <CheckCircle :size="18" style="color: var(--success)" />
          <div class="result-info">
            <p class="result-title">Sauvegarde creee avec succes</p>
            <p class="result-path font-mono">{{ backupResult.path }}</p>
            <p class="result-items">{{ backupResult.items.length }} element(s) sauvegardes</p>
          </div>
        </div>
      </NCard>

      <!-- Previous backups -->
      <NCard>
        <template #header>
          <div class="section-header">
            <FolderArchive :size="16" />
            <span>Sauvegardes precedentes</span>
            <NButton variant="secondary" size="sm" :loading="backupsLoading" @click="loadBackups" style="margin-left: auto">
              <RefreshCw :size="14" />
            </NButton>
          </div>
        </template>

        <div v-if="backupsLoading" class="loading-state">
          <NSpinner :size="24" />
          <p>Chargement...</p>
        </div>

        <div v-else-if="previousBackups.length === 0" class="empty-state">
          <FolderArchive :size="32" style="color: var(--text-muted); opacity: 0.3" />
          <p>Aucune sauvegarde trouvee</p>
        </div>

        <div v-else class="backups-list">
          <div v-for="backup in previousBackups" :key="backup.filename" class="backup-entry">
            <div class="entry-icon">
              <FolderArchive :size="18" style="color: var(--accent-primary)" />
            </div>
            <div class="entry-info">
              <span class="entry-name font-mono">{{ backup.filename }}</span>
              <span class="entry-meta">
                <Clock :size="12" /> {{ backup.date }}
                &middot; {{ backup.size }}
                <template v-if="backup.items_count > 0">&middot; {{ backup.items_count }} éléments</template>
              </span>
            </div>
            <NButton variant="ghost" size="sm" @click="openEntryFolder(backup.filename)" style="margin-left:auto;flex-shrink:0">
              <FolderOpen :size="12" />
            </NButton>
          </div>
        </div>
      </NCard>
    </div>
  </div>
</template>

<style scoped>
.backup {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.page-header h1 {
  font-size: 24px;
  font-weight: 700;
}

.page-subtitle {
  color: var(--text-muted);
  font-size: 13px;
  margin-top: 2px;
}

.backup-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  align-items: start;
}

@media (max-width: 1000px) {
  .backup-grid { grid-template-columns: 1fr; }
}

.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.header-btns {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 6px;
}

.link-btn {
  background: none;
  border: none;
  color: var(--accent-primary);
  cursor: pointer;
  font-family: inherit;
  font-size: 12px;
  padding: 2px 4px;
}

.link-btn:hover { text-decoration: underline; }

.separator { color: var(--text-muted); font-size: 12px; }

.items-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.backup-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border: none;
  border-radius: var(--radius-md);
  background: transparent;
  cursor: pointer;
  font-family: inherit;
  text-align: left;
  width: 100%;
  transition: all var(--transition-fast);
  color: var(--text-secondary);
}

.backup-item:hover {
  background: var(--bg-tertiary);
}

.backup-item.checked .check-icon {
  color: var(--accent-primary);
}

.backup-item:not(.checked) .check-icon {
  color: var(--text-muted);
}

.item-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.item-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}

.item-desc {
  font-size: 11px;
  color: var(--text-muted);
}

.backup-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: 16px;
  margin-top: 12px;
  border-top: 1px solid var(--border);
}

.selected-count {
  font-size: 12px;
  color: var(--text-muted);
}

.backup-progress {
  margin-top: 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.progress-status {
  font-size: 13px;
  color: var(--text-secondary);
}

.backup-result {
  margin-top: 16px;
  padding: 16px;
  background: var(--success-muted);
  border-radius: var(--radius-md);
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.result-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.result-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--success);
}

.result-path {
  font-size: 11px;
  color: var(--text-secondary);
}

.result-items {
  font-size: 12px;
  color: var(--text-muted);
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 40px;
  color: var(--text-muted);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 40px;
  color: var(--text-muted);
  font-size: 13px;
}

.backups-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.backup-entry {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-radius: var(--radius-md);
  transition: background var(--transition-fast);
}

.backup-entry:hover {
  background: var(--bg-tertiary);
}

.entry-icon {
  width: 36px;
  height: 36px;
  border-radius: var(--radius-md);
  background: var(--accent-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.entry-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.entry-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}

.entry-meta {
  font-size: 11px;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  gap: 4px;
}

.font-mono {
  font-family: "JetBrains Mono", monospace;
}

.export-format-row {
  display: flex; align-items: center; gap: 10px; flex-wrap: wrap;
  margin-top: 12px; border-top: 1px solid var(--border); padding-top: 12px;
}
.format-tabs { display: flex; gap: 4px; }
.fmt-btn {
  padding: 3px 10px; border: 1px solid var(--border); border-radius: var(--radius-sm);
  background: var(--bg-tertiary); color: var(--text-secondary); cursor: pointer;
  font-size: 12px; font-family: monospace; transition: all var(--transition-fast);
}
.fmt-btn.active { border-color: var(--accent-primary); color: var(--accent-primary); background: var(--accent-muted); }
.fmt-btn:hover:not(.active) { border-color: var(--text-muted); color: var(--text-primary); }

.path-row {
  display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-top: 10px;
}
.path-code {
  font-size: 11px; color: var(--accent-primary); flex: 1;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  font-family: "JetBrains Mono", monospace;
}

/* Preset buttons */
.preset-btns {
  display: flex;
  gap: 4px;
}

.preset-btn {
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: var(--bg-tertiary);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 11px;
  font-family: inherit;
  font-weight: 500;
  transition: all var(--transition-fast);
}

.preset-btn:hover { border-color: var(--text-muted); color: var(--text-primary); }

.preset-essential:hover { border-color: var(--success); color: var(--success); background: rgba(var(--success-rgb, 34,197,94), 0.1); }
.preset-dev:hover       { border-color: var(--accent-primary); color: var(--accent-primary); background: var(--accent-muted); }
.preset-full:hover      { border-color: var(--warning, #f59e0b); color: var(--warning, #f59e0b); background: rgba(245,158,11,0.08); }

/* Category headers */
.category-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px 4px;
  margin-top: 8px;
}

.category-header:first-child { margin-top: 0; }

.cat-label {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--accent-primary);
  opacity: 0.8;
}

.cat-count {
  font-size: 10px;
  color: var(--text-muted);
  font-family: "JetBrains Mono", monospace;
}

.sensitive-badge {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 9px;
  font-weight: 700;
  padding: 1px 5px;
  border-radius: 3px;
  background: rgba(255, 193, 7, 0.12);
  border: 1px solid rgba(255, 193, 7, 0.4);
  color: var(--warning, #f59e0b);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  white-space: nowrap;
}
</style>
