<template>
  <div class="hosts-root">
    <!-- Banner -->
    <div class="hosts-banner">
      <div class="hosts-banner-icon"><FileText :size="24" /></div>
      <div class="hosts-banner-text">
        <div class="hosts-banner-title">Éditeur Hosts</div>
        <div class="hosts-banner-desc">Gérez le fichier <code class="path-code">C:\Windows\System32\drivers\etc\hosts</code></div>
      </div>
      <div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap">
        <button class="hosts-btn hosts-btn-primary" :disabled="loading" @click="loadEntries"><RefreshCw :size="13" /> Actualiser</button>
        <button class="hosts-btn" @click="doBackup"><Save :size="13" /> Sauvegarde .bak</button>
        <button class="hosts-btn" @click="exportHosts">↓ Exporter hosts</button>
        <button class="hosts-btn" :class="showBlocklists ? 'hosts-btn-primary' : ''" @click="showBlocklists = !showBlocklists">
          📋 Blocklists
        </button>
        <span v-if="msg" :class="msgErr ? 'h-err' : 'h-ok'" class="h-msg">{{ msg }}</span>
      </div>
    </div>

    <div v-if="loading" class="hosts-loading"><div class="hosts-spinner" /> Lecture du fichier hosts...</div>

    <div v-else>
      <!-- Stats row -->
      <div v-if="entries.length > 0" class="hosts-stats">
        <div class="hosts-stat">
          <span class="hs-val">{{ entries.length }}</span>
          <span class="hs-lbl">Total</span>
        </div>
        <div class="hosts-stat">
          <span class="hs-val hs-green">{{ entries.filter(e => e.active).length }}</span>
          <span class="hs-lbl">Actives</span>
        </div>
        <div class="hosts-stat">
          <span class="hs-val hs-gray">{{ entries.filter(e => !e.active).length }}</span>
          <span class="hs-lbl">Désactivées</span>
        </div>
        <div class="hosts-stat">
          <span class="hs-val hs-blue">{{ entries.filter(e => e.ip.startsWith('127.')).length }}</span>
          <span class="hs-lbl">Localhost</span>
        </div>
      </div>

      <!-- Table -->
      <div v-if="entries.length > 0" class="hosts-table-wrap">
        <table class="hosts-table">
          <thead>
            <tr>
              <th style="width:50px">Actif</th>
              <th>Adresse IP</th>
              <th>Hostname</th>
              <th>Commentaire</th>
              <th style="width:50px">Ligne</th>
              <th style="width:80px">Action</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="e in entries" :key="e.line_number" :class="!e.active ? 'row-inactive' : ''">
              <td style="text-align:center">
                <button class="toggle-btn" :class="e.active ? 'tog-on' : 'tog-off'"
                  @click="toggleEntry(e.line_number, !e.active)" :title="e.active ? 'Désactiver' : 'Activer'">
                  {{ e.active ? '●' : '○' }}
                </button>
              </td>
              <td>
                <span class="ip-badge" :class="ipClass(e.ip)">{{ e.ip }}</span>
              </td>
              <td class="hostname-cell">{{ e.hostname }}</td>
              <td class="comment-cell">{{ e.comment }}</td>
              <td style="text-align:center"><span class="line-num">{{ e.line_number }}</span></td>
              <td>
                <button class="hosts-btn hosts-btn-danger hosts-btn-sm" @click="deleteEntry(e.line_number)">
                  <Trash2 :size="11" /> Suppr.
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <div v-else class="hosts-empty">
        <FileText :size="28" style="opacity:.2" />
        <span>Aucune entrée dans le fichier hosts</span>
      </div>

      <!-- Blocklists prédéfinies -->
      <div v-if="showBlocklists" class="hosts-add-section" style="border-color:rgba(59,130,246,.3)">
        <div class="hosts-add-title" style="color:#60a5fa">📋 Blocklists prédéfinies</div>
        <div style="padding:14px;display:flex;flex-direction:column;gap:8px">
          <div v-for="bl in BLOCKLISTS" :key="bl.name"
            style="display:flex;align-items:center;gap:12px;padding:10px 12px;background:var(--bg-tertiary);border-radius:8px;border:1px solid var(--border)">
            <div style="flex:1">
              <div style="font-size:12px;font-weight:600">{{ bl.name }}</div>
              <div style="font-size:11px;color:var(--text-muted)">{{ bl.description }} · {{ bl.count }} entrées</div>
            </div>
            <span style="font-size:10px;padding:2px 7px;border-radius:4px"
              :style="{ background: bl.category === 'ads' ? 'rgba(249,115,22,.12)' : bl.category === 'malware' ? 'rgba(239,68,68,.12)' : 'rgba(59,130,246,.12)',
                        color: bl.category === 'ads' ? '#f97316' : bl.category === 'malware' ? '#ef4444' : '#60a5fa' }">
              {{ bl.category }}
            </span>
            <button class="hosts-btn hosts-btn-primary hosts-btn-sm"
              :disabled="importingList !== null"
              @click="importBlocklist(bl)">
              <span v-if="importingList === bl.name">...</span>
              <span v-else>+ Importer</span>
            </button>
          </div>
          <div v-if="importMsg" style="font-size:12px;padding:6px 10px;border-radius:5px"
            :style="{ color: importMsg.includes('Erreur') ? 'var(--error)' : 'var(--success)', background: importMsg.includes('Erreur') ? 'rgba(239,68,68,.08)' : 'rgba(34,197,94,.08)' }">
            {{ importMsg }}
          </div>
          <div style="font-size:11px;color:var(--text-muted);border-top:1px solid var(--border);padding-top:8px">
            ⚠ L'import de blocklists ajoute un grand nombre d'entrées. Utilisez "Sauvegarde .bak" avant d'importer.
          </div>
        </div>
      </div>

      <!-- Test de résolution DNS -->
      <div class="hosts-add-section">
        <div class="hosts-add-title">🔍 Tester la résolution d'un hostname</div>
        <div style="padding:14px;display:flex;gap:8px;align-items:center;flex-wrap:wrap">
          <input v-model="testHostname" class="hosts-input" style="flex:1;min-width:200px"
            placeholder="exemple.com" @keyup.enter="testResolution" />
          <button class="hosts-btn hosts-btn-primary" :disabled="testLoading || !testHostname" @click="testResolution">
            {{ testLoading ? 'Résolution...' : 'Tester' }}
          </button>
          <div v-if="testResult" style="font-size:12px;padding:4px 10px;border-radius:5px;font-family:monospace"
            :style="{ color: testResult.includes('Erreur') ? 'var(--error)' : 'var(--success)', background: testResult.includes('Erreur') ? 'rgba(239,68,68,.08)' : 'rgba(34,197,94,.08)' }">
            {{ testHostname }} → {{ testResult }}
          </div>
        </div>
      </div>

      <!-- Add form -->
      <div class="hosts-add-section">
        <div class="hosts-add-title"><Plus :size="14" /> Ajouter une entrée</div>
        <div class="hosts-add-form">
          <div class="hosts-field">
            <label>Adresse IP</label>
            <input v-model="newIp" class="hosts-input" placeholder="127.0.0.1" />
          </div>
          <div class="hosts-field" style="flex:1">
            <label>Hostname</label>
            <input v-model="newHost" class="hosts-input" placeholder="mon-site.local" />
          </div>
          <div class="hosts-field" style="flex:1">
            <label>Commentaire (optionnel)</label>
            <input v-model="newComment" class="hosts-input" placeholder="Description..." />
          </div>
          <button class="hosts-btn hosts-btn-primary" :disabled="!newIp || !newHost" @click="addEntry" style="align-self:flex-end">
            <Plus :size="13" /> Ajouter
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from "@/utils/invoke";
import { FileText, RefreshCw, Save, Plus, Trash2 } from 'lucide-vue-next'

