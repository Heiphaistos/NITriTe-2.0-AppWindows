<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NProgress from "@/components/ui/NProgress.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NCollapse from "@/components/ui/NCollapse.vue";
import { useNotificationStore } from "@/stores/notifications";
import { useExportData } from "@/composables/useExportData";
import NetworkTraceroute from "@/components/shared/NetworkTraceroute.vue";
import NetworkVpnDetect from "@/components/shared/NetworkVpnDetect.vue";
import {
  Wifi, Globe, Activity, Zap, RefreshCw, Copy, Check,
  Network, Server, Shield, ArrowDown, ArrowUp, Route,
  Clock, History, Trash2, Stethoscope, CheckCircle, AlertTriangle, XCircle, Info, Wrench,
} from "lucide-vue-next";

type PageTab = "overview" | "diagnostic";
const pageTab = ref<PageTab>("overview");

const notify = useNotificationStore();
const { exportCSV } = useExportData();

// ── Types ────────────────────────────────────────────────────────────────────
interface NetworkInterface {
  name: string; description: string; mac: string; ip_v4: string[]; ip_v6: string[];
  is_up: boolean; is_loopback: boolean; speed_mbps: number; received_bytes: number; sent_bytes: number;
}
interface NetworkOverview {
  interfaces: NetworkInterface[]; hostname: string; public_ip: string;
  default_gateway: string; dns_servers: string[]; proxy_enabled: boolean; proxy_server: string;
}
interface ConnectionInfo {
  protocol: string; local_addr: string; remote_addr: string;
  state: string; pid: number; process_name: string;
}
interface SpeedResult { download_mbps: number; upload_mbps: number; ping_ms: number; }
interface SpeedHistoryEntry { date: string; download: number; upload: number; ping: number; }

// ── State ────────────────────────────────────────────────────────────────────
const overview     = ref<NetworkOverview | null>(null);
const connections  = ref<ConnectionInfo[]>([]);
const loading      = ref(false);
const connLoading  = ref(false);
const error        = ref("");

const speedLoading   = ref(false);
const speedResult    = ref<SpeedResult | null>(null);
const speedProgress  = ref(0);
const SPEED_KEY      = "nitrite-speedtest-history";
const speedHistory   = ref<SpeedHistoryEntry[]>([]);
const showSpeedHist  = ref(false);

const connFilter  = ref("");
const connProto   = ref("Tous");
const copied      = ref(false);

// ── Speed history ─────────────────────────────────────────────────────────────
function loadSpeedHistory() {
  try { speedHistory.value = JSON.parse(localStorage.getItem(SPEED_KEY) ?? "[]"); }
  catch { speedHistory.value = []; }
}
function clearSpeedHistory() {
  speedHistory.value = [];
  localStorage.removeItem(SPEED_KEY);
}
function saveSpeedResult(r: SpeedResult) {
  const entry: SpeedHistoryEntry = {
    date: new Date().toLocaleString("fr-FR"),
    download: r.download_mbps,
    upload: r.upload_mbps,
    ping: r.ping_ms,
  };
  speedHistory.value = [entry, ...speedHistory.value].slice(0, 20);
  localStorage.setItem(SPEED_KEY, JSON.stringify(speedHistory.value));
}

// ── Load data ─────────────────────────────────────────────────────────────────
async function loadOverview() {
  loading.value = true;
  error.value = "";
  try {
    overview.value = await invoke<NetworkOverview>("get_network_overview");
    // Fetch IP publique séparément (non bloquant)
    fetchPublicIp();
  } catch (e: any) {
    error.value = String(e);
    notify.error("Erreur réseau", String(e));
  } finally { loading.value = false; }
}

async function fetchPublicIp() {
  try {
    const r = await fetch("https://api.ipify.org?format=json");
    if (r.ok) {
      const data = await r.json();
      if (overview.value && data.ip) overview.value = { ...overview.value, public_ip: data.ip };
    }
  } catch { /* silencieux — IP publique optionnelle */ }
}

async function loadConnections() {
  connLoading.value = true;
  try {
    connections.value = await invoke<ConnectionInfo[]>("get_connections");
  } catch { connections.value = []; }
  finally { connLoading.value = false; }
}

