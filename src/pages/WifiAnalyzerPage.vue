<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
import { csvCell } from "@/composables/useExportData";
import { invoke } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import { useNotificationStore } from "@/stores/notifications";
import { Wifi, RefreshCw, Play, Square, Lock, Unlock, Download, Filter } from "lucide-vue-next";

const notify = useNotificationStore();

interface WifiNetwork {
  ssid: string; bssid: string; signal_percent: number; channel: number;
  band: string; authentication: string; network_type: string; radio_type: string;
}

const networks    = ref<WifiNetwork[]>([]);
const loading     = ref(false);
const autoRefresh = ref(false);
const countdown   = ref(30);
const SORT_OPTS = ['signal','channel','ssid'] as const
const BAND_OPTS = [{v:'all' as const,l:'Tous'},{v:'2.4' as const,l:'2.4G'},{v:'5' as const,l:'5G'}]
const SEC_OPTS  = [{v:'all' as const,l:'Tous'},{v:'secured' as const,l:'Sécurisé'},{v:'open' as const,l:'Ouvert'}]
const sortBy      = ref<"signal"|"channel"|"ssid">("signal");
const filterBand  = ref<"all"|"2.4"|"5">("all");
const filterSec   = ref<"all"|"open"|"secured">("all");
let   timer: ReturnType<typeof setInterval> | null = null;
let   cdTimer: ReturnType<typeof setInterval> | null = null;

// --- Canvas heatmap ---
const heatCanvas24 = ref<HTMLCanvasElement | null>(null);
const heatCanvas5  = ref<HTMLCanvasElement | null>(null);

// --- Computed ---
const sorted = computed(() => {
  let list = [...networks.value];
  if (filterBand.value === "2.4") list = list.filter(n => !n.band.includes("5"));
  if (filterBand.value === "5")   list = list.filter(n => n.band.includes("5"));
  if (filterSec.value === "open")    list = list.filter(n => n.authentication === "Open");
  if (filterSec.value === "secured") list = list.filter(n => n.authentication !== "Open");
  return list.sort((a, b) => {
    if (sortBy.value === "signal")  return b.signal_percent - a.signal_percent;
    if (sortBy.value === "channel") return a.channel - b.channel;
    return a.ssid.localeCompare(b.ssid);
  });
});

const channels24 = computed(() => {
  const ch: Record<number, WifiNetwork[]> = {};
  networks.value.filter(n => !n.band.includes("5")).forEach(n => {
    if (!ch[n.channel]) ch[n.channel] = [];
    ch[n.channel].push(n);
  });
  return ch;
});

const channels5 = computed(() => {
  const ch: Record<number, WifiNetwork[]> = {};
  networks.value.filter(n => n.band.includes("5")).forEach(n => {
    if (!ch[n.channel]) ch[n.channel] = [];
    ch[n.channel].push(n);
  });
  return ch;
});

const bestChannel24 = computed(() => {
  const candidates = [1, 6, 11];
  return candidates.sort((a, b) => (channels24.value[a]?.length || 0) - (channels24.value[b]?.length || 0))[0];
});

const bestChannel5 = computed(() => {
  // Pas de 5G détecté → pas de reco.
  if (!Object.keys(channels5.value).length) return null;
  // Évaluer les canaux 5G STANDARD non-recouvrants, pas seulement les canaux
  // occupés : sinon un canal réellement libre (0 réseau) n'était jamais proposé
  // et la reco « canal 5 GHz libre » restait du code mort.
  const std5 = [36, 40, 44, 48, 149, 153, 157, 161];
  return std5.slice().sort((a, b) => (channels5.value[a]?.length || 0) - (channels5.value[b]?.length || 0))[0];
});

// Congestion globale 2.4 GHz (score 0–100)
const congestion24 = computed(() => {
  const total = networks.value.filter(n => !n.band.includes("5")).length;
  return Math.min(100, Math.round((total / 12) * 100));
});