interface HostsEntry { ip: string; hostname: string; comment: string; active: boolean; line_number: number }
const entries = ref<HostsEntry[]>([]); const loading = ref(false)
const msg = ref(''); const msgErr = ref(false)
const newIp = ref(''); const newHost = ref(''); const newComment = ref('')

function showMsg(t: string, err = false) { msg.value = t; msgErr.value = err; setTimeout(() => { msg.value = '' }, 3000) }

async function loadEntries() { loading.value = true; try { entries.value = await invoke<HostsEntry[]>('get_hosts_entries') } finally { loading.value = false } }
async function addEntry() {
  try { showMsg(await invoke<string>('add_hosts_entry', { ip: newIp.value, hostname: newHost.value, comment: newComment.value })); newIp.value = ''; newHost.value = ''; newComment.value = ''; await loadEntries() }
  catch(e) { showMsg(String(e), true) }
}
async function deleteEntry(n: number) { try { await invoke<string>('delete_hosts_entry', { lineNumber: n }); showMsg('Supprimé'); await loadEntries() } catch(e) { showMsg(String(e), true) } }
async function toggleEntry(n: number, en: boolean) { try { await invoke<string>('toggle_hosts_entry', { lineNumber: n, enable: en }); await loadEntries() } catch(e) { showMsg(String(e), true) } }
async function doBackup() {
  try {
    const content = await invoke<string>('get_hosts_raw');
    const { save } = await import('@tauri-apps/plugin-dialog');
    const filePath = await save({
      defaultPath: 'hosts.bak',
      filters: [{ name: 'Sauvegarde', extensions: ['bak'] }]
    });
    if (!filePath) return;
    const { writeTextFile } = await import('@tauri-apps/plugin-fs');
    await writeTextFile(filePath, content);
    showMsg('Sauvegarde enregistrée : ' + filePath);
  } catch (e: any) { showMsg('Erreur sauvegarde : ' + String(e), true); }
}

