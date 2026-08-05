<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import { useNotificationStore } from "@/stores/notifications";
import { Globe, RefreshCw, Trash2, Zap, Activity, FlaskConical, History, ShieldCheck } from "lucide-vue-next";

const notify = useNotificationStore();

interface DnsPreset  { id: string; name: string; primary: string; secondary: string; description: string; }
interface AdapterInfo { name: string; description: string; status: string; current_dns: string[]; }

interface DnsTestResult {
  server: string;
  host: string;
  records: string[];
  success: boolean;
  latency_ms: number;
  error?: string;
}

interface DnsHistoryEntry {
  timestamp: string;
  action: string;
  adapter: string;
  dns: string;
}

const presets         = ref<DnsPreset[]>([]);
const adapters        = ref<AdapterInfo[]>([]);
const selectedAdapter = ref("");
const selectedPreset  = ref("");
const loading         = ref(false);
const applying        = ref(false);
const flushing        = ref(false);
const testing         = ref(false);
const customPrimary   = ref("");
const customSecondary = ref("");
const showCustom      = ref(false);
const latencies       = ref<Record<string, number>>({});

// DNS Real Test (nslookup)
const dnsTestHost     = ref("google.com");
const dnsTestServer   = ref("");
const dnsRealTesting  = ref(false);
const dnsTestResults  = ref<DnsTestResult[]>([]);

// DNS Change History
const dnsHistory      = ref<DnsHistoryEntry[]>([]);

const presetColors: Record<string, string> = {
  google: "#4285f4", cloudflare: "#f38020", "cloudflare-doh": "#f38020",
  opendns: "#6bb5ff", nextdns: "#00adef", quad9: "#7b5ea7", adguard: "#68bc71", auto: "#6b7280",
};

function latencyVariant(ms: number): "success"|"warning"|"danger"|"neutral" {
  if (ms < 0)   return "danger";
  if (ms < 30)  return "success";
  if (ms < 80)  return "warning";
  return "danger";
}
function latencyLabel(ms: number): string {
  if (ms < 0) return "Timeout";
  return `${ms} ms`;
}

function loadDnsHistory() {
  try {
    const raw = localStorage.getItem("nitrite_dns_history");
    if (raw) dnsHistory.value = JSON.parse(raw);
  } catch { /* ignore */ }
}

function saveDnsHistory(entry: DnsHistoryEntry) {
  dnsHistory.value.unshift(entry);
  if (dnsHistory.value.length > 30) dnsHistory.value = dnsHistory.value.slice(0, 30);
  try {
    localStorage.setItem("nitrite_dns_history", JSON.stringify(dnsHistory.value));
  } catch { /* ignore */ }
}

function clearDnsHistory() {
  dnsHistory.value = [];
  localStorage.removeItem("nitrite_dns_history");
  notify.success("Historique efface", "L'historique DNS a ete supprime");
}

onMounted(async () => {
  loading.value = true;
  loadDnsHistory();
  try {
    [presets.value, adapters.value] = await Promise.all([
      invoke<DnsPreset[]>("get_dns_presets"),
      invoke<AdapterInfo[]>("get_network_adapters_for_dns"),
    ]);
    if (adapters.value.length > 0) selectedAdapter.value = adapters.value[0].name;
  } catch (e: unknown) {
    notify.error("Erreur chargement", String(e));
  } finally {
    loading.value = false;
  }
});

async function testAllLatencies() {
  testing.value = true;
  latencies.value = {};
  const toTest = presets.value.filter(p => p.primary);
  await Promise.all(toTest.map(async p => {
    try {
      const ms = await invoke<number>("ping_dns", { ip: p.primary });
      latencies.value[p.id] = ms;
    } catch { latencies.value[p.id] = -1; }
  }));
  testing.value = false;
  notify.success("Test termine", "Latences DNS mesurees");
}

