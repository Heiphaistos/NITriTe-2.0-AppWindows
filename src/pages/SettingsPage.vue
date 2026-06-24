<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import NButton from "@/components/ui/NButton.vue";
import NInput from "@/components/ui/NInput.vue";
import NToggle from "@/components/ui/NToggle.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import { useAppStore, type ThemeName } from "@/stores/app";
import { useNotificationStore } from "@/stores/notifications";
import { useAiStore } from "@/stores/ai";
import {
  Settings, Palette, Activity, Bot, Download, Info,
  Save, RotateCcw, Wifi, CheckCircle, XCircle, FolderOpen,
  Upload, Zap, Bell, Shield, Monitor, Database,
} from "lucide-vue-next";

const appStore = useAppStore();
const notify   = useNotificationStore();
const aiStore  = useAiStore();

// ── État onglets ─────────────────────────────────────────────
type Tab = "interface" | "performance" | "notifications" | "ai" | "export" | "about";
const activeTab = ref<Tab>("interface");

// ── Paramètres locaux ─────────────────────────────────────────
// ollamaUrl / ollamaModel / temperature → proxy vers le store AI (v-model compatible)
const ollamaUrl   = computed({ get: () => aiStore.ollamaUrl,   set: v => { aiStore.ollamaUrl   = v; markChanged(); } });
const ollamaModel = computed({ get: () => aiStore.ollamaModel, set: v => { aiStore.ollamaModel = v; markChanged(); } });
const ollamaTemp  = computed({ get: () => aiStore.temperature, set: v => { aiStore.temperature = v; markChanged(); } });
const monitorInterval = ref(2000);
const processCount = ref(10);
const sidebarDefault = ref(false);
const autoSave = ref(false);
const exportFormat = ref<"json" | "txt" | "html" | "md">("json");
// Performance settings
const diskCacheEnabled = ref(true);
const gpuAcceleration = ref(true);
const backgroundTasks = ref(true);
const maxHistoryEntries = ref(100);
const autoRefreshDashboard = ref(true);
const dashboardRefreshMs = ref(3000);
// Notification settings
const notifSounds = ref(false);
const notifDesktop = ref(true);
const notifErrors = ref(true);
const notifSuccess = ref(true);
const notifPosition = ref<"top-right" | "bottom-right" | "top-left" | "bottom-left">("top-right");
const ollamaStatus = ref<"idle" | "ok" | "error">("idle");
const ollamaTesting = ref(false);
const saving = ref(false);
const changed = ref(false);

function markChanged() { changed.value = true; }

// ── Thèmes ────────────────────────────────────────────────────
const themes: { id: ThemeName; label: string; color: string }[] = [
  { id: "nitrite-dark",   label: "Nitrite Dark",      color: "#f97316" },
  { id: "cyber-blue",     label: "Cyber Blue",        color: "#3b82f6" },
  { id: "matrix-green",   label: "Matrix Green",      color: "#22c55e" },
  { id: "purple-haze",    label: "Purple Haze",       color: "#a855f7" },
  { id: "red-alert",      label: "Red Alert",         color: "#ef4444" },
  { id: "arctic-light",   label: "Arctic Light",      color: "#0ea5e9" },
  { id: "midnight-gold",  label: "Midnight Gold",     color: "#eab308" },
  { id: "neon-synthwave", label: "Neon Synthwave",    color: "#f0abfc" },
  { id: "ocean-deep",     label: "Ocean Deep",        color: "#06b6d4" },
  { id: "rose-quartz",    label: "Rose Quartz",       color: "#f43f5e" },
  { id: "void-dark",      label: "Void Dark (AMOLED)", color: "#6366f1" },
  { id: "forest-green",   label: "Forest Green",      color: "#16a34a" },
  { id: "copper-rust",    label: "Copper Rust",       color: "#d97706" },
  { id: "slate-steel",    label: "Slate Steel",       color: "#64748b" },
  { id: "inferno",        label: "Inferno",           color: "#ff4500" },
  { id: "aurora",         label: "Aurora Borealis",   color: "#00d4aa" },
  { id: "moonlight",      label: "Moonlight",         color: "#7dd3fc" },
  { id: "ember-glow",     label: "Ember Glow",        color: "#fb923c" },
  { id: "cobalt-night",   label: "Cobalt Night",      color: "#2563eb" },
  { id: "volcanic",       label: "Volcanic",          color: "#f97316" },
  { id: "sakura",         label: "Sakura (Clair)",    color: "#ec4899" },
  { id: "jade-temple",    label: "Jade Temple",       color: "#10b981" },
  { id: "hacker",         label: "Hacker Terminal",   color: "#00ff41" },
  { id: "ice-storm",      label: "Ice Storm (Clair)", color: "#0284c7" },
  { id: "custom",         label: "Custom",            color: "#6b7280" },
];

