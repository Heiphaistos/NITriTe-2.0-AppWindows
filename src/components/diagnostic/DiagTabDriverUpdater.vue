<script setup lang="ts">
import { ref, computed } from "vue";
import { invokeRaw as invoke } from "@/utils/invoke";
import { useNotificationStore } from "@/stores/notifications";
const notify = useNotificationStore();
import NBadge from "@/components/ui/NBadge.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import { HardDrive, Search, Download, AlertTriangle, CheckCircle, FolderOpen, Zap } from "lucide-vue-next";

interface HardwareDevice {
  name: string; device_id: string; hardware_ids: string[]; compatible_ids: string[];
  manufacturer: string; class: string; driver_version: string; driver_date: string;
  status: string; config_error: number; has_driver_problem: boolean;
}
interface DriverMatch {
  device_name: string; device_id: string; matched_hw_id: string;
  inf_path: string; inf_name: string; driver_provider: string;
  driver_version: string; driver_date: string; match_type: string; score: number;
}
interface DriverScanResult {
  devices: HardwareDevice[]; matches: DriverMatch[]; inf_count: number;
  scan_time_ms: number; pack_folder: string; devices_with_match: number; devices_with_problem: number;
}
interface InstallResult { inf_path: string; success: boolean; output: string; duration_secs: number; }

// ─── State ─────────────────────────────────────────────────────────────────────
const devices = ref<HardwareDevice[]>([]);
const devicesLoading = ref(false);
const packFolder = ref("");
const scanResult = ref<DriverScanResult | null>(null);
const scanLoading = ref(false);
const installResults = ref<Record<string, InstallResult>>({});
const installing = ref<string | null>(null);
const selectedMatches = ref<Set<string>>(new Set());
const searchQuery = ref("");
const filterMode = ref<"all"|"problems"|"matched">("all");
const problemDevices = ref<string[]>([]);
const problemsLoading = ref(false);

// ─── Computed ──────────────────────────────────────────────────────────────────
const filteredDevices = computed(() => {
  let list = devices.value;
  if (filterMode.value === "problems") list = list.filter(d => d.has_driver_problem);
  if (filterMode.value === "matched") {
    const matchedIds = new Set(scanResult.value?.matches.map(m => m.device_id) ?? []);
    list = list.filter(d => matchedIds.has(d.device_id));
  }
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase();
    list = list.filter(d => d.name.toLowerCase().includes(q) || d.class.toLowerCase().includes(q) || d.manufacturer.toLowerCase().includes(q));
  }
  return list;
});

const matchedByDevice = computed(() => {
  const map: Record<string, DriverMatch[]> = {};
  for (const m of (scanResult.value?.matches ?? [])) {
    if (!map[m.device_id]) map[m.device_id] = [];
    map[m.device_id].push(m);
  }
  return map;
});

// ─── Actions ───────────────────────────────────────────────────────────────────
async function launchSdi() {
  try {
    (window as any).__nitrite_sdi_active = true;
    setTimeout(() => { (window as any).__nitrite_sdi_active = false; }, 60000);
    await invoke("launch_sdi");
  } catch (e) {
    (window as any).__nitrite_sdi_active = false;
    notify.warning("SDI non trouvé sur ce PC", "Redirection vers le site officiel...");
    await invoke("open_url", { url: "https://www.snappy-driver-installer.org/download/" }).catch(() => {});
  }
}

async function chooseSdiLocation() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const result = await open({ title: "Sélectionner SDI.exe", filters: [{ name: "Exécutable", extensions: ["exe"] }] });
    if (result && typeof result === "string") {
      await invoke("run_system_command", { cmd: result, args: [] });
      notify.success("SDI lancé !", result);
    }
  } catch (e: any) {
    notify.error("Erreur lancement SDI", String(e));
  }
}

async function loadDevices() {
  devicesLoading.value = true;
  devices.value = [];
  try {
    devices.value = await invoke<HardwareDevice[]>("get_hardware_devices");
  } catch {}
  finally { devicesLoading.value = false; }
}

async function selectFolder() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const result = await open({ directory: true, title: "Sélectionner le dossier de drivers" });
    if (result && typeof result === "string") packFolder.value = result;
  } catch {
    // Fallback: let user type the path
  }
}

