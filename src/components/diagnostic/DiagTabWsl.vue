<template>
  <div class="wsl-root">
    <!-- Banner -->
    <div class="wsl-banner">
      <div class="wsl-banner-icon"><Terminal :size="26" /></div>
      <div class="wsl-banner-text">
        <div class="wsl-banner-title">WSL — Windows Subsystem for Linux</div>
        <div class="wsl-banner-desc">Gérez vos distributions Linux et exécutez des commandes</div>
      </div>
      <div style="display:flex;gap:8px;align-items:center">
        <button class="wsl-btn wsl-btn-primary" :disabled="loading" @click="load"><RefreshCw :size="13" /> Actualiser</button>
        <span v-if="versionMsg" class="wsl-msg-ok">{{ versionMsg }}</span>
      </div>
    </div>

    <div v-if="loading" class="wsl-loading"><div class="wsl-spinner" /> Lecture des informations WSL...</div>

    <div v-else-if="info">
      <!-- Not installed -->
      <div v-if="!info.installed" class="wsl-unavailable">
        <div class="wsl-unavail-icon"><Terminal :size="32" /></div>
        <div>
          <div class="wsl-unavail-title">WSL non disponible</div>
          <div class="wsl-unavail-desc">{{ info.error || "WSL n'est pas installé sur ce système." }}</div>
          <div class="wsl-unavail-hint">Pour installer WSL : <code class="wsl-code">wsl --install</code> dans un terminal administrateur</div>
        </div>
      </div>

      <div v-else>
        <!-- Stats -->
        <div class="wsl-stats">
          <div class="wsl-stat wsl-stat-purple">
            <div class="wsl-stat-val">{{ info.wsl_version || 'N/A' }}</div>
            <div class="wsl-stat-lbl">Version WSL</div>
          </div>
          <div class="wsl-stat wsl-stat-blue">
            <div class="wsl-stat-val" style="font-size:13px;line-height:1.3">{{ info.kernel_version || 'N/A' }}</div>
            <div class="wsl-stat-lbl">Kernel Linux</div>
          </div>
          <div class="wsl-stat wsl-stat-green">
            <div class="wsl-stat-val">{{ info.distros.filter(d => d.state === 'Running').length }}</div>
            <div class="wsl-stat-lbl">En cours</div>
          </div>
          <div class="wsl-stat wsl-stat-gray">
            <div class="wsl-stat-val">{{ info.distros.length }}</div>
            <div class="wsl-stat-lbl">Distributions</div>
          </div>
        </div>

        <!-- Default version switcher -->
        <div class="wsl-version-bar">
          <div class="wsl-version-label">Version WSL par défaut</div>
          <div class="wsl-version-btns">
            <button class="wsl-ver-btn" :class="info.default_version === 1 ? 'ver-active' : ''" @click="setVersion(1)">WSL 1</button>
            <button class="wsl-ver-btn" :class="info.default_version === 2 ? 'ver-active' : ''" @click="setVersion(2)">WSL 2</button>
          </div>
          <div class="wsl-ver-hint">WSL 2 recommandé — performances natives du kernel Linux</div>
        </div>

        <!-- Distros -->
        <div class="wsl-section-title"><List :size="14" /> Distributions installées ({{ info.distros.length }})</div>
        <div v-if="info.distros.length === 0" class="wsl-empty">
          <Terminal :size="24" style="opacity:.2" />
          <span>Aucune distribution installée — lancez <code class="wsl-code">wsl --install -d Ubuntu</code></span>
        </div>
        <div v-else class="wsl-distros">
          <div v-for="d in info.distros" :key="d.name" class="wsl-distro" :class="d.is_default ? 'distro-default' : ''">
            <div class="wsl-distro-icon" :class="d.state === 'Running' ? 'dicon-running' : 'dicon-stopped'">
              <Terminal :size="16" />
            </div>
            <div class="wsl-distro-body">
              <div class="wsl-distro-name">
                {{ d.name }}
                <span v-if="d.is_default" class="wsl-default-badge"><Star :size="10" /> Défaut</span>
              </div>
              <div class="wsl-distro-meta">
                <span class="wsl-ver-chip">WSL {{ d.version }}</span>
              </div>
            </div>
            <div class="wsl-distro-state">
              <span :class="d.state === 'Running' ? 'state-running' : 'state-stopped'">
                {{ d.state === 'Running' ? '● En cours' : '○ Arrêté' }}
              </span>
            </div>
          </div>
        </div>

        <!-- Terminal -->
        <div class="wsl-section-title" style="margin-top:6px"><Play :size="14" /> Exécuter une commande</div>
        <div class="wsl-cmd-panel">
          <div class="wsl-cmd-row">
            <select v-model="selectedDistro" class="wsl-select">
              <option value="">Distribution par défaut</option>
              <option v-for="d in info.distros" :key="d.name" :value="d.name">{{ d.name }}</option>
            </select>
            <input v-model="wslCmd" class="wsl-input" placeholder="uname -a, df -h, free -m..." @keyup.enter="runCmd" />
            <button class="wsl-btn wsl-btn-primary" :disabled="cmdLoading || !wslCmd" @click="runCmd">
              <Play :size="13" /> Exécuter
            </button>
          </div>
          <div v-if="cmdLoading" class="wsl-cmd-loading"><div class="wsl-spinner-sm" /> Exécution en cours...</div>
          <pre v-else-if="cmdOutput" class="wsl-output">{{ cmdOutput }}</pre>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from "@/utils/invoke";
