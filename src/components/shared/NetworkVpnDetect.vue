<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@/utils/invoke";
import type { CommandResult } from "@/types/diagnostic";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NBadge from "@/components/ui/NBadge.vue";
import { ShieldCheck, ShieldOff, AlertTriangle } from "lucide-vue-next";

// `detected` est `null` quand la vérification elle-même a échoué (commande PS
// en erreur, sortie vide/non-JSON...) — distinct de `false` (vérifié, aucun
// VPN trouvé). Auparavant les deux cas étaient confondus et affichaient le
// même badge vert "Pas de VPN", un faux résultat négatif rassurant alors que
// la détection n'avait en réalité pas pu s'exécuter.
interface VpnStatus { detected: boolean | null; adapter_name: string; details: string; }

const vpnChecking = ref(false);
const vpnStatus   = ref<VpnStatus | null>(null);

async function detectVpn() {
  vpnChecking.value = true;
  vpnStatus.value = null;
  try {
    const ps = `
$vpnKeywords = @('vpn','tun','tap','wireguard','nordvpn','expressvpn','openvpn','cisco','pulse','anyconnect','forti','globalprotect','proton','mullvad','surfshark')
$adapters = Get-NetAdapter -ErrorAction SilentlyContinue | Where-Object { $_.Status -eq 'Up' }
$found = $adapters | Where-Object {
    $n = $_.InterfaceDescription.ToLower() + ' ' + $_.Name.ToLower()
    $match = $false
    foreach ($kw in $vpnKeywords) { if ($n -like "*$kw*") { $match = $true; break } }
    $match
} | Select-Object -First 1
if ($found) {
    @{ detected=$true; name=[string]$found.Name; desc=[string]$found.InterfaceDescription } | ConvertTo-Json -Compress
} else {
    @{ detected=$false; name=''; desc='Aucun adaptateur VPN detecte' } | ConvertTo-Json -Compress
}`;
    const res = await invoke<CommandResult>("run_system_command", {
      cmd: "powershell",
      args: ["-NoProfile", "-NonInteractive", "-Command", ps],
    });
    const out = (res?.stdout ?? "").trim();
    const parsed = JSON.parse(out);
    vpnStatus.value = {
      detected: parsed.detected === true,
      adapter_name: parsed.name || "",
      details: parsed.desc || "",
    };
  } catch {
    vpnStatus.value = { detected: null, adapter_name: "", details: "La vérification a échoué (commande indisponible ou bloquée)." };
  }
  vpnChecking.value = false;
}
</script>

<template>
  <NCard>
    <template #header>
      <div class="section-header">
        <ShieldCheck :size="16" />
        <span>Detection VPN</span>
        <NBadge
          v-if="vpnStatus"
          :variant="vpnStatus.detected === null ? 'neutral' : vpnStatus.detected ? 'warning' : 'success'"
          style="margin-left:auto"
        >
          {{ vpnStatus.detected === null ? "Vérification échouée" : vpnStatus.detected ? "VPN Actif" : "Pas de VPN" }}
        </NBadge>
      </div>
    </template>
    <div class="vpn-zone">
      <NButton variant="primary" size="md" :loading="vpnChecking" @click="detectVpn">
        <ShieldCheck :size="14" />
        Analyser les adaptateurs VPN
      </NButton>
      <div v-if="vpnStatus" class="vpn-result" :class="vpnStatus.detected === null ? 'vpn-unknown' : vpnStatus.detected ? 'vpn-active' : 'vpn-none'">
        <component :is="vpnStatus.detected === null ? AlertTriangle : vpnStatus.detected ? ShieldOff : ShieldCheck" :size="18" />
        <div class="vpn-text">
          <span class="vpn-title">{{ vpnStatus.detected === null ? "Vérification impossible" : vpnStatus.detected ? "VPN detecte" : "Aucun VPN detecte" }}</span>
          <span v-if="vpnStatus.adapter_name" class="vpn-sub font-mono">
            Adaptateur : {{ vpnStatus.adapter_name }}
          </span>
          <span class="vpn-sub">{{ vpnStatus.details }}</span>
        </div>
      </div>
      <p class="vpn-note">
        Detection basee sur les noms d'adaptateurs reseau (WireGuard, OpenVPN, TAP, NordVPN, etc.).
        Un VPN actif peut modifier vos DNS et votre IP publique.
      </p>
    </div>
  </NCard>
</template>

<style scoped>
.section-header { display:flex; align-items:center; gap:8px; }
.vpn-zone { display:flex; flex-direction:column; gap:12px; }
.vpn-result { display:flex; align-items:flex-start; gap:12px; padding:12px 16px; border-radius:var(--radius-md); border:1px solid var(--border); }
.vpn-active { background:rgba(245,158,11,.08); border-color:rgba(245,158,11,.35); color:var(--warning); }
.vpn-none { background:rgba(34,197,94,.08); border-color:rgba(34,197,94,.25); color:var(--success); }
.vpn-unknown { background:rgba(245,158,11,.08); border-color:rgba(245,158,11,.25); color:var(--warning); }
.vpn-text { display:flex; flex-direction:column; gap:3px; }
.vpn-title { font-size:13px; font-weight:600; color:var(--text-primary); }
.vpn-sub { font-size:12px; color:var(--text-muted); }
.vpn-note { font-size:11px; color:var(--text-muted); line-height:1.5; margin:0; }
.font-mono { font-family:"JetBrains Mono",monospace; font-size:12px; }
</style>
