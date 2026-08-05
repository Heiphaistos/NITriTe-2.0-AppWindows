<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import { useNotificationStore } from "@/stores/notifications";
import { Globe, RefreshCw, Trash2, CheckCircle, CheckSquare, Square } from "lucide-vue-next";

const notify = useNotificationStore();

interface BrowserCache { id: string; name: string; detected: boolean; cache_size_mb: number; selected: boolean; }
interface BrowserCleanResult { freed_mb: number; files_deleted: number; }
const browsers = ref<BrowserCache[]>([]);
const loading = ref(false);
const cleaning = ref(false);
const cleanResult = ref<{ freed: number; deleted: number } | null>(null);

const total = computed(() => browsers.value.reduce((s, b) => s + b.cache_size_mb, 0));

async function load() {
  loading.value = true; cleanResult.value = null;
  try {
    const data = await invoke<BrowserCache[]>("get_browser_cache_sizes");
    browsers.value = data.filter(b => b.detected).map(b => ({ ...b, selected: true }));
  } catch (e: unknown) {
    // Ne jamais afficher de données fictives comme si elles étaient réelles :
    // l'ancien fallback montrait 3 navigateurs inventés (245.3/128.7/89.2 MB)
    // en cas d'échec de détection, masquant une vraie erreur derrière un
    // faux succès plausible.
    browsers.value = [];
    notify.error("Détection des caches impossible", String(e));
  } finally { loading.value = false; }
}

async function clean() {
  const selected = browsers.value.filter(b => b.selected).map(b => b.id);
  if (!selected.length) { notify.warning("Selectionnez au moins un navigateur"); return; }
  cleaning.value = true;
  try {
    const results = await invoke<BrowserCleanResult[]>("clean_browser_cache", { browserIds: selected });
    const freed = results.reduce((s, r) => s + r.freed_mb, 0);
    const deleted = results.reduce((s, r) => s + r.files_deleted, 0);
    cleanResult.value = { freed, deleted };
    notify.success("Nettoyage terminé", `${freed.toFixed(1)} MB libérés, ${deleted} fichiers`);
    await load();
  } catch (e: unknown) { notify.error("Erreur nettoyage", String(e)); }
  finally { cleaning.value = false; }
}

onMounted(load);
</script>

<template>
  <NCard>
    <template #header>
      <div style="display:flex;align-items:center;gap:8px">
        <Globe :size="16" style="color:var(--accent-primary)" />
        <span>Caches Navigateurs</span>
        <span v-if="!loading && browsers.length" style="font-size:11px;color:var(--text-muted);margin-left:4px">
          {{ total.toFixed(1) }} MB
        </span>
        <NButton variant="secondary" size="sm" :loading="loading" @click="load" style="margin-left:auto">
          <RefreshCw :size="14" />
        </NButton>
      </div>
    </template>

    <div v-if="loading" style="display:flex;align-items:center;gap:8px;padding:12px 0">
      <NSpinner :size="20" /><span style="font-size:13px;color:var(--text-muted)">Détection...</span>
    </div>
    <div v-else-if="!browsers.length" style="font-size:13px;color:var(--text-muted);padding:8px 0">
      Aucun navigateur détecté avec du cache.
    </div>
    <div v-else>
      <div style="display:flex;gap:6px;margin-bottom:8px">
        <button @click="browsers.forEach(b => b.selected = true)"
          style="font-size:11px;padding:3px 8px;border:1px solid var(--border);border-radius:4px;background:var(--bg-secondary);color:var(--text-secondary);cursor:pointer">
          Tout sélectionner
        </button>
        <button @click="browsers.forEach(b => b.selected = false)"
          style="font-size:11px;padding:3px 8px;border:1px solid var(--border);border-radius:4px;background:var(--bg-secondary);color:var(--text-secondary);cursor:pointer">
          Tout désélectionner
        </button>
      </div>
      <div style="display:flex;flex-direction:column;gap:6px">
        <button v-for="b in browsers" :key="b.id"
          @click="b.selected = !b.selected"
          style="display:flex;align-items:center;gap:10px;padding:8px 12px;border-radius:var(--radius-md);cursor:pointer;font-size:13px;font-family:inherit;text-align:left;transition:all .15s"
          :style="{ border: b.selected ? '1px solid var(--accent-primary)' : '1px solid var(--border)', background: b.selected ? 'var(--bg-elevated)' : 'var(--bg-tertiary)', color: 'var(--text-primary)' }">
          <component :is="b.selected ? CheckSquare : Square" :size="16" :style="{ color: b.selected ? 'var(--accent-primary)' : 'var(--text-muted)' }" />
          <span style="flex:1">{{ b.name }}</span>
          <span style="font-size:11px;color:var(--text-muted)">{{ b.cache_size_mb.toFixed(1) }} MB</span>
        </button>
      </div>
      <div style="display:flex;align-items:center;gap:10px;margin-top:10px">
        <div v-if="cleanResult" style="display:flex;align-items:center;gap:6px;font-size:12px;color:var(--success)">
          <CheckCircle :size="13" />
          <span>{{ cleanResult.freed.toFixed(1) }} MB libérés ({{ cleanResult.deleted }} fichiers)</span>
        </div>
        <NButton variant="primary" size="sm" :loading="cleaning" :disabled="cleaning || !browsers.filter(b=>b.selected).length" @click="clean" style="margin-left:auto">
          <Trash2 :size="14" /> Nettoyer les caches
        </NButton>
      </div>
    </div>
  </NCard>
</template>
