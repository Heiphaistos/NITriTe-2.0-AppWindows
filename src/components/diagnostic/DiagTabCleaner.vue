<template>
  <div class="cl-root">
    <!-- Banner -->
    <div class="cl-banner">
      <div class="cl-banner-icon"><Trash2 :size="26" /></div>
      <div class="cl-banner-text">
        <div class="cl-banner-title">Nettoyeur Système</div>
        <div class="cl-banner-desc">
          <span v-if="!targets.length">Analysez et libérez de l'espace disque</span>
          <span v-else>{{ totalMb.toFixed(0) }} MB détectés — {{ selectedTargets.size }} sélectionné(s) ({{ totalSelectedMb.toFixed(0) }} MB)</span>
        </div>
      </div>
      <div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap">
        <button class="cl-btn cl-btn-primary" :disabled="loading || cleaning !== null" @click="loadTargets">
          <RefreshCw :size="13" /> Analyser
        </button>
        <button v-if="targets.length > 0" class="cl-btn" @click="toggleAll" :disabled="cleaning !== null">
          <component :is="allSelected ? CheckSquare : Square" :size="13" />
          {{ allSelected ? 'Désélectionner tout' : 'Tout sélectionner' }}
        </button>
        <button v-if="selectedTargets.size > 0 && cleanTotal === 0" class="cl-btn cl-btn-danger" :disabled="cleaning !== null" @click="cleanSelected">
          <Trash2 :size="13" /> Nettoyer ({{ selectedTargets.size }}) · {{ totalSelectedMb.toFixed(0) }} MB
        </button>
        <div v-if="cleanTotal > 0" class="cl-batch-progress">
          <div class="cl-spinner-sm" />
          <span>{{ cleanProgress }}/{{ cleanTotal }} nettoyé(s)&nbsp;–&nbsp;<em>{{ cleaning }}</em></span>
          <div class="cl-batch-bar"><div class="cl-batch-fill" :style="{ width: (cleanProgress/cleanTotal*100)+'%' }" /></div>
        </div>
        <button class="cl-btn" :class="showBrowserSection ? 'cl-btn-primary' : ''" @click="toggleBrowserSection">
          <Globe :size="13" /> Cache navigateurs
        </button>
      </div>
    </div>

    <div v-if="loading" class="cl-loading"><div class="cl-spinner" /> Analyse du système en cours...</div>

    <div v-else-if="targets.length > 0">
      <!-- Summary stats -->
      <div class="cl-stats">
        <div v-for="cat in categories" :key="cat" class="cl-stat">
          <div class="cl-stat-val">{{ targetsByCategory(cat).reduce((s,t)=>s+t.size_mb,0).toFixed(0) }} MB</div>
          <div class="cl-stat-lbl">{{ cat }}</div>
        </div>
      </div>

      <!-- Targets by category -->
      <div v-for="cat in categories" :key="cat" class="cl-category">
        <div class="cl-cat-header" @click="toggleCategory(cat)" style="cursor:pointer" title="Tout sélectionner / déselectionner cette catégorie">
          <span class="cl-cat-dot" />
          {{ cat }}
          <span class="cl-cat-count">{{ targetsByCategory(cat).length }} éléments</span>
          <span class="cl-cat-sel">{{ targetsByCategory(cat).filter(t => selectedTargets.has(t.name)).length }}/{{ targetsByCategory(cat).length }} sélectionnés</span>
        </div>
        <div class="cl-items">
          <div v-for="t in targetsByCategory(cat)" :key="t.name"
               class="cl-item" :class="{ 'cl-item-sel': selectedTargets.has(t.name), 'cl-item-done': results[t.name]?.success }"
               @click="toggleTarget(t.name)">
            <div class="cl-checkbox" :class="selectedTargets.has(t.name) ? 'cb-checked' : ''">
              <span v-if="selectedTargets.has(t.name)">✓</span>
            </div>
            <div class="cl-item-info">
              <div class="cl-item-name">{{ t.name }}</div>
              <div class="cl-item-path" :title="t.path">{{ t.path || 'Corbeille système' }}</div>
            </div>
            <div class="cl-item-size">
              <span :class="t.size_mb > 500 ? 'sz-xl' : t.size_mb > 100 ? 'sz-lg' : t.size_mb > 10 ? 'sz-md' : 'sz-sm'">
                {{ t.size_mb >= 1024 ? (t.size_mb/1024).toFixed(1)+' GB' : t.size_mb.toFixed(0)+' MB' }}
              </span>
              <span class="cl-fcount">{{ t.file_count }} fichiers</span>
            </div>
            <div class="cl-item-action" @click.stop>
              <div v-if="cleaning === t.name" class="cl-spinner-sm" />
              <div v-else-if="results[t.name]?.success" class="cl-freed">
                <span class="cl-check">✓</span>
                {{ results[t.name].freed_mb.toFixed(0) }} MB libérés
              </div>
              <button v-else class="cl-btn cl-btn-sm cl-btn-clean" :disabled="cleaning !== null" @click="cleanOne(t.name)">
                Nettoyer
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-else-if="!loading" class="cl-empty">
      <Trash2 :size="28" style="opacity:.2" />
      <span>Cliquez sur "Analyser" pour rechercher les fichiers temporaires</span>
    </div>

    <!-- Browser cache section -->
    <div v-if="showBrowserSection" class="cl-browser">
      <div class="cl-browser-header">
        <Globe :size="14" /> Cache des navigateurs
        <button class="cl-btn cl-btn-primary" style="margin-left:auto;padding:4px 10px;font-size:11px" :disabled="browserLoading" @click="loadBrowserCaches">
          <RefreshCw :size="11" /> Actualiser
        </button>
      </div>
      <div v-if="browserLoading" class="cl-loading" style="padding:12px 16px"><div class="cl-spinner-sm" /> Analyse des caches...</div>
      <div v-else class="cl-browser-list">
        <div v-for="b in browserCaches.filter(b => b.exists)" :key="b.browser" class="cl-browser-row">
          <div class="cl-browser-icon">{{ b.browser[0] }}</div>
          <div class="cl-browser-info">
            <div class="cl-browser-name">{{ b.browser }}</div>
            <div class="cl-browser-path">{{ b.path }}</div>
          </div>
          <span :class="b.size_mb > 500 ? 'sz-xl' : b.size_mb > 100 ? 'sz-lg' : 'sz-sm'">
            {{ b.size_mb >= 1024 ? (b.size_mb/1024).toFixed(1)+'GB' : b.size_mb.toFixed(0)+'MB' }}
          </span>
          <button class="cl-btn cl-btn-sm cl-btn-danger" :disabled="cleaningBrowser === b.browser || b.size_mb === 0" @click="cleanBrowserCache(b)">
            <div v-if="cleaningBrowser === b.browser" class="cl-spinner-sm" />
            <Trash2 v-else :size="11" />
            Vider
          </button>
        </div>
        <div v-if="!browserCaches.some(b => b.exists)" style="padding:16px;text-align:center;color:var(--text-muted);font-size:12px">
          Aucun navigateur détecté
        </div>
      </div>
    </div>

    <!-- Large Files finder -->
    <div class="cl-finder">
      <div class="cl-finder-header"><Search :size="14" /> Recherche de grands fichiers</div>
      <div class="cl-finder-body">
        <input v-model="largeFolder" class="cl-input" placeholder="Dossier (ex: C:\Users)" style="flex:2" />
        <input v-model.number="minSizeMb" type="number" class="cl-input" placeholder="MB min" style="width:90px" />
        <button class="cl-btn cl-btn-primary" :disabled="largLoading" @click="findLarge">
          <Search :size="13" /> Chercher
        </button>
      </div>
      <div v-if="largLoading" class="cl-finder-loading"><div class="cl-spinner-sm" /> Recherche en cours...</div>
      <div v-else-if="largeFiles.length > 0" class="cl-finder-results">
        <div v-for="f in largeFiles" :key="f.path" class="cl-large-file">
          <div class="cl-lf-ext">{{ f.ext || '?' }}</div>
          <div class="cl-lf-info">
            <div class="cl-lf-name">{{ f.name }}</div>
            <div class="cl-lf-path">{{ f.path }}</div>
          </div>
          <div class="cl-lf-right">
            <span class="cl-lf-size">{{ f.mb }} MB</span>
            <span class="cl-lf-mod">{{ f.mod }}</span>
          </div>
        </div>
      </div>
      <div v-else-if="largeHasScanned" class="cl-finder-loading">Aucun fichier volumineux trouvé pour ce dossier/seuil.</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke, invokeRaw, isTauriContext } from "@/utils/invoke";
