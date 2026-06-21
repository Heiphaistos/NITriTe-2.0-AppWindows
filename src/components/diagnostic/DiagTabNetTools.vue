<script setup lang="ts">
import { ref, onMounted } from "vue";
import NBadge from "@/components/ui/NBadge.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NButton from "@/components/ui/NButton.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import { Globe, Wifi, Search, Server, Activity } from "lucide-vue-next";
import { useExportData } from '@/composables/useExportData';
import { invoke, invokeRaw } from "@/utils/invoke";
const { exportCSV, exportTXT } = useExportData();

function exportPingResult(result: PingResult | null) {
  if (!result) return;
  exportTXT([
    `=== Résultat Ping ===`,
    `Hôte: ${result.host}`,
    `Min: ${result.min_ms}ms | Moy: ${result.avg_ms}ms | Max: ${result.max_ms}ms`,
    `Paquets: ${result.packets_sent} envoyés, ${result.packets_received} reçus`,
    `Perte: ${result.loss_percent}%`,
    `Date: ${new Date().toLocaleString('fr-FR')}`,
  ], 'ping-result');
}

function exportTracertResult(hops: TracertHop[]) {
  if (!hops?.length) return;
  exportCSV(hops.map(h => ({
    Saut: h.hop, Adresse: h.address || '*', Latence: h.ms > 0 ? h.ms + 'ms' : 'timeout',
  })), 'traceroute-result');
}

function exportPortScanResult(ports: PortScanResult[]) {
  if (!ports?.length) return;
  exportCSV(ports.map(p => ({
    Port: p.port, Etat: p.open ? 'Ouvert' : 'Fermé', Service: p.service || '',
  })), 'portscan-result');
}

function exportWifiNetworksList(networks: WifiNetwork[]) {
  if (!networks?.length) return;
  exportCSV(networks.map(n => ({
    SSID: n.ssid, BSSID: n.bssid, Signal: n.signal_percent + '%',
    Bande: n.band, Canal: n.channel, Securite: n.auth,
  })), 'wifi-networks');
}

interface PingResult { host: string; success: boolean; avg_ms: number; min_ms: number; max_ms: number; packets_sent: number; packets_received: number; loss_percent: number; }
interface TracertHop { hop: number; address: string; ms: number; }
interface DnsResult { host: string; records: string[]; query_type: string; success: boolean; }
interface IpConfigAdapter { name: string; ipv4: string; ipv6: string; prefix_len: number; gateway: string; dns_servers: string[]; mac: string; dhcp_enabled: boolean; }
interface ArpEntry { ip: string; mac: string; entry_type: string; interface: string; }
interface RouteEntry { network: string; netmask: string; gateway: string; interface: string; metric: number; }
interface PortScanResult { port: number; open: boolean; service: string; }
interface WifiNetwork { ssid: string; bssid: string; signal_percent: number; channel: string; auth: string; encryption: string; band: string; }
interface OpenPort { protocol: string; local_address: string; local_port: number; remote_address: string; state: string; pid: number; process: string; }
interface HttpCheckResult { url: string; status_code: number; status_text: string; headers: string[]; time_ms: number; success: boolean; }
interface NetShareEntry { name: string; path: string; comment: string; host: string; }
interface BandwidthResult { download_mbps: number; latency_ms: number; test_host: string; success: boolean; }

