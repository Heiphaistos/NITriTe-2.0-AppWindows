<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import NBadge from "@/components/ui/NBadge.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NButton from "@/components/ui/NButton.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import { Share2, FolderOpen, HardDrive, Globe, Lock, Settings, RefreshCw, XCircle } from "lucide-vue-next";

const actionMsg = ref("");
const actionLoading = ref(false);

async function openShareMgmt() {
  await invoke("run_system_command", { cmd: "cmd", args: ["/c", "start", "fsmgmt.msc"] }).catch(() => {});
}

async function openComputerMgmt() {
  await invoke("run_system_command", { cmd: "cmd", args: ["/c", "start", "compmgmt.msc"] }).catch(() => {});
}

async function openPrinterMgmt() {
  await invoke("run_system_command", { cmd: "control", args: ["printers"] }).catch(() => {
    invoke("run_system_command", { cmd: "cmd", args: ["/c", "start", "ms-settings:printers"] }).catch(() => {});
  });
}

async function disconnectAllSessions() {
  if (!confirm("Déconnecter toutes les sessions SMB actives ?")) return;
  actionLoading.value = true;
  try {
    await invoke("run_system_command", {
      cmd: "cmd",
      args: ["/c", "net session /delete /y"]
    });
    actionMsg.value = "Sessions SMB déconnectées ✓";
    setTimeout(() => data.value && (data.value.smb_sessions = []), 500);
  } catch (e: any) {
    actionMsg.value = "Erreur : " + String(e);
  } finally {
    actionLoading.value = false;
    setTimeout(() => { actionMsg.value = ""; }, 4000);
  }
}

interface NetworkShare {
  name: string; path: string; description: string;
  share_type: string; current_uses: number; max_uses: number; is_hidden: boolean;
}
interface MappedDrive {
  drive_letter: string; remote_path: string; status: string;
}
interface SmbSession {
  client_name: string; client_ip: string; user: string; idle_time: string;
}
interface SharesInfo {
  shares: NetworkShare[]; admin_shares: NetworkShare[];
  mapped_drives: MappedDrive[]; smb_sessions: SmbSession[];
  open_files: number; computer_name: string; workgroup: string;
}

const data = ref<SharesInfo | null>(null);
const loading = ref(true);
const error = ref("");

onMounted(async () => {
  try {
    data.value = await invoke<SharesInfo>("get_network_shares");
  } catch (e: any) { error.value = e?.toString() ?? "Erreur"; }
  finally { loading.value = false; }
});
</script>