// Recommandations contextuelles
const recommendations = computed(() => {
  const recs: { icon: string; title: string; desc: string; level: "ok"|"warn"|"danger" }[] = [];
  const open = networks.value.filter(n => n.authentication === "Open");
  if (open.length > 0)
    recs.push({ icon: "🔓", title: `${open.length} réseau(x) ouvert(s)`, desc: "Des réseaux sans chiffrement sont détectés. Évitez de vous y connecter.", level: "warn" });
  if (congestion24.value >= 70)
    recs.push({ icon: "📶", title: "Bande 2.4 GHz saturée", desc: `${congestion24.value}% de saturation. Basculez sur 5 GHz si votre routeur le supporte.`, level: "danger" });
  else if (congestion24.value >= 40)
    recs.push({ icon: "⚠️", title: "Bande 2.4 GHz chargée", desc: `Canal recommandé : ${bestChannel24.value}. Configurez votre routeur.`, level: "warn" });
  else
    recs.push({ icon: "✅", title: "Bande 2.4 GHz dégagée", desc: `Canal optimal : ${bestChannel24.value}. Environnement favorable.`, level: "ok" });
  if (bestChannel5.value && (channels5.value[bestChannel5.value]?.length || 0) === 0)
    recs.push({ icon: "⚡", title: "Canal 5 GHz libre détecté", desc: `Canal ${bestChannel5.value} disponible — excellente performance potentielle.`, level: "ok" });
  return recs;
});

// --- Helpers ---
function signalColor(pct: number): string {
  if (pct >= 70) return "var(--success)";
  if (pct >= 40) return "var(--warning)";
  return "var(--danger)";
}

function signalBars(pct: number): number {
  if (pct >= 75) return 4;
  if (pct >= 50) return 3;
  if (pct >= 25) return 2;
  return 1;
}

function maxChCount(map: Record<number, WifiNetwork[]>): number {
  const vals = Object.values(map).map(v => v.length);
  return Math.max(...vals, 1);
}

// --- Heatmap canvas ---
function drawHeatmap(canvas: HTMLCanvasElement, chMap: Record<number, WifiNetwork[]>, channels: number[], best: number | null) {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const W = canvas.width, H = canvas.height;
  ctx.clearRect(0, 0, W, H);
  const maxCount = maxChCount(chMap);
  const colW = W / channels.length;

  channels.forEach((ch, i) => {
    const nets = chMap[ch] || [];
    const count = nets.length;
    const ratio = count / maxCount;
    // couleur de chaleur : bleu → vert → jaune → rouge
    let r = 0, g = 0, b = 0;
    if (ratio < 0.33) { r = 0; g = Math.round(ratio / 0.33 * 200); b = 255; }
    else if (ratio < 0.66) { const t = (ratio - 0.33) / 0.33; r = Math.round(t * 255); g = 200; b = Math.round(255 * (1 - t)); }
    else { r = 255; g = Math.round(200 * (1 - (ratio - 0.66) / 0.34)); b = 0; }

    const alpha = count === 0 ? 0.08 : 0.15 + ratio * 0.75;
    ctx.fillStyle = `rgba(${r},${g},${b},${alpha})`;
    ctx.fillRect(i * colW + 1, 0, colW - 2, H - 22);

    // Barre proportion
    const barH = Math.max(count === 0 ? 0 : 4, ratio * (H - 40));
    ctx.fillStyle = ch === best ? "rgba(74,222,128,.9)" : `rgba(${r},${g},${b},0.85)`;
    ctx.beginPath();
    ctx.roundRect(i * colW + 3, H - 22 - barH, colW - 6, barH, 3);
    ctx.fill();

    // Label canal
    ctx.fillStyle = ch === best ? "#4ade80" : count > 0 ? "#e2e8f0" : "#64748b";
    ctx.font = `${ch === best ? "bold " : ""}11px system-ui`;
    ctx.textAlign = "center";
    ctx.fillText(String(ch), i * colW + colW / 2, H - 6);

    // Compteur
    if (count > 0) {
      ctx.fillStyle = ch === best ? "#4ade80" : "#94a3b8";
      ctx.font = "10px system-ui";
      ctx.fillText(String(count), i * colW + colW / 2, H - 26);
    }
  });

  // Bordure canal recommandé
  if (best !== null) {
    const idx = channels.indexOf(best);
    if (idx >= 0) {
      ctx.strokeStyle = "rgba(74,222,128,0.7)";
      ctx.lineWidth = 2;
      ctx.strokeRect(idx * colW + 1, 0, colW - 2, H - 22);
    }
  }
}

async function redrawHeatmaps() {
  await nextTick();
  if (heatCanvas24.value && Object.keys(channels24.value).length) {
    drawHeatmap(heatCanvas24.value, channels24.value, [1,2,3,4,5,6,7,8,9,10,11,12,13], bestChannel24.value);
  }
  if (heatCanvas5.value && Object.keys(channels5.value).length) {
    const ch5list = [...new Set(networks.value.filter(n => n.band.includes("5")).map(n => n.channel))].sort((a,b)=>a-b);
    drawHeatmap(heatCanvas5.value, channels5.value, ch5list, bestChannel5.value);
  }
}

