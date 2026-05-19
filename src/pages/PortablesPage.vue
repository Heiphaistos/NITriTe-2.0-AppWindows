<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NSearchBar from "@/components/ui/NSearchBar.vue";
import { useNotificationStore } from "@/stores/notifications";
import { PORTABLE_APPS, PORTABLE_CATEGORIES } from "@/data/portableApps";
import type { PortableApp } from "@/data/portableApps";
import {
  Package, Download, Play, FolderOpen, Star,
  HardDrive, FileCode, Globe, Wrench, Film,
  FileText, Trash2, Shield, Image, RefreshCw, Database,
  Clock,
} from "lucide-vue-next";

const notify = useNotificationStore();

const installedMap   = ref<Record<string, boolean>>({});
const search         = ref("");
const activeCategory = ref("Tous");
const showPopular    = ref(false);
const loading        = ref(false);

// 'local' = local icon failed, try remote; 'all' = both failed, show lucide
const iconFailed = ref<Record<string, 'local' | 'all'>>({});

// ── Favoris (localStorage) ────────────────────────────────────
const FAV_KEY    = "nitrite-portables-fav";
const RECENT_KEY = "nitrite-portables-recent";

function safeParse<T>(key: string, fallback: T): T {
  try { const v = localStorage.getItem(key); return v ? JSON.parse(v) : fallback; }
  catch { return fallback; }
}
const favorites = ref<Set<string>>(new Set(safeParse<string[]>(FAV_KEY, [])));
const recentIds  = ref<string[]>(safeParse<string[]>(RECENT_KEY, []));

function saveFavorites() {
  localStorage.setItem(FAV_KEY, JSON.stringify([...favorites.value]));
}
function saveRecent() {
  localStorage.setItem(RECENT_KEY, JSON.stringify(recentIds.value));
}

function toggleFavorite(app: PortableApp) {
  if (favorites.value.has(app.id)) {
    favorites.value.delete(app.id);
  } else {
    favorites.value.add(app.id);
  }
  favorites.value = new Set(favorites.value);
  saveFavorites();
}

function trackRecent(app: PortableApp) {
  recentIds.value = [app.id, ...recentIds.value.filter(id => id !== app.id)].slice(0, 5);
  saveRecent();
}

// ── Catalogue local enrichi ────────────────────────────────────
const apps = ref<PortableApp[]>(PORTABLE_APPS);

// ── Icônes par catégorie ──────────────────────────────────────
const CAT_ICONS: Record<string, any> = {
  Système:       HardDrive,
  Réseau:        Globe,
  Développement: FileCode,
  Utilitaires:   Wrench,
  Multimédia:    Film,
  Bureautique:   FileText,
  Nettoyage:     Trash2,
  Sécurité:      Shield,
  Graphisme:     Image,
  Récupération:  Database,
};
const catIcon = (cat: string) => CAT_ICONS[cat] ?? Package;

// Retourne l'URL de l'icône : local d'abord, puis remote, puis "" (→ lucide icon)
function portableIconSrc(app: PortableApp): string {
  const state = iconFailed.value[app.id];
  if (!state) {
    return `/icons/portables/${app.id}.png`;
  }
  if (state === 'local' && app.url) {
    try {
      const host = new URL(app.url).hostname;
      return `https://www.google.com/s2/favicons?domain=${host}&sz=32`;
    } catch { return ""; }
  }
  return ""; // affiche lucide
}

function onPortableIconError(event: Event, app: PortableApp) {
  const src = (event.target as HTMLImageElement).src;
  if (src.includes('/icons/portables/')) {
    // Essai local échoué → tente remote
    iconFailed.value = { ...iconFailed.value, [app.id]: 'local' };
  } else {
    // Remote aussi échoué → affiche lucide
    iconFailed.value = { ...iconFailed.value, [app.id]: 'all' };
    (event.target as HTMLImageElement).style.display = 'none';
  }
}

// ── Compteurs par catégorie ───────────────────────────────────
const categoryCounts = computed(() => {
  const counts: Record<string, number> = { Tous: apps.value.length };
  for (const a of apps.value) {
    counts[a.category] = (counts[a.category] ?? 0) + 1;
  }
  return counts;
});

