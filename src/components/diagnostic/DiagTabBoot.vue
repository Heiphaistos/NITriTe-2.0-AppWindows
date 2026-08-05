<template>
  <div class="boot-root">
    <!-- Banner -->
    <div class="boot-banner">
      <div class="boot-banner-icon"><Settings :size="26" /></div>
      <div class="boot-banner-text">
        <div class="boot-banner-title">Gestionnaire de Démarrage</div>
        <div class="boot-banner-desc">Configurez le timeout, l'entrée par défaut et accédez à WinRE</div>
      </div>
      <div class="boot-banner-right">
        <button class="boot-btn boot-btn-primary" :disabled="loading" @click="load">
          <RefreshCw :size="13" /> Actualiser
        </button>
        <span v-if="msg" :class="msgErr ? 'boot-err' : 'boot-ok'">{{ msg }}</span>
      </div>
    </div>

    <div v-if="loading" class="boot-loading"><div class="boot-spinner" />Lecture de la configuration BCD...</div>

    <div v-else-if="config">
      <!-- Settings panel -->
      <div class="boot-settings-panel">
        <div class="boot-settings-row">
          <div class="boot-settings-label">
            <Clock :size="14" style="color:#7c3aed" />
            <span>Timeout de démarrage</span>
          </div>
          <div style="display:flex;align-items:center;gap:10px">
            <input v-model.number="timeout" type="range" min="0" max="60" class="boot-range" />
            <input v-model.number="timeout" type="number" min="0" max="999" class="boot-num-input" />
            <span style="font-size:12px;opacity:.6">secondes</span>
            <button class="boot-btn" @click="saveTimeout">Appliquer</button>
          </div>
        </div>
        <div class="boot-settings-row">
          <div class="boot-settings-label">
            <Star :size="14" style="color:#f59e0b" />
            <span>Entrée par défaut</span>
          </div>
          <code class="boot-mono">{{ config.default_id || '(non défini)' }}</code>
        </div>
      </div>

      <!-- Entries -->
      <div class="boot-section-title"><List :size="15" /> Entrées de démarrage ({{ config.entries.length }})</div>
      <div class="boot-entries">
        <div v-for="e in config.entries" :key="e.id" class="boot-entry" :class="e.is_default ? 'boot-default' : ''">
          <div class="boot-entry-left">
            <div class="boot-entry-icon" :class="e.is_default ? 'icon-default' : 'icon-normal'">
              <Star v-if="e.is_default" :size="16" />
              <Monitor v-else :size="16" />
            </div>
          </div>
          <div class="boot-entry-body">
            <div class="boot-entry-name">{{ e.description || e.id }}</div>
            <div class="boot-entry-details">
              <code class="boot-mini-code">{{ e.id }}</code>
              <span v-if="e.path" class="boot-detail-chip">{{ e.path }}</span>
              <span v-if="e.device" class="boot-detail-chip">{{ e.device }}</span>
            </div>
          </div>
          <div class="boot-entry-right">
            <span v-if="e.is_default" class="boot-badge-default"><Star :size="11" /> Par défaut</span>
            <button v-else class="boot-btn boot-btn-sm" @click="setDefault(e.id)">Définir par défaut</button>
          </div>
        </div>
      </div>

      <!-- Danger -->
      <div class="boot-danger-zone">
        <div class="boot-danger-header"><AlertTriangle :size="15" /> Actions Avancées</div>
        <div class="boot-danger-body">
          <div>
            <div class="boot-danger-title">Redémarrer en mode Récupération (WinRE)</div>
            <div class="boot-danger-desc">Redémarrage immédiat vers les options avancées Windows. Sauvegardez votre travail.</div>
          </div>
          <button class="boot-btn boot-btn-danger" @click="confirmRecovery = true">
            <Power :size="14" /> Redémarrer
          </button>
        </div>
      </div>
    </div>

    <!-- Confirm overlay -->
    <Teleport to="body">
      <div v-if="confirmRecovery" class="boot-overlay" @click.self="confirmRecovery = false">
        <div class="boot-confirm">
          <div class="boot-confirm-icon"><AlertTriangle :size="36" /></div>
          <div class="boot-confirm-title">Confirmer le redémarrage</div>
          <div class="boot-confirm-desc">Le PC va redémarrer maintenant en mode récupération. Toutes les applications non sauvegardées seront perdues.</div>
          <div class="boot-confirm-actions">
            <button class="boot-btn" @click="confirmRecovery = false">Annuler</button>
            <button class="boot-btn boot-btn-danger" @click="doRecovery">Confirmer le redémarrage</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from "@/utils/invoke";
