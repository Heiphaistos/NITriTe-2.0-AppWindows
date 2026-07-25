<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, shallowRef } from "vue";
import { invoke, isTauriContext } from "@/utils/invoke";
import { useNotificationStore } from "@/stores/notifications";

const LHM_URL = "https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/releases/latest";
const lhmPortableExists = ref(false);
const lhmLaunching = ref(false);

async function openLhm() {
  await invoke("open_url", { url: LHM_URL }).catch(() => window.open(LHM_URL, "_blank"));
}

async function launchLhmPortable() {
  lhmLaunching.value = true;
  try {
    await invoke("launch_lhm_portable");
    // Attendre 3s puis re-poll les capteurs
    setTimeout(() => fetchSensors(), 3000);
  } catch (e: unknown) {
    if (isTauriContext()) notify.error("LHM", (e instanceof Error ? e.message : String(e)).slice(0, 120));
  }
  lhmLaunching.value = false;
}
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import { Cpu, HardDrive, Monitor, Thermometer, Gauge, Zap, Wind, RefreshCw, Activity, AlertCircle } from "lucide-vue-next";

interface SensorReading {
  hardware: string;
  hardware_type: string;
  name: string;
  sensor_type: string;
  value: number;
  unit: string;
  min: number;
  max: number;
  source: string;
}

interface HardwareGroup {
  hardware: string;
  hardware_type: string;
  source: string;
  temps:        SensorReading[];
  fans:         SensorReading[];
  loads:        SensorReading[];
  clocks:       SensorReading[];
  voltages:     SensorReading[];
  powers:       SensorReading[];
  others:       SensorReading[];
}

const sensors  = shallowRef<SensorReading[]>([]);
const loading  = ref(false);
const lastUpdate = ref<Date | null>(null);
let   pollTimer: ReturnType<typeof setInterval> | null = null;
const notify = useNotificationStore();
const sensorsErrorShown = ref(false);

const POLL_MS = 3000;

// Group sensors by hardware component
const groups = computed<HardwareGroup[]>(() => {
  const map = new Map<string, HardwareGroup>();
  for (const s of sensors.value) {
    const key = s.hardware || "Inconnu";
    if (!map.has(key)) {
      map.set(key, {
        hardware: s.hardware, hardware_type: s.hardware_type,
        source: s.source, temps: [], fans: [], loads: [], clocks: [], voltages: [], powers: [], others: [],
      });
    }
    const g = map.get(key)!;
    switch (s.sensor_type) {
      case "Temperature": g.temps.push(s); break;
      case "Fan":         g.fans.push(s); break;
      case "Load":        g.loads.push(s); break;
      case "Clock":       g.clocks.push(s); break;
      case "Voltage":     g.voltages.push(s); break;
      case "Power":       g.powers.push(s); break;
      default:            g.others.push(s); break;
    }
  }
  return [...map.values()];
});

const hasLHM    = computed(() => sensors.value.some(s => s.source === "LibreHardwareMonitor"));
const hasSensors = computed(() => sensors.value.length > 0);

// Hardware type → icon
function hwIcon(hwType: string) {
  if (hwType.includes("CPU"))       return Cpu;
  if (hwType.includes("GPU"))       return Monitor;
  if (hwType.includes("Stockage"))  return HardDrive;
  return Activity;
}

function tempColor(t: number): string {
  if (t < 50) return "var(--success)";
  if (t < 70) return "var(--warning)";
  if (t < 85) return "hsl(30,90%,55%)";
  return "var(--danger)";
}
function tempBg(t: number): string {
  if (t < 50) return "rgba(var(--success-rgb,72,199,142),0.12)";
  if (t < 70) return "rgba(var(--warning-rgb,255,193,7),0.12)";
  return "rgba(var(--danger-rgb,255,75,75),0.12)";
}
function tempLabel(t: number): string {
  if (t < 50) return "Normal";
  if (t < 70) return "Chaud";
  if (t < 85) return "Très chaud";
  return "Critique";
}
function tempVariant(t: number): "success" | "warning" | "danger" {
  if (t < 50) return "success";
  if (t < 70) return "warning";
  return "danger";
}
function formatVal(v: number, unit: string): string {
  if (unit === "V") return v.toFixed(3);
  if (unit === "W") return v.toFixed(1);
  if (unit === "MHz") return v >= 1000 ? `${(v/1000).toFixed(2)} GHz` : `${v.toFixed(0)} MHz`;
  if (unit === "RPM") return v.toFixed(0);
  return v.toFixed(1);
}

