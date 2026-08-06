<script setup lang="ts">
import { CheckCircle, AlertTriangle, Thermometer, HardDrive } from "lucide-vue-next";
import type { StoragePhysical, VolumeInfo, SmartDiskInfo } from "@/types/diagnostic";
import NBadge from "@/components/ui/NBadge.vue";
import NProgress from "@/components/ui/NProgress.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import NCollapse from "@/components/ui/NCollapse.vue";

const props = defineProps<{
  tab: string;
  storageList: StoragePhysical[];
  volumes: VolumeInfo[];
  smartData: SmartDiskInfo[];
  smartLoading?: boolean;
}>();

function smartForDisk(model: string) {
  return props.smartData.find(s =>
    s.name && model && (
      s.name.toLowerCase().includes(model.toLowerCase().slice(0, 12)) ||
      model.toLowerCase().includes(s.name.toLowerCase().slice(0, 12))
    )
  ) ?? null;
}

function healthVariant(h: string) {
  const l = h.toLowerCase();
  if (l === "healthy" || l === "sain" || l.includes("ok")) return "success";
  if (l.includes("warning") || l.includes("avertissement")) return "warning";
  if (l.includes("unhealthy") || l.includes("critical") || l.includes("critique")) return "danger";
  return "neutral";
}
</script>

