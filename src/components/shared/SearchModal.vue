<script setup lang="ts">
import { ref, computed, watch, nextTick, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { Search, X, ArrowRight, FileText, Layout, Zap } from "lucide-vue-next";
import Fuse from "fuse.js";
import SEARCH_INDEX, { type SearchEntry } from "@/data/searchIndex";

const props = defineProps<{ modelValue: boolean }>();
const emit = defineEmits<{ "update:modelValue": [v: boolean] }>();
const router = useRouter();

const query = ref("");
const debouncedQuery = ref("");
const inputEl = ref<HTMLInputElement | null>(null);
const selectedIndex = ref(0);

let debounceTimer: ReturnType<typeof setTimeout> | null = null;
watch(query, (val) => {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => { debouncedQuery.value = val; }, 300);
});
onUnmounted(() => { if (debounceTimer) clearTimeout(debounceTimer); });

const fuse = new Fuse(SEARCH_INDEX, {
  keys: [
    { name: "label", weight: 0.5 },
    { name: "description", weight: 0.3 },
    { name: "keywords", weight: 0.15 },
    { name: "section", weight: 0.05 },
  ],
  threshold: 0.4,
  includeScore: true,
  minMatchCharLength: 2,
  ignoreLocation: true,
});

const results = computed<SearchEntry[]>(() => {
  const q = debouncedQuery.value.trim();
  if (!q) return SEARCH_INDEX;
  return fuse.search(q).map((r) => r.item);
});

// Grouper les résultats par section pour un affichage lisible
const groupedResults = computed(() => {
  const groups: Record<string, SearchEntry[]> = {};
  for (const entry of results.value) {
    if (!groups[entry.section]) groups[entry.section] = [];
    groups[entry.section].push(entry);
  }
  return groups;
});

// Index linéaire pour la navigation clavier
const flatResults = computed(() => results.value);

watch(() => props.modelValue, (open) => {
  // Un debounceTimer en attente d'une saisie AVANT une fermeture rapide (<300ms)
  // survivait à la réouverture : query/debouncedQuery étaient bien remis à "",
  // mais le timer se déclenchait ensuite et réécrivait debouncedQuery avec
  // l'ancienne saisie — affichant des résultats pour une recherche invisible
  // dans un champ pourtant vide à l'écran. Toujours annuler le timer ici.
  if (debounceTimer) { clearTimeout(debounceTimer); debounceTimer = null; }
  if (open) {
    query.value = "";
    debouncedQuery.value = "";
    selectedIndex.value = 0;
    nextTick(() => inputEl.value?.focus());
  }
});

watch(query, () => { selectedIndex.value = 0; });

function close() {
  emit("update:modelValue", false);
}

function navigate(entry: SearchEntry) {
  if (entry.query) {
    router.push({ path: entry.route, query: entry.query });
  } else {
    router.push(entry.route);
  }
  close();
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    close();
  } else if (e.key === "ArrowDown") {
    e.preventDefault();
    selectedIndex.value = Math.min(selectedIndex.value + 1, flatResults.value.length - 1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0);
  } else if (e.key === "Enter") {
    e.preventDefault();
    const entry = flatResults.value[selectedIndex.value];
    if (entry) navigate(entry);
  }
}

function getTypeIcon(type: SearchEntry["type"]) {
  if (type === "tab") return Layout;
  if (type === "feature") return Zap;
  return FileText;
}

function getTypeLabel(type: SearchEntry["type"]) {
  if (type === "tab") return "Onglet";
  if (type === "feature") return "Fonction";
  return "Page";
}

