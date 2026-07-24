<script setup lang="ts">
import { ref, computed, provide, onMounted, onUnmounted, onErrorCaptured, nextTick } from "vue";
import { useRoute, useRouter } from "vue-router";
import logoUrl from "@/assets/nitrite-logo.jpg";
import AppSidebar from "@/components/layout/AppSidebar.vue";
import AppHeader from "@/components/layout/AppHeader.vue";
import AppStatusBar from "@/components/layout/AppStatusBar.vue";
import NToast from "@/components/ui/NToast.vue";
import NAlertBanner from "@/components/ui/NAlertBanner.vue";
import SearchModal from "@/components/shared/SearchModal.vue";
import KeyboardShortcutsModal from "@/components/ui/KeyboardShortcutsModal.vue";
import { useAppStore } from "@/stores/app";
import { useLayoutStore } from "@/stores/layoutStore";
import { useDataCache } from "@/stores/dataCache";
import { useProactiveAlerts } from "@/composables/useProactiveAlerts";
import { logger } from "@/utils/logger";
import { sdiRelease } from "@/utils/sdiGuard";

const { start: startAlerts, stop: stopAlerts } = useProactiveAlerts();
const appVersion = __APP_VERSION__;
function handleKeyDown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === "k") { e.preventDefault(); searchOpen.value = !searchOpen.value; }
  if ((e.ctrlKey || e.metaKey) && e.key === "b") { e.preventDefault(); toggleSidebar(); localStorage.setItem("nitrite-sidebar", String(sidebarCollapsed.value)); }
  if (e.key === "?" && !e.ctrlKey && !e.metaKey && !e.altKey) {
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag !== "INPUT" && tag !== "TEXTAREA" && tag !== "SELECT") { e.preventDefault(); shortcutsOpen.value = !shortcutsOpen.value; }
  }
}

onUnmounted(() => { stopAlerts(); window.removeEventListener("keydown", handleKeyDown); });

const route       = useRoute();
const router      = useRouter();

// Pages exécutant des opérations longues (scan, mises à jour, install, nettoyage…) :
// mises en cache par <keep-alive> pour que l'opération et son état survivent à la
// navigation (sinon le composant est détruit et l'UI se réinitialise). Les pages
// purement « live » (Monitoring, Températures, Dashboard…) ne sont PAS listées et
// se rafraîchissent normalement à chaque entrée.
const persistentPages = [
  "DiagnosticPage", "OptimizationsPage", "MasterInstallPage", "UpdatesPage",
  "DriversPage", "BackupPage", "UninstallerPage", "ClonePage", "DataRecoveryPage",
  "ScanVirusPage", "BenchmarkPage", "CleanerPage", "TurboModePage",
  "DependencyManagerPage", "DuplicateFinderPage", "BigFilesFinderPage",
  "DiskVisualizerPage", "WinPEModePage", "RestorePointsPage", "PortScannerPage",
];
const appContent  = ref<HTMLElement | null>(null);
const appStore    = useAppStore();
const layoutStore = useLayoutStore();
const dataCache   = useDataCache();

const sidebarCollapsed = ref(false);
const searchOpen       = ref(false);
const shortcutsOpen    = ref(false);
const appReady         = ref(false);

// Vidéo de fond du splash : une fois terminée (elle ne boucle pas), on la
// démonte et on affiche sa dernière frame en image statique — évite de
// garder un <video> arrêté en mémoire si le chargement traîne encore.
const splashVideoEnded = ref(false);

// ── Preloader ──────────────────────────────────────────────────────────────────
interface LoadTask { label: string; status: "pending" | "running" | "done" | "error" }