function ipClass(ip: string) { if (ip.startsWith('127.')) return 'ip-localhost'; if (ip.startsWith('::1') || ip === '0.0.0.0') return 'ip-special'; return 'ip-normal'; }

// ── Blocklists prédéfinies ────────────────────────────────────────────────────
interface Blocklist {
  name: string;
  description: string;
  url: string;
  count: string;
  category: 'ads' | 'malware' | 'privacy' | 'social';
}

const BLOCKLISTS: Blocklist[] = [
  {
    name: "StevenBlack Ads",
    description: "Publicités et trackers",
    url: "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts",
    count: "~150k",
    category: "ads",
  },
  {
    name: "AdAway",
    description: "Publicités mobiles",
    url: "https://adaway.org/hosts.txt",
    count: "~40k",
    category: "ads",
  },
  {
    name: "MVPS Hosts",
    description: "Spam, malware, trackers",
    url: "https://winhelp2002.mvps.org/hosts.txt",
    count: "~15k",
    category: "malware",
  },
  {
    name: "Cameleon",
    description: "Publicités + scripts malveillants",
    url: "https://sysctl.org/cameleon/hosts",
    count: "~21k",
    category: "ads",
  },
];

const showBlocklists = ref(false);
const importingList = ref<string | null>(null);
const importMsg = ref("");

async function importBlocklist(bl: Blocklist) {
  if (!confirm(`Importer "${bl.name}" (~${bl.count} entrées) ? Cela ajoutera des entrées au fichier hosts.`)) return;
  importingList.value = bl.name;
  importMsg.value = "Téléchargement en cours...";
  try {
    const result = await invoke<string>("import_hosts_blocklist", { url: bl.url, listName: bl.name });
    importMsg.value = result || `Import "${bl.name}" terminé ✓`;
    await loadEntries();
  } catch (e: any) {
    importMsg.value = "Erreur import : " + String(e);
  }
  importingList.value = null;
  setTimeout(() => { importMsg.value = ""; }, 5000);
}

// ── Test de résolution DNS ────────────────────────────────────────────────────
const testHostname = ref("");
const testResult = ref("");
const testLoading = ref(false);

async function testResolution() {
  if (!testHostname.value.trim()) return;
  testLoading.value = true;
  testResult.value = "";
  try {
    const result = await invoke<string>("resolve_hostname", { hostname: testHostname.value.trim() });
    testResult.value = result || "Résolu";
  } catch (e: any) {
    testResult.value = "Erreur : " + String(e);
  }
  testLoading.value = false;
}

// ── Export fichier hosts complet ──────────────────────────────────────────────
async function exportHosts() {
  try {
    const content = await invoke<string>("get_hosts_raw");
    const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "hosts_backup_" + new Date().toISOString().slice(0, 10) + ".txt";
    a.click();
    URL.revokeObjectURL(url);
  } catch (e: any) {
    showMsg("Erreur export : " + String(e), true);
  }
}

onMounted(loadEntries)
</script>

<style scoped>
.hosts-root { display: flex; flex-direction: column; gap: 14px; }
.hosts-banner { display: flex; align-items: center; gap: 16px; padding: 18px 22px;
  background: linear-gradient(135deg,rgba(16,185,129,.12),rgba(5,150,105,.07));
  border: 1px solid rgba(16,185,129,.3); border-radius: 14px; }
