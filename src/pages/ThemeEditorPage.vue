<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useThemeEditorStore } from "@/stores/themeEditor";
import { useLayoutStore } from "@/stores/layoutStore";
import { useNotificationStore } from "@/stores/notifications";
import ThemePreview from "@/components/theme/ThemePreview.vue";
import ThemeEditorLayoutTab from "@/components/theme/ThemeEditorLayoutTab.vue";
import ThemeEditorThemeTab from "@/components/theme/ThemeEditorThemeTab.vue";
import {
  Palette, Save, Download, Upload, Eye, EyeOff, Copy, Layout, Monitor,
} from "lucide-vue-next";

const store = useThemeEditorStore();
const layoutStore = useLayoutStore();
const notify = useNotificationStore();
const activeTab = ref<"theme" | "layout">("theme");
const fileInput = ref<HTMLInputElement | null>(null);

onMounted(() => {
  store.loadSavedThemes();
  const appContent = document.querySelector(".app-content") as HTMLElement | null;
  if (appContent) {
    appContent.dataset.prevOverflow = appContent.style.overflow;
    appContent.style.overflow = "hidden";
  }
});

onUnmounted(() => {
  if (store.globalPreviewActive) store.toggleGlobalPreview(false);
  const appContent = document.querySelector(".app-content") as HTMLElement | null;
  if (appContent) {
    appContent.style.overflow = appContent.dataset.prevOverflow ?? "";
  }
});

function handleSave() {
  store.saveTheme();
  notify.success("Thème sauvegardé", `"${store.themeName}" ajouté à vos thèmes.`);
}

function handleExport() {
  const json = store.exportTheme();
  const blob = new Blob([json], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `${store.themeName.replace(/\s+/g, "_")}.theme.json`;
  a.click();
  URL.revokeObjectURL(url);
  notify.success("Exporté", "Thème exporté en JSON.");
}

function handleImportClick() { fileInput.value?.click(); }

function handleFileImport(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0];
  if (!file) return;
  const reader = new FileReader();
  reader.onload = (ev) => {
    const result = store.importTheme(ev.target?.result as string);
    if (result.ok) notify.success("Importé", "Thème chargé avec succès.");
    else notify.error("Erreur import thème", result.error ?? "Fichier invalide");
  };
  reader.readAsText(file);
  (e.target as HTMLInputElement).value = "";
}

function toggleGlobalPreview() {
  store.toggleGlobalPreview(!store.globalPreviewActive);
  notify.info(
    store.globalPreviewActive ? "Aperçu global activé" : "Aperçu global désactivé",
    store.globalPreviewActive ? "L'app entière reflète vos changements." : "Les couleurs d'origine sont restaurées.",
  );
}

function copyCurrentTheme() {
  navigator.clipboard.writeText(store.exportTheme()).then(() => {
    notify.success("Copié", "Thème copié dans le presse-papiers.");
  });
}
</script>

<template>
  <div class="te-page">

    <!-- Header -->
    <div class="te-header">
      <div class="te-header-left">
        <div class="te-title-icon"><Palette :size="20" /></div>
        <div>
          <h1 class="te-title">Éditeur de Thème</h1>
          <p class="te-subtitle">Personnalisez chaque couleur, rayon et effet de l'interface</p>
        </div>
      </div>
      <div class="te-header-actions">
        <input v-model="store.themeName" class="te-name-input" placeholder="Nom du thème..." />
        <button class="te-action-btn" :class="{ active: store.globalPreviewActive }" @click="toggleGlobalPreview" title="Aperçu global">
          <Eye v-if="!store.globalPreviewActive" :size="15" />
          <EyeOff v-else :size="15" />
          {{ store.globalPreviewActive ? "Désactiver aperçu" : "Aperçu global" }}
        </button>
        <button class="te-action-btn" @click="copyCurrentTheme" title="Copier JSON"><Copy :size="15" /></button>
        <button class="te-action-btn" @click="handleImportClick" title="Importer JSON"><Upload :size="15" /> Importer</button>
        <button class="te-action-btn" @click="handleExport" title="Exporter JSON"><Download :size="15" /> Exporter</button>
        <button class="te-action-btn te-action-btn--primary" @click="handleSave"><Save :size="15" /> Sauvegarder</button>
      </div>
      <input ref="fileInput" type="file" accept=".json" style="display:none" @change="handleFileImport" />
    </div>

    <!-- Body -->
    <div class="te-body">

      <!-- Left panel: controls -->
      <div class="te-controls">

        <!-- Tab switcher -->
        <div class="te-tabs">
          <button class="te-tab" :class="{ active: activeTab === 'theme' }" @click="activeTab = 'theme'">
            <Palette :size="13" /> Thème Couleurs
          </button>
          <button class="te-tab" :class="{ active: activeTab === 'layout' }" @click="activeTab = 'layout'">
            <Layout :size="13" /> Disposition
          </button>
        </div>

        <ThemeEditorLayoutTab v-if="activeTab === 'layout'" />
        <ThemeEditorThemeTab v-if="activeTab === 'theme'" />

      </div>

      <!-- Right panel: live preview -->
      <div class="te-preview-panel">
        <div class="te-preview-header">
          <Monitor :size="14" />
          <span>Prévisualisation en temps réel</span>
          <span class="te-preview-badge">LIVE</span>
        </div>
        <div class="te-preview-content">
          <ThemePreview />
        </div>
      </div>

    </div>
  </div>
</template>

<style src="@/assets/themeEditor.css"></style>