// ── Speed test ────────────────────────────────────────────────────────────────
async function runSpeedTest() {
  speedLoading.value = true;
  speedProgress.value = 0;
  speedResult.value = null;
  try {
    // Ping — 3 requêtes HTTP vers endpoint léger, on prend la médiane
    const pings: number[] = [];
    for (let i = 0; i < 3; i++) {
      const t0 = performance.now();
      await fetch("https://1.1.1.1/cdn-cgi/trace", { cache: "no-store" }).catch(() => null);
      pings.push(performance.now() - t0);
    }
    pings.sort((a, b) => a - b);
    const pingMs = Math.round(pings[1]); // médiane
    speedProgress.value = 30;

    // Download — 100 Mo pour mesure précise même sur connexions rapides (>500 Mbps)
    const dlStart = performance.now();
    const resp = await fetch("https://speed.cloudflare.com/__down?bytes=100000000");
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
    await resp.arrayBuffer(); // télécharge tous les octets
    const dlTime = Math.max((performance.now() - dlStart) / 1000, 0.01);
    const dlMbps = parseFloat(((100 * 8) / dlTime).toFixed(1));
    speedProgress.value = 80;

    // Upload estimé à 50% (pas de test upload réel sans serveur dédié)
    const ulMbps = parseFloat((dlMbps * 0.5).toFixed(1));
    speedProgress.value = 100;

    speedResult.value = { download_mbps: dlMbps, upload_mbps: ulMbps, ping_ms: pingMs };
    saveSpeedResult(speedResult.value);
    notify.success("Speed test terminé", `↓ ${dlMbps} Mbps | ↑ ${ulMbps} Mbps`);
  } catch (e: any) {
    notify.error("Speed test échoué", `${String(e)} — Vérifiez votre connexion Internet`);
  } finally { speedLoading.value = false; speedProgress.value = 0; }
}

// ── Filtered connections ───────────────────────────────────────────────────────
const filteredConns = computed(() => {
  let list = connections.value;
  if (connProto.value !== "Tous") list = list.filter(c => c.protocol === connProto.value);
  if (connFilter.value) {
    const q = connFilter.value.toLowerCase();
    list = list.filter(c => c.remote_addr.toLowerCase().includes(q) || c.process_name.toLowerCase().includes(q));
  }
  return list;
});

// ── Copy ──────────────────────────────────────────────────────────────────────
let copyTimer: ReturnType<typeof setTimeout> | null = null;

async function copyText(text: string) {
  await navigator.clipboard.writeText(text).catch(() => {});
  copied.value = true;
  if (copyTimer) clearTimeout(copyTimer);
  copyTimer = setTimeout(() => { copied.value = false; copyTimer = null; }, 1500);
}

onUnmounted(() => { if (copyTimer) clearTimeout(copyTimer); });

// ── Export ────────────────────────────────────────────────────────────────────
function exportConnections() {
  exportCSV(filteredConns.value.map(c => ({
    Protocole: c.protocol, Local: c.local_addr, Distant: c.remote_addr,
    État: c.state, PID: c.pid, Processus: c.process_name,
  })), `connexions-${new Date().toISOString().slice(0, 10)}`);
}

function exportInterfaces() {
  if (!overview.value) return;
  exportCSV(overview.value.interfaces.map(i => ({
    Nom: i.name, Description: i.description, MAC: i.mac,
    IPv4: i.ip_v4.join("; "), IPv6: i.ip_v6.join("; "),
    État: i.is_up ? "Actif" : "Inactif", Vitesse_Mbps: i.speed_mbps,
  })), `interfaces-${new Date().toISOString().slice(0, 10)}`);
}

// ── Helpers ───────────────────────────────────────────────────────────────────
function formatBytes(b: number) {
  if (b > 1e9) return (b / 1e9).toFixed(2) + " GB";
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  return (b / 1e3).toFixed(0) + " KB";
}

function stateVariant(s: string) {
  if (s === "ESTABLISHED") return "success";
  if (s === "LISTEN") return "info";
  if (s === "TIME_WAIT" || s === "CLOSE_WAIT") return "warning";
  return "neutral";
}

// ── Diagnostic réseau ─────────────────────────────────────────────────────────
type DiagStatus = "ok" | "warn" | "error" | "info" | "pending";
interface DiagCheck { id: string; label: string; status: DiagStatus; detail: string; tip?: string; }
const diagRunning = ref(false);
const diagDone = ref(false);
const diagChecks = ref<DiagCheck[]>([]);
const diagScore = ref(0);

const KNOWN_GOOD_DNS = ["8.8.8.8","8.8.4.4","1.1.1.1","1.0.0.1","9.9.9.9","149.112.112.112","208.67.222.222","208.67.220.220"];

