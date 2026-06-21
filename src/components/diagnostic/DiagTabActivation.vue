<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import { cachedInvoke } from "@/composables/useCachedInvoke";
import { Key, Shield, ShieldCheck, ShieldOff, ExternalLink, AlertTriangle, CheckCircle } from "lucide-vue-next";
import NBadge from "@/components/ui/NBadge.vue";
import NButton from "@/components/ui/NButton.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";

interface WinLicense {
  product_name: string; activation_status: string; partial_product_key: string;
  full_product_key: string; office_name: string; office_status: string;
  office_key: string; office_full_key: string;
}
interface ProductKey { product: string; key: string; key_type: string; }

const licenseInfo = ref<WinLicense | null>(null);
const loading = ref(true);
const activating = ref(false);
const statusMsg = ref("");
const allKeys = ref<ProductKey[]>([]);
const keysLoading = ref(false);
const keysLoaded = ref(false);

onMounted(async () => {
  try { licenseInfo.value = await cachedInvoke<WinLicense>("get_windows_license"); }
  catch { /* silencieux */ }
  loading.value = false;
});

async function scanAllKeys() {
  keysLoading.value = true;
  try { allKeys.value = await invoke<ProductKey[]>("get_all_product_keys"); }
  catch { allKeys.value = []; }
  finally { keysLoading.value = false; keysLoaded.value = true; }
}

function keyTypeVariant(t: string): "success"|"warning"|"info"|"default" {
  if (t === "Windows") return "success";
  if (t === "Office") return "warning";
  if (t === "Software") return "info";
  return "default";
}

async function openMas() {
  activating.value = true;
  statusMsg.value = "Lancement de MAS en administrateur...";
  try {
    const cmd1 = `powershell -WindowStyle Hidden -Command "Start-Process powershell -Verb RunAs -ArgumentList '-NoProfile -WindowStyle Normal -Command irm https://get.activated.win | iex'"`;
    await invoke("execute_script", { content: cmd1, scriptType: "cmd" }).catch(async () => {
      const cmd2 = `powershell -WindowStyle Hidden -Command "Start-Process powershell -Verb RunAs -ArgumentList '-NoProfile -WindowStyle Normal -Command [Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; irm https://get.activated.win | iex'"`;
      await invoke("execute_script", { content: cmd2, scriptType: "cmd" });
    });
    statusMsg.value = "Fenêtre PowerShell MAS ouverte. Suivez les instructions à l'écran.";
  } catch {
    statusMsg.value = "Erreur: lancez manuellement: irm https://get.activated.win | iex";
  }
  activating.value = false;
}

function actStatus(s: string): "success" | "danger" | "warning" | "default" {
  if (!s) return "default";
  const l = s.toLowerCase();
  if (l.includes("licenci") || l.includes("activ")) return "success";
  if (l.includes("non") || l.includes("invalide") || l.includes("expire")) return "danger";
  return "warning";
}
</script>

