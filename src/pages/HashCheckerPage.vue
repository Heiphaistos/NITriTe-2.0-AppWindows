<script setup lang="ts">
import { ref, computed } from "vue";
import { csvCell } from "@/composables/useExportData";
import { invoke } from "@/utils/invoke";
import { fetchWithTimeout } from "@/utils/fetchTimeout";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import { useNotificationStore } from "@/stores/notifications";
import { Hash, Upload, Copy, CheckCircle, FolderOpen, Download, Trash2, Shield, AlertTriangle, ExternalLink, Scan } from "lucide-vue-next";

const notify = useNotificationStore();

interface HashResult { path: string; algorithm: string; hash: string; size_bytes: number; }
interface VtResult { hash: string; malicious: number; suspicious: number; undetected: number; total: number; link: string; queried: boolean; loading: boolean; }

const results      = ref<HashResult[]>([]);
const vtResults    = ref<Record<string, VtResult>>({});
const loading      = ref(false);
const selectedAlgo = ref("SHA256");
const algos        = ["MD5", "SHA1", "SHA256"];
const verifyHash   = ref("");
const referenceHash = ref("");
const dragOver     = ref(false);
const batchMode    = ref(false);
const batchTotal   = ref(0);
const batchDone    = ref(0);
const vtApiKey     = ref((() => { try { return localStorage.getItem("vt_api_key") || ""; } catch { return ""; } })());
const showVtConfig = ref(false);

function saveVtKey() {
  try { localStorage.setItem("vt_api_key", vtApiKey.value); } catch { /* storage indisponible */ }
  showVtConfig.value = false;
  notify.success("Clé sauvegardée", "Clé VirusTotal enregistrée");
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

async function hashPath(path: string) {
  if (!path) return;
  loading.value = true;
  try {
    const r = await invoke<HashResult>("hash_file", { path, algorithm: selectedAlgo.value });
    const idx = results.value.findIndex(x => x.path === path && x.algorithm === selectedAlgo.value);
    if (idx !== -1) results.value.splice(idx, 1);
    results.value.unshift(r);
    if (results.value.length > 50) results.value.pop();
  } catch (e: unknown) {
    notify.error("Erreur hachage", String(e));
  } finally {
    loading.value = false;
  }
}

async function hashFolder(path: string) {
  loading.value = true; batchMode.value = true; batchDone.value = 0; batchTotal.value = 0;
  try {
    const res = await invoke<HashResult[]>("hash_folder", { path, algorithm: selectedAlgo.value, maxFiles: 200 });
    batchTotal.value = res.length;
    for (const r of res) {
      results.value.unshift(r);
      batchDone.value++;
    }
    if (results.value.length > 200) results.value.splice(200);
    notify.success("Batch terminé", `${res.length} fichier(s) haché(s)`);
  } catch (e: unknown) {
    notify.error("Erreur batch", String(e));
  } finally {
    loading.value = false; batchMode.value = false;
  }
}

async function pickFile() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const p = await open({ title: "Sélectionner un fichier ou dossier", directory: false, multiple: false });
    if (p && typeof p === "string") await hashPath(p);
  } catch (e: unknown) { notify.error("Erreur", String(e)); }
}

async function pickFolder() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const p = await open({ title: "Sélectionner un dossier", directory: true });
    if (p && typeof p === "string") await hashFolder(p);
  } catch (e: unknown) { notify.error("Erreur", String(e)); }
}

function onDrop(e: DragEvent) {
  dragOver.value = false;
  const files = e.dataTransfer?.files;
  if (!files || files.length === 0) return;
  const f = files[0] as File & { path?: string };
  // f.name est un simple nom de fichier ("rapport.pdf"), pas un chemin — le
  // hacher tel quel enverrait hash_file sur un chemin relatif au CWD de l'app
  // (jamais l'emplacement réel du fichier glissé), soit une erreur "introuvable"
  // trompeuse, soit — pire — le hash d'un AUTRE fichier portant le même nom
  // s'il en existe un dans le CWD. Sans f.path exploitable, mieux vaut refuser
  // explicitement que deviner.
  if (!f.path) {
    notify.error("Chemin indisponible", "Impossible de récupérer le chemin réel du fichier déposé — utilisez le bouton de sélection.");
    return;
  }
  hashPath(f.path);
}