// --- Export CSV ---
function exportCSV() {
  if (!networks.value.length) { notify.warning("Aucune donnée", "Effectuez un scan d'abord."); return; }
  const header = "SSID,BSSID,Signal%,Canal,Bande,Authentification,Type réseau,Radio";
  const rows = networks.value.map(n =>
    [n.ssid, n.bssid, n.signal_percent, n.channel, n.band, n.authentication, n.network_type, n.radio_type].map(csvCell).join(",")
  );
  const csv = [header, ...rows].join("\n");
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `wifi_scan_${new Date().toISOString().slice(0,19).replace(/[T:]/g,"-")}.csv`;
  a.click();
  URL.revokeObjectURL(url);
  notify.success("Export CSV", `${networks.value.length} réseaux exportés`);
}

// --- Scan ---
async function scan(silent = false) {
  if (!silent) loading.value = true;
  try {
    const res = await invoke<WifiNetwork[]>("get_nearby_wifi");
    networks.value = res;
    if (!silent && res.length === 0) notify.warning("Aucun réseau", "Vérifiez que le Wi-Fi est activé");
    await redrawHeatmaps();
  } catch (e: unknown) {
    if (!silent) notify.error("Erreur scan WiFi", String(e));
  } finally {
    if (!silent) loading.value = false;
  }
}

function toggleAutoRefresh() {
  if (autoRefresh.value) {
    stopAuto();
  } else {
    autoRefresh.value = true;
    countdown.value = 30;
    scan(true);
    timer   = setInterval(() => { scan(true); countdown.value = 30; }, 30000);
    cdTimer = setInterval(() => { countdown.value = Math.max(0, countdown.value - 1); }, 1000);
  }
}

function stopAuto() {
  autoRefresh.value = false;
  if (timer)   { clearInterval(timer);   timer = null; }
  if (cdTimer) { clearInterval(cdTimer); cdTimer = null; }
}

onMounted(async () => { await scan(); });
onUnmounted(() => {
  stopAuto();
  // Libérer les contextes canvas pour éviter les fuites mémoire
  if (heatCanvas24.value) {
    const ctx = heatCanvas24.value.getContext("2d");
    if (ctx) ctx.clearRect(0, 0, heatCanvas24.value.width, heatCanvas24.value.height);
  }
  if (heatCanvas5.value) {
    const ctx = heatCanvas5.value.getContext("2d");
    if (ctx) ctx.clearRect(0, 0, heatCanvas5.value.width, heatCanvas5.value.height);
  }
});
</script>