import { Trash2, RefreshCw, Search, CheckSquare, Square, Globe } from 'lucide-vue-next'
import { useNotificationStore } from '@/stores/notifications'

interface CleanTarget { name: string; path: string; size_mb: number; file_count: number; category: string }
interface CleanResult { target: string; freed_mb: number; files_deleted: number; success: boolean; message: string }
interface LargeFile { name: string; path: string; mb: number; ext: string; mod: string }
interface BrowserCacheInfo { browser: string; path: string; size_mb: number; exists: boolean }

const notify = useNotificationStore()

const targets = ref<CleanTarget[]>([]); const loading = ref(false)
const cleaning = ref<string | null>(null); const selectedTargets = ref(new Set<string>())
const results = ref<Record<string, CleanResult>>({})
const cleanProgress = ref(0); const cleanTotal = ref(0)
const largeFolder = ref('C:\\Users'); const minSizeMb = ref(100)
const largLoading = ref(false); const largeFiles = ref<LargeFile[]>([]); const largeHasScanned = ref(false)

// Browser cache
const browserCaches = ref<BrowserCacheInfo[]>([])
const browserLoading = ref(false)
const cleaningBrowser = ref<string | null>(null)
const showBrowserSection = ref(false)

const categories = computed(() => [...new Set(targets.value.map(t => t.category))])
const totalMb = computed(() => targets.value.reduce((s, t) => s + t.size_mb, 0))
const totalSelectedMb = computed(() => targets.value.filter(t => selectedTargets.value.has(t.name)).reduce((s, t) => s + t.size_mb, 0))
const allSelected = computed(() => targets.value.length > 0 && targets.value.every(t => selectedTargets.value.has(t.name)))