const loadTasks = ref<LoadTask[]>([
  { label: "Interface & thème",              status: "pending" },
  { label: "Monitoring système",             status: "pending" },
  { label: "Informations CPU",               status: "pending" },
  { label: "Informations mémoire RAM",       status: "pending" },
  { label: "Disques & volumes",              status: "pending" },
  { label: "Réseau & connexions",            status: "pending" },
  { label: "GPU & affichage",                status: "pending" },
  { label: "Comptes utilisateurs",           status: "pending" },
  { label: "Logiciels installés",            status: "pending" },
  { label: "Pilotes système",                status: "pending" },
  { label: "Processus actifs",               status: "pending" },
  { label: "Services Windows",               status: "pending" },
  { label: "Journaux d'événements",          status: "pending" },
  { label: "Pare-feu & règles réseau",       status: "pending" },
  { label: "Licence & activation",           status: "pending" },
  { label: "Historique BSOD",                status: "pending" },
  { label: "Points de restauration",         status: "pending" },
  { label: "Bluetooth & périphériques",      status: "pending" },
  { label: "Partages réseau",                status: "pending" },
  { label: "Certificats système",            status: "pending" },
  { label: "Tâches planifiées",              status: "pending" },
  { label: "Variables d'environnement",      status: "pending" },
  { label: "Informations BIOS & carte mère", status: "pending" },
  { label: "Assistant IA",                   status: "pending" },
]);

const doneCount    = computed(() => loadTasks.value.filter(t => t.status === "done" || t.status === "error").length);
const loadProgress = computed(() => Math.round((doneCount.value / loadTasks.value.length) * 100));
const currentLabel = computed(() => {
  const running = loadTasks.value.find(t => t.status === "running");
  return running?.label ?? (loadProgress.value === 100 ? "Prêt !" : "Chargement…");
});

// ── Layout ────────────────────────────────────────────────────────────────────
function toggleSidebar() { sidebarCollapsed.value = !sidebarCollapsed.value; }
function openSearch()    { searchOpen.value = true; }

provide("sidebarCollapsed", sidebarCollapsed);
provide("toggleSidebar",    toggleSidebar);
provide("openSearch",       openSearch);

const isRightSidebar      = computed(() => layoutStore.state.sidebarPosition === "right");
const currentSidebarWidth = computed(() => sidebarCollapsed.value ? 64 : layoutStore.sidebarWidthPx);

// ── Scroll reset automatique à chaque navigation ─────────────────────────────
router.afterEach(() => {
  pageError.value = null;
  if (appContent.value) appContent.value.scrollTop = 0;
});

// ── Capture d'erreurs pages (diagnostic écrans noirs) ────────────────────────
const pageError = ref<{ message: string; stack?: string } | null>(null);

onErrorCaptured((err: unknown, _instance, info) => {
  const msg = err instanceof Error ? err.message : String(err);
  const stack = err instanceof Error ? err.stack : undefined;
  console.error("[Nitrite] Erreur Vue capturée :", info, err);
  logger.vue(info, err);
  pageError.value = { message: `[${info}] ${msg}`, stack };
  return false;
});

// Capture les erreurs de chargement de route (import() raté)
router.onError((err) => {
  const msg = err instanceof Error ? err.message : String(err);
  console.error("[Nitrite] Erreur Router :", err);
  logger.router(err);
  pageError.value = { message: `[router] ${msg}`, stack: err instanceof Error ? err.stack : undefined };
});