// ── Filtres ───────────────────────────────────────────────────
const filteredApps = computed(() => {
  let result = apps.value;
  if (activeCategory.value !== "Tous") {
    result = result.filter((a) => a.category === activeCategory.value);
  }
  if (showPopular.value) {
    result = result.filter((a) => a.popular);
  }
  const q = search.value.toLowerCase().trim();
  if (q) {
    result = result.filter(
      (a) =>
        a.name.toLowerCase().includes(q) ||
        a.description.toLowerCase().includes(q) ||
        a.category.toLowerCase().includes(q)
    );
  }
  return result;
});

const popularCount = computed(() => apps.value.filter((a) => a.popular).length);

const favoriteApps = computed(() =>
  apps.value.filter(a => favorites.value.has(a.id))
);

const recentApps = computed(() =>
  recentIds.value.map(id => apps.value.find(a => a.id === id)).filter(Boolean) as PortableApp[]
);

function hasVersionMismatch(app: PortableApp): boolean {
  if (!app.version || !installedMap.value[app.id]) return false;
  const installed = (installedMap.value as any)[`${app.id}_version`];
  return installed ? installed !== app.version : false;
}

// ── Actions ───────────────────────────────────────────────────
async function openDownload(app: PortableApp) {
  if (!app.url) {
    notify.warning(`Pas d'URL pour ${app.name}`);
    return;
  }
  try {
    await invoke("open_url", { url: app.url });
    notify.info(`Ouverture de la page de téléchargement pour ${app.name}`);
  } catch {
    notify.error("Impossible d'ouvrir le lien de téléchargement");
  }
}

async function launchApp(app: PortableApp) {
  try {
    await invoke("launch_portable", { appId: app.id });
    trackRecent(app);
    notify.success(`${app.name} lancé`);
  } catch (e: any) {
    notify.error(e?.toString() || `Impossible de lancer ${app.name}`);
  }
}

async function openPortablesFolder() {
  try {
    await invoke("open_portables_dir");
  } catch {
    notify.error("Impossible d'ouvrir le dossier");
  }
}

async function refreshInstalled() {
  loading.value = true;
  const results = await Promise.allSettled(
    apps.value.map(app => invoke<boolean>("check_portable_installed", { appId: app.id }))
  );
  const map: Record<string, boolean> = {};
  apps.value.forEach((app, i) => {
    const r = results[i];
    map[app.id] = r.status === "fulfilled" ? r.value : false;
  });
  installedMap.value = map;
  loading.value = false;
}
</script>