import { Settings, RefreshCw, List, Star, Monitor, AlertTriangle, Clock, Power } from 'lucide-vue-next'

interface BcdEntry { id: string; description: string; entry_type: string; device: string; path: string; locale: string; is_default: boolean }
interface BootConfig { entries: BcdEntry[]; default_id: string; timeout_secs: number; safe_mode: boolean; debug_mode: boolean }

const loading = ref(false); const config = ref<BootConfig | null>(null)
const timeout = ref(30); const msg = ref(''); const msgErr = ref(false)
const confirmRecovery = ref(false)

function showMsg(t: string, err = false) { msg.value = t; msgErr.value = err; setTimeout(() => { msg.value = '' }, 3000) }

async function load() {
  loading.value = true
  try { config.value = await invoke<BootConfig>('get_boot_config'); timeout.value = config.value.timeout_secs }
  catch(e) { showMsg(String(e), true) }
  finally { loading.value = false }
}
async function saveTimeout() {
  try { showMsg(await invoke<string>('set_boot_timeout', { seconds: timeout.value })) }
  catch(e) { showMsg(String(e), true) }
}
async function setDefault(entryId: string) {
  try { showMsg(await invoke<string>('set_default_boot', { entryId })); await load() }
  catch(e) { showMsg(String(e), true) }
}
async function doRecovery() { confirmRecovery.value = false; try { await invoke<string>('boot_to_recovery') } catch(e) { showMsg(String(e), true) } }

onMounted(load)
</script>

<style scoped>
.boot-root { display: flex; flex-direction: column; gap: 14px; }

.boot-banner { display: flex; align-items: center; gap: 16px; padding: 20px 22px;
  background: linear-gradient(135deg, rgba(245,158,11,.15), rgba(234,88,12,.08));
  border: 1px solid rgba(245,158,11,.3); border-radius: 14px; }
