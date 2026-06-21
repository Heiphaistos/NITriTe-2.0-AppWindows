<script setup lang="ts">
import { invoke } from "@/utils/invoke";
import { ref, onMounted, onUnmounted } from "vue";
import { Monitor, Settings, RefreshCw, Cpu, Zap, Thermometer, WifiOff } from "lucide-vue-next";
import NBadge from "@/components/ui/NBadge.vue";
import NButton from "@/components/ui/NButton.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import NCollapse from "@/components/ui/NCollapse.vue";

interface GpuDetailed {
  name: string; adapter_ram_mb: number; driver_version: string;
  driver_date: string; video_processor: string; video_mode: string;
  current_resolution: string; current_refresh_rate: number;
  status: string; pnp_device_id: string; adapter_dac_type: string;
}

const props = defineProps<{ gpuList: GpuDetailed[] }>();

interface GpuTemp { name: string; temp_celsius: number; source: string; }
const gpuTemps      = ref<GpuTemp[]>([]);
const tempsLoading  = ref(false);
let   pollTimer: ReturnType<typeof setInterval> | null = null;

async function loadGpuTemps() {
  tempsLoading.value = true;
  try { gpuTemps.value = await invoke<GpuTemp[]>("get_gpu_temps"); }
  catch { gpuTemps.value = []; }
  finally { tempsLoading.value = false; }
}

onMounted(() => {
  loadGpuTemps();
  // Polling temps réel toutes les 3 secondes
  pollTimer = setInterval(loadGpuTemps, 3000);
});
onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer);
});

function vramStr(mb: number): string {
  return mb >= 1024 ? `${(mb / 1024).toFixed(0)} GB` : `${mb} MB`;
}

function gpuVendor(name: string): { label: string; color: string } {
  const n = name.toLowerCase();
  if (n.includes("nvidia") || n.includes("geforce") || n.includes("quadro") || n.includes("rtx") || n.includes("gtx"))
    return { label: "NVIDIA", color: "#76b900" };
  if (n.includes("amd") || n.includes("radeon") || n.includes("rx ") || n.includes("vega") || n.includes("firepro"))
    return { label: "AMD", color: "#ed1c24" };
  if (n.includes("intel") || n.includes("uhd") || n.includes("iris") || n.includes("arc"))
    return { label: "Intel", color: "#0071c5" };
  return { label: "Autre", color: "#6b7280" };
}

function gpuCategory(g: GpuDetailed): string {
  const name = (g.name || "").toLowerCase();
  if (name.includes("uhd") || name.includes("iris") || name.includes("vega") && !name.includes("radeon rx"))
    return "Intégré";
  return "Dédié";
}

function vramScore(mb: number): { label: string; variant: "success"|"warning"|"danger" } {
  if (mb >= 8192) return { label: "Gaming / Workstation", variant: "success" };
  if (mb >= 4096) return { label: "Gaming standard", variant: "success" };
  if (mb >= 2048) return { label: "Bureautique avancé", variant: "warning" };
  return { label: "Usage basique", variant: "danger" };
}

function openDevMgr() { invoke("open_path", { path: "devmgmt.msc" }).catch(() => {}); }
function openWinUpdate() { invoke("open_path", { path: "ms-settings:windowsupdate-driver" }).catch(() => {}); }
function openDxDiag() {
  invoke("execute_script", { content: "Start-Process dxdiag", scriptType: "powershell" }).catch(() => {});
}
</script>

