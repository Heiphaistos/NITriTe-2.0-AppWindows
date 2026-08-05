<template>
  <div class="bt-root">
    <!-- Banner -->
    <div class="bt-banner">
      <div class="bt-banner-icon"><Bluetooth :size="26" /></div>
      <div class="bt-banner-text">
        <div class="bt-banner-title">Bluetooth</div>
        <div class="bt-banner-desc">Adaptateurs, périphériques associés et état de connexion</div>
      </div>
      <div style="display:flex;gap:8px;align-items:center">
        <button class="bt-btn bt-btn-primary" :disabled="loading" @click="load"><RefreshCw :size="13" /> Actualiser</button>
        <span v-if="msg" :class="msgErr ? 'bt-msg-err' : 'bt-msg-ok'">{{ msg }}</span>
      </div>
    </div>

    <div v-if="loading" class="bt-loading"><div class="bt-spinner" /> Lecture des périphériques Bluetooth...</div>

    <div v-else-if="report">
      <!-- Not available -->
      <div v-if="!report.bt_available" class="bt-unavailable">
        <div class="bt-unavail-icon"><Bluetooth :size="32" /></div>
        <div>
          <div class="bt-unavail-title">Bluetooth non disponible</div>
          <div class="bt-unavail-desc">{{ report.error || 'Aucun adaptateur Bluetooth détecté sur ce système.' }}</div>
        </div>
      </div>

      <div v-else>
        <!-- Stats -->
        <div class="bt-stats">
          <div class="bt-stat bt-stat-blue">
            <div class="bt-stat-val">{{ report.adapters.length }}</div>
            <div class="bt-stat-lbl">Adaptateurs</div>
          </div>
          <div class="bt-stat bt-stat-green">
            <div class="bt-stat-val">{{ report.adapters.filter(a => a.enabled).length }}</div>
            <div class="bt-stat-lbl">Actifs</div>
          </div>
          <div class="bt-stat bt-stat-purple">
            <div class="bt-stat-val">{{ report.devices.length }}</div>
            <div class="bt-stat-lbl">Périphériques</div>
          </div>
          <div class="bt-stat bt-stat-teal">
            <div class="bt-stat-val">{{ report.devices.filter(d => d.connected).length }}</div>
            <div class="bt-stat-lbl">Connectés</div>
          </div>
        </div>

        <!-- Adapters -->
        <div class="bt-section-title"><Cpu :size="14" /> Adaptateurs ({{ report.adapters.length }})</div>
        <div class="bt-adapters">
          <div v-for="a in report.adapters" :key="a.name" class="bt-adapter">
            <div class="bt-adapter-led" :class="a.enabled ? 'led-on' : 'led-off'">
              <div class="bt-led-dot" />
            </div>
            <div class="bt-adapter-body">
              <div class="bt-adapter-name">{{ a.name }}</div>
              <div class="bt-adapter-meta">
                <span v-if="a.manufacturer" style="opacity:.6">{{ a.manufacturer }}</span>
                <span v-if="a.address" class="bt-addr-chip">{{ a.address }}</span>
                <span :class="a.enabled ? 'status-on' : 'status-off'">{{ a.enabled ? '● Actif' : '○ Inactif' }}</span>
              </div>
            </div>
            <div class="bt-adapter-actions">
              <button class="bt-btn bt-btn-sm bt-btn-green" @click="toggleBt(true)">Activer</button>
              <button class="bt-btn bt-btn-sm bt-btn-gray" @click="toggleBt(false)">Désactiver</button>
            </div>
          </div>
        </div>

        <!-- Devices -->
        <div class="bt-section-title"><Smartphone :size="14" /> Périphériques associés ({{ report.devices.length }})</div>
        <div v-if="report.devices.length === 0" class="bt-empty">
          <Bluetooth :size="24" style="opacity:.2" />
          <span>Aucun périphérique Bluetooth associé</span>
        </div>
        <div v-else class="bt-devices">
          <div v-for="d in report.devices" :key="d.address" class="bt-device">
            <div class="bt-device-emoji">{{ catIcon(d.category) }}</div>
            <div class="bt-device-body">
              <div class="bt-device-name">{{ sanitize(d.name) || d.name }}</div>
              <div class="bt-device-meta">
                <span class="bt-cat-chip">{{ sanitize(d.category) || d.category }}</span>
                <span v-if="d.manufacturer" style="font-size:11px;opacity:.5">{{ d.manufacturer }}</span>
                <span v-if="d.address" class="bt-addr-chip">{{ d.address }}</span>
              </div>
            </div>
            <div class="bt-device-right">
              <span :class="d.connected ? 'status-on' : 'status-off'" style="font-size:12px;font-weight:500">
                {{ d.connected ? '● Connecté' : '○ Déconnecté' }}
              </span>
              <span v-if="d.paired" class="bt-paired-badge">Associé</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from "@/utils/invoke";
