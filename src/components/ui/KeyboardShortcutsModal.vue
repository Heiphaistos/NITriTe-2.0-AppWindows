<script setup lang="ts">
import NModal from "@/components/ui/NModal.vue";

defineProps<{ modelValue: boolean }>();
defineEmits<{ "update:modelValue": [v: boolean] }>();

// Liste vérifiée cycle 139 contre le code réel : 4 raccourcis annoncés ici
// n'étaient câblés nulle part dans l'app (F5/Ctrl+R "Diagnostic", Tab
// "Terminal", Ctrl+Entrée "Scripts") — retirés plutôt que de continuer à
// promettre une fonctionnalité inexistante. Ctrl+L "Terminal" a été câblé
// (TerminalPage.vue::handleKeyDown) au lieu d'être retiré, seul cas où la
// fonction cible (clearOutput()) existait déjà et ne demandait qu'un
// raccourci clavier.
const sections = [
  {
    title: "Navigation",
    shortcuts: [
      { keys: ["Ctrl", "K"], desc: "Ouvrir la recherche" },
      { keys: ["Ctrl", "B"], desc: "Réduire la sidebar" },
      { keys: ["?"], desc: "Raccourcis clavier" },
      { keys: ["Esc"], desc: "Fermer la modale" },
    ],
  },
  {
    title: "Terminal",
    shortcuts: [
      { keys: ["Ctrl", "L"], desc: "Vider la console" },
      { keys: ["↑ / ↓"], desc: "Historique des commandes" },
    ],
  },
];
</script>

<template>
  <NModal :open="modelValue" title="Raccourcis Clavier" @update:open="$emit('update:modelValue', $event)">
    <div class="shortcuts-grid">
      <div v-for="section in sections" :key="section.title" class="shortcut-section">
        <p class="section-title">{{ section.title }}</p>
        <div v-for="s in section.shortcuts" :key="s.desc" class="shortcut-row">
          <div class="keys">
            <kbd v-for="k in s.keys" :key="k" class="key">{{ k }}</kbd>
          </div>
          <span class="shortcut-desc">{{ s.desc }}</span>
        </div>
      </div>
    </div>
    <p class="shortcuts-hint">Appuyez sur <kbd class="key">?</kbd> n'importe où pour ouvrir ce panneau.</p>
  </NModal>
</template>

<style scoped>
.shortcuts-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}
@media (max-width: 600px) { .shortcuts-grid { grid-template-columns: 1fr; } }
.shortcut-section { display: flex; flex-direction: column; gap: 8px; }
.section-title {
  font-size: 11px; font-weight: 700; text-transform: uppercase;
  letter-spacing: .08em; color: var(--accent-primary); margin-bottom: 4px;
}
.shortcut-row {
  display: flex; align-items: center; gap: 12px;
  padding: 6px 8px; border-radius: var(--radius-md);
  transition: background var(--transition-fast);
}
.shortcut-row:hover { background: var(--bg-tertiary); }
.keys { display: flex; gap: 4px; flex-shrink: 0; }
.key {
  padding: 2px 7px; border-radius: var(--radius-sm);
  background: var(--bg-elevated); border: 1px solid var(--border);
  font-family: "JetBrains Mono", monospace; font-size: 11px;
  color: var(--text-primary); font-weight: 600;
  box-shadow: 0 2px 0 var(--border);
}
.shortcut-desc { font-size: 12px; color: var(--text-secondary); }
.shortcuts-hint {
  margin-top: 16px; padding-top: 12px; border-top: 1px solid var(--border);
  font-size: 12px; color: var(--text-muted); text-align: center;
}
</style>