<template>
  <div class="diag-tab-content">
    <DiagBanner :icon="Key" title="Activation Windows" desc="Statut de la licence et clés produit" color="gold" />

    <!-- Titre -->
    <p class="diag-section-label">État d'activation</p>

    <div v-if="loading" class="diag-loading">
      <NSpinner :size="14" class="diag-spinner" /> Chargement...
    </div>
    <template v-else>
      <!-- Statut Windows -->
      <div class="card-block" style="margin-bottom:12px">
        <div class="block-title">
          <Key :size="15" style="color:var(--accent)" />
          <span>Windows</span>
          <NBadge v-if="licenseInfo" :variant="actStatus(licenseInfo.activation_status)">
            {{ licenseInfo.activation_status || "Inconnu" }}
          </NBadge>
        </div>
        <div v-if="licenseInfo" class="info-grid">
          <div class="info-row"><span>Produit</span><span>{{ licenseInfo.product_name }}</span></div>
          <div class="info-row"><span>Clé complète</span>
            <code v-if="licenseInfo.full_product_key" style="color:var(--accent)">{{ licenseInfo.full_product_key }}</code>
            <span v-else class="muted">XXXXX-XXXXX-XXXXX-XXXXX-{{ licenseInfo.partial_product_key }}</span>
          </div>
        </div>
      </div>

      <!-- Statut Office (affiché même si clé non récupérable — ex: Ohook/KMS) -->
      <div v-if="licenseInfo && (licenseInfo.office_name || licenseInfo.office_status)" class="card-block" style="margin-bottom:12px">
        <div class="block-title">
          <Key :size="15" style="color:var(--warning)" />
          <span>{{ licenseInfo.office_name || "Microsoft Office" }}</span>
          <NBadge :variant="actStatus(licenseInfo.office_status)">{{ licenseInfo.office_status || "Inconnu" }}</NBadge>
        </div>
        <div class="info-grid">
          <div class="info-row"><span>Clé complète</span>
            <code v-if="licenseInfo.office_full_key" style="color:var(--accent)">{{ licenseInfo.office_full_key }}</code>
            <span v-else-if="licenseInfo.office_key" class="muted">XXXXX-XXXXX-XXXXX-XXXXX-{{ licenseInfo.office_key }}</span>
            <span v-else class="muted">Non disponible</span>
          </div>
        </div>
      </div>
    </template>

    <!-- MAS Activation -->
    <p class="diag-section-label">Activation MAS (Microsoft Activation Scripts)</p>

    <!-- Warning -->
    <div class="card-block" style="border:1px solid var(--warning);margin-bottom:12px">
      <div style="display:flex;gap:8px;align-items:flex-start">
        <AlertTriangle :size="15" style="color:var(--warning);flex-shrink:0;margin-top:2px" />
        <div style="font-size:12px;line-height:1.6;color:var(--text-secondary)">
          <strong style="color:var(--text)">MAS</strong> (massgravel) est un projet open-source qui utilise des méthodes
          légitimes reconnues par Microsoft (HWID, KMS38) pour activer Windows et Office.
          L'activation s'effectue dans une <strong>fenêtre PowerShell élevée</strong> séparée — vous verrez exactement
          ce qui se passe. Aucune donnée n'est envoyée à des tiers.
          <br><br>
          Source officielle :
          <code style="font-size:10px">github.com/massgravel/Microsoft-Activation-Scripts</code>
        </div>
      </div>
    </div>

    <!-- Méthodes -->
    <div class="card-block" style="margin-bottom:12px">
      <p style="font-size:12px;color:var(--text-secondary);margin-bottom:12px">
        Cliquer sur un bouton ouvre une fenêtre PowerShell en mode administrateur avec le menu MAS interactif.
        Sélectionnez la méthode souhaitée dans le menu affiché.
      </p>

      <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;margin-bottom:12px">
        <!-- HWID -->
        <div class="card-block" style="margin:0;border:1px solid var(--border-secondary)">
          <div style="display:flex;align-items:center;gap:6px;margin-bottom:6px">
            <ShieldCheck :size="14" style="color:var(--success)" />
            <strong style="font-size:13px">HWID — Windows</strong>
            <NBadge variant="success" style="font-size:10px">Permanent</NBadge>
          </div>
          <p style="font-size:12px;color:var(--text-secondary);margin-bottom:10px">
            Licence numérique permanente liée au matériel. Windows 10/11. Nécessite internet.
          </p>
        </div>

        <!-- KMS38 -->
        <div class="card-block" style="margin:0;border:1px solid var(--border-secondary)">
          <div style="display:flex;align-items:center;gap:6px;margin-bottom:6px">
            <Shield :size="14" style="color:var(--info)" />
            <strong style="font-size:13px">KMS38 — Windows</strong>
            <NBadge variant="info" style="font-size:10px">Jusqu'en 2038</NBadge>
          </div>
          <p style="font-size:12px;color:var(--text-secondary);margin-bottom:10px">
            Activation KMS valide jusqu'en 2038. Windows 10/11. Sans internet.
          </p>
        </div>

        <!-- Online KMS -->
        <div class="card-block" style="margin:0;border:1px solid var(--border-secondary)">
          <div style="display:flex;align-items:center;gap:6px;margin-bottom:6px">
            <Shield :size="14" style="color:var(--warning)" />
            <strong style="font-size:13px">KMS Online — W+Office</strong>
            <NBadge variant="warning" style="font-size:10px">180 jours</NBadge>
          </div>
          <p style="font-size:12px;color:var(--text-secondary);margin-bottom:10px">
            Active Windows ET Office via KMS. Renouvellement auto tous les 180 jours.
          </p>
        </div>

        <!-- Ohook -->
        <div class="card-block" style="margin:0;border:1px solid var(--border-secondary)">
          <div style="display:flex;align-items:center;gap:6px;margin-bottom:6px">
            <ShieldCheck :size="14" style="color:var(--success)" />
            <strong style="font-size:13px">Ohook — Office</strong>
            <NBadge variant="success" style="font-size:10px">Permanent</NBadge>
          </div>
          <p style="font-size:12px;color:var(--text-secondary);margin-bottom:10px">
            Activation permanente d'Office (toutes versions). Sans internet.
          </p>
        </div>
      </div>

      <!-- Bouton unique MAS -->
      <div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap">
        <NButton variant="primary" :disabled="activating" @click="openMas">
          <NSpinner v-if="activating" :size="13" />
          <Key v-else :size="14" />
          {{ activating ? 'Ouverture...' : 'Ouvrir le menu MAS (admin)' }}
        </NButton>
        <NButton variant="ghost" size="sm" @click="invoke('open_url', { url: 'https://github.com/massgravel/Microsoft-Activation-Scripts' }).catch(() => {})">
          <ExternalLink :size="12" /> GitHub MAS
        </NButton>
        <NButton variant="ghost" size="sm" @click="invoke('open_url', { url: 'https://massgrave.dev/office_c2r_links' }).catch(() => {})">
          <ExternalLink :size="12" /> Télécharger Office (Français)
        </NButton>
      </div>

      <div v-if="statusMsg" style="margin-top:10px;font-size:12px;display:flex;align-items:center;gap:6px"
        :style="{ color: statusMsg.startsWith('Erreur') ? 'var(--error)' : 'var(--success)' }">
        <CheckCircle v-if="!statusMsg.startsWith('Erreur')" :size="13" />
        {{ statusMsg }}
      </div>
    </div>

    <!-- Scanner toutes les clés -->
    <p class="diag-section-label">Scanner toutes les clés de licence</p>
    <div class="card-block" style="margin-bottom:12px">
      <div style="display:flex;align-items:center;gap:8px;margin-bottom:10px">
        <NButton variant="secondary" :disabled="keysLoading" @click="scanAllKeys">
          <NSpinner v-if="keysLoading" :size="13" />
          <Key v-else :size="14" />
          {{ keysLoading ? 'Analyse du registre...' : 'Scanner les clés (Windows, Office, logiciels)' }}
        </NButton>
      </div>
      <div v-if="keysLoaded">
        <div v-if="allKeys.length === 0" style="font-size:12px;color:var(--text-muted)">
          Aucune clé de licence détectée dans le registre.
        </div>
        <div v-else>
          <div style="margin-bottom:8px;font-size:12px;color:var(--text-secondary)">{{ allKeys.length }} clé(s) trouvée(s) :</div>
          <div class="table-wrap">
            <table class="data-table">
              <thead><tr><th>Type</th><th>Produit</th><th>Clé</th></tr></thead>
              <tbody>
                <tr v-for="(k, i) in allKeys" :key="i">
                  <td><NBadge :variant="keyTypeVariant(k.key_type)" style="font-size:10px">{{ k.key_type }}</NBadge></td>
                  <td style="font-weight:500;max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ k.product }}</td>
                  <td><code style="color:var(--accent);font-size:12px;letter-spacing:.05em">{{ k.key }}</code></td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>

    <!-- Lien paramètres Windows -->
    <p class="diag-section-label">Paramètres système</p>
    <div class="card-block">
      <div style="display:flex;gap:8px;flex-wrap:wrap">
        <NButton variant="ghost" size="sm"
          @click="invoke('open_url', { url: 'ms-settings:activation' }).catch(() => {})">
          <ExternalLink :size="12" /> Paramètres d'activation Windows
        </NButton>
        <NButton variant="ghost" size="sm"
          @click="invoke('execute_tool', { command: 'slmgr /xpr', isUrl: false }).catch(() => {})">
          <Key :size="12" /> Vérifier expiration licence (slmgr /xpr)
        </NButton>
        <NButton variant="ghost" size="sm"
          @click="invoke('execute_tool', { command: 'slmgr /dlv', isUrl: false }).catch(() => {})">
          <Key :size="12" /> Infos détaillées licence (slmgr /dlv)
        </NButton>
      </div>
    </div>
  </div>
</template>