import { cachedInvoke } from '@/composables/useCachedInvoke'
import { Bluetooth, RefreshCw, Cpu, Smartphone } from 'lucide-vue-next'

interface BluetoothAdapter { name: string; address: string; enabled: boolean; manufacturer: string; driver_version: string }
interface BluetoothDevice { name: string; address: string; device_class: string; paired: boolean; connected: boolean; trusted: boolean; rssi: number; manufacturer: string; category: string }
interface BluetoothReport { adapters: BluetoothAdapter[]; devices: BluetoothDevice[]; bt_available: boolean; error: string }

const loading = ref(false); const report = ref<BluetoothReport | null>(null)
const msg = ref(''); const msgErr = ref(false)

function showMsg(t: string, err = false) { msg.value = t; msgErr.value = err; setTimeout(() => { msg.value = '' }, 3000) }

async function load() { loading.value = true; try { report.value = await cachedInvoke<BluetoothReport>('get_bluetooth_info') } catch (e) { showMsg(String(e), true) } finally { loading.value = false } }

let btReloadTimer: ReturnType<typeof setTimeout> | null = null;
async function toggleBt(enable: boolean) {
  try { const r = await invoke<string>('toggle_bluetooth', { enable }); showMsg(r); btReloadTimer = setTimeout(load, 1500) }
  catch(e) { showMsg(String(e), true) }
}

function sanitize(s: string): string {
  // Remove non-printable / non-ASCII characters
  return s ? s.replace(/[^\x20-\x7EÀ-ÿ]/g, '').trim() : '';
}

function catIcon(cat: string): string {
  const clean = sanitize(cat);
  const m: Record<string, string> = {
    'Audio': '🎧', 'Souris': '🖱️', 'Mouse': '🖱️',
    'Clavier': '⌨️', 'Keyboard': '⌨️',
    'Manette': '🎮', 'Gamepad': '🎮', 'Controller': '🎮',
    'T\u00e9l\u00e9phone': '📱', 'Phone': '📱', 'Smartphone': '📱',
    'Casque': '🎧', 'Headset': '🎧',
    'Autre': '📡', 'Other': '📡',
  };
  return m[clean] || m[cat] || '📡';
}

onMounted(load)
onUnmounted(() => { if (btReloadTimer) clearTimeout(btReloadTimer); })
</script>

<style scoped>
.bt-root { display: flex; flex-direction: column; gap: 14px; }

.bt-banner { display: flex; align-items: center; gap: 16px; padding: 18px 22px;
  background: linear-gradient(135deg, rgba(59,130,246,.13), rgba(37,99,235,.07));
  border: 1px solid rgba(59,130,246,.3); border-radius: 14px; }
