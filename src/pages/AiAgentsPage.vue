<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onUnmounted } from "vue";
import { invoke, isTauriContext } from "@/utils/invoke";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NDropdown from "@/components/ui/NDropdown.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NProgress from "@/components/ui/NProgress.vue";
import NModal from "@/components/ui/NModal.vue";
import { useNotificationStore } from "@/stores/notifications";
import { useExportData } from "@/composables/useExportData";
import {
  Bot, Send, Terminal, Play, CheckCircle, XCircle, RefreshCw,
  Cpu, MessageSquare, Settings, FolderOpen, Square, Download,
  Zap, Server, Package, History, Trash2, Plus, Clock, RotateCcw,
} from "lucide-vue-next";

const notify = useNotificationStore();
const { exportJSON } = useExportData();

// ─── Backend selector ──────────────────────────────────────────────────────────
const backend = ref<"llamacpp" | "ollama">("llamacpp");

// ─── Catalogue modèles ─────────────────────────────────────────────────────────
interface CatalogEntry { id: string; name: string; description: string; size_gb: number; ram_gb: number; vram_gb: number; url: string; filename: string; recommended: boolean }
const catalog = ref<CatalogEntry[]>([]);

// ─── llama.cpp portable ────────────────────────────────────────────────────────
interface GgufModel { name: string; path: string; size_gb: number }
interface DownloadProgress { name: string; downloaded_mb: number; total_mb: number; percent: number; done: boolean; error: string | null }

const ggufModels     = ref<GgufModel[]>([]);
const selectedGguf   = ref("");
const serverBin      = ref("");
const serverRunning       = ref(false);
const serverChecking      = ref(false);
const startingServer      = ref(false);
const serverStartCancelled = ref(false);

// Téléchargement
const downloadProgress = ref<Record<string, DownloadProgress>>({});
const downloadingServer = ref(false);
const downloadingModel  = ref<string | null>(null);

let unlistenDownload: (() => void) | null = null;

async function setupDownloadListener() {
  unlistenDownload = await listen<DownloadProgress>("ai:download-progress", (ev) => {
    const p = ev.payload;
    downloadProgress.value[p.name] = p;
    if (p.done) {
      setTimeout(() => { delete downloadProgress.value[p.name]; }, 2000);
    }
  });
}

async function downloadServer() {
  downloadingServer.value = true;
  try {
    const path = await invoke<string>("ai_download_server");
    serverBin.value = path;
    notify.success("llama-server", "Installé dans logiciel/AI/");
  } catch (e: unknown) {
    notify.error("Téléchargement échoué", String(e));
  }
  downloadingServer.value = false;
}

async function downloadModel(entry: CatalogEntry) {
  downloadingModel.value = entry.id;
  try {
    const path = await invoke<string>("ai_download_model", { url: entry.url, filename: entry.filename });
    await scanGgufModels();
    selectedGguf.value = path;
    notify.success(entry.name, "Modèle téléchargé dans models/");
  } catch (e: unknown) {
    notify.error("Téléchargement échoué", String(e));
  }
  downloadingModel.value = null;
}

const serverDlProgress = computed(() => downloadProgress.value["llama-server.exe"]);
function modelDlProgress(entry: CatalogEntry) { return downloadProgress.value[entry.filename]; }

// ─── Scan modèles locaux ───────────────────────────────────────────────────────
async function detectServer() {
  try { const bin = await invoke<string | null>("ai_find_llamacpp_server"); if (bin) serverBin.value = bin; } catch { /* llama.cpp server non trouvé — normal si pas installé */ }
}
async function scanGgufModels() {
  try {
    const list = await invoke<GgufModel[]>("ai_list_gguf_models");
    ggufModels.value = list;
    if (list.length && !selectedGguf.value) selectedGguf.value = list[0].path;
  } catch { /* aucun modèle GGUF trouvé — normal au premier lancement */ }
}
async function browseModelFile() {
  try {
    const path = await open({ filters: [{ name: "GGUF", extensions: ["gguf"] }] });
    if (path && typeof path === "string") {
      if (!validateAndSetGguf(path)) {
        notify.error("Extension invalide", "Seuls les fichiers .gguf sont acceptés.");
        return;
      }
      selectedGguf.value = path;
      const name = path.split(/[\\/]/).pop() ?? path;
      if (!ggufModels.value.find(m => m.path === path))
        ggufModels.value.unshift({ name, path, size_gb: 0 });
    }
  } catch (e) { notify.error("Erreur sélection fichier", String(e)); }
}

