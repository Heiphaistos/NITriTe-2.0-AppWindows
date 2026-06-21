<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import NBadge from "@/components/ui/NBadge.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NButton from "@/components/ui/NButton.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import { Shield, CheckCircle, AlertTriangle, ArrowRight, ArrowLeft, Settings, ExternalLink, RefreshCw } from "lucide-vue-next";
import { useExportData } from "@/composables/useExportData";

const fwActionLoading = ref(false);
const fwMsg = ref("");
const fwMsgErr = ref(false);

function showFwMsg(msg: string, err = false) {
  fwMsg.value = msg; fwMsgErr.value = err;
  setTimeout(() => { fwMsg.value = ""; }, 4000);
}

async function enableAllProfiles() {
  fwActionLoading.value = true;
  try {
    await invoke("enable_firewall_all_profiles");
    showFwMsg("Pare-feu activé sur tous les profils ✓");
  } catch (e: any) { showFwMsg("Erreur : " + String(e), true); }
  fwActionLoading.value = false;
}

async function disableAllProfiles() {
  if (!confirm("Désactiver le pare-feu sur tous les profils ? Cela réduit la sécurité du système.")) return;
  fwActionLoading.value = true;
  try {
    await invoke("run_system_command", {
      cmd: "cmd",
      args: ["/c", "netsh advfirewall set allprofiles state off"]
    });
    showFwMsg("Pare-feu désactivé sur tous les profils");
  } catch (e: any) { showFwMsg("Erreur : " + String(e), true); }
  fwActionLoading.value = false;
}

async function openFirewallSettings() {
  await invoke("run_system_command", { cmd: "cmd", args: ["/c", "start", "wf.msc"] }).catch(() => {});
}

async function openDefenderSettings() {
  await invoke("open_url", { url: "ms-settings:windowsdefender" }).catch(() => {});
}

interface FirewallRule {
  name: string; direction: string; action: string; enabled: boolean;
  profile: string; protocol: string; local_port: string; program: string; group: string;
}
interface FirewallProfile {
  name: string; enabled: boolean; default_inbound: string; default_outbound: string; log_file: string;
}
interface FirewallInfo {
  profiles: FirewallProfile[];
  rules: FirewallRule[];
  inbound_allow: number; inbound_block: number;
  outbound_allow: number; outbound_block: number; total_custom: number;
}

const data = ref<FirewallInfo | null>(null);
const loading = ref(true);
const error = ref("");
const filter = ref<"all" | "in" | "out" | "block">("all");
const { exportCSV } = useExportData();

onMounted(async () => {
  try {
    data.value = await invoke<FirewallInfo>("get_firewall_rules", undefined, 60000);
  } catch (e: any) { error.value = e?.toString() ?? "Erreur"; }
  finally { loading.value = false; }
});

function filteredRules(rules: FirewallRule[]) {
  if (filter.value === "in") return rules.filter(r => r.direction === "Inbound");
  if (filter.value === "out") return rules.filter(r => r.direction === "Outbound");
  if (filter.value === "block") return rules.filter(r => r.action === "Block");
  return rules;
}

function doExportRules() {
  if (!data.value) return;
  exportCSV(data.value.rules.map(r => ({
    Nom: r.name, Direction: r.direction, Action: r.action,
    Profil: r.profile, Protocole: r.protocol, Port: r.local_port,
    Programme: r.program, Groupe: r.group,
  })), 'firewall-regles-' + new Date().toISOString().slice(0,10));
}
</script>

