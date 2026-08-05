<script setup lang="ts">
import { ref, onMounted } from "vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import { invoke } from "@/utils/invoke";
import { cachedInvoke } from "@/composables/useCachedInvoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NSkeleton from "@/components/ui/NSkeleton.vue";
import { useNotificationStore } from "@/stores/notifications";
import { Bluetooth, BluetoothOff, RefreshCw, Radio, Smartphone, Headphones, Laptop, Mouse, Trash2, Settings } from "lucide-vue-next";

const notify = useNotificationStore();

interface BluetoothDevice {
  name: string; address: string; device_class: string;
  paired: boolean; connected: boolean; trusted: boolean;
  rssi: number; manufacturer: string; category: string;
  device_id?: string; battery_level?: number; last_connected?: string;
}
interface BluetoothAdapter {
  name: string; address: string; enabled: boolean;
  manufacturer: string; driver_version: string;
}
interface BluetoothReport {
  adapters: BluetoothAdapter[]; devices: BluetoothDevice[];
  bt_available: boolean; error: string;
}

const report = ref<BluetoothReport | null>(null);
const loading = ref(true);
const toggling = ref(false);
const forgetting = ref<string | null>(null);

function deviceIcon(cat: string) {
  const c = cat.toLowerCase();
  if (c.includes("audio") || c.includes("headph")) return Headphones;
  if (c.includes("phone") || c.includes("mobile")) return Smartphone;
  if (c.includes("mouse") || c.includes("hid")) return Mouse;
  if (c.includes("computer")) return Laptop;
  return Radio;
}

/** Retourne la couleur CSS selon le niveau de batterie */
function batteryColor(level: number): string {
  if (level > 50) return "var(--success)";
  if (level > 20) return "var(--warning)";
  return "var(--danger)";
}

/** Retourne le nombre de barres signal selon le RSSI */
function rssiBars(rssi: number): number {
  if (rssi >= -40) return 4;
  if (rssi >= -70) return 3;
  if (rssi >= -85) return 2;
  return 1;
}

/** Formate une date ISO en "DD/MM/YYYY HH:mm" */
function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    if (isNaN(d.getTime())) return iso;
    return d.toLocaleDateString("fr-FR") + " " + d.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
  } catch {
    return iso;
  }
}

async function load() {
  loading.value = true;
  try {
    report.value = await cachedInvoke<BluetoothReport>("get_bluetooth_info");
  } catch (e: unknown) {
    notify.error("Erreur Bluetooth", String(e));
  }
  loading.value = false;
}

async function toggleBt(enable: boolean) {
  if (!enable && !(await confirm("Désactiver le Bluetooth ? Cela déconnectera tous les appareils associés.", { title: "Nitrite", kind: "warning" }))) return;
  toggling.value = true;
  try {
    await invoke("toggle_bluetooth", { enable });
    notify.success(enable ? "Bluetooth activé" : "Bluetooth désactivé");
    await load();
  } catch (e: unknown) {
    notify.error("Erreur", String(e));
  }
  toggling.value = false;
}