// Real DNS resolution test via run_nslookup
async function testDnsReal() {
  const host = dnsTestHost.value.trim();
  if (!host) return;
  dnsRealTesting.value = true;
  dnsTestResults.value = [];

  // Determine which servers to test
  const serversToTest: { id: string; name: string; ip: string }[] = [];

  if (dnsTestServer.value.trim()) {
    // Custom server specified
    serversToTest.push({ id: "custom", name: "Personnalise", ip: dnsTestServer.value.trim() });
  } else {
    // Test all preset primary servers
    presets.value
      .filter(p => p.primary && p.primary !== "")
      .forEach(p => serversToTest.push({ id: p.id, name: p.name, ip: p.primary }));
  }

  await Promise.all(serversToTest.map(async srv => {
    const before = Date.now();
    try {
      const result = await invoke<{ host: string; records: string[]; query_type: string; success: boolean }>(
        "run_nslookup",
        { host, recordType: "A", dnsServer: srv.ip }
      );
      const elapsed = Date.now() - before;
      dnsTestResults.value.push({
        server: srv.name,
        host,
        records: result.records,
        success: result.success,
        latency_ms: elapsed,
      });
    } catch (e: unknown) {
      const elapsed = Date.now() - before;
      dnsTestResults.value.push({
        server: srv.name,
        host,
        records: [],
        success: false,
        latency_ms: elapsed,
        error: String(e),
      });
    }
  }));

  // Sort: success first, then by latency
  dnsTestResults.value.sort((a, b) => {
    if (a.success && !b.success) return -1;
    if (!a.success && b.success) return 1;
    return a.latency_ms - b.latency_ms;
  });

  dnsRealTesting.value = false;
  if (dnsTestResults.value.some(r => r.success)) {
    notify.success("Test DNS reel", `Resolution de "${host}" terminee`);
  } else {
    notify.warning("Test DNS reel", `Aucune resolution reussie pour "${host}"`);
  }
}

async function applyDns() {
  if (!selectedAdapter.value) { notify.warning("Aucun adaptateur", "Selectionnez un adaptateur reseau"); return; }
  const preset = showCustom.value ? null : presets.value.find(p => p.id === selectedPreset.value);
  if (!showCustom.value && !preset) { notify.warning("Aucun preset", "Selectionnez un serveur DNS"); return; }
  const primary   = showCustom.value ? customPrimary.value   : preset!.primary;
  const secondary = showCustom.value ? customSecondary.value : preset!.secondary;
  applying.value = true;
  try {
    await invoke("switch_dns", { adapter: selectedAdapter.value, primary, secondary });
    notify.success("DNS change", `Adaptateur : ${selectedAdapter.value}`);
    adapters.value = await invoke<AdapterInfo[]>("get_network_adapters_for_dns");

    // Save to history
    const dnsLabel = showCustom.value
      ? `Personnalise (${primary})`
      : `${preset?.name} (${primary})`;
    saveDnsHistory({
      timestamp: new Date().toLocaleString("fr-FR"),
      action: "Changement DNS",
      adapter: selectedAdapter.value,
      dns: dnsLabel,
    });
  } catch (e: unknown) {
    notify.error("Erreur DNS", String(e));
  } finally { applying.value = false; }
}

async function flush() {
  flushing.value = true;
  try {
    await invoke("flush_dns_cache");
    notify.success("DNS Flush", "Cache DNS vide avec succes");
    saveDnsHistory({
      timestamp: new Date().toLocaleString("fr-FR"),
      action: "Flush cache DNS",
      adapter: selectedAdapter.value || "—",
      dns: "—",
    });
  } catch (e: unknown) {
    notify.error("Erreur flush", String(e));
  } finally { flushing.value = false; }
}

async function reload() {
  loading.value = true;
  try { adapters.value = await invoke<AdapterInfo[]>("get_network_adapters_for_dns"); }
  catch (e: unknown) { notify.error("Erreur chargement", String(e)); }
  finally { loading.value = false; }
}

function bestPreset(): string {
  const entries = Object.entries(latencies.value).filter(([,v]) => v >= 0);
  if (!entries.length) return "";
  return entries.sort((a,b) => a[1]-b[1])[0][0];
}
</script>

