<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import type { AppConfig } from "@/types/diagnostic";
import { save, open, confirm } from "@tauri-apps/plugin-dialog";
import { writeTextFile, readTextFile } from "@tauri-apps/plugin-fs";
import { useNotificationStore } from "@/stores/notifications";
import { useAppStore } from "@/stores/app";
import { useAiStore } from "@/stores/ai";
import NButton from "@/components/ui/NButton.vue";
import NInput from "@/components/ui/NInput.vue";
import NModal from "@/components/ui/NModal.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NBadge from "@/components/ui/NBadge.vue";
import {
  User, Plus, Trash2, Download, Upload, Play,
  Save, Settings, Clock, CheckCircle,
} from "lucide-vue-next";

const notify  = useNotificationStore();
const appStore = useAppStore();
const aiStore  = useAiStore();

interface Profile {
  name:        string;
  description: string;
  created_at:  string;
  version:     string;
  config:      Record<string, any>;
}

const profiles    = ref<Profile[]>([]);
const loading     = ref(false);
const applying    = ref<string | null>(null);
const deleting    = ref<string | null>(null);

// Modal création
const showCreate  = ref(false);
const newName     = ref("");
const newDesc     = ref("");
const creating    = ref(false);

// Modal détails
const showDetail  = ref(false);
const detailProfile = ref<Profile | null>(null);

async function loadProfiles() {
  loading.value = true;
  try { profiles.value = await invoke<Profile[]>("list_profiles"); }
  catch (e: unknown) { notify.error("Profils", String(e)); }
  loading.value = false;
}

async function createProfile() {
  const trimmedName = newName.value.trim();
  if (!trimmedName) return;
  // save_profile_cmd écrase silencieusement le fichier existant (profile_filename()
  // est déterministe sur le nom, cf. utils/profiles.rs) — sans ce check, créer un
  // profil avec un nom déjà utilisé efface l'ancien sans aucun avertissement.
  if (profiles.value.some(p => p.name === trimmedName)) {
    if (!(await confirm(`Un profil "${trimmedName}" existe déjà.\n\nLe remplacer ?`, { title: "Nitrite", kind: "warning" }))) return;
  }
  creating.value = true;
  try {
    const cfg = await invoke<AppConfig>("get_config");
    const profile: Profile = {
      name:       trimmedName,
      description: newDesc.value.trim(),
      created_at: new Date().toISOString(),
      version:    __APP_VERSION__,
      config: {
        ...cfg,
        ollama_url:         aiStore.ollamaUrl,
        ollama_model:       aiStore.ollamaModel,
        ollama_temperature: aiStore.temperature,
        theme:              appStore.theme,
        font_size:          appStore.fontSize,
        show_animations:    appStore.showAnimations,
      },
    };
    await invoke("save_profile_cmd", { profile });
    notify.success("Profil créé", `"${profile.name}" sauvegardé`);
    showCreate.value = false;
    newName.value = "";
    newDesc.value = "";
    await loadProfiles();
  } catch (e: unknown) { notify.error("Erreur", String(e)); }
  creating.value = false;
}

async function applyProfile(profile: Profile) {
  applying.value = profile.name;
  try {
    const cfg = profile.config;
    // Appliquer la config Rust
    await invoke("save_config", { config: cfg });
    // Appliquer le thème + UI
    if (cfg.theme)           appStore.setTheme(cfg.theme);
    if (cfg.font_size)       appStore.setFontSize(cfg.font_size);
    if (cfg.show_animations === false) {
      appStore.showAnimations = false;
      document.documentElement.classList.add("no-animations");
    } else {
      appStore.showAnimations = true;
      document.documentElement.classList.remove("no-animations");
    }
    // Appliquer config IA
    if (cfg.ollama_url)         aiStore.ollamaUrl   = cfg.ollama_url;
    if (cfg.ollama_model)       aiStore.ollamaModel = cfg.ollama_model;
    if (cfg.ollama_temperature) aiStore.temperature = cfg.ollama_temperature;
    notify.success("Profil appliqué", `Configuration "${profile.name}" chargée`);
  } catch (e: unknown) { notify.error("Erreur", String(e)); }
  applying.value = null;
}

async function deleteProfile(name: string) {
  // Suppression permanente sans sauvegarde ni export automatique — même
  // convention que deleteDll/deleteTask/removeStartup ailleurs dans le repo.
  if (!(await confirm(`Supprimer le profil "${name}" ?\n\nCette action est irréversible.`, { title: "Nitrite", kind: "warning" }))) return;
  deleting.value = name;
  try {
    await invoke("delete_profile_cmd", { name });
    notify.success("Supprimé", `Profil "${name}" supprimé`);
    await loadProfiles();
  } catch (e: unknown) { notify.error("Erreur", String(e)); }
  deleting.value = null;
}