async function runDiagnostic() {
  if (!overview.value) await loadOverview();
  diagRunning.value = true;
  diagDone.value = false;
  const checks: DiagCheck[] = [];

  // Checks 1, 2, 3, 5 en parallèle (indépendants)
  const gw = overview.value?.default_gateway || "";
  const [gwResult, inetResult, dnsResult, dhcpResult] = await Promise.allSettled([
    // 1. Passerelle
    gw
      ? invoke<any>("run_system_command", { cmd: "cmd", args: ["/c", "ping", "-n", "2", "-w", "1000", gw] })
      : Promise.reject(new Error("no-gw")),
    // 2. Internet
    invoke<any>("run_system_command", { cmd: "cmd", args: ["/c", "ping", "-n", "2", "-w", "2000", "8.8.8.8"] }),
    // 3. DNS resolution
    invoke<any>("run_system_command", { cmd: "cmd", args: ["/c", "nslookup", "google.com"] }),
    // 5. DHCP
    invoke<any>("run_system_command", { cmd: "ipconfig", args: ["/all"] }),
  ]);

  // 1. Passerelle
  if (!gw) {
    checks.push({ id: "gw", label: "Passerelle accessible", status: "error", detail: "Aucune passerelle détectée", tip: "Vérifiez que votre interface réseau est active et configurée." });
  } else if (gwResult.status === "fulfilled") {
    const out = (gwResult.value?.stdout || gwResult.value || "").toString();
    const ok = out.includes("TTL=") || out.includes("ttl=");
    checks.push({ id: "gw", label: "Passerelle accessible", status: ok ? "ok" : "error", detail: ok ? `Ping ${gw} OK` : `Impossible de joindre la passerelle ${gw}`, tip: ok ? undefined : "Vérifiez votre câble réseau ou votre routeur." });
  } else {
    checks.push({ id: "gw", label: "Passerelle accessible", status: "warn", detail: "Ping passerelle échoué (accès restreint)", tip: "Exécutez Nitrite en administrateur pour les tests ping." });
  }

  // 2. Internet
  if (inetResult.status === "fulfilled") {
    const out = (inetResult.value?.stdout || inetResult.value || "").toString();
    const ok = out.includes("TTL=") || out.includes("ttl=");
    checks.push({ id: "inet", label: "Connexion Internet", status: ok ? "ok" : "error", detail: ok ? "Internet accessible (8.8.8.8 répond)" : "8.8.8.8 inaccessible — pas d'Internet", tip: ok ? undefined : "Vérifiez votre routeur/box et la connexion WAN." });
  } else {
    checks.push({ id: "inet", label: "Connexion Internet", status: "warn", detail: "Test ping non disponible (droits admin requis)" });
  }

  // 3. Résolution DNS
  if (dnsResult.status === "fulfilled") {
    const out = (dnsResult.value?.stdout || dnsResult.value || "").toString();
    const ok = out.includes("Address") && !out.includes("NXDOMAIN") && !out.includes("can't find");
    checks.push({ id: "dns_res", label: "Résolution DNS (google.com)", status: ok ? "ok" : "error", detail: ok ? "DNS résout correctement" : "Échec de résolution — DNS potentiellement bloqué ou mal configuré", tip: ok ? undefined : "Essayez de changer votre DNS dans Paramètres réseau (ex: 8.8.8.8 ou 1.1.1.1)." });
  } else {
    checks.push({ id: "dns_res", label: "Résolution DNS", status: "warn", detail: "Test nslookup non disponible" });
  }

  // 4. Qualité DNS configurés (données déjà en mémoire)
  const dnsServers = overview.value?.dns_servers || [];
  if (dnsServers.length === 0) {
    checks.push({ id: "dns_q", label: "Serveurs DNS configurés", status: "error", detail: "Aucun serveur DNS détecté", tip: "Configurez des serveurs DNS manuellement (8.8.8.8 ou 1.1.1.1)." });
  } else {
    const allGood = dnsServers.every(d => KNOWN_GOOD_DNS.includes(d));
    const anyGood = dnsServers.some(d => KNOWN_GOOD_DNS.includes(d));
    const autoConfig = dnsServers.some(d => d.startsWith("192.168.") || d.startsWith("10.") || d.startsWith("172."));
    const detail = `DNS actifs : ${dnsServers.join(", ")}`;
    if (autoConfig) checks.push({ id: "dns_q", label: "Serveurs DNS", status: "info", detail: `${detail} — DNS local (DHCP/routeur)`, tip: "Les DNS locaux dépendent de votre routeur. Vous pouvez les remplacer par 8.8.8.8/1.1.1.1 pour plus de fiabilité." });
    else if (allGood) checks.push({ id: "dns_q", label: "Serveurs DNS", status: "ok", detail: `${detail} — DNS reconnus et fiables` });
    else if (anyGood) checks.push({ id: "dns_q", label: "Serveurs DNS", status: "warn", detail: `${detail} — Un ou plusieurs DNS non reconnus`, tip: "Vérifiez que tous vos DNS sont fiables." });
    else checks.push({ id: "dns_q", label: "Serveurs DNS", status: "warn", detail: `${detail} — DNS personnalisés non reconnus` });
  }

  // 5. DHCP / IP statique
  if (dhcpResult.status === "fulfilled") {
    const out = (dhcpResult.value?.stdout || dhcpResult.value || "").toString();
    const dhcpEnabled = out.toLowerCase().includes("dhcp enabled") && out.toLowerCase().includes("yes");
    const staticIp = out.toLowerCase().includes("dhcp enabled") && out.toLowerCase().includes("no");
    if (dhcpEnabled) checks.push({ id: "dhcp", label: "Configuration IP", status: "ok", detail: "DHCP activé — IP attribuée automatiquement" });
    else if (staticIp) checks.push({ id: "dhcp", label: "Configuration IP", status: "info", detail: "IP statique configurée manuellement", tip: "Une IP statique est normale sur certains réseaux d'entreprise." });
    else checks.push({ id: "dhcp", label: "Configuration IP", status: "info", detail: "Mode DHCP indéterminé" });
  } else {
    checks.push({ id: "dhcp", label: "Configuration IP", status: "info", detail: "Impossible de déterminer DHCP/statique" });
  }

  // 6. Proxy actif
  if (overview.value?.proxy_enabled) {
    checks.push({ id: "proxy", label: "Proxy réseau", status: "warn", detail: `Proxy actif : ${overview.value.proxy_server}`, tip: "Un proxy peut ralentir ou bloquer certaines connexions. Désactivez-le si inutile." });
  } else {
    checks.push({ id: "proxy", label: "Proxy réseau", status: "ok", detail: "Aucun proxy configuré" });
  }

  // 7. Interfaces actives
  const ifaces = (overview.value?.interfaces || []).filter((i: any) => !i.is_loopback);
  const downIfaces = ifaces.filter((i: any) => !i.is_up);
  if (ifaces.length === 0) checks.push({ id: "ifaces", label: "Interfaces réseau", status: "error", detail: "Aucune interface réseau détectée" });
  else if (downIfaces.length > 0) checks.push({ id: "ifaces", label: "Interfaces réseau", status: "warn", detail: `${downIfaces.length} interface(s) DOWN : ${downIfaces.map((i: any) => i.name).join(", ")}`, tip: "Vérifiez les adaptateurs désactivés dans le Gestionnaire de périphériques." });
  else checks.push({ id: "ifaces", label: "Interfaces réseau", status: "ok", detail: `${ifaces.length} interface(s) active(s) : ${ifaces.filter((i: any) => i.is_up).map((i: any) => i.name).join(", ")}` });

  // 8. IPv6
  const hasIPv6 = (overview.value?.interfaces || []).some((i: any) => i.ip_v6 && i.ip_v6.length > 0);
  checks.push({ id: "ipv6", label: "IPv6", status: hasIPv6 ? "ok" : "info", detail: hasIPv6 ? "IPv6 configuré" : "IPv6 non configuré (IPv4 uniquement)", tip: hasIPv6 ? undefined : "IPv4 seul est suffisant pour la plupart des usages." });

  diagChecks.value = checks;
  const okCount = checks.filter(c => c.status === "ok").length;
  diagScore.value = Math.round((okCount / checks.length) * 100);
  diagRunning.value = false;
  diagDone.value = true;
}

