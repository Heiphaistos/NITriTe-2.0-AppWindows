<script setup lang="ts">
import { ref, watch } from "vue";
import NModal from "./NModal.vue";
import NButton from "./NButton.vue";
import { Cpu, MemoryStick, HardDrive } from "lucide-vue-next";
import { normalizeThresholds } from "@/utils/alertThresholds";

export interface AlertThresholds {
  cpu_warn: number;
  cpu_crit: number;
  ram_warn: number;
  ram_crit: number;
  disk_warn: number;
  disk_crit: number;
}

const props = defineProps<{ open: boolean; modelValue: AlertThresholds }>();
const emit = defineEmits<{
  close: [];
  "update:modelValue": [v: AlertThresholds];
}>();

const draft = ref<AlertThresholds>({ ...props.modelValue });
watch(() => props.open, (v) => { if (v) draft.value = { ...props.modelValue }; });

function save() {
  const normalized = normalizeThresholds(draft.value, props.modelValue);
  emit("update:modelValue", normalized);
  try { localStorage.setItem("nitrite_alert_thresholds", JSON.stringify(normalized)); } catch { /* ignore */ }
  emit("close");
}
</script>

<template>
  <NModal :open="open" title="Seuils d'alertes personnalisables" width="440px" @close="emit('close')">
    <div class="threshold-body">
      <div class="th-row">
        <span class="th-label"><Cpu :size="13" /> CPU</span>
        <label>Avertissement <input type="number" v-model.number="draft.cpu_warn" min="50" max="100" class="th-input" />%</label>
        <label>Critique <input type="number" v-model.number="draft.cpu_crit" min="50" max="100" class="th-input" />%</label>
      </div>
      <div class="th-row">
        <span class="th-label"><MemoryStick :size="13" /> RAM</span>
        <label>Avertissement <input type="number" v-model.number="draft.ram_warn" min="50" max="100" class="th-input" />%</label>
        <label>Critique <input type="number" v-model.number="draft.ram_crit" min="50" max="100" class="th-input" />%</label>
      </div>
      <div class="th-row">
        <span class="th-label"><HardDrive :size="13" /> Disque</span>
        <label>Avertissement <input type="number" v-model.number="draft.disk_warn" min="50" max="100" class="th-input" />%</label>
        <label>Critique <input type="number" v-model.number="draft.disk_crit" min="50" max="100" class="th-input" />%</label>
      </div>
    </div>
    <template #footer>
      <NButton variant="ghost" size="sm" @click="emit('close')">Annuler</NButton>
      <NButton variant="primary" size="sm" @click="save">Enregistrer</NButton>
    </template>
  </NModal>
</template>

<style scoped>
.threshold-body { display: flex; flex-direction: column; gap: 12px; }
.th-row {
  display: flex; align-items: center; gap: 12px; padding: 10px 12px;
  background: var(--bg-tertiary); border-radius: var(--radius-md); flex-wrap: wrap;
}
.th-label { display: flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 600; color: var(--text-secondary); min-width: 72px; }
.th-row label { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text-secondary); }
.th-input {
  width: 52px; padding: 3px 6px; font-size: 12px; font-family: "JetBrains Mono", monospace;
  background: var(--bg-elevated); border: 1px solid var(--border); border-radius: var(--radius-sm);
  color: var(--text-primary); text-align: center;
}
.th-input:focus { outline: none; border-color: var(--accent-primary); }
</style>