<template>
  <div class="wifi-page">
    <div class="page-header">
      <div>
        <h1>Analyseur WiFi</h1>
        <p class="page-subtitle">Réseaux environnants, heatmap canaux, recommandations et export CSV</p>
      </div>
      <div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center">
        <NButton variant="secondary" size="sm" @click="exportCSV" :disabled="!networks.length">
          <Download :size="14" /> Export CSV
        </NButton>
        <NButton :variant="autoRefresh ? 'warning' : 'secondary'" size="sm" @click="toggleAutoRefresh">
          <component :is="autoRefresh ? Square : Play" :size="14" />
          {{ autoRefresh ? `Auto (${countdown}s)` : 'Auto 30s' }}
        </NButton>
        <NButton variant="secondary" size="sm" :loading="loading" @click="scan()">
          <RefreshCw :size="14" /> Rafraîchir
        </NButton>
      </div>
    </div>

    <div v-if="loading" style="display:flex;justify-content:center;padding:40px"><NSpinner :size="24" /></div>

    <template v-if="!loading">
      <!-- Stats résumé -->
      <div class="stats-row" v-if="networks.length">
        <div class="stat-card">
          <div class="stat-val">{{ networks.length }}</div>
          <div class="stat-label">Réseaux détectés</div>
        </div>
        <div class="stat-card">
          <div class="stat-val" style="color:var(--accent-primary)">{{ networks.filter(n => n.band.includes('5')).length }}</div>
          <div class="stat-label">Réseaux 5 GHz</div>
        </div>
        <div class="stat-card">
          <div class="stat-val" style="color:var(--warning)">{{ networks.filter(n => !n.band.includes('5')).length }}</div>
          <div class="stat-label">Réseaux 2.4 GHz</div>
        </div>
        <div class="stat-card">
          <div class="stat-val" :style="{ color: congestion24 >= 70 ? 'var(--danger)' : congestion24 >= 40 ? 'var(--warning)' : 'var(--success)' }">
            {{ congestion24 }}%
          </div>
          <div class="stat-label">Saturation 2.4 GHz</div>
        </div>
      </div>

      <!-- Recommandations contextuelles -->
      <div v-if="networks.length" class="reco-grid">
        <div v-for="r in recommendations" :key="r.title" class="reco-card" :class="`reco-${r.level}`">
          <span class="reco-icon">{{ r.icon }}</span>
          <div>
            <div class="reco-title">{{ r.title }}</div>
            <div class="reco-desc">{{ r.desc }}</div>
          </div>
        </div>
      </div>

      <!-- Filtres + liste réseaux -->
      <NCard>
        <template #header>
          <div style="display:flex;align-items:center;gap:8px;width:100%;flex-wrap:wrap">
            <Wifi :size="16" />
            <span>Réseaux détectés ({{ sorted.length }}/{{ networks.length }})</span>
            <div style="margin-left:auto;display:flex;gap:4px;flex-wrap:wrap;align-items:center">
              <Filter :size="12" style="color:var(--text-muted)" />
              <button v-for="s in SORT_OPTS" :key="s" class="sort-btn" :class="{active:sortBy===s}" @click="sortBy=s">{{ s }}</button>
              <div class="sep-v"></div>
              <button v-for="b in BAND_OPTS" :key="b.v" class="sort-btn" :class="{active:filterBand===b.v}" @click="filterBand=b.v">{{ b.l }}</button>
              <div class="sep-v"></div>
              <button v-for="s in SEC_OPTS" :key="s.v" class="sort-btn" :class="{active:filterSec===s.v}" @click="filterSec=s.v">{{ s.l }}</button>
            </div>
          </div>
        </template>

        <div v-if="networks.length === 0" class="empty-state">
          <Wifi :size="32" style="color:var(--text-muted);opacity:.2" />
          <p>Aucun réseau WiFi détecté</p>
        </div>
        <div v-else-if="sorted.length === 0" class="empty-state">
          <Wifi :size="32" style="color:var(--text-muted);opacity:.2" />
          <p>Aucun réseau ne correspond aux filtres</p>
        </div>

        <div v-else class="networks-list">
          <div v-for="n in sorted" :key="n.bssid || n.ssid" class="network-row">
            <div class="signal-bars">
              <div v-for="i in 4" :key="i" class="bar-seg" :class="{ active: i <= signalBars(n.signal_percent) }" :style="{ background: i <= signalBars(n.signal_percent) ? signalColor(n.signal_percent) : '' }"></div>
            </div>
            <div class="net-info">
              <span class="net-ssid">{{ n.ssid || "(réseau caché)" }}</span>
              <span class="net-bssid">{{ n.bssid }}</span>
            </div>
            <div class="net-badges">
              <NBadge :variant="n.band.includes('5') ? 'accent' : 'warning'" size="sm">{{ n.band }}</NBadge>
              <NBadge variant="neutral" size="sm">Ch {{ n.channel || "?" }}</NBadge>
              <NBadge :variant="n.authentication === 'Open' ? 'danger' : 'neutral'" size="sm">
                <component :is="n.authentication === 'Open' ? Unlock : Lock" :size="10" style="margin-right:3px" />
                {{ n.authentication }}
              </NBadge>
              <NBadge v-if="n.radio_type" variant="neutral" size="sm">{{ n.radio_type }}</NBadge>
            </div>
            <div class="signal-pct" :style="{ color: signalColor(n.signal_percent) }">{{ n.signal_percent }}%</div>
          </div>
        </div>
      </NCard>

      <!-- Heatmap canaux 2.4 GHz -->
      <NCard v-if="Object.keys(channels24).length">
        <template #header>
          <div style="display:flex;align-items:center;gap:8px">
            <span>Heatmap canaux 2.4 GHz</span>
            <NBadge variant="success" size="sm">Recommandé : canal {{ bestChannel24 }}</NBadge>
            <NBadge :variant="congestion24 >= 70 ? 'danger' : congestion24 >= 40 ? 'warning' : 'success'" size="sm">
              Saturation {{ congestion24 }}%
            </NBadge>
          </div>
        </template>
        <canvas ref="heatCanvas24" class="heat-canvas" width="780" height="140"></canvas>
        <p class="ch-tip">Canaux non-chevauchants 2.4 GHz : <strong>1, 6, 11</strong> · Canal optimal ici : <strong>{{ bestChannel24 }}</strong> · Couleur = densité (bleu→vert→rouge)</p>
      </NCard>

      <!-- Heatmap canaux 5 GHz -->
      <NCard v-if="Object.keys(channels5).length">
        <template #header>
          <div style="display:flex;align-items:center;gap:8px">
            <span>Heatmap canaux 5 GHz</span>
            <NBadge v-if="bestChannel5" variant="accent" size="sm">Recommandé : canal {{ bestChannel5 }}</NBadge>
          </div>
        </template>
        <canvas ref="heatCanvas5" class="heat-canvas" width="780" height="140"></canvas>
        <p class="ch-tip">Les canaux 5 GHz sont non-chevauchants · Canal le moins peuplé : <strong>{{ bestChannel5 ?? '—' }}</strong></p>
      </NCard>
    </template>
  </div>