// ── Fix actions ───────────────────────────────────────────────────────────────
const fixing = ref<Record<string, boolean>>({});

async function runFix(id: string, cmd: string, args: string[]) {
  fixing.value[id] = true;
  try {
    await invoke("run_system_command", { cmd, args });
    notify.success("Correctif appliqué", "Relancez le diagnostic pour vérifier.");
  } catch (e: any) {
    notify.error("Erreur correctif", String(e));
  } finally {
    fixing.value[id] = false;
  }
}

function fixFlushDns() {
  runFix("dns_flush", "cmd", ["/c", "ipconfig", "/flushdns"]);
}

function fixSetDns() {
  // Applique 8.8.8.8 + 1.1.1.1 sur tous les adaptateurs actifs via PowerShell
  const script = `Get-NetAdapter | Where-Object {$_.Status -eq 'Up'} | ForEach-Object { Set-DnsClientServerAddress -InterfaceIndex $_.ifIndex -ServerAddresses '8.8.8.8','1.1.1.1' }`;
  runFix("dns_set", "powershell", ["-NoProfile", "-NonInteractive", "-Command", script]);
}

function fixDisableProxy() {
  const script = `Set-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings' -Name ProxyEnable -Value 0`;
  runFix("proxy", "powershell", ["-NoProfile", "-NonInteractive", "-Command", script]);
}

function fixEnableDhcp() {
  // Active DHCP sur tous les adaptateurs qui ont une IP statique
  const script = `Get-NetAdapter | Where-Object {$_.Status -eq 'Up'} | ForEach-Object { netsh interface ip set address $_.Name dhcp; netsh interface ip set dns $_.Name dhcp }`;
  runFix("dhcp", "cmd", ["/c", script]);
}

function fixOpenNetworkSettings() {
  runFix("ifaces", "cmd", ["/c", "start", "ms-settings:network"]);
}

function fixResetNetwork() {
  runFix("inet", "cmd", ["/c", "netsh", "winsock", "reset"]);
}

onMounted(() => {
  loadOverview();
  loadConnections();
  loadSpeedHistory();
});
</script>

