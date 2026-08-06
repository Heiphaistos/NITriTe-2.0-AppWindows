<script setup lang="ts">
import { ref, nextTick, onMounted } from "vue";
import { invoke, isTauriContext } from "@/utils/invoke";
import type { ShellResult } from "@/types/diagnostic";
import NButton from "@/components/ui/NButton.vue";
import { Terminal, Trash2, Send, Clock, ChevronRight } from "lucide-vue-next";

interface OutputLine {
  timestamp: string;
  type: "cmd" | "stdout" | "stderr" | "info";
  text: string;
}

interface ShellInfo {
  id: string;
  name: string;
  path: string;
  available: boolean;
}

// ─── Quick commands ───────────────────────────────────────────────────────────
const QUICK_COMMANDS: { label: string; cmd: string; shell?: string }[] = [
  { label: "IP Config",      cmd: "ipconfig /all" },
  { label: "Ping Google",    cmd: "ping -n 4 8.8.8.8" },
  { label: "Processus",      cmd: "tasklist /fo table" },
  { label: "Disques",        cmd: "wmic logicaldisk get caption,freespace,size" },
  { label: "Temp CPU",       cmd: "wmic /namespace:\\\\root\\wmi PATH MSAcpi_ThermalZoneTemperature get CurrentTemperature" },
  { label: "Uptime",         cmd: "net statistics workstation | findstr /i statistics" },
  { label: "Ports ouverts",  cmd: "netstat -ano | findstr LISTENING" },
  { label: "Vars env",       cmd: "set", shell: "cmd" },
  { label: "Services actifs",cmd: "sc query type= all state= running" },
  { label: "Événements récents", cmd: "wevtutil qe System /c:5 /rd:true /f:text" },
];

const HISTORY_KEY = "nitrite_terminal_history";
const MAX_HISTORY = 100;

const input = ref("");
const output = ref<OutputLine[]>([]);
const outputEl = ref<HTMLDivElement | null>(null);
const inputEl = ref<HTMLInputElement | null>(null);
const history = ref<string[]>([]);
const historyIndex = ref(-1);
const running = ref(false);
const shells = ref<ShellInfo[]>([]);
const activeShell = ref("cmd");
const showQuickPanel = ref(false);

// ─── Persistance historique ───────────────────────────────────────────────────
function loadHistory() {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    if (raw) history.value = JSON.parse(raw) as string[];
  } catch { /* ignore */ }
  historyIndex.value = history.value.length;
}

function saveHistory() {
  try {
    // Garder les MAX_HISTORY dernières entrées
    const trimmed = history.value.slice(-MAX_HISTORY);
    history.value = trimmed;
    localStorage.setItem(HISTORY_KEY, JSON.stringify(trimmed));
  } catch { /* ignore */ }
}

function clearHistory() {
  history.value = [];
  historyIndex.value = 0;
  localStorage.removeItem(HISTORY_KEY);
}

// ─── Shells ───────────────────────────────────────────────────────────────────
async function loadShells() {
  try {
    shells.value = await invoke<ShellInfo[]>("detect_shells");
    const available = shells.value.filter(s => s.available);
    if (available.length > 0) activeShell.value = available[0].id;
  } catch (e: unknown) {
    if (!isTauriContext()) {
      shells.value = [
        { id: "cmd", name: "CMD", path: "cmd.exe", available: true },
        { id: "powershell", name: "PowerShell", path: "powershell.exe", available: true },
      ];
    } else {
      shells.value = [];
      const { useNotificationStore } = await import("@/stores/notifications");
      useNotificationStore().error("Shells", (e instanceof Error ? e.message : String(e)).slice(0, 120));
    }
  }
}

function getShellLabel(id: string): string {
  return shells.value.find(s => s.id === id)?.name ?? id;
}