async function exportProfile(name: string) {
  try {
    const json = await invoke<string | null>("export_profile_json", { name });
    if (!json) { notify.error("Export", "Profil introuvable"); return; }
    const path = await save({
      defaultPath: `${name.replace(/[^a-z0-9_-]/gi, "_")}_profile.json`,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (path) {
      await writeTextFile(path, json);
      notify.success("Exporté", `Profil sauvegardé dans ${path}`);
    }
  } catch (e: unknown) { notify.error("Export", String(e)); }
}

async function importProfile() {
  try {
    const path = await open({
      multiple: false,
      filters: [{ name: "Profil JSON", extensions: ["json"] }],
    });
    if (!path || Array.isArray(path)) return;
    const json = await readTextFile(path as string);
    const profile = await invoke<Profile>("import_profile_json", { json });
    if (profiles.value.some(p => p.name === profile.name)) {
      if (!(await confirm(`Un profil "${profile.name}" existe déjà.\n\nLe remplacer ?`, { title: "Nitrite", kind: "warning" }))) return;
    }
    await invoke("save_profile_cmd", { profile });
    notify.success("Importé", `Profil "${profile.name}" importé`);
    await loadProfiles();
  } catch (e: unknown) { notify.error("Import", String(e)); }
}

function openDetail(profile: Profile) {
  detailProfile.value = profile;
  showDetail.value = true;
}

function formatDate(iso: string) {
  try { return new Date(iso).toLocaleString("fr-FR"); }
  catch { return iso; }
}

onMounted(loadProfiles);
</script>

<template>
  <div class="profiles-page">
    <div class="page-header">
      <div>
        <h1><User :size="24" style="margin-right:10px;vertical-align:middle;color:var(--accent-primary)" />Profils de Configuration</h1>
        <p class="page-subtitle">Sauvegardez et restaurez des configurations complètes (thème, IA, préférences)</p>
      </div>
      <div style="display:flex;gap:8px">
        <NButton variant="ghost" size="sm" @click="importProfile">
          <Upload :size="14" /> Importer
        </NButton>
        <NButton variant="primary" @click="showCreate = true">
          <Plus :size="14" /> Nouveau profil
        </NButton>
      </div>
    </div>

    <!-- État chargement -->
    <div v-if="loading" class="empty-state">
      <NSpinner :size="28" />
      <p>Chargement des profils...</p>
    </div>

    <!-- Aucun profil -->
    <div v-else-if="!profiles.length" class="empty-state">
      <User :size="48" class="empty-icon" />
      <h3>Aucun profil sauvegardé</h3>
      <p>Créez votre premier profil pour sauvegarder votre configuration actuelle.</p>
      <NButton variant="primary" style="margin-top:12px" @click="showCreate = true">
        <Plus :size="14" /> Créer un profil
      </NButton>
    </div>

    <!-- Liste profils -->
    <div v-else class="profiles-grid">
      <div v-for="p in profiles" :key="p.name" class="profile-card">
        <div class="profile-header">
          <div class="profile-icon">
            <User :size="20" />
          </div>
          <div class="profile-meta">
            <div class="profile-name">{{ p.name }}</div>
            <div class="profile-desc" v-if="p.description">{{ p.description }}</div>
            <div class="profile-date">
              <Clock :size="11" />
              {{ formatDate(p.created_at) }}
            </div>
          </div>
          <NBadge variant="neutral" style="font-size:10px;margin-left:auto">v{{ p.version }}</NBadge>
        </div>

        <!-- Aperçu config -->
        <div class="profile-preview">
          <span v-if="p.config?.theme" class="preview-pill">🎨 {{ p.config.theme }}</span>
          <span v-if="p.config?.ollama_model" class="preview-pill">🤖 {{ p.config.ollama_model }}</span>
          <span v-if="p.config?.font_size" class="preview-pill">🔤 {{ p.config.font_size }}</span>
        </div>

        <div class="profile-actions">
          <NButton variant="primary" size="sm" :disabled="applying === p.name" @click="applyProfile(p)">
            <NSpinner v-if="applying === p.name" :size="12" />
            <Play v-else :size="12" />
            Appliquer
          </NButton>
          <NButton variant="ghost" size="sm" @click="openDetail(p)">
            <Settings :size="12" /> Détails
          </NButton>
          <NButton variant="ghost" size="sm" @click="exportProfile(p.name)">
            <Download :size="12" />
          </NButton>
          <NButton variant="danger" size="sm" :disabled="deleting === p.name" @click="deleteProfile(p.name)">
            <NSpinner v-if="deleting === p.name" :size="12" />
            <Trash2 v-else :size="12" />
          </NButton>
        </div>
      </div>
    </div>

    <!-- Modal création -->
    <NModal :open="showCreate" @close="showCreate = false" title="Nouveau profil">
      <div style="display:flex;flex-direction:column;gap:14px;padding:4px 0">
        <div>
          <label class="field-label">Nom du profil *</label>
          <NInput v-model="newName" placeholder="Ex: Bureau Pro, Config Dev, Gaming..." />
        </div>
        <div>
          <label class="field-label">Description (optionnel)</label>
          <NInput v-model="newDesc" placeholder="Décrivez ce profil..." />
        </div>
        <div class="snapshot-info">
          <CheckCircle :size="14" style="color:var(--success);flex-shrink:0" />
          <span>Capture : thème <strong>{{ appStore.theme }}</strong>, modèle IA <strong>{{ aiStore.ollamaModel }}</strong>, toutes les préférences courantes.</span>
        </div>
      </div>
      <template #footer>
        <NButton variant="ghost" @click="showCreate = false">Annuler</NButton>
        <NButton variant="primary" :disabled="!newName.trim() || creating" @click="createProfile">
          <NSpinner v-if="creating" :size="12" />
          <Save v-else :size="14" />
          Créer le profil
        </NButton>
      </template>
    </NModal>

    <!-- Modal détails -->
    <NModal :open="showDetail && !!detailProfile" @close="showDetail = false" :title="detailProfile ? `Profil : ${detailProfile.name}` : ''">
      <div v-if="detailProfile" style="font-size:12px;display:flex;flex-direction:column;gap:6px">
        <div class="detail-row"><span>Créé le</span><span>{{ formatDate(detailProfile.created_at) }}</span></div>
        <div class="detail-row"><span>Version</span><span>{{ detailProfile.version }}</span></div>
        <div class="detail-row"><span>Description</span><span>{{ detailProfile.description || '—' }}</span></div>
        <hr style="border-color:var(--border);margin:6px 0">
        <div v-for="(v, k) in detailProfile.config" :key="String(k)" class="detail-row">
          <span class="muted">{{ k }}</span>
          <span>{{ typeof v === 'boolean' ? (v ? 'oui' : 'non') : v }}</span>
        </div>
      </div>
      <template #footer>
        <NButton variant="ghost" @click="showDetail = false">Fermer</NButton>
        <NButton variant="primary" @click="applyProfile(detailProfile!); showDetail = false">
          <Play :size="13" /> Appliquer
        </NButton>
      </template>
    </NModal>
  </div>
</template>

<style scoped>
.profiles-page { display:flex;flex-direction:column;gap:20px; }
.page-header { display:flex;justify-content:space-between;align-items:flex-start;flex-wrap:wrap;gap:12px; }
.page-header h1 { font-size:22px;font-weight:700; }
.page-subtitle { color:var(--text-muted);font-size:13px;margin-top:3px; }
.empty-state { display:flex;flex-direction:column;align-items:center;gap:10px;padding:60px 20px;color:var(--text-secondary);text-align:center; }
.empty-icon { opacity:.3; }
.empty-state h3 { font-size:16px;font-weight:600;color:var(--text-primary); }
.empty-state p { font-size:13px;max-width:360px; }
.profiles-grid { display:grid;grid-template-columns:repeat(auto-fill,minmax(300px,1fr));gap:16px; }
.profile-card {
  background:var(--bg-secondary);border:1px solid var(--border);border-radius:10px;
  padding:16px;display:flex;flex-direction:column;gap:12px;
  transition:border-color .2s,box-shadow .2s;
}
.profile-card:hover { border-color:var(--accent-primary);box-shadow:0 0 0 1px var(--accent-primary-muted,rgba(249,115,22,.12)); }
.profile-header { display:flex;align-items:flex-start;gap:12px; }
.profile-icon { width:38px;height:38px;border-radius:8px;background:var(--accent-primary-muted,rgba(249,115,22,.15));
  display:flex;align-items:center;justify-content:center;color:var(--accent-primary);flex-shrink:0; }
.profile-meta { flex:1;min-width:0; }
.profile-name { font-weight:600;font-size:14px;color:var(--text-primary);white-space:nowrap;overflow:hidden;text-overflow:ellipsis; }
.profile-desc { font-size:11px;color:var(--text-muted);margin-top:2px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis; }
.profile-date { display:flex;align-items:center;gap:4px;font-size:10px;color:var(--text-muted);margin-top:4px; }
.profile-preview { display:flex;flex-wrap:wrap;gap:6px; }
.preview-pill { font-size:10px;padding:2px 8px;background:var(--bg-tertiary);border:1px solid var(--border);border-radius:12px;color:var(--text-secondary); }
.profile-actions { display:flex;gap:6px;flex-wrap:wrap;border-top:1px solid var(--border);padding-top:10px; }
.field-label { font-size:12px;color:var(--text-muted);margin-bottom:6px;display:block; }
.snapshot-info { display:flex;align-items:flex-start;gap:8px;font-size:12px;color:var(--text-secondary);
  padding:10px 12px;background:var(--bg-secondary);border-radius:6px;border:1px solid var(--border);line-height:1.5; }
.detail-row { display:flex;justify-content:space-between;gap:16px;padding:3px 0;border-bottom:1px solid var(--border); }
.detail-row span:first-child { color:var(--text-muted);white-space:nowrap; }
.detail-row span:last-child { color:var(--text-primary);text-align:right;word-break:break-all; }
</style>