// ─── Serveur ──────────────────────────────────────────────────────────────────
async function startServer() {
  if (!selectedGguf.value) { notify.warning("Modèle requis", "Sélectionnez un fichier .gguf"); return; }
  serverStartCancelled.value = false;
  startingServer.value = true;
  try {
    await invoke("ai_start_llamacpp", { modelPath: selectedGguf.value });
    notify.info("llama.cpp", "Démarrage en cours (chargement du modèle)...");
    for (let i = 0; i < 60; i++) {
      if (serverStartCancelled.value) break;
      await new Promise(r => setTimeout(r, 1000));
      if (serverStartCancelled.value) break;
      // ai_llamacpp_status rejette explicitement si le process a crashé —
      // distinct d'un simple "pas encore prêt" (Ok(false)) : on arrête le
      // polling immédiatement au lieu d'attendre les 60 tentatives.
      try {
        const ok = await invoke<boolean>("ai_llamacpp_status");
        if (ok) { serverRunning.value = true; notify.success("llama.cpp", "Serveur prêt !"); break; }
      } catch (e: unknown) {
        notify.error("llama.cpp", String(e));
        break;
      }
    }
    if (!serverRunning.value && !serverStartCancelled.value)
      notify.warning("llama.cpp", "Serveur démarré — attente du chargement du modèle.");
  } catch (e: unknown) { if (!serverStartCancelled.value) notify.error("llama.cpp", String(e)); }
  startingServer.value = false;
}
async function stopServer() {
  try { await invoke("ai_stop_llamacpp"); serverRunning.value = false; } catch {}
}
async function checkLlamacppStatus() {
  serverChecking.value = true;
  serverRunning.value = await invoke<boolean>("ai_llamacpp_status").catch(() => false);
  serverChecking.value = false;
}

// ─── Ollama ────────────────────────────────────────────────────────────────────
const ollamaConnected     = ref(false);
const ollamaModels        = ref<{ value: string; label: string }[]>([]);
const selectedOllama      = ref("");
const ollamaModelsLoading = ref(false);

async function checkOllamaStatus() {
  ollamaConnected.value = await invoke<boolean>("ai_check").catch(() => false);
}
async function loadOllamaModels() {
  ollamaModelsLoading.value = true;
  try {
    const list = await invoke<{ name: string; size_gb: number }[]>("ai_list_models");
    ollamaModels.value = list.map(m => ({ value: m.name, label: `${m.name} (${m.size_gb.toFixed(1)} GB)` }));
    if (list.length && !selectedOllama.value) selectedOllama.value = list[0].name;
  } catch (e: unknown) {
    if (!isTauriContext()) {
      ollamaModels.value = [{ value: "llama3.2", label: "llama3.2" }];
      if (!selectedOllama.value) selectedOllama.value = "llama3.2";
    } else {
      ollamaModels.value = [];
      notify.warning("Ollama", "Impossible de lister les modèles — Ollama est-il démarré ?");
    }
  }
  ollamaModelsLoading.value = false;
}

// ─── Paramètres ────────────────────────────────────────────────────────────────
// Borne valide : une valeur corrompue en localStorage (parseInt → NaN) rendrait
// slice(0, NaN) équivalent à slice(0, 0) et effacerait TOUT l'historique.
function sanitizeMaxConv(n: number): number {
  return Number.isFinite(n) && n > 0 ? Math.min(Math.floor(n), 1000) : 50;
}
const maxConversations = ref<number>(
  sanitizeMaxConv(parseInt(localStorage.getItem("nitrite_ai_max_conv") ?? "50", 10))
);
function saveMaxConversations() {
  maxConversations.value = sanitizeMaxConv(maxConversations.value);
  localStorage.setItem("nitrite_ai_max_conv", String(maxConversations.value));
}

// ─── Validation chemin GGUF custom ─────────────────────────────────────────────
const ggufPathError = ref("");
function validateAndSetGguf(path: string) {
  if (!path.toLowerCase().endsWith(".gguf")) {
    ggufPathError.value = "Le fichier doit avoir l'extension .gguf";
    return false;
  }
  ggufPathError.value = "";
  return true;
}

// ─── Historique conversations (localStorage) ───────────────────────────────────
interface Conversation {
  id: string;
  title: string;
  created_at: string;
  model: string;
  messages: ChatMessage[];
}

const conversations   = ref<Conversation[]>([]);
const showHistory     = ref(false);
const currentConvId   = ref<string | null>(null);

function loadConversations() {
  try { conversations.value = JSON.parse(localStorage.getItem("nitrite_ai_conversations") ?? "[]"); }
  catch { conversations.value = []; }
}

function persistConversations() {
  const cap = sanitizeMaxConv(maxConversations.value);
  localStorage.setItem("nitrite_ai_conversations", JSON.stringify(conversations.value.slice(0, cap)));
}

function exportConversations() {
  if (!conversations.value.length) { notify.warning("Aucune conversation", "L'historique est vide."); return; }
  exportJSON(conversations.value, `nitrite_ai_conversations_${new Date().toISOString().slice(0, 10)}`);
  notify.success("Export réussi", `${conversations.value.length} conversation(s) exportée(s)`);
}

function resetCurrentConversation() {
  if (currentConvId.value) {
    const idx = conversations.value.findIndex(c => c.id === currentConvId.value);
    if (idx >= 0) {
      conversations.value[idx].messages = [];
      persistConversations();
    }
  }
  messages.value = [];
  currentConvId.value = null;
}