// ── Démarrage ─────────────────────────────────────────────────────────────────
onMounted(async () => {
  startAlerts(60000);
  // Masquer le préloader HTML natif dès que Vue est prêt
  window.__hideNativeBoot?.();

  await nextTick();

  // ── Tâche 0 : Interface (synchrone) ──
  loadTasks.value[0].status = "running";
  appStore.loadSavedTheme();
  appStore.loadSidebarState();
  sidebarCollapsed.value = appStore.sidebarCollapsed;
  layoutStore.applyToDocument();
  window.addEventListener("keydown", handleKeyDown);
  loadTasks.value[0].status = "done";

  // ── Import Tauri ──
  let inv: ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null = null;
  try { const api = await import("@tauri-apps/api/core"); inv = api.invoke; } catch { /* dev */ }

  // ── Wrapper : marque la tâche, invoque, met en cache, marque done ──
  const load = async (idx: number, cmd: string, args?: Record<string, unknown>) => {
    loadTasks.value[idx].status = "running";
    if (inv) {
      try {
        const key   = args ? `${cmd}::${JSON.stringify(args)}` : cmd;
        const result = await inv(cmd, args);
        dataCache.set(key, result);
      } catch { /* non critique — la page re-fetchera si besoin */ }
      loadTasks.value[idx].status = "done";
    } else {
      await new Promise(r => setTimeout(r, 300));
      loadTasks.value[idx].status = "done";
    }
  };

  // ── Tâche 1 : Monitoring (doit démarrer en premier, fournit les events) ──
  await load(1, "start_monitoring");

  // ── Tâches 2-23 : 3 batches avec yields pour garder le thread libre ──────
  // Batch A — infos système de base (les plus rapides)
  await Promise.allSettled([
    load(2,  "get_system_info"),
    load(3,  "get_ram_detailed"),
    load(4,  "get_storage_physical_info"),
    load(5,  "get_network_overview"),
    load(6,  "get_gpu_detailed"),
    load(7,  "get_user_accounts"),
  ]);
  // Yield réel → libère le thread UI entre chaque batch
  await new Promise(r => setTimeout(r, 0));

  // Batch B — données moyennement lourdes
  await Promise.allSettled([
    load(8,  "get_apps"),
    load(9,  "get_sys_drivers_list"),
    load(10, "get_running_processes"),
    load(11, "get_windows_services"),
    load(12, "get_event_logs", { logName: "System", count: 50 }),
    load(13, "get_firewall_rules"),
  ]);
  await new Promise(r => setTimeout(r, 0));

  // Batch C — données secondaires / lentes
  await Promise.allSettled([
    load(14, "get_windows_license"),
    load(15, "get_bsod_history"),
    load(16, "list_restore_points_cmd"),
    load(17, "get_bluetooth_info"),
    load(18, "get_network_shares"),
    load(19, "get_certificates"),
    load(20, "get_scheduled_tasks"),
    load(21, "get_environment_variables"),
    load(22, "get_bios_info"),
    load(23, "ai_find_llamacpp_server"),
  ]);

  // ── Handler fermeture ──
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    await win.listen("tauri://close-requested", async () => {
      if (window.__nitrite_sdi_active) { sdiRelease(); return; }
      try { if (inv) await inv("cleanup_on_exit"); } catch { await win.destroy(); }
    });
  } catch { /* dev */ }

  // Transition vers l'app
  await new Promise(r => setTimeout(r, 400));
  appReady.value = true;
});
</script>