.boot-banner-icon { width: 50px; height: 50px; border-radius: 12px;
  background: linear-gradient(135deg,#f59e0b,#ea580c); display: flex; align-items: center;
  justify-content: center; color: #fff; flex-shrink: 0; box-shadow: 0 4px 16px rgba(245,158,11,.4); }
.boot-banner-text { flex: 1; }
.boot-banner-title { font-size: 17px; font-weight: 700; margin-bottom: 2px; }
.boot-banner-desc { font-size: 12px; opacity: .6; }
.boot-banner-right { display: flex; align-items: center; gap: 10px; }

.boot-loading { display: flex; align-items: center; gap: 10px; padding: 20px; font-size: 13px; color: var(--text-muted); }
.boot-spinner { width: 16px; height: 16px; border: 2px solid rgba(255,255,255,.15); border-top-color: #f59e0b; border-radius: 50%; animation: spin .8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.boot-settings-panel { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 12px; overflow: hidden; }
.boot-settings-row { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 14px 18px; border-bottom: 1px solid var(--border); }
.boot-settings-row:last-child { border-bottom: none; }
.boot-settings-label { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 500; min-width: 200px; }
.boot-range { accent-color: #f59e0b; width: 120px; }
.boot-num-input { width: 60px; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 6px; padding: 4px 8px; color: var(--text-primary); font-size: 12px; text-align: center; outline: none; }
.boot-mono { font-family: 'JetBrains Mono', monospace; font-size: 11px; background: var(--bg-tertiary); padding: 4px 10px; border-radius: 6px; color: var(--text-muted); }

.boot-section-title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 600; opacity: .7; }
.boot-entries { display: flex; flex-direction: column; gap: 6px; }
.boot-entry { display: flex; align-items: center; gap: 12px; padding: 14px 16px;
  background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 12px; transition: border-color 150ms; }
.boot-entry:hover { border-color: rgba(245,158,11,.3); }
.boot-default { border-color: rgba(245,158,11,.4) !important; background: rgba(245,158,11,.05) !important; }
.boot-entry-icon { width: 38px; height: 38px; border-radius: 9px; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.icon-default { background: rgba(245,158,11,.15); color: #f59e0b; }
.icon-normal  { background: var(--bg-tertiary); color: var(--text-muted); }
.boot-entry-body { flex: 1; min-width: 0; }
.boot-entry-name { font-size: 13px; font-weight: 600; margin-bottom: 5px; }
.boot-entry-details { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.boot-mini-code { font-family: 'JetBrains Mono', monospace; font-size: 10px; color: var(--text-muted); background: var(--bg-tertiary); padding: 1px 6px; border-radius: 4px; }
.boot-detail-chip { font-size: 10px; background: var(--bg-tertiary); color: var(--text-muted); padding: 2px 7px; border-radius: 5px; }
.boot-entry-right { display: flex; align-items: center; }
.boot-badge-default { display: flex; align-items: center; gap: 4px; background: rgba(245,158,11,.15); color: #f59e0b; border: 1px solid rgba(245,158,11,.3); border-radius: 6px; padding: 4px 10px; font-size: 11px; font-weight: 600; }

.boot-danger-zone { background: rgba(239,68,68,.05); border: 1px solid rgba(239,68,68,.2); border-radius: 12px; overflow: hidden; }
.boot-danger-header { display: flex; align-items: center; gap: 8px; padding: 12px 16px; font-size: 12px; font-weight: 600; color: #ef4444; border-bottom: 1px solid rgba(239,68,68,.15); background: rgba(239,68,68,.05); }
.boot-danger-body { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 14px 16px; }
.boot-danger-title { font-size: 13px; font-weight: 500; margin-bottom: 3px; }
.boot-danger-desc { font-size: 11px; opacity: .6; }

.boot-overlay { position: fixed; inset: 0; background: rgba(0,0,0,.7); display: flex; align-items: center; justify-content: center; z-index: 9999; backdrop-filter: blur(4px); }
.boot-confirm { background: var(--bg-primary); border: 1px solid rgba(239,68,68,.4); border-radius: 16px; padding: 28px; text-align: center; max-width: 380px; box-shadow: 0 20px 60px rgba(0,0,0,.5); }
.boot-confirm-icon { color: #ef4444; margin-bottom: 12px; }
.boot-confirm-title { font-size: 18px; font-weight: 700; margin-bottom: 8px; }
.boot-confirm-desc { font-size: 12px; opacity: .6; line-height: 1.6; margin-bottom: 20px; }
.boot-confirm-actions { display: flex; justify-content: center; gap: 10px; }

.boot-btn { display: inline-flex; align-items: center; gap: 6px; padding: 8px 16px; border-radius: 8px; border: 1px solid var(--border); background: var(--bg-secondary); color: var(--text-secondary); font-size: 12px; cursor: pointer; transition: all 150ms; font-family: inherit; }
.boot-btn:hover { color: var(--text-primary); border-color: var(--text-muted); }
.boot-btn-primary { background: rgba(245,158,11,.15); color: #f59e0b; border-color: rgba(245,158,11,.3); }
.boot-btn-sm { padding: 5px 12px; font-size: 11px; }
.boot-btn-danger { background: #ef4444; color: #fff; border-color: #ef4444; }
.boot-btn-danger:hover { background: #dc2626; border-color: #dc2626; color: #fff; }
.boot-ok  { font-size: 12px; color: #22c55e; }
.boot-err { font-size: 12px; color: #ef4444; }
</style>