function saveCurrentConversation() {
  if (!messages.value.length) return;
  const title = messages.value.find(m => m.role === "user")?.content.slice(0, 50) ?? "Conversation";
  if (currentConvId.value) {
    const idx = conversations.value.findIndex(c => c.id === currentConvId.value);
    if (idx >= 0) { conversations.value[idx].messages = [...messages.value]; persistConversations(); return; }
  }
  const id = Date.now().toString();
  currentConvId.value = id;
  conversations.value.unshift({
    id, title, created_at: new Date().toLocaleString("fr-FR"),
    model: activeModel.value, messages: [...messages.value],
  });
  persistConversations();
}

function loadConversation(conv: Conversation) {
  messages.value = [...conv.messages];
  currentConvId.value = conv.id;
  showHistory.value = false;
  scrollChat();
}

function deleteConversation(id: string) {
  conversations.value = conversations.value.filter(c => c.id !== id);
  if (currentConvId.value === id) { currentConvId.value = null; }
  persistConversations();
}

function newConversation() {
  if (messages.value.length) saveCurrentConversation();
  messages.value = [];
  currentConvId.value = null;
}

// ─── Chat ──────────────────────────────────────────────────────────────────────
const systemPromptOptions = [
  { value: "assistant_it", label: "Assistant IT" },
  { value: "diagnostic",   label: "Diagnostic système" },
  { value: "custom",       label: "Personnalisé" },
];
const selectedPromptType = ref("assistant_it");
const customSystemPrompt = ref("");
const systemPromptPresets: Record<string, string> = {
  assistant_it: "Tu es un assistant IT expert en maintenance Windows. Tu réponds en français de manière concise et technique. Quand tu proposes une commande, encadre-la avec ```cmd ou ```powershell.",
  diagnostic:   "Tu es un spécialiste du diagnostic système Windows. Analyse les problèmes décrits, identifie les causes probables et propose des solutions concrètes.",
};
const activeSystemPrompt = computed(() =>
  selectedPromptType.value === "custom" ? customSystemPrompt.value : (systemPromptPresets[selectedPromptType.value] ?? "")
);

interface ChatMessage { role: "user" | "assistant"; content: string; command?: string; commandResult?: string; commandRunning?: boolean }
const messages  = ref<ChatMessage[]>([]);
const userInput = ref("");
const sending   = ref(false);
const chatRef   = ref<HTMLElement | null>(null);

const ggufOptions = computed(() => ggufModels.value.map(m => ({
  value: m.path,
  label: `${m.name} (${m.size_gb > 0 ? m.size_gb.toFixed(1) + " GB" : "?"})`,
})));
const activeModel = computed(() => {
  if (backend.value === "llamacpp")
    return ggufModels.value.find(m => m.path === selectedGguf.value)?.name ?? selectedGguf.value.split(/[\\/]/).pop() ?? "";
  return selectedOllama.value;
});

function scrollChat() { nextTick(() => { if (chatRef.value) chatRef.value.scrollTop = chatRef.value.scrollHeight; }); }
function extractCommand(t: string) { const m = t.match(/```(?:cmd|powershell|bash|shell)\n([\s\S]*?)```/); return m ? m[1].trim() : null; }

async function sendMessage() {
  const text = userInput.value.trim();
  if (!text || sending.value) return;
  if (backend.value === "llamacpp" && !serverRunning.value) {
    notify.warning("Serveur arrêté", "Démarrez le serveur llama.cpp d'abord."); return;
  }
  messages.value.push({ role: "user", content: text });
  userInput.value = "";
  sending.value = true;
  scrollChat();
  const history = messages.value.slice(0, -1).slice(-20).map(m => ({ role: m.role, content: m.content }));
  try {
    const timeoutPromise = new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error("TIMEOUT")), 30_000)
    );
    const queryPromise = invoke<string>("ai_query", {
      model: activeModel.value || undefined,
      prompt: text,
      systemPrompt: activeSystemPrompt.value || null,
      history: history.length ? history : null,
    });
    const response = await Promise.race([queryPromise, timeoutPromise]);
    messages.value.push({ role: "assistant", content: response, command: extractCommand(response) ?? undefined });
    saveCurrentConversation();
  } catch (err: unknown) {
    const isTimeout = String(err).includes("TIMEOUT");
    const hint = isTimeout
      ? "Délai dépassé — le modèle n'a pas répondu dans les 30 secondes. Vérifiez la charge du serveur."
      : (backend.value === "llamacpp" ? "Vérifiez que le serveur est démarré." : "Lancez `ollama serve`.");
    messages.value.push({ role: "assistant", content: `⚠ ${isTimeout ? "Délai dépassé." : "Erreur IA."}\n\n${hint}${isTimeout ? "" : `\n\nDétail : ${String(err)}`}` });
  } finally { sending.value = false; scrollChat(); }
}

async function executeCommand(i: number) {
  const msg = messages.value[i];
  if (!msg.command) return;
  msg.commandRunning = true;
  try {
    const r = await invoke<{ success: boolean; stdout: string; stderr: string }>("ai_execute_command", { command: msg.command });
    msg.commandResult = r.stdout || r.stderr || "Aucune sortie";
    // Toast selon le vrai résultat : un toast « succès » vert pour une commande
    // échouée serait un faux signal.
    if (r.success) notify.success("Commande exécutée", "Succès");
    else notify.warning("Commande terminée", "Échec (voir la sortie)");
  } catch (e: unknown) {
    if (!isTauriContext()) { msg.commandResult = "Simulation — OK"; }
    else {
      const err = e instanceof Error ? e.message : String(e);
      msg.commandResult = `Erreur : ${err}`;
      notify.error("Erreur commande", err.slice(0, 120));
    }
  }
  finally { msg.commandRunning = false; scrollChat(); }
}

