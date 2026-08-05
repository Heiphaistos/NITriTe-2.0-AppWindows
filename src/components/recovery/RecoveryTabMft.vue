<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke, isTauriContext } from "@/utils/invoke";
import { Search, Database } from "lucide-vue-next";
import NSpinner from "@/components/ui/NSpinner.vue";
import NButton from "@/components/ui/NButton.vue";
import { useNotificationStore } from "@/stores/notifications";

const notify = useNotificationStore();

interface RecoveredFile { name: string; path: string; size_bytes: number; deleted_date: string; source: string; is_dir: boolean; }

const mftFiles = ref<RecoveredFile[]>([]);
const loadingMft = ref(false);
const mftDrive = ref("C");
const ntfsDrives = ref<string[]>([]);
const hasScanned = ref(false);

async function loadDrives() {
  try { ntfsDrives.value = await invoke<string[]>("get_ntfs_drives"); }
  catch (e: unknown) {
    if (isTauriContext()) notify.warning("Lecteurs NTFS", (e instanceof Error ? e.message : String(e)).slice(0, 120));
  }
}

async function scanMft() {
  loadingMft.value = true; mftFiles.value = [];
  try {
    mftFiles.value = await invoke<RecoveredFile[]>("scan_deleted_files", { drive: mftDrive.value });
  } catch (e: unknown) { notify.error("Erreur scan MFT", (e instanceof Error ? e.message : String(e)).slice(0, 120)); }
  loadingMft.value = false; hasScanned.value = true;
}

async function scanAllMft() {
  loadingMft.value = true; mftFiles.value = [];
  try {
    mftFiles.value = await invoke<RecoveredFile[]>("scan_all_deleted_files");
  } catch (e: unknown) { notify.error("Erreur scan global", (e instanceof Error ? e.message : String(e)).slice(0, 120)); }
  loadingMft.value = false; hasScanned.value = true;
}

function formatDate(raw: string) {
  if (!raw) return "—";
  try { return new Date(raw).toLocaleString("fr-FR"); } catch { return raw; }
}

onMounted(loadDrives);
</script>

<template>
  <div>
    <div class="info-banner">
      <Search :size="14" />
      <div>
        <strong>Journal NTFS (USN)</strong> — Le système de fichiers NTFS enregistre toutes les opérations récentes dans un journal interne.
        Cet outil scanne ce journal pour retrouver les noms des fichiers récemment supprimés.
        <br><em style="font-size:11px;opacity:.8">⚠ Le journal ne contient que les métadonnées (nom, date). Pour récupérer le contenu, utilisez Recuva ou TestDisk.</em>
      </div>
    </div>

    <div class="toolbar">
      <label class="drive-label">Lecteur :</label>
      <select v-model="mftDrive" class="drive-select-sm">
        <option v-for="d in (ntfsDrives.length ? ntfsDrives.map(x => x.replace(':','')) : ['C','D','E','F','G'])" :key="d" :value="d">{{ d }}:</option>
      </select>
      <NButton variant="primary" size="sm" :loading="loadingMft" @click="scanMft">
        <Search :size="13" /> Scanner
      </NButton>
      <NButton variant="ghost" size="sm" :loading="loadingMft" @click="scanAllMft">
        <Database :size="13" /> Tous les lecteurs NTFS
      </NButton>
      <span class="count">{{ mftFiles.length }} entrée(s)</span>
    </div>

    <div v-if="loadingMft" class="loading-state"><NSpinner :size="20" /><span>Lecture du journal NTFS...</span></div>
    <div v-else-if="mftFiles.length === 0" class="empty">
      <Database :size="28" />
      <!-- Sans hasScanned, ce même message "Lancez le scan" restait affiché même
           après un scan réellement effectué n'ayant rien trouvé (journal USN sans
           suppression récente, ou désactivé) — laissant croire à l'utilisateur
           qu'il n'a encore rien lancé alors qu'un scan complet vient d'avoir lieu. -->
      <p>{{ hasScanned ? 'Aucune suppression récente trouvée dans le journal NTFS.' : 'Lancez le scan pour voir les suppressions récentes' }}</p>
    </div>
    <div v-else class="files-table files-table-mft">
      <div class="file-row-mft header-row">
        <span>Nom du fichier</span><span>Date suppression</span><span>Source</span>
      </div>
      <div v-for="f in mftFiles" :key="f.path" class="file-row-mft">
        <span class="file-name">{{ f.name }}</span>
        <span class="file-date">{{ formatDate(f.deleted_date) }}</span>
        <span class="file-source badge-muted">Journal MFT</span>
      </div>
    </div>
    <p class="hint-note">💡 Pour récupérer ces fichiers, utilisez Recuva ou TestDisk (disponibles dans l'installeur).</p>
  </div>
</template>