async function fetchSensors() {
  if (loading.value) return;
  loading.value = true;
  try {
    const data = await invoke<SensorReading[]>("get_all_sensors");
    sensors.value = data;
    sensorsErrorShown.value = false;
    lastUpdate.value = new Date();
  } catch (e: unknown) {
    if (isTauriContext() && !sensorsErrorShown.value) {
      sensorsErrorShown.value = true;
      notify.warning("Capteurs", (e instanceof Error ? e.message : String(e)).slice(0, 120));
    }
  }
  loading.value = false;
}

onMounted(async () => {
  fetchSensors();
  pollTimer = setInterval(fetchSensors, POLL_MS);
  lhmPortableExists.value = await invoke<boolean>("check_lhm_portable").catch(() => false);
});
onUnmounted(() => { if (pollTimer) clearInterval(pollTimer); });
</script>

<template>
  <div class="temp-page">
    <div class="page-header">
      <div class="header-icon"><Thermometer :size="22" /></div>
      <div>
        <h1>Températures & Capteurs</h1>
        <p class="subtitle">
          <span v-if="hasLHM" class="lhm-badge">● LibreHardwareMonitor détecté — données complètes</span>
          <span v-else class="lhm-badge warn">● LHM non détecté — données partielles</span>
        </p>
      </div>
      <NButton variant="ghost" size="sm" :loading="loading" @click="fetchSensors" style="margin-left:auto">
        <RefreshCw :size="13" /> Actualiser
      </NButton>
    </div>

    <!-- Bannière installation LHM -->
    <div v-if="!hasLHM" class="lhm-install-banner">
      <AlertCircle :size="16" style="flex-shrink:0;color:var(--warning)" />
      <div style="flex:1">
        <strong>LibreHardwareMonitor requis pour les données complètes</strong><br>
        <span v-if="lhmPortableExists">
          LHM portable détecté dans <code>logiciel/LibreHardwareMonitor/</code>. Lancez-le en tant qu'administrateur pour activer les capteurs.
        </span>
        <span v-else>
          Placez <code>LibreHardwareMonitor.exe</code> dans <code>logiciel/LibreHardwareMonitor/</code>
          ou <a href="#" @click.prevent="openLhm">téléchargez-le ici</a>.
          Il fournit : températures par cœur, ventilateurs RPM, tensions CPU/GPU, fréquences d'horloge, consommation électrique.
        </span>
      </div>
      <NButton
        v-if="lhmPortableExists"
        variant="primary"
        size="sm"
        :loading="lhmLaunching"
        @click="launchLhmPortable"
        style="flex-shrink:0"
      >
        <Zap :size="13" /> Lancer LHM (admin)
      </NButton>
    </div>

    <!-- Aucune donnée -->
    <NCard v-if="!hasSensors && !loading">
      <div class="empty-state">
        <Thermometer :size="32" style="opacity:0.3" />
        <p>Aucune donnée de capteur disponible.<br>Installez LibreHardwareMonitor (admin) pour les données complètes.</p>
      </div>
    </NCard>

    <!-- Groupes par composant matériel -->
    <div v-for="group in groups" :key="group.hardware" class="hw-group">
      <NCard>
        <template #header>
          <div class="group-header">
            <component :is="hwIcon(group.hardware_type)" :size="16" style="color:var(--accent-primary)" />
            <span class="group-name">{{ group.hardware }}</span>
            <NBadge variant="neutral" size="sm" style="font-size:10px">{{ group.hardware_type }}</NBadge>
            <span class="group-source muted">{{ group.source }}</span>
          </div>
        </template>

        <div class="sensors-layout">

          <!-- Températures -->
          <div v-if="group.temps.length" class="sensor-section">
            <div class="sensor-section-label"><Thermometer :size="13" /> Températures</div>
            <div class="temp-grid">
              <div v-for="s in group.temps" :key="s.name" class="temp-card" :style="{ borderColor: tempColor(s.value), background: tempBg(s.value) }">
                <div class="temp-name">{{ s.name }}</div>
                <div class="temp-val" :style="{ color: tempColor(s.value) }">{{ s.value.toFixed(1) }}°C</div>
                <div class="temp-meta">
                  <span class="muted" v-if="s.min > 0">min {{ s.min.toFixed(0) }}°</span>
                  <span class="muted" v-if="s.max > 0">max {{ s.max.toFixed(0) }}°</span>
                  <NBadge :variant="tempVariant(s.value)" size="sm">{{ tempLabel(s.value) }}</NBadge>
                </div>
                <div class="temp-bar">
                  <div class="temp-bar-fill" :style="{ width: Math.min(s.max > 0 ? s.value/s.max*100 : s.value/110*100, 100)+'%', background: tempColor(s.value) }"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- Charges / Load -->
          <div v-if="group.loads.length" class="sensor-section">
            <div class="sensor-section-label"><Activity :size="13" /> Charge</div>
            <div class="load-grid">
              <div v-for="s in group.loads" :key="s.name" class="load-row">
                <span class="load-name">{{ s.name }}</span>
                <div class="load-bar-wrap">
                  <div class="load-bar">
                    <div class="load-bar-fill" :style="{ width: Math.min(s.value,100)+'%', background: s.value>85?'var(--danger)':s.value>60?'var(--warning)':'var(--accent-primary)' }"></div>
                  </div>
                </div>
                <span class="load-val">{{ s.value.toFixed(1) }}%</span>
              </div>
            </div>
          </div>

          <!-- Ventilateurs -->
          <div v-if="group.fans.length" class="sensor-section">
            <div class="sensor-section-label"><Wind :size="13" /> Ventilateurs</div>
            <div class="info-rows">
              <div v-for="s in group.fans" :key="s.name" class="info-row">
                <span>{{ s.name }}</span>
                <span class="mono">{{ formatVal(s.value, s.unit) }} RPM</span>
              </div>
            </div>
          </div>

          <!-- Horloges -->
          <div v-if="group.clocks.length" class="sensor-section">
            <div class="sensor-section-label"><Gauge :size="13" /> Fréquences</div>
            <div class="info-rows">
              <div v-for="s in group.clocks" :key="s.name" class="info-row">
                <span>{{ s.name }}</span>
                <span class="mono">{{ formatVal(s.value, s.unit) }}</span>
              </div>
            </div>
          </div>

          <!-- Tensions -->
          <div v-if="group.voltages.length" class="sensor-section">
            <div class="sensor-section-label"><Zap :size="13" /> Tensions</div>
            <div class="info-rows">
              <div v-for="s in group.voltages" :key="s.name" class="info-row">
                <span>{{ s.name }}</span>
                <span class="mono">{{ formatVal(s.value, s.unit) }} V</span>
              </div>
            </div>
          </div>

          <!-- Puissances -->
          <div v-if="group.powers.length" class="sensor-section">
            <div class="sensor-section-label"><Zap :size="13" /> Consommation</div>
            <div class="info-rows">
              <div v-for="s in group.powers" :key="s.name" class="info-row">
                <span>{{ s.name }}</span>
                <span class="mono">{{ formatVal(s.value, s.unit) }} W</span>
              </div>
            </div>
          </div>

          <!-- Autres -->
          <div v-if="group.others.length" class="sensor-section">
            <div class="sensor-section-label"><Activity :size="13" /> Autres</div>
            <div class="info-rows">
              <div v-for="s in group.others" :key="s.name" class="info-row">
                <span>{{ s.name }}</span>
                <span class="mono">{{ formatVal(s.value, s.unit) }} {{ s.unit }}</span>
              </div>
            </div>
          </div>

        </div>
      </NCard>
    </div>

    <div v-if="lastUpdate" class="update-footer muted">
      Dernière mise à jour : {{ lastUpdate.toLocaleTimeString("fr-FR") }} · Actualisation toutes les {{ POLL_MS/1000 }}s
    </div>
  </div>