<template>
  <div class="diag-tab-content">
    <DiagBanner :icon="Shield" title="Pare-feu Windows" desc="Règles de filtrage réseau et profils de sécurité" color="red" />

    <!-- Actions rapides pare-feu -->
    <div class="diag-section" style="display:flex;gap:8px;flex-wrap:wrap;align-items:center">
      <NButton variant="success" size="sm" :disabled="fwActionLoading" @click="enableAllProfiles">
        <NSpinner v-if="fwActionLoading" :size="12" />
        <Shield v-else :size="13" /> Activer tous les profils
      </NButton>
      <NButton variant="ghost" size="sm" :disabled="fwActionLoading" @click="disableAllProfiles">
        <Shield :size="13" /> Désactiver pare-feu
      </NButton>
      <NButton variant="ghost" size="sm" @click="openFirewallSettings">
        <Settings :size="13" /> Ouvrir wf.msc (avancé)
      </NButton>
      <NButton variant="ghost" size="sm" @click="openDefenderSettings">
        <ExternalLink :size="13" /> Paramètres sécurité Windows
      </NButton>
      <NButton variant="ghost" size="sm" @click="doExportRules" :disabled="!data">
        ↓ Export règles CSV
      </NButton>
      <span v-if="fwMsg"
        :style="{ fontSize: '12px', color: fwMsgErr ? 'var(--error)' : 'var(--success)', padding: '4px 10px', borderRadius: '6px', background: fwMsgErr ? 'rgba(239,68,68,.08)' : 'rgba(34,197,94,.08)' }">
        {{ fwMsg }}
      </span>
    </div>

    <div v-if="loading" class="diag-loading"><div class="diag-spinner"></div> Chargement règles pare-feu...</div>
    <div v-else-if="error" style="color:var(--error)">⚠ {{ error }}</div>
    <div v-else-if="data" style="display:flex;flex-direction:column;gap:14px">

      <!-- Profils -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Profils Pare-feu Windows</p>
        <div style="display:flex;gap:10px;flex-wrap:wrap">
          <div v-for="p in data.profiles" :key="p.name"
            style="flex:1;min-width:160px;background:var(--bg-secondary);border-radius:6px;padding:12px;border:1px solid var(--border)">
            <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px">
              <component :is="p.enabled ? CheckCircle : AlertTriangle" :size="14"
                :class="p.enabled ? 'ic-ok' : 'ic-warn'" />
              <strong style="font-size:13px">{{ p.name }}</strong>
            </div>
            <NBadge :variant="p.enabled ? 'success' : 'danger'" style="margin-bottom:6px">
              {{ p.enabled ? 'Activé' : 'DÉSACTIVÉ' }}
            </NBadge>
            <div style="font-size:12px;color:var(--text-secondary);margin-top:4px">
              Entrant: {{ p.default_inbound }} | Sortant: {{ p.default_outbound }}
            </div>
          </div>
        </div>
      </div>

      <!-- Stats règles -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Statistiques</p>
        <div class="info-grid">
          <div class="info-row"><span>Règles personnalisées actives</span>
            <NBadge variant="info">{{ data.total_custom }}</NBadge>
          </div>
          <div class="info-row">
            <span style="display:flex;align-items:center;gap:4px"><ArrowLeft :size="11" /> Entrant Autorisé</span>
            <NBadge variant="success">{{ data.inbound_allow }}</NBadge>
          </div>
          <div class="info-row">
            <span style="display:flex;align-items:center;gap:4px"><ArrowLeft :size="11" /> Entrant Bloqué</span>
            <NBadge :variant="data.inbound_block > 0 ? 'warning' : 'neutral'">{{ data.inbound_block }}</NBadge>
          </div>
          <div class="info-row">
            <span style="display:flex;align-items:center;gap:4px"><ArrowRight :size="11" /> Sortant Autorisé</span>
            <NBadge variant="success">{{ data.outbound_allow }}</NBadge>
          </div>
          <div class="info-row">
            <span style="display:flex;align-items:center;gap:4px"><ArrowRight :size="11" /> Sortant Bloqué</span>
            <NBadge :variant="data.outbound_block > 0 ? 'warning' : 'neutral'">{{ data.outbound_block }}</NBadge>
          </div>
        </div>
      </div>

      <!-- Règles -->
      <div class="diag-section">
        <div style="display:flex;align-items:center;gap:8px;margin-bottom:10px;flex-wrap:wrap">
          <p class="diag-section-label" style="margin:0;flex:1">Règles personnalisées ({{ data.rules.length }})</p>
          <div style="display:flex;gap:4px">
            <button v-for="[k, l] in [['all','Tout'],['in','Entrant'],['out','Sortant'],['block','Bloqué']]" :key="k"
              @click="filter = k as any"
              :style="{
                padding:'3px 10px', borderRadius:'4px', border:'1px solid var(--border)',
                fontSize:'11px', cursor:'pointer',
                background: filter === k ? 'var(--accent)' : 'var(--bg-secondary)',
                color: filter === k ? 'white' : 'var(--text-secondary)'
              }">{{ l }}</button>
          </div>
        </div>
        <div v-if="!data.rules.length" class="muted" style="font-size:13px">Aucune règle personnalisée trouvée.</div>
        <div v-else style="overflow-x:auto">
          <table style="width:100%;border-collapse:collapse;font-size:11px">
            <thead>
              <tr style="background:var(--bg-secondary)">
                <th style="padding:5px 8px;text-align:left;color:var(--text-secondary);font-weight:500;min-width:160px">Nom</th>
                <th style="padding:5px 8px;text-align:left;color:var(--text-secondary);font-weight:500">Direction</th>
                <th style="padding:5px 8px;text-align:left;color:var(--text-secondary);font-weight:500">Action</th>
                <th style="padding:5px 8px;text-align:left;color:var(--text-secondary);font-weight:500">Profil</th>
                <th style="padding:5px 8px;text-align:left;color:var(--text-secondary);font-weight:500">Proto</th>
                <th style="padding:5px 8px;text-align:left;color:var(--text-secondary);font-weight:500">Port</th>
                <th style="padding:5px 8px;text-align:left;color:var(--text-secondary);font-weight:500;min-width:140px">Programme</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(r, i) in filteredRules(data.rules)" :key="i"
                style="border-bottom:1px solid var(--border)">
                <td style="padding:5px 8px;font-size:11px">{{ r.name }}</td>
                <td style="padding:5px 8px">
                  <NBadge :variant="r.direction === 'Inbound' ? 'info' : 'neutral'" style="font-size:9px">{{ r.direction }}</NBadge>
                </td>
                <td style="padding:5px 8px">
                  <NBadge :variant="r.action === 'Allow' ? 'success' : 'danger'" style="font-size:9px">{{ r.action }}</NBadge>
                </td>
                <td style="padding:5px 8px;color:var(--text-secondary)">{{ r.profile }}</td>
                <td style="padding:5px 8px;color:var(--text-secondary)">{{ r.protocol || 'Any' }}</td>
                <td style="padding:5px 8px;color:var(--text-secondary)">{{ r.local_port || 'Any' }}</td>
                <td style="padding:5px 8px;color:var(--text-secondary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:200px">
                  {{ r.program && r.program !== '%SystemRoot%\\system32\\svchost.exe' ? r.program : '—' }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>