<template>
  <div class="diag-tab-content">
    <DiagBanner
      :icon="Monitor"
      title="Carte(s) Graphique(s)"
      desc="VRAM, drivers, résolution et processeur vidéo"
      color="purple"
    />

    <!-- Actions rapides -->
    <div class="gpu-actions">
      <NButton variant="secondary" size="sm" @click="openDevMgr">
        <Settings :size="13" /> Gestionnaire de périphériques
      </NButton>
      <NButton variant="secondary" size="sm" @click="openWinUpdate">
        <RefreshCw :size="13" /> Windows Update (pilotes)
      </NButton>
      <NButton variant="ghost" size="sm" @click="openDxDiag">
        <Cpu :size="13" /> DirectX Diagnostic
      </NButton>
    </div>

    <div v-if="!gpuList.length" class="diag-loading"><div class="diag-spinner"></div> Aucun GPU détecté...</div>
    <template v-else>
      <p class="diag-section-label">Carte(s) graphique(s) — {{ gpuList.length }} détectée(s)</p>
      <div v-for="(g, i) in gpuList" :key="i" class="card-block">
        <div class="block-title">
          <span>{{ g.name }}</span>
          <div style="display:flex;gap:6px;align-items:center">
            <!-- Vendor badge -->
            <span class="vendor-badge" :style="{ background: gpuVendor(g.name).color + '22', color: gpuVendor(g.name).color, borderColor: gpuVendor(g.name).color + '44' }">
              {{ gpuVendor(g.name).label }}
            </span>
            <!-- Dédié / Intégré -->
            <NBadge :variant="gpuCategory(g) === 'Dédié' ? 'success' : 'warning'">{{ gpuCategory(g) }}</NBadge>
            <!-- Statut -->
            <NBadge :variant="g.status === 'OK' ? 'success' : 'warning'">{{ g.status }}</NBadge>
          </div>
        </div>

        <!-- VRAM score -->
        <div v-if="g.adapter_ram_mb > 0" class="vram-score-row">
          <Zap :size="13" :style="{ color: vramScore(g.adapter_ram_mb).variant === 'success' ? 'var(--success)' : 'var(--warning)' }" />
          <span>{{ vramStr(g.adapter_ram_mb) }} VRAM — </span>
          <NBadge :variant="vramScore(g.adapter_ram_mb).variant" size="sm">{{ vramScore(g.adapter_ram_mb).label }}</NBadge>
        </div>

        <div class="info-grid">
          <div class="info-row"><span>Mémoire vidéo (VRAM)</span><NBadge variant="info">{{ vramStr(g.adapter_ram_mb) }}</NBadge></div>
          <div class="info-row"><span>Résolution actuelle</span><span>{{ g.current_resolution || "N/A" }}</span></div>
          <div class="info-row"><span>Fréquence de rafraîchissement</span><span>{{ g.current_refresh_rate > 0 ? g.current_refresh_rate + ' Hz' : 'N/A' }}</span></div>
          <div class="info-row"><span>Version du driver</span><code>{{ g.driver_version || "N/A" }}</code></div>
          <div class="info-row"><span>Date du driver</span><span>{{ g.driver_date || "N/A" }}</span></div>
          <div class="info-row"><span>Processeur vidéo</span><span>{{ g.video_processor || "N/A" }}</span></div>
          <div class="info-row"><span>Mode vidéo</span><span class="muted" style="font-size:11px">{{ g.video_mode || "N/A" }}</span></div>
          <div class="info-row"><span>Type DAC</span><span>{{ g.adapter_dac_type || "N/A" }}</span></div>
          <div class="info-row info-full"><span>PNP Device ID</span><code style="font-size:9px;word-break:break-all">{{ g.pnp_device_id || "N/A" }}</code></div>
        </div>
      </div>
    </template>

    <!-- Températures GPU -->
    <NCollapse title="Températures GPU" storageKey="diag-gpu-temps" :defaultOpen="true" style="margin-top:4px">
      <template #header-extra>
        <span v-if="!tempsLoading" class="temp-live-dot" title="Actualisation toutes les 3s">● LIVE</span>
        <RefreshCw v-else :size="11" class="temp-spin" />
      </template>

      <!-- Aucune donnée du tout -->
      <div v-if="gpuTemps.length === 0 && !tempsLoading" class="card-block" style="margin-top:0">
        <div style="display:flex;align-items:flex-start;gap:10px;font-size:12px;color:var(--text-muted)">
          <WifiOff :size="16" style="flex-shrink:0;margin-top:1px;color:var(--warning)" />
          <div>
            <div style="font-weight:600;color:var(--text-secondary);margin-bottom:4px">Températures GPU non lisibles</div>
            <div>Les drivers NVIDIA/AMD modernes ne remontent pas la température via WMI standard.</div>
            <div style="margin-top:6px">Installer l'un de ces outils pour activer la lecture :</div>
            <div style="margin-top:4px;display:flex;gap:6px;flex-wrap:wrap">
              <span class="temp-tool-badge">OpenHardwareMonitor</span>
              <span class="temp-tool-badge">LibreHardwareMonitor</span>
              <span class="temp-tool-badge">HWiNFO64</span>
              <span class="temp-tool-badge">MSI Afterburner</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Spinner premier chargement -->
      <div v-else-if="gpuTemps.length === 0 && tempsLoading" style="display:flex;align-items:center;gap:8px;font-size:12px;color:var(--text-muted);padding:8px 0">
        <RefreshCw :size="13" class="temp-spin" /> Lecture des températures…
      </div>

      <!-- Cartes températures -->
      <div v-else style="display:flex;flex-wrap:wrap;gap:10px">
        <div
          v-for="(gt, i) in gpuTemps" :key="i"
          class="gpu-temp-card"
          :class="gt.source === 'unavailable' ? 'gpu-temp-na' : gt.temp_celsius >= 90 ? 'gpu-temp-hot' : gt.temp_celsius >= 75 ? 'gpu-temp-warm' : 'gpu-temp-ok'"
        >
          <Thermometer :size="18" />
          <div class="gpu-temp-value">
            <template v-if="gt.source === 'unavailable'">N/A</template>
            <template v-else>{{ gt.temp_celsius.toFixed(0) }}°C</template>
          </div>
          <div class="gpu-temp-name" :title="gt.name">{{ gt.name }}</div>
          <span class="gpu-temp-src">{{ gt.source }}</span>
          <!-- Barre de chaleur -->
          <div v-if="gt.source !== 'unavailable'" class="gpu-temp-bar">
            <div class="gpu-temp-fill" :style="{
              width: Math.min(100, (gt.temp_celsius / 100) * 100) + '%',
              background: gt.temp_celsius >= 90 ? 'var(--danger)' : gt.temp_celsius >= 75 ? 'var(--warning)' : 'var(--success)'
            }" />
          </div>
        </div>
      </div>
    </NCollapse>

    <!-- Conseil driver -->
    <div class="driver-tip">
      <Cpu :size="13" style="color:var(--accent-primary);flex-shrink:0" />
      <span>Pour les mises à jour NVIDIA/AMD, utilisez les outils officiels : <strong>GeForce Experience</strong>, <strong>AMD Software</strong> ou <strong>DDU</strong> pour une désinstallation propre.</span>
    </div>
  </div>