.hosts-banner-icon { width: 48px; height: 48px; border-radius: 12px; background: linear-gradient(135deg,#10b981,#059669); display: flex; align-items: center; justify-content: center; color: #fff; flex-shrink: 0; box-shadow: 0 4px 14px rgba(16,185,129,.4); }
.hosts-banner-text { flex: 1; }
.hosts-banner-title { font-size: 17px; font-weight: 700; margin-bottom: 3px; }
.hosts-banner-desc { font-size: 12px; opacity: .7; }
.path-code { font-family: 'JetBrains Mono',monospace; font-size: 11px; background: rgba(0,0,0,.2); padding: 1px 5px; border-radius: 4px; }

.hosts-loading { display: flex; align-items: center; gap: 10px; padding: 20px; font-size: 13px; color: var(--text-muted); }
.hosts-spinner { width: 15px; height: 15px; border: 2px solid rgba(255,255,255,.15); border-top-color: #10b981; border-radius: 50%; animation: spin .8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.hosts-stats { display: flex; gap: 10px; }
.hosts-stat { flex: 1; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 10px; padding: 12px; text-align: center; }
.hs-val { display: block; font-size: 22px; font-weight: 700; }
.hs-lbl { font-size: 10px; opacity: .5; text-transform: uppercase; }
.hs-green { color: #22c55e; }
.hs-gray  { color: #6b7280; }
.hs-blue  { color: #3b82f6; }

.hosts-table-wrap { border: 1px solid var(--border); border-radius: 12px; overflow: hidden; }
.hosts-table { width: 100%; border-collapse: collapse; font-size: 12px; }
.hosts-table thead tr { background: var(--bg-tertiary); }
.hosts-table th { padding: 8px 12px; text-align: left; color: var(--text-muted); font-size: 10px; text-transform: uppercase; letter-spacing: .04em; border-bottom: 1px solid var(--border); }
.hosts-table td { padding: 8px 12px; border-bottom: 1px solid var(--border); }
.hosts-table tbody tr:last-child td { border-bottom: none; }
.hosts-table tbody tr:hover td { background: var(--bg-tertiary); }
.row-inactive td { opacity: .45; text-decoration: line-through; }

.toggle-btn { background: none; border: none; cursor: pointer; font-size: 18px; transition: color 150ms; padding: 0; }
.tog-on  { color: #22c55e; }
.tog-off { color: #4b5563; }

.ip-badge { font-family: 'JetBrains Mono',monospace; font-size: 11px; padding: 2px 8px; border-radius: 6px; }
.ip-localhost { background: rgba(16,185,129,.12); color: #10b981; }
.ip-special   { background: rgba(124,58,237,.12);  color: #7c3aed; }
.ip-normal    { background: var(--bg-tertiary); color: var(--text-secondary); }
.hostname-cell { font-weight: 500; }
.comment-cell { font-size: 11px; opacity: .5; font-style: italic; }
.line-num { font-size: 10px; opacity: .4; background: var(--bg-tertiary); padding: 1px 5px; border-radius: 4px; }

.hosts-empty { display: flex; align-items: center; justify-content: center; gap: 12px; padding: 28px; font-size: 13px; color: var(--text-muted); background: var(--bg-secondary); border-radius: 12px; border: 1px solid var(--border); }

.hosts-add-section { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 14px; overflow: hidden; }
.hosts-add-title { display: flex; align-items: center; gap: 8px; padding: 12px 16px; font-size: 12px; font-weight: 600; border-bottom: 1px solid var(--border); background: var(--bg-tertiary); opacity: .8; }
.hosts-add-form { display: flex; gap: 10px; padding: 16px; align-items: flex-end; flex-wrap: wrap; }
.hosts-field { display: flex; flex-direction: column; gap: 4px; }
.hosts-field label { font-size: 10px; opacity: .5; text-transform: uppercase; letter-spacing: .04em; }
.hosts-input { background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 8px; padding: 8px 12px; color: var(--text-primary); font-size: 12px; outline: none; min-width: 130px; }
.hosts-input:focus { border-color: #10b981; }

.hosts-btn { display: inline-flex; align-items: center; gap: 5px; padding: 8px 14px; border-radius: 8px; border: 1px solid var(--border); background: var(--bg-secondary); color: var(--text-secondary); font-size: 12px; cursor: pointer; transition: all 150ms; font-family: inherit; }
.hosts-btn:disabled { opacity: .4; cursor: not-allowed; }
.hosts-btn-primary { background: rgba(16,185,129,.15); color: #10b981; border-color: rgba(16,185,129,.3); }
.hosts-btn-primary:hover:not(:disabled) { background: rgba(16,185,129,.25); }
.hosts-btn-danger { color: #ef4444; border-color: rgba(239,68,68,.3); }
.hosts-btn-danger:hover { background: rgba(239,68,68,.1); }
.hosts-btn-sm { padding: 4px 10px; font-size: 11px; }

.h-msg { font-size: 12px; padding: 5px 10px; border-radius: 6px; }
.h-ok  { color: #22c55e; background: rgba(34,197,94,.1); }
.h-err { color: #ef4444; background: rgba(239,68,68,.1); }
</style>