async function forgetDevice(d: BluetoothDevice) {
  if (!d.device_id) {
    notify.error("Impossible d'oublier", "Identifiant d'appareil manquant");
    return;
  }
  if (!(await confirm(`Oublier l'appareil "${d.name || d.address}" ? Cette action supprimera le jumelage.`, { title: "Nitrite", kind: "warning" }))) return;
  forgetting.value = d.device_id;
  try {
    const safeId = d.device_id.replace(/'/g, "''");
    // -ErrorAction Stop : sans ça un échec non-terminant sort en exit 0 → faux succès.
    const r = await invoke<{ success: boolean; stderr: string }>("run_system_command", {
      cmd: "powershell",
      args: ["-Command", `Remove-PnpDevice -InstanceId '${safeId}' -Confirm:$false -ErrorAction Stop`],
    });
    if (!r.success) throw new Error(r.stderr?.trim() || "La suppression du périphérique a échoué");
    notify.success("Appareil oublié", d.name || d.address);
    await load();
  } catch (e: unknown) {
    notify.error("Erreur", String(e));
  }
  forgetting.value = null;
}

async function openBtSettings() {
  try {
    await invoke("open_url", { url: "ms-settings:bluetooth" });
  } catch (e: unknown) {
    notify.error("Erreur", String(e));
  }
}

onMounted(load);
</script>

<template>
  <div class="bt-page">
    <div class="page-header">
      <div class="header-icon"><Bluetooth :size="22" /></div>
      <div>
        <h1>Bluetooth</h1>
        <p class="subtitle">Adaptateurs et appareils Bluetooth</p>
      </div>
      <div class="header-actions">
        <NButton variant="ghost" size="sm" @click="openBtSettings">
          <Settings :size="13" /> Paramètres BT
        </NButton>
        <NButton variant="ghost" size="sm" :loading="loading" @click="load">
          <RefreshCw :size="13" /> Actualiser
        </NButton>
      </div>
    </div>

    <div v-if="loading">
      <NSkeleton v-for="i in 3" :key="i" height="56px" style="margin-bottom:8px" />
    </div>

    <template v-else-if="report">
      <div v-if="!report.bt_available" class="no-bt-banner">
        <BluetoothOff :size="16" />
        <span v-if="report.error">Erreur de lecture Bluetooth : {{ report.error }}</span>
        <span v-else>Aucun adaptateur Bluetooth détecté sur cette machine.</span>
      </div>

      <!-- Adaptateurs -->
      <NCard v-if="report.adapters.length">
        <template #header>
          <div class="section-header"><Radio :size="15" /><span>Adaptateurs ({{ report.adapters.length }})</span></div>
        </template>
        <div class="adapters-list">
          <div v-for="a in report.adapters" :key="a.address" class="adapter-row">
            <div class="adapter-icon" :class="a.enabled ? 'on' : 'off'">
              <Bluetooth :size="16" />
            </div>
            <div class="adapter-info">
              <span class="adapter-name">{{ a.name }}</span>
              <span class="adapter-meta">{{ a.manufacturer }} · v{{ a.driver_version }} · {{ a.address }}</span>
            </div>
            <NBadge :variant="a.enabled ? 'success' : 'neutral'">{{ a.enabled ? 'Activé' : 'Désactivé' }}</NBadge>
            <div class="adapter-btns">
              <NButton v-if="!a.enabled" variant="primary" size="sm" :loading="toggling" @click="toggleBt(true)">Activer</NButton>
              <NButton v-else variant="danger" size="sm" :loading="toggling" @click="toggleBt(false)">Désactiver</NButton>
            </div>
          </div>
        </div>
      </NCard>

      <!-- Appareils -->
      <NCard>
        <template #header>
          <div class="section-header"><Bluetooth :size="15" /><span>Appareils ({{ report.devices.length }})</span></div>
        </template>

        <div v-if="report.devices.length === 0" class="empty-state">
          <Bluetooth :size="28" style="opacity:.25" />
          <p>Aucun appareil Bluetooth associé</p>
        </div>

        <div v-else class="devices-grid">
          <div v-for="d in report.devices" :key="d.address" class="device-card">
            <div class="device-icon">
              <component :is="deviceIcon(d.category)" :size="20" />
            </div>
            <div class="device-info">
              <span class="device-name">{{ d.name || 'Appareil inconnu' }}</span>
              <span class="device-addr">{{ d.address }}</span>
              <span class="device-meta">{{ d.manufacturer || d.device_class }}</span>

              <!-- Indicateur batterie -->
              <span
                v-if="d.battery_level !== undefined && d.battery_level > 0"
                class="device-battery"
                :style="{ color: batteryColor(d.battery_level) }"
              >
                🔋 {{ d.battery_level }}%
              </span>

              <!-- Barres signal RSSI -->
              <div v-if="d.rssi !== undefined && d.rssi !== 0" class="rssi-bars">
                <span
                  v-for="b in 4"
                  :key="b"
                  class="rssi-bar"
                  :class="{ active: b <= rssiBars(d.rssi) }"
                  :style="{ height: (6 + b * 3) + 'px' }"
                ></span>
                <span class="rssi-label">{{ d.rssi }} dBm</span>
              </div>

              <!-- Dernière connexion -->
              <span v-if="d.last_connected" class="device-last-conn">
                Dernière connexion : {{ formatDate(d.last_connected) }}
              </span>
            </div>
            <div class="device-right">
              <div class="device-badges">
                <NBadge v-if="d.connected" variant="success" dot>Connecté</NBadge>
                <NBadge v-if="d.paired" variant="accent">Jumelé</NBadge>
                <NBadge v-if="d.trusted" variant="neutral">Approuvé</NBadge>
              </div>
              <!-- Bouton Oublier -->
              <NButton
                v-if="d.paired && d.device_id"
                variant="danger"
                size="sm"
                :loading="forgetting === d.device_id"
                @click="forgetDevice(d)"
                style="margin-top:6px"
              >
                <Trash2 :size="12" /> Oublier
              </NButton>
            </div>
          </div>
        </div>
      </NCard>
    </template>
  </div>
</template>

<style scoped>
.bt-page { display: flex; flex-direction: column; gap: 14px; }
.page-header { display: flex; align-items: center; gap: 12px; }
.header-icon { width: 42px; height: 42px; border-radius: var(--radius-lg); background: var(--accent-muted); display: flex; align-items: center; justify-content: center; color: var(--accent-primary); flex-shrink: 0; }
h1 { font-size: 22px; font-weight: 700; }
.subtitle { font-size: 12px; color: var(--text-muted); }
.header-actions { margin-left: auto; display: flex; gap: 6px; }
.no-bt-banner { display: flex; align-items: center; gap: 10px; padding: 12px 16px; background: var(--warning-muted); border: 1px solid color-mix(in srgb, var(--warning) 30%, transparent); border-radius: var(--radius-md); font-size: 13px; color: var(--warning); }
.section-header { display: flex; align-items: center; gap: 8px; }
.adapters-list { display: flex; flex-direction: column; gap: 8px; }
.adapter-row { display: flex; align-items: center; gap: 12px; padding: 12px; background: var(--bg-tertiary); border-radius: var(--radius-md); }
.adapter-icon { width: 36px; height: 36px; border-radius: var(--radius-md); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.adapter-icon.on { background: var(--accent-muted); color: var(--accent-primary); }
.adapter-icon.off { background: var(--bg-elevated); color: var(--text-muted); }
.adapter-info { flex: 1; display: flex; flex-direction: column; gap: 2px; }
.adapter-name { font-size: 13px; font-weight: 600; color: var(--text-primary); }
.adapter-meta { font-size: 11px; color: var(--text-muted); font-family: monospace; }
.adapter-btns { flex-shrink: 0; }
.devices-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 10px; }
.device-card { padding: 14px; background: var(--bg-tertiary); border-radius: var(--radius-lg); display: flex; flex-direction: column; gap: 10px; border: 1px solid var(--border); transition: border-color var(--transition-fast); }
.device-card:hover { border-color: var(--text-muted); }
.device-icon { width: 40px; height: 40px; border-radius: var(--radius-md); background: var(--accent-muted); color: var(--accent-primary); display: flex; align-items: center; justify-content: center; }
.device-name { font-size: 13px; font-weight: 600; color: var(--text-primary); }
.device-addr { font-family: monospace; font-size: 10px; color: var(--text-muted); }
.device-meta { font-size: 11px; color: var(--text-muted); }
.device-battery { font-size: 11px; font-weight: 600; }
.device-last-conn { font-size: 10px; color: var(--text-muted); font-style: italic; }
.device-info { display: flex; flex-direction: column; gap: 3px; flex: 1; }
.device-right { display: flex; flex-direction: column; align-items: flex-end; }
.device-badges { display: flex; gap: 4px; flex-wrap: wrap; justify-content: flex-end; }
.empty-state { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 30px; color: var(--text-muted); font-size: 13px; }

/* Barres RSSI */
.rssi-bars { display: flex; align-items: flex-end; gap: 2px; }
.rssi-bar { width: 4px; border-radius: 2px 2px 0 0; background: var(--border); transition: background .2s; }
.rssi-bar.active { background: var(--accent-primary); }
.rssi-label { font-size: 10px; color: var(--text-muted); margin-left: 4px; }
</style>