<template>
  <!-- Disques physiques -->
  <template v-if="tab === 'disks'">
    <div class="diag-tab-content">
      <DiagBanner :icon="HardDrive" title="Disques Physiques" desc="Modèle, interface, firmware et données SMART" color="emerald" />
      <div v-if="!storageList.length" class="diag-empty">Aucun disque physique détecté</div>
      <template v-else>
        <NCollapse
          :title="`Disques physiques — ${storageList.length} détecté(s)`"
          storageKey="diag-storage-disks"
          defaultOpen
        >
          <div v-for="(d, i) in storageList" :key="d.serial_number || d.model + i" class="card-block">
            <!-- En-tête disque -->
            <div class="block-title">
              <HardDrive :size="16" style="color:var(--accent)" />
              <span>{{ d.model }}</span>
              <NBadge :variant="d.media_type === 'SSD' ? 'success' : d.media_type === 'NVMe' ? 'info' : 'default'">{{ d.media_type }}</NBadge>
              <NBadge variant="info">{{ d.interface_type }}</NBadge>
              <NBadge :variant="d.status === 'OK' ? 'success' : 'danger'">{{ d.status }}</NBadge>
            </div>

            <!-- Infos de base -->
            <div class="info-grid">
              <div class="info-row"><span>Taille</span><span style="font-weight:600">{{ d.size_gb.toFixed(0) }} GB</span></div>
              <div class="info-row"><span>Partitions</span><span>{{ d.partitions }}</span></div>
              <div class="info-row"><span>Firmware</span><code>{{ d.firmware_revision || "N/A" }}</code></div>
              <div class="info-row"><span>N° de série</span><code>{{ d.serial_number || "N/A" }}</code></div>
              <div class="info-row info-full"><span>PNP Device ID</span><code style="font-size:9px;word-break:break-all">{{ d.pnp_device_id || "N/A" }}</code></div>
            </div>

            <!-- Section SMART -->
            <template v-if="smartLoading && smartData.length === 0">
              <div class="muted" style="font-size:11px;margin-top:8px;font-style:italic">
                SMART en cours de chargement...
              </div>
            </template>
            <template v-else>
              <NCollapse
                title="Santé SMART"
                :storageKey="`diag-smart-${i}`"
                defaultOpen
                variant="subtle"
              >
                <template v-if="!smartForDisk(d.model)">
                  <span class="muted" style="font-size:12px;font-style:italic">Données SMART non associées à ce disque</span>
                </template>
                <template v-else v-for="smart in [smartForDisk(d.model)].filter(s => s !== null)" :key="`smart-${d.serial_number || d.model}`">
                  <div class="info-grid">
                    <div class="info-row">
                      <span>État SMART</span>
                      <div style="display:flex;align-items:center;gap:6px">
                        <component :is="smart.health_status === 'Healthy' || smart.health_status === 'Sain' ? CheckCircle : AlertTriangle"
                          :size="14" :class="smart.health_status === 'Healthy' || smart.health_status === 'Sain' ? 'ic-ok' : 'ic-warn'" />
                        <NBadge :variant="healthVariant(smart.health_status)">{{ smart.health_status }}</NBadge>
                      </div>
                    </div>
                    <div class="info-row">
                      <span>Statut opérationnel</span>
                      <NBadge :variant="smart.operational_status === 'OK' ? 'success' : 'warning'">{{ smart.operational_status }}</NBadge>
                    </div>
                    <div class="info-row"><span>Type de média</span><NBadge :variant="smart.media_type === 'SSD' ? 'success' : smart.media_type === 'NVMe' ? 'info' : 'default'">{{ smart.media_type }}</NBadge></div>
                    <div class="info-row">
                      <span style="display:flex;align-items:center;gap:4px"><Thermometer :size="13" /> Température</span>
                      <NBadge v-if="smart.temperature >= 0"
                        :variant="smart.temperature > 60 ? 'danger' : smart.temperature > 45 ? 'warning' : 'success'">
                        {{ smart.temperature }}°C
                      </NBadge>
                      <span v-else class="muted">N/A</span>
                    </div>
                    <div class="info-row" v-if="smart.wear_level >= 0">
                      <span>Usure SSD</span>
                      <div style="display:flex;align-items:center;gap:8px">
                        <NBadge :variant="smart.wear_level < 20 ? 'success' : smart.wear_level < 50 ? 'warning' : 'danger'">
                          {{ smart.wear_level }}%
                        </NBadge>
                        <span class="muted" style="font-size:11px">{{ 100 - smart.wear_level }}% restant</span>
                      </div>
                    </div>
                    <div class="info-row" v-if="smart.power_on_hours > 0">
                      <span>Heures allumé</span>
                      <span>{{ smart.power_on_hours.toLocaleString() }} h
                        <span class="muted" style="font-size:11px">(≈ {{ (smart.power_on_hours / 8760).toFixed(1) }} ans)</span>
                      </span>
                    </div>
                    <div class="info-row" v-if="smart.start_stop_cycles > 0">
                      <span>Cycles allumage</span><span>{{ smart.start_stop_cycles.toLocaleString() }}</span>
                    </div>
                    <div class="info-row">
                      <span>Erreurs lecture</span>
                      <NBadge :variant="smart.read_errors_uncorrected > 0 ? 'danger' : 'success'">
                        {{ smart.read_errors_uncorrected > 0 ? smart.read_errors_uncorrected : 'Aucune' }}
                      </NBadge>
                    </div>
                    <div class="info-row">
                      <span>Erreurs écriture</span>
                      <NBadge :variant="smart.write_errors_uncorrected > 0 ? 'danger' : 'success'">
                        {{ smart.write_errors_uncorrected > 0 ? smart.write_errors_uncorrected : 'Aucune' }}
                      </NBadge>
                    </div>
                    <div class="info-row" v-if="smart.reallocated_sectors >= 0">
                      <span>Secteurs réaffectés</span>
                      <NBadge :variant="smart.reallocated_sectors > 0 ? 'danger' : 'success'">
                        {{ smart.reallocated_sectors > 0 ? smart.reallocated_sectors : 'Aucun' }}
                      </NBadge>
                    </div>
                  </div>
                </template>
              </NCollapse>
            </template>
          </div>
        </NCollapse>
      </template>
    </div>
  </template>

  <!-- Volumes logiques — fusionnés dans l'onglet Disques -->
  <template v-if="tab === 'disks' && volumes.length > 0">
    <div class="diag-tab-content" style="padding-top:0">
      <NCollapse
        :title="`Volumes & Partitions — ${volumes.length} volume(s)`"
        storageKey="diag-storage-volumes"
        defaultOpen
      >
        <div v-for="(v, i) in volumes" :key="i" class="card-block">
          <div class="block-title">
            <code style="font-size:16px">{{ v.drive_letter }}</code>
            <span>{{ v.label || "Sans nom" }}</span>
            <NBadge variant="info">{{ v.filesystem }}</NBadge>
            <NBadge :variant="v.drive_type === 'Fixed' || v.drive_type === 'Disque fixe' ? 'default' : 'neutral'">{{ v.drive_type }}</NBadge>
            <NBadge :variant="healthVariant(v.health_status)">{{ v.health_status }}</NBadge>
          </div>
          <div class="diag-stat-row" style="margin-bottom:12px">
            <span style="min-width:60px">Espace</span>
            <NProgress
              :value="v.used_percent"
              :variant="v.used_percent > 90 ? 'danger' : v.used_percent > 80 ? 'warning' : 'default'"
              size="sm" showLabel
            />
            <span style="min-width:160px;text-align:right;font-size:12px;color:var(--text-secondary)">
              {{ v.free_gb.toFixed(1) }} GB libres / {{ v.total_gb.toFixed(1) }} GB
            </span>
          </div>
          <div class="info-grid">
            <div class="info-row"><span>Total</span><span>{{ v.total_gb.toFixed(2) }} GB</span></div>
            <div class="info-row"><span>Utilisé</span><span>{{ v.used_gb.toFixed(2) }} GB ({{ v.used_percent.toFixed(1) }}%)</span></div>
            <div class="info-row"><span>Libre</span><span>{{ v.free_gb.toFixed(2) }} GB</span></div>
            <div class="info-row"><span>Système de fichiers</span><NBadge variant="info">{{ v.filesystem }}</NBadge></div>
            <div class="info-row"><span>Type de volume</span><span>{{ v.drive_type }}</span></div>
            <div class="info-row"><span>Santé</span><NBadge :variant="healthVariant(v.health_status)">{{ v.health_status }}</NBadge></div>
            <div class="info-row"><span>Statut opérationnel</span><span>{{ v.operational_status }}</span></div>
          </div>
        </div>
      </NCollapse>
    </div>
  </template>
</template>
