<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@/utils/invoke";
import NBadge from "@/components/ui/NBadge.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NButton from "@/components/ui/NButton.vue";
import NModal from "@/components/ui/NModal.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import { Database, AlertTriangle, CheckCircle, Key, FolderOpen, ChevronRight, Edit2, Trash2, RefreshCw } from "lucide-vue-next";
import { useNotificationStore } from "@/stores/notifications";

interface RegEntry {
  hive: string; key: string; name: string; value: string; suspicious: boolean;
}
interface RegistryPersistence {
  run_hklm: RegEntry[]; run_hkcu: RegEntry[]; run_once: RegEntry[];
  appinit_dlls: string[]; ifeo_debuggers: RegEntry[]; winlogon: RegEntry[];
  lsa_packages: string[]; browser_hijack: RegEntry[]; total_suspicious: number;
}

const data = ref<RegistryPersistence | null>(null);
const loading = ref(true);
const error = ref("");
const notify = useNotificationStore();

onMounted(async () => {
  try {
    data.value = await invoke<RegistryPersistence>("get_registry_persistence");
  } catch (e: any) { error.value = e?.toString() ?? "Erreur"; }
  finally { loading.value = false; }
});

const allRunEntries = computed(() => {
  if (!data.value) return [];
  return [...(data.value.run_hklm || []), ...(data.value.run_hkcu || []), ...(data.value.run_once || [])];
});

// ── Navigateur de registre ────────────────────────────────────────────────────
const activePanel = ref<"security" | "browser">("security");

const ROOT_HIVES = ["HKEY_LOCAL_MACHINE", "HKEY_CURRENT_USER", "HKEY_CLASSES_ROOT", "HKEY_USERS", "HKEY_CURRENT_CONFIG"];

interface RegValue { name: string; kind: string; data: string; }
interface RegBrowseResult { path: string; subkeys: string[]; values: RegValue[]; error?: string; }

const browsePath  = ref("HKEY_LOCAL_MACHINE");
const browseInput = ref("HKEY_LOCAL_MACHINE");
const browsing    = ref(false);
const browseResult = ref<RegBrowseResult | null>(null);
const breadcrumb  = computed(() => {
  const parts = browsePath.value.replace(/\\/g, "/").split("/").filter(Boolean);
  const crumbs = [];
  let acc = "";
  for (const p of parts) {
    acc = acc ? `${acc}\\${p}` : p;
    crumbs.push({ label: p, path: acc });
  }
  return crumbs;
});

async function browseTo(path: string) {
  browsePath.value = path;
  browseInput.value = path;
  browsing.value = true;
  try {
    browseResult.value = await invoke<RegBrowseResult>("registry_browse", { path });
  } catch (e: any) {
    browseResult.value = { path, subkeys: [], values: [], error: String(e) };
  }
  browsing.value = false;
}

function openSubkey(name: string) {
  browseTo(`${browsePath.value}\\${name}`);
}

// Modal édition
const editModal  = ref(false);
const editName   = ref("");
const editData   = ref("");
const editSaving = ref(false);

function startEdit(v: RegValue) {
  editName.value = v.name;
  editData.value = v.data;
  editModal.value = true;
}

async function saveEdit() {
  editSaving.value = true;
  try {
    await invoke("registry_set_value", { path: browsePath.value, name: editName.value, data: editData.value });
    editModal.value = false;
    await browseTo(browsePath.value);
  } catch (e: any) { notify.error("Erreur registre", String(e)); }
  editSaving.value = false;
}

async function deleteValue(name: string) {
  if (!confirm(`Supprimer la valeur "${name}" ?`)) return;
  try {
    await invoke("registry_delete_value", { path: browsePath.value, name });
    await browseTo(browsePath.value);
  } catch (e: any) { notify.error("Erreur registre", String(e)); }
}

async function openInRegedit(regPath: string) {
  const ps = `$p="${regPath}".Replace("HKLM:","HKEY_LOCAL_MACHINE").Replace("HKCU:","HKEY_CURRENT_USER").Replace("HKLM\\\\","HKEY_LOCAL_MACHINE\\\\").Replace("HKCU\\\\","HKEY_CURRENT_USER\\\\"); try { Set-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Applets\\Regedit' -Name 'LastKey' -Value $p -ErrorAction Stop } catch {}; Start-Process regedit.exe`;
  await invoke("run_system_command", { cmd: "powershell", args: ["-WindowStyle", "Hidden", "-Command", ps] }).catch(() => {});
}