function copyHash(hash: string) {
  navigator.clipboard.writeText(hash).then(() => notify.success("Copié", hash.slice(0, 20) + "..."));
}

// VirusTotal lookup via API publique (v3)
async function checkVirusTotal(r: HashResult) {
  if (!vtApiKey.value) {
    notify.error("Clé VT manquante", "Configurez votre clé API VirusTotal");
    showVtConfig.value = true;
    return;
  }
  const key = r.hash + "_" + r.algorithm;
  vtResults.value[key] = { hash: r.hash, malicious: 0, suspicious: 0, undetected: 0, total: 0, link: "", queried: false, loading: true };
  const vtAbort = new AbortController();
  const vtTimeout = setTimeout(() => vtAbort.abort(), 15000);
  try {
    const resp = await fetch(`https://www.virustotal.com/api/v3/files/${r.hash}`, {
      headers: { "x-apikey": vtApiKey.value },
      signal: vtAbort.signal,
    });
    if (resp.status === 404) {
      vtResults.value[key] = { hash: r.hash, malicious: 0, suspicious: 0, undetected: 0, total: 0, link: `https://www.virustotal.com/gui/file/${r.hash}`, queried: true, loading: false };
      notify.info("Non trouvé", "Hash inconnu de VirusTotal");
      return;
    }
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
    const data = await resp.json();
    const stats = data?.data?.attributes?.last_analysis_stats || {};
    vtResults.value[key] = {
      hash: r.hash,
      malicious: stats.malicious || 0,
      suspicious: stats.suspicious || 0,
      undetected: stats.undetected || 0,
      total: (stats.malicious || 0) + (stats.suspicious || 0) + (stats.undetected || 0) + (stats.harmless || 0),
      link: `https://www.virustotal.com/gui/file/${r.hash}`,
      queried: true,
      loading: false,
    };
  } catch (e: unknown) {
    delete vtResults.value[key];
    const msg = e instanceof Error && e.name === "AbortError" ? "Délai dépassé (15s)" : (e instanceof Error ? e.message : String(e)).slice(0, 120);
    notify.error("Erreur VT", msg);
  } finally {
    clearTimeout(vtTimeout);
  }
}

function vtKey(r: HashResult) { return r.hash + "_" + r.algorithm; }

// ── Scan rapide sans clé API ────────────────────────────────────────────────
interface QuickResult { status: "loading" | "safe" | "malware" | "unknown"; detail: string; }
const quickResults = ref<Record<string, QuickResult>>({});

async function checkQuick(r: HashResult) {
  const key = vtKey(r);
  quickResults.value[key] = { status: "loading", detail: "Vérification en cours..." };
  const hash = r.hash.toLowerCase();
  const algo = r.algorithm.toLowerCase(); // md5, sha1, sha256

  // 1. CIRCL HASHLOOKUP — base NSRL (fichiers connus sains, pas de clé requise)
  try {
    const resp = await fetchWithTimeout(`https://hashlookup.circl.lu/lookup/${algo}/${hash}`, {}, 10000);
    if (resp.ok) {
      const data = await resp.json();
      const name = data["FileName"] || data["ProductName"] || "fichier référencé";
      quickResults.value[key] = { status: "safe", detail: `Connu et sain — ${name} (NSRL/CIRCL)` };
      return;
    }
  } catch { /* service indisponible, continue */ }

  // 2. MalwareBazaar — base de malwares connus (abuse.ch, pas de clé requise)
  try {
    const form = new FormData();
    form.append("query", "get_info");
    form.append("hash", hash);
    const resp = await fetchWithTimeout("https://mb-api.abuse.ch/api/v1/", { method: "POST", body: form }, 15000);
    if (resp.ok) {
      const data = await resp.json();
      if (data.query_status === "hash_not_found") {
        quickResults.value[key] = { status: "unknown", detail: "Non trouvé dans les bases connues (ni sain ni malveillant)" };
      } else if (data.query_status === "ok" && data.data?.length) {
        const info = data.data[0];
        const tags = info.tags ? info.tags.join(", ") : "";
        quickResults.value[key] = { status: "malware", detail: `MALWARE : ${info.signature || "inconnu"}${tags ? ` (${tags})` : ""}` };
      } else {
        quickResults.value[key] = { status: "unknown", detail: "Résultat indéterminé" };
      }
      return;
    }
  } catch { /* service indisponible */ }

  quickResults.value[key] = { status: "unknown", detail: "Services de lookup indisponibles (vérifiez la connexion)" };
}