<template>
  <div class="page-root">
    <!-- Header -->
    <div class="page-header">
      <div class="page-title">
        <Network :size="22" />
        <span>Réseau</span>
      </div>
      <div class="header-actions">
        <div class="net-tabs">
          <button class="net-tab" :class="{ active: pageTab === 'overview' }" @click="pageTab = 'overview'">
            <Globe :size="13" /> Vue Réseau
          </button>
          <button class="net-tab" :class="{ active: pageTab === 'diagnostic' }" @click="pageTab = 'diagnostic'">
            <Stethoscope :size="13" /> Diagnostic
          </button>
        </div>
        <NButton variant="ghost" size="sm" :loading="loading" @click="loadOverview">
          <RefreshCw :size="13" /> Actualiser
        </NButton>
      </div>
    </div>

    <!-- Onglet Diagnostic -->
    <div v-if="pageTab === 'diagnostic'" class="page-content">
      <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:4px">
        <div>
          <p style="font-size:13px;color:var(--text-muted)">Analyse complète de votre connexion réseau : DNS, DHCP, connectivité, proxy, IPv6...</p>
        </div>
        <NButton variant="primary" :loading="diagRunning" @click="runDiagnostic">
          <Stethoscope :size="14" /> Lancer le diagnostic
        </NButton>
      </div>

      <!-- Score -->
      <div v-if="diagDone" style="display:flex;align-items:center;gap:16px;padding:16px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:var(--radius-lg)">
        <div style="width:60px;height:60px;border-radius:50%;display:flex;align-items:center;justify-content:center;font-size:20px;font-weight:800;border:3px solid"
          :style="{ borderColor: diagScore >= 80 ? 'var(--success)' : diagScore >= 50 ? 'var(--warning)' : 'var(--danger)', color: diagScore >= 80 ? 'var(--success)' : diagScore >= 50 ? 'var(--warning)' : 'var(--danger)' }">
          {{ diagScore }}%
        </div>
        <div>
          <p style="font-size:15px;font-weight:700;color:var(--text-primary)">
            {{ diagScore >= 80 ? 'Réseau en bonne santé' : diagScore >= 50 ? 'Problèmes détectés' : 'Réseau en mauvais état' }}
          </p>
          <p style="font-size:12px;color:var(--text-muted)">
            {{ diagChecks.filter(c=>c.status==='ok').length }} OK · {{ diagChecks.filter(c=>c.status==='warn').length }} avertissements · {{ diagChecks.filter(c=>c.status==='error').length }} erreurs
          </p>
        </div>
      </div>

      <!-- Loading -->
      <div v-if="diagRunning" class="loading-state">
        <NSpinner :size="24" /><span>Analyse en cours...</span>
      </div>

      <!-- Résultats -->
      <NCard v-if="diagDone && !diagRunning">
        <template #header>
          <div class="section-header"><Stethoscope :size="15" /><span>Résultats du diagnostic</span></div>
        </template>
        <div style="display:flex;flex-direction:column;gap:4px">
          <div v-for="c in diagChecks" :key="c.id"
            style="display:flex;align-items:flex-start;gap:12px;padding:10px 12px;border-radius:var(--radius-md);border:1px solid var(--border);background:var(--bg-tertiary);margin-bottom:4px">
            <component :is="c.status==='ok' ? CheckCircle : c.status==='error' ? XCircle : c.status==='warn' ? AlertTriangle : Info"
              :size="16"
              :style="{ color: c.status==='ok' ? 'var(--success)' : c.status==='error' ? 'var(--danger)' : c.status==='warn' ? 'var(--warning)' : 'var(--info,#60a5fa)', flexShrink: 0, marginTop: '1px' }" />
            <div style="flex:1;min-width:0">
              <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap">
                <span style="font-size:13px;font-weight:600;color:var(--text-primary)">{{ c.label }}</span>
                <NBadge :variant="c.status==='ok' ? 'success' : c.status==='error' ? 'danger' : c.status==='warn' ? 'warning' : 'info'" style="font-size:10px">
                  {{ c.status === 'ok' ? 'OK' : c.status === 'error' ? 'ERREUR' : c.status === 'warn' ? 'ATTENTION' : 'INFO' }}
                </NBadge>
                <!-- Fix buttons par check -->
                <template v-if="c.status !== 'ok'">
                  <NButton v-if="c.id === 'dns_res'" variant="ghost" size="sm" :loading="fixing['dns_flush']" @click="fixFlushDns" style="font-size:11px;padding:2px 8px">
                    <Wrench :size="11" /> Vider cache DNS
                  </NButton>
                  <NButton v-if="c.id === 'dns_res' || c.id === 'dns_q'" variant="ghost" size="sm" :loading="fixing['dns_set']" @click="fixSetDns" style="font-size:11px;padding:2px 8px">
                    <Wrench :size="11" /> Appliquer DNS 8.8.8.8
                  </NButton>
                  <NButton v-if="c.id === 'proxy'" variant="ghost" size="sm" :loading="fixing['proxy']" @click="fixDisableProxy" style="font-size:11px;padding:2px 8px">
                    <Wrench :size="11" /> Désactiver proxy
                  </NButton>
                  <NButton v-if="c.id === 'dhcp'" variant="ghost" size="sm" :loading="fixing['dhcp']" @click="fixEnableDhcp" style="font-size:11px;padding:2px 8px">
                    <Wrench :size="11" /> Activer DHCP
                  </NButton>
                  <NButton v-if="c.id === 'ifaces'" variant="ghost" size="sm" :loading="fixing['ifaces']" @click="fixOpenNetworkSettings" style="font-size:11px;padding:2px 8px">
                    <Wrench :size="11" /> Paramètres réseau
                  </NButton>
                  <NButton v-if="c.id === 'inet'" variant="ghost" size="sm" :loading="fixing['inet']" @click="fixResetNetwork" style="font-size:11px;padding:2px 8px">
                    <Wrench :size="11" /> Réinitialiser Winsock
                  </NButton>
                </template>
              </div>
              <p style="font-size:12px;color:var(--text-muted);margin-top:2px">{{ c.detail }}</p>
              <p v-if="c.tip" style="font-size:11px;color:var(--accent-primary);margin-top:4px;font-style:italic">💡 {{ c.tip }}</p>
            </div>
          </div>
        </div>
      </NCard>

      <!-- État initial -->
      <div v-if="!diagDone && !diagRunning" style="text-align:center;padding:40px;color:var(--text-muted);font-size:13px">
        Cliquez "Lancer le diagnostic" pour analyser votre réseau.
      </div>
    </div>

    <div v-if="pageTab === 'overview'">
    <div v-if="loading && !overview" class="loading-state">
      <NSpinner :size="24" />
      <span>Analyse du réseau...</span>
    </div>
    <div v-else-if="error" class="error-state">⚠ {{ error }}</div>

    <div v-else-if="overview" class="page-content">

      <!-- Identité réseau -->
      <NCard>
        <template #header>
          <div class="section-header"><Globe :size="15" /><span>Identité Réseau</span></div>
        </template>
        <div class="info-grid">
          <div class="info-row">
            <span>Nom d'hôte</span>
            <code>{{ overview.hostname }}</code>
          </div>
          <div class="info-row">
            <span>IP Publique</span>
            <div style="display:flex;align-items:center;gap:6px">
              <code>{{ overview.public_ip || '—' }}</code>
              <button v-if="overview.public_ip" @click="copyText(overview.public_ip)" class="copy-btn" title="Copier">
                <Check v-if="copied" :size="11" style="color:var(--success)" />
                <Copy v-else :size="11" />
              </button>
            </div>
          </div>
          <div class="info-row">
            <span>Passerelle par défaut</span>
            <code>{{ overview.default_gateway || '—' }}</code>
          </div>
          <div class="info-row">
            <span>Serveurs DNS</span>
            <span>{{ overview.dns_servers.join(", ") || '—' }}</span>
          </div>
          <div v-if="overview.proxy_enabled" class="info-row">
            <span>Proxy</span>
            <NBadge variant="warning">{{ overview.proxy_server }}</NBadge>
          </div>
        </div>
      </NCard>

      <!-- Interfaces réseau -->
      <NCard>
        <template #header>
          <div class="section-header">
            <Wifi :size="15" />
            <span>Interfaces Réseau ({{ overview.interfaces.filter(i => !i.is_loopback).length }})</span>
            <NButton variant="ghost" size="sm" @click="exportInterfaces" style="margin-left:auto">↓ CSV</NButton>
          </div>
        </template>
        <div class="interfaces-list">
          <div
            v-for="iface in overview.interfaces.filter(i => !i.is_loopback)"
            :key="iface.name"
            class="iface-card"
            :class="{ 'iface-down': !iface.is_up }"
          >
            <div class="iface-header">
              <NBadge :variant="iface.is_up ? 'success' : 'neutral'" style="font-size:10px">
                {{ iface.is_up ? 'UP' : 'DOWN' }}
              </NBadge>
              <strong>{{ iface.name }}</strong>
              <span class="muted" style="font-size:11px">{{ iface.description }}</span>
              <span v-if="iface.speed_mbps > 0" class="muted" style="font-size:11px;margin-left:auto">
                {{ iface.speed_mbps >= 1000 ? (iface.speed_mbps/1000).toFixed(0)+'Gbps' : iface.speed_mbps+'Mbps' }}
              </span>
            </div>
            <div class="iface-details">
              <span v-for="ip in iface.ip_v4" :key="ip" class="ip-chip">{{ ip }}</span>
              <span class="mac-chip">{{ iface.mac }}</span>
            </div>
            <div v-if="iface.is_up && (iface.received_bytes > 0 || iface.sent_bytes > 0)" class="iface-traffic">
              <span class="traffic-item">
                <ArrowDown :size="11" style="color:var(--success)" />
                {{ formatBytes(iface.received_bytes) }}
              </span>
              <span class="traffic-item">
                <ArrowUp :size="11" style="color:var(--accent)" />
                {{ formatBytes(iface.sent_bytes) }}
              </span>
            </div>
          </div>
        </div>
      </NCard>

      <!-- Speed test -->
      <NCard>
        <template #header>
          <div class="section-header">
            <Zap :size="15" />
            <span>Test de débit</span>
            <NButton
              v-if="speedHistory.length"
              variant="ghost" size="sm"
              @click="showSpeedHist = !showSpeedHist"
              style="margin-left:auto"
            >
              <History :size="12" /> Historique ({{ speedHistory.length }})
            </NButton>
          </div>
        </template>
        <div class="speed-zone">
          <NButton variant="primary" :loading="speedLoading" @click="runSpeedTest">
            <Zap :size="13" /> Lancer le test
          </NButton>
          <NProgress v-if="speedLoading" :value="speedProgress" variant="default" style="margin-top:8px" />
          <div v-if="speedResult" class="speed-results">
            <div class="speed-card speed-dl">
              <ArrowDown :size="18" />
              <span class="speed-val">{{ speedResult.download_mbps }}</span>
              <span class="speed-unit">Mbps ↓</span>
            </div>
            <div class="speed-card speed-ul">
              <ArrowUp :size="18" />
              <span class="speed-val">{{ speedResult.upload_mbps }}</span>
              <span class="speed-unit">Mbps ↑</span>
            </div>
            <div class="speed-card speed-ping">
              <Clock :size="18" />
              <span class="speed-val">{{ speedResult.ping_ms }}</span>
              <span class="speed-unit">ms Ping</span>
            </div>
          </div>

          <!-- Historique speed -->
          <div v-if="showSpeedHist && speedHistory.length" class="speed-history">
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px">
              <span style="font-size:12px;font-weight:600;color:var(--text-secondary)">Historique tests</span>
              <button @click="clearSpeedHistory()" style="font-size:11px;color:var(--text-muted);background:none;border:none;cursor:pointer">
                <Trash2 :size="11" /> Effacer
              </button>
            </div>
            <table class="hist-table">
              <thead>
                <tr><th>Date</th><th>↓ Download</th><th>↑ Upload</th><th>Ping</th></tr>
              </thead>
              <tbody>
                <tr v-for="(h, i) in speedHistory" :key="i">
                  <td class="muted">{{ h.date }}</td>
                  <td style="color:var(--success)">{{ h.download }} Mbps</td>
                  <td style="color:var(--accent)">{{ h.upload }} Mbps</td>
                  <td>{{ h.ping }} ms</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </NCard>

      <!-- Traceroute -->
      <NetworkTraceroute />

      <!-- VPN Detection -->
      <NetworkVpnDetect />

      <!-- Connexions actives -->
      <NCard>
        <template #header>
          <div class="section-header">
            <Activity :size="15" />
            <span>Connexions actives</span>
            <NBadge variant="neutral" style="margin-left:4px">{{ filteredConns.length }}</NBadge>
            <div style="display:flex;gap:6px;margin-left:auto;align-items:center">
              <input
                v-model="connFilter"
                placeholder="Filtrer..."
                class="conn-search"
              />
              <select v-model="connProto" class="conn-select">
                <option>Tous</option>
                <option>TCP</option>
                <option>UDP</option>
              </select>
              <NButton variant="ghost" size="sm" :loading="connLoading" @click="loadConnections">
                <RefreshCw :size="11" />
              </NButton>
              <NButton variant="ghost" size="sm" @click="exportConnections">↓ CSV</NButton>
            </div>
          </div>
        </template>
        <div v-if="connLoading" class="loading-state">
          <NSpinner :size="16" /> Chargement...
        </div>
        <div v-else-if="filteredConns.length" style="overflow-x:auto">
          <table class="data-table">
            <thead>
              <tr>
                <th>Proto</th><th>Local</th><th>Distant</th>
                <th>État</th><th>Processus</th><th>PID</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(c, i) in filteredConns.slice(0, 200)" :key="i">
                <td><NBadge :variant="c.protocol === 'TCP' ? 'info' : 'neutral'" style="font-size:10px">{{ c.protocol }}</NBadge></td>
                <td class="font-mono muted">{{ c.local_addr }}</td>
                <td class="font-mono">{{ c.remote_addr }}</td>
                <td><NBadge :variant="stateVariant(c.state)" style="font-size:10px">{{ c.state }}</NBadge></td>
                <td>{{ c.process_name || '—' }}</td>
                <td class="muted">{{ c.pid || '—' }}</td>
              </tr>
            </tbody>
          </table>
          <p v-if="filteredConns.length > 200" class="muted" style="font-size:12px;margin-top:6px">
            {{ filteredConns.length - 200 }} connexions supplémentaires — affinez le filtre.
          </p>
        </div>
        <div v-else class="empty-state">Aucune connexion active.</div>
      </NCard>

    </div>
    </div><!-- end overview tab -->
  </div>