async function runScan() {
  if (!packFolder.value) return;
  scanLoading.value = true;
  scanResult.value = null;
  selectedMatches.value = new Set();
  try {
    // Collect all hardware IDs from loaded devices
    const allIds: string[] = [];
    for (const d of devices.value) {
      allIds.push(...d.hardware_ids, ...d.compatible_ids);
    }
    const uniqueIds = [...new Set(allIds)];
    scanResult.value = await invoke<DriverScanResult>("scan_driver_folder", {
      folderPath: packFolder.value,
      deviceIds: uniqueIds,
    });
    // Auto-select all matches
    for (const m of scanResult.value?.matches ?? []) {
      selectedMatches.value.add(m.device_id);
    }
  } catch (e) { notify.error("Scan drivers échoué", String(e)); }
  finally { scanLoading.value = false; }
}

async function installDriver(match: DriverMatch) {
  installing.value = match.device_id;
  try {
    const result = await invoke<InstallResult>("install_driver", { infPath: match.inf_path });
    installResults.value[match.device_id] = result;
  } catch {}
  finally { installing.value = null; }
}

async function installSelected() {
  if (!scanResult.value) return;
  for (const m of scanResult.value.matches) {
    if (selectedMatches.value.has(m.device_id)) {
      await installDriver(m);
    }
  }
}

async function checkProblems() {
  problemsLoading.value = true;
  try {
    problemDevices.value = await invoke<string[]>("check_driver_updates_winupdate");
  } catch {}
  finally { problemsLoading.value = false; }
}

function toggleSelect(id: string) {
  if (selectedMatches.value.has(id)) selectedMatches.value.delete(id);
  else selectedMatches.value.add(id);
}

function matchTypeColor(t: string) {
  if (t === "exact") return "success";
  if (t === "compatible") return "warning";
  return "neutral";
}

function errLabel(code: number) {
  if (code === 0) return "";
  if (code === 1) return "Code 1 — Config error";
  if (code === 10) return "Code 10 — Ne démarre pas";
  if (code === 28) return "Code 28 — Pilote absent";
  if (code === 43) return "Code 43 — Erreur signalée";
  if (code === 45) return "Code 45 — Non connecté";
  return `Code ${code}`;
}
</script>