function targetsByCategory(cat: string) { return targets.value.filter(t => t.category === cat) }

function toggleTarget(name: string) {
  if (selectedTargets.value.has(name)) selectedTargets.value.delete(name)
  else selectedTargets.value.add(name)
  selectedTargets.value = new Set(selectedTargets.value)
}

function toggleAll() {
  if (allSelected.value) selectedTargets.value = new Set()
  else selectedTargets.value = new Set(targets.value.map(t => t.name))
}

function toggleCategory(cat: string) {
  const catItems = targetsByCategory(cat)
  const allCatSel = catItems.every(t => selectedTargets.value.has(t.name))
  const newSet = new Set(selectedTargets.value)
  if (allCatSel) catItems.forEach(t => newSet.delete(t.name))
  else catItems.forEach(t => newSet.add(t.name))
  selectedTargets.value = newSet
}

async function loadTargets() {
  loading.value = true
  try { targets.value = await invokeRaw<CleanTarget[]>('get_clean_targets') }
  catch (e: unknown) { if (isTauriContext()) notify.error("Analyse système", (e instanceof Error ? e.message : String(e)).slice(0, 120)); }
  finally { loading.value = false }
}

async function cleanOne(name: string) {
  cleaning.value = name
  try {
    const r = await invokeRaw<CleanResult>('clean_target', { targetName: name })
    results.value[name] = r
    // Mise à jour en place — évite le rechargement complet de la liste
    const idx = targets.value.findIndex(t => t.name === name)
    if (idx !== -1) targets.value[idx] = { ...targets.value[idx], size_mb: 0, file_count: 0 }
  } catch (e: unknown) {
    // Re-throw : cleanSelected() compte les échecs via son propre catch — ne
    // pas avaler l'erreur ici casserait son décompte failCount. La notif
    // couvre le clic direct (bouton "Nettoyer" seul), hors du batch.
    if (isTauriContext()) notify.error(`Erreur nettoyage ${name}`, (e instanceof Error ? e.message : String(e)).slice(0, 120));
    throw e;
  } finally { cleaning.value = null }
}