// ── Chargement config ─────────────────────────────────────────
onMounted(async () => {
  // Charge la config IA depuis Rust → propage dans le store AI
  await aiStore.loadFromConfig();
  try {
    const cfg = await invoke<any>("get_config");
    monitorInterval.value      = cfg.monitor_interval_ms    ?? 2000;
    processCount.value         = cfg.process_count          ?? 10;
    sidebarDefault.value       = cfg.sidebar_collapsed      ?? false;
    exportFormat.value         = cfg.export_format          ?? "json";
    autoSave.value             = cfg.auto_save              ?? false;
    diskCacheEnabled.value     = cfg.disk_cache_enabled     ?? true;
    gpuAcceleration.value      = cfg.gpu_acceleration       ?? true;
    backgroundTasks.value      = cfg.background_tasks       ?? true;
    maxHistoryEntries.value    = cfg.max_history_entries    ?? 100;
    autoRefreshDashboard.value = cfg.auto_refresh_dashboard ?? true;
    dashboardRefreshMs.value   = cfg.dashboard_refresh_ms  ?? 3000;
    notifSounds.value          = cfg.notif_sounds           ?? false;
    notifDesktop.value         = cfg.notif_desktop          ?? true;
    notifErrors.value          = cfg.notif_errors           ?? true;
    notifSuccess.value         = cfg.notif_success          ?? true;
    if (cfg.notif_position) notifPosition.value = cfg.notif_position;
    if (cfg.font_size) appStore.setFontSize(cfg.font_size);
    if (cfg.show_animations === false) {
      appStore.showAnimations = false;
      document.documentElement.classList.add("no-animations");
    }
  } catch { /* dev fallback */ }
});

// ── Test Ollama ───────────────────────────────────────────────
async function testOllama() {
  ollamaTesting.value = true;
  ollamaStatus.value = "idle";
  try {
    const ok = await invoke<boolean>("ai_check");
    ollamaStatus.value = ok ? "ok" : "error";
  } catch {
    ollamaStatus.value = "error";
  }
  ollamaTesting.value = false;
}

// ── Export / Import config complète ──────────────────────────
async function exportConfig() {
  try {
    const cfg = await invoke<any>("get_config");
    const payload = JSON.stringify({ ...cfg, __nitrite_version: "6.0.0", __exported_at: new Date().toISOString() }, null, 2);
    const { save } = await import("@tauri-apps/plugin-dialog");
    const { writeTextFile } = await import("@tauri-apps/plugin-fs");
    const path = await save({ defaultPath: "nitrite-config.json", filters: [{ name: "JSON", extensions: ["json"] }] });
    if (path) { await writeTextFile(path, payload); notify.success("Config exportée", path); }
  } catch (e: any) { notify.error("Export config", String(e)); }
}

async function importConfig() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const { readTextFile } = await import("@tauri-apps/plugin-fs");
    const path = await open({ filters: [{ name: "Config NiTriTe JSON", extensions: ["json"] }] });
    if (!path || Array.isArray(path)) return;
    const raw = await readTextFile(path as string);
    const cfg = JSON.parse(raw);
    delete cfg.__nitrite_version; delete cfg.__exported_at;
    await invoke("save_config", { config: cfg });
    // Appliquer dans les stores
    if (cfg.theme)          appStore.setTheme(cfg.theme);
    if (cfg.font_size)      appStore.setFontSize(cfg.font_size);
    if (cfg.ollama_url)     aiStore.ollamaUrl   = cfg.ollama_url;
    if (cfg.ollama_model)   aiStore.ollamaModel = cfg.ollama_model;
    if (cfg.ollama_temperature) aiStore.temperature = cfg.ollama_temperature;
    if (cfg.export_format)  exportFormat.value  = cfg.export_format;
    if (cfg.monitor_interval_ms) monitorInterval.value = cfg.monitor_interval_ms;
    if (cfg.process_count)  processCount.value  = cfg.process_count;
    notify.success("Config importée", "Paramètres rechargés depuis le fichier");
  } catch (e: any) { notify.error("Import config", String(e)); }
}

