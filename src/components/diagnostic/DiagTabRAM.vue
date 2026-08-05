<script setup lang="ts">
import { computed } from "vue";
import { invoke } from "@/utils/invoke";
import { HardDrive } from "lucide-vue-next";
import NBadge from "@/components/ui/NBadge.vue";
import NButton from "@/components/ui/NButton.vue";
import NProgress from "@/components/ui/NProgress.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import NCollapse from "@/components/ui/NCollapse.vue";
import type { RamDetailed, SysInfo } from "@/types/diagnostic";

const props = defineProps<{
  ramData: RamDetailed | null;
  sysInfo: SysInfo | null;
}>();

const usagePercent = computed(() => {
  if (!props.sysInfo) return 0;
  return props.sysInfo.ram.usage_percent;
});
const channels = computed(() => {
  if (!props.ramData) return "";
  const populated = props.ramData.used_slots;
  if (populated === 1) return "Single Channel";
  if (populated === 2) return "Dual Channel (probable)";
  if (populated === 4) return "Quad Channel (probable)";
  return `${populated} modules`;
});

async function openCpuZ() {
  try {
    await invoke('run_system_command', {
      cmd: 'cmd',
      args: ['/c', 'start "" cpuz_x64.exe 2>nul || start "" cpuz.exe 2>nul'],
    });
  } catch { /* lancement CPU-Z non critique */ }
}
</script>

<template>
  <div class="diag-tab-content">
    <DiagBanner
      :icon="HardDrive"
      title="Mémoire RAM"
      desc="Modules installés, slots, vitesse et configuration"
      color="blue"
    />
    <template v-if="ramData">
      <NCollapse title="Résumé RAM" storageKey="diag-ram-summary" :defaultOpen="true">
        <div class="info-grid" style="margin-bottom:12px">
          <div class="info-row">
            <span>Slots utilisés</span>
            <div style="display:flex;align-items:center;gap:6px">
              <NBadge variant="info">{{ ramData.used_slots }} / {{ ramData.total_slots }}</NBadge>
              <span v-if="ramData.total_slots > ramData.used_slots * 2" class="muted" style="font-size:10px">(max BIOS)</span>
            </div>
          </div>
          <div class="info-row"><span>Slots physiques occupés</span><NBadge variant="success">{{ ramData.used_slots }} barrette(s)</NBadge></div>
          <div class="info-row"><span>Capacité totale</span><NBadge variant="success">{{ ramData.total_capacity_gb.toFixed(0) }} GB</NBadge></div>
          <div class="info-row"><span>Configuration</span><span>{{ channels }}</span></div>
          <div v-if="sysInfo" class="info-row"><span>Utilisée</span><span>{{ sysInfo.ram.used_gb.toFixed(1) }} GB ({{ Math.round(sysInfo.ram.usage_percent) }}%)</span></div>
        </div>

        <div v-if="sysInfo" class="diag-stat-row" style="margin-bottom:16px">
          <span>Usage RAM</span>
          <NProgress :value="sysInfo.ram.usage_percent" :variant="sysInfo.ram.usage_percent > 85 ? 'danger' : sysInfo.ram.usage_percent > 70 ? 'warning' : 'default'" size="sm" showLabel />
        </div>
        <div class="info-row" style="border-top:1px solid var(--border);margin-top:8px;padding-top:8px">
          <span style="color:var(--text-muted)">Analyse avancée</span>
          <NButton variant="ghost" size="sm" @click="openCpuZ">
            Ouvrir CPU-Z (timings XMP)
          </NButton>
        </div>
      </NCollapse>

      <NCollapse title="Modules installés" storageKey="diag-ram-modules" :defaultOpen="true">
        <div v-for="(s, i) in ramData.slots" :key="i" class="card-block">
          <div class="block-title">
            <span>{{ s.device_locator }}</span>
            <span class="muted" style="font-size:12px;font-weight:400">— {{ s.bank_label }}</span>
            <NBadge variant="info">{{ s.capacity_gb.toFixed(0) }} GB</NBadge>
            <NBadge variant="success">{{ s.memory_type }}-{{ s.speed_mhz }}</NBadge>
          </div>
          <div class="info-grid">
            <div class="info-row"><span>Capacité</span><span>{{ s.capacity_gb.toFixed(0) }} GB</span></div>
            <div class="info-row"><span>Type mémoire</span><NBadge variant="info">{{ s.memory_type }}</NBadge></div>
            <div class="info-row"><span>Vitesse nominale (XMP/JEDEC)</span><span>{{ s.speed_mhz }} MHz</span></div>
            <div class="info-row"><span>Vitesse configurée</span><span>{{ s.configured_speed_mhz }} MHz</span></div>
            <div class="info-row"><span>Format</span><NBadge variant="default">{{ s.form_factor }}</NBadge></div>
            <div class="info-row"><span>Largeur de données</span><span>{{ s.data_width > 0 ? s.data_width + ' bits' : 'N/A' }}</span></div>
            <div class="info-row"><span>Fabricant</span><span>{{ s.manufacturer || "N/A" }}</span></div>
            <div class="info-row"><span>Numéro de pièce (P/N)</span><code>{{ s.part_number || "N/A" }}</code></div>
            <div class="info-row info-full"><span>Numéro de série</span><code>{{ s.serial_number || "N/A" }}</code></div>
            <!-- Tensions si disponibles -->
            <div v-if="s.min_voltage && s.min_voltage > 0" class="info-row">
              <span>Tension minimale</span>
              <span>{{ (s.min_voltage / 1000).toFixed(2) }} V</span>
            </div>
            <div v-if="s.configured_voltage && s.configured_voltage > 0" class="info-row">
              <span>Tension configurée</span>
              <NBadge variant="info">{{ (s.configured_voltage / 1000).toFixed(2) }} V</NBadge>
            </div>
          </div>
        </div>
      </NCollapse>
    </template>
    <div v-else class="diag-empty">Informations RAM non disponibles</div>
  </div>
</template>