function isSelected(entry: SearchEntry) {
  return flatResults.value[selectedIndex.value]?.id === entry.id;
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="modelValue" class="search-overlay" @click.self="close">
        <div class="search-modal" @keydown="handleKeyDown">

          <!-- Input -->
          <div class="search-input-row">
            <Search :size="18" class="search-icon" />
            <input
              ref="inputEl"
              v-model="query"
              type="text"
              class="search-input"
              placeholder="Chercher une page, un onglet, une fonctionnalité..."
              autocomplete="off"
            />
            <button class="close-btn" @click="close"><X :size="16" /></button>
          </div>

          <!-- Résultats -->
          <div class="search-results">
            <div v-if="flatResults.length === 0" class="no-results">
              <Search :size="28" class="no-results-icon" />
              <span>Aucun résultat pour "{{ query }}"</span>
            </div>

            <template v-else>
              <div
                v-for="(entries, section) in groupedResults"
                :key="section"
                class="result-group"
              >
                <div class="group-header">{{ section }}</div>
                <button
                  v-for="entry in entries"
                  :key="entry.id"
                  class="result-item"
                  :class="{ selected: isSelected(entry) }"
                  @click="navigate(entry)"
                  @mouseenter="selectedIndex = flatResults.indexOf(entry)"
                >
                  <div class="result-left">
                    <span class="result-label">{{ entry.label }}</span>
                    <span class="result-desc">{{ entry.description }}</span>
                  </div>
                  <div class="result-right">
                    <span class="type-badge" :class="`type-${entry.type}`">
                      <component :is="getTypeIcon(entry.type)" :size="10" />
                      {{ getTypeLabel(entry.type) }}
                    </span>
                    <ArrowRight :size="14" class="result-arrow" />
                  </div>
                </button>
              </div>
            </template>
          </div>

          <!-- Footer -->
          <div class="search-footer">
            <span>{{ flatResults.length }} résultat{{ flatResults.length !== 1 ? 's' : '' }}</span>
            <span class="footer-sep">·</span>
            <span><kbd>↑↓</kbd> naviguer</span>
            <span><kbd>↵</kbd> ouvrir</span>
            <span><kbd>esc</kbd> fermer</span>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.search-overlay {
  position: fixed;
  inset: 0;
  z-index: 9000;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(8px) saturate(0.95);
  -webkit-backdrop-filter: blur(8px) saturate(0.95);
  display: flex;
  justify-content: center;
  padding-top: 10vh;
}

.search-modal {
  width: 620px;
  max-height: 560px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-hover);
  border-radius: var(--radius-xl);
  box-shadow: 0 32px 80px rgba(0, 0, 0, 0.7), 0 0 0 1px rgba(255,255,255,0.04);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: slide-in-up 200ms cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
}

/* Input row */
.search-input-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 18px;
  border-bottom: 1px solid var(--border);
  background: linear-gradient(135deg, var(--bg-tertiary) 0%, var(--bg-secondary) 100%);
  flex-shrink: 0;
}

.search-icon { color: var(--accent-primary); flex-shrink: 0; }

.search-input {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  color: var(--text-primary);
  font-family: inherit;
  font-size: 15px;
}
.search-input::placeholder { color: var(--text-muted); }

.close-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  transition: all 0.15s;
  flex-shrink: 0;
}
.close-btn:hover { background: var(--bg-tertiary); color: var(--text-primary); }

/* Results */
.search-results {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
  scrollbar-width: thin;
}

.no-results {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 32px 24px;
  color: var(--text-muted);
  font-size: 13px;
}
.no-results-icon { opacity: 0.3; }

/* Group */
.result-group {
  margin-bottom: 4px;
}

.group-header {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-muted);
  padding: 8px 12px 3px;
}

/* Item */
.result-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  width: 100%;
  padding: 8px 12px;
  border: none;
  background: transparent;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-family: inherit;
  text-align: left;
  transition: background 0.1s;
}

.result-item:hover,
.result-item.selected {
  background: var(--surface-glass-hover, rgba(255,255,255,0.05));
}

.result-item.selected {
  box-shadow: inset 2px 0 0 var(--accent-primary);
}

.result-left {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.result-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.result-desc {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.result-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.type-badge {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 10px;
  font-weight: 600;
  padding: 2px 7px;
  border-radius: 10px;
}

.type-page {
  background: rgba(249, 115, 22, 0.12);
  color: var(--accent-primary);
}

.type-tab {
  background: rgba(59, 130, 246, 0.12);
  color: #60a5fa;
}

.type-feature {
  background: rgba(16, 185, 129, 0.12);
  color: #34d399;
}

.result-arrow {
  color: var(--text-muted);
  opacity: 0;
  transition: opacity 0.15s;
}
.result-item.selected .result-arrow,
.result-item:hover .result-arrow {
  opacity: 1;
  color: var(--accent-primary);
}

/* Footer */
.search-footer {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  border-top: 1px solid var(--border);
  font-size: 11px;
  color: var(--text-muted);
  flex-shrink: 0;
}

.footer-sep { opacity: 0.4; }

.search-footer kbd {
  padding: 1px 5px;
  border: 1px solid var(--border);
  border-radius: 3px;
  background: var(--bg-tertiary);
  font-family: inherit;
  font-size: 10px;
  margin-right: 3px;
}

/* Transitions */
.modal-enter-active { transition: opacity 200ms ease; }
.modal-leave-active { transition: opacity 120ms ease; }
.modal-enter-from,
.modal-leave-to { opacity: 0; }
</style>