function vtStatus(vt: VtResult): "clean" | "suspicious" | "malicious" | "unknown" {
  if (!vt.queried) return "unknown";
  if (vt.total === 0) return "unknown";
  if (vt.malicious > 0) return "malicious";
  if (vt.suspicious > 0) return "suspicious";
  return "clean";
}

// Comparaison hash référence
const referenceMatch = computed(() => {
  if (!referenceHash.value.trim() || results.value.length === 0) return null;
  const ref = referenceHash.value.trim().toLowerCase();
  // Chercher la correspondance parmi TOUS les fichiers hashés, pas seulement le
  // premier : sinon un hash de référence valide pour le 2e fichier serait
  // rapporté « modifié ».
  return results.value.some(r => r.hash.toLowerCase() === ref);
});

function exportCsv() {
  const rows = ["Fichier,Algorithme,Hash,Taille,VT_Malicieux,VT_Suspicieux,VT_Total,VT_Lien"];
  for (const r of results.value) {
    const vt = vtResults.value[vtKey(r)];
    rows.push([r.path, r.algorithm, r.hash, r.size_bytes, vt?.malicious ?? "", vt?.suspicious ?? "", vt?.total ?? "", vt?.link ?? ""].map(csvCell).join(","));
  }
  const blob = new Blob([rows.join("\n")], { type: "text/csv" });
  const a = document.createElement("a"); a.href = URL.createObjectURL(blob);
  a.download = `hashes_${new Date().toISOString().slice(0,10)}.csv`; a.click();
}

function clearResults() { results.value = []; verifyHash.value = ""; referenceHash.value = ""; vtResults.value = {}; }
</script>

