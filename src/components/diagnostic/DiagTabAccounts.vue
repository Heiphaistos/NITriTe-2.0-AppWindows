<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import { invoke } from "@/utils/invoke";
import { cachedInvoke } from "@/composables/useCachedInvoke";
import NBadge from "@/components/ui/NBadge.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NButton from "@/components/ui/NButton.vue";
import DiagBanner from "@/components/ui/DiagBanner.vue";
import { Users, Shield, Lock, Key, Settings, AlertTriangle, CheckCircle, ExternalLink } from "lucide-vue-next";
import { useExportData } from "@/composables/useExportData";

const togglingUser = ref<string | null>(null);
const toggleMsg = ref("");

async function toggleUser(name: string, enable: boolean) {
  // Désactiver un compte n'avait aucune confirmation (activer reste sans risque).
  // Le bouton "Désactiver" est déjà masqué pour les comptes admin (voir template,
  // corrigé pour utiliser le vrai champ is_admin au lieu d'une recherche de
  // substring "admin" dans le nom) — cette confirmation couvre les comptes restants.
  if (!enable && !(await confirm(`Désactiver le compte "${name}" ?`, { title: "Nitrite", kind: "warning" }))) return;
  togglingUser.value = name;
  toggleMsg.value = "";
  try {
    await invoke("run_system_command", {
      cmd: "cmd",
      args: ["/c", "start", "cmd", "/k",
        enable
          ? `net user "${name}" /active:yes && echo Compte activé && pause`
          : `net user "${name}" /active:no && echo Compte désactivé && pause`
      ]
    });
    toggleMsg.value = enable ? `${name} : activation demandée` : `${name} : désactivation demandée`;
    setTimeout(() => { toggleMsg.value = ""; }, 4000);
  } catch (e: unknown) {
    toggleMsg.value = "Erreur : " + String(e);
  }
  togglingUser.value = null;
}

async function openTool(tool: string) {
  await invoke("run_system_command", { cmd: "cmd", args: ["/c", "start", tool] }).catch(() => {});
}

async function openSettings(uri: string) {
  await invoke("open_url", { url: uri }).catch(() => {});
}

function doExportUsers() {
  if (!data.value) return;
  exportCSV(data.value.users.map(u => ({
    Nom: u.name, NomComplet: u.full_name, Type: u.account_type,
    Actif: u.enabled ? 'Oui' : 'Non', Admin: u.is_admin ? 'Oui' : 'Non',
    DerniereConnexion: u.last_logon, Description: u.description,
  })), 'comptes-' + new Date().toISOString().slice(0,10));
}

interface UserAccount {
  name: string; full_name: string; description: string;
  enabled: boolean; last_logon: string;
  password_required: boolean; is_admin: boolean; account_type: string;
}
interface LocalGroup {
  name: string; description: string; member_count: number; members: string[];
}
interface PasswordPolicy {
  min_length: number; max_age_days: number; min_age_days: number;
  complexity: boolean; lockout_threshold: number; lockout_duration: number;
  history_count: number;
}
interface AccountsInfo {
  users: UserAccount[]; groups: LocalGroup[];
  policy: PasswordPolicy; total_enabled: number; total_admin: number;
}

const data = ref<AccountsInfo | null>(null);
const loading = ref(true);
const error = ref("");
const { exportCSV } = useExportData();

onMounted(async () => {
  try {
    data.value = await cachedInvoke<AccountsInfo>("get_user_accounts");
  } catch (e: unknown) { error.value = (e instanceof Error ? e.message : String(e)) || "Erreur"; }
  finally { loading.value = false; }
});
</script>