<template>
  <div class="portables-page">

    <!-- En-tête ─────────────────────────────────────── -->
    <div class="port-header">
      <div>
        <h1><Package :size="20" /> Applications Portables</h1>
        <p class="port-subtitle">
          {{ apps.length }} applications • {{ popularCount }} recommandées — aucune installation requise
        </p>
      </div>
      <div class="port-header-actions">
        <NButton variant="secondary" size="sm" @click="openPortablesFolder">
          <FolderOpen :size="13" /> Dossier logiciel/
        </NButton>
        <NButton variant="ghost" size="sm" @click="refreshInstalled" :loading="loading">
          <RefreshCw :size="13" /> Vérifier installées
        </NButton>
      </div>
    </div>

    <!-- Filtres ─────────────────────────────────────── -->
    <div class="port-filters">
      <NSearchBar v-model="search" placeholder="Rechercher parmi {{ apps.length }} applications…" />
      <div class="port-cats">
        <button
          v-for="cat in PORTABLE_CATEGORIES" :key="cat"
          class="port-cat-btn"
          :class="{ active: activeCategory === cat && !showPopular }"
          @click="activeCategory = cat; showPopular = false"
        >
          <component :is="cat === 'Tous' ? Package : catIcon(cat)" :size="12" />
          {{ cat }}
          <span class="port-cat-count">{{ categoryCounts[cat] ?? 0 }}</span>
        </button>
        <button
          class="port-cat-btn port-cat-popular"
          :class="{ active: showPopular }"
          @click="showPopular = !showPopular; if (showPopular) activeCategory = 'Tous'"
        >
          <Star :size="12" />
          Recommandées
          <span class="port-cat-count">{{ popularCount }}</span>
        </button>
      </div>
    </div>

    <!-- Résultats count ─────────────────────────────── -->
    <div class="port-results-bar">
      <span>{{ filteredApps.length }} application{{ filteredApps.length > 1 ? 's' : '' }}</span>
      <span v-if="search" class="port-clear-search" @click="search = ''">✕ Effacer</span>
    </div>

    <!-- Section Récemment lancées ───────────────────── -->
    <template v-if="recentApps.length > 0 && !search">
      <div class="port-section-title"><Clock :size="13" /> Récemment lancées</div>
      <div class="port-grid port-grid--compact">
        <div
          v-for="app in recentApps"
          :key="`recent-${app.id}`"
          class="port-card"
          :class="{ 'port-card--installed': installedMap[app.id] }"
        >
          <div class="port-card-icon">
            <component :is="catIcon(app.category)" :size="22" />
          </div>
          <div class="port-card-body">
            <div class="port-card-top">
              <span class="port-card-name">{{ app.name }}</span>
              <div class="port-card-badges">
                <NBadge variant="neutral" size="sm">{{ app.category }}</NBadge>
              </div>
            </div>
            <div class="port-card-bottom" style="margin-top:4px">
              <span class="port-card-size">{{ app.size }}</span>
              <div class="port-card-actions">
                <NButton v-if="installedMap[app.id]" variant="primary" size="sm" @click="launchApp(app)">
                  <Play :size="12" /> Lancer
                </NButton>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- Section Favoris ─────────────────────────────── -->
    <template v-if="favoriteApps.length > 0 && !search">
      <div class="port-section-title"><Star :size="13" style="color:#eab308" /> Favoris</div>
      <div class="port-grid port-grid--compact">
        <div
          v-for="app in favoriteApps"
          :key="`fav-${app.id}`"
          class="port-card port-card--fav"
          :class="{ 'port-card--installed': installedMap[app.id] }"
        >
          <div class="port-card-icon">
            <component :is="catIcon(app.category)" :size="22" />
            <Star class="port-popular-star" :size="10" />
          </div>
          <div class="port-card-body">
            <div class="port-card-top">
              <span class="port-card-name">{{ app.name }}</span>
              <div class="port-card-badges">
                <NBadge v-if="installedMap[app.id]" variant="success" size="sm">✓</NBadge>
                <NBadge v-if="hasVersionMismatch(app)" variant="warning" size="sm">Update dispo</NBadge>
              </div>
            </div>
            <div class="port-card-bottom" style="margin-top:4px">
              <span class="port-card-size">{{ app.size }}</span>
              <div class="port-card-actions">
                <NButton v-if="installedMap[app.id]" variant="primary" size="sm" @click="launchApp(app)">
                  <Play :size="12" /> Lancer
                </NButton>
                <NButton variant="ghost" size="sm" @click="toggleFavorite(app)" title="Retirer des favoris">
                  <Star :size="12" style="color:#eab308;fill:#eab308" />
                </NButton>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- Grille d'apps principale ────────────────────── -->
    <div class="port-grid">
      <div
        v-for="app in filteredApps"
        :key="app.id"
        class="port-card"
        :class="{ 'port-card--installed': installedMap[app.id] }"
      >
        <div class="port-card-icon">
          <img
            v-if="portableIconSrc(app)"
            :src="portableIconSrc(app)"
            :alt="app.name"
            class="port-favicon"
            loading="lazy"
            @error="onPortableIconError($event, app)"
          />
          <component
            v-if="!portableIconSrc(app)"
            :is="catIcon(app.category)"
            :size="22"
          />
          <Star v-if="app.popular" class="port-popular-star" :size="10" />
        </div>

        <div class="port-card-body">
          <div class="port-card-top">
            <span class="port-card-name">{{ app.name }}</span>
            <div class="port-card-badges">
              <NBadge v-if="installedMap[app.id]" variant="success" size="sm">✓ Installée</NBadge>
              <NBadge v-if="hasVersionMismatch(app)" variant="warning" size="sm">Update dispo</NBadge>
              <NBadge variant="neutral" size="sm">{{ app.category }}</NBadge>
            </div>
          </div>
          <p class="port-card-desc">{{ app.description }}</p>
          <div class="port-card-bottom">
            <span class="port-card-size">{{ app.size }}</span>
            <div class="port-card-actions">
              <NButton
                v-if="installedMap[app.id]"
                variant="primary"
                size="sm"
                @click="launchApp(app)"
              >
                <Play :size="12" /> Lancer
              </NButton>
              <NButton
                variant="secondary"
                size="sm"
                @click="openDownload(app)"
              >
                <Download :size="12" />
                {{ installedMap[app.id] ? 'MAJ' : 'Télécharger' }}
              </NButton>
              <NButton
                variant="ghost"
                size="sm"
                @click="toggleFavorite(app)"
                :title="favorites.has(app.id) ? 'Retirer des favoris' : 'Ajouter aux favoris'"
              >
                <Star :size="12" :style="favorites.has(app.id) ? 'color:#eab308;fill:#eab308' : 'color:var(--text-muted)'" />
              </NButton>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- État vide ───────────────────────────────────── -->
    <div v-if="filteredApps.length === 0" class="port-empty">
      <Package :size="36" style="color:var(--text-muted);opacity:0.3" />
      <p>Aucune application ne correspond à <strong>{{ search || activeCategory }}</strong></p>
      <NButton variant="ghost" size="sm" @click="search=''; activeCategory='Tous'; showPopular=false">
        Réinitialiser les filtres
      </NButton>
    </div>

  </div>