function now(): string {
  return new Date().toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

function scrollToBottom() {
  nextTick(() => {
    if (outputEl.value) {
      outputEl.value.scrollTop = outputEl.value.scrollHeight;
    }
  });
}

function pushLine(type: OutputLine["type"], text: string) {
  output.value.push({ timestamp: now(), type, text });
  scrollToBottom();
}

// ─── Exécution ────────────────────────────────────────────────────────────────
async function executeCommand(cmd?: string) {
  const raw = (cmd ?? input.value).trim();
  if (!raw || running.value) return;

  pushLine("cmd", raw);

  // Eviter les doublons consécutifs dans l'historique
  if (history.value[history.value.length - 1] !== raw) {
    history.value.push(raw);
    saveHistory();
  }
  historyIndex.value = history.value.length;
  if (!cmd) input.value = "";
  running.value = true;

  try {
    const result = await invoke<ShellResult>("run_in_shell", {
      shellId: activeShell.value,
      command: raw,
    });

    const stdout = result?.stdout ?? "";
    const stderr = result?.stderr ?? "";

    if (stdout) {
      for (const line of stdout.split("\n")) {
        if (line.trim()) pushLine("stdout", line.trimEnd());
      }
    }
    if (stderr) {
      for (const line of stderr.split("\n")) {
        if (line.trim()) pushLine("stderr", line.trimEnd());
      }
    }
    if (!stdout && !stderr) {
      pushLine("info", "Commande executee (aucune sortie)");
    }
  } catch (e: unknown) {
    pushLine("stderr", `Erreur: ${(e instanceof Error ? e.message : String(e)) || "Commande echouee"}`);
  }

  running.value = false;
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === "Enter") {
    executeCommand();
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "l") {
    // Documenté dans KeyboardShortcutsModal.vue ("Ctrl+L : Vider la console")
    // mais jamais réellement câblé — clearOutput() n'était accessible que via
    // le bouton, le raccourci annoncé ne faisait donc rien.
    e.preventDefault();
    clearOutput();
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    if (historyIndex.value > 0) {
      historyIndex.value--;
      input.value = history.value[historyIndex.value];
    }
  } else if (e.key === "ArrowDown") {
    e.preventDefault();
    if (historyIndex.value < history.value.length - 1) {
      historyIndex.value++;
      input.value = history.value[historyIndex.value];
    } else {
      historyIndex.value = history.value.length;
      input.value = "";
    }
  }
}

function clearOutput() {
  output.value = [];
  pushLine("info", "Terminal efface");
}

function focusInput() {
  nextTick(() => inputEl.value?.focus());
}

function onShellChange() {
  focusInput();
}

function runQuickCommand(qc: { label: string; cmd: string; shell?: string }) {
  if (qc.shell) activeShell.value = qc.shell;
  executeCommand(qc.cmd);
  showQuickPanel.value = false;
  focusInput();
}

onMounted(() => {
  loadShells();
  loadHistory();
  pushLine("info", `NiTriTe Terminal — ${history.value.length} commande(s) dans l'historique. Tapez ou choisissez un raccourci.`);
});
</script>

<template>
  <div class="terminal-page">
    <div class="page-header">
      <div>
        <h1>Terminal</h1>
        <p class="page-subtitle">Executeur de commandes systeme</p>
      </div>
      <div class="header-actions">
        <!-- Shell selector -->
        <div class="shell-selector">
          <label class="shell-label">Shell :</label>
          <select v-model="activeShell" class="shell-select" @change="onShellChange">
            <option
              v-for="shell in shells.filter(s => s.available)"
              :key="shell.id"
              :value="shell.id"
            >
              {{ shell.name }}
            </option>
          </select>
        </div>
        <!-- Quick commands toggle -->
        <NButton variant="secondary" size="sm" @click="showQuickPanel = !showQuickPanel" :class="{ 'btn-active': showQuickPanel }">
          <ChevronRight :size="14" :style="{ transform: showQuickPanel ? 'rotate(90deg)' : 'none', transition: 'transform .2s' }" />
          Raccourcis
        </NButton>
        <!-- Historique infos + effacer -->
        <NButton variant="secondary" size="sm" @click="clearHistory" title="Effacer l'historique persistant">
          <Clock :size="14" />
          {{ history.length }}
        </NButton>
        <NButton variant="secondary" size="sm" @click="clearOutput">
          <Trash2 :size="14" />
          Effacer
        </NButton>
      </div>
    </div>

    <!-- Quick commands panel -->
    <div v-if="showQuickPanel" class="quick-panel">
      <div class="quick-panel-title">Commandes rapides</div>
      <div class="quick-grid">
        <button
          v-for="qc in QUICK_COMMANDS"
          :key="qc.cmd"
          class="quick-chip"
          :title="qc.cmd"
          @click="runQuickCommand(qc)"
        >
          <ChevronRight :size="11" />
          {{ qc.label }}
        </button>
      </div>
    </div>

    <div class="terminal-container">
      <div class="terminal-output" ref="outputEl">
        <div
          v-for="(line, i) in output"
          :key="i"
          class="output-line"
          :class="`output-${line.type}`"
        >
          <span class="line-time">{{ line.timestamp }}</span>
          <span v-if="line.type === 'cmd'" class="line-prompt">&gt;</span>
          <span v-else-if="line.type === 'stderr'" class="line-prefix">ERR</span>
          <span v-else-if="line.type === 'info'" class="line-prefix info">INF</span>
          <span class="line-text">{{ line.text }}</span>
        </div>
        <div v-if="output.length === 0" class="terminal-empty">
          <Terminal :size="32" />
          <span>Pret. Tapez une commande ci-dessous.</span>
        </div>
      </div>

      <div class="terminal-input">
        <span class="shell-badge">{{ getShellLabel(activeShell) }}</span>
        <span class="input-prompt">&gt;</span>
        <input
          ref="inputEl"
          v-model="input"
          type="text"
          class="cmd-input"
          placeholder="Entrez une commande... (↑↓ historique)"
          :disabled="running"
          @keydown="handleKeyDown"
          autofocus
        />
        <NButton variant="primary" size="sm" :loading="running" @click="executeCommand()">
          <Send :size="14" />
        </NButton>
      </div>
    </div>
  </div>