<template>
  <div class="diag-tab-content">
    <DiagBanner :icon="Users" title="Comptes Utilisateurs" desc="Comptes locaux, groupes et politique de mots de passe" color="purple" />

    <div v-if="loading" class="diag-loading">
      <NSpinner :size="16" class="diag-spinner" /><span>Chargement comptes...</span>
    </div>
    <div v-else-if="error" style="color:var(--error)">⚠ {{ error }}</div>
    <div v-else-if="data" style="display:flex;flex-direction:column;gap:14px">

      <!-- Outils rapides -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0"><Settings :size="13" style="display:inline;margin-right:4px" />Outils de gestion</p>
        <div style="display:flex;gap:8px;flex-wrap:wrap;margin-bottom:10px">
          <NButton variant="ghost" size="sm" @click="openTool('lusrmgr.msc')">
            <Users :size="12" /> Gestion utilisateurs locaux
          </NButton>
          <NButton variant="ghost" size="sm" @click="openTool('netplwiz')">
            <Key :size="12" /> Comptes utilisateurs (netplwiz)
          </NButton>
          <NButton variant="ghost" size="sm" @click="openSettings('ms-settings:otherusers')">
            <ExternalLink :size="12" /> Autres utilisateurs (Paramètres)
          </NButton>
          <NButton variant="ghost" size="sm" @click="openSettings('ms-settings:signin')">
            <Lock :size="12" /> Options de connexion
          </NButton>
          <NButton variant="ghost" size="sm" @click="doExportUsers" :disabled="!data">
            ↓ Exporter comptes CSV
          </NButton>
        </div>
        <div v-if="toggleMsg" style="font-size:12px;color:var(--accent);padding:4px 0">{{ toggleMsg }}</div>
      </div>

      <!-- Résumé -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Résumé</p>
        <div class="info-grid">
          <div class="info-row"><span>Comptes actifs</span>
            <NBadge variant="info">{{ data.total_enabled }} / {{ data.users.length }}</NBadge>
          </div>
          <div class="info-row"><span>Administrateurs</span>
            <NBadge :variant="data.total_admin > 2 ? 'warning' : 'success'">{{ data.total_admin }}</NBadge>
          </div>
        </div>
      </div>

      <!-- Comptes utilisateurs -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">
          <Users :size="13" style="display:inline;margin-right:4px" />Comptes locaux ({{ data.users.length }})
        </p>
        <div style="overflow-x:auto">
          <table style="width:100%;border-collapse:collapse;font-size:12px">
            <thead>
              <tr style="background:var(--bg-secondary)">
                <th style="padding:6px 10px;text-align:left;color:var(--text-secondary);font-weight:500">Compte</th>
                <th style="padding:6px 10px;text-align:left;color:var(--text-secondary);font-weight:500">Nom complet</th>
                <th style="padding:6px 10px;text-align:left;color:var(--text-secondary);font-weight:500">Type</th>
                <th style="padding:6px 10px;text-align:left;color:var(--text-secondary);font-weight:500">Statut</th>
                <th style="padding:6px 10px;text-align:left;color:var(--text-secondary);font-weight:500">Admin</th>
                <th style="padding:6px 10px;text-align:left;color:var(--text-secondary);font-weight:500">Dernière connexion</th>
                <th style="padding:6px 10px;text-align:left;color:var(--text-secondary);font-weight:500">Action</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="u in data.users" :key="u.name"
                style="border-bottom:1px solid var(--border)"
                :style="{ opacity: u.enabled ? 1 : 0.5 }">
                <td style="padding:6px 10px">
                  <div style="display:flex;align-items:center;gap:6px">
                    <component :is="u.is_admin ? Shield : Users" :size="12"
                      :style="{ color: u.is_admin ? 'var(--accent)' : 'var(--text-muted)' }" />
                    <code>{{ u.name }}</code>
                  </div>
                </td>
                <td style="padding:6px 10px;color:var(--text-secondary)">{{ u.full_name || "—" }}</td>
                <td style="padding:6px 10px">
                  <NBadge :variant="u.account_type === 'Microsoft' ? 'info' : u.account_type === 'Azure AD' ? 'success' : 'neutral'">
                    {{ u.account_type }}
                  </NBadge>
                </td>
                <td style="padding:6px 10px">
                  <NBadge :variant="u.enabled ? 'success' : 'neutral'">{{ u.enabled ? 'Actif' : 'Désactivé' }}</NBadge>
                </td>
                <td style="padding:6px 10px">
                  <NBadge v-if="u.is_admin" variant="warning"><Shield :size="10" /> Admin</NBadge>
                  <span v-else class="muted">—</span>
                </td>
                <td style="padding:6px 10px;color:var(--text-secondary);font-size:11px">{{ u.last_logon }}</td>
                <td style="padding:4px 6px">
                  <button v-if="u.enabled && !u.is_admin"
                    @click="toggleUser(u.name, false)"
                    :disabled="togglingUser === u.name"
                    style="font-size:10px;padding:2px 7px;border-radius:4px;border:1px solid rgba(239,68,68,.4);background:rgba(239,68,68,.08);color:#ef4444;cursor:pointer">
                    Désactiver
                  </button>
                  <button v-else-if="!u.enabled"
                    @click="toggleUser(u.name, true)"
                    :disabled="togglingUser === u.name"
                    style="font-size:10px;padding:2px 7px;border-radius:4px;border:1px solid rgba(34,197,94,.4);background:rgba(34,197,94,.08);color:#22c55e;cursor:pointer">
                    Activer
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Politique de mots de passe -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">
          <Key :size="13" style="display:inline;margin-right:4px" />Politique de mots de passe
        </p>
        <div class="info-grid">
          <div class="info-row"><span>Longueur minimale</span>
            <NBadge :variant="data.policy.min_length >= 8 ? 'success' : data.policy.min_length > 0 ? 'warning' : 'danger'">
              {{ data.policy.min_length > 0 ? data.policy.min_length + ' caractères' : 'Non définie ⚠' }}
            </NBadge>
          </div>
          <div class="info-row"><span>Complexité requise</span>
            <NBadge :variant="data.policy.complexity ? 'success' : 'warning'">
              {{ data.policy.complexity ? 'Oui' : 'Non' }}
            </NBadge>
          </div>
          <div class="info-row"><span>Expiration max</span>
            <span>{{ data.policy.max_age_days > 0 ? data.policy.max_age_days + ' jours' : 'Jamais' }}</span>
          </div>
          <div class="info-row"><span>Âge minimum</span>
            <span>{{ data.policy.min_age_days > 0 ? data.policy.min_age_days + ' jours' : 'Aucun' }}</span>
          </div>
          <div class="info-row"><span>Verrouillage après</span>
            <NBadge :variant="data.policy.lockout_threshold > 0 ? 'success' : 'warning'">
              {{ data.policy.lockout_threshold > 0 ? data.policy.lockout_threshold + ' tentatives' : 'Désactivé ⚠' }}
            </NBadge>
          </div>
          <div class="info-row" v-if="data.policy.lockout_threshold > 0"><span>Durée verrouillage</span>
            <span>{{ data.policy.lockout_duration > 0 ? data.policy.lockout_duration + ' min' : 'Admin requis' }}</span>
          </div>
          <div class="info-row"><span>Historique mots de passe</span>
            <span>{{ data.policy.history_count > 0 ? data.policy.history_count + ' mémorisés' : 'Aucun' }}</span>
          </div>
        </div>
      </div>

      <!-- Analyse sécurité comptes -->
      <div class="diag-section" style="border-left:3px solid var(--accent)">
        <p class="diag-section-label" style="margin:0 0 8px 0"><Shield :size="13" style="display:inline;margin-right:4px" />Analyse sécurité</p>
        <div style="display:flex;flex-direction:column;gap:6px">
          <div style="display:flex;align-items:center;gap:8px;font-size:12px">
            <component :is="data.total_admin <= 2 ? CheckCircle : AlertTriangle" :size="13"
              :class="data.total_admin <= 2 ? 'ic-ok' : 'ic-warn'" />
            <span>{{ data.total_admin }} administrateur(s) —
              <span :style="{ color: data.total_admin <= 2 ? 'var(--success)' : 'var(--warning)' }">
                {{ data.total_admin <= 1 ? 'OK' : data.total_admin === 2 ? 'Acceptable' : 'Trop d\'admins — vérifier' }}
              </span>
            </span>
          </div>
          <div style="display:flex;align-items:center;gap:8px;font-size:12px">
            <component :is="data.policy.min_length >= 8 ? CheckCircle : AlertTriangle" :size="13"
              :class="data.policy.min_length >= 8 ? 'ic-ok' : 'ic-warn'" />
            <span>Mot de passe min. {{ data.policy.min_length }} car. —
              <span :style="{ color: data.policy.min_length >= 12 ? 'var(--success)' : data.policy.min_length >= 8 ? 'var(--warning)' : 'var(--error)' }">
                {{ data.policy.min_length >= 12 ? 'Fort' : data.policy.min_length >= 8 ? 'Moyen' : 'Faible' }}
              </span>
            </span>
          </div>
          <div style="display:flex;align-items:center;gap:8px;font-size:12px">
            <component :is="data.policy.complexity ? CheckCircle : AlertTriangle" :size="13"
              :class="data.policy.complexity ? 'ic-ok' : 'ic-warn'" />
            <span>Complexité des mots de passe : <strong>{{ data.policy.complexity ? 'Activée' : 'Désactivée ⚠' }}</strong></span>
          </div>
          <div style="display:flex;align-items:center;gap:8px;font-size:12px">
            <component :is="data.policy.lockout_threshold > 0 ? CheckCircle : AlertTriangle" :size="13"
              :class="data.policy.lockout_threshold > 0 ? 'ic-ok' : 'ic-warn'" />
            <span>Verrouillage compte : <strong>{{ data.policy.lockout_threshold > 0 ? 'Activé (' + data.policy.lockout_threshold + ' tentatives)' : 'Désactivé ⚠ (brute-force possible)' }}</strong></span>
          </div>
          <div v-if="data.users.some(u => u.enabled && u.name.toLowerCase() === 'guest')"
            style="display:flex;align-items:center;gap:8px;font-size:12px;color:var(--warning)">
            <AlertTriangle :size="13" /> Compte Invité (Guest) actif — recommandé de le désactiver
          </div>
        </div>
      </div>

      <!-- Groupes locaux -->
      <div class="diag-section">
        <p class="diag-section-label" style="margin:0 0 8px 0">Groupes locaux ({{ data.groups.length }})</p>
        <div v-for="g in data.groups" :key="g.name"
          style="padding:8px 0;border-bottom:1px solid var(--border)">
          <div style="display:flex;align-items:center;gap:8px;margin-bottom:4px">
            <Shield :size="12" style="color:var(--accent);flex-shrink:0" />
            <strong style="font-size:12px">{{ g.name }}</strong>
            <NBadge variant="neutral" style="font-size:10px">{{ g.member_count }} membre(s)</NBadge>
            <span class="muted" style="font-size:11px">{{ g.description }}</span>
          </div>
          <div v-if="g.members.length" style="display:flex;gap:4px;flex-wrap:wrap;padding-left:20px">
            <code v-for="(m, i) in g.members" :key="i"
              style="font-size:10px;background:var(--bg-secondary);padding:1px 5px;border-radius:3px">{{ m }}</code>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
