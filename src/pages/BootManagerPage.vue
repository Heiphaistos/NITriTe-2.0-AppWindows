<script setup lang="ts">
import { ref, onMounted } from "vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import { invoke } from "@/utils/invoke";
import type { CommandResult } from "@/types/diagnostic";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import NSkeleton from "@/components/ui/NSkeleton.vue";
import { useNotificationStore } from "@/stores/notifications";
import { Server, Star, Clock, RefreshCw, AlertTriangle, Shield, Pencil, Check, X, Settings } from "lucide-vue-next";

const notify = useNotificationStore();

interface BcdEntry {
  id: string; description: string; entry_type: string;
  device: string; path: string; locale: string; is_default: boolean;
}
interface BootConfig {
  entries: BcdEntry[]; default_id: string; timeout_secs: number;
  safe_mode: boolean; debug_mode: boolean;
  last_boot_time_secs?: number;
}

const config = ref<BootConfig | null>(null);
const loading = ref(true);
const newTimeout = ref(10);
const savingTimeout = ref(false);
const booting = ref(false);

// Edit description inline
const editingId = ref<string | null>(null);
const editingDesc = ref("");
const savingDesc = ref(false);

// Safe mode toggle
const togglingMode = ref(false);

function startEdit(e: BcdEntry) {
  editingId.value = e.id;
  editingDesc.value = e.description || "";
}

function cancelEdit() {
  editingId.value = null;
  editingDesc.value = "";
}

async function saveDesc(e: BcdEntry) {
  const desc = editingDesc.value.trim();
  if (!desc) return;
  if (desc.length > 200) { notify.error("Description trop longue", "Maximum 200 caractères."); return; }
  // Bloque les caractères pouvant causer une injection de commande via bcdedit
  if (/[&|;<>"`\n\r]/.test(desc)) { notify.error("Caractères invalides", "La description ne peut pas contenir & | ; < > \" ` ou des sauts de ligne."); return; }
  savingDesc.value = true;
  try {
    // run_system_command ne rejette jamais sur un code de sortie non-nul (voir
    // execute_system_command) — sans verifier .success, un bcdedit refuse
    // (app non elevee) affichait quand meme "Description mise a jour".
    const r = await invoke<CommandResult>("run_system_command", {
      cmd: "bcdedit",
      args: ["/set", `{${e.id}}`, "description", desc],
    });
    if (r.success) {
      notify.success("Description mise à jour");
      cancelEdit();
      await load();
    } else {
      notify.error("Échec de la mise à jour", r.stderr || r.stdout || `code ${r.exit_code}`);
    }
  } catch (err: unknown) {
    notify.error("Erreur", String(err));
  }
  savingDesc.value = false;
}

async function enableSafeMode() {
  if (!(await confirm("Activer le Safe Mode au prochain démarrage ? Le système redémarrera en mode minimal.", { title: "Nitrite", kind: "warning" }))) return;
  togglingMode.value = true;
  try {
    const r = await invoke<CommandResult>("run_system_command", {
      cmd: "bcdedit",
      args: ["/set", "{current}", "safeboot", "minimal"],
    });
    if (r.success) {
      notify.success("Safe Mode activé", "Prendra effet au prochain démarrage");
      await load();
    } else {
      notify.error("Échec de l'activation du Safe Mode", r.stderr || r.stdout || `code ${r.exit_code}`);
    }
  } catch (e: unknown) {
    notify.error("Erreur", String(e));
  }
  togglingMode.value = false;
}

async function disableSafeMode() {
  if (!(await confirm("Désactiver le Safe Mode ? Le système démarrera normalement au prochain redémarrage.", { title: "Nitrite", kind: "warning" }))) return;
  togglingMode.value = true;
  try {
    const r = await invoke<CommandResult>("run_system_command", {
      cmd: "bcdedit",
      args: ["/deletevalue", "{current}", "safeboot"],
    });
    if (r.success) {
      notify.success("Safe Mode désactivé");
      await load();
    } else {
      notify.error("Échec de la désactivation du Safe Mode", r.stderr || r.stdout || `code ${r.exit_code}`);
    }
  } catch (e: unknown) {
    notify.error("Erreur", String(e));
  }
  togglingMode.value = false;
}

async function openMsconfig() {
  try {
    await invoke("run_system_command", { cmd: "cmd", args: ["/c", "start", "msconfig"] });
  } catch (e: unknown) {
    notify.error("Erreur", String(e));
  }
}

async function load() {
  loading.value = true;
  try {
    config.value = await invoke<BootConfig>("get_boot_config");
    if (config.value) newTimeout.value = config.value.timeout_secs;
  } catch (e: unknown) {
    notify.error("Erreur BCD", String(e));
  }
  loading.value = false;
}

async function setDefault(id: string) {
  try {
    await invoke("set_default_boot", { id });
    notify.success("Entrée de démarrage modifiée");
    await load();
  } catch (e: unknown) {
    notify.error("Erreur", String(e));
  }
}