.bt-banner-icon { width: 48px; height: 48px; border-radius: 12px;
  background: linear-gradient(135deg,#3b82f6,#1d4ed8); display: flex; align-items: center;
  justify-content: center; color: #fff; flex-shrink: 0; box-shadow: 0 4px 14px rgba(59,130,246,.4); }
.bt-banner-text { flex: 1; }
.bt-banner-title { font-size: 17px; font-weight: 700; margin-bottom: 3px; }
.bt-banner-desc { font-size: 12px; opacity: .7; }
.bt-msg-ok  { font-size: 12px; color: #22c55e; background: rgba(34,197,94,.1);  padding: 4px 10px; border-radius: 6px; }
.bt-msg-err { font-size: 12px; color: #ef4444; background: rgba(239,68,68,.1); padding: 4px 10px; border-radius: 6px; }

.bt-loading { display: flex; align-items: center; gap: 10px; padding: 20px; font-size: 13px; color: var(--text-muted); }
.bt-spinner { width: 15px; height: 15px; border: 2px solid rgba(255,255,255,.15); border-top-color: #3b82f6; border-radius: 50%; animation: spin .8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.bt-unavailable { display: flex; align-items: center; gap: 20px; padding: 28px;
  background: rgba(59,130,246,.07); border: 1px solid rgba(59,130,246,.2); border-radius: 14px; }
.bt-unavail-icon { width: 56px; height: 56px; border-radius: 14px; background: rgba(59,130,246,.15);
  color: #3b82f6; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.bt-unavail-title { font-size: 15px; font-weight: 600; margin-bottom: 5px; }
.bt-unavail-desc { font-size: 12px; opacity: .6; }

.bt-stats { display: grid; grid-template-columns: repeat(4,1fr); gap: 10px; }
.bt-stat { border-radius: 12px; padding: 16px; text-align: center; border: 1px solid transparent; }
.bt-stat-blue   { background: rgba(59,130,246,.1);  border-color: rgba(59,130,246,.25); }
.bt-stat-green  { background: rgba(34,197,94,.1);   border-color: rgba(34,197,94,.25); }
.bt-stat-purple { background: rgba(124,58,237,.1);  border-color: rgba(124,58,237,.25); }
.bt-stat-teal   { background: rgba(20,184,166,.1);  border-color: rgba(20,184,166,.25); }
.bt-stat-val { font-size: 24px; font-weight: 700; }
.bt-stat-lbl { font-size: 10px; opacity: .5; text-transform: uppercase; margin-top: 2px; }

.bt-section-title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 600; opacity: .7; }

.bt-adapters { display: flex; flex-direction: column; gap: 6px; }
.bt-adapter { display: flex; align-items: center; gap: 12px; padding: 14px 16px;
  background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 12px; transition: border-color 150ms; }
.bt-adapter:hover { border-color: rgba(59,130,246,.3); }
.bt-adapter-led { width: 36px; height: 36px; border-radius: 50%; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.led-on  { background: rgba(34,197,94,.15); box-shadow: 0 0 10px rgba(34,197,94,.2); }
.led-off { background: var(--bg-tertiary); }
.bt-led-dot { width: 10px; height: 10px; border-radius: 50%; background: currentColor; }
.led-on  .bt-led-dot { color: #22c55e; box-shadow: 0 0 6px #22c55e; }
.led-off .bt-led-dot { color: #6b7280; }
.bt-adapter-body { flex: 1; }
.bt-adapter-name { font-size: 13px; font-weight: 600; margin-bottom: 4px; }
.bt-adapter-meta { display: flex; align-items: center; gap: 10px; font-size: 11px; }
.bt-adapter-actions { display: flex; gap: 6px; }

.bt-empty { display: flex; align-items: center; gap: 12px; padding: 22px; font-size: 12px;
  color: var(--text-muted); background: var(--bg-secondary); border-radius: 12px; border: 1px solid var(--border); }

.bt-devices { display: flex; flex-direction: column; gap: 6px; }
.bt-device { display: flex; align-items: center; gap: 12px; padding: 12px 16px;
  background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 12px; transition: border-color 150ms; }
.bt-device:hover { border-color: rgba(59,130,246,.3); }
.bt-device-emoji { font-size: 22px; min-width: 32px; text-align: center; }
.bt-device-body { flex: 1; }
.bt-device-name { font-size: 13px; font-weight: 600; margin-bottom: 4px; }
.bt-device-meta { display: flex; align-items: center; gap: 8px; font-size: 11px; flex-wrap: wrap; }
.bt-device-right { display: flex; flex-direction: column; align-items: flex-end; gap: 4px; }

.bt-cat-chip { background: rgba(124,58,237,.12); color: #a78bfa; border: 1px solid rgba(124,58,237,.25); border-radius: 4px; padding: 1px 7px; font-size: 10px; }
.bt-addr-chip { font-family: 'JetBrains Mono',monospace; font-size: 10px; opacity: .4; background: var(--bg-tertiary); padding: 1px 6px; border-radius: 4px; }
.bt-paired-badge { font-size: 10px; background: rgba(59,130,246,.12); color: #60a5fa; border: 1px solid rgba(59,130,246,.25); border-radius: 4px; padding: 2px 7px; }

.status-on  { color: #22c55e; }
.status-off { color: #6b7280; }

.bt-btn { display: inline-flex; align-items: center; gap: 5px; padding: 8px 14px; border-radius: 8px;
  border: 1px solid var(--border); background: var(--bg-secondary); color: var(--text-secondary);
  font-size: 12px; cursor: pointer; transition: all 150ms; font-family: inherit; }
.bt-btn:disabled { opacity: .4; cursor: not-allowed; }
.bt-btn-primary { background: rgba(59,130,246,.15); color: #60a5fa; border-color: rgba(59,130,246,.3); }
.bt-btn-primary:hover:not(:disabled) { background: rgba(59,130,246,.25); }
.bt-btn-sm { padding: 5px 10px; font-size: 11px; }
.bt-btn-green { color: #22c55e; border-color: rgba(34,197,94,.3); }
.bt-btn-green:hover { background: rgba(34,197,94,.1); }
.bt-btn-gray { color: var(--text-muted); }
.bt-btn-gray:hover { border-color: var(--text-muted); color: var(--text-primary); }
</style>
