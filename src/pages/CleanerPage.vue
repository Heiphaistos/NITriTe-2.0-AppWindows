<script setup lang="ts">
defineOptions({ name: "CleanerPage" });
import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@/utils/invoke";
import type { CommandResult } from "@/types/diagnostic";
import { listen } from "@tauri-apps/api/event";
import { confirm } from "@tauri-apps/plugin-dialog";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NProgress from "@/components/ui/NProgress.vue";
import NSkeleton from "@/components/ui/NSkeleton.vue";
import { useNotificationStore } from "@/stores/notifications";
import { Trash2, RefreshCw, Search, FileText, CheckSquare, Square, HardDrive, Zap, Shield, ChevronDown, Eye } from "lucide-vue-next";

const notify = useNotificationStore();

interface CleanTarget { name: string; path: string; size_mb: number; file_count: number; category: string; }
interface LargeFile { name: string; path: string; mb: number; ext: string; mod: string; }
interface CleanResult { target: string; success: boolean; freed_mb: number; files_deleted: number; message: string; }
interface CleanerProgress { scanned: number; total: number; item: CleanTarget | null; done: boolean; }

const targets    = ref<CleanTarget[]>([]);
const largeFiles = ref<LargeFile[]>([]);

// État de scan streaming
const scanProgress = ref(0);
const scanTotal    = ref(12);
const scanning     = ref(false);

const loadingLarge = ref(false);
const largeScanned = ref(false); // true après premier scan manuel
const selected     = ref<Set<string>>(new Set());
const cleaning     = ref(false);
const cleanProgress = ref(0);
const totalFreed   = ref(0);
const largeMinMb   = ref(100);
const quarantineMode = ref(false);

// Prévisualisation des fichiers d'une cible
const expandedTarget = ref<string | null>(null);
const previewFiles = ref<Record<string, string[]>>({});
const previewLoading = ref<string | null>(null);

async function togglePreview(t: CleanTarget) {
  if (expandedTarget.value === t.name) { expandedTarget.value = null; return; }
  expandedTarget.value = t.name;
  if (previewFiles.value[t.name]) return; // déjà chargé
  previewLoading.value = t.name;
  try {
    const r = await invoke<CommandResult>("run_system_command", {
      cmd: "powershell",
      args: ["-NoProfile", "-Command",
        `Get-ChildItem -LiteralPath '${t.path.replace(/'/g, "''")}' -Recurse -File -EA SilentlyContinue | Select-Object -First 100 | ForEach-Object { $_.FullName }`],
    });
    const lines: string[] = (r?.stdout ?? "").split("\n").map((l: string) => l.trim()).filter((l: string) => l.length > 0);
    previewFiles.value[t.name] = lines;
  } catch {
    previewFiles.value[t.name] = ["Impossible de lister les fichiers"];
  }
  previewLoading.value = null;
}

interface QuarantineEntry { name: string; path: string; file_count: number; size_mb: number; }
const quarantineList = ref<QuarantineEntry[]>([]);
const quarantineLoading = ref(false);

async function loadQuarantine() {
  quarantineLoading.value = true;
  try { quarantineList.value = await invoke<QuarantineEntry[]>("list_quarantine"); }
  catch { quarantineList.value = []; }
  quarantineLoading.value = false;
}

async function clearQuarantineEntry(name: string | null) {
  // Suppression permanente et irréversible — la quarantaine existe justement
  // comme alternative récupérable à la suppression directe, donc la vider
  // mérite la même confirmation que les autres suppressions définitives du
  // repo (deleteDll, deleteTask, removeStartup...).
  const msg = name
    ? `Supprimer définitivement "${name}" de la quarantaine ?\n\nCette action est irréversible.`
    : "Vider toute la quarantaine ?\n\nTous les éléments seront supprimés définitivement. Cette action est irréversible.";
  if (!(await confirm(msg, { title: "Nitrite", kind: "warning" }))) return;
  try {
    await invoke("clear_quarantine", { entryName: name });
    await loadQuarantine();
    notify.success("Quarantaine vidée");
  } catch (e: unknown) { notify.error("Erreur", String(e)); }
}

let unlistenCleaner: (() => void) | null = null;

const grouped = computed(() => {
  const groups: Record<string, CleanTarget[]> = {};
  for (const t of targets.value) {
    if (!groups[t.category]) groups[t.category] = [];
    groups[t.category].push(t);
  }
  return groups;
});

const selectedSizeMb = computed(() =>
  targets.value.filter(t => selected.value.has(t.name)).reduce((s, t) => s + t.size_mb, 0)
);