<template>
  <!-- ── Preloader ── -->
  <Transition name="splash">
    <div v-if="!appReady" class="splash-screen">

      <!-- ── Fond vidéo (sans son) — se fige sur la dernière image si le
           chargement dure plus longtemps que la vidéo ── -->
      <video
        v-if="!splashVideoEnded"
        class="splash-bg-media"
        src="/splash.mp4"
        autoplay
        muted
        playsinline
        disablepictureinpicture
        preload="auto"
        @ended="splashVideoEnded = true"
      />
      <img
        v-else
        src="/splash-last-frame.jpg"
        class="splash-bg-media"
        alt=""
      />

      <!-- ── Fond statique ── -->
      <div class="splash-overlay" />

      <!-- ── Mascotte flottante (pseudo-3D) — attend la fin du chargement ── -->
      <div class="splash-mascot-stage" :class="{ 'is-ready': appReady }">
        <div class="splash-mascot-orbit">
          <img :src="logoUrl" class="splash-mascot" alt="Nitrite" />
        </div>
        <div class="splash-mascot-shadow" />
      </div>

      <!-- ── Panel de chargement (bas de l'écran) ── -->
      <div class="splash-content">

        <!-- Logo + titre -->
        <div class="splash-brand">
          <div class="splash-brand-text">
            <div class="splash-title">NiTriTe</div>
            <div class="splash-version">v{{ appVersion }}</div>
          </div>
        </div>

        <!-- Barre de progression -->
        <div class="splash-progress-wrap">
          <div class="splash-progress-bar">
            <div class="splash-progress-fill" :style="{ width: `${loadProgress}%` }" />
          </div>
          <div class="splash-pct">{{ loadProgress }}%</div>
        </div>

        <!-- Label tâche active -->
        <div class="splash-label">{{ currentLabel }}</div>

        <!-- Grille de tâches (compact) -->
        <div class="splash-grid">
          <div
            v-for="(task, i) in loadTasks"
            :key="i"
            class="splash-task"
            :class="task.status"
          >
            <span class="task-icon">
              <svg v-if="task.status === 'done'" width="10" height="10" viewBox="0 0 10 10">
                <circle cx="5" cy="5" r="5" fill="#22c55e" fill-opacity="0.2"/>
                <path d="M2.5 5l1.7 1.7L7.5 3.3" stroke="#22c55e" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
              </svg>
              <svg v-else-if="task.status === 'running'" class="spin" width="10" height="10" viewBox="0 0 10 10">
                <circle cx="5" cy="5" r="4" stroke="#f97316" stroke-width="1.5" stroke-dasharray="16" stroke-dashoffset="8" stroke-linecap="round" fill="none"/>
              </svg>
              <span v-else class="task-dot" />
            </span>
            <span class="task-label">{{ task.label }}</span>
          </div>
        </div>

        <!-- Compteur -->
        <div class="splash-counter">{{ doneCount }} / {{ loadTasks.length }} modules chargés</div>
      </div>

    </div>
  </Transition>

  <NAlertBanner />

  <!-- ── Application ── -->
  <div
    v-if="appReady"
    class="app-layout"
    :class="[`sidebar-pos-${layoutStore.state.sidebarPosition}`, `density-${layoutStore.state.density}`]"
    :data-density="layoutStore.state.density"
    :data-sidebar-pos="layoutStore.state.sidebarPosition"
  >
    <AppSidebar
      :collapsed="sidebarCollapsed"
      :position="layoutStore.state.sidebarPosition"
      :width="layoutStore.state.sidebarWidth"
      :mode="layoutStore.state.sidebarMode"
      @toggle="toggleSidebar"
    />
    <div
      class="app-main"
      :class="{ 'sidebar-collapsed': sidebarCollapsed }"
      :style="{
        [isRightSidebar ? 'marginRight' : 'marginLeft']: `${currentSidebarWidth}px`,
        [isRightSidebar ? 'marginLeft'  : 'marginRight']: '0',
      }"
    >
      <AppHeader v-if="layoutStore.state.headerVisible" @open-search="openSearch" />
      <main ref="appContent" class="app-content" :style="{ padding: `${layoutStore.state.contentPadding}px` }">
        <div
          class="app-content-inner"
          :style="{ maxWidth: layoutStore.state.contentMaxWidth === 'full' ? '100%' : layoutStore.state.contentMaxWidth, margin: '0 auto' }"
        >
          <router-view v-slot="{ Component }">
            <transition name="page">
              <keep-alive :include="persistentPages">
                <component :is="Component" :key="route.path" />
              </keep-alive>
            </transition>
          </router-view>

          <!-- ── Overlay diagnostic erreur page ── -->
          <div v-if="pageError" class="page-error-overlay">
            <div class="page-error-box">
              <div class="page-error-title">⚠ Erreur de rendu détectée</div>
              <div class="page-error-msg">{{ pageError.message }}</div>
              <pre v-if="pageError.stack" class="page-error-stack">{{ pageError.stack }}</pre>
              <button class="page-error-dismiss" @click="pageError = null">Fermer</button>
            </div>
          </div>
        </div>
      </main>
      <AppStatusBar />
    </div>
  </div>

  <NToast />
  <SearchModal v-model="searchOpen" />
  <KeyboardShortcutsModal v-model="shortcutsOpen" />
</template>

<style scoped>
/* ── Splash ───────────────────────────────────────────────────────────────── */
.splash-screen {
  position: fixed; inset: 0; z-index: 99999;
  background: #09090b; overflow: hidden;
}

/* Vidéo/image de fond — plein écran, sous l'overlay et le contenu */
.splash-bg-media {
  position: absolute; inset: 0; z-index: 0;
  width: 100%; height: 100%;
  object-fit: cover; object-position: center;
  pointer-events: none;
}

/* Overlay — assombrit la vidéo pour garder le texte lisible, glow orange au centre */
.splash-overlay {
  position: absolute; inset: 0; z-index: 1;
  background:
    radial-gradient(ellipse at 50% 30%, rgba(249,115,22,0.10) 0%, transparent 60%),
    linear-gradient(to bottom, rgba(9,9,11,0.45) 0%, rgba(9,9,11,0.55) 45%, rgba(9,9,11,0.92) 100%);
  pointer-events: none;
}