import { Terminal, RefreshCw, List, Star, Play } from 'lucide-vue-next'

interface WslDistro { name: string; state: string; version: number; is_default: boolean }
interface WslInfo { installed: boolean; default_version: number; distros: WslDistro[]; kernel_version: string; wsl_version: string; error: string }

const loading = ref(false); const info = ref<WslInfo | null>(null)
const selectedDistro = ref(''); const wslCmd = ref('')
const cmdLoading = ref(false); const cmdOutput = ref(''); const versionMsg = ref('')

async function load() { loading.value = true; try { info.value = await invoke<WslInfo>('get_wsl_info') } catch(e) { versionMsg.value = String(e) } finally { loading.value = false } }

async function setVersion(v: number) {
  try {
    await invoke<string>('wsl_set_default_version', { version: v })
    versionMsg.value = `WSL ${v} défini comme version par défaut`
    setTimeout(() => { versionMsg.value = '' }, 2500)
    await load()
  } catch(e) { versionMsg.value = String(e) }
}

async function runCmd() {
  if (!wslCmd.value.trim()) return
  cmdLoading.value = true; cmdOutput.value = ''
  try { cmdOutput.value = await invoke<string>('wsl_run_command', { distro: selectedDistro.value, command: wslCmd.value }) }
  catch(e) { cmdOutput.value = String(e) }
  finally { cmdLoading.value = false }
}

onMounted(load)
</script>

<style scoped>
.wsl-root { display: flex; flex-direction: column; gap: 14px; }

.wsl-banner { display: flex; align-items: center; gap: 16px; padding: 18px 22px;
  background: linear-gradient(135deg, rgba(124,58,237,.14), rgba(109,40,217,.07));
  border: 1px solid rgba(124,58,237,.3); border-radius: 14px; }