async function cleanSelected() {
  const names = Array.from(selectedTargets.value)
  cleanTotal.value = names.length
  cleanProgress.value = 0
  let failCount = 0
  for (const name of names) {
    try {
      await cleanOne(name)
      if (results.value[name]?.success === false) failCount++
    } catch {
      failCount++
    }
    cleanProgress.value++
  }
  selectedTargets.value.clear()
  cleanProgress.value = 0; cleanTotal.value = 0
  // Un seul rechargement à la fin pour avoir les tailles réelles
  await loadTargets()
  if (failCount === 0) {
    notify.success('Nettoyage terminé', `${names.length} élément(s) traités`)
  } else if (failCount < names.length) {
    notify.error('Nettoyage partiel', `${names.length - failCount}/${names.length} élément(s) traités — ${failCount} échec(s)`)
  } else {
    notify.error('Échec du nettoyage', `Aucun des ${names.length} élément(s) n'a pu être traité`)
  }
}

async function findLarge() {
  largLoading.value = true; largeHasScanned.value = true; largeFiles.value = []
  try { largeFiles.value = await invoke<LargeFile[]>('get_large_files', { folder: largeFolder.value, minSizeMb: minSizeMb.value }) }
  catch (e: unknown) { if (isTauriContext()) notify.error("Recherche grands fichiers", (e instanceof Error ? e.message : String(e)).slice(0, 120)); }
  finally { largLoading.value = false }
}

async function loadBrowserCaches() {
  browserLoading.value = true
  try { browserCaches.value = await invoke<BrowserCacheInfo[]>('get_browser_cache_info') }
  catch (e: unknown) {
    browserCaches.value = [];
    if (isTauriContext()) notify.error("Cache navigateurs", (e instanceof Error ? e.message : String(e)).slice(0, 120));
  }
  finally { browserLoading.value = false }
}

async function cleanBrowserCache(b: BrowserCacheInfo) {
  cleaningBrowser.value = b.browser
  try {
    const freed = await invoke<number>('clean_browser_cache_path', { browserPath: b.path })
    notify.success(`Cache ${b.browser} vidé`, `${freed.toFixed(0)} MB libérés`)
    await loadBrowserCaches()
  } catch (e: unknown) {
    notify.error(`Erreur ${b.browser}`, String(e))
  } finally { cleaningBrowser.value = null }
}

function toggleBrowserSection() {
  showBrowserSection.value = !showBrowserSection.value
  if (showBrowserSection.value && !browserCaches.value.length) loadBrowserCaches()
}

onMounted(loadTargets)
</script>

<style scoped>
.cl-root { display: flex; flex-direction: column; gap: 14px; }

.cl-banner { display: flex; align-items: center; gap: 16px; padding: 18px 22px;
  background: linear-gradient(135deg, rgba(249,115,22,.13), rgba(234,88,12,.07));
  border: 1px solid rgba(249,115,22,.3); border-radius: 14px; }
