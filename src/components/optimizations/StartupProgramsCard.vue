<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import NCard from "@/components/ui/NCard.vue";
import NButton from "@/components/ui/NButton.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NBadge from "@/components/ui/NBadge.vue";
import { useNotificationStore } from "@/stores/notifications";
import { confirm } from "@tauri-apps/plugin-dialog";
import { Rocket, RefreshCw, XCircle } from "lucide-vue-next";

const notify = useNotificationStore();

interface StartupProgram { name: string; command: string; location: string; user: string; }
const programs = ref<StartupProgram[]>([]);
const loading = ref(true);

async function load() {
  loading.value = true;
  try {
    programs.value = await invoke<StartupProgram[]>("get_startup_programs");
  } catch (e: unknown) {
    // Ne jamais afficher de données fictives comme si elles étaient réelles :
    // l'ancien fallback montrait 3 programmes inventés (Discord/OneDrive/
    // SecurityHealth) avec un bouton "Désactiver" bien réel et fonctionnel en
    // cas d'échec de détection — masquant une vraie erreur derrière un faux
    // succès plausible, avec une action destructive dessus.
    programs.value = [];
    notify.error("Détection des programmes au démarrage impossible", String(e));
  } finally { loading.value = false; }
}

async function disable(prog: StartupProgram) {
  // "Désactiver" est trompeur : le backend fait Remove-ItemProperty (voir
  // maintenance/cleanup.rs) — la valeur de registre est réellement supprimée,
  // pas juste basculée. Même sévérité que les autres suppressions du repo
  // qui demandent déjà confirmation (deleteTask, deleteDll...).
  if (!(await confirm(`Retirer "${prog.name}" du démarrage ?\n\nL'entrée de registre sera supprimée.`, { title: "Nitrite", kind: "warning" }))) return;
  try {
    await invoke("disable_startup_program", { name: prog.name, location: prog.location });
    notify.success(`${prog.name} désactivé du démarrage`);
    await load();
  } catch (e: unknown) { notify.error(`Impossible de désactiver ${prog.name}`, (e instanceof Error ? e.message : String(e)).slice(0, 120)); }
}

onMounted(load);
</script>

<template>
  <NCard>
    <template #header>
      <div style="display:flex;align-items:center;gap:8px">
        <div style="width:26px;height:26px;border-radius:6px;background:linear-gradient(135deg,#f97316,#c2410c);display:flex;align-items:center;justify-content:center">
          <Rocket :size="14" style="color:#fff" />
        </div>
        <span>Programmes au démarrage</span>
        <NButton variant="secondary" size="sm" :loading="loading" @click="load" style="margin-left:auto">
          <RefreshCw :size="14" />
        </NButton>
      </div>
    </template>

    <div v-if="loading" style="display:flex;align-items:center;gap:8px;padding:12px 0">
      <NSpinner :size="20" /><span style="font-size:13px;color:var(--text-muted)">Chargement...</span>
    </div>
    <div v-else-if="!programs.length" style="font-size:13px;color:var(--text-muted);padding:8px 0">
      Aucun programme au démarrage détecté.
    </div>
    <div v-else style="overflow-x:auto">
      <table style="width:100%;border-collapse:collapse;font-size:12px">
        <thead>
          <tr>
            <th style="text-align:left;padding:6px 10px;color:var(--text-muted);font-weight:600;border-bottom:1px solid var(--border)">Nom</th>
            <th style="text-align:left;padding:6px 10px;color:var(--text-muted);font-weight:600;border-bottom:1px solid var(--border)">Commande</th>
            <th style="text-align:left;padding:6px 10px;color:var(--text-muted);font-weight:600;border-bottom:1px solid var(--border)">Emplacement</th>
            <th style="text-align:left;padding:6px 10px;color:var(--text-muted);font-weight:600;border-bottom:1px solid var(--border)">Utilisateur</th>
            <th style="text-align:left;padding:6px 10px;color:var(--text-muted);font-weight:600;border-bottom:1px solid var(--border)">Action</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="prog in programs" :key="prog.name" style="border-bottom:1px solid var(--border)">
            <td style="padding:7px 10px;font-weight:600;color:var(--text-primary)">{{ prog.name }}</td>
            <td style="padding:7px 10px;color:var(--text-muted);font-family:monospace;font-size:11px;max-width:280px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ prog.command }}</td>
            <td style="padding:7px 10px;color:var(--text-muted);font-family:monospace;font-size:11px">{{ prog.location }}</td>
            <td style="padding:7px 10px">
              <NBadge :variant="prog.user === 'Systeme' ? 'warning' : 'accent'">{{ prog.user }}</NBadge>
            </td>
            <td style="padding:7px 10px">
              <NButton variant="danger" size="sm" :disabled="prog.user === 'Systeme'" @click="disable(prog)">
                <XCircle :size="12" /> Désactiver
              </NButton>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </NCard>
</template>