<template>
  <div class="dns-page">
    <div class="page-header">
      <div>
        <h1>DNS Switcher</h1>
        <p class="page-subtitle">Changez votre serveur DNS en 1 clic — testez la latence avant de choisir</p>
      </div>
      <div style="display:flex;gap:8px">
        <NButton variant="secondary" size="sm" :loading="testing" @click="testAllLatencies">
          <Activity :size="14" /> Tester latences
        </NButton>
        <NButton variant="secondary" size="sm" :loading="flushing" @click="flush">
          <Trash2 :size="14" /> Vider cache DNS
        </NButton>
        <NButton variant="ghost" size="sm" :loading="loading" @click="reload">
          <RefreshCw :size="14" />
        </NButton>
      </div>
    </div>

    <!-- Meilleur DNS banner -->
    <div v-if="bestPreset()" class="best-banner">
      <Activity :size="15" style="color:var(--success)" />
      <span>Meilleur DNS detecte : <strong>{{ presets.find(p=>p.id===bestPreset())?.name }}</strong>
        ({{ latencies[bestPreset()] }} ms)</span>
      <NButton variant="primary" size="sm" @click="selectedPreset = bestPreset(); showCustom = false">
        Selectionner
      </NButton>
    </div>

    <div class="dns-layout">
      <!-- Adaptateurs -->
      <NCard>
        <template #header>
          <div style="display:flex;align-items:center;gap:8px">
            <Globe :size="16" />
            <span>Adaptateur reseau</span>
          </div>
        </template>
        <div v-if="loading" style="display:flex;justify-content:center;padding:24px"><NSpinner /></div>
        <div v-else class="adapters-list">
          <button
            v-for="a in adapters" :key="a.name"
            class="adapter-btn" :class="{ selected: selectedAdapter === a.name }"
            @click="selectedAdapter = a.name"
          >
            <div class="adapter-info">
              <span class="adapter-name">{{ a.name }}</span>
              <span class="adapter-desc">{{ a.description }}</span>
            </div>
            <div class="adapter-dns">
              <span v-for="d in a.current_dns" :key="d" class="dns-tag">{{ d }}</span>
              <span v-if="!a.current_dns.length" class="dns-tag muted">Auto</span>
            </div>
          </button>
        </div>
        <div v-if="!loading && adapters.length === 0" class="empty-state">Aucun adaptateur actif detecte</div>
      </NCard>

      <!-- Presets -->
      <NCard>
        <template #header>
          <div style="display:flex;align-items:center;gap:8px;width:100%">
            <Zap :size="16" />
            <span>Serveur DNS</span>
            <button class="link-btn" style="margin-left:auto" @click="showCustom = !showCustom">
              {{ showCustom ? "← Presets" : "Personnalise →" }}
            </button>
          </div>
        </template>

        <div v-if="!showCustom" class="presets-grid">
          <button
            v-for="p in presets" :key="p.id"
            class="preset-card" :class="{ selected: selectedPreset === p.id, best: bestPreset() === p.id }"
            :style="selectedPreset === p.id ? `border-color:${presetColors[p.id] || '#f97316'}` : ''"
            @click="selectedPreset = p.id; showCustom = false"
          >
            <div class="preset-dot" :style="{ background: presetColors[p.id] || '#888' }"></div>
            <div class="preset-info">
              <div style="display:flex;align-items:center;gap:6px">
                <span class="preset-name">{{ p.name }}</span>
                <span v-if="bestPreset() === p.id" class="best-tag">Meilleur</span>
              </div>
              <span class="preset-desc">{{ p.description }}</span>
              <div v-if="p.primary" style="display:flex;gap:4px;margin-top:4px;align-items:center">
                <code class="dns-ip">{{ p.primary }}</code>
                <code v-if="p.secondary" class="dns-ip">{{ p.secondary }}</code>
                <NBadge v-if="latencies[p.id] !== undefined" :variant="latencyVariant(latencies[p.id])" size="sm">
                  {{ latencyLabel(latencies[p.id]) }}
                </NBadge>
                <NSpinner v-else-if="testing && p.primary" :size="10" />
              </div>
            </div>
          </button>
        </div>

        <div v-else class="custom-dns">
          <label class="field-label">DNS Primaire</label>
          <input v-model="customPrimary" class="dns-input" placeholder="Ex: 8.8.8.8" />
          <label class="field-label" style="margin-top:12px">DNS Secondaire (optionnel)</label>
          <input v-model="customSecondary" class="dns-input" placeholder="Ex: 8.8.4.4" />
        </div>

        <div style="margin-top:16px;border-top:1px solid var(--border);padding-top:16px">
          <NButton variant="primary" :loading="applying" :disabled="!selectedAdapter || (!selectedPreset && !showCustom)" @click="applyDns" style="width:100%">
            <Globe :size="14" /> Appliquer le DNS
          </NButton>
        </div>
      </NCard>
    </div>

    <!-- Test DNS Reel (nslookup) -->
    <NCard>
      <template #header>
        <div style="display:flex;align-items:center;gap:8px">
          <FlaskConical :size="16" />
          <span>Test DNS Reel (nslookup)</span>
        </div>
      </template>
      <div class="dns-test-zone">
        <div class="dns-test-inputs">
          <div class="dns-test-field">
            <label class="field-label">Domaine a resoudre</label>
            <input v-model="dnsTestHost" class="dns-input" placeholder="Ex: google.com, github.com" @keyup.enter="testDnsReal" />
          </div>
          <div class="dns-test-field">
            <label class="field-label">Serveur DNS (laisser vide = tester tous)</label>
            <input v-model="dnsTestServer" class="dns-input" placeholder="Ex: 8.8.8.8 — vide = tous les presets" @keyup.enter="testDnsReal" />
          </div>
          <NButton variant="primary" :loading="dnsRealTesting" @click="testDnsReal" style="align-self:flex-end">
            <FlaskConical :size="14" /> Tester
          </NButton>
        </div>

        <div v-if="dnsRealTesting" class="dns-loading">
          <NSpinner :size="14" />
          <span>Resolution DNS en cours...</span>
        </div>

        <div v-if="dnsTestResults.length" class="dns-results">
          <div
            v-for="(r, i) in dnsTestResults"
            :key="i"
            class="dns-result-row"
            :class="r.success ? 'dns-ok' : 'dns-fail'"
          >
            <div class="dns-result-header">
              <span class="dns-result-server">{{ r.server }}</span>
              <NBadge :variant="r.success ? 'success' : 'danger'" size="sm">
                {{ r.success ? "OK" : "Echec" }}
              </NBadge>
              <span class="dns-result-lat font-mono">{{ r.latency_ms }} ms</span>
            </div>
            <div v-if="r.success && r.records.length" class="dns-result-records">
              <code v-for="(rec, j) in r.records" :key="j" class="dns-record">{{ rec }}</code>
            </div>
            <div v-else-if="!r.success" class="dns-result-error">
              {{ r.error || "Aucun enregistrement retourne" }}
            </div>
          </div>
        </div>

        <!-- DoH notice -->
        <div class="doh-notice">
          <ShieldCheck :size="13" style="flex-shrink:0;color:var(--accent-primary)" />
          <span>
            <strong>DNS over HTTPS (DoH)</strong> — Cloudflare et NextDNS proposent des variantes DoH
            (<code>1.1.1.1</code> / <code>https://cloudflare-dns.com/dns-query</code>).
            DoH chiffre vos requetes DNS mais necessite une configuration navigateur ou OS specifique.
            Ce test utilise la resolution systeme standard (UDP/53).
          </span>
        </div>
      </div>
    </NCard>

    <!-- Historique DNS -->
    <NCard v-if="dnsHistory.length">
      <template #header>
        <div style="display:flex;align-items:center;gap:8px;width:100%">
          <History :size="16" />
          <span>Historique des changements DNS</span>
          <NBadge variant="neutral" style="margin-left:auto">{{ dnsHistory.length }}</NBadge>
          <button class="link-btn" style="margin-left:8px;font-size:11px;color:var(--danger)" @click="clearDnsHistory">
            Effacer
          </button>
        </div>
      </template>
      <div class="history-list">
        <div v-for="(entry, i) in dnsHistory" :key="i" class="history-entry">
          <span class="history-ts">{{ entry.timestamp }}</span>
          <NBadge variant="neutral" size="sm">{{ entry.action }}</NBadge>
          <span class="history-adapter font-mono">{{ entry.adapter }}</span>
          <span class="history-dns">{{ entry.dns }}</span>
        </div>
      </div>
    </NCard>

    <!-- Informations -->
    <NCard>
      <template #header><span>Informations</span></template>
      <p style="font-size:13px;color:var(--text-secondary);line-height:1.6">
        Le changement DNS necessite les droits <strong>Administrateur</strong>. Apres changement, videz le cache DNS
        pour que les nouvelles resolutions soient immediates. Pour revenir au DNS automatique, selectionnez
        <strong>Automatique (DHCP)</strong>. Utilisez <strong>Tester latences</strong> pour choisir le serveur le plus rapide.
      </p>
    </NCard>
  </div>
</template>

<style scoped>
.dns-page { display:flex; flex-direction:column; gap:16px; }
.page-header { display:flex; justify-content:space-between; align-items:flex-start; flex-wrap:wrap; gap:12px; }
.page-header h1 { font-size:24px; font-weight:700; }
.page-subtitle { color:var(--text-muted); font-size:13px; margin-top:2px; }

.best-banner {
  display:flex; align-items:center; gap:10px; flex-wrap:wrap;
  padding:10px 16px; background:rgba(34,197,94,.08); border:1px solid rgba(34,197,94,.25);
  border-radius:var(--radius-md); font-size:13px; color:var(--text-primary);
}
.best-banner span { flex:1; }

.dns-layout { display:grid; grid-template-columns:1fr 1fr; gap:16px; align-items:start; }
@media (max-width:900px) { .dns-layout { grid-template-columns:1fr; } }

.adapters-list { display:flex; flex-direction:column; gap:6px; }
.adapter-btn { display:flex; align-items:center; justify-content:space-between; gap:12px; padding:12px 14px; border-radius:var(--radius-md); border:1px solid var(--border); background:var(--bg-tertiary); cursor:pointer; text-align:left; width:100%; transition:all var(--transition-fast); }
.adapter-btn:hover { border-color:var(--border-hover); }
.adapter-btn.selected { border-color:var(--accent-primary); background:var(--accent-muted); }
.adapter-info { display:flex; flex-direction:column; gap:2px; }
.adapter-name { font-size:13px; font-weight:600; color:var(--text-primary); }
.adapter-desc { font-size:11px; color:var(--text-muted); }
.adapter-dns { display:flex; flex-wrap:wrap; gap:4px; }
.dns-tag { font-size:10px; font-family:monospace; background:var(--bg-secondary); border:1px solid var(--border); border-radius:4px; padding:1px 6px; color:var(--text-secondary); }
.dns-tag.muted { color:var(--text-muted); }

.presets-grid { display:flex; flex-direction:column; gap:6px; }
.preset-card { display:flex; align-items:center; gap:12px; padding:10px 12px; border:1px solid var(--border); border-radius:var(--radius-md); background:var(--bg-tertiary); cursor:pointer; text-align:left; width:100%; transition:all var(--transition-fast); }
.preset-card:hover { border-color:var(--border-hover); }
.preset-card.selected { background:var(--accent-muted); }
.preset-card.best { border-color:rgba(34,197,94,.4); }
.preset-dot { width:10px; height:10px; border-radius:50%; flex-shrink:0; }
.preset-info { display:flex; flex-direction:column; gap:2px; flex:1; min-width:0; }
.preset-name { font-size:13px; font-weight:600; color:var(--text-primary); }
.preset-desc { font-size:11px; color:var(--text-muted); }
.dns-ip { font-size:10px; background:var(--bg-secondary); border:1px solid var(--border); border-radius:4px; padding:1px 6px; color:var(--accent-primary); }
.best-tag { font-size:10px; font-weight:700; color:var(--success); }

.custom-dns { display:flex; flex-direction:column; gap:6px; }
.field-label { font-size:11px; color:var(--text-muted); font-weight:600; text-transform:uppercase; letter-spacing:.06em; }
.dns-input { padding:8px 12px; border:1px solid var(--border); border-radius:var(--radius-md); background:var(--bg-tertiary); color:var(--text-primary); font-family:monospace; font-size:13px; outline:none; width:100%; box-sizing:border-box; }
.dns-input:focus { border-color:var(--accent-primary); }

.empty-state { text-align:center; padding:24px; color:var(--text-muted); font-size:13px; }
.link-btn { background:none; border:none; color:var(--accent-primary); cursor:pointer; font-size:12px; }

/* DNS Real Test */
.dns-test-zone { display:flex; flex-direction:column; gap:14px; }

.dns-test-inputs {
  display: grid;
  grid-template-columns: 1fr 1fr auto;
  gap: 10px;
  align-items: flex-end;
}

@media (max-width:800px) {
  .dns-test-inputs { grid-template-columns: 1fr; }
}

.dns-test-field { display:flex; flex-direction:column; gap:5px; }

.dns-loading {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-muted);
}