.cl-banner-icon { width: 48px; height: 48px; border-radius: 12px;
  background: linear-gradient(135deg,#f97316,#ea580c); display: flex; align-items: center;
  justify-content: center; color: #fff; flex-shrink: 0; box-shadow: 0 4px 14px rgba(249,115,22,.4); }
.cl-banner-text { flex: 1; }
.cl-banner-title { font-size: 17px; font-weight: 700; margin-bottom: 3px; }
.cl-banner-desc { font-size: 12px; opacity: .7; }

.cl-loading { display: flex; align-items: center; gap: 10px; padding: 20px; font-size: 13px; color: var(--text-muted); }
.cl-spinner { width: 15px; height: 15px; border: 2px solid rgba(255,255,255,.15); border-top-color: #f97316; border-radius: 50%; animation: spin .8s linear infinite; }
.cl-spinner-sm { width: 13px; height: 13px; border: 2px solid rgba(255,255,255,.15); border-top-color: #f97316; border-radius: 50%; animation: spin .8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.cl-stats { display: flex; gap: 8px; flex-wrap: wrap; }
.cl-stat { flex: 1; min-width: 100px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 10px; padding: 12px; text-align: center; }
.cl-stat-val { font-size: 18px; font-weight: 700; color: #f97316; }
.cl-stat-lbl { font-size: 10px; opacity: .5; text-transform: uppercase; margin-top: 2px; }

.cl-empty { display: flex; align-items: center; gap: 12px; padding: 28px; font-size: 13px; color: var(--text-muted);
  background: var(--bg-secondary); border-radius: 12px; border: 1px solid var(--border); }

.cl-category { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 12px; overflow: hidden; }
.cl-cat-header { display: flex; align-items: center; gap: 8px; padding: 10px 14px;
  background: var(--bg-tertiary); font-size: 11px; font-weight: 600; text-transform: uppercase;
  letter-spacing: .05em; opacity: .7; border-bottom: 1px solid var(--border); }
.cl-cat-dot { width: 7px; height: 7px; border-radius: 50%; background: #f97316; flex-shrink: 0; }
.cl-cat-count { margin-left: auto; font-weight: 400; opacity: .5; }

.cl-items { display: flex; flex-direction: column; }
.cl-item { display: flex; align-items: center; gap: 12px; padding: 10px 14px; cursor: pointer;
  border-bottom: 1px solid var(--border); transition: background 150ms; }
.cl-item:last-child { border-bottom: none; }
.cl-item:hover { background: var(--bg-tertiary); }
.cl-item-sel { background: rgba(249,115,22,.06) !important; }
.cl-item-done { opacity: .5; }

.cl-checkbox { width: 18px; height: 18px; border-radius: 4px; border: 1px solid var(--border); display: flex; align-items: center; justify-content: center; font-size: 11px; flex-shrink: 0; transition: all 150ms; }
.cb-checked { background: #f97316; border-color: #f97316; color: #fff; font-weight: 700; }

.cl-item-info { flex: 1; min-width: 0; }
.cl-item-name { font-size: 12px; font-weight: 500; margin-bottom: 2px; }
.cl-item-path { font-size: 10px; opacity: .45; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.cl-item-size { display: flex; flex-direction: column; align-items: flex-end; min-width: 70px; }
.sz-xl { color: #ef4444; font-size: 13px; font-weight: 700; }
.sz-lg { color: #f97316; font-size: 13px; font-weight: 600; }
.sz-md { color: #f59e0b; font-size: 12px; font-weight: 600; }
.sz-sm { font-size: 12px; opacity: .7; }
.cl-fcount { font-size: 10px; opacity: .4; margin-top: 1px; }

.cl-item-action { min-width: 110px; display: flex; justify-content: flex-end; align-items: center; }
.cl-freed { display: flex; align-items: center; gap: 5px; font-size: 11px; color: #22c55e; }
.cl-check { font-size: 14px; }

/* Large files finder */
.cl-finder { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 14px; overflow: hidden; }
.cl-finder-header { display: flex; align-items: center; gap: 8px; padding: 12px 16px; font-size: 12px; font-weight: 600; opacity: .7; border-bottom: 1px solid var(--border); background: var(--bg-tertiary); }
.cl-finder-body { display: flex; gap: 8px; padding: 14px; flex-wrap: wrap; }
.cl-input { background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 8px; padding: 8px 12px; color: var(--text-primary); font-size: 12px; outline: none; }
.cl-input:focus { border-color: #f97316; }
.cl-finder-loading { display: flex; align-items: center; gap: 8px; padding: 12px 16px; font-size: 12px; color: var(--text-muted); border-top: 1px solid var(--border); }

.cl-finder-results { border-top: 1px solid var(--border); max-height: 300px; overflow-y: auto; }
.cl-large-file { display: flex; align-items: center; gap: 12px; padding: 10px 16px; border-bottom: 1px solid var(--border); }
.cl-large-file:last-child { border-bottom: none; }
.cl-large-file:hover { background: var(--bg-tertiary); }
.cl-lf-ext { font-family: 'JetBrains Mono',monospace; font-size: 10px; background: rgba(249,115,22,.12); color: #f97316; border: 1px solid rgba(249,115,22,.25); border-radius: 5px; padding: 3px 8px; min-width: 36px; text-align: center; flex-shrink: 0; }
.cl-lf-info { flex: 1; min-width: 0; }
.cl-lf-name { font-size: 12px; font-weight: 500; }
.cl-lf-path { font-size: 10px; opacity: .45; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.cl-lf-right { display: flex; flex-direction: column; align-items: flex-end; }
.cl-lf-size { font-size: 13px; font-weight: 700; color: #f97316; }
.cl-lf-mod { font-size: 10px; opacity: .4; }

/* Buttons */
.cl-btn { display: inline-flex; align-items: center; gap: 5px; padding: 8px 14px; border-radius: 8px;
  border: 1px solid var(--border); background: var(--bg-secondary); color: var(--text-secondary);
  font-size: 12px; cursor: pointer; transition: all 150ms; font-family: inherit; }
.cl-btn:disabled { opacity: .4; cursor: not-allowed; }
.cl-btn-primary { background: rgba(249,115,22,.15); color: #f97316; border-color: rgba(249,115,22,.3); }
.cl-btn-primary:hover:not(:disabled) { background: rgba(249,115,22,.25); }
.cl-btn-danger { background: rgba(239,68,68,.12); color: #ef4444; border-color: rgba(239,68,68,.3); }
.cl-btn-danger:hover:not(:disabled) { background: rgba(239,68,68,.2); }
.cl-btn-sm { padding: 4px 10px; font-size: 11px; }
.cl-btn-clean { color: var(--text-muted); }
.cl-btn-clean:hover:not(:disabled) { color: #f97316; border-color: rgba(249,115,22,.4); }

/* Category select-all */
.cl-cat-sel { font-size: 10px; opacity: .5; margin-left: 8px; }
.cl-cat-header:hover { background: var(--bg-secondary); }

/* Batch clean progress */
.cl-batch-progress { display:flex; align-items:center; gap:8px; padding:6px 12px; background:rgba(249,115,22,.08); border:1px solid rgba(249,115,22,.2); border-radius:8px; font-size:12px; color:var(--text-secondary); flex-wrap:wrap; }
.cl-batch-bar { flex:1; min-width:80px; height:4px; background:rgba(255,255,255,.1); border-radius:2px; overflow:hidden; }
.cl-batch-fill { height:100%; background:#f97316; border-radius:2px; transition:width 300ms ease; }

/* Browser cache */
.cl-browser { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 12px; overflow: hidden; }
.cl-browser-header { display: flex; align-items: center; gap: 8px; padding: 10px 16px;
  font-size: 12px; font-weight: 600; background: var(--bg-tertiary); border-bottom: 1px solid var(--border); }
.cl-browser-list { display: flex; flex-direction: column; }
.cl-browser-row { display: flex; align-items: center; gap: 12px; padding: 10px 16px; border-bottom: 1px solid var(--border); }
.cl-browser-row:last-child { border-bottom: none; }
.cl-browser-row:hover { background: var(--bg-tertiary); }
.cl-browser-icon { width: 28px; height: 28px; border-radius: 6px; background: var(--accent-muted); display: flex; align-items: center; justify-content: center; font-size: 13px; font-weight: 700; color: var(--accent-primary); flex-shrink: 0; }
.cl-browser-info { flex: 1; min-width: 0; }
.cl-browser-name { font-size: 13px; font-weight: 500; }
.cl-browser-path { font-size: 10px; opacity: .4; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