async function saveTimeout() {
  savingTimeout.value = true;
  try {
    await invoke("set_boot_timeout", { seconds: newTimeout.value });
    notify.success("Délai de démarrage mis à jour", `${newTimeout.value} secondes`);
    await load();
  } catch (e: unknown) {
    notify.error("Erreur", String(e));
  }
  savingTimeout.value = false;
}

async function bootToRecovery() {
  booting.value = true;
  try {
    await invoke("boot_to_recovery");
    notify.info("Redémarrage", "Le système va redémarrer en mode récupération...");
  } catch (e: unknown) {
    notify.error("Erreur", String(e));
  }
  booting.value = false;
}

onMounted(load);
</script>

<template>
  <div class="boot-page">
    <div class="page-header">
      <div class="header-icon"><Server :size="22" /></div>
      <div>
        <h1>Gestionnaire de Démarrage</h1>
        <p class="subtitle">Configuration BCD (Boot Configuration Data)</p>
      </div>
      <div class="header-actions">
        <NButton variant="ghost" size="sm" @click="openMsconfig">
          <Settings :size="13" /> msconfig
        </NButton>
        <NButton variant="ghost" size="sm" :loading="loading" @click="load">
          <RefreshCw :size="13" /> Actualiser
        </NButton>
      </div>
    </div>

    <div class="warning-banner">
      <AlertTriangle :size="14" />
      <span>Modifier la configuration de démarrage peut rendre le système inopérant. Procédez avec précaution.</span>
    </div>

    <!-- Boot time -->
    <div v-if="config?.last_boot_time_secs && config.last_boot_time_secs > 0" class="boot-time-banner">
      <Clock :size="14" />
      <span>Dernier démarrage : <strong>{{ config.last_boot_time_secs }}s</strong></span>
    </div>

    <div v-if="loading">
      <NSkeleton v-for="i in 3" :key="i" height="68px" style="margin-bottom:8px" />
    </div>

    <template v-else-if="config">
      <!-- Entrées BCD -->
      <NCard>
        <template #header>
          <div class="section-header"><Server :size="15" /><span>Entrées de démarrage ({{ config.entries.length }})</span></div>
        </template>
        <div v-if="config.entries.length === 0" class="empty-state">
          ⚠ Aucune entrée de démarrage détectée — la lecture de la configuration BCD a peut-être échoué. Réessayez ou consultez les journaux.
        </div>
        <div v-else class="entries-list">
          <div
            v-for="e in config.entries" :key="e.id"
            class="entry-card"
            :class="{ 'is-default': e.is_default }"
          >
            <div class="entry-left">
              <div class="entry-icon" :class="e.is_default ? 'default' : ''">
                <Star v-if="e.is_default" :size="16" />
                <Server v-else :size="16" />
              </div>
              <div class="entry-info">
                <!-- Description inline edit -->
                <div v-if="editingId === e.id" class="desc-edit-row">
                  <input
                    v-model="editingDesc"
                    class="desc-input"
                    @keydown.enter="saveDesc(e)"
                    @keydown.escape="cancelEdit"
                    autofocus
                  />
                  <NButton variant="primary" size="sm" :loading="savingDesc" @click="saveDesc(e)">
                    <Check :size="12" />
                  </NButton>
                  <NButton variant="ghost" size="sm" @click="cancelEdit">
                    <X :size="12" />
                  </NButton>
                </div>
                <span v-else class="entry-desc">{{ e.description || 'Windows Boot Manager' }}</span>
                <span class="entry-id">{{ e.id }}</span>
                <span class="entry-meta" v-if="e.device">{{ e.device }}</span>
              </div>
            </div>
            <div class="entry-right">
              <NBadge v-if="e.is_default" variant="success" dot>Défaut</NBadge>
              <NBadge v-if="e.entry_type" variant="neutral">{{ e.entry_type }}</NBadge>
              <NButton
                v-if="editingId !== e.id"
                variant="ghost"
                size="sm"
                @click="startEdit(e)"
                title="Modifier la description"
              >
                <Pencil :size="12" />
              </NButton>
              <NButton v-if="!e.is_default" variant="secondary" size="sm" @click="setDefault(e.id)">
                <Star :size="12" /> Définir par défaut
              </NButton>
            </div>
          </div>
        </div>
      </NCard>

      <!-- Délai de démarrage -->
      <NCard>
        <template #header>
          <div class="section-header"><Clock :size="15" /><span>Délai de démarrage</span></div>
        </template>
        <div class="timeout-row">
          <p class="timeout-desc">Temps d'affichage du menu de démarrage avant boot automatique</p>
          <div class="timeout-ctrl">
            <input type="range" v-model.number="newTimeout" min="0" max="60" step="1" class="timeout-slider" />
            <span class="timeout-val">{{ newTimeout }}s</span>
            <NButton variant="primary" size="sm" :loading="savingTimeout" @click="saveTimeout">Appliquer</NButton>
          </div>
        </div>
        <div class="info-row">
          <span>Mode actuel :</span>
          <NBadge v-if="config.safe_mode" variant="warning">Safe Mode</NBadge>
          <NBadge v-if="config.debug_mode" variant="danger">Debug Mode</NBadge>
          <NBadge v-if="!config.safe_mode && !config.debug_mode" variant="success">Normal</NBadge>
          <!-- Safe Mode toggle -->
          <NButton
            v-if="!config.safe_mode"
            variant="warning"
            size="sm"
            :loading="togglingMode"
            @click="enableSafeMode"
            style="margin-left:auto"
          >Activer Safe Mode</NButton>
          <NButton
            v-else
            variant="secondary"
            size="sm"
            :loading="togglingMode"
            @click="disableSafeMode"
            style="margin-left:auto"
          >Désactiver Safe Mode</NButton>
        </div>
      </NCard>

      <!-- Récupération -->
      <NCard>
        <template #header>
          <div class="section-header"><Shield :size="15" /><span>Récupération système</span></div>
        </template>
        <div class="recovery-section">
          <p class="recovery-desc">Redémarre le système en environnement de récupération Windows (WinRE) au prochain démarrage.</p>
          <NButton variant="danger" size="sm" :loading="booting" @click="bootToRecovery">
            <Shield :size="13" /> Redémarrer en récupération
          </NButton>
        </div>
      </NCard>
    </template>
  </div>