/* ── Mascotte flottante — pseudo-3D (perspective + rotation + ombre synchronisée),
   tourne en boucle tant que appReady est faux ; petit salut final a l'arrivee ── */
.splash-mascot-stage {
  position: absolute; left: 50%; top: 32%; transform: translate(-50%, -50%);
  z-index: 2;
  display: flex; flex-direction: column; align-items: center; gap: 18px;
  perspective: 800px;
}
.splash-mascot-orbit {
  transform-style: preserve-3d;
  /* !important : le mascotte est une boucle decorative discrete (flottement
     lent, faible amplitude) — sans ça la regle globale "reduire les
     animations" (animations.css, elle-meme en !important) l'ecrasait a
     0.01ms des le premier essai reel, la figeant en plein milieu de l'ecran
     de demarrage comme un simple logo colle. Le reste de l'app continue de
     respecter la preference d'accessibilite normalement. */
  animation: mascot-float 3.2s ease-in-out infinite !important;
}
.splash-mascot-stage.is-ready .splash-mascot-orbit {
  animation: mascot-greet 600ms ease-out 1 !important;
}
.splash-mascot {
  width: 160px; height: 160px; object-fit: contain;
  filter: drop-shadow(0 18px 20px rgba(0,0,0,0.45));
}
.splash-mascot-shadow {
  width: 90px; height: 16px; border-radius: 50%;
  background: radial-gradient(ellipse, rgba(249,115,22,0.45) 0%, transparent 70%);
  animation: mascot-shadow 3.2s ease-in-out infinite !important;
}
.splash-mascot-stage.is-ready .splash-mascot-shadow { animation: none !important; opacity: 0.35; }

@keyframes mascot-float {
  0%, 100% { transform: translateY(0) rotateY(-10deg) rotateZ(-2deg); }
  50%      { transform: translateY(-18px) rotateY(10deg) rotateZ(2deg); }
}
@keyframes mascot-shadow {
  0%, 100% { transform: scale(1);    opacity: 0.45; }
  50%      { transform: scale(0.75); opacity: 0.25; }
}
@keyframes mascot-greet {
  0%   { transform: translateY(-18px) rotateY(10deg) scale(1); }
  40%  { transform: translateY(-28px) rotateY(-14deg) scale(1.08); }
  70%  { transform: translateY(0)     rotateY(8deg)   scale(1); }
  100% { transform: translateY(0)     rotateY(0deg)   scale(1); }
}

/* ── Panel chargement (bas de l'écran) ── */
.splash-content {
  position: absolute; bottom: 0; left: 0; right: 0;
  z-index: 2;
  display: flex; flex-direction: column; align-items: center; gap: 8px;
  padding: 24px 32px 32px;
  animation: splash-in 400ms ease forwards;
}
@keyframes splash-in { from { opacity: 0; transform: translateY(12px); } to { opacity: 1; transform: translateY(0); } }

/* Logo + titre alignés en ligne */
.splash-brand {
  display: flex; align-items: center; gap: 14px; margin-bottom: 4px;
}

