<template>
  <div class="drivers-root">

    <!-- Sub-tabs -->
    <div class="drivers-subtabs">
      <button v-for="t in SUBTABS" :key="t.id"
        class="drivers-subtab" :class="{ active: subTab === t.id }"
        @click="subTab = t.id as SubTab">
        <component :is="t.icon" :size="14" />
        {{ t.label }}
        <span v-if="t.badge" class="subtab-badge">{{ t.badge }}</span>
      </button>
    </div>

    <!-- ===== ONGLET SDI / Updater ===== -->
    <div v-if="subTab === 'updater'">
      <Suspense><DiagTabDriverUpdater /></Suspense>
    </div>

    <!-- ===== ONGLET 1 : Liste des pilotes ===== -->
    <div v-if="subTab === 'list'">
      <div v-if="loadingList" class="drivers-loading">
        <div class="drv-spinner" /> Analyse des pilotes installés...
      </div>
      <div v-else-if="listData">
        <!-- Stats bar -->
        <div class="drv-stats">
          <div class="drv-stat drv-stat-blue">
            <div class="drv-stat-val">{{ listData.total }}</div>
            <div class="drv-stat-lbl">Total</div>
          </div>
          <div class="drv-stat" :class="listData.error_count > 0 ? 'drv-stat-red' : 'drv-stat-green'">
            <div class="drv-stat-val">{{ listData.error_count }}</div>
            <div class="drv-stat-lbl">Erreurs</div>
          </div>
          <div class="drv-stat" :class="listData.unsigned_count > 0 ? 'drv-stat-orange' : 'drv-stat-green'">
            <div class="drv-stat-val">{{ listData.unsigned_count }}</div>
            <div class="drv-stat-lbl">Non signés</div>
          </div>
          <div class="drv-stat drv-stat-green">
            <div class="drv-stat-val">{{ listData.total - listData.error_count - listData.unsigned_count }}</div>
            <div class="drv-stat-lbl">OK</div>
          </div>
        </div>

        <!-- Filters -->
        <div class="drv-filters">
          <div class="drv-search">
            <Search :size="13" />
            <input v-model="listSearch" placeholder="Rechercher nom, fournisseur, classe..." />
          </div>
          <div class="drv-filter-btns">
            <button v-for="f in LIST_FILTERS" :key="f.k"
              class="drv-filter-btn" :class="{ active: listFilter === f.k }"
              @click="listFilter = f.k">
              {{ f.l }}
              <span v-if="f.k === 'errors' && listData.error_count > 0" class="filter-count red">{{ listData.error_count }}</span>
              <span v-if="f.k === 'unsigned' && listData.unsigned_count > 0" class="filter-count orange">{{ listData.unsigned_count }}</span>
            </button>
          </div>
        </div>

        <!-- Table -->
        <div class="drv-table-wrap">
          <table class="drv-table">
            <thead>
              <tr>
                <th>Nom du pilote</th>
                <th>Fournisseur</th>
                <th>Classe</th>
                <th>Version</th>
                <th>Signé</th>
                <th>Statut</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="filteredList.length === 0">
                <td colspan="6" class="td-muted" style="text-align:center;padding:16px 8px">
                  {{ (listData?.drivers.length ?? 0) === 0 ? 'Aucun pilote détecté.' : 'Aucun pilote ne correspond à la recherche/au filtre.' }}
                </td>
              </tr>
              <tr v-for="d in filteredList.slice(0, 300)" :key="`drv-${d.name}-${d.provider}`"
                :class="{ 'row-error': d.config_error !== 0, 'row-unsigned': d.config_error === 0 && !d.signed }">
                <td class="td-name">
                  <AlertTriangle v-if="d.config_error !== 0" :size="11" class="icon-err" />
                  <Shield v-else-if="!d.signed" :size="11" class="icon-warn" />
                  {{ d.name }}
                </td>
                <td class="td-muted">{{ d.provider || '—' }}</td>
                <td class="td-class">{{ d.class || '—' }}</td>
                <td><code class="drv-code">{{ d.version || '—' }}</code></td>
                <td>
                  <span class="drv-badge" :class="d.signed ? 'badge-ok' : 'badge-warn'">
                    {{ d.signed ? '✓ Signé' : '⚠ Non signé' }}
                  </span>
                </td>
                <td>
                  <span class="drv-badge" :class="d.config_error === 0 ? 'badge-ok' : 'badge-err'">
                    {{ d.config_error === 0 ? (d.status || 'OK') : `Erreur ${d.config_error}` }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
          <div v-if="filteredList.length > 300" class="drv-truncated">
            +{{ filteredList.length - 300 }} pilotes — affinez la recherche
          </div>
        </div>
      </div>
    </div>

    <!-- ===== ONGLET 2 : Périphériques avec problèmes ===== -->
    <div v-if="subTab === 'problems'">
      <div class="wu-section">
        <button class="drv-btn drv-btn-primary" :disabled="problemsLoading" @click="loadProblems">
          <RefreshCw :size="14" /> Vérifier les périphériques
        </button>
        <span v-if="problemsLoading" class="wu-loading"><div class="drv-spinner" /> Analyse WMI en cours...</span>
      </div>

      <div v-if="problemsLoaded">
        <div v-if="problemDevices.length === 0" class="wu-empty">
          <CheckCircle :size="18" style="color:#22c55e" />
          <span style="color:#22c55e;font-weight:600">Aucun problème de pilote détecté</span>
          <span style="color:var(--text-muted)">— Tous les périphériques fonctionnent correctement.</span>
        </div>
        <div v-else>
          <div class="wu-summary" style="grid-template-columns:repeat(2,1fr)">
            <div class="wu-sum-card sum-orange">
              <div class="wu-sum-val">{{ problemDevices.length }}</div>
              <div class="wu-sum-lbl">Périphérique(s) en erreur</div>
            </div>
            <div class="wu-sum-card sum-red">
              <div class="wu-sum-val">{{ new Set(problemDevices.map(d => d.error_code)).size }}</div>
              <div class="wu-sum-lbl">Code(s) d'erreur différents</div>
            </div>
          </div>
          <div class="wu-list">
            <div v-for="d in problemDevices" :key="d.device_id" class="wu-item" style="border-color:rgba(239,68,68,.2)">
              <div class="wu-item-info">
                <div class="wu-item-title" style="display:flex;align-items:center;gap:6px">
                  <AlertTriangle :size="14" style="color:#f59e0b;flex-shrink:0" />
                  {{ d.name }}
                </div>
                <div class="wu-item-meta">
                  <span class="wu-tag wu-tag-orange">Code {{ d.error_code }}</span>
                  <span v-if="d.class" class="wu-tag">{{ d.class }}</span>
                  <span class="wu-tag wu-tag-red">{{ d.error_description }}</span>
                </div>
                <div style="font-size:10px;color:var(--text-muted);margin-top:4px;font-family:monospace">{{ d.device_id }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>
      <div v-else class="wu-empty" style="color:var(--text-muted)">
        Cliquez sur "Vérifier les périphériques" pour analyser les erreurs de pilotes
      </div>
    </div>

    <!-- ===== ONGLET 2 : MAJ Windows Update ===== -->
    <div v-if="subTab === 'wu'">
      <div class="wu-section">
        <button class="drv-btn drv-btn-primary" :disabled="wuLoading" @click="checkWU">
          <RefreshCw :size="14" /> Vérifier les mises à jour
        </button>
        <button v-if="wuData && wuData.pending_count > 0" class="drv-btn drv-btn-orange"
          :disabled="wuLoading || installing" @click="installAllWU">
          <Download :size="14" /> Installer tout ({{ wuData.pending_count }})
        </button>
        <span v-if="wuLoading" class="wu-loading"><div class="drv-spinner" /> Interrogation WUA...</span>
        <span v-if="installMsg" class="wu-msg" :class="installErr ? 'wu-err' : 'wu-ok'">{{ installMsg }}</span>
      </div>

      <div v-if="wuData">
        <!-- Summary -->
        <div class="wu-summary">
          <div class="wu-sum-card" :class="wuData.pending_count > 0 ? 'sum-orange' : 'sum-green'">
            <div class="wu-sum-val">{{ wuData.pending_count }}</div>
            <div class="wu-sum-lbl">Mise(s) à jour en attente</div>
          </div>
          <div class="wu-sum-card sum-blue">
            <div class="wu-sum-val">{{ wuData.total_size_mb.toFixed(1) }} MB</div>
            <div class="wu-sum-lbl">Taille totale</div>
          </div>
          <div class="wu-sum-card" :class="wuData.wu_enabled ? 'sum-green' : 'sum-red'">
            <div class="wu-sum-val" style="font-size:13px">{{ wuData.wu_enabled ? '● Actif' : '○ Inactif' }}</div>
            <div class="wu-sum-lbl">Windows Update</div>
          </div>
        </div>

        <div v-if="wuData.pending_count === 0 && wuData.updates.length === 0" class="wu-empty">
          <span style="color:#22c55e;font-size:20px">✓</span>
          Tous les pilotes sont à jour
        </div>

        <!-- Updates list -->
        <div v-else class="wu-list">
          <div v-for="u in wuData.updates" :key="u.update_id" class="wu-item">
            <div class="wu-item-info">
              <div class="wu-item-title">{{ u.title }}</div>
              <div class="wu-item-meta">
                <span v-if="u.driver_class" class="wu-tag">{{ u.driver_class }}</span>
                <span v-if="u.driver_manufacturer" class="wu-tag">{{ u.driver_manufacturer }}</span>
                <span v-if="u.driver_version" class="wu-tag mono">v{{ u.driver_version }}</span>
                <span class="wu-tag">{{ u.size_mb.toFixed(1) }} MB</span>
                <span v-if="u.is_mandatory" class="wu-tag wu-tag-red">Obligatoire</span>
              </div>
            </div>
            <button class="drv-btn drv-btn-sm" :disabled="installing" @click="installSingleWU(u.update_id)">
              <Download :size="12" /> Installer
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- ===== ONGLET 3 : MAJ via Packs INF ===== -->
    <div v-if="subTab === 'packs'">
      <div class="packs-section">
        <div class="packs-info">
          <HardDrive :size="16" style="color:var(--accent-primary)" />
          <span>Sélectionnez un dossier contenant des packs de pilotes (.inf) — compatible Snappy Driver</span>
          <button class="drv-btn drv-btn-primary" style="margin-left:auto" @click="launchSdi">
            <Zap :size="13" /> Snappy Driver Installer
          </button>
        </div>

        <!-- Step 1: scan hardware -->
        <div class="packs-step" :class="{ done: hwDevices.length > 0 }">
          <div class="packs-step-header">
            <span class="step-num">1</span> Scanner les périphériques matériels
            <span v-if="hwDevices.length > 0" class="step-done">✓ {{ hwDevices.length }} périphériques</span>
          </div>
          <button class="drv-btn drv-btn-primary" :disabled="hwLoading" @click="scanHW">
            <Cpu :size="14" /> Scanner le matériel
          </button>
          <div v-if="hwLoading" class="wu-loading"><div class="drv-spinner" /> Lecture PnP...</div>
        </div>

        <!-- Step 2: select folder -->
        <div class="packs-step" :class="{ done: packsFolder }">
          <div class="packs-step-header">
            <span class="step-num">2</span> Sélectionner le dossier de packs
            <span v-if="packsFolder" class="step-done">✓ {{ packsFolder }}</span>
          </div>
          <button class="drv-btn drv-btn-primary" @click="pickFolder">
            <FolderOpen :size="14" /> Choisir le dossier
          </button>
        </div>

        <!-- Step 3: analyze -->
        <div class="packs-step" :class="{ done: scanResult !== null }">
          <div class="packs-step-header">
            <span class="step-num">3</span> Analyser la compatibilité
          </div>
          <button class="drv-btn drv-btn-primary"
            :disabled="!hwDevices.length || !packsFolder || packLoading"
            @click="analyzeCompatibility">
            <Search :size="14" /> Analyser
          </button>
          <div v-if="packLoading" class="wu-loading"><div class="drv-spinner" /> Scan INF en cours (peut prendre 30s)...</div>
        </div>
      </div>

      <!-- Results -->
      <div v-if="scanResult">
        <div class="wu-summary">
          <div class="wu-sum-card sum-blue">
            <div class="wu-sum-val">{{ scanResult.total_inf_scanned }}</div>
            <div class="wu-sum-lbl">Fichiers INF scannés</div>
          </div>
          <div class="wu-sum-card" :class="scanResult.matches.length > 0 ? 'sum-orange' : 'sum-green'">
            <div class="wu-sum-val">{{ scanResult.matches.length }}</div>
            <div class="wu-sum-lbl">Compatibles trouvés</div>
          </div>
          <div class="wu-sum-card sum-blue">
            <div class="wu-sum-val">{{ scanResult.scan_duration_ms }}ms</div>
            <div class="wu-sum-lbl">Durée scan</div>
          </div>
        </div>

        <div v-if="scanResult.matches.length === 0" class="wu-empty">
          Aucun pilote compatible trouvé dans ce dossier.
        </div>

        <div v-else>
          <div style="display:flex;gap:8px;margin-bottom:10px">
            <button class="drv-btn drv-btn-orange" :disabled="installing" @click="installAll">
              <Download :size="14" /> Installer tout ({{ scanResult.matches.length }})
            </button>
          </div>
          <div class="wu-list">
            <div v-for="m in scanResult.matches" :key="m.inf_path" class="wu-item">
              <div class="wu-item-info">
                <div class="wu-item-title">{{ m.device_name }}</div>
                <div class="wu-item-meta">
                  <span class="wu-tag">{{ m.inf_file_name }}</span>
                  <span v-if="m.provider" class="wu-tag">{{ m.provider }}</span>
                  <span v-if="m.version" class="wu-tag mono">v{{ m.version }}</span>
                  <span class="wu-tag" :class="m.match_type === 'exact' ? 'wu-tag-green' : 'wu-tag-orange'">
                    {{ m.match_type === 'exact' ? '✓ Exact' : '~ Compatible' }}
                  </span>
                </div>
              </div>
              <button class="drv-btn drv-btn-sm" :disabled="installing" @click="installSinglePack(m.inf_path)">
                <Download :size="12" /> Installer
              </button>
            </div>
          </div>
        </div>
      </div>

      <div v-if="installMsg" class="wu-msg" :class="installErr ? 'wu-err' : 'wu-ok'" style="margin-top:10px">{{ installMsg }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, shallowRef, defineAsyncComponent } from "vue";
import { invoke, invokeRaw, isTauriContext } from "@/utils/invoke";
import { sdiAcquire, sdiRelease } from "@/utils/sdiGuard";
import type { DriverInstallResult } from "@/types/diagnostic";
import { open } from "@tauri-apps/plugin-dialog";
import { useNotificationStore } from "@/stores/notifications";
import { Search, AlertTriangle, Shield, RefreshCw, Download, HardDrive, Cpu, FolderOpen, CheckCircle, Zap } from "lucide-vue-next";
const DiagTabDriverUpdater = defineAsyncComponent(() => import("./DiagTabDriverUpdater.vue"));
const notify = useNotificationStore();

// ---- Types ----
interface PnpDriver { name: string; provider: string; version: string; date: string; class: string; inf: string; signed: boolean; status: string; config_error: number }
interface SysDriversData { drivers: PnpDriver[]; total: number; unsigned_count: number; error_count: number }
interface WuDriverUpdate { title: string; driver_class: string; driver_manufacturer: string; driver_model: string; driver_version: string; size_mb: number; update_id: string; is_downloaded: boolean; is_mandatory: boolean }
interface WuDriverSummary { pending_count: number; total_size_mb: number; updates: WuDriverUpdate[]; wu_enabled: boolean; last_check: string }
interface DriverMatch { device_name: string; hardware_id: string; inf_path: string; inf_file_name: string; provider: string; version: string; date: string; match_type: string }
interface DriverScanResult { matches: DriverMatch[]; total_inf_scanned: number; total_devices_checked: number; scan_duration_ms: number }
interface HardwareDevice { name: string; hardware_ids: string[]; compatible_ids: string[]; class: string; status: string; pnp_device_id: string }

type SubTab = 'list' | 'problems' | 'wu' | 'packs' | 'updater'

const SUBTABS = shallowRef([
  { id: 'list',     label: 'Pilotes installés',           icon: Shield,         badge: null as number | null },
  { id: 'problems', label: 'Périphériques avec problèmes', icon: AlertTriangle,  badge: null as number | null },
  { id: 'wu',       label: 'MAJ Windows Update',          icon: RefreshCw,      badge: null as number | null },
  { id: 'packs',    label: 'MAJ via Packs INF',           icon: HardDrive,      badge: null as number | null },
  { id: 'updater',  label: 'Mise à jour auto (SDI)',      icon: Zap,            badge: null as number | null },
])

const LIST_FILTERS = [{ k: 'all' as const, l: 'Tous' }, { k: 'errors' as const, l: 'Erreurs' }, { k: 'unsigned' as const, l: 'Non signés' }]

const subTab = ref<SubTab>('list')

// --- Problems tab ---
interface ProblemDevice { name: string; device_id: string; error_code: number; error_description: string; class: string; status: string; }
const problemDevices = ref<ProblemDevice[]>([])
const problemsLoading = ref(false)
const problemsLoaded = ref(false)

async function loadProblems() {
  problemsLoading.value = true
  try { problemDevices.value = await invoke<ProblemDevice[]>("get_problem_devices") }
  catch (e) { showMsg(String(e), true) }
  finally { problemsLoading.value = false; problemsLoaded.value = true }
}

// --- List tab ---
const loadingList = ref(true)
const listData = ref<SysDriversData | null>(null)
const listSearch = ref("")
const listFilter = ref<"all" | "errors" | "unsigned">("all")

const filteredList = computed(() => {
  if (!listData.value) return []
  let list = listData.value.drivers
  if (listFilter.value === 'errors') list = list.filter(d => d.config_error !== 0)
  if (listFilter.value === 'unsigned') list = list.filter(d => !d.signed)
  const q = listSearch.value.toLowerCase()
  if (q) list = list.filter(d => d.name.toLowerCase().includes(q) || d.provider.toLowerCase().includes(q) || d.class.toLowerCase().includes(q))
  return list
})

onMounted(async () => {
  try { listData.value = await invoke<SysDriversData>("get_sys_drivers_list") }
  catch (e: unknown) { if (isTauriContext()) notify.error("Drivers système", (e instanceof Error ? e.message : String(e)).slice(0, 120)); }
  finally { loadingList.value = false }
})

// --- WU tab ---
const wuLoading = ref(false)
const wuData = ref<WuDriverSummary | null>(null)
const installing = ref(false)
const installMsg = ref("")
const installErr = ref(false)

function showMsg(msg: string, err = false) {
  installMsg.value = msg; installErr.value = err
  setTimeout(() => { installMsg.value = "" }, 4000)
}

async function launchSdi() {
  try {
    sdiAcquire();
    await invoke("launch_sdi");
  } catch (e) {
    sdiRelease();
    showMsg("SDI introuvable : " + String(e), true);
  }
}

async function checkWU() {
  wuLoading.value = true; wuData.value = null
  try { wuData.value = await invoke<WuDriverSummary>("check_driver_updates_winupdate") }
  catch(e) { showMsg(String(e), true) }
  finally { wuLoading.value = false }
}

async function installAllWU() {
  installing.value = true
  try {
    const r = await invoke<DriverInstallResult>("install_all_driver_updates")
    showMsg(r.output.slice(0, 120), !r.success)
    await checkWU()
  } catch(e) { showMsg(String(e), true) }
  finally { installing.value = false }
}

async function installSingleWU(updateId: string) {
  installing.value = true
  try {
    const r = await invoke<DriverInstallResult>("install_driver_winupdate", { updateId })
    showMsg(r.output.slice(0, 120), !r.success)
  } catch(e) { showMsg(String(e), true) }
  finally { installing.value = false }
}

// --- Packs tab ---
const hwLoading = ref(false)
const hwDevices = ref<HardwareDevice[]>([])
const packsFolder = ref("")
const packLoading = ref(false)
const scanResult = ref<DriverScanResult | null>(null)

async function scanHW() {
  hwLoading.value = true
  try { hwDevices.value = await invoke<HardwareDevice[]>("get_hardware_devices") }
  catch (e: unknown) { if (isTauriContext()) notify.warning("Périphériques", (e instanceof Error ? e.message : String(e)).slice(0, 120)); }
  finally { hwLoading.value = false }
}

async function pickFolder() {
  const selected = await open({ directory: true, multiple: false, title: "Sélectionner le dossier de packs de pilotes" })
  if (selected && typeof selected === 'string') packsFolder.value = selected
}

async function analyzeCompatibility() {
  if (!hwDevices.value.length || !packsFolder.value) return
  packLoading.value = true; scanResult.value = null
  try {
    const deviceIds = hwDevices.value.flatMap(d => [...d.hardware_ids, ...d.compatible_ids])
    scanResult.value = await invoke<DriverScanResult>("scan_driver_folder", { folderPath: packsFolder.value, deviceIds })
  } catch(e) { showMsg(String(e), true) }
  finally { packLoading.value = false }
}

async function installSinglePack(infPath: string) {
  installing.value = true
  try {
    const r = await invoke<DriverInstallResult>("install_driver", { infPath })
    showMsg(r.output.slice(0, 120), !r.success)
  } catch(e) { showMsg(String(e), true) }
  finally { installing.value = false }
}

async function installAll() {
  if (!scanResult.value) return
  installing.value = true
  let ok = 0, fail = 0
  for (const m of scanResult.value.matches) {
    try {
      const r = await invokeRaw<DriverInstallResult>("install_driver", { infPath: m.inf_path })
      if (r.success) ok++; else fail++;
    } catch { fail++ }
  }
  showMsg(
    `${ok} pilote(s) installé(s)${fail > 0 ? `, ${fail} échec(s)` : ''}`,
    fail > 0 && ok === 0
  )
  installing.value = false
}
</script>

<style scoped>
.drivers-root { display: flex; flex-direction: column; gap: 14px; }

/* Sub-tabs */
.drivers-subtabs { display: flex; gap: 4px; border-bottom: 1px solid var(--border); padding-bottom: 0; }
.drivers-subtab {
  display: flex; align-items: center; gap: 6px;
  padding: 8px 16px; font-size: 12px; font-weight: 500;
  border: none; border-bottom: 2px solid transparent;
  background: none; cursor: pointer; color: var(--text-muted);
  transition: all 150ms ease; white-space: nowrap; margin-bottom: -1px;
}
.drivers-subtab:hover { color: var(--text-primary); }
.drivers-subtab.active { color: var(--accent-primary); border-bottom-color: var(--accent-primary); }
.subtab-badge { background: var(--accent-primary); color: #fff; font-size: 10px; padding: 1px 5px; border-radius: 8px; }

/* Stats */
.drv-stats { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; }
.drv-stat { border-radius: 10px; padding: 14px; text-align: center; border: 1px solid transparent; }
.drv-stat-blue  { background: rgba(59,130,246,.1); border-color: rgba(59,130,246,.25); }
.drv-stat-green { background: rgba(34,197,94,.1);  border-color: rgba(34,197,94,.25); }
.drv-stat-orange{ background: rgba(245,158,11,.1); border-color: rgba(245,158,11,.25); }
.drv-stat-red   { background: rgba(239,68,68,.1);  border-color: rgba(239,68,68,.25); }
.drv-stat-val { font-size: 26px; font-weight: 700; }
.drv-stat-lbl { font-size: 10px; opacity: .6; text-transform: uppercase; margin-top: 2px; }

/* Filters */
.drv-filters { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.drv-search { display: flex; align-items: center; gap: 6px; flex: 1; min-width: 200px;
  background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 8px; padding: 6px 10px; color: var(--text-muted); }
.drv-search input { flex: 1; background: none; border: none; outline: none; font-size: 12px; color: var(--text-primary); }
.drv-search input::placeholder { color: var(--text-muted); }
.drv-filter-btns { display: flex; gap: 4px; }
.drv-filter-btn { display: flex; align-items: center; gap: 4px; padding: 5px 12px; border-radius: 6px;
  border: 1px solid var(--border); background: var(--bg-secondary); cursor: pointer;
  font-size: 11px; color: var(--text-secondary); transition: all 150ms; }
.drv-filter-btn:hover { color: var(--text-primary); }
.drv-filter-btn.active { background: var(--accent-primary); color: #fff; border-color: var(--accent-primary); }
.filter-count { border-radius: 8px; font-size: 10px; padding: 0 5px; font-weight: 700; }
.filter-count.red { background: rgba(239,68,68,.2); color: #ef4444; }
.filter-count.orange { background: rgba(245,158,11,.2); color: #f59e0b; }

/* Table */
.drv-table-wrap { overflow-x: auto; border: 1px solid var(--border); border-radius: 10px; }
.drv-table { width: 100%; border-collapse: collapse; font-size: 11px; }
.drv-table thead tr { background: var(--bg-tertiary); }
.drv-table th { padding: 8px 10px; text-align: left; color: var(--text-muted); font-size: 10px; text-transform: uppercase; letter-spacing: .04em; border-bottom: 1px solid var(--border); }
.drv-table td { padding: 6px 10px; border-bottom: 1px solid var(--border); }
.drv-table tbody tr:hover td { background: var(--bg-tertiary); }
.drv-table tbody tr:last-child td { border-bottom: none; }
.row-error td { background: rgba(239,68,68,.04) !important; }
.row-unsigned td { background: rgba(245,158,11,.03) !important; }
.td-name { max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; display: flex; align-items: center; gap: 4px; }
.td-muted { color: var(--text-muted); max-width: 160px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.td-class { color: var(--text-muted); }
.drv-code { font-family: 'JetBrains Mono', monospace; font-size: 10px; background: var(--bg-tertiary); padding: 1px 5px; border-radius: 3px; }
.drv-badge { font-size: 10px; padding: 2px 7px; border-radius: 5px; font-weight: 500; }
.badge-ok   { background: rgba(34,197,94,.15);  color: #22c55e; }
.badge-warn { background: rgba(245,158,11,.15); color: #f59e0b; }
.badge-err  { background: rgba(239,68,68,.15);  color: #ef4444; }
.icon-err { color: #ef4444; flex-shrink: 0; }
.icon-warn { color: #f59e0b; flex-shrink: 0; }
.drv-truncated { font-size: 11px; color: var(--text-muted); padding: 8px 12px; text-align: center; }

/* WU */
.wu-section { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-bottom: 12px; }
.wu-summary { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; margin-bottom: 14px; }
.wu-sum-card { border-radius: 10px; padding: 14px; text-align: center; border: 1px solid transparent; }
.sum-blue   { background: rgba(59,130,246,.1);  border-color: rgba(59,130,246,.25); }
.sum-green  { background: rgba(34,197,94,.1);   border-color: rgba(34,197,94,.25); }
.sum-orange { background: rgba(245,158,11,.1);  border-color: rgba(245,158,11,.25); }
.sum-red    { background: rgba(239,68,68,.1);   border-color: rgba(239,68,68,.25); }
.wu-sum-val { font-size: 22px; font-weight: 700; }
.wu-sum-lbl { font-size: 10px; opacity: .6; text-transform: uppercase; margin-top: 2px; }
.wu-empty { display: flex; align-items: center; gap: 10px; padding: 20px; background: var(--bg-secondary); border-radius: 10px; font-size: 13px; color: var(--text-muted); }
.wu-list { display: flex; flex-direction: column; gap: 6px; }
.wu-item { display: flex; align-items: center; gap: 12px; padding: 12px 14px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 10px; transition: border-color 150ms; }
.wu-item:hover { border-color: var(--accent-primary); }
.wu-item-info { flex: 1; min-width: 0; }
.wu-item-title { font-size: 13px; font-weight: 500; margin-bottom: 5px; }
.wu-item-meta { display: flex; gap: 5px; flex-wrap: wrap; }
.wu-tag { font-size: 10px; padding: 2px 7px; border-radius: 5px; background: var(--bg-tertiary); color: var(--text-muted); }
.wu-tag-green { background: rgba(34,197,94,.15); color: #22c55e; }
.wu-tag-orange{ background: rgba(245,158,11,.15);color: #f59e0b; }
.wu-tag-red   { background: rgba(239,68,68,.15); color: #ef4444; }
.mono { font-family: 'JetBrains Mono', monospace; }
.wu-loading { display: flex; align-items: center; gap: 8px; font-size: 12px; opacity: .7; }
.wu-msg { font-size: 12px; padding: 6px 12px; border-radius: 6px; }
.wu-ok  { color: #22c55e; background: rgba(34,197,94,.1); }
.wu-err { color: #ef4444; background: rgba(239,68,68,.1); }

/* Packs */
.packs-section { display: flex; flex-direction: column; gap: 12px; margin-bottom: 16px; }
.packs-info { display: flex; align-items: center; gap: 8px; padding: 10px 14px; background: rgba(124,58,237,.08); border: 1px solid rgba(124,58,237,.2); border-radius: 10px; font-size: 12px; color: var(--text-secondary); }
.packs-step { padding: 14px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 10px; display: flex; flex-direction: column; gap: 10px; }
.packs-step.done { border-color: rgba(34,197,94,.3); background: rgba(34,197,94,.04); }
.packs-step-header { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 600; }
.step-num { width: 22px; height: 22px; border-radius: 50%; background: var(--accent-primary); color: #fff; font-size: 11px; font-weight: 700; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.step-done { font-size: 11px; color: #22c55e; background: rgba(34,197,94,.1); border-radius: 5px; padding: 2px 8px; margin-left: auto; }

/* Buttons */
.drv-btn { display: inline-flex; align-items: center; gap: 6px; padding: 7px 14px; border-radius: 8px; border: 1px solid var(--border); background: var(--bg-secondary); color: var(--text-secondary); font-size: 12px; cursor: pointer; transition: all 150ms; font-family: inherit; }
.drv-btn:hover { color: var(--text-primary); border-color: var(--text-muted); }
.drv-btn:disabled { opacity: .4; cursor: not-allowed; }
.drv-btn-primary { background: var(--accent-primary); color: #fff; border-color: var(--accent-primary); }
.drv-btn-primary:hover { opacity: .85; }
.drv-btn-orange { background: #f59e0b; color: #fff; border-color: #f59e0b; }
.drv-btn-sm { padding: 5px 10px; font-size: 11px; }

/* Spinner */
.drv-spinner { width: 14px; height: 14px; border: 2px solid rgba(255,255,255,.2); border-top-color: var(--accent-primary); border-radius: 50%; animation: spin .8s linear infinite; flex-shrink: 0; }
@keyframes spin { to { transform: rotate(360deg); } }

.drivers-loading { display: flex; align-items: center; gap: 10px; padding: 20px; font-size: 13px; color: var(--text-muted); }
</style>