// ─── State ─────────────────────────────────────────────────────────────────────
const pingHost = ref("8.8.8.8"); const pingCount = ref(4); const pingResult = ref<PingResult|null>(null); const pingLoading = ref(false);
const tracertHost = ref("google.com"); const tracertHops = ref<TracertHop[]>([]); const tracertLoading = ref(false);
const nsHost = ref("google.com"); const nsType = ref("A"); const nsResult = ref<DnsResult|null>(null); const nsLoading = ref(false);
const ipConfig = ref<IpConfigAdapter[]>([]); const ipLoading = ref(true);
const arpTable = ref<ArpEntry[]>([]); const arpLoading = ref(false);
const routeTable = ref<RouteEntry[]>([]); const routeLoading = ref(false); const routeFilter = ref("");
const scanHost = ref("localhost"); const scanPorts = ref("21,22,23,25,53,80,110,135,139,143,443,445,1433,3306,3389,5900,8080"); const scanResults = ref<PortScanResult[]>([]); const scanLoading = ref(false);
const wifiNetworks = ref<WifiNetwork[]>([]); const wifiLoading = ref(false);
const openPorts = ref<OpenPort[]>([]); const openPortsLoading = ref(false); const openPortFilter = ref("");
const httpUrl = ref("https://google.com"); const httpResult = ref<HttpCheckResult|null>(null); const httpLoading = ref(false);
const sharesHost = ref("localhost"); const shares = ref<NetShareEntry[]>([]); const sharesLoading = ref(false);
const bandwidth = ref<BandwidthResult|null>(null); const bwLoading = ref(false);

onMounted(async () => {
  try { ipConfig.value = await invoke<IpConfigAdapter[]>("get_ip_config"); } catch {}
  finally { ipLoading.value = false; }
});

async function inv<T>(cmd: string, args?: any): Promise<T|null> {
  try { return await invoke<T>(cmd, args); } catch { return null; }
}

async function doPing() { pingLoading.value=true; pingResult.value=null; pingResult.value=await inv("run_ping",{host:pingHost.value,count:pingCount.value}); pingLoading.value=false; }
async function doTracert() {
  tracertLoading.value=true;
  tracertHops.value=[];
  try {
    tracertHops.value = await invokeRaw<TracertHop[]>("run_traceroute", { host: tracertHost.value }) ?? [];
  } catch {
    tracertHops.value = [];
  }
  tracertLoading.value=false;
}
async function doNslookup() { nsLoading.value=true; nsResult.value=await inv("run_nslookup",{host:nsHost.value,recordType:nsType.value}); nsLoading.value=false; }
async function doArp() { arpLoading.value=true; arpTable.value=await inv("get_arp_table")??[]; arpLoading.value=false; }
async function doRoute() { routeLoading.value=true; routeTable.value=await inv("get_route_table")??[]; routeLoading.value=false; }
async function doPortScan() {
  scanLoading.value=true; scanResults.value=[];
  const ports = scanPorts.value.split(/[,\s]+/).map(p=>parseInt(p.trim())).filter(n=>!isNaN(n)&&n>0&&n<65536);
  scanResults.value=await inv("scan_ports",{host:scanHost.value,ports})??[];
  scanLoading.value=false;
}
async function doWifi() { wifiLoading.value=true; wifiNetworks.value=await inv("get_wifi_networks")??[]; wifiLoading.value=false; }
async function doOpenPorts() { openPortsLoading.value=true; openPorts.value=await inv("get_local_open_ports")??[]; openPortsLoading.value=false; }
async function doHttp() { httpLoading.value=true; httpResult.value=await inv("check_http",{url:httpUrl.value}); httpLoading.value=false; }
async function doShares() { sharesLoading.value=true; shares.value=await inv("get_net_shares",{host:sharesHost.value})??[]; sharesLoading.value=false; }
async function doBandwidth() { bwLoading.value=true; bandwidth.value=await inv("test_bandwidth"); bwLoading.value=false; }

function sigColor(s: number) { return s>=70?'var(--success)':s>=40?'var(--warning)':'var(--error)'; }

const filteredRoute = () => routeTable.value.filter(r=>!routeFilter.value||r.network.includes(routeFilter.value)||r.gateway.includes(routeFilter.value)||r.interface.toLowerCase().includes(routeFilter.value.toLowerCase()));
const filteredOpenPorts = () => openPorts.value.filter(p=>!openPortFilter.value||String(p.local_port).includes(openPortFilter.value)||p.process.toLowerCase().includes(openPortFilter.value.toLowerCase()));
</script>