function clearChat() { messages.value = []; }
function handleKeydown(e: KeyboardEvent) { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendMessage(); } }
function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}
function formatMessage(text: string): string {
  // Strip null bytes to prevent sentinel injection (e.g. \x00CODEBLOCK0\x00 in LLM output)
  const codeBlocks: string[] = [];
  let safe = text.replace(/\x00/g, '').replace(/```(\w*)\n([\s\S]*?)```/g, (_m, _lang, code) => {
    codeBlocks.push(code);
    return `\x00CODEBLOCK${codeBlocks.length - 1}\x00`;
  });
  // Échapper le HTML du texte courant (hors blocs code)
  safe = escapeHtml(safe);
  // Réinsérer les blocs code avec leur contenu échappé
  safe = safe.replace(/\x00CODEBLOCK(\d+)\x00/g, (_m, i) =>
    `<pre class="code-block"><code>${escapeHtml(codeBlocks[Number(i)])}</code></pre>`
  );
  return safe
    .replace(/`([^`]+)`/g, (_m, c) => `<code class="inline-code">${escapeHtml(c)}</code>`)
    .replace(/\n/g, "<br>");
}

function fmtSize(s: number) { return s >= 1 ? `${s.toFixed(1)} GB` : `${(s * 1024).toFixed(0)} MB`; }

onMounted(async () => {
  await setupDownloadListener();
  catalog.value = await invoke<CatalogEntry[]>("ai_model_catalog").catch(() => []);
  await detectServer();
  await Promise.all([scanGgufModels(), checkLlamacppStatus(), checkOllamaStatus(), loadOllamaModels()]);
  loadConversations();
});
onUnmounted(() => {
  if (unlistenDownload) unlistenDownload();
  serverStartCancelled.value = true;
});
</script>

<template>
  <div class="ai-agents">
    <div class="page-header">
      <div>
        <h1>Assistant IA</h1>
        <p class="page-subtitle">IA locale — 100% portable, aucune installation sur le PC client</p>
      </div>
    </div>

    <!-- Backend selector -->
    <div class="backend-selector">
      <button :class="['backend-btn', { active: backend === 'llamacpp' }]" @click="backend = 'llamacpp'">
        <Zap :size="18" style="flex-shrink:0;color:var(--accent-primary)" />
        <div>
          <div class="bk-title">llama.cpp Portable <NBadge variant="success" style="font-size:10px;margin-left:6px">Recommandé</NBadge></div>
          <div class="bk-sub">Binaire unique + fichier .gguf — zéro installation</div>
        </div>
      </button>
      <button :class="['backend-btn', { active: backend === 'ollama' }]" @click="backend = 'ollama'">
        <Server :size="18" style="flex-shrink:0" />
        <div>
          <div class="bk-title">Ollama</div>
          <div class="bk-sub">Requiert Ollama installé sur le système</div>
        </div>
        <div class="status-dot" :class="{ ok: ollamaConnected }" />
      </button>
    </div>

    <div class="ai-layout">
      <!-- Sidebar -->
      <div class="ai-sidebar">

        <!-- ── llama.cpp mode ── -->
        <template v-if="backend === 'llamacpp'">

          <!-- ÉTAPE 1 : Moteur IA -->
          <NCard>
            <template #header>
              <div class="section-header"><Package :size="15" /><span>Étape 1 — Moteur IA</span></div>
            </template>
            <div class="config-section">
              <div class="portable-badge">
                <CheckCircle :size="12" style="color:var(--success)" />
                <span>100% portable — aucune installation système</span>
              </div>

              <div v-if="serverBin" class="step-ok"><CheckCircle :size="14" /> Moteur prêt <span class="ok-badge">✓ Dans le dossier app</span></div>
              <div v-else class="step-missing"><XCircle :size="14" /> Moteur IA non téléchargé</div>

              <NProgress v-if="serverDlProgress && !serverDlProgress.done"
                :value="serverDlProgress.percent" :max="100" :glow="true" style="height:6px" />
              <div v-if="serverDlProgress && !serverDlProgress.done" class="dl-info">
                {{ serverDlProgress.downloaded_mb.toFixed(0) }} MB / {{ serverDlProgress.total_mb.toFixed(0) }} MB
              </div>

              <NButton v-if="!serverBin" variant="primary" size="sm" :disabled="downloadingServer" @click="downloadServer">
                <NSpinner v-if="downloadingServer" :size="13" /><Download v-else :size="13" />
                {{ downloadingServer ? "Téléchargement..." : "Télécharger le moteur IA" }}
              </NButton>
              <NButton v-else variant="ghost" size="sm" @click="downloadServer">
                <RefreshCw :size="13" /> Mettre à jour
              </NButton>
              <p class="step-note">Le moteur se télécharge dans <code>logiciel/AI/</code> — rien n'est installé sur Windows.</p>
            </div>
          </NCard>

          <!-- ÉTAPE 2 : Modèle IA -->
          <NCard>
            <template #header>
              <div class="section-header"><Cpu :size="15" /><span>Étape 2 — Modèle IA</span></div>
            </template>
            <div class="config-section">
              <!-- Modèles locaux -->
              <NDropdown v-if="ggufOptions.length" :options="ggufOptions" v-model="selectedGguf" placeholder="Sélectionner..." />

              <!-- Catalogue de téléchargement -->
              <div class="catalog">
                <div v-for="entry in catalog" :key="entry.id" class="catalog-item" :class="{ recommended: entry.recommended }">
                  <div class="catalog-info">
                    <div class="catalog-name">
                      {{ entry.name }}
                      <NBadge v-if="entry.recommended" variant="warning" style="font-size:9px;margin-left:4px">★ Recommandé</NBadge>
                    </div>
                    <div class="catalog-desc">{{ entry.description }}</div>
                    <div class="catalog-meta">
                      <span class="catalog-size">{{ fmtSize(entry.size_gb) }}</span>
                      <span class="meta-sep">·</span>
                      <span class="catalog-ram">RAM {{ entry.ram_gb }} GB</span>
                      <template v-if="entry.vram_gb > 0">
                        <span class="meta-sep">·</span>
                        <span class="catalog-vram">VRAM {{ entry.vram_gb }} GB</span>
                      </template>
                    </div>
                  </div>
                  <div class="catalog-action">
                    <NProgress v-if="modelDlProgress(entry) && !modelDlProgress(entry).done"
                      :value="modelDlProgress(entry).percent" :max="100" style="height:4px;width:60px" />
                    <NButton v-else variant="secondary" size="sm"
                      :disabled="downloadingModel === entry.id" @click="downloadModel(entry)">
                      <NSpinner v-if="downloadingModel === entry.id" :size="12" /><Download v-else :size="12" />
                    </NButton>
                  </div>
                </div>
              </div>

              <div style="display:flex;gap:6px;align-items:center">
                <NButton variant="ghost" size="sm" @click="browseModelFile" style="flex:1">
                  <FolderOpen :size="12" /> Parcourir...
                </NButton>
                <NButton variant="ghost" size="sm" @click="scanGgufModels"><RefreshCw :size="12" /></NButton>
              </div>
            </div>
          </NCard>

          <!-- ÉTAPE 3 : Démarrer -->
          <NCard>
            <template #header>
              <div class="section-header"><Play :size="15" /><span>Étape 3 — Démarrer</span></div>
            </template>
            <div class="config-section">
              <div class="server-status" :class="{ running: serverRunning }">
                <template v-if="serverChecking"><NSpinner :size="13" /> Vérification...</template>
                <template v-else-if="serverRunning"><CheckCircle :size="13" /> Serveur actif</template>
                <template v-else><XCircle :size="13" /> Serveur arrêté</template>
              </div>
              <div style="display:flex;gap:8px">
                <NButton v-if="!serverRunning" variant="primary" size="sm" style="flex:1"
                  :disabled="startingServer || !selectedGguf || !serverBin" @click="startServer">
                  <NSpinner v-if="startingServer" :size="13" /><Play v-else :size="13" />
                  {{ startingServer ? "Chargement..." : "Démarrer" }}
                </NButton>
                <NButton v-else variant="danger" size="sm" style="flex:1" @click="stopServer">
                  <Square :size="13" /> Arrêter
                </NButton>
                <NButton variant="ghost" size="sm" @click="checkLlamacppStatus"><RefreshCw :size="13" /></NButton>
              </div>
              <p v-if="!serverBin || !selectedGguf" style="font-size:11px;color:var(--text-muted)">
                Complétez les étapes 1 et 2 avant de démarrer.
              </p>
            </div>
          </NCard>
        </template>

        <!-- ── Ollama mode ── -->
        <NCard v-else>
          <template #header>
            <div class="section-header">
              <Cpu :size="15" /><span>Modèle Ollama</span>
              <span style="margin-left:auto" :style="{ color: ollamaConnected ? 'var(--success)' : 'var(--danger)' }">
                <CheckCircle v-if="ollamaConnected" :size="14" /><XCircle v-else :size="14" />
              </span>
            </div>
          </template>
          <div class="config-section">
            <NDropdown :options="ollamaModels" v-model="selectedOllama" placeholder="Sélectionner..." />
            <NButton variant="secondary" size="sm" :loading="ollamaModelsLoading" @click="loadOllamaModels">
              <RefreshCw :size="13" /> Rafraîchir
            </NButton>
            <div v-if="!ollamaConnected" class="hint-box">
              Ollama non détecté.<br>
              <code>ollama serve</code> puis <code>ollama pull llama3.2</code>
            </div>
          </div>
        </NCard>

        <!-- Paramètres -->
        <NCard>
          <template #header><div class="section-header"><Settings :size="15" /><span>Paramètres</span></div></template>
          <div class="config-section">
            <div class="setting-row">
              <label class="setting-label">Limite conversations</label>
              <span class="setting-value">{{ maxConversations }}</span>
            </div>
            <input
              type="range"
              v-model.number="maxConversations"
              min="10" max="200" step="10"
              class="setting-slider"
              @change="saveMaxConversations"
            />
            <div class="setting-hint">Conversations max sauvegardées en localStorage</div>
          </div>
        </NCard>

        <!-- Prompt système (commun) -->
        <NCard>
          <template #header><div class="section-header"><Settings :size="15" /><span>Prompt système</span></div></template>
          <div class="config-section">
            <NDropdown :options="systemPromptOptions" v-model="selectedPromptType" placeholder="Type de prompt" />
            <textarea v-if="selectedPromptType === 'custom'" v-model="customSystemPrompt"
              class="prompt-textarea" placeholder="Entrez votre prompt système..." rows="4" />
            <div v-else class="prompt-preview">{{ activeSystemPrompt }}</div>
          </div>
        </NCard>
      </div>

      <!-- Chat -->
      <NCard class="chat-card">
        <template #header>
          <div class="section-header">
            <MessageSquare :size="15" /><span>Conversation</span>
            <span v-if="activeModel" style="margin-left:8px;font-size:11px;color:var(--text-muted)">{{ activeModel }}</span>
            <div style="margin-left:auto;display:flex;gap:6px">
              <NButton variant="ghost" size="sm" @click="showHistory = true">
                <History :size="13" /> ({{ conversations.length }})
              </NButton>
              <NButton variant="ghost" size="sm" @click="exportConversations" title="Export JSON conversations">
                <Download :size="13" /> Export JSON
              </NButton>
              <NButton v-if="messages.length" variant="ghost" size="sm" @click="resetCurrentConversation" title="Réinitialiser la conversation">
                <RotateCcw :size="13" />
              </NButton>
              <NButton variant="ghost" size="sm" @click="newConversation" title="Nouvelle conversation">
                <Plus :size="13" />
              </NButton>
              <NButton v-if="messages.length" variant="secondary" size="sm" @click="clearChat">Effacer</NButton>
            </div>
          </div>
        </template>
        <div ref="chatRef" class="chat-messages">
          <div v-if="!messages.length" class="chat-empty">
            <Bot :size="40" style="opacity:.2" />
            <p>Posez une question sur la maintenance système</p>
            <div class="suggestion-chips">
              <button class="chip" @click="userInput='Mon PC est lent, que faire ?'; sendMessage()">Mon PC est lent</button>
              <button class="chip" @click="userInput='Comment vérifier les fichiers système ?'; sendMessage()">Vérifier fichiers système</button>
              <button class="chip" @click="userInput='Comment optimiser le démarrage ?'; sendMessage()">Optimiser démarrage</button>
            </div>
          </div>
          <template v-for="(msg, i) in messages" :key="i">
            <div class="message" :class="msg.role">
              <div class="message-avatar">
                <Bot v-if="msg.role === 'assistant'" :size="16" /><MessageSquare v-else :size="16" />
              </div>
              <div class="message-content">
                <div class="message-role">{{ msg.role === 'assistant' ? 'Assistant' : 'Vous' }}</div>
                <div class="message-text" v-html="formatMessage(msg.content)" />
                <div v-if="msg.command" class="command-block">
                  <div class="command-header"><Terminal :size="13" /><span>Commande suggérée</span></div>
                  <pre class="command-code">{{ msg.command }}</pre>
                  <div class="command-actions">
                    <NButton variant="primary" size="sm" :loading="msg.commandRunning"
                      :disabled="msg.commandRunning || !!msg.commandResult" @click="executeCommand(i)">
                      <Play :size="13" /> Exécuter
                    </NButton>
                  </div>
                  <div v-if="msg.commandResult" class="command-result"><pre>{{ msg.commandResult }}</pre></div>
                </div>
              </div>
            </div>
          </template>
          <div v-if="sending" class="message assistant">
            <div class="message-avatar"><Bot :size="16" /></div>
            <div class="message-content" style="display:flex;align-items:center;gap:8px">
              <NSpinner :size="15" /><span style="font-size:12px;color:var(--text-muted)">Réflexion en cours...</span>
            </div>
          </div>
        </div>
        <div class="chat-input-area">
          <textarea v-model="userInput" class="chat-input" placeholder="Posez votre question..." rows="2" @keydown="handleKeydown" />
          <NButton variant="primary" :disabled="!userInput.trim() || sending" :loading="sending" @click="sendMessage">
            <Send :size="16" />
          </NButton>
        </div>
      </NCard>
    </div>
  </div>

  <!-- Modal historique conversations -->
  <NModal :open="showHistory" @close="showHistory = false" title="Historique des conversations">
    <div v-if="conversations.length === 0" style="text-align:center;padding:30px;color:var(--text-muted);font-size:13px">
      <History :size="32" style="opacity:.3;margin-bottom:10px" />
      <p>Aucune conversation sauvegardée.<br>Démarrez une conversation pour qu'elle apparaisse ici.</p>
    </div>
    <div v-else style="display:flex;flex-direction:column;gap:8px;max-height:420px;overflow-y:auto">
      <div v-for="c in conversations" :key="c.id" class="conv-item" :class="{ 'conv-active': c.id === currentConvId }">
        <div class="conv-info" @click="loadConversation(c)" style="cursor:pointer;flex:1">
          <div class="conv-title">{{ c.title }}</div>
          <div style="display:flex;align-items:center;gap:6px;margin-top:3px">
            <span style="font-size:10px;color:var(--text-muted)"><Clock :size="10" style="display:inline" /> {{ c.created_at }}</span>
            <NBadge variant="neutral" style="font-size:9px">{{ c.messages.length }} msgs</NBadge>
            <span v-if="c.model" style="font-size:10px;color:var(--text-muted)">· {{ c.model.split('/').pop() }}</span>
          </div>
        </div>
        <NButton variant="danger" size="sm" @click.stop="deleteConversation(c.id)"><Trash2 :size="12" /></NButton>
      </div>
    </div>
    <template #footer>
      <NButton variant="ghost" @click="conversations = []; persistConversations()" style="margin-right:auto;color:var(--danger)">
        <Trash2 :size="13" /> Tout supprimer
      </NButton>
      <NButton variant="ghost" @click="showHistory = false">Fermer</NButton>
    </template>
  </NModal>
</template>

<style scoped>
.ai-agents { display:flex; flex-direction:column; gap:16px; }
.page-header h1 { font-size:22px; font-weight:700; }
.page-subtitle { color:var(--text-muted); font-size:13px; margin-top:2px; }

.backend-selector { display:grid; grid-template-columns:1fr 1fr; gap:10px; }
.backend-btn { display:flex; align-items:center; gap:12px; padding:14px 16px; border:2px solid var(--border); border-radius:10px; background:var(--bg-secondary); cursor:pointer; text-align:left; transition:all .15s; color:var(--text-primary); }
.backend-btn:hover { border-color:var(--accent-muted); }
.backend-btn.active { border-color:var(--accent-primary); background:color-mix(in srgb,var(--accent-primary) 8%,var(--bg-secondary)); }
.bk-title { font-weight:600; font-size:13px; display:flex; align-items:center; }
.bk-sub { font-size:11px; color:var(--text-muted); margin-top:2px; }
.status-dot { width:10px; height:10px; border-radius:50%; background:var(--danger); margin-left:auto; flex-shrink:0; }
.status-dot.ok { background:var(--success); }

.ai-layout { display:grid; grid-template-columns:300px 1fr; gap:16px; align-items:start; }
@media (max-width:1100px) { .ai-layout { grid-template-columns:1fr; } }
.ai-sidebar { display:flex; flex-direction:column; gap:14px; }
.section-header { display:flex; align-items:center; gap:8px; width:100%; font-size:13px; font-weight:600; }
.config-section { display:flex; flex-direction:column; gap:10px; }

/* Steps */
.portable-badge { display:flex; align-items:center; gap:5px; font-size:11px; color:var(--success); background:var(--success-muted); padding:5px 8px; border-radius:5px; }
.step-ok { display:flex; align-items:center; gap:6px; font-size:12px; color:var(--success); font-weight:500; }
.step-missing { display:flex; align-items:center; gap:6px; font-size:12px; color:var(--danger); }
.ok-badge { background:var(--success-muted); color:var(--success); font-size:10px; padding:1px 6px; border-radius:4px; margin-left:4px; }
.dl-info { font-size:11px; color:var(--text-muted); text-align:right; }
.step-note { font-size:10px; color:var(--text-muted); line-height:1.5; margin:0; }
.step-note code { color:var(--accent-primary); font-size:10px; }

/* Catalog */
.catalog { display:flex; flex-direction:column; gap:8px; max-height:260px; overflow-y:auto; padding-right:4px; scrollbar-width:thin; scrollbar-color:var(--border) transparent; }
.catalog-item { display:flex; align-items:center; justify-content:space-between; gap:10px; padding:10px 12px; border:1px solid var(--border); border-radius:8px; background:var(--bg-tertiary); transition:border-color .15s; }
.catalog-item.recommended { border-color:color-mix(in srgb,var(--warning) 40%,var(--border)); }
.catalog-item:hover { border-color:var(--accent-muted); }
.catalog-info { flex:1; min-width:0; }
.catalog-name { font-size:12px; font-weight:600; display:flex; align-items:center; }
.catalog-desc { font-size:11px; color:var(--text-muted); margin-top:2px; line-height:1.4; }
.catalog-meta { display:flex; align-items:center; gap:5px; margin-top:4px; flex-wrap:wrap; }
.catalog-size { font-size:11px; color:var(--accent-primary); font-weight:500; font-family:monospace; }
.catalog-ram { font-size:10px; color:var(--text-muted); font-weight:500; }
.catalog-vram { font-size:10px; color:#a78bfa; font-weight:600; }
.meta-sep { font-size:10px; color:var(--border-hover); }
.catalog-action { flex-shrink:0; }

/* Server status */
.server-status { display:flex; align-items:center; gap:6px; font-size:12px; font-weight:500; padding:7px 10px; border-radius:6px; background:var(--danger-muted); color:var(--danger); }
.server-status.running { background:var(--success-muted); color:var(--success); }

.hint-box { font-size:12px; color:var(--text-muted); line-height:1.6; padding:10px; background:var(--bg-tertiary); border-radius:6px; }
.hint-box code { color:var(--accent-primary); background:var(--bg-primary); padding:1px 5px; border-radius:3px; font-size:11px; }
.prompt-textarea { width:100%; padding:10px; background:var(--bg-primary); border:1px solid var(--border); border-radius:var(--radius-md); color:var(--text-primary); font-family:inherit; font-size:12px; resize:vertical; outline:none; }
.prompt-textarea:focus { border-color:var(--accent-primary); }
.prompt-preview { font-size:12px; color:var(--text-muted); line-height:1.5; padding:10px; background:var(--bg-tertiary); border-radius:var(--radius-md); }

/* Chat */
.chat-card { display:flex; flex-direction:column; }
.chat-messages { min-height:400px; max-height:560px; overflow-y:auto; padding:8px 0; display:flex; flex-direction:column; gap:12px; }
.chat-empty { display:flex; flex-direction:column; align-items:center; justify-content:center; gap:12px; height:300px; color:var(--text-muted); font-size:13px; }
.suggestion-chips { display:flex; gap:8px; flex-wrap:wrap; justify-content:center; }
.chip { padding:6px 14px; border-radius:99px; border:1px solid var(--border); background:var(--bg-tertiary); color:var(--text-secondary); font-family:inherit; font-size:12px; cursor:pointer; transition:all .15s; }
.chip:hover { border-color:var(--accent-primary); color:var(--accent-primary); }
.message { display:flex; gap:10px; padding:8px 12px; border-radius:var(--radius-md); }
.message.user { background:var(--bg-tertiary); }
.message-avatar { width:28px; height:28px; border-radius:var(--radius-md); display:flex; align-items:center; justify-content:center; flex-shrink:0; background:var(--bg-secondary); color:var(--text-muted); }
.message.assistant .message-avatar { background:var(--accent-muted); color:var(--accent-primary); }
.message-content { flex:1; min-width:0; }
.message-role { font-size:11px; font-weight:600; color:var(--text-muted); margin-bottom:4px; text-transform:uppercase; letter-spacing:.3px; }
.message-text { font-size:13px; color:var(--text-primary); line-height:1.6; }
.message-text :deep(.code-block) { background:var(--bg-primary); border:1px solid var(--border); border-radius:var(--radius-md); padding:10px 12px; margin:8px 0; font-family:"JetBrains Mono",monospace; font-size:12px; overflow-x:auto; }
.message-text :deep(.inline-code) { background:var(--bg-tertiary); padding:1px 6px; border-radius:4px; font-family:"JetBrains Mono",monospace; font-size:12px; }
.command-block { margin-top:10px; border:1px solid var(--border); border-radius:var(--radius-md); overflow:hidden; }
.command-header { display:flex; align-items:center; gap:6px; padding:7px 12px; background:var(--bg-tertiary); font-size:11px; font-weight:500; color:var(--text-muted); text-transform:uppercase; }
.command-code { padding:10px 12px; background:var(--bg-primary); font-family:"JetBrains Mono",monospace; font-size:12px; color:var(--accent-primary); margin:0; white-space:pre-wrap; }
.command-actions { padding:8px 12px; background:var(--bg-tertiary); border-top:1px solid var(--border); }
.command-result { padding:10px 12px; border-top:1px solid var(--border); background:var(--bg-primary); }
.command-result pre { font-family:"JetBrains Mono",monospace; font-size:12px; color:var(--success); margin:0; white-space:pre-wrap; }
.chat-input-area { display:flex; gap:8px; align-items:flex-end; padding-top:12px; margin-top:12px; border-top:1px solid var(--border); }
.chat-input { flex:1; padding:10px 12px; background:var(--bg-primary); border:1px solid var(--border); border-radius:var(--radius-md); color:var(--text-primary); font-family:inherit; font-size:13px; resize:none; outline:none; }
.chat-input:focus { border-color:var(--accent-primary); }

/* Paramètres slider */
.setting-row { display:flex; justify-content:space-between; align-items:center; font-size:12px; color:var(--text-secondary); }
.setting-label { font-weight:500; }
.setting-value { font-weight:700; color:var(--accent-primary); font-family:monospace; }
.setting-slider { width:100%; accent-color:var(--accent-primary); cursor:pointer; height:4px; }
.setting-hint { font-size:10px; color:var(--text-muted); }

/* Historique conversations */
.conv-item { display:flex; align-items:flex-start; gap:10px; padding:10px 12px; border:1px solid var(--border); border-radius:8px; background:var(--bg-secondary); transition:border-color .15s; }
.conv-item:hover { border-color:var(--accent-muted); }
.conv-item.conv-active { border-color:var(--accent-primary); background:color-mix(in srgb,var(--accent-primary) 6%,var(--bg-secondary)); }
.conv-info { flex:1; min-width:0; }
.conv-title { font-size:12px; font-weight:500; color:var(--text-primary); white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
</style>