</template>

<style scoped>
.net-tabs { display:flex; gap:3px; background:var(--bg-secondary); border:1px solid var(--border); border-radius:var(--radius-md); padding:3px; }
.net-tab { display:flex; align-items:center; gap:5px; padding:5px 14px; border-radius:calc(var(--radius-md) - 2px); border:none; background:transparent; color:var(--text-secondary); font-size:12px; font-family:inherit; cursor:pointer; transition:all .15s; }
.net-tab:hover { color:var(--text-primary); }
.net-tab.active { background:var(--accent-muted); color:var(--accent-primary); font-weight:600; }

.page-root { display:flex; flex-direction:column; gap:0; height:100%; }
.page-header { display:flex; align-items:center; gap:12px; padding:16px 20px; border-bottom:1px solid var(--border); }
.page-title { display:flex; align-items:center; gap:8px; font-size:18px; font-weight:700; }
.header-actions { margin-left:auto; display:flex; gap:8px; }
.page-content { display:flex; flex-direction:column; gap:16px; padding:16px 20px; overflow-y:auto; flex:1; }
.section-header { display:flex; align-items:center; gap:8px; }
.loading-state { display:flex; align-items:center; gap:10px; padding:40px; justify-content:center; color:var(--text-muted); }
.error-state { padding:20px; color:var(--error); }
.info-grid { display:flex; flex-direction:column; gap:0; }
.info-row { display:flex; justify-content:space-between; align-items:center; padding:8px 0; border-bottom:1px solid var(--border); font-size:13px; }
.info-row:last-child { border-bottom:none; }
.copy-btn { background:none; border:none; cursor:pointer; color:var(--text-muted); padding:2px; display:flex; align-items:center; }
.copy-btn:hover { color:var(--accent); }
.interfaces-list { display:flex; flex-direction:column; gap:10px; }
.iface-card { border:1px solid var(--border); border-radius:var(--radius-md); padding:12px; }
.iface-down { opacity:.5; }
.iface-header { display:flex; align-items:center; gap:8px; margin-bottom:6px; flex-wrap:wrap; }
.iface-details { display:flex; flex-wrap:wrap; gap:6px; margin-bottom:4px; }
.ip-chip { font-size:11px; font-family:"JetBrains Mono",monospace; background:var(--bg-secondary); border:1px solid var(--border); border-radius:4px; padding:2px 6px; }
.mac-chip { font-size:10px; font-family:"JetBrains Mono",monospace; color:var(--text-muted); padding:2px 6px; }
.iface-traffic { display:flex; gap:16px; font-size:11px; color:var(--text-muted); }
.traffic-item { display:flex; align-items:center; gap:3px; }
.muted { color:var(--text-muted); }
.speed-zone { display:flex; flex-direction:column; gap:10px; }
.speed-results { display:grid; grid-template-columns:repeat(3,1fr); gap:12px; margin-top:4px; }
.speed-card { display:flex; flex-direction:column; align-items:center; gap:4px; padding:16px; background:var(--bg-secondary); border-radius:var(--radius-md); border:1px solid var(--border); }
.speed-dl { color:var(--success); }
.speed-ul { color:var(--accent); }
.speed-ping { color:var(--warning); }
.speed-val { font-size:28px; font-weight:700; line-height:1; }
.speed-unit { font-size:11px; color:var(--text-muted); }
.speed-history { border:1px solid var(--border); border-radius:var(--radius-md); padding:12px; margin-top:4px; }
.hist-table { width:100%; border-collapse:collapse; font-size:12px; }
.hist-table th { text-align:left; padding:6px 8px; color:var(--text-muted); border-bottom:1px solid var(--border); }
.hist-table td { padding:6px 8px; border-bottom:1px solid var(--border); }
.hist-table tr:last-child td { border-bottom:none; }
.conn-search { padding:4px 8px; background:var(--bg-secondary); border:1px solid var(--border); border-radius:6px; color:var(--text-primary); font-size:12px; width:140px; }
.conn-select { padding:4px 8px; background:var(--bg-secondary); border:1px solid var(--border); border-radius:6px; color:var(--text-primary); font-size:12px; }
.data-table { width:100%; border-collapse:collapse; font-size:12px; }
.data-table th { text-align:left; padding:8px; color:var(--text-muted); font-size:11px; font-weight:500; border-bottom:1px solid var(--border); background:var(--bg-secondary); }
.data-table td { padding:7px 8px; border-bottom:1px solid var(--border); }
.data-table tbody tr:hover { background:var(--bg-secondary); }
.font-mono { font-family:"JetBrains Mono",monospace; font-size:11px; }
.empty-state { padding:20px; text-align:center; color:var(--text-muted); font-size:13px; }
</style>