function formatMb(mb: number) {
  if (mb < 1) return `${Math.round(mb * 1024)} Ko`;
  if (mb < 1024) return `${mb.toFixed(1)} Mo`;
  return `${(mb / 1024).toFixed(2)} Go`;
}

async function load() {
  // Reset état
  targets.value = [];
  selected.value = new Set();
  scanning.value = true;
  scanProgress.value = 0;

  // Écouter les événements streaming
  if (unlistenCleaner) { unlistenCleaner(); unlistenCleaner = null; }
  unlistenCleaner = await listen<CleanerProgress>("cleaner:progress", (ev) => {
    const p = ev.payload;
    scanProgress.value = p.scanned;
    scanTotal.value = p.total;
    if (p.item) {
      targets.value.push(p.item);
      // Auto-sélection des cibles sûres
      if (["Temp", "Logs", "Cache système"].includes(p.item.category))
        selected.value = new Set([...selected.value, p.item.name]);
    }
    if (p.done) scanning.value = false;
  });

  // Lancer le scan en fire-and-forget (non-bloquant → UI reste fluide)
  invoke("scan_clean_targets_stream").catch(() => { scanning.value = false; });
}

async function loadLargeFiles() {
  loadingLarge.value = true;
  largeScanned.value = true;
  try {
    largeFiles.value = await invoke<LargeFile[]>("get_large_files", {
      folder: "C:\\Users",
      minSizeMb: largeMinMb.value,
    });
  } catch (e: unknown) {
    notify.error("Erreur", String(e));
  }
  loadingLarge.value = false;
}

async function clean() {
  const names = [...selected.value];
  if (!names.length) return;
  cleaning.value = true; cleanProgress.value = 0; totalFreed.value = 0;
  const cmd = quarantineMode.value ? "quarantine_target" : "clean_target";
  let failCount = 0;
  for (let i = 0; i < names.length; i++) {
    try {
      const r = await invoke<CleanResult>(cmd, { targetName: names[i] });
      if (r.success) totalFreed.value += r.freed_mb;
      else failCount++;
    } catch {
      failCount++;
    }
    cleanProgress.value = Math.round(((i + 1) / names.length) * 100);
  }
  const action = quarantineMode.value ? "mis en quarantaine" : "libérés";
  const msg = `${formatMb(totalFreed.value)} ${action}`;
  const label = quarantineMode.value ? "Quarantaine" : "Nettoyage";
  if (failCount === 0) {
    notify.success(`${label} terminé${quarantineMode.value ? "e" : ""}`, msg);
  } else if (failCount < names.length) {
    notify.error(`${label} partiel${quarantineMode.value ? "le" : ""}`, `${msg} — ${failCount} cible(s) en échec`);
  } else {
    notify.error(`Échec du ${label.toLowerCase()}`, `Aucune cible n'a pu être traitée (${names.length} échec(s))`);
  }
  cleaning.value = false;
  if (quarantineMode.value) await loadQuarantine();
  load();
}

function openFolder(filePath: string) {
  const folder = filePath.split("\\").slice(0, -1).join("\\");
  invoke("open_path", { path: folder }).catch(() => {});
}

onMounted(() => { load(); loadQuarantine(); });
onUnmounted(() => { if (unlistenCleaner) unlistenCleaner(); });
</script>