<template>
  <div class="diag-tab-content">
    <DiagBanner :icon="Share2" title="Partages & SMB" desc="Dossiers partagés et sessions réseau actives" color="blue" />

    <!-- Actions rapides -->
    <div class="diag-section" style="display:flex;gap:8px;flex-wrap:wrap;align-items:center">
      <NButton variant="ghost" size="sm" @click="openShareMgmt">
        <FolderOpen :size="13" /> Gestion des partages (fsmgmt)
      </NButton>
      <NButton variant="ghost" size="sm" @click="openComputerMgmt">
        <Settings :size="13" /> Gestion de l'ordinateur
      </NButton>
      <NButton variant="ghost" size="sm" @click="openPrinterMgmt">
        <Settings :size="13" /> Gestion des imprimantes
      </NButton>
      <NButton v-if="data && data.smb_sessions.length > 0" variant="danger" size="sm"
        :disabled="actionLoading" @click="disconnectAllSessions">
        <NSpinner v-if="actionLoading" :size="12" />
        <XCircle v-else :size="13" /> Déconnecter toutes les sessions SMB
      </NButton>
      <span v-if="actionMsg" style="font-size:12px;color:var(--success)">{{ actionMsg }}</span>
    </div>

    <div v-if="loading" class="diag-loading"><div class="diag-spinner"></div> Chargement partages réseau...</div>
    <div v-else-if="error" style="color:var(--error)">⚠ {{ error }}</div>
    <div v-else-if="data" style="display:flex;flex-direction:column;gap:14px">

      <!-- Identité réseau -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Identité Réseau</p>
        <div class="info-grid">
          <div class="info-row"><span>Nom d'ordinateur</span><code>{{ data.computer_name }}</code></div>
          <div v-if="data.workgroup" class="info-row"><span>Groupe de travail / Domaine</span><span>{{ data.workgroup }}</span></div>
          <div class="info-row"><span>Fichiers ouverts (SMB)</span>
            <NBadge :variant="data.open_files > 0 ? 'warning' : 'neutral'">{{ data.open_files }}</NBadge>
          </div>
          <div class="info-row"><span>Sessions SMB actives</span>
            <NBadge :variant="data.smb_sessions.length > 0 ? 'warning' : 'neutral'">{{ data.smb_sessions.length }}</NBadge>
          </div>
        </div>
      </div>

      <!-- Partages personnalisés -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">
          <FolderOpen :size="13" style="display:inline;margin-right:4px" />Partages réseau ({{ data.shares.length }})
        </p>
        <div v-if="!data.shares.length" class="muted" style="font-size:13px">Aucun partage personnalisé configuré.</div>
        <div v-for="(s, i) in data.shares" :key="i"
          style="display:flex;align-items:center;gap:10px;padding:8px 0;border-bottom:1px solid var(--border);flex-wrap:wrap">
          <FolderOpen :size="14" style="color:var(--accent);flex-shrink:0" />
          <div style="flex:1;min-width:120px">
            <code style="font-size:12px">{{ s.name }}</code>
            <div class="muted" style="font-size:11px">{{ s.description }}</div>
          </div>
          <code style="font-size:12px;color:var(--text-secondary);flex:1;min-width:160px">{{ s.path }}</code>
          <NBadge variant="neutral">{{ s.share_type }}</NBadge>
          <NBadge :variant="s.current_uses > 0 ? 'warning' : 'neutral'" style="font-size:10px">
            {{ s.current_uses }} connexion(s)
          </NBadge>
          <span class="muted" style="font-size:10px">
            Max: {{ s.max_uses > 0 ? s.max_uses : '∞' }}
          </span>
        </div>
      </div>

      <!-- Partages administratifs -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">
          <Lock :size="13" style="display:inline;margin-right:4px" />Partages administratifs ({{ data.admin_shares.length }})
        </p>
        <div style="display:flex;gap:6px;flex-wrap:wrap">
          <div v-for="(s, i) in data.admin_shares" :key="i"
            style="background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;padding:8px 12px;min-width:120px">
            <code style="font-size:12px;color:var(--accent)">{{ s.name }}</code>
            <div class="muted" style="font-size:10px;margin-top:2px">{{ s.path || '—' }}</div>
            <NBadge :variant="s.current_uses > 0 ? 'warning' : 'neutral'" style="font-size:9px;margin-top:4px">
              {{ s.current_uses }} actif(s)
            </NBadge>
          </div>
        </div>
      </div>

      <!-- Lecteurs mappés -->
      <div v-if="data.mapped_drives.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">
          <HardDrive :size="13" style="display:inline;margin-right:4px" />Lecteurs réseau mappés
        </p>
        <div v-for="(d, i) in data.mapped_drives" :key="i"
          style="display:flex;align-items:center;gap:10px;padding:6px 0;border-bottom:1px solid var(--border);font-size:12px">
          <code style="min-width:30px;color:var(--accent)">{{ d.drive_letter }}</code>
          <Globe :size="12" class="muted" />
          <span style="flex:1">{{ d.remote_path }}</span>
          <NBadge :variant="d.status === 'OK' ? 'success' : 'warning'" style="font-size:10px">{{ d.status || "—" }}</NBadge>
        </div>
      </div>

      <!-- Sessions SMB -->
      <div v-if="data.smb_sessions.length" class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Sessions SMB actives</p>
        <div v-for="(s, i) in data.smb_sessions" :key="i"
          style="display:flex;align-items:center;gap:10px;padding:6px 0;border-bottom:1px solid var(--border);font-size:12px">
          <Globe :size="12" style="color:var(--warning)" />
          <span style="flex:1">{{ s.client_name || s.client_ip }}</span>
          <span class="muted">{{ s.user }}</span>
          <span class="muted" style="font-size:11px">Idle: {{ s.idle_time }}s</span>
        </div>
      </div>
    </div>
  </div>
</template>