.wsl-banner-icon { width: 48px; height: 48px; border-radius: 12px;
  background: linear-gradient(135deg,#7c3aed,#4c1d95); display: flex; align-items: center;
  justify-content: center; color: #fff; flex-shrink: 0; box-shadow: 0 4px 14px rgba(124,58,237,.4); }
.wsl-banner-text { flex: 1; }
.wsl-banner-title { font-size: 17px; font-weight: 700; margin-bottom: 3px; }
.wsl-banner-desc { font-size: 12px; opacity: .7; }
.wsl-msg-ok { font-size: 12px; color: #22c55e; background: rgba(34,197,94,.1); padding: 4px 10px; border-radius: 6px; }

.wsl-loading { display: flex; align-items: center; gap: 10px; padding: 20px; font-size: 13px; color: var(--text-muted); }
.wsl-spinner { width: 15px; height: 15px; border: 2px solid rgba(255,255,255,.15); border-top-color: #7c3aed; border-radius: 50%; animation: spin .8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.wsl-unavailable { display: flex; align-items: flex-start; gap: 20px; padding: 24px;
  background: rgba(124,58,237,.07); border: 1px solid rgba(124,58,237,.2); border-radius: 14px; }
.wsl-unavail-icon { width: 56px; height: 56px; border-radius: 14px; background: rgba(124,58,237,.15);
  color: #7c3aed; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.wsl-unavail-title { font-size: 15px; font-weight: 600; margin-bottom: 5px; }
.wsl-unavail-desc { font-size: 12px; opacity: .6; margin-bottom: 8px; }
.wsl-unavail-hint { font-size: 12px; opacity: .7; }
.wsl-code { font-family: 'JetBrains Mono',monospace; font-size: 11px; background: rgba(0,0,0,.3); padding: 1px 6px; border-radius: 4px; }

.wsl-stats { display: grid; grid-template-columns: repeat(4,1fr); gap: 10px; }
.wsl-stat { border-radius: 12px; padding: 16px; text-align: center; border: 1px solid transparent; }
.wsl-stat-purple { background: rgba(124,58,237,.1); border-color: rgba(124,58,237,.25); }
.wsl-stat-blue   { background: rgba(59,130,246,.1);  border-color: rgba(59,130,246,.25); }
.wsl-stat-green  { background: rgba(34,197,94,.1);   border-color: rgba(34,197,94,.25); }
.wsl-stat-gray   { background: var(--bg-secondary);  border-color: var(--border); }
.wsl-stat-val { font-size: 22px; font-weight: 700; margin-bottom: 3px; }
.wsl-stat-lbl { font-size: 10px; opacity: .5; text-transform: uppercase; }

.wsl-version-bar { display: flex; align-items: center; gap: 14px; padding: 12px 18px;
  background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 12px; }
.wsl-version-label { font-size: 13px; font-weight: 500; flex-shrink: 0; }
.wsl-version-btns { display: flex; gap: 4px; }
.wsl-ver-btn { padding: 5px 18px; border-radius: 7px; border: 1px solid var(--border);
  background: var(--bg-tertiary); color: var(--text-secondary); font-size: 12px; cursor: pointer; transition: all 150ms; font-family: inherit; }
.wsl-ver-btn:hover { border-color: #7c3aed; color: #7c3aed; }
.ver-active { background: rgba(124,58,237,.2) !important; color: #a78bfa !important; border-color: rgba(124,58,237,.5) !important; font-weight: 600; }
.wsl-ver-hint { font-size: 11px; opacity: .45; flex: 1; }

.wsl-section-title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 600; opacity: .7; }

.wsl-empty { display: flex; align-items: center; gap: 12px; padding: 22px; font-size: 12px; color: var(--text-muted);
  background: var(--bg-secondary); border-radius: 12px; border: 1px solid var(--border); }

.wsl-distros { display: flex; flex-direction: column; gap: 6px; }
.wsl-distro { display: flex; align-items: center; gap: 12px; padding: 12px 16px;
  background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 12px; transition: border-color 150ms; }
.wsl-distro:hover { border-color: rgba(124,58,237,.3); }
.distro-default { border-color: rgba(245,158,11,.35) !important; background: rgba(245,158,11,.05) !important; }
.wsl-distro-icon { width: 36px; height: 36px; border-radius: 9px; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.dicon-running { background: rgba(34,197,94,.15); color: #22c55e; }
.dicon-stopped { background: var(--bg-tertiary); color: var(--text-muted); }
.wsl-distro-body { flex: 1; }
.wsl-distro-name { font-size: 13px; font-weight: 600; display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
.wsl-default-badge { display: inline-flex; align-items: center; gap: 3px; background: rgba(245,158,11,.15); color: #f59e0b; border: 1px solid rgba(245,158,11,.3); border-radius: 5px; padding: 2px 7px; font-size: 10px; font-weight: 600; }
.wsl-distro-meta { display: flex; gap: 6px; align-items: center; }
.wsl-ver-chip { font-size: 10px; background: rgba(124,58,237,.15); color: #a78bfa; border: 1px solid rgba(124,58,237,.25); border-radius: 4px; padding: 1px 7px; }
.wsl-distro-state { font-size: 12px; font-weight: 500; }
.state-running { color: #22c55e; }
.state-stopped { color: #6b7280; }

.wsl-cmd-panel { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 14px; overflow: hidden; }
.wsl-cmd-row { display: flex; gap: 8px; padding: 14px; align-items: center; flex-wrap: wrap; }
.wsl-select { background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 8px; padding: 8px 10px; color: var(--text-primary); font-size: 12px; outline: none; min-width: 160px; }
.wsl-input { flex: 1; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 8px; padding: 8px 12px; color: var(--text-primary); font-size: 12px; outline: none; font-family: 'JetBrains Mono',monospace; min-width: 200px; }
.wsl-input:focus, .wsl-select:focus { border-color: #7c3aed; }
.wsl-cmd-loading { display: flex; align-items: center; gap: 8px; padding: 12px 14px; font-size: 12px; color: var(--text-muted); border-top: 1px solid var(--border); }
.wsl-spinner-sm { width: 13px; height: 13px; border: 2px solid rgba(255,255,255,.15); border-top-color: #7c3aed; border-radius: 50%; animation: spin .8s linear infinite; }
.wsl-output { margin: 0; padding: 14px 16px; background: #0d0d0d; border-top: 1px solid rgba(124,58,237,.3);
  font-family: 'JetBrains Mono',monospace; font-size: 12px; white-space: pre-wrap; max-height: 320px; overflow-y: auto;
  color: #e5e5e5; line-height: 1.7; letter-spacing: .02em;
  text-shadow: 0 0 8px rgba(167,139,250,.3); }

.wsl-btn { display: inline-flex; align-items: center; gap: 5px; padding: 8px 14px; border-radius: 8px;
  border: 1px solid var(--border); background: var(--bg-secondary); color: var(--text-secondary);
  font-size: 12px; cursor: pointer; transition: all 150ms; font-family: inherit; }
.wsl-btn:disabled { opacity: .4; cursor: not-allowed; }
.wsl-btn-primary { background: rgba(124,58,237,.15); color: #a78bfa; border-color: rgba(124,58,237,.3); }
.wsl-btn-primary:hover:not(:disabled) { background: rgba(124,58,237,.25); }
</style>