</template>

<style scoped>
.temp-page { display: flex; flex-direction: column; gap: 16px; }
.page-header { display: flex; align-items: center; gap: 12px; }
.header-icon { width: 42px; height: 42px; border-radius: var(--radius-lg); background: var(--danger-muted, rgba(255,75,75,.12)); display: flex; align-items: center; justify-content: center; color: var(--danger); flex-shrink: 0; }
h1 { font-size: 22px; font-weight: 700; }
.subtitle { font-size: 12px; color: var(--text-muted); }
.lhm-badge { font-size: 11px; }
.lhm-badge.warn { color: var(--warning); }

.lhm-install-banner { display: flex; gap: 12px; align-items: flex-start; padding: 14px 16px; background: rgba(255,193,7,.08); border: 1px solid rgba(255,193,7,.3); border-radius: var(--radius-xl); font-size: 12px; line-height: 1.6; }
.lhm-install-banner a { color: var(--accent-primary); }

.empty-state { display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 40px; color: var(--text-muted); font-size: 13px; text-align: center; }

.hw-group { display: flex; flex-direction: column; }
.group-header { display: flex; align-items: center; gap: 8px; width: 100%; }
.group-name { font-weight: 600; font-size: 14px; }
.group-source { font-size: 11px; margin-left: auto; }