// ── Ouvrir dossier exports ────────────────────────────────────
async function openExportFolder() {
  try {
    const dir = await invoke<string>("get_export_dir");
    await invoke("open_path", { path: dir });
  } catch (e: any) {
    notify.error("Impossible d'ouvrir le dossier", String(e));
  }
}

// ── Sauvegarde ───────────────────────────────────────────────
async function saveSettings() {
  saving.value = true;
  try {
    await invoke("save_config", {
      config: {
        theme:                   appStore.theme,
        language:                appStore.language,
        sidebar_collapsed:       sidebarDefault.value,
        ollama_url:              aiStore.ollamaUrl,
        ollama_model:            aiStore.ollamaModel,
        ollama_temperature:      aiStore.temperature,
        monitor_interval_ms:     monitorInterval.value,
        show_animations:         appStore.showAnimations,
        compact_mode:            false,
        notifications_enabled:   true,
        process_count:           processCount.value,
        font_size:               appStore.fontSize,
        export_format:           exportFormat.value,
        auto_save:               autoSave.value,
        disk_cache_enabled:      diskCacheEnabled.value,
        gpu_acceleration:        gpuAcceleration.value,
        background_tasks:        backgroundTasks.value,
        max_history_entries:     maxHistoryEntries.value,
        auto_refresh_dashboard:  autoRefreshDashboard.value,
        dashboard_refresh_ms:    dashboardRefreshMs.value,
        notif_sounds:            notifSounds.value,
        notif_desktop:           notifDesktop.value,
        notif_errors:            notifErrors.value,
        notif_success:           notifSuccess.value,
        notif_position:          notifPosition.value,
      },
    });
    notify.success("Paramètres sauvegardés");
    changed.value = false;
  } catch (e: any) {
    notify.error("Erreur sauvegarde", String(e));
  }
  saving.value = false;
}

// ── Reset ─────────────────────────────────────────────────────
function resetDefaults() {
  appStore.setTheme("nitrite-dark");
  appStore.setFontSize("normal");
  if (!appStore.showAnimations) appStore.toggleAnimations();
  aiStore.ollamaUrl   = "http://localhost:11434";
  aiStore.ollamaModel = "llama3:8b";
  aiStore.temperature = 0.7;
  monitorInterval.value = 2000;
  processCount.value = 10;
  sidebarDefault.value = false;
  exportFormat.value = "json";
  changed.value = true;
  notify.info("Paramètres réinitialisés — cliquez Save pour confirmer");
}

const tabs: { id: Tab; label: string; icon: any }[] = [
  { id: "interface",     label: "Interface",     icon: Palette   },
  { id: "performance",   label: "Performance",   icon: Zap       },
  { id: "notifications", label: "Notifications", icon: Bell      },
  { id: "ai",            label: "IA (Ollama)",   icon: Bot       },
  { id: "export",        label: "Export",        icon: Download  },
  { id: "about",         label: "À propos",      icon: Info      },
];
</script>