<template>
  <div class="cleaner-page">
    <div class="page-header">
      <div class="header-icon"><Trash2 :size="22" /></div>
      <div>
        <h1>Nettoyeur Avancé</h1>
        <p class="subtitle">Libérez de l'espace disque en supprimant les fichiers inutiles</p>
      </div>
      <NButton variant="ghost" size="sm" :loading="scanning" @click="load" style="margin-left:auto">
        <RefreshCw :size="13" /> Analyser
      </NButton>
    </div>

    <!-- Barre d'action -->
    <div class="action-bar">
      <div class="space-info">
        <HardDrive :size="14" />
        <span v-if="selected.size">
          <strong>{{ formatMb(selectedSizeMb) }}</strong> sélectionnés
        </span>
        <span v-if="totalFreed > 0" class="freed-badge">
          <Zap :size="12" /> {{ formatMb(totalFreed) }} libérés
        </span>
      </div>
      <div style="display:flex;gap:8px;align-items:center">
        <button
          class="quarantine-toggle"
          :class="{ 'quarantine-on': quarantineMode }"
          @click="quarantineMode = !quarantineMode"
          :title="quarantineMode ? 'Mode quarantaine actif — déplacer vers quarantaine' : 'Mode suppression directe'"
        >
          <Shield :size="13" />
          {{ quarantineMode ? 'Quarantaine' : 'Suppression' }}
        </button>
        <NButton variant="ghost" size="sm" @click="selected = new Set(targets.map(t => t.name))">Tout</NButton>
        <NButton variant="ghost" size="sm" @click="selected = new Set()">Aucun</NButton>
        <NButton
          :variant="quarantineMode ? 'secondary' : 'danger'" size="sm"
          :loading="cleaning"
          :disabled="selected.size === 0 || cleaning"
          @click="clean"
        >
          <component :is="quarantineMode ? Shield : Trash2" :size="13" />
          {{ quarantineMode ? 'Quarantaine' : 'Nettoyer' }} ({{ selected.size }})
        </NButton>
      </div>
    </div>

    <!-- Progression nettoyage -->
    <NProgress v-if="cleaning" :value="cleanProgress" showLabel size="sm" />

    <!-- Barre de progression du scan — ne bloque pas l'UI -->
    <div v-if="scanning" class="scan-progress-bar">
      <div class="scan-label">
        <Search :size="13" style="color:var(--accent-primary)" />
        <span>Analyse en cours… {{ scanProgress }}/{{ scanTotal }}</span>
      </div>
      <NProgress :value="scanProgress" :max="scanTotal" size="sm" style="flex:1" />
    </div>

    <!-- Cibles groupées par catégorie -->
    <div class="targets-sections">
      <NCard v-for="(items, cat) in grouped" :key="cat">
        <template #header>
          <div class="section-header">
            <span class="cat-label">{{ cat }}</span>
            <NBadge variant="neutral">{{ items.length }}</NBadge>
            <span class="cat-size">{{ formatMb(items.reduce((s, i) => s + i.size_mb, 0)) }}</span>
          </div>
        </template>
        <div class="targets-list">
          <div v-for="t in items" :key="t.name" class="target-wrapper">
            <button
              class="target-row"
              :class="{ checked: selected.has(t.name) }"
              @click="selected.has(t.name) ? selected.delete(t.name) : selected.add(t.name); selected = new Set(selected)"
            >
              <component :is="selected.has(t.name) ? CheckSquare : Square" :size="15" class="check-ic" />
              <div class="target-info">
                <span class="target-label">{{ t.name }}</span>
                <span class="target-desc">{{ t.path || '—' }}</span>
              </div>
              <div class="target-meta">
                <span class="target-size">{{ formatMb(t.size_mb) }}</span>
                <NBadge v-if="t.file_count > 0" variant="neutral" size="sm">{{ t.file_count }} fichiers</NBadge>
              </div>
              <button
                class="preview-btn"
                :title="expandedTarget === t.name ? 'Masquer les fichiers' : 'Voir les fichiers'"
                @click.stop="togglePreview(t)"
              >
                <Eye v-if="expandedTarget !== t.name" :size="13" />
                <ChevronDown v-else :size="13" style="transform:rotate(180deg)" />
              </button>
            </button>
            <!-- Liste des fichiers prévisualisés -->
            <div v-if="expandedTarget === t.name" class="preview-panel">
              <div v-if="previewLoading === t.name" class="preview-loading">
                <RefreshCw :size="12" style="animation:spin 1s linear infinite" /> Chargement…
              </div>
              <div v-else-if="previewFiles[t.name]?.length" class="preview-files">
                <div class="preview-header">
                  Fichiers qui seront supprimés ({{ previewFiles[t.name].length >= 100 ? '100+' : previewFiles[t.name].length }}) :
                </div>
                <div v-for="f in previewFiles[t.name]" :key="f" class="preview-file">{{ f }}</div>
                <div v-if="previewFiles[t.name].length >= 100" class="preview-more">+ d'autres fichiers non listés…</div>
              </div>
              <div v-else class="preview-empty">Dossier vide ou inaccessible</div>
            </div>
          </div>
        </div>
      </NCard>
    </div>

    <!-- Quarantaine -->
    <NCard v-if="quarantineList.length > 0 || quarantineLoading">
      <template #header>
        <div class="section-header">
          <Shield :size="15" style="color:var(--warning)" /><span>Quarantaine</span>
          <NBadge v-if="quarantineList.length > 0" variant="warning">{{ quarantineList.length }}</NBadge>
          <NButton variant="ghost" size="sm" style="margin-left:auto" @click="loadQuarantine" :loading="quarantineLoading">
            <RefreshCw :size="12" />
          </NButton>
          <NButton variant="danger" size="sm" @click="clearQuarantineEntry(null)" :disabled="quarantineList.length === 0">
            <Trash2 :size="12" /> Tout supprimer
          </NButton>
        </div>
      </template>
      <div v-if="quarantineLoading" class="empty-state" style="padding:16px">Chargement...</div>
      <div v-else-if="!quarantineList.length" class="empty-state" style="padding:16px">Quarantaine vide.</div>
      <div v-else class="targets-list">
        <div v-for="q in quarantineList" :key="q.name" class="target-row" style="cursor:default">
          <Shield :size="14" style="color:var(--warning);flex-shrink:0" />
          <div class="target-info">
            <span class="target-label">{{ q.name }}</span>
            <span class="target-desc">{{ q.file_count }} fichiers — {{ formatMb(q.size_mb) }}</span>
          </div>
          <NButton variant="ghost" size="sm" @click="clearQuarantineEntry(q.name)" title="Supprimer définitivement">
            <Trash2 :size="12" />
          </NButton>
        </div>
      </div>
    </NCard>

    <!-- Gros fichiers — scan manuel uniquement (peut prendre 1-5 min sur C:\Users) -->
    <NCard>
      <template #header>
        <div class="section-header">
          <Search :size="15" /><span>Fichiers volumineux</span>
          <div style="margin-left:auto;display:flex;align-items:center;gap:8px">
            <span style="font-size:11px;color:var(--text-muted)">Min :</span>
            <select v-model="largeMinMb" class="mini-select">
              <option :value="50">50 Mo</option>
              <option :value="100">100 Mo</option>
              <option :value="500">500 Mo</option>
              <option :value="1000">1 Go</option>
            </select>
            <NButton variant="primary" size="sm" :loading="loadingLarge" @click="loadLargeFiles">
              <Search :size="12" /> Analyser
            </NButton>
          </div>
        </div>
      </template>
      <!-- Avant premier scan : invite à lancer manuellement -->
      <div v-if="!largeScanned && !loadingLarge" class="large-not-scanned">
        <Search :size="28" style="opacity:.2" />
        <p>Cliquez sur <strong>Analyser</strong> pour rechercher les fichiers volumineux dans <code>C:\Users</code></p>
        <p class="large-warning">Cette analyse peut prendre 1 à 5 minutes. Vous pouvez continuer à utiliser l'application pendant ce temps.</p>
      </div>
      <div v-else-if="loadingLarge" class="loading-state">
        <div class="large-scan-info">
          <Search :size="16" style="color:var(--accent-primary)" />
          <span>Scan de C:\Users en cours… Vous pouvez naviguer librement.</span>
        </div>
        <NSkeleton v-for="i in 5" :key="i" height="36px" style="margin-bottom:4px" />
      </div>
      <div v-else-if="!largeFiles.length" class="empty-state"><FileText :size="24" style="opacity:.25" /><p>Aucun fichier > {{ largeMinMb }} Mo trouvé</p></div>
      <div v-else class="large-files-list">
        <div v-for="f in largeFiles" :key="f.path" class="large-file-row">
          <span class="file-ext">{{ f.ext || '—' }}</span>
          <span class="file-path">{{ f.path }}</span>
          <span class="file-size">{{ formatMb(f.mb) }}</span>
          <NButton variant="ghost" size="sm" @click="openFolder(f.path)">Ouvrir</NButton>
        </div>
      </div>
    </NCard>
  </div>