</template>

<style scoped>
.terminal-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
  height: 100%;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 8px;
}

.page-header h1 { font-size: 24px; font-weight: 700; }
.page-subtitle { color: var(--text-muted); font-size: 13px; margin-top: 2px; }
.header-actions { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }

/* Quick commands panel */
.quick-panel {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 12px 14px;
}
.quick-panel-title {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: .07em;
  color: var(--text-muted);
  margin-bottom: 10px;
}
.quick-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.quick-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.quick-chip:hover {
  background: var(--accent-muted);
  border-color: rgba(249,115,22,.4);
  color: var(--accent-primary);
}

.btn-active {
  background: var(--accent-muted) !important;
  border-color: rgba(249,115,22,.4) !important;
  color: var(--accent-primary) !important;
}

.terminal-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: #0d1117;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
  min-height: 400px;
}

.terminal-output {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  font-family: "JetBrains Mono", "Cascadia Code", "Consolas", monospace;
  font-size: 13px;
  line-height: 1.6;
}

.terminal-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  height: 100%;
  color: #484f58;
  min-height: 200px;
}

.output-line {
  display: flex;
  gap: 8px;
  /* word-wrap activé : les longues lignes passent à la ligne */
  white-space: pre-wrap;
  word-break: break-word;
  overflow-wrap: break-word;
}

.line-time {
  color: #484f58;
  flex-shrink: 0;
  font-size: 11px;
  min-width: 60px;
}

.line-prompt {
  color: #58a6ff;
  font-weight: 700;
  flex-shrink: 0;
}

.line-prefix {
  color: #f85149;
  font-weight: 600;
  font-size: 11px;
  flex-shrink: 0;
  min-width: 28px;
}

.line-prefix.info {
  color: #58a6ff;
}

.output-cmd .line-text { color: #c9d1d9; font-weight: 500; }
.output-stdout .line-text { color: #8b949e; }
.output-stderr .line-text { color: #f85149; }
.output-info .line-text { color: #58a6ff; font-style: italic; }

/* La zone .line-text doit aussi wrapper */
.line-text {
  flex: 1;
  min-width: 0;
  word-break: break-word;
  overflow-wrap: break-word;
}

.terminal-input {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid #21262d;
  background: #010409;
}

.input-prompt {
  color: #58a6ff;
  font-family: "JetBrains Mono", monospace;
  font-weight: 700;
  font-size: 14px;
  flex-shrink: 0;
}

.cmd-input {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  color: #c9d1d9;
  font-family: "JetBrains Mono", "Cascadia Code", "Consolas", monospace;
  font-size: 13px;
  min-width: 0;
}

.cmd-input::placeholder { color: #484f58; }
.cmd-input:disabled { opacity: 0.5; }

.shell-selector {
  display: flex;
  align-items: center;
  gap: 6px;
}

.shell-label {
  font-size: 12px;
  color: var(--text-muted);
}

.shell-select {
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-family: inherit;
  font-size: 12px;
  padding: 4px 8px;
  cursor: pointer;
}

.shell-badge {
  font-size: 10px;
  font-weight: 600;
  color: #7ee787;
  background: #1b4332;
  padding: 2px 8px;
  border-radius: 4px;
  flex-shrink: 0;
  font-family: "JetBrains Mono", monospace;
  text-transform: uppercase;
}
</style>