<template>
  <div class="diag-tab-content">
    <DiagBanner :icon="Globe" title="Outils Réseau" desc="Ping, traceroute, nslookup et diagnostics connectivité" color="cyan" />

    <div style="display:flex;flex-direction:column;gap:14px">

      <!-- IP Config -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0"><Wifi :size="13" style="display:inline;margin-right:4px" />Configuration IP</p>
        <div v-if="ipLoading" class="diag-loading"><div class="diag-spinner"></div> Chargement...</div>
        <div v-for="(a, i) in ipConfig" :key="i" style="padding:8px 0;border-bottom:1px solid var(--border)">
          <div style="font-weight:600;font-size:12px;margin-bottom:4px">{{ a.name }}</div>
          <div class="info-grid">
            <div class="info-row"><span>IPv4</span><code>{{ a.ipv4||'—' }}{{ a.prefix_len?'/'+a.prefix_len:'' }}</code></div>
            <div class="info-row" v-if="a.ipv6"><span>IPv6</span><code style="font-size:10px">{{ a.ipv6 }}</code></div>
            <div class="info-row"><span>Passerelle</span><code>{{ a.gateway||'—' }}</code></div>
            <div class="info-row"><span>DNS</span><code>{{ a.dns_servers.join(', ')||'—' }}</code></div>
            <div class="info-row"><span>MAC</span><code>{{ a.mac||'—' }}</code></div>
            <div class="info-row"><span>DHCP</span><NBadge :variant="a.dhcp_enabled?'success':'neutral'" style="font-size:9px">{{ a.dhcp_enabled?'Activé':'Statique' }}</NBadge></div>
          </div>
        </div>
      </div>

      <!-- WiFi réseaux proches -->
      <div class="diag-section">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px">
          <p class="diag-section-label" style="margin:0"><Wifi :size="13" style="display:inline;margin-right:4px" />Réseaux WiFi proches</p>
          <button @click="doWifi" :disabled="wifiLoading" style="padding:4px 12px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:11px;cursor:pointer;display:flex;align-items:center;gap:4px">
            <NSpinner v-if="wifiLoading" :size="11" />Actualiser
          </button>
        </div>
        <div v-if="wifiLoading" class="diag-loading"><div class="diag-spinner"></div> Scan WiFi...</div>
        <div v-else-if="wifiNetworks.length">
          <div v-for="(n, i) in wifiNetworks" :key="i" style="display:flex;align-items:center;gap:10px;padding:5px 0;border-bottom:1px solid var(--border)">
            <div style="width:38px;text-align:center;font-size:10px;font-weight:700" :style="{color:sigColor(n.signal_percent)}">{{ n.signal_percent }}%</div>
            <div style="flex:1;min-width:0">
              <div style="font-size:12px;font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">{{ n.ssid||'(caché)' }}</div>
              <div style="font-size:10px;color:var(--text-secondary)">{{ n.bssid }} — Ch.{{ n.channel }} — {{ n.band }}</div>
            </div>
            <NBadge variant="neutral" style="font-size:9px">{{ n.auth }}</NBadge>
          </div>
          <NButton variant="ghost" size="sm" @click="exportWifiNetworksList(wifiNetworks)" style="margin-top:6px">↓ Export CSV</NButton>
        </div>
        <p v-else style="font-size:12px;color:var(--text-secondary)">Cliquez Actualiser pour scanner les réseaux.</p>
      </div>

      <!-- Ping -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0"><Search :size="13" style="display:inline;margin-right:4px" />Ping</p>
        <div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-bottom:8px">
          <input v-model="pingHost" placeholder="Hôte ou IP" style="flex:1;min-width:130px;padding:5px 10px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-size:12px" @keyup.enter="doPing" />
          <select v-model="pingCount" style="padding:5px 8px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-size:12px">
            <option :value="4">4</option><option :value="8">8</option><option :value="10">10</option>
          </select>
          <button @click="doPing" :disabled="pingLoading" style="padding:5px 14px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:12px;cursor:pointer;display:flex;align-items:center;gap:4px">
            <NSpinner v-if="pingLoading" :size="11" />Ping
          </button>
        </div>
        <div v-if="pingResult" style="padding:8px;background:var(--bg-secondary);border-radius:6px">
          <div style="display:flex;align-items:center;gap:8px;margin-bottom:6px">
            <NBadge :variant="pingResult.success?'success':'danger'">{{ pingResult.success?'Joignable':'Injoignable' }}</NBadge>
            <code style="font-size:12px">{{ pingResult.host }}</code>
          </div>
          <div v-if="pingResult.success" class="info-grid">
            <div class="info-row"><span>Latence moy.</span><strong style="color:var(--accent)">{{ pingResult.avg_ms }} ms</strong></div>
            <div class="info-row"><span>Min / Max</span><span>{{ pingResult.min_ms }} / {{ pingResult.max_ms }} ms</span></div>
            <div class="info-row"><span>Paquets</span><span>{{ pingResult.packets_received }}/{{ pingResult.packets_sent }} ({{ pingResult.loss_percent }}% perte)</span></div>
          </div>
          <NButton variant="ghost" size="sm" @click="exportPingResult(pingResult)" style="margin-top:6px">↓ Exporter TXT</NButton>
        </div>
      </div>

      <!-- Traceroute -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Traceroute</p>
        <div style="display:flex;gap:8px;align-items:center;margin-bottom:8px">
          <input v-model="tracertHost" placeholder="Hôte cible" style="flex:1;padding:5px 10px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-size:12px" @keyup.enter="doTracert" />
          <button @click="doTracert" :disabled="tracertLoading" style="padding:5px 14px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:12px;cursor:pointer;display:flex;align-items:center;gap:4px">
            <NSpinner v-if="tracertLoading" :size="11" />Tracer
          </button>
        </div>
        <div v-if="tracertLoading" class="diag-loading"><div class="diag-spinner"></div> Traceroute... (30-60s)</div>
        <div v-else-if="tracertHops.length" style="overflow-x:auto">
          <table style="width:100%;border-collapse:collapse;font-size:12px">
            <thead><tr style="background:var(--bg-secondary)"><th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">#</th><th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">Adresse</th><th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">ms</th></tr></thead>
            <tbody>
              <tr v-for="h in tracertHops" :key="h.hop" style="border-bottom:1px solid var(--border)">
                <td style="padding:3px 8px;color:var(--text-secondary)">{{ h.hop }}</td>
                <td style="padding:3px 8px"><code>{{ h.address||'*' }}</code></td>
                <td style="padding:3px 8px;color:var(--text-secondary)">{{ h.ms||'—' }}</td>
              </tr>
            </tbody>
          </table>
          <NButton variant="ghost" size="sm" @click="exportTracertResult(tracertHops)" style="margin-top:6px">↓ Export CSV</NButton>
        </div>
        <div v-else style="font-size:12px;color:var(--text-secondary)">
          Aucun saut détecté. Vérifiez la connectivité ou essayez une adresse IP directement.
        </div>
      </div>

      <!-- Scan de ports -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0"><Activity :size="13" style="display:inline;margin-right:4px" />Scan de ports TCP</p>
        <div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-bottom:6px">
          <input v-model="scanHost" placeholder="Hôte cible" style="width:140px;padding:5px 10px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-size:12px" />
          <input v-model="scanPorts" placeholder="Ports (ex: 80,443,3389)" style="flex:1;min-width:200px;padding:5px 10px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-size:12px" />
          <button @click="doPortScan" :disabled="scanLoading" style="padding:5px 14px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:12px;cursor:pointer;display:flex;align-items:center;gap:4px">
            <NSpinner v-if="scanLoading" :size="11" />Scanner
          </button>
        </div>
        <div v-if="scanLoading" class="diag-loading"><div class="diag-spinner"></div> Scan en cours...</div>
        <div v-else-if="scanResults.length">
          <div style="display:flex;flex-wrap:wrap;gap:6px;margin-top:4px">
            <div v-for="r in scanResults" :key="r.port" style="display:flex;align-items:center;gap:4px;padding:3px 8px;border-radius:5px;font-size:11px" :style="{background:r.open?'rgba(74,222,128,0.1)':'rgba(100,116,139,0.1)',border:r.open?'1px solid rgba(74,222,128,0.3)':'1px solid var(--border)'}">
              <span :style="{color:r.open?'var(--success)':'var(--text-muted)'}">{{ r.open?'●':'○' }}</span>
              <strong :style="{color:r.open?'var(--text-primary)':'var(--text-muted)'}">{{ r.port }}</strong>
              <span v-if="r.service" style="color:var(--accent);font-size:9px">{{ r.service }}</span>
            </div>
          </div>
          <NButton variant="ghost" size="sm" @click="exportPortScanResult(scanResults)" style="margin-top:6px">↓ Export CSV</NButton>
        </div>
      </div>

      <!-- Ports ouverts locaux -->
      <div class="diag-section">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px">
          <p class="diag-section-label" style="margin:0"><Server :size="13" style="display:inline;margin-right:4px" />Ports en écoute (local)</p>
          <button @click="doOpenPorts" :disabled="openPortsLoading" style="padding:4px 12px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:11px;cursor:pointer;display:flex;align-items:center;gap:4px">
            <NSpinner v-if="openPortsLoading" :size="11" />Actualiser
          </button>
        </div>
        <input v-if="openPorts.length" v-model="openPortFilter" placeholder="Filtrer port/process..." style="width:100%;margin-bottom:6px;padding:4px 10px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-size:11px" />
        <div v-if="openPortsLoading" class="diag-loading"><div class="diag-spinner"></div></div>
        <div v-else-if="openPorts.length" style="overflow-x:auto;max-height:200px;overflow-y:auto">
          <table style="width:100%;border-collapse:collapse;font-size:11px">
            <thead><tr style="background:var(--bg-secondary);position:sticky;top:0">
              <th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">Port</th>
              <th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">Adresse</th>
              <th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">PID</th>
              <th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">Processus</th>
            </tr></thead>
            <tbody>
              <tr v-for="(p, i) in filteredOpenPorts()" :key="i" style="border-bottom:1px solid var(--border)">
                <td style="padding:3px 8px"><code style="color:var(--accent)">{{ p.local_port }}</code></td>
                <td style="padding:3px 8px;color:var(--text-secondary)">{{ p.local_address }}</td>
                <td style="padding:3px 8px;color:var(--text-secondary)">{{ p.pid }}</td>
                <td style="padding:3px 8px">{{ p.process }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <p v-else style="font-size:12px;color:var(--text-secondary)">Cliquez Actualiser pour charger.</p>
      </div>

      <!-- Table ARP -->
      <div class="diag-section">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px">
          <p class="diag-section-label" style="margin:0">Table ARP</p>
          <button @click="doArp" :disabled="arpLoading" style="padding:4px 12px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:11px;cursor:pointer;display:flex;align-items:center;gap:4px">
            <NSpinner v-if="arpLoading" :size="11" />Charger
          </button>
        </div>
        <div v-if="arpTable.length" style="overflow-x:auto;max-height:180px;overflow-y:auto">
          <table style="width:100%;border-collapse:collapse;font-size:11px">
            <thead><tr style="background:var(--bg-secondary)"><th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">IP</th><th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">MAC</th><th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">Type</th><th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">Interface</th></tr></thead>
            <tbody>
              <tr v-for="(e, i) in arpTable" :key="i" style="border-bottom:1px solid var(--border)">
                <td style="padding:3px 8px"><code>{{ e.ip }}</code></td>
                <td style="padding:3px 8px;color:var(--text-secondary)"><code>{{ e.mac }}</code></td>
                <td style="padding:3px 8px;color:var(--text-secondary)">{{ e.entry_type }}</td>
                <td style="padding:3px 8px;font-size:10px;color:var(--text-secondary)">{{ e.interface }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Table de routage -->
      <div class="diag-section">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px">
          <p class="diag-section-label" style="margin:0">Table de routage</p>
          <button @click="doRoute" :disabled="routeLoading" style="padding:4px 12px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:11px;cursor:pointer;display:flex;align-items:center;gap:4px">
            <NSpinner v-if="routeLoading" :size="11" />Charger
          </button>
        </div>
        <input v-if="routeTable.length" v-model="routeFilter" placeholder="Filtrer réseau/gateway/interface..." style="width:100%;margin-bottom:6px;padding:4px 10px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-size:11px" />
        <div v-if="routeTable.length" style="overflow-x:auto;max-height:200px;overflow-y:auto">
          <table style="width:100%;border-collapse:collapse;font-size:11px">
            <thead><tr style="background:var(--bg-secondary);position:sticky;top:0"><th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">Réseau</th><th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">Masque</th><th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">Passerelle</th><th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">Interface</th><th style="padding:4px 8px;text-align:left;color:var(--text-secondary)">Métrique</th></tr></thead>
            <tbody>
              <tr v-for="(r, i) in filteredRoute()" :key="i" style="border-bottom:1px solid var(--border)">
                <td style="padding:3px 8px"><code style="font-size:10px">{{ r.network }}</code></td>
                <td style="padding:3px 8px;color:var(--text-secondary)">{{ r.netmask }}</td>
                <td style="padding:3px 8px"><code style="font-size:10px">{{ r.gateway||'—' }}</code></td>
                <td style="padding:3px 8px;font-size:10px;color:var(--text-secondary);max-width:160px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ r.interface }}</td>
                <td style="padding:3px 8px;color:var(--text-secondary)">{{ r.metric }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- DNS Lookup -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0"><Server :size="13" style="display:inline;margin-right:4px" />DNS Lookup</p>
        <div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-bottom:8px">
          <input v-model="nsHost" placeholder="Domaine" style="flex:1;min-width:140px;padding:5px 10px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-size:12px" @keyup.enter="doNslookup" />
          <select v-model="nsType" style="padding:5px 8px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-size:12px">
            <option>A</option><option>AAAA</option><option>MX</option><option>NS</option><option>TXT</option><option>CNAME</option><option>SOA</option><option>SRV</option>
          </select>
          <button @click="doNslookup" :disabled="nsLoading" style="padding:5px 14px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:12px;cursor:pointer;display:flex;align-items:center;gap:4px">
            <NSpinner v-if="nsLoading" :size="11" />Résoudre
          </button>
        </div>
        <div v-if="nsResult" style="padding:8px;background:var(--bg-secondary);border-radius:6px">
          <div style="display:flex;align-items:center;gap:8px;margin-bottom:6px">
            <NBadge :variant="nsResult.success?'success':'danger'">{{ nsResult.success?'Résolu':'Échec' }}</NBadge>
            <code style="font-size:12px">{{ nsResult.host }}</code>
            <NBadge variant="neutral" style="font-size:9px">{{ nsResult.query_type }}</NBadge>
          </div>
          <div v-for="(r, i) in nsResult.records" :key="i" style="padding:2px 0"><code style="font-size:12px;color:var(--accent)">{{ r }}</code></div>
          <p v-if="!nsResult.records.length&&nsResult.success" style="font-size:12px;color:var(--text-secondary)">Aucun enregistrement.</p>
        </div>
      </div>

      <!-- Vérification HTTP -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Vérification HTTP/HTTPS</p>
        <div style="display:flex;gap:8px;align-items:center;margin-bottom:8px">
          <input v-model="httpUrl" placeholder="https://example.com" style="flex:1;padding:5px 10px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-size:12px" @keyup.enter="doHttp" />
          <button @click="doHttp" :disabled="httpLoading" style="padding:5px 14px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:12px;cursor:pointer;display:flex;align-items:center;gap:4px">
            <NSpinner v-if="httpLoading" :size="11" />Vérifier
          </button>
        </div>
        <div v-if="httpResult" style="padding:8px;background:var(--bg-secondary);border-radius:6px">
          <div style="display:flex;align-items:center;gap:8px;margin-bottom:6px">
            <NBadge :variant="httpResult.success?'success':'danger'">{{ httpResult.status_code||0 }} {{ httpResult.status_text }}</NBadge>
            <span style="font-size:12px;color:var(--text-secondary)">{{ httpResult.time_ms }} ms</span>
          </div>
          <div v-if="httpResult.headers.length" style="max-height:120px;overflow-y:auto">
            <div v-for="(h, i) in httpResult.headers" :key="i" style="font-size:10px;color:var(--text-secondary);padding:1px 0;font-family:monospace">{{ h }}</div>
          </div>
        </div>
      </div>

      <!-- Partages réseau -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Partages réseau (SMB)</p>
        <div style="display:flex;gap:8px;align-items:center;margin-bottom:8px">
          <input v-model="sharesHost" placeholder="Hôte (localhost ou IP)" style="flex:1;padding:5px 10px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;color:var(--text-primary);font-size:12px" @keyup.enter="doShares" />
          <button @click="doShares" :disabled="sharesLoading" style="padding:5px 14px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:12px;cursor:pointer;display:flex;align-items:center;gap:4px">
            <NSpinner v-if="sharesLoading" :size="11" />Lister
          </button>
        </div>
        <div v-if="shares.length">
          <div v-for="(s, i) in shares" :key="i" style="display:flex;gap:12px;padding:5px 0;border-bottom:1px solid var(--border);font-size:12px">
            <code style="color:var(--accent);min-width:100px">{{ s.name }}</code>
            <span style="color:var(--text-secondary)">{{ s.path }}</span>
            <span v-if="s.comment" style="color:var(--text-secondary);font-size:10px">{{ s.comment }}</span>
          </div>
        </div>
      </div>

      <!-- Test de débit -->
      <div class="diag-section">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px">
          <p class="diag-section-label" style="margin:0">Test de débit internet</p>
          <button @click="doBandwidth" :disabled="bwLoading" style="padding:4px 12px;background:var(--accent);color:white;border:none;border-radius:6px;font-size:11px;cursor:pointer;display:flex;align-items:center;gap:4px">
            <NSpinner v-if="bwLoading" :size="11" />Tester (50MB)
          </button>
        </div>
        <div v-if="bwLoading" class="diag-loading"><div class="diag-spinner"></div> Téléchargement 5MB depuis Cloudflare...</div>
        <div v-else-if="bandwidth" style="display:flex;gap:20px;align-items:center">
          <div style="text-align:center">
            <div style="font-size:28px;font-weight:700;color:var(--accent)">{{ bandwidth.download_mbps }}</div>
            <div style="font-size:12px;color:var(--text-secondary)">Mbps ↓</div>
          </div>
          <div style="text-align:center">
            <div style="font-size:22px;font-weight:600;color:var(--text-secondary)">{{ bandwidth.latency_ms }}</div>
            <div style="font-size:12px;color:var(--text-secondary)">ms latence</div>
          </div>
          <div style="font-size:10px;color:var(--text-secondary)">via {{ bandwidth.test_host }}</div>
          <NBadge :variant="bandwidth.success?'success':'danger'" style="font-size:9px">{{ bandwidth.success?'OK':'Échec' }}</NBadge>
        </div>
      </div>

    </div>
  </div>
</template>