<template>
  <div style="display:flex;flex-direction:column;gap:14px">

    <!-- En-tête -->
    <div class="diag-section" style="background:linear-gradient(135deg,rgba(124,154,245,0.08),rgba(167,139,250,0.08));border-color:rgba(124,154,245,0.2)">
      <p class="diag-section-label" style="margin:0 0 6px 0;font-size:13px">
        <HardDrive :size="13" style="display:inline;margin-right:4px" />Mise à jour des pilotes — Mode Offline
      </p>
      <p style="font-size:12px;color:var(--text-secondary);margin:0">
        Compatible avec les packs de drivers extraits (SDIO / DriverPack Solution / manuel).
        Scanne les fichiers <code>.inf</code> pour trouver les pilotes correspondants à votre matériel.
      </p>
      <div style="display:flex;gap:8px;flex-wrap:wrap;margin-top:10px">
        <button @click="launchSdi" style="padding:6px 14px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:12px;cursor:pointer;display:inline-flex;align-items:center;gap:6px">
          <Zap :size="13" /> Snappy Driver Installer
        </button>
        <button @click="chooseSdiLocation" style="padding:6px 14px;background:var(--bg-secondary);color:var(--text-secondary);border:1px solid var(--border);border-radius:6px;font-size:12px;cursor:pointer;display:inline-flex;align-items:center;gap:6px">
          <FolderOpen :size="13" /> Choisir emplacement SDI...
        </button>
      </div>
    </div>

    <!-- Étape 1 : Scanner le matériel -->
    <div class="diag-section">
      <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px">
        <p class="diag-section-label" style="margin:0">
          <span style="background:var(--accent);color:white;border-radius:50%;width:18px;height:18px;display:inline-flex;align-items:center;justify-content:center;font-size:10px;margin-right:6px;font-weight:700">1</span>
          Scanner le matériel système
        </p>
        <button @click="loadDevices" :disabled="devicesLoading"
          style="padding:5px 14px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:12px;cursor:pointer;display:flex;align-items:center;gap:6px">
          <NSpinner v-if="devicesLoading" :size="12" />
          <Search v-else :size="12" />
          {{ devicesLoading ? 'Analyse...' : 'Scanner les périphériques' }}
        </button>
      </div>

      <div v-if="devicesLoading" style="display:flex;align-items:center;gap:8px;color:var(--text-secondary);font-size:12px">
        <NSpinner :size="14" />Enumération des périphériques PnP (peut prendre 10-20s)...
      </div>

      <div v-else-if="devices.length">
        <!-- Stats -->
        <div style="display:grid;grid-template-columns:repeat(4,1fr);gap:8px;margin-bottom:10px">
          <div class="diag-section" style="text-align:center;padding:8px">
            <div style="font-size:20px;font-weight:700;color:var(--accent)">{{ devices.length }}</div>
            <div style="font-size:10px;color:var(--text-secondary)">Périphériques</div>
          </div>
          <div class="diag-section" style="text-align:center;padding:8px">
            <div style="font-size:20px;font-weight:700" :style="{color:devices.filter(d=>d.has_driver_problem).length>0?'var(--error)':'var(--success)'}">{{ devices.filter(d=>d.has_driver_problem).length }}</div>
            <div style="font-size:10px;color:var(--text-secondary)">Problèmes</div>
          </div>
          <div class="diag-section" style="text-align:center;padding:8px">
            <div style="font-size:20px;font-weight:700;color:var(--success)">{{ scanResult?.devices_with_match ?? 0 }}</div>
            <div style="font-size:10px;color:var(--text-secondary)">Correspondances</div>
          </div>
          <div class="diag-section" style="text-align:center;padding:8px">
            <div style="font-size:20px;font-weight:700;color:var(--text-secondary)">{{ scanResult?.inf_count ?? 0 }}</div>
            <div style="font-size:10px;color:var(--text-secondary)">INF scannés</div>
          </div>
        </div>

        <!-- Filtres + Recherche -->
        <div style="display:flex;gap:6px;align-items:center;flex-wrap:wrap;margin-bottom:8px">
          <div style="position:relative;flex:1;min-width:150px">
            <Search :size="11" style="position:absolute;left:8px;top:50%;transform:translateY(-50%);color:var(--text-secondary)" />
            <input v-model="searchQuery" placeholder="Filtrer..." style="width:100%;padding:4px 8px 4px 24px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-size:11px" />
          </div>
          <button v-for="f in [{k:'all',l:'Tous'},{k:'problems',l:'⚠ Problèmes'},{k:'matched',l:'✓ Trouvés'}]" :key="f.k"
            @click="filterMode=(f.k as any)"
            :style="{padding:'3px 10px',borderRadius:'6px',border:'1px solid var(--border)',fontSize:'10px',cursor:'pointer',background:filterMode===f.k?'var(--accent)':'var(--bg-secondary)',color:filterMode===f.k?'white':'var(--text-secondary)'}">
            {{ f.l }}
          </button>
        </div>

        <!-- Liste des périphériques -->
        <div style="max-height:260px;overflow-y:auto">
          <div v-for="d in filteredDevices.slice(0,200)" :key="d.device_id"
            style="display:flex;align-items:center;gap:8px;padding:5px 6px;border-bottom:1px solid var(--border)"
            :style="{background:d.has_driver_problem?'rgba(239,68,68,0.04)':''}">
            <AlertTriangle v-if="d.has_driver_problem" :size="11" style="color:var(--error);flex-shrink:0" />
            <CheckCircle v-else-if="matchedByDevice[d.device_id]" :size="11" style="color:var(--success);flex-shrink:0" />
            <div v-else style="width:11px;flex-shrink:0" />
            <div style="flex:1;min-width:0">
              <div style="font-size:11px;font-weight:500;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">{{ d.name||d.device_id }}</div>
              <div style="font-size:9px;color:var(--text-secondary)">{{ d.class||'—' }} • {{ d.manufacturer||'—' }}</div>
            </div>
            <span v-if="d.has_driver_problem" style="font-size:9px;color:var(--error)">{{ errLabel(d.config_error) }}</span>
            <NBadge v-if="matchedByDevice[d.device_id]" variant="success" style="font-size:8px;flex-shrink:0">Driver trouvé</NBadge>
          </div>
        </div>
        <p v-if="filteredDevices.length > 200" style="font-size:10px;color:var(--text-secondary);margin-top:4px">{{ filteredDevices.length - 200 }} de plus — affinez la recherche.</p>
      </div>

      <div v-else style="font-size:12px;color:var(--text-secondary)">Cliquez "Scanner les périphériques" pour commencer.</div>
    </div>

    <!-- Étape 2 : Sélectionner dossier drivers -->
    <div class="diag-section">
      <p class="diag-section-label" style="margin:0 0 8px 0">
        <span style="background:var(--accent);color:white;border-radius:50%;width:18px;height:18px;display:inline-flex;align-items:center;justify-content:center;font-size:10px;margin-right:6px;font-weight:700">2</span>
        Sélectionner le dossier de packs de drivers
      </p>
      <p style="font-size:10px;color:var(--text-secondary);margin:0 0 8px 0">
        Extrayez les packs SDIO / DriverPack Solution et pointez vers le dossier racine contenant les fichiers .inf
      </p>
      <div style="display:flex;gap:8px;align-items:center">
        <input v-model="packFolder" placeholder="C:\DriverPacks ou clé USB (ex: E:\Drivers)" style="flex:1;padding:5px 10px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-size:12px;font-family:monospace" />
        <button @click="selectFolder" style="padding:5px 12px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;font-size:12px;cursor:pointer;color:var(--text-secondary);display:flex;align-items:center;gap:4px">
          <FolderOpen :size="12" />Parcourir
        </button>
      </div>
    </div>

    <!-- Étape 3 : Analyser -->
    <div class="diag-section">
      <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px">
        <p class="diag-section-label" style="margin:0">
          <span style="background:var(--accent);color:white;border-radius:50%;width:18px;height:18px;display:inline-flex;align-items:center;justify-content:center;font-size:10px;margin-right:6px;font-weight:700">3</span>
          Analyser la compatibilité
        </p>
        <button @click="runScan" :disabled="scanLoading||!packFolder||!devices.length"
          style="padding:5px 16px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:12px;cursor:pointer;display:flex;align-items:center;gap:6px;font-weight:600">
          <NSpinner v-if="scanLoading" :size="12" />
          <Search v-else :size="12" />
          {{ scanLoading ? 'Analyse des INF...' : 'Analyser les drivers' }}
        </button>
      </div>
      <div v-if="scanLoading" style="font-size:12px;color:var(--text-secondary);display:flex;align-items:center;gap:8px">
        <NSpinner :size="14" />Scan récursif des fichiers .inf (peut prendre 1-5 min selon la taille des packs)...
      </div>
      <div v-else-if="scanResult">
        <div style="font-size:12px;color:var(--text-secondary);margin-bottom:8px">
          {{ scanResult.inf_count }} fichiers INF analysés en {{ (scanResult.scan_time_ms/1000).toFixed(1) }}s —
          <strong style="color:var(--success)">{{ scanResult.matches.length }} correspondance(s) trouvée(s)</strong>
        </div>

        <!-- Résultats de matching -->
        <div v-if="scanResult.matches.length">
          <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:6px">
            <span style="font-size:11px;font-weight:600">Pilotes disponibles :</span>
            <label style="display:flex;align-items:center;gap:4px;font-size:11px;cursor:pointer">
              <input type="checkbox" :checked="selectedMatches.size===scanResult.matches.length" @change="e=>{if((e.target as HTMLInputElement).checked){scanResult!.matches.forEach(m=>selectedMatches.add(m.device_id))}else{selectedMatches.clear()}}" />
              Tout sélectionner
            </label>
          </div>
          <div v-for="m in scanResult.matches" :key="m.device_id"
            style="display:flex;align-items:center;gap:8px;padding:6px;border:1px solid var(--border);border-radius:6px;margin-bottom:4px"
            :style="{background:selectedMatches.has(m.device_id)?'rgba(124,154,245,0.06)':''}">
            <input type="checkbox" :checked="selectedMatches.has(m.device_id)" @change="toggleSelect(m.device_id)" style="cursor:pointer" />
            <div style="flex:1;min-width:0">
              <div style="font-size:11px;font-weight:600">{{ m.device_id.split('\\').slice(0,2).join('\\') }}</div>
              <div style="font-size:10px;color:var(--text-secondary)">{{ m.inf_name }} — {{ m.driver_provider||'?' }} {{ m.driver_version }}</div>
              <div style="font-size:9px;color:var(--text-secondary);font-family:monospace;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">{{ m.inf_path }}</div>
            </div>
            <NBadge :variant="matchTypeColor(m.match_type)" style="font-size:8px;flex-shrink:0">{{ m.match_type }}</NBadge>
            <div style="flex-shrink:0">
              <div v-if="installResults[m.device_id]">
                <NBadge :variant="installResults[m.device_id].success?'success':'danger'" style="font-size:9px">
                  {{ installResults[m.device_id].success?'Installé ✓':'Échec ✗' }}
                </NBadge>
              </div>
              <button v-else @click="installDriver(m)" :disabled="installing!==null"
                style="padding:3px 10px;background:var(--success);color:white;border:none;border-radius:5px;font-size:10px;cursor:pointer;display:flex;align-items:center;gap:3px">
                <NSpinner v-if="installing===m.device_id" :size="9" />
                <Download v-else :size="9" />
                Installer
              </button>
            </div>
          </div>

          <!-- Installer sélection -->
          <div style="margin-top:10px;display:flex;justify-content:flex-end">
            <button @click="installSelected" :disabled="installing!==null||selectedMatches.size===0"
              style="padding:7px 20px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:12px;cursor:pointer;display:flex;align-items:center;gap:6px;font-weight:600">
              <NSpinner v-if="installing" :size="13" />
              <Download v-else :size="13" />
              Installer les {{ selectedMatches.size }} sélectionnés
            </button>
          </div>
        </div>

        <div v-else style="font-size:12px;color:var(--text-secondary)">
          Aucune correspondance trouvée dans ce dossier. Vérifiez que les fichiers .inf sont bien présents.
        </div>
      </div>
      <div v-else style="font-size:12px;color:var(--text-secondary)">
        Renseignez le dossier et cliquez "Analyser" pour matcher votre matériel avec les drivers disponibles.
      </div>
    </div>

    <!-- Pilotes avec problèmes (WU check) -->
    <div class="diag-section">
      <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px">
        <p class="diag-section-label" style="margin:0">
          <AlertTriangle :size="12" style="display:inline;margin-right:4px;color:var(--warning)" />Périphériques avec problèmes de pilote
        </p>
        <button @click="checkProblems" :disabled="problemsLoading"
          style="padding:4px 12px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;font-size:11px;cursor:pointer;color:var(--text-secondary);display:flex;align-items:center;gap:4px">
          <NSpinner v-if="problemsLoading" :size="11" />Vérifier
        </button>
      </div>
      <div v-if="problemDevices.length">
        <div v-for="(n, i) in problemDevices" :key="i" style="padding:3px 0;border-bottom:1px solid var(--border);font-size:11px">
          <AlertTriangle :size="10" style="color:var(--warning);margin-right:4px" />{{ n }}
        </div>
      </div>
      <p v-else-if="!problemsLoading" style="font-size:12px;color:var(--text-secondary)">Cliquez Vérifier pour lister les pilotes en erreur.</p>
    </div>

    <!-- Résultats d'installation -->
    <div v-if="Object.keys(installResults).length" class="diag-section">
      <p class="diag-section-label" style="margin:0 0 8px 0">Résultats d'installation</p>
      <div v-for="(r, devId) in installResults" :key="devId" style="margin-bottom:8px">
        <div style="display:flex;align-items:center;gap:6px;margin-bottom:4px">
          <NBadge :variant="r.success?'success':'danger'" style="font-size:9px">{{ r.success?'Succès':'Échec' }}</NBadge>
          <code style="font-size:10px">{{ r.inf_path.split('\\').pop() }}</code>
          <span style="font-size:10px;color:var(--text-secondary)">{{ r.duration_secs }}s</span>
        </div>
        <pre style="font-size:9px;color:var(--text-secondary);background:var(--bg-secondary);padding:6px;border-radius:5px;overflow-x:auto;white-space:pre-wrap;max-height:100px;overflow-y:auto">{{ r.output||'(vide)' }}</pre>
      </div>
    </div>

  </div>
</template>