function kindVariant(kind: string): "info" | "warning" | "success" | "neutral" {
  if (kind === "String" || kind === "ExpandString") return "info";
  if (kind === "DWord" || kind === "QWord") return "success";
  if (kind === "Binary") return "warning";
  return "neutral";
}

// ── Recherche dans les valeurs ────────────────────────────────────────────────
const searchQuery = ref("");
const searchResults = ref<Array<{path: string; name: string; kind: string; data: string}>>([]);
const searching = ref(false);
const searchDone = ref(false);

async function searchRegistry() {
  if (!searchQuery.value.trim()) return;
  searching.value = true;
  searchDone.value = false;
  searchResults.value = [];
  try {
    const result = await invoke<{path: string; subkeys: string[]; values: Array<{name: string; kind: string; data: string}>}>(
      "registry_browse", { path: browsePath.value }
    );
    const q = searchQuery.value.toLowerCase();
    const matches = (result.values || [])
      .filter(v => v.name.toLowerCase().includes(q) || v.data.toLowerCase().includes(q))
      .map(v => ({ path: browsePath.value, name: v.name, kind: v.kind, data: v.data }));
    searchResults.value = matches;
  } catch (e: any) {
    searchResults.value = [];
  }
  searching.value = false;
  searchDone.value = true;
}