<template>
  <div class="hash-page">
    <div class="page-header">
      <div>
        <h1>Vérificateur de Hash</h1>
        <p class="page-subtitle">Calculez et vérifiez l'intégrité de vos fichiers — MD5, SHA1, SHA256 + VirusTotal</p>
      </div>
      <div style="display:flex;gap:8px;align-items:center">
        <NButton variant="ghost" size="sm" @click="showVtConfig = !showVtConfig" title="Configuration VirusTotal">
          <Shield :size="14" /> VT
        </NButton>
        <template v-if="results.length">
          <NButton variant="ghost" size="sm" @click="exportCsv"><Download :size="14" /> CSV</NButton>
          <NButton variant="ghost" size="sm" @click="clearResults"><Trash2 :size="14" /></NButton>
        </template>
      </div>
    </div>

    <!-- Config VT -->
    <div v-if="showVtConfig" class="vt-config-banner">
      <Shield :size="14" style="color:var(--accent-primary);flex-shrink:0" />
      <span style="font-size:12px;color:var(--text-muted);white-space:nowrap">Clé API VirusTotal :</span>
      <input v-model="vtApiKey" class="vt-key-input" type="password" placeholder="Votre clé VT publique (gratuite sur virustotal.com)" />
      <NButton variant="primary" size="sm" @click="saveVtKey">Sauvegarder</NButton>
      <NButton variant="ghost" size="sm" @click="showVtConfig = false">Fermer</NButton>
    </div>

    <div class="hash-layout">
      <NCard>
        <template #header>
          <div style="display:flex;align-items:center;gap:8px">
            <Hash :size="16" />
            <span>Calculer un hash</span>
          </div>
        </template>

        <div class="algo-row">
          <span style="font-size:12px;color:var(--text-muted)">Algorithme :</span>
          <div class="algo-tabs">
            <button v-for="a in algos" :key="a" class="algo-btn" :class="{ active: selectedAlgo === a }" @click="selectedAlgo = a">{{ a }}</button>
          </div>
        </div>

        <!-- Drop zone fichier -->
        <div
          class="drop-zone"
          :class="{ 'over': dragOver, loading }"
          @dragover.prevent="dragOver = true"
          @dragleave="dragOver = false"
          @drop.prevent="onDrop"
          @click="pickFile"
        >
          <Upload :size="28" style="color:var(--text-muted);opacity:.5" />
          <p class="drop-label">Glissez un fichier ici</p>
          <p class="drop-sub">ou cliquez pour parcourir</p>
          <div v-if="loading && !batchMode" class="drop-spinner">Calcul en cours...</div>
        </div>

        <!-- Batch dossier -->
        <NButton variant="secondary" size="sm" style="width:100%;margin-top:8px" :loading="loading && batchMode" @click="pickFolder">
          <FolderOpen :size="14" /> Hacher un dossier entier (batch)
        </NButton>

        <!-- Progression batch -->
        <div v-if="batchMode && batchTotal > 0" class="batch-progress">
          <div class="batch-bar" :style="{ width: `${(batchDone / batchTotal) * 100}%` }"></div>
          <span>{{ batchDone }} / {{ batchTotal }}</span>
        </div>

        <!-- Hash référence (comparaison officielle) -->
        <div class="verify-row">
          <label class="field-label">Hash de référence (officiel)</label>
          <input v-model="referenceHash" class="verify-input" placeholder="Hash publié par l'éditeur..." />
          <div v-if="referenceHash.trim() && results.length" class="ref-result">
            <span v-if="referenceMatch === true" class="match ok"><CheckCircle :size="12" /> Hash identique — fichier intact</span>
            <span v-else-if="referenceMatch === false" class="match nok"><AlertTriangle :size="12" /> Hash différent — fichier modifié !</span>
          </div>
        </div>

        <!-- Verify input (comparaison manuelle) -->
        <div class="verify-row">
          <label class="field-label">Comparer avec un autre hash</label>
          <input v-model="verifyHash" class="verify-input" placeholder="Collez un hash à comparer..." />
        </div>
      </NCard>

      <!-- Results -->
      <NCard>
        <template #header>
          <span>Résultats ({{ results.length }})</span>
        </template>

        <div v-if="results.length === 0" class="empty-state">
          <Hash :size="32" style="color:var(--text-muted);opacity:.2" />
          <p>Aucun fichier analysé</p>
        </div>

        <div v-else class="results-list">
          <div v-for="r in results" :key="r.path + r.algorithm" class="result-card">
            <div class="res-header">
              <span class="res-filename">{{ r.path.split('\\').pop() || r.path }}</span>
              <NBadge variant="accent">{{ r.algorithm }}</NBadge>
              <NBadge variant="neutral">{{ formatSize(r.size_bytes) }}</NBadge>
            </div>
            <div class="hash-row">
              <code class="hash-value">{{ r.hash }}</code>
              <button class="copy-btn" @click="copyHash(r.hash)" title="Copier"><Copy :size="13" /></button>
            </div>

            <!-- Comparaison verifyHash -->
            <div v-if="verifyHash.trim()" class="compare-row">
              <span v-if="r.hash.toLowerCase() === verifyHash.trim().toLowerCase()" class="match ok">✓ Hash identique</span>
              <span v-else class="match nok">✗ Hash différent</span>
            </div>

            <!-- Scan rapide sans clé -->
            <div class="vt-row" style="margin-top:6px">
              <template v-if="quickResults[vtKey(r)]?.status === 'loading'">
                <NSpinner :size="12" /><span class="vt-label">Analyse en cours...</span>
              </template>
              <template v-else-if="quickResults[vtKey(r)]">
                <div class="vt-badge" :class="quickResults[vtKey(r)].status">
                  <Scan :size="11" />
                  <span>{{ quickResults[vtKey(r)].detail }}</span>
                </div>
              </template>
              <template v-else>
                <button class="vt-check-btn" @click="checkQuick(r)">
                  <Scan :size="11" /> Scan rapide (sans clé API)
                </button>
              </template>
            </div>

            <!-- VirusTotal -->
            <div class="vt-row">
              <template v-if="vtResults[vtKey(r)]?.loading">
                <NSpinner :size="12" /><span class="vt-label">Interrogation VirusTotal...</span>
              </template>
              <template v-else-if="vtResults[vtKey(r)]?.queried">
                <div class="vt-badge" :class="vtStatus(vtResults[vtKey(r)])">
                  <Shield :size="11" />
                  <span v-if="vtStatus(vtResults[vtKey(r)]) === 'clean'">Propre (0/{{ vtResults[vtKey(r)].total }})</span>
                  <span v-else-if="vtStatus(vtResults[vtKey(r)]) === 'malicious'">Malveillant — {{ vtResults[vtKey(r)].malicious }}/{{ vtResults[vtKey(r)].total }}</span>
                  <span v-else-if="vtStatus(vtResults[vtKey(r)]) === 'suspicious'">Suspect — {{ vtResults[vtKey(r)].suspicious }}/{{ vtResults[vtKey(r)].total }}</span>
                  <span v-else>Non trouvé dans VT</span>
                </div>
                <a v-if="vtResults[vtKey(r)].link" :href="vtResults[vtKey(r)].link" target="_blank" class="vt-link" title="Voir sur VirusTotal"><ExternalLink :size="11" /></a>
              </template>
              <template v-else>
                <button class="vt-check-btn" @click="checkVirusTotal(r)" :disabled="!vtApiKey">
                  <Shield :size="11" /> Vérifier sur VirusTotal
                </button>
              </template>
            </div>

            <div class="res-path">{{ r.path }}</div>
          </div>
        </div>
      </NCard>
    </div>
  </div>