</template>

<style scoped>
.gpu-actions { display:flex; gap:8px; flex-wrap:wrap; margin-bottom:12px; }
.vendor-badge { font-size:10px; font-weight:700; padding:2px 7px; border:1px solid; border-radius:4px; }
.vram-score-row { display:flex; align-items:center; gap:6px; font-size:12px; color:var(--text-secondary); margin-bottom:8px; }
.driver-tip {
  display:flex; align-items:flex-start; gap:8px; padding:10px 14px;
  background:rgba(249,115,22,.06); border:1px solid rgba(249,115,22,.2);
  border-radius:var(--radius-md); font-size:12px; color:var(--text-secondary); line-height:1.5;
}
/* Live indicator */
.temp-live-dot {
  font-size:9px; font-weight:700; letter-spacing:.5px;
  color:var(--success); margin-left:8px; animation: pulse-dot 2s infinite;
}
@keyframes pulse-dot { 0%,100%{opacity:1} 50%{opacity:.3} }
.temp-spin { animation: spin-anim .8s linear infinite; }
@keyframes spin-anim { to{transform:rotate(360deg)} }

/* Temp cards */
.gpu-temp-card {
  display:flex; flex-direction:column; align-items:center;
  background:var(--bg-secondary); border:1px solid var(--border);
  border-radius:10px; padding:14px 20px; gap:5px; min-width:220px;
  transition:border-color .2s, box-shadow .2s;
}
.gpu-temp-card:hover { border-color:var(--border-hover); box-shadow:0 4px 12px rgba(0,0,0,.15); }
.gpu-temp-ok   { border-color:rgba(34,197,94,.25); }
.gpu-temp-warm { border-color:rgba(245,158,11,.35); }
.gpu-temp-hot  { border-color:rgba(239,68,68,.4); box-shadow:0 0 10px rgba(239,68,68,.12); }
.gpu-temp-na   { opacity:.6; }
.gpu-temp-ok   svg { color:var(--success); }
.gpu-temp-warm svg { color:var(--warning); }
.gpu-temp-hot  svg { color:var(--danger); }
.gpu-temp-na   svg { color:var(--text-muted); }
.gpu-temp-value { font-size:28px; font-weight:700; line-height:1; }
.gpu-temp-ok   .gpu-temp-value { color:var(--success); }
.gpu-temp-warm .gpu-temp-value { color:var(--warning); }
.gpu-temp-hot  .gpu-temp-value { color:var(--danger); }
.gpu-temp-na   .gpu-temp-value { color:var(--text-muted); }
.gpu-temp-name { font-size:11px; font-weight:600; text-align:center; max-width:205px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; color:var(--text-primary); }
.gpu-temp-src  { font-size:10px; color:var(--text-muted); }
.gpu-temp-bar  { width:100%; height:4px; background:var(--border); border-radius:2px; overflow:hidden; margin-top:2px; }
.gpu-temp-fill { height:100%; border-radius:2px; transition:width .5s ease; }

/* Tool badges */
.temp-tool-badge {
  font-size:10px; font-weight:600; padding:2px 8px;
  background:rgba(99,102,241,.1); border:1px solid rgba(99,102,241,.25);
  color:#818cf8; border-radius:4px;
}
</style>