</template>

<style scoped>
.cleaner-page { display: flex; flex-direction: column; gap: 14px; }
.page-header { display: flex; align-items: center; gap: 12px; }
.header-icon { width: 42px; height: 42px; border-radius: var(--radius-lg); background: var(--danger-muted); display: flex; align-items: center; justify-content: center; color: var(--danger); flex-shrink: 0; }
h1 { font-size: 22px; font-weight: 700; }
.subtitle { font-size: 12px; color: var(--text-muted); }
.action-bar { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 10px 14px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius-md); }
.space-info { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--text-secondary); }
.freed-badge { display: flex; align-items: center; gap: 4px; color: var(--success); font-weight: 700; }
.section-header { display: flex; align-items: center; gap: 8px; width: 100%; }
.cat-label { font-size: 13px; font-weight: 700; color: var(--text-primary); }
.cat-size { margin-left: auto; font-size: 11px; color: var(--text-muted); font-family: "JetBrains Mono", monospace; }
.targets-list { display: flex; flex-direction: column; gap: 2px; }
.target-row { display: flex; align-items: center; gap: 10px; padding: 9px 10px; border: none; border-radius: var(--radius-md); background: transparent; cursor: pointer; font-family: inherit; text-align: left; width: 100%; transition: background var(--transition-fast); }
.target-row:hover { background: var(--bg-tertiary); }
.target-row.checked .check-ic { color: var(--accent-primary); }
.check-ic { color: var(--text-muted); flex-shrink: 0; }
.target-info { flex: 1; display: flex; flex-direction: column; gap: 2px; }
.target-label { font-size: 13px; font-weight: 500; color: var(--text-primary); }
.target-desc { font-size: 11px; color: var(--text-muted); }
.target-meta { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
.target-size { font-family: "JetBrains Mono", monospace; font-size: 12px; color: var(--text-secondary); }
.mini-select { padding: 4px 8px; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-primary); font-size: 12px; cursor: pointer; }
.large-files-list { display: flex; flex-direction: column; gap: 2px; }
.large-file-row { display: grid; grid-template-columns: 60px 1fr 80px auto; align-items: center; gap: 10px; padding: 7px 10px; border-bottom: 1px solid var(--border); font-size: 12px; }
.large-file-row:last-child { border-bottom: none; }
.file-ext { font-family: monospace; background: var(--bg-elevated); border-radius: var(--radius-sm); padding: 2px 6px; font-size: 10px; font-weight: 700; color: var(--text-muted); text-align: center; }
.file-path { color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: "JetBrains Mono", monospace; font-size: 11px; }
.file-size { font-family: "JetBrains Mono", monospace; color: var(--danger); font-weight: 700; text-align: right; }
.loading-state, .empty-state { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 30px; color: var(--text-muted); font-size: 13px; }
.scan-progress-bar { display: flex; align-items: center; gap: 10px; padding: 8px 12px; background: color-mix(in srgb,var(--accent-primary) 8%,var(--bg-secondary)); border: 1px solid color-mix(in srgb,var(--accent-primary) 30%,var(--border)); border-radius: var(--radius-md); }
.scan-label { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--accent-primary); white-space: nowrap; font-weight: 500; }
.large-not-scanned { display: flex; flex-direction: column; align-items: center; gap: 10px; padding: 32px 20px; color: var(--text-muted); font-size: 13px; text-align: center; }
.large-not-scanned code { color: var(--accent-primary); background: var(--bg-tertiary); padding: 1px 6px; border-radius: 4px; font-size: 12px; }
.large-warning { font-size: 11px; color: color-mix(in srgb,var(--success) 80%,var(--text-muted)); background: var(--success-muted); padding: 6px 12px; border-radius: 6px; }
.large-scan-info { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--accent-primary); margin-bottom: 8px; }
.quarantine-toggle { display: flex; align-items: center; gap: 5px; padding: 5px 10px; border: 1px solid var(--border); border-radius: var(--radius-sm); background: var(--bg-tertiary); color: var(--text-secondary); cursor: pointer; font-size: 12px; font-family: inherit; transition: all .15s; }
.quarantine-toggle:hover { border-color: var(--warning); color: var(--warning); }
.quarantine-toggle.quarantine-on { border-color: var(--warning); color: var(--warning); background: rgba(255,193,7,.08); }
.target-wrapper { display: flex; flex-direction: column; }
.preview-btn { background: none; border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-muted); cursor: pointer; display: flex; align-items: center; padding: 3px 5px; flex-shrink: 0; transition: all .15s; }
.preview-btn:hover { border-color: var(--accent-primary); color: var(--accent-primary); }
.preview-panel { background: var(--bg-tertiary); border: 1px solid var(--border); border-top: none; border-radius: 0 0 var(--radius-md) var(--radius-md); padding: 10px 12px; }
.preview-loading { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text-muted); }
.preview-header { font-size: 11px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: .06em; margin-bottom: 6px; }
.preview-files { max-height: 200px; overflow-y: auto; display: flex; flex-direction: column; gap: 1px; }
.preview-file { font-family: "JetBrains Mono", monospace; font-size: 11px; color: var(--text-secondary); padding: 2px 4px; border-radius: 3px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.preview-file:hover { background: var(--bg-secondary); color: var(--text-primary); }
.preview-more { font-size: 11px; color: var(--text-muted); font-style: italic; margin-top: 4px; }
.preview-empty { font-size: 12px; color: var(--text-muted); font-style: italic; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
</style>