</template>

<style scoped>
.hash-page { display:flex; flex-direction:column; gap:16px; }
.page-header { display:flex; justify-content:space-between; align-items:flex-start; flex-wrap:wrap; gap:12px; }
.page-header h1 { font-size:24px; font-weight:700; }
.page-subtitle { color:var(--text-muted); font-size:13px; margin-top:2px; }

.hash-layout { display:grid; grid-template-columns:1fr 1fr; gap:16px; align-items:start; }
@media (max-width:900px) { .hash-layout { grid-template-columns:1fr; } }

.vt-config-banner {
  display:flex; align-items:center; gap:10px; flex-wrap:wrap;
  padding:10px 16px; background:var(--accent-muted); border:1px solid var(--accent-primary);
  border-radius:var(--radius-md);
}
.vt-key-input {
  flex:1; min-width:200px; padding:6px 10px; border:1px solid var(--border); border-radius:var(--radius-sm);
  background:var(--bg-tertiary); color:var(--text-primary); font-size:12px; outline:none; font-family:monospace;
}
.vt-key-input:focus { border-color:var(--accent-primary); }

.algo-row { display:flex; align-items:center; gap:10px; margin-bottom:16px; }
.algo-tabs { display:flex; gap:4px; }
.algo-btn { padding:3px 12px; border:1px solid var(--border); border-radius:var(--radius-sm); background:var(--bg-tertiary); color:var(--text-secondary); cursor:pointer; font-size:12px; font-family:monospace; }
.algo-btn.active { border-color:var(--accent-primary); color:var(--accent-primary); background:var(--accent-muted); }

.drop-zone {
  border:2px dashed var(--border); border-radius:var(--radius-lg); padding:36px 24px;
  display:flex; flex-direction:column; align-items:center; gap:6px;
  cursor:pointer; background:var(--bg-secondary); transition:all var(--transition-fast);
}
.drop-zone:hover, .drop-zone.over { border-color:var(--accent-primary); background:var(--accent-muted); }
.drop-zone.loading { opacity:.6; pointer-events:none; }
.drop-label { font-size:14px; font-weight:600; color:var(--text-secondary); }
.drop-sub   { font-size:12px; color:var(--text-muted); }
.drop-spinner { font-size:12px; color:var(--accent-primary); animation: pulse 1s infinite; }