function exportRegKey() {
  if (!browseResult.value) return;
  const lines: string[] = [
    "Windows Registry Editor Version 5.00",
    "",
    `[${browsePath.value}]`,
  ];
  for (const v of browseResult.value.values) {
    const name = v.name === "(Default)" ? "@" : `"${v.name}"`;
    if (v.kind === "String" || v.kind === "ExpandString") {
      lines.push(`${name}="${v.data.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`);
    } else if (v.kind === "DWord") {
      const hex = parseInt(v.data || "0").toString(16).padStart(8, "0");
      lines.push(`${name}=dword:${hex}`);
    } else {
      lines.push(`; ${name}=${v.kind}:${v.data}`);
    }
  }
  const content = lines.join("\r\n");
  const blob = new Blob(["\ufeff" + content], { type: "application/octet-stream" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  const safeName = browsePath.value.replace(/[\\:*?"<>|]/g, "_").slice(0, 50);
  a.download = safeName + ".reg";
  a.click();
  URL.revokeObjectURL(url);
}
</script>

<template>
  <div class="diag-tab-content">
    <DiagBanner :icon="Database" title="Registre Windows" desc="Analyse de persistance et navigateur de registre" color="amber" />

    <!-- Sélecteur de panneau -->
    <div class="reg-panel-tabs">
      <button :class="['reg-tab', { active: activePanel === 'security' }]" @click="activePanel = 'security'">
        <AlertTriangle :size="14" /> Analyse sécurité
      </button>
      <button :class="['reg-tab', { active: activePanel === 'browser' }]" @click="activePanel = 'browser'; if (!browseResult) browseTo('HKEY_LOCAL_MACHINE')">
        <FolderOpen :size="14" /> Navigateur
      </button>
    </div>

    <!-- ── Navigateur ── -->
    <div v-if="activePanel === 'browser'" class="reg-browser">
      <!-- Barre d'adresse -->
      <div class="reg-addr-bar">
        <input v-model="browseInput" class="reg-addr-input" placeholder="HKEY_LOCAL_MACHINE\SOFTWARE\..." @keyup.enter="browseTo(browseInput)" />
        <NButton variant="primary" size="sm" :disabled="browsing" @click="browseTo(browseInput)">
          <RefreshCw v-if="browsing" :size="13" style="animation:spin .8s linear infinite" /><ChevronRight v-else :size="13" /> Aller
        </NButton>
        <NButton v-if="browseResult && !browseResult.error" variant="ghost" size="sm" @click="exportRegKey" title="Exporter en .reg">
          ↓ .reg
        </NButton>
      </div>

      <!-- Breadcrumb -->
      <div class="reg-breadcrumb">
        <button v-for="(crumb, i) in breadcrumb" :key="crumb.path" class="crumb-btn" @click="browseTo(crumb.path)">
          {{ crumb.label }}<span v-if="i < breadcrumb.length - 1" class="crumb-sep">\</span>
        </button>
      </div>

      <!-- Hives racine -->
      <div v-if="!browseResult && !browsing" class="reg-hives">
        <button v-for="h in ROOT_HIVES" :key="h" class="hive-btn" @click="browseTo(h)">
          <FolderOpen :size="14" style="color:var(--warning)" /> {{ h }}
        </button>
      </div>

      <div v-if="browsing" class="diag-loading"><div class="diag-spinner"></div> Lecture du registre...</div>

      <div v-else-if="browseResult">
        <!-- Erreur -->
        <div v-if="browseResult.error" style="color:var(--danger);font-size:13px;padding:8px">
          <AlertTriangle :size="13" style="display:inline;margin-right:5px" />{{ browseResult.error }}
        </div>
        <template v-else>
          <!-- Sous-clés -->
          <div v-if="browseResult.subkeys.length" class="reg-section">
            <div class="reg-section-title"><FolderOpen :size="13" /> Sous-clés ({{ browseResult.subkeys.length }})</div>
            <div class="reg-subkeys">
              <button v-for="sk in browseResult.subkeys" :key="sk" class="reg-subkey" @click="openSubkey(sk)">
                <FolderOpen :size="13" style="color:var(--warning);flex-shrink:0" />
                <span>{{ sk }}</span>
                <ChevronRight :size="12" style="margin-left:auto;opacity:.4" />
              </button>
            </div>
          </div>

          <!-- Valeurs -->
          <div class="reg-section">
            <div class="reg-section-title"><Key :size="13" /> Valeurs ({{ browseResult.values.length }})</div>
            <div v-if="browseResult && !browseResult.error" style="margin:8px 10px;display:flex;gap:6px;align-items:center">
              <input v-model="searchQuery" class="reg-addr-input" style="flex:1;font-size:12px;padding:5px 10px"
                placeholder="Rechercher dans les valeurs (nom ou données)..."
                @keyup.enter="searchRegistry" />
              <NButton variant="ghost" size="sm" :disabled="searching" @click="searchRegistry">
                <span v-if="searching">...</span><span v-else>🔍 Chercher</span>
              </NButton>
            </div>
            <div v-if="searchDone && searchResults.length === 0" style="font-size:12px;color:var(--text-muted);padding:4px 10px">
              Aucun résultat pour "{{ searchQuery }}" dans ce niveau.
            </div>
            <div v-if="searchResults.length > 0" style="margin:0 10px 10px;background:rgba(245,158,11,.06);border:1px solid rgba(245,158,11,.2);border-radius:6px;padding:8px">
              <div style="font-size:11px;color:var(--warning);margin-bottom:6px;font-weight:600">
                {{ searchResults.length }} résultat(s) pour "{{ searchQuery }}"
              </div>
              <div v-for="(r, i) in searchResults" :key="i" style="font-size:11px;padding:3px 0;border-bottom:1px solid rgba(245,158,11,.1)">
                <code style="color:var(--accent)">{{ r.name }}</code>
                <NBadge :variant="'neutral'" style="font-size:9px;margin:0 6px">{{ r.kind }}</NBadge>
                <span style="color:var(--text-secondary)">{{ r.data.length > 50 ? r.data.slice(0,50)+'...' : r.data }}</span>
              </div>
            </div>
            <div v-if="!browseResult.values.length" class="muted" style="font-size:12px;padding:6px">(Aucune valeur)</div>
            <div v-else class="reg-values-table">
              <div class="reg-values-head">
                <span>Nom</span><span>Type</span><span>Données</span><span></span>
              </div>
              <div v-for="v in browseResult.values" :key="v.name" class="reg-value-row">
                <code class="val-name">{{ v.name }}</code>
                <NBadge :variant="kindVariant(v.kind)" style="font-size:9px">{{ v.kind }}</NBadge>
                <span class="val-data">{{ v.data.length > 60 ? v.data.slice(0,60)+'...' : v.data }}</span>
                <div class="val-actions">
                  <button class="val-btn" @click="startEdit(v)" title="Modifier"><Edit2 :size="11" /></button>
                  <button class="val-btn danger" @click="deleteValue(v.name)" title="Supprimer"><Trash2 :size="11" /></button>
                </div>
              </div>
            </div>
          </div>
        </template>
      </div>

      <!-- Modal édition valeur -->
      <NModal :open="editModal" @close="editModal = false" :title="`Modifier : ${editName}`">
        <div style="display:flex;flex-direction:column;gap:10px">
          <div>
            <label style="font-size:12px;color:var(--text-muted);display:block;margin-bottom:4px">Valeur</label>
            <textarea v-model="editData" rows="4" style="width:100%;padding:8px;background:var(--bg-tertiary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-family:'JetBrains Mono',monospace;font-size:12px;outline:none;resize:vertical" />
          </div>
          <div style="font-size:11px;color:var(--warning)">⚠ Modification directe du registre — assurez-vous de savoir ce que vous faites.</div>
        </div>
        <template #footer>
          <NButton variant="ghost" @click="editModal = false">Annuler</NButton>
          <NButton variant="primary" :disabled="editSaving" @click="saveEdit">
            <NSpinner v-if="editSaving" :size="12" /> Enregistrer
          </NButton>
        </template>
      </NModal>
    </div>

    <!-- ── Sécurité (analyse persistance) ── -->
    <div v-if="activePanel === 'security'">
    <div v-if="loading" class="diag-loading"><div class="diag-spinner"></div> Analyse registre...</div>
    <div v-else-if="error" style="color:var(--error)">⚠ {{ error }}</div>
    <div v-else-if="data" style="display:flex;flex-direction:column;gap:14px">

      <!-- Bilan -->
      <div class="diag-section" :style="{ borderLeft: `3px solid ${data.total_suspicious > 0 ? 'var(--warning)' : 'var(--success)'}` }">
        <p class="diag-section-label" style="margin:0 0 8px 0">Bilan Persistance Registre</p>
        <div v-if="data.total_suspicious === 0" style="color:var(--success);font-size:13px">
          <CheckCircle :size="14" style="display:inline;margin-right:4px" />Aucune entrée suspecte détectée
        </div>
        <div v-else style="color:var(--warning);font-size:13px">
          <AlertTriangle :size="14" style="display:inline;margin-right:4px" />{{ data.total_suspicious }} entrée(s) suspecte(s) détectée(s)
        </div>
        <div class="info-grid" style="margin-top:10px">
          <div class="info-row"><span>HKLM\Run</span><NBadge variant="neutral">{{ data.run_hklm.length }}</NBadge></div>
          <div class="info-row"><span>HKCU\Run</span><NBadge variant="neutral">{{ data.run_hkcu.length }}</NBadge></div>
          <div class="info-row"><span>RunOnce</span><NBadge variant="neutral">{{ data.run_once.length }}</NBadge></div>
          <div class="info-row"><span>AppInit DLLs</span>
            <NBadge :variant="data.appinit_dlls.length > 0 ? 'danger' : 'success'">{{ data.appinit_dlls.length }}</NBadge>
          </div>
          <div class="info-row"><span>IFEO Debuggers</span>
            <NBadge :variant="data.ifeo_debuggers.length > 0 ? 'danger' : 'success'">{{ data.ifeo_debuggers.length }}</NBadge>
          </div>
          <div class="info-row"><span>Winlogon hijack</span>
            <NBadge :variant="data.winlogon.length > 0 ? 'danger' : 'success'">{{ data.winlogon.length }}</NBadge>
          </div>
        </div>
      </div>

      <!-- Entrées Run -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">
          <Key :size="13" style="display:inline;margin-right:4px" />Clés Run / RunOnce ({{ allRunEntries.length }})
        </p>
        <div v-if="!allRunEntries.length" class="muted" style="font-size:13px">Aucune entrée.</div>
        <div v-for="(e, i) in allRunEntries" :key="i"
          style="padding:7px 0;border-bottom:1px solid var(--border)">
          <div style="display:flex;align-items:center;gap:8px;margin-bottom:3px">
            <component :is="e.suspicious ? AlertTriangle : CheckCircle" :size="12"
              :class="e.suspicious ? 'ic-warn' : 'ic-ok'" />
            <code style="font-size:11px;color:var(--accent)">{{ e.name }}</code>
            <NBadge :variant="e.hive === 'HKLM' ? 'info' : 'neutral'" style="font-size:9px">{{ e.hive }}</NBadge>
            <NBadge v-if="e.suspicious" variant="danger" style="font-size:9px">SUSPECT</NBadge>
            <button class="val-btn" style="font-size:10px;padding:1px 6px;width:auto" @click="openInRegedit(e.key)" title="Ouvrir dans Regedit">↗ Regedit</button>
          </div>
          <div class="muted" style="font-size:11px;padding-left:20px;word-break:break-all">{{ e.value }}</div>
        </div>
      </div>

      <!-- AppInit DLLs (très suspect) -->
      <div v-if="data.appinit_dlls.length" class="diag-section"
        style="border-left:3px solid var(--error)">
        <p class="diag-section-label" style="margin:0 0 8px 0">⚠ AppInit DLLs (malware classique)</p>
        <div v-for="(d, i) in data.appinit_dlls" :key="i"
          style="padding:4px 0;font-size:12px">
          <AlertTriangle :size="12" class="ic-warn" style="display:inline;margin-right:6px" />
          <code>{{ d }}</code>
        </div>
      </div>

      <!-- IFEO Debugger hijacks -->
      <div v-if="data.ifeo_debuggers.length" class="diag-section"
        style="border-left:3px solid var(--error)">
        <p class="diag-section-label" style="margin:0 0 8px 0">⚠ IFEO — Debugger Hijacks</p>
        <div v-for="(e, i) in data.ifeo_debuggers" :key="i" class="list-row">
          <AlertTriangle :size="12" class="ic-warn" style="flex-shrink:0" />
          <code class="list-name">{{ e.key.split('\\').pop() }}</code>
          <div class="muted" style="flex:1;overflow:hidden;text-overflow:ellipsis;font-size:11px">→ {{ e.value }}</div>
          <button class="val-btn" style="font-size:10px;padding:1px 6px;width:auto;flex-shrink:0" @click="openInRegedit(e.key)" title="Ouvrir dans Regedit">↗ Regedit</button>
        </div>
      </div>

      <!-- Winlogon hijacks -->
      <div v-if="data.winlogon.length" class="diag-section"
        style="border-left:3px solid var(--error)">
        <p class="diag-section-label" style="margin:0 0 8px 0">⚠ Winlogon — Modifications suspectes</p>
        <div v-for="(e, i) in data.winlogon" :key="i" class="list-row">
          <AlertTriangle :size="12" class="ic-warn" style="flex-shrink:0" />
          <code class="list-name">{{ e.name }}</code>
          <div class="muted" style="flex:1;font-size:11px">{{ e.value }}</div>
          <button class="val-btn" style="font-size:10px;padding:1px 6px;width:auto;flex-shrink:0" @click="openInRegedit(e.key)" title="Ouvrir dans Regedit">↗ Regedit</button>
        </div>
      </div>

      <!-- LSA packages hors standard -->
      <div v-if="data.lsa_packages.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">LSA Packages non-standard</p>
        <div style="display:flex;gap:6px;flex-wrap:wrap">
          <code v-for="(p, i) in data.lsa_packages" :key="i"
            style="font-size:11px;background:var(--bg-secondary);padding:2px 6px;border-radius:4px;color:var(--warning)">{{ p }}</code>
        </div>
      </div>

      <!-- Proxy / hijack navigateur -->
      <div v-if="data.browser_hijack.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Proxy Internet détecté</p>
        <div v-for="(e, i) in data.browser_hijack" :key="i" class="list-row">
          <code class="list-name">{{ e.name }}</code>
          <div class="muted" style="flex:1;font-size:11px">{{ e.value }}</div>
          <button class="val-btn" style="font-size:10px;padding:1px 6px;width:auto;flex-shrink:0" @click="openInRegedit(e.key)" title="Ouvrir dans Regedit">↗ Regedit</button>
        </div>
      </div>
    </div>
    </div><!-- fin panel sécurité -->

  </div>
</template>

<style scoped>
/* Onglets panneau */
.reg-panel-tabs { display:flex; gap:4px; border-bottom:1px solid var(--border); margin-bottom:12px; }
.reg-tab { display:flex; align-items:center; gap:6px; padding:8px 16px; border:none; border-bottom:2px solid transparent; background:none; color:var(--text-muted); font-family:inherit; font-size:13px; font-weight:500; cursor:pointer; transition:all .15s; }
.reg-tab:hover { color:var(--text-primary); }
.reg-tab.active { color:var(--accent-primary); border-bottom-color:var(--accent-primary); }

/* Navigateur */
.reg-browser { display:flex; flex-direction:column; gap:10px; }
.reg-addr-bar { display:flex; gap:8px; }
.reg-addr-input { flex:1; padding:8px 12px; background:var(--bg-tertiary); border:1px solid var(--border); border-radius:var(--radius-md); color:var(--text-primary); font-family:"JetBrains Mono",monospace; font-size:12px; outline:none; }
.reg-addr-input:focus { border-color:var(--accent-primary); }
.reg-breadcrumb { display:flex; flex-wrap:wrap; align-items:center; gap:2px; font-size:11px; padding:4px 0; }
.crumb-btn { background:none; border:none; color:var(--text-secondary); cursor:pointer; font-family:inherit; font-size:11px; padding:2px 4px; border-radius:3px; }
.crumb-btn:hover { background:var(--bg-tertiary); color:var(--accent-primary); }
.crumb-sep { color:var(--border-hover); margin:0 2px; }
.reg-hives { display:flex; flex-direction:column; gap:6px; }
.hive-btn { display:flex; align-items:center; gap:10px; padding:10px 14px; border:1px solid var(--border); border-radius:8px; background:var(--bg-secondary); color:var(--text-primary); font-family:"JetBrains Mono",monospace; font-size:12px; cursor:pointer; transition:border-color .15s; text-align:left; }
.hive-btn:hover { border-color:var(--warning); }
.reg-section { background:var(--bg-secondary); border:1px solid var(--border); border-radius:8px; overflow:hidden; }
.reg-section-title { display:flex; align-items:center; gap:6px; padding:8px 12px; font-size:11px; font-weight:600; text-transform:uppercase; letter-spacing:.04em; color:var(--text-muted); border-bottom:1px solid var(--border); background:var(--bg-tertiary); }
.reg-subkeys { display:flex; flex-direction:column; max-height:200px; overflow-y:auto; }
.reg-subkey { display:flex; align-items:center; gap:8px; padding:7px 12px; border:none; background:none; color:var(--text-primary); font-family:"JetBrains Mono",monospace; font-size:12px; cursor:pointer; transition:background .1s; text-align:left; border-bottom:1px solid var(--border); }
.reg-subkey:last-child { border-bottom:none; }
.reg-subkey:hover { background:var(--bg-tertiary); }
.reg-subkey span { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.reg-values-table { display:flex; flex-direction:column; max-height:280px; overflow-y:auto; }
.reg-values-head { display:grid; grid-template-columns:1fr 90px 2fr 60px; padding:5px 10px; font-size:10px; font-weight:700; text-transform:uppercase; letter-spacing:.04em; color:var(--text-muted); border-bottom:1px solid var(--border); background:var(--bg-tertiary); }
.reg-value-row { display:grid; grid-template-columns:1fr 90px 2fr 60px; align-items:center; padding:6px 10px; border-bottom:1px solid var(--border); font-size:12px; transition:background .1s; }
.reg-value-row:last-child { border-bottom:none; }
.reg-value-row:hover { background:var(--bg-tertiary); }
.val-name { font-family:"JetBrains Mono",monospace; color:var(--accent-primary); font-size:11px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.val-data { color:var(--text-secondary); font-size:11px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.val-actions { display:flex; gap:3px; justify-content:flex-end; }
.val-btn { display:flex; align-items:center; justify-content:center; width:22px; height:22px; border:none; border-radius:4px; background:var(--bg-tertiary); color:var(--text-muted); cursor:pointer; transition:all .1s; }
.val-btn:hover { background:var(--bg-elevated); color:var(--text-primary); }
.val-btn.danger:hover { background:var(--danger-muted); color:var(--danger); }
@keyframes spin { to { transform:rotate(360deg); } }
</style>