.sensors-layout { display: flex; flex-direction: column; gap: 18px; }
.sensor-section { display: flex; flex-direction: column; gap: 8px; }
.sensor-section-label { display: flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: .06em; color: var(--text-muted); padding-bottom: 4px; border-bottom: 1px solid var(--border); }

/* Températures en cartes */
.temp-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px,1fr)); gap: 8px; }
.temp-card { padding: 10px 12px; border-radius: var(--radius-lg); border: 1px solid; display: flex; flex-direction: column; gap: 4px; }
.temp-name { font-size: 11px; color: var(--text-muted); line-height: 1.3; }
.temp-val { font-size: 24px; font-weight: 900; font-family: "JetBrains Mono", monospace; line-height: 1; }
.temp-meta { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; font-size: 10px; }
.temp-bar { height: 3px; background: var(--border); border-radius: 2px; overflow: hidden; margin-top: 4px; }
.temp-bar-fill { height: 100%; border-radius: 2px; transition: width .3s, background .3s; }

/* Load bars */
.load-grid { display: flex; flex-direction: column; gap: 5px; }
.load-row { display: grid; grid-template-columns: 160px 1fr 45px; align-items: center; gap: 8px; }
.load-name { font-size: 12px; color: var(--text-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.load-bar-wrap { min-width: 0; }
.load-bar { height: 6px; background: var(--bg-tertiary); border-radius: 3px; overflow: hidden; }
.load-bar-fill { height: 100%; border-radius: 3px; transition: width .4s, background .4s; }
.load-val { font-size: 11px; font-family: "JetBrains Mono", monospace; text-align: right; color: var(--text-secondary); }

/* Info rows (fans, clocks, voltages) */
.info-rows { display: flex; flex-direction: column; gap: 0; }
.info-row { display: flex; justify-content: space-between; align-items: center; gap: 8px; padding: 4px 0; border-bottom: 1px solid var(--border); font-size: 12px; }
.info-row:last-child { border-bottom: none; }
/* Noms de capteurs LibreHardwareMonitor parfois très longs — tronquer au
   lieu de déborder (flex-basis:auto par défaut refuse de rétrécir sous la
   largeur du contenu, sans min-width:0 le texte pousse hors du conteneur). */
.info-row span:first-child { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.mono { font-family: "JetBrains Mono", monospace; color: var(--text-secondary); flex-shrink: 0; }

.muted { color: var(--text-muted); }
.update-footer { font-size: 11px; text-align: center; padding: 8px 0; }
</style>