.batch-progress { position:relative; height:4px; background:var(--bg-tertiary); border-radius:99px; overflow:hidden; margin-top:8px; display:flex; align-items:center; }
.batch-bar { height:100%; background:var(--accent-primary); border-radius:99px; transition:width .3s ease; }
.batch-progress span { position:absolute; right:0; font-size:10px; color:var(--text-muted); top:-16px; }

.verify-row { margin-top:12px; }
.field-label { font-size:11px; color:var(--text-muted); font-weight:600; text-transform:uppercase; letter-spacing:.06em; display:block; margin-bottom:6px; }
.verify-input { width:100%; padding:8px 12px; border:1px solid var(--border); border-radius:var(--radius-md); background:var(--bg-tertiary); color:var(--text-primary); font-family:monospace; font-size:12px; outline:none; box-sizing:border-box; }
.verify-input:focus { border-color:var(--accent-primary); }

.ref-result { margin-top:6px; }

.empty-state { display:flex; flex-direction:column; align-items:center; gap:8px; padding:48px; color:var(--text-muted); font-size:13px; }
.results-list { display:flex; flex-direction:column; gap:10px; max-height:600px; overflow-y:auto; }
.result-card { padding:12px; background:var(--bg-tertiary); border-radius:var(--radius-md); border:1px solid var(--border); }
.res-header { display:flex; align-items:center; gap:8px; margin-bottom:8px; flex-wrap:wrap; }
.res-filename { font-weight:600; font-size:13px; flex:1; min-width:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.hash-row { display:flex; align-items:center; gap:8px; }
.hash-value { font-size:11px; font-family:monospace; color:var(--accent-primary); flex:1; word-break:break-all; }
.copy-btn { background:none; border:none; color:var(--text-muted); cursor:pointer; padding:4px; border-radius:4px; }
.copy-btn:hover { color:var(--text-primary); background:var(--bg-secondary); }
.compare-row { margin-top:6px; }
.match { font-size:12px; font-weight:600; display:flex; align-items:center; gap:4px; }
.match.ok  { color:var(--success); }
.match.nok { color:var(--danger); }
.res-path { font-size:10px; color:var(--text-muted); margin-top:4px; font-family:monospace; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }

/* VirusTotal */
.vt-row { display:flex; align-items:center; gap:6px; margin-top:8px; }
.vt-label { font-size:11px; color:var(--text-muted); }
.vt-badge {
  display:inline-flex; align-items:center; gap:4px; padding:2px 8px;
  border-radius:99px; font-size:11px; font-weight:600; border:1px solid;
}
.vt-badge.clean, .vt-badge.safe { color:#22c55e; background:rgba(34,197,94,.1); border-color:rgba(34,197,94,.3); }
.vt-badge.malicious, .vt-badge.malware { color:#ef4444; background:rgba(239,68,68,.1); border-color:rgba(239,68,68,.3); }
.vt-badge.suspicious { color:#f59e0b; background:rgba(245,158,11,.1); border-color:rgba(245,158,11,.3); }
.vt-badge.unknown { color:var(--text-muted); background:var(--bg-secondary); border-color:var(--border); }
.vt-link { color:var(--text-muted); display:flex; align-items:center; padding:2px; }
.vt-link:hover { color:var(--accent-primary); }
.vt-check-btn {
  background:none; border:1px solid var(--border); border-radius:var(--radius-sm);
  color:var(--text-muted); font-size:11px; padding:2px 8px; cursor:pointer;
  display:inline-flex; align-items:center; gap:4px; transition:all .15s;
}
.vt-check-btn:hover:not(:disabled) { color:var(--accent-primary); border-color:var(--accent-primary); background:var(--accent-muted); }
.vt-check-btn:disabled { opacity:.4; cursor:default; }

@keyframes pulse { 0%,100%{opacity:1} 50%{opacity:.5} }
</style>
