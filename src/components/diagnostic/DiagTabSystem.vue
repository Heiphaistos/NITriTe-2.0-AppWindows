<script setup lang="ts">
import { Monitor, Cpu, CircuitBoard, Shield, Lock } from "lucide-vue-next";
import NBadge from "@/components/ui/NBadge.vue";
import NProgress from "@/components/ui/NProgress.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import NCollapse from "@/components/ui/NCollapse.vue";
import type { SysInfo, BiosInfo, MoboDetailed, OsExtended, BiosExtended, MoboExtended } from "@/types/diagnostic";

const props = defineProps<{
  tab: string;
  sysInfo: SysInfo | null;
  biosInfo: BiosInfo | null;
  moboInfo: MoboDetailed | null;
  osExtended: OsExtended | null;
  biosExtended?: BiosExtended | null;
  moboExtended?: MoboExtended | null;
}>();
</script>

<template>
  <!-- OS -->
  <template v-if="tab === 'os'">
    <div class="diag-tab-content">
      <DiagBanner
        :icon="Monitor"
        title="Système d'exploitation"
        desc="Version, architecture, ressources temps réel et informations avancées"
        color="blue"
      />
      <template v-if="sysInfo">
        <NCollapse title="Système d'exploitation" storageKey="diag-os-base" defaultOpen>
          <div class="info-grid">
            <div class="info-row"><span>OS</span><span>{{ sysInfo.os.name }} {{ sysInfo.os.version }}</span></div>
            <div class="info-row"><span>Architecture</span><NBadge variant="info">{{ sysInfo.os.architecture }}</NBadge></div>
            <div class="info-row"><span>Hostname</span><code>{{ sysInfo.os.hostname }}</code></div>
            <div class="info-row"><span>Build</span><code>{{ sysInfo.os.build_number }}</code></div>
            <div class="info-row"><span>RAM</span><span>{{ sysInfo.ram.total_gb.toFixed(1) }} GB ({{ Math.round(sysInfo.ram.usage_percent) }}% utilisé)</span></div>
            <div class="info-row"><span>Stockage C:</span>
              <span v-if="sysInfo.disks[0]?.partitions[0]">{{ (sysInfo.disks[0].partitions[0].used_gb || 0).toFixed(0) }} / {{ (sysInfo.disks[0].partitions[0].total_gb || 0).toFixed(0) }} GB</span>
              <span v-else>N/A</span>
            </div>
          </div>
        </NCollapse>

        <NCollapse title="Ressources temps réel" storageKey="diag-os-realtime" defaultOpen>
          <div class="diag-stat-row"><span>CPU</span><NProgress :value="sysInfo.cpu.usage_percent" size="sm" showLabel /></div>
          <div class="diag-stat-row"><span>RAM</span><NProgress :value="sysInfo.ram.usage_percent" size="sm" showLabel /></div>
          <div v-if="sysInfo.disks[0]?.partitions[0]" class="diag-stat-row">
            <span>Disque C:</span>
            <NProgress :value="sysInfo.disks[0].partitions[0].usage_percent" :variant="sysInfo.disks[0].partitions[0].usage_percent > 90 ? 'danger' : 'default'" size="sm" showLabel />
          </div>
        </NCollapse>

        <template v-if="osExtended">
          <NCollapse title="Informations avancées" storageKey="diag-os-advanced" defaultOpen>
            <div class="info-grid">
              <div class="info-row"><span>Installé le</span><span>{{ osExtended.install_date }}</span></div>
              <div class="info-row"><span>Dernier démarrage</span><span>{{ osExtended.last_boot_time }}</span></div>
              <div class="info-row"><span>Utilisateur enregistré</span><span>{{ osExtended.registered_user || "N/A" }}</span></div>
              <div class="info-row"><span>Organisation</span><span>{{ osExtended.organization || "N/A" }}</span></div>
              <div class="info-row"><span>Langue OS</span><span>{{ osExtended.os_language }}</span></div>
              <div class="info-row"><span>Fuseau horaire</span><span>{{ osExtended.timezone }}</span></div>
              <div class="info-row"><span>Type de produit</span><NBadge variant="info">{{ osExtended.product_type }}</NBadge></div>
              <div class="info-row"><span>Redémarrage en attente</span>
                <NBadge :variant="osExtended.pending_reboot ? 'warning' : 'success'">{{ osExtended.pending_reboot ? "Oui" : "Non" }}</NBadge>
              </div>
              <div class="info-row"><span>Code pays</span><span>{{ osExtended.country_code }}</span></div>
              <div class="info-row"><span>Locale</span><code>{{ osExtended.locale }}</code></div>
            </div>
          </NCollapse>

          <NCollapse title="Répertoires système" storageKey="diag-os-dirs" :defaultOpen="false">
            <div class="info-grid">
              <div class="info-row"><span>Windows</span><code class="muted" style="font-size:11px">{{ osExtended.windows_directory }}</code></div>
              <div class="info-row"><span>System32</span><code class="muted" style="font-size:11px">{{ osExtended.system_directory }}</code></div>
              <div class="info-row"><span>Lecteur système</span><span>{{ osExtended.system_drive }}</span></div>
              <div class="info-row"><span>Périphérique de boot</span><code class="muted" style="font-size:11px">{{ osExtended.boot_device }}</code></div>
              <div class="info-row"><span>Fichier de page</span><code class="muted" style="font-size:11px">{{ osExtended.page_file_path }}</code></div>
            </div>
          </NCollapse>

          <NCollapse title="Mémoire virtuelle" storageKey="diag-os-vmem" :defaultOpen="false">
            <div class="info-grid">
              <div class="info-row"><span>Mém. virtuelle totale</span><span>{{ osExtended.total_virtual_memory_gb.toFixed(1) }} GB</span></div>
              <div class="info-row"><span>Mém. virtuelle libre</span><span>{{ osExtended.free_virtual_memory_gb.toFixed(1) }} GB</span></div>
              <div class="info-row"><span>Swap total</span><span>{{ osExtended.total_swap_gb > 0 ? osExtended.total_swap_gb.toFixed(1) + ' GB' : 'N/A' }}</span></div>
              <div class="info-row"><span>RAM physique libre</span><span>{{ osExtended.free_physical_gb.toFixed(1) }} GB</span></div>
            </div>
          </NCollapse>

          <NCollapse title="Environnement logiciel" storageKey="diag-os-software" :defaultOpen="false">
            <div class="info-grid">
              <div class="info-row"><span>PowerShell</span><NBadge variant="info">v{{ osExtended.powershell_version }}</NBadge></div>
              <div class="info-row info-full"><span>.NET Frameworks</span>
                <span class="mono" style="text-align:right">{{ osExtended.dotnet_versions.length ? osExtended.dotnet_versions.join(' · ') : 'N/A' }}</span>
              </div>
            </div>
          </NCollapse>
        </template>
      </template>
      <div v-else class="diag-empty">Informations système non disponibles</div>
    </div>
  </template>

  <!-- BIOS -->
  <template v-else-if="tab === 'bios'">
    <div class="diag-tab-content">
      <DiagBanner
        :icon="Cpu"
        title="BIOS / UEFI"
        desc="Firmware, sécurité, TPM et informations avancées du système"
        color="amber"
      />
      <template v-if="biosInfo">
        <NCollapse title="BIOS / UEFI" storageKey="diag-bios-base" defaultOpen>
          <div class="info-grid">
            <div class="info-row"><span>Fabricant</span><span>{{ biosInfo.manufacturer }}</span></div>
            <div class="info-row"><span>Version</span><code>{{ biosInfo.version }}</code></div>
            <div class="info-row"><span>Date de sortie</span><span>{{ biosInfo.release_date }}</span></div>
            <div class="info-row"><span>Version SMBIOS</span><NBadge variant="info">{{ biosInfo.smbios_version }}</NBadge></div>
            <div class="info-row info-full"><span>Numéro de série système</span><code>{{ biosInfo.serial_number || "N/A" }}</code></div>
          </div>
        </NCollapse>

        <template v-if="biosExtended">
          <NCollapse title="Firmware & Sécurité" storageKey="diag-bios-security" defaultOpen>
            <div class="info-grid">
              <div class="info-row">
                <span>Type firmware</span>
                <NBadge :variant="biosExtended.firmware_type === 'UEFI' ? 'success' : 'warning'">
                  {{ biosExtended.firmware_type || '...' }}
                </NBadge>
              </div>
              <div class="info-row">
                <span>Secure Boot</span>
                <NBadge :variant="biosExtended.secure_boot ? 'success' : 'neutral'">
                  <Lock :size="10" style="display:inline;margin-right:3px" />
                  {{ biosExtended.secure_boot ? 'Activé' : 'Désactivé' }}
                </NBadge>
              </div>
              <div class="info-row">
                <span>TPM présent</span>
                <NBadge :variant="biosExtended.tpm_present ? 'success' : 'neutral'">
                  <Shield :size="10" style="display:inline;margin-right:3px" />
                  {{ biosExtended.tpm_present ? 'Oui' : 'Non' }}
                </NBadge>
              </div>
              <div v-if="biosExtended.tpm_present" class="info-row">
                <span>TPM activé</span>
                <NBadge :variant="biosExtended.tpm_enabled ? 'success' : 'warning'">{{ biosExtended.tpm_enabled ? 'Oui' : 'Non' }}</NBadge>
              </div>
              <div v-if="biosExtended.tpm_spec_version" class="info-row">
                <span>Version TPM</span><NBadge variant="info">TPM {{ biosExtended.tpm_spec_version }}</NBadge>
              </div>
              <div class="info-row">
                <span>Type de châssis</span><span>{{ biosExtended.chassis_type || 'N/A' }}</span>
              </div>
              <div class="info-row">
                <span>Wake-on-LAN</span>
                <NBadge :variant="biosExtended.wake_on_lan ? 'info' : 'neutral'">{{ biosExtended.wake_on_lan ? 'Activé' : 'Désactivé' }}</NBadge>
              </div>
              <div class="info-row">
                <span>Démarrage rapide</span>
                <NBadge :variant="biosExtended.fast_boot ? 'info' : 'neutral'">{{ biosExtended.fast_boot ? 'Activé' : 'Désactivé' }}</NBadge>
              </div>
            </div>
          </NCollapse>
        </template>

        <!-- biosExtended est résolu dans le même Promise.all que biosInfo côté
             DiagnosticPage.vue (avec repli .catch(() => null)) : si biosInfo est
             déjà présent ici, biosExtended a déjà fini de se résoudre — null à ce
             stade est définitif, pas un chargement encore en cours. -->
        <div v-if="!biosExtended" class="diag-empty" style="margin-top:8px">Informations BIOS étendues non disponibles</div>

        <NCollapse title="Informations complémentaires" storageKey="diag-bios-info" :defaultOpen="false">
          <div class="info-grid">
            <div class="info-row"><span>Fabricant BIOS</span><span>{{ biosInfo.manufacturer }}</span></div>
            <div class="info-row"><span>Version firmware</span><code>{{ biosInfo.version }}</code></div>
            <div class="info-row"><span>Date de sortie</span><span>{{ biosInfo.release_date }}</span></div>
            <div class="info-row"><span>Version SMBIOS</span><NBadge variant="info">{{ biosInfo.smbios_version }}</NBadge></div>
            <div class="info-row info-full"><span>Numéro de série système</span><code>{{ biosInfo.serial_number || 'Non communiqué' }}</code></div>
            <div v-if="biosExtended" class="info-row"><span>Type de châssis</span><span>{{ biosExtended.chassis_type || 'N/A' }}</span></div>
            <div v-if="biosExtended" class="info-row"><span>Mode firmware</span>
              <NBadge :variant="biosExtended.firmware_type === 'UEFI' ? 'success' : 'warning'">{{ biosExtended.firmware_type || 'BIOS Legacy' }}</NBadge>
            </div>
          </div>
          <div class="card-block" style="margin-top:8px;border:none;background:var(--bg-tertiary)">
            <span class="muted" style="font-size:11px;line-height:1.6">
              <strong>SMBIOS</strong> (System Management BIOS) standardise la façon dont le firmware communique les informations matérielles au système d'exploitation.
              Le numéro de série système identifie la machine auprès du fabricant pour le support et les garanties.
              La version UEFI remplace le BIOS Legacy depuis 2012 avec support du Secure Boot et du disque GPT.
            </span>
          </div>
        </NCollapse>
      </template>
      <div v-else class="diag-empty">Informations BIOS non disponibles</div>
    </div>
  </template>

  <!-- Carte Mère -->
  <template v-else-if="tab === 'mobo'">
    <div class="diag-tab-content">
      <DiagBanner
        :icon="CircuitBoard"
        title="Carte Mère"
        desc="Fabricant, modèle, slots d'extension, socket CPU et informations hardware"
        color="teal"
      />
      <template v-if="moboInfo">
        <NCollapse title="Carte Mère" storageKey="diag-mobo-base" defaultOpen>
          <div class="info-grid">
            <div class="info-row"><span>Fabricant</span><span>{{ moboInfo.manufacturer }}</span></div>
            <div class="info-row"><span>Modèle</span><span>{{ moboInfo.product }}</span></div>
            <div class="info-row"><span>Version</span><span>{{ moboInfo.version || "N/A" }}</span></div>
            <div class="info-row"><span>Numéro de série</span><code>{{ moboInfo.serial_number || "N/A" }}</code></div>
            <div class="info-row"><span>Statut WMI</span>
              <NBadge :variant="moboInfo.status === 'OK' ? 'success' : 'warning'">{{ moboInfo.status }}</NBadge>
            </div>
          </div>
        </NCollapse>

        <template v-if="moboExtended">
          <NCollapse title="Informations système étendues" storageKey="diag-mobo-extended" defaultOpen>
            <div class="info-grid">
              <div v-if="moboExtended.model" class="info-row"><span>Modèle PC</span><span>{{ moboExtended.model }}</span></div>
              <div v-if="moboExtended.cpu_socket" class="info-row"><span>Socket CPU</span><NBadge variant="info">{{ moboExtended.cpu_socket }}</NBadge></div>
              <div class="info-row">
                <span>Temp. carte mère</span>
                <NBadge :variant="(moboExtended.motherboard_temp_c ?? 0) > 0 ? ((moboExtended.motherboard_temp_c ?? 0) > 70 ? 'danger' : (moboExtended.motherboard_temp_c ?? 0) > 50 ? 'warning' : 'success') : 'neutral'">
                  {{ (moboExtended.motherboard_temp_c ?? 0) > 0 ? moboExtended.motherboard_temp_c + ' °C' : 'Non disponible' }}
                </NBadge>
              </div>
            </div>
          </NCollapse>

          <NCollapse
            v-if="moboExtended.expansion_slots?.length"
            :title="`Slots d'extension (${moboExtended.slot_count})`"
            storageKey="diag-mobo-slots"
            :defaultOpen="false"
          >
            <div class="table-wrap">
              <table class="data-table">
                <thead>
                  <tr><th>Désignation</th><th>Type</th><th>Statut</th><th>Largeur</th></tr>
                </thead>
                <tbody>
                  <tr v-for="(slot, i) in moboExtended.expansion_slots" :key="i">
                    <td style="font-weight:500">{{ slot.name || `Slot ${i+1}` }}</td>
                    <td><NBadge variant="default" style="font-size:10px">{{ slot.type || '—' }}</NBadge></td>
                    <td>
                      <NBadge :variant="slot.status == 3 || slot.status === 'Available' ? 'success' : slot.status == 4 || slot.status === 'In Use' ? 'info' : 'neutral'" style="font-size:10px">
                        {{ slot.status == 3 ? 'Disponible' : slot.status == 4 ? 'Utilisé' : slot.status }}
                      </NBadge>
                    </td>
                    <td class="muted">{{ (slot.max_data_width ?? 0) > 0 ? slot.max_data_width + ' bit' : '—' }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </NCollapse>
        </template>
        <!-- moboExtended est résolu dans le même Promise.all que moboInfo côté
             DiagnosticPage.vue : même raisonnement que biosExtended ci-dessus. -->
        <div v-if="!moboExtended" class="diag-empty" style="margin-top:8px">Informations étendues non disponibles</div>

        <NCollapse title="À propos de cette carte mère" storageKey="diag-mobo-about" :defaultOpen="false">
          <div class="info-grid" style="margin-bottom:10px">
            <div class="info-row"><span>Fabricant</span><span>{{ moboInfo.manufacturer }}</span></div>
            <div class="info-row"><span>Modèle</span><span style="font-weight:600">{{ moboInfo.product }}</span></div>
            <div v-if="moboInfo.version && moboInfo.version !== 'N/A'" class="info-row"><span>Révision PCB</span><NBadge variant="info">{{ moboInfo.version }}</NBadge></div>
            <div v-if="moboExtended?.cpu_socket" class="info-row"><span>Socket CPU</span><NBadge variant="success">{{ moboExtended.cpu_socket }}</NBadge></div>
            <div v-if="(moboExtended?.slot_count ?? 0) > 0" class="info-row"><span>Slots d'extension</span><span>{{ moboExtended!.slot_count }} slots PCIe/PCI/M.2</span></div>
            <div v-if="(moboExtended?.motherboard_temp_c ?? 0) > 0" class="info-row"><span>Température actuelle</span>
              <NBadge :variant="(moboExtended!.motherboard_temp_c! > 70) ? 'danger' : (moboExtended!.motherboard_temp_c! > 50) ? 'warning' : 'success'">
                {{ moboExtended!.motherboard_temp_c }}°C
              </NBadge>
            </div>
          </div>
          <div class="card-block" style="margin-top:0;border:none;background:var(--bg-tertiary)">
            <p class="muted" style="font-size:11px;line-height:1.7">
              La <strong>carte mère</strong> est le circuit imprimé central qui interconnecte CPU, RAM, stockage, GPU et périphériques.
              Le <strong>numéro de série</strong> permet d'identifier la carte pour le support et la garantie fabricant.
              Pour les mises à jour BIOS/UEFI ou les manuels de votre carte, recherchez le modèle
              <strong>{{ moboInfo.product }}</strong> sur le site de <strong>{{ moboInfo.manufacturer }}</strong>.
            </p>
          </div>
        </NCollapse>
      </template>
      <div v-else class="diag-empty">Informations carte mère non disponibles (droits admin requis)</div>
    </div>
  </template>
</template>