</template>

<style scoped>
.portables-page {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

/* ── Header ────────────────────────────────── */
.port-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 10px;
}
.port-header h1 {
  font-size: 20px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
}
.port-subtitle { color: var(--text-muted); font-size: 12px; margin-top: 3px; }
.port-header-actions { display: flex; gap: 6px; }

/* ── Filtres ────────────────────────────────── */
.port-filters {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.port-cats {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}
.port-cat-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 5px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  color: var(--text-muted);
  font-family: inherit;
  font-size: 11.5px;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.port-cat-btn:hover { background: var(--bg-tertiary); color: var(--text-secondary); }
.port-cat-btn.active {
  background: var(--accent-muted);
  color: var(--accent-primary);
  border-color: rgba(249,115,22,0.4);
}
.port-cat-count {
  background: var(--bg-elevated);
  color: var(--text-muted);
  font-size: 10px;
  padding: 1px 5px;
  border-radius: 99px;
  font-weight: 600;
  min-width: 18px;
  text-align: center;
}
.port-cat-btn.active .port-cat-count {
  background: rgba(249,115,22,0.2);
  color: var(--accent-primary);
}
.port-cat-popular.active {
  background: rgba(234, 179, 8, 0.1);
  color: #eab308;
  border-color: rgba(234, 179, 8, 0.4);
}
.port-cat-popular.active .port-cat-count {
  background: rgba(234, 179, 8, 0.2);
  color: #eab308;
}

/* ── Barre résultats ────────────────────────── */
.port-results-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 11.5px;
  color: var(--text-muted);
}
.port-clear-search {
  cursor: pointer;
  color: var(--accent-primary);
  font-weight: 500;
}
.port-clear-search:hover { text-decoration: underline; }

/* ── Grille ─────────────────────────────────── */
.port-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 8px;
}

/* ── Card ───────────────────────────────────── */
.port-card {
  display: flex;
  gap: 12px;
  padding: 12px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  transition: border-color 0.15s, background 0.15s, transform 0.12s;
  cursor: default;
}
.port-card:hover {
  border-color: var(--border-hover);
  background: var(--bg-tertiary);
  transform: translateY(-1px);
}
.port-card--installed { border-color: rgba(74, 222, 128, 0.25); }

.port-card-icon {
  width: 44px;
  height: 44px;
  border-radius: var(--radius-md);
  background: var(--accent-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--accent-primary);
  position: relative;
  overflow: hidden;
}
.port-favicon {
  width: 28px;
  height: 28px;
  object-fit: contain;
  border-radius: 4px;
}
.port-popular-star {
  position: absolute;
  top: -4px;
  right: -4px;
  background: #eab308;
  color: #000;
  border-radius: 50%;
  padding: 2px;
  width: 14px;
  height: 14px;
}

.port-card-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}
.port-card-top {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.port-card-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
}
.port-card-badges { display: flex; gap: 4px; flex-wrap: wrap; margin-left: auto; }
.port-card-desc {
  font-size: 11.5px;
  color: var(--text-muted);
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.port-card-bottom {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 2px;
}
.port-card-size {
  font-size: 11px;
  color: var(--text-muted);
  font-family: "JetBrains Mono", monospace;
}
.port-card-actions { display: flex; gap: 4px; }

/* ── Section titles ────────────────────────── */
.port-section-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  padding: 4px 0 2px;
  border-bottom: 1px solid var(--border);
}

.port-grid--compact {
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
}

.port-card--fav {
  border-color: rgba(234, 179, 8, 0.3);
}

/* ── État vide ──────────────────────────────── */
.port-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 48px 24px;
  text-align: center;
  color: var(--text-muted);
}
.port-empty p { font-size: 14px; }
</style>