</template>

<style scoped>
.boot-page { display: flex; flex-direction: column; gap: 14px; }
.page-header { display: flex; align-items: center; gap: 12px; }
.header-icon { width: 42px; height: 42px; border-radius: var(--radius-lg); background: var(--info-muted, color-mix(in srgb, #60a5fa 15%, transparent)); display: flex; align-items: center; justify-content: center; color: #60a5fa; flex-shrink: 0; }
h1 { font-size: 22px; font-weight: 700; }
.subtitle { font-size: 12px; color: var(--text-muted); }
.header-actions { margin-left: auto; display: flex; gap: 6px; }
.warning-banner { display: flex; gap: 10px; align-items: center; padding: 10px 14px; background: var(--danger-muted); border: 1px solid color-mix(in srgb, var(--danger) 30%, transparent); border-radius: var(--radius-md); font-size: 12px; color: var(--danger); }
.boot-time-banner { display: flex; gap: 8px; align-items: center; padding: 8px 14px; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: var(--radius-md); font-size: 12px; color: var(--text-secondary); }
.section-header { display: flex; align-items: center; gap: 8px; }
.entries-list { display: flex; flex-direction: column; gap: 8px; }
.entry-card { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 14px; background: var(--bg-tertiary); border-radius: var(--radius-lg); border: 1px solid var(--border); transition: border-color .15s; }
.entry-card.is-default { border-color: var(--success); background: var(--success-muted); }
.entry-card:hover { border-color: var(--text-muted); }
.entry-left { display: flex; align-items: center; gap: 12px; flex: 1; min-width: 0; }
.entry-icon { width: 36px; height: 36px; border-radius: var(--radius-md); display: flex; align-items: center; justify-content: center; flex-shrink: 0; background: var(--bg-elevated); color: var(--text-muted); }
.entry-icon.default { background: var(--success-muted); color: var(--success); }
.entry-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1; }
.entry-desc { font-size: 13px; font-weight: 600; color: var(--text-primary); }
.entry-id { font-family: "JetBrains Mono", monospace; font-size: 10px; color: var(--text-muted); }
.entry-meta { font-size: 11px; color: var(--text-secondary); }
.entry-right { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
.desc-edit-row { display: flex; align-items: center; gap: 6px; }
.desc-input { flex: 1; min-width: 160px; padding: 4px 8px; background: var(--bg-elevated); border: 1px solid var(--accent-primary); border-radius: var(--radius-sm); color: var(--text-primary); font-size: 12px; outline: none; }
.timeout-row { display: flex; flex-direction: column; gap: 10px; }
.timeout-desc { font-size: 12px; color: var(--text-muted); }
.timeout-ctrl { display: flex; align-items: center; gap: 12px; }
.timeout-slider { flex: 1; accent-color: var(--accent-primary); }
.timeout-val { font-family: "JetBrains Mono", monospace; font-size: 14px; font-weight: 700; color: var(--accent-primary); width: 32px; }
.info-row { display: flex; align-items: center; gap: 8px; margin-top: 10px; font-size: 12px; color: var(--text-muted); }
.recovery-section { display: flex; align-items: flex-start; justify-content: space-between; gap: 14px; }
.recovery-desc { font-size: 12px; color: var(--text-secondary); max-width: 460px; line-height: 1.6; }
</style>