</template>

<style scoped>
.wifi-page { display:flex; flex-direction:column; gap:16px; }
.page-header { display:flex; justify-content:space-between; align-items:flex-start; flex-wrap:wrap; gap:12px; }
.page-header h1 { font-size:24px; font-weight:700; }
.page-subtitle { color:var(--text-muted); font-size:13px; margin-top:2px; }

.stats-row { display:grid; grid-template-columns:repeat(4,1fr); gap:12px; }
@media(max-width:700px){.stats-row{grid-template-columns:repeat(2,1fr);}}
.stat-card { background:var(--bg-secondary); border:1px solid var(--border); border-radius:var(--radius-md); padding:16px; text-align:center; }
.stat-val { font-size:22px; font-weight:700; color:var(--text-primary); }
.stat-label { font-size:11px; color:var(--text-muted); margin-top:4px; }

.reco-grid { display:grid; grid-template-columns:repeat(auto-fill,minmax(260px,1fr)); gap:10px; }
.reco-card { display:flex; align-items:flex-start; gap:12px; padding:12px 14px; border-radius:var(--radius-md); border:1px solid var(--border); background:var(--bg-secondary); }
.reco-ok    { border-color:rgba(74,222,128,.25); background:rgba(74,222,128,.04); }
.reco-warn  { border-color:rgba(250,204,21,.25); background:rgba(250,204,21,.04); }
.reco-danger{ border-color:rgba(239,68,68,.25);  background:rgba(239,68,68,.04);  }
.reco-icon { font-size:20px; flex-shrink:0; margin-top:1px; }
.reco-title { font-size:12px; font-weight:600; color:var(--text-primary); }
.reco-desc  { font-size:11px; color:var(--text-muted); margin-top:3px; line-height:1.45; }

.sort-btn { padding:3px 10px; border:1px solid var(--border); background:var(--bg-tertiary); color:var(--text-secondary); font-size:11px; cursor:pointer; font-family:inherit; border-radius:var(--radius-sm); }
.sort-btn.active { background:var(--accent-muted); border-color:var(--accent-primary); color:var(--accent-primary); }
.sep-v { width:1px; height:14px; background:var(--border); margin:0 2px; }

.networks-list { display:flex; flex-direction:column; gap:4px; }
.network-row { display:flex; align-items:center; gap:12px; padding:8px 10px; border-radius:var(--radius-sm); }
.network-row:hover { background:var(--bg-tertiary); }
.signal-bars { display:flex; gap:2px; align-items:flex-end; width:32px; height:18px; flex-shrink:0; }
.bar-seg { flex:1; background:var(--border); border-radius:1px; }
.bar-seg:nth-child(1){height:25%} .bar-seg:nth-child(2){height:50%} .bar-seg:nth-child(3){height:75%} .bar-seg:nth-child(4){height:100%}
.net-info { flex:1; min-width:0; display:flex; flex-direction:column; gap:2px; }
.net-ssid { font-size:13px; font-weight:600; color:var(--text-primary); overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.net-bssid { font-size:10px; color:var(--text-muted); font-family:monospace; }
.net-badges { display:flex; gap:4px; flex-wrap:wrap; align-items:center; }
.signal-pct { font-size:13px; font-weight:700; font-family:monospace; flex-shrink:0; min-width:40px; text-align:right; }

.heat-canvas { width:100%; height:140px; border-radius:var(--radius-md); display:block; background:var(--bg-primary); }
.ch-tip { font-size:12px; color:var(--text-muted); margin-top:10px; padding-top:10px; border-top:1px solid var(--border); }

.empty-state { display:flex; flex-direction:column; align-items:center; gap:8px; padding:48px; color:var(--text-muted); font-size:13px; }
</style>