.splash-brand-text { display: flex; flex-direction: column; gap: 2px; }
.splash-title {
  font-size: 22px; font-weight: 800; letter-spacing: -0.5px;
  background: linear-gradient(135deg, #fafafa 40%, #f97316);
  -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text;
}
.splash-version {
  font-size: 10px; color: #52525b;
  font-family: "JetBrains Mono", monospace; letter-spacing: 0.05em;
}

/* Progress */
.splash-progress-wrap { display:flex; align-items:center; gap:8px; width:100%; max-width:520px; }
.splash-progress-bar  { flex:1; height:3px; background:rgba(255,255,255,0.1); border-radius:99px; overflow:hidden; }
.splash-progress-fill {
  height:100%; border-radius:99px;
  background: linear-gradient(90deg, #ea580c, #f97316, #fb923c);
  box-shadow: 0 0 10px rgba(249,115,22,0.6);
  transition: width 300ms cubic-bezier(0.4,0,0.2,1);
}
.splash-pct { font-size:11px; font-weight:700; color:#f97316; font-family:"JetBrains Mono",monospace; min-width:30px; text-align:right; }
.splash-label { font-size:11px; color:#71717a; min-height:15px; }

/* Grille de tâches */
.splash-grid {
  display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 2px 12px;
  width: 100%; max-width: 620px; margin-top: 4px;
  background: rgba(17,17,19,0.75);
  backdrop-filter: blur(16px);
  border: 1px solid rgba(255,255,255,0.07);
  border-radius: 10px;
  padding: 10px 14px;
}
.splash-task {
  display: flex; align-items: center; gap: 5px;
  opacity: 0.25; transition: opacity 200ms ease;
}
.splash-task.running { opacity: 1; }
.splash-task.done    { opacity: 0.55; }

.task-icon { width:12px; height:12px; display:flex; align-items:center; justify-content:center; flex-shrink:0; }
.task-dot  { width:4px; height:4px; border-radius:50%; background:#3f3f46; display:block; }
.task-label { font-size:10px; color:#a1a1aa; line-height:1.3; }
.splash-task.running .task-label { color:#f97316; font-weight:600; }
.splash-task.done    .task-label { color:#4ade80; }

.spin { animation: spin 0.9s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.splash-counter { font-size:10px; color:#52525b; font-family:"JetBrains Mono",monospace; }

.splash-leave-active { transition: opacity 500ms ease; }
.splash-leave-to { opacity: 0; }

/* ── App Layout ─────────────────────────────────────────────────────────── */
.app-layout {
  display:flex; height:100vh; overflow:hidden;
  background:var(--bg-primary); color:var(--text-primary);
  animation: app-in 350ms ease forwards;
  font-size: var(--layout-font-size, 13px);
}
@keyframes app-in { from { opacity:0 } to { opacity:1 } }
.sidebar-pos-right { flex-direction:row-reverse; }
.app-main { flex:1; display:flex; flex-direction:column; transition:margin var(--transition-normal); min-width:0; }
.app-content { flex:1; overflow-y:auto; overflow-x:hidden; }
.app-content-inner { width:100%; position: relative; }

/* transition gérée globalement — voir <style> ci-dessous */

.density-compact  :deep(.ncard) { padding: calc(var(--layout-density-pad-md, 8px) * 0.72); }
.density-spacious :deep(button.nav-item) { padding: 10px 12px; }
.density-compact  :deep(button.nav-item) { padding: 5px 8px; font-size: 11px; }

/* ── Diagnostic d'erreur page ── */
.page-error-overlay {
  position: absolute; inset: 0; z-index: 9999;
  display: flex; align-items: center; justify-content: center;
  background: rgba(0, 0, 0, 0.75); backdrop-filter: blur(4px);
  pointer-events: all;
}
.page-error-box {
  max-width: 640px; width: 90%; padding: 24px;
  background: #1c0a0a; border: 1px solid #ef4444;
  border-radius: 12px; box-shadow: 0 0 32px rgba(239,68,68,0.4);
  display: flex; flex-direction: column; gap: 12px;
}
.page-error-title { font-size: 15px; font-weight: 700; color: #ef4444; }
.page-error-msg { font-size: 13px; color: #fca5a5; word-break: break-all; }
.page-error-stack {
  font-size: 10px; color: #71717a; font-family: "JetBrains Mono", monospace;
  white-space: pre-wrap; word-break: break-all; max-height: 200px;
  overflow-y: auto; background: #0c0c0e; border-radius: 6px; padding: 8px;
}
.page-error-dismiss {
  align-self: flex-end; padding: 6px 16px; font-size: 12px;
  background: #ef4444; color: #fff; border: none; border-radius: 6px;
  cursor: pointer; font-family: inherit;
}
.page-error-dismiss:hover { background: #dc2626; }
</style>

<!-- Transition de page : fade simultané (pas de mode out-in = pas de blocage) -->
<style>
.page-enter-active { transition: opacity 120ms ease; }
.page-leave-active { transition: opacity 120ms ease; position: absolute; width: 100%; top: 0; left: 0; pointer-events: none; }
.page-enter-from   { opacity: 0; }
.page-leave-to     { opacity: 0; }
</style>