.dns-results { display:flex; flex-direction:column; gap:8px; }

.dns-result-row {
  padding: 10px 14px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.dns-ok { background: rgba(34,197,94,.06); border-color: rgba(34,197,94,.2); }
.dns-fail { background: rgba(239,68,68,.06); border-color: rgba(239,68,68,.2); }

.dns-result-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.dns-result-server {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  flex: 1;
}

.dns-result-lat {
  font-size: 12px;
  color: var(--text-muted);
}

.dns-result-records {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.dns-record {
  font-size: 11px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 2px 8px;
  color: var(--accent-primary);
  font-family: "JetBrains Mono", monospace;
}

.dns-result-error {
  font-size: 12px;
  color: var(--danger);
  font-family: "JetBrains Mono", monospace;
}

/* DoH Notice */
.doh-notice {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px 14px;
  background: var(--accent-muted);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.6;
}

.doh-notice code {
  font-family: "JetBrains Mono", monospace;
  font-size: 11px;
  background: var(--bg-secondary);
  padding: 0 4px;
  border-radius: 3px;
  color: var(--accent-primary);
}

/* History */
.history-list { display:flex; flex-direction:column; gap:5px; }

.history-entry {
  display: grid;
  grid-template-columns: 140px auto 1fr 1fr;
  align-items: center;
  gap: 10px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  transition: background var(--transition-fast);
}

.history-entry:hover { background: var(--bg-tertiary); }

.history-ts { font-size:11px; color:var(--text-muted); }
.history-adapter { font-size:11px; color:var(--text-secondary); }
.history-dns { font-size:12px; color:var(--text-primary); }

.font-mono { font-family:"JetBrains Mono",monospace; font-size:12px; }
</style>