<template>
  <div class="settings-page">
    <DiagBanner :icon="Settings" title="Paramètres" desc="Interface, performance, IA et exports" color="indigo" />

    <div class="settings-layout">
      <!-- Sidebar onglets -->
      <nav class="settings-nav">
        <button
          v-for="t in tabs"
          :key="t.id"
          class="settings-nav-item"
          :class="{ active: activeTab === t.id }"
          @click="activeTab = t.id"
        >
          <component :is="t.icon" :size="15" />
          <span>{{ t.label }}</span>
        </button>
      </nav>

      <!-- Contenu -->
      <div class="settings-content">

        <!-- ══ INTERFACE ══ -->
        <div v-if="activeTab === 'interface'" class="tab-section">
          <h2 class="tab-title"><Palette :size="16" /> Interface</h2>

          <div class="setting-group">
            <p class="setting-label">Thème</p>
            <div class="theme-grid">
              <button
                v-for="t in themes"
                :key="t.id"
                class="theme-btn"
                :class="{ active: appStore.theme === t.id }"
                @click="appStore.setTheme(t.id); markChanged()"
              >
                <div class="theme-swatch" :style="{ background: t.color, boxShadow: appStore.theme === t.id ? `0 0 10px ${t.color}80` : 'none' }"></div>
                <span>{{ t.label }}</span>
                <span v-if="appStore.theme === t.id" class="theme-check">✓</span>
              </button>
            </div>
          </div>

          <div class="setting-group">
            <p class="setting-label">Taille de police</p>
            <div class="btn-group">
              <button
                v-for="sz in ['small','normal','large'] as const"
                :key="sz"
                class="size-btn"
                :class="{ active: appStore.fontSize === sz }"
                @click="appStore.setFontSize(sz); markChanged()"
              >{{ sz === 'small' ? 'Petit' : sz === 'normal' ? 'Normal' : 'Grand' }}</button>
            </div>
          </div>

          <div class="setting-group">
            <div class="setting-row">
              <div>
                <p class="setting-label">Animations</p>
                <p class="setting-desc">Transitions et effets visuels</p>
              </div>
              <NToggle :modelValue="appStore.showAnimations" label="" @update:modelValue="appStore.toggleAnimations(); markChanged()" />
            </div>
          </div>

          <div class="setting-group">
            <p class="setting-label">Langue</p>
            <div class="btn-group">
              <button
                class="size-btn"
                :class="{ active: appStore.language === 'fr' }"
                @click="appStore.language = 'fr'; markChanged()"
              >Français</button>
              <button
                class="size-btn"
                :class="{ active: appStore.language === 'en' }"
                @click="appStore.language = 'en'; markChanged()"
              >English</button>
            </div>
          </div>
        </div>

        <!-- ══ PERFORMANCE ══ -->
        <div v-else-if="activeTab === 'performance'" class="tab-section">
          <h2 class="tab-title"><Zap :size="16" /> Performance</h2>

          <div class="setting-group">
            <p class="setting-label">Intervalle de monitoring</p>
            <p class="setting-desc">Fréquence de rafraîchissement des données système (en millisecondes)</p>
            <div style="display:flex;align-items:center;gap:12px;margin-top:6px">
              <input type="range" min="500" max="10000" step="500" v-model.number="monitorInterval"
                @input="markChanged" class="range-slider" style="flex:1" />
              <span class="range-value">{{ (monitorInterval/1000).toFixed(1) }}s</span>
            </div>
            <div class="range-labels"><span>0.5s (rapide)</span><span>2s</span><span>10s (économique)</span></div>
          </div>

          <div class="setting-group">
            <p class="setting-label">Nombre de processus affichés</p>
            <p class="setting-desc">Limite d'affichage dans l'onglet Processus (10–500)</p>
            <div style="display:flex;align-items:center;gap:12px;margin-top:6px">
              <input type="range" min="10" max="500" step="10" v-model.number="processCount"
                @input="markChanged" class="range-slider" style="flex:1" />
              <span class="range-value">{{ processCount }}</span>
            </div>
          </div>

          <div class="setting-group">
            <p class="setting-label">Intervalle tableau de bord</p>
            <p class="setting-desc">Rafraîchissement du Dashboard (CPU, RAM, réseau)</p>
            <div style="display:flex;align-items:center;gap:12px;margin-top:6px">
              <input type="range" min="1000" max="10000" step="1000" v-model.number="dashboardRefreshMs"
                @input="markChanged" class="range-slider" style="flex:1" />
              <span class="range-value">{{ dashboardRefreshMs/1000 }}s</span>
            </div>
          </div>

          <div class="setting-group">
            <p class="setting-label">Historique des actions</p>
            <p class="setting-desc">Nombre maximum d'entrées mémorisées dans l'historique</p>
            <div style="display:flex;align-items:center;gap:12px;margin-top:6px">
              <input type="range" min="25" max="500" step="25" v-model.number="maxHistoryEntries"
                @input="markChanged" class="range-slider" style="flex:1" />
              <span class="range-value">{{ maxHistoryEntries }}</span>
            </div>
          </div>

          <div class="setting-group">
            <div class="setting-row">
              <div>
                <p class="setting-label">Rafraîchissement auto du Dashboard</p>
                <p class="setting-desc">Actualise les données automatiquement en arrière-plan</p>
              </div>
              <NToggle :modelValue="autoRefreshDashboard" label="" @update:modelValue="autoRefreshDashboard = $event; markChanged()" />
            </div>
          </div>

          <div class="setting-group">
            <div class="setting-row">
              <div>
                <p class="setting-label">Cache disque</p>
                <p class="setting-desc">Mise en cache des données de diagnostic pour des réponses plus rapides</p>
              </div>
              <NToggle :modelValue="diskCacheEnabled" label="" @update:modelValue="diskCacheEnabled = $event; markChanged()" />
            </div>
          </div>

          <div class="setting-group">
            <div class="setting-row">
              <div>
                <p class="setting-label">Accélération GPU</p>
                <p class="setting-desc">Utilise le GPU pour le rendu de l'interface (désactiver si instable)</p>
              </div>
              <NToggle :modelValue="gpuAcceleration" label="" @update:modelValue="gpuAcceleration = $event; markChanged()" />
            </div>
          </div>

          <div class="setting-group">
            <div class="setting-row">
              <div>
                <p class="setting-label">Tâches en arrière-plan</p>
                <p class="setting-desc">Permet à Nitrite d'exécuter des tâches pendant que vous naviguez</p>
              </div>
              <NToggle :modelValue="backgroundTasks" label="" @update:modelValue="backgroundTasks = $event; markChanged()" />
            </div>
          </div>

          <div class="setting-group">
            <div class="setting-row">
              <div>
                <p class="setting-label">Barre latérale repliée par défaut</p>
                <p class="setting-desc">La sidebar démarre en mode icônes uniquement</p>
              </div>
              <NToggle :modelValue="sidebarDefault" label="" @update:modelValue="sidebarDefault = $event; markChanged()" />
            </div>
          </div>
        </div>

        <!-- ══ NOTIFICATIONS ══ -->
        <div v-else-if="activeTab === 'notifications'" class="tab-section">
          <h2 class="tab-title"><Bell :size="16" /> Notifications</h2>

          <div class="setting-group">
            <p class="setting-label">Position des notifications</p>
            <div class="btn-group">
              <button v-for="pos in ['top-right','bottom-right','top-left','bottom-left'] as const" :key="pos"
                class="size-btn" :class="{ active: notifPosition === pos }"
                @click="notifPosition = pos; markChanged()">
                {{ pos === 'top-right' ? 'Haut droite' : pos === 'bottom-right' ? 'Bas droite' : pos === 'top-left' ? 'Haut gauche' : 'Bas gauche' }}
              </button>
            </div>
          </div>

          <div class="setting-group">
            <div class="setting-row">
              <div>
                <p class="setting-label">Notifications de bureau</p>
                <p class="setting-desc">Affiche les alertes via le centre de notifications Windows</p>
              </div>
              <NToggle :modelValue="notifDesktop" label="" @update:modelValue="notifDesktop = $event; markChanged()" />
            </div>
          </div>

          <div class="setting-group">
            <div class="setting-row">
              <div>
                <p class="setting-label">Notifications d'erreur</p>
                <p class="setting-desc">Affiche une notification pour chaque erreur détectée</p>
              </div>
              <NToggle :modelValue="notifErrors" label="" @update:modelValue="notifErrors = $event; markChanged()" />
            </div>
          </div>

          <div class="setting-group">
            <div class="setting-row">
              <div>
                <p class="setting-label">Notifications de succès</p>
                <p class="setting-desc">Confirme visuellement les actions réussies</p>
              </div>
              <NToggle :modelValue="notifSuccess" label="" @update:modelValue="notifSuccess = $event; markChanged()" />
            </div>
          </div>

          <div class="setting-group">
            <div class="setting-row">
              <div>
                <p class="setting-label">Sons de notification</p>
                <p class="setting-desc">Active les sons lors de l'affichage des toasts</p>
              </div>
              <NToggle :modelValue="notifSounds" label="" @update:modelValue="notifSounds = $event; markChanged()" />
            </div>
          </div>
        </div>

        <!-- ══ IA ══ -->
        <div v-else-if="activeTab === 'ai'" class="tab-section">
          <h2 class="tab-title"><Bot :size="16" /> IA (Ollama)</h2>

          <div class="setting-group">
            <label class="setting-label">URL Ollama</label>
            <NInput v-model="ollamaUrl" placeholder="http://localhost:11434" @input="markChanged" />
          </div>

          <div class="setting-group">
            <label class="setting-label">Modèle par défaut</label>
            <NInput v-model="ollamaModel" placeholder="llama3:8b" @input="markChanged" />
          </div>

          <div class="setting-group">
            <label class="setting-label">Température (créativité) — {{ ollamaTemp.toFixed(1) }}</label>
            <input type="range" min="0" max="2" step="0.1" v-model.number="ollamaTemp"
              @input="markChanged" class="range-slider" />
            <div class="range-labels"><span>Précis (0)</span><span>Équilibré (0.7)</span><span>Créatif (2)</span></div>
          </div>

          <div class="setting-group">
            <div class="ai-test-row">
              <NButton variant="ghost" :loading="ollamaTesting" @click="testOllama">
                <Wifi :size="13" /> Tester la connexion
              </NButton>
              <div v-if="ollamaStatus !== 'idle'" class="ai-status" :class="ollamaStatus">
                <CheckCircle v-if="ollamaStatus === 'ok'" :size="14" />
                <XCircle    v-else                        :size="14" />
                <span>{{ ollamaStatus === 'ok' ? 'Connecté' : 'Hors ligne' }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- ══ EXPORT ══ -->
        <div v-else-if="activeTab === 'export'" class="tab-section">
          <h2 class="tab-title"><Download :size="16" /> Export</h2>

          <div class="setting-group">
            <p class="setting-label">Format préféré</p>
            <div class="radio-group">
              <label v-for="fmt in ['json','txt','html','md']" :key="fmt" class="radio-item">
                <input
                  type="radio"
                  :value="fmt"
                  v-model="exportFormat"
                  @change="markChanged"
                />
                <span>.{{ fmt.toUpperCase() }}</span>
              </label>
            </div>
          </div>

          <div class="setting-group">
            <p class="setting-label">Historique des exports</p>
            <NButton variant="ghost" @click="openExportFolder">
              <FolderOpen :size="13" /> Ouvrir le dossier exports
            </NButton>
          </div>

          <div class="setting-group">
            <p class="setting-label">Configuration complète</p>
            <p class="setting-hint">Exportez ou importez tous vos paramètres NiTriTe (thème, IA, préférences) vers un fichier JSON.</p>
            <div style="display:flex;gap:8px;flex-wrap:wrap;margin-top:8px">
              <NButton variant="secondary" @click="exportConfig">
                <Download :size="13" /> Exporter la config
              </NButton>
              <NButton variant="ghost" @click="importConfig">
                <Upload :size="13" /> Importer une config
              </NButton>
            </div>
          </div>
        </div>

        <!-- ══ À PROPOS ══ -->
        <div v-else-if="activeTab === 'about'" class="tab-section">
          <h2 class="tab-title"><Info :size="16" /> À propos</h2>

          <div class="about-card">
            <div class="about-badge">v6.0.0 — BETA</div>
            <p class="about-name">Nitrite 2.0</p>
            <p class="about-sub">Outil de diagnostic et maintenance Windows</p>
            <div class="about-beta-notice">
              Application en version bêta — des bugs peuvent survenir.
            </div>
          </div>

          <div class="about-rows">
            <div class="about-row"><span>Stack</span><span>Tauri v2 + Rust + Vue 3 + TypeScript</span></div>
            <div class="about-row"><span>Auteur</span><span>Momo (heiphaistos44-crypto)</span></div>
            <div class="about-row"><span>Site web</span><a class="about-link" href="https://site-web-ni-tri-te-v-2.vercel.app/" target="_blank">site-web-ni-tri-te-v-2.vercel.app</a></div>
            <div class="about-row"><span>GitHub</span><a class="about-link" href="https://github.com/heiphaistos44-crypto" target="_blank">github.com/heiphaistos44-crypto</a></div>
            <div class="about-row"><span>Contact</span><a class="about-link" href="mailto:contactnitrite@gmail.com">contactnitrite@gmail.com</a></div>
            <div class="about-row"><span>Données</span><span style="color:var(--success)">✓ 100% local — aucune télémétrie, aucune collecte de données</span></div>
            <div class="about-row"><span>Propriété</span><span>© 2025 Nitrite — Reproduction interdite sans autorisation</span></div>
          </div>

          <div class="about-actions">
            <NButton variant="ghost" @click="resetDefaults">
              <RotateCcw :size="13" /> Réinitialiser tous les paramètres
            </NButton>
          </div>
        </div>

      </div>
    </div>

    <!-- Bouton Save flottant -->
    <div class="save-fab" :class="{ visible: changed }">
      <NButton variant="primary" :loading="saving" @click="saveSettings">
        <Save :size="14" /> Sauvegarder
      </NButton>
    </div>
  </div>
</template>

<style scoped src="./SettingsPage.css"></style>
