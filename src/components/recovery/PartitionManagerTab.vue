<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@/utils/invoke";
import NButton from "@/components/ui/NButton.vue";
import NSpinner from "@/components/ui/NSpinner.vue";
import NBadge from "@/components/ui/NBadge.vue";
import { useNotificationStore } from "@/stores/notifications";
import {
  HardDrive, RefreshCw, Thermometer, Clock, Shield,
  Zap, Trash2, Plus, Edit2, RotateCcw, AlertTriangle, CheckCircle,
  Maximize2, Download, Upload, Search, Activity,
} from "lucide-vue-next";

const notify = useNotificationStore();

interface DiskSmartInfo {
  disk_index: number; label: string; health: string;
  temperature?: number; power_on_hours?: number; serial: string;
  size_gb: number; media_type: string; wear_level?: number; reallocated_sectors?: number;
}
interface PartitionDetail {
  disk_index: number; part_index: number; disk_label: string;
  letter: string; label: string; size_gb: number; free_gb: number;
  file_system: string; part_type: string; is_system: boolean; is_boot: boolean; health: string;
}

const smart = ref<DiskSmartInfo[]>([]);
const partitions = ref<PartitionDetail[]>([]);
const loading = ref(false);
const opLoading = ref(false);

// ── Modals ─────────────────────────────────────────────────────────────────────
const modalFormat = ref(false);
const modalLetter = ref(false);
const modalCreate = ref(false);
const modalDelete = ref(false);
const modalInit   = ref(false);

const targetPart = ref<PartitionDetail | null>(null);
const fmtFs      = ref("NTFS");
const fmtLabel   = ref("");
const newLetter  = ref("");
const newSizeMb  = ref<number | undefined>(undefined);
const initStyle  = ref("GPT");
const confirmTxt = ref("");

// ── Computed ───────────────────────────────────────────────────────────────────
const diskGroups = computed(() => {
  const map = new Map<number, { label: string; parts: PartitionDetail[] }>();
  for (const p of partitions.value) {
    if (!map.has(p.disk_index)) map.set(p.disk_index, { label: p.disk_label, parts: [] });
    map.get(p.disk_index)!.parts.push(p);
  }
  return Array.from(map.entries()).map(([idx, g]) => ({ idx, label: g.label, parts: g.parts }));
});

// ── Load ───────────────────────────────────────────────────────────────────────
async function loadAll() {
  loading.value = true;
  try {
    [smart.value, partitions.value] = await Promise.all([
      invoke<DiskSmartInfo[]>("get_disks_smart"),
      invoke<PartitionDetail[]>("get_partition_list"),
    ]);
  } catch (e: any) { notify.error("Erreur chargement", String(e)); }
  loading.value = false;
}

onMounted(loadAll);

// ── Opérations ─────────────────────────────────────────────────────────────────
function openFormat(p: PartitionDetail) {
  targetPart.value = p; fmtFs.value = "NTFS"; fmtLabel.value = p.label;
  modalFormat.value = true;
}
function openLetter(p: PartitionDetail) {
  targetPart.value = p; newLetter.value = ""; modalLetter.value = true;
}
function openCreate(p: PartitionDetail) {
  targetPart.value = p; newSizeMb.value = undefined; modalCreate.value = true;
}
function openDelete(p: PartitionDetail) {
  targetPart.value = p; confirmTxt.value = ""; modalDelete.value = true;
}
function openInit(idx: number, label: string) {
  targetPart.value = { disk_index: idx, part_index: 0, disk_label: label, letter: "", label: "", size_gb: 0, free_gb: 0, file_system: "", part_type: "", is_system: false, is_boot: false, health: "" };
  initStyle.value = "GPT"; confirmTxt.value = ""; modalInit.value = true;
}

async function doFormat() {
  if (!targetPart.value) return;
  opLoading.value = true;
  try {
    await invoke("format_partition_cmd", { letter: targetPart.value.letter, fs: fmtFs.value, label: fmtLabel.value });
    notify.success("Formatage terminé", `${targetPart.value.letter} formaté en ${fmtFs.value}`);
    modalFormat.value = false; await loadAll();
  } catch (e: any) { notify.error("Erreur formatage", String(e)); }
  finally { opLoading.value = false; }
}

async function doAssignLetter() {
  if (!targetPart.value || !newLetter.value) return;
  opLoading.value = true;
  try {
    await invoke("assign_drive_letter_cmd", { diskIndex: targetPart.value.disk_index, partIndex: targetPart.value.part_index, letter: newLetter.value });
    notify.success("Lettre assignée", `${newLetter.value.toUpperCase()}: attribuée`);
    modalLetter.value = false; await loadAll();
  } catch (e: any) { notify.error("Erreur", String(e)); }
  finally { opLoading.value = false; }
}

async function doCreate() {
  if (!targetPart.value) return;
  opLoading.value = true;
  try {
    await invoke("create_partition_cmd", { diskIndex: targetPart.value.disk_index, sizeMb: newSizeMb.value ?? null });
    notify.success("Partition créée", "NTFS — lettre assignée automatiquement");
    modalCreate.value = false; await loadAll();
  } catch (e: any) { notify.error("Erreur création", String(e)); }
  finally { opLoading.value = false; }
}

async function doDelete() {
  if (!targetPart.value || confirmTxt.value !== "SUPPRIMER") return;
  opLoading.value = true;
  try {
    await invoke("delete_partition_cmd", { diskIndex: targetPart.value.disk_index, partIndex: targetPart.value.part_index });
    notify.success("Partition supprimée", `Disque ${targetPart.value.disk_index} — partition ${targetPart.value.part_index}`);
    modalDelete.value = false; await loadAll();
  } catch (e: any) { notify.error("Erreur suppression", String(e)); }
  finally { opLoading.value = false; }
}

async function doInit() {
  if (!targetPart.value || confirmTxt.value !== "INITIALISER") return;
  opLoading.value = true;
  try {
    await invoke("initialize_disk_cmd", { diskIndex: targetPart.value.disk_index, style: initStyle.value });
    notify.success("Disque initialisé", `Disque ${targetPart.value.disk_index} — ${initStyle.value}`);
    modalInit.value = false; await loadAll();
  } catch (e: any) { notify.error("Erreur init", String(e)); }
  finally { opLoading.value = false; }
}

// ── Resize partition ───────────────────────────────────────────────────────────
const modalResize = ref(false);
interface SizeLimits { min_bytes: number; max_bytes: number; current_bytes: number; }
const resizeLimits = ref<SizeLimits | null>(null);
const resizeSizeMb = ref(0);

async function openResize(p: PartitionDetail) {
  targetPart.value = p; resizeLimits.value = null; opLoading.value = true;
  try {
    resizeLimits.value = await invoke<SizeLimits>("get_partition_resize_limits_cmd", {
      diskIndex: p.disk_index, partIndex: p.part_index,
    });
    resizeSizeMb.value = Math.round((resizeLimits.value.current_bytes) / (1024 * 1024));
    modalResize.value = true;
  } catch (e: any) { notify.error("Limites de resize", String(e)); }
  opLoading.value = false;
}
async function doResize() {
  if (!targetPart.value || resizeSizeMb.value <= 0) return;
  opLoading.value = true;
  try {
    await invoke("resize_partition_cmd", {
      diskIndex: targetPart.value.disk_index,
      partIndex: targetPart.value.part_index,
      newSizeMb: resizeSizeMb.value,
    });
    notify.success("Partition redimensionnée", `${targetPart.value.letter} → ${resizeSizeMb.value} Mo`);
    modalResize.value = false; await loadAll();
  } catch (e: any) { notify.error("Erreur resize", String(e)); }
  finally { opLoading.value = false; }
}

// ── MBR Backup / Restore ────────────────────────────────────────────────────────
const modalMbr = ref(false);
const mbrDiskIndex = ref(0);
const mbrOutputPath = ref("C:\\NiTriTe\\MBR_backup_disk0.bin");
const mbrRestorePath = ref("");
const mbrMode = ref<"backup" | "restore">("backup");

function openMbr(diskIdx: number, mode: "backup" | "restore") {
  mbrDiskIndex.value = diskIdx;
  mbrMode.value = mode;
  mbrOutputPath.value = `C:\\NiTriTe\\MBR_backup_disk${diskIdx}.bin`;
  mbrRestorePath.value = "";
  modalMbr.value = true;
}
async function doMbrOp() {
  opLoading.value = true;
  try {
    if (mbrMode.value === "backup") {
      const res = await invoke<string>("backup_mbr_cmd", { diskIndex: mbrDiskIndex.value, outputPath: mbrOutputPath.value });
      notify.success("MBR sauvegardé", res);
    } else {
      const res = await invoke<string>("restore_mbr_cmd", { diskIndex: mbrDiskIndex.value, mbrPath: mbrRestorePath.value });
      notify.success("MBR restauré", res);
    }
    modalMbr.value = false;
  } catch (e: any) { notify.error("Erreur MBR", String(e)); }
  finally { opLoading.value = false; }
}

// ── Scan partitions perdues ─────────────────────────────────────────────────────
interface LostPartition { disk_index: number; offset_bytes: number; size_bytes: number; signature: string; fs_hint: string; description: string; }
const lostPartitions = ref<LostPartition[]>([]);
const scanningLost = ref(false);
const scannedDisk = ref<number | null>(null);

async function doScanLost(diskIdx: number) {
  scanningLost.value = true; scannedDisk.value = diskIdx;
  try {
    lostPartitions.value = await invoke<LostPartition[]>("scan_lost_partitions_cmd", { diskIndex: diskIdx });
    if (lostPartitions.value.length === 0) notify.info("Aucune partition perdue", "Aucune région récupérable détectée sur ce disque.");
    else notify.warning("Partitions perdues détectées", `${lostPartitions.value.length} région(s) trouvée(s).`);
  } catch (e: any) { notify.error("Erreur scan", String(e)); }
  finally { scanningLost.value = false; }
}

const fmtBytes = (b: number) => b >= 1_073_741_824 ? `${(b / 1_073_741_824).toFixed(1)} GB` : b >= 1_048_576 ? `${(b / 1_048_576).toFixed(1)} MB` : `${(b / 1024).toFixed(0)} KB`;

// ── Helpers ────────────────────────────────────────────────────────────────────
function formatGb(n: number) { return n >= 1000 ? `${(n / 1000).toFixed(1)} To` : `${n.toFixed(1)} Go`; }
function usedPct(p: PartitionDetail) { return p.size_gb > 0 ? Math.round(((p.size_gb - p.free_gb) / p.size_gb) * 100) : 0; }

function healthVariant(h: string): "success" | "warning" | "danger" | "default" {
  if (!h || h === "Unknown") return "default";
  h = h.toLowerCase();
  if (h.includes("healthy") || h === "ok") return "success";
  if (h.includes("warning")) return "warning";
  if (h.includes("unhealthy") || h.includes("error")) return "danger";
  return "default";
}
function smartColor(h: string) {
  h = h?.toLowerCase() ?? "";
  if (h.includes("healthy")) return "var(--success)";
  if (h.includes("warning")) return "var(--warning)";
  if (h.includes("unhealthy")) return "var(--danger)";
  return "var(--text-muted)";
}
function typeLabel(t: string) {
  const map: Record<string, string> = { Basic: "Base", System: "Système", Recovery: "Récup.", IFS: "IFS", Unknown: "?", Unallocated: "Non alloué", GPT: "GPT" };
  return map[t] ?? t;
}
function typeColor(t: string) {
  if (t === "System") return "var(--danger)";
  if (t === "Recovery") return "var(--warning)";
  if (t === "Unallocated") return "var(--text-muted)";
  return "var(--accent-primary)";
}
</script>

<template>
  <div class="partition-mgr">

    <!-- Bannière admin -->
    <div class="admin-banner">
      <Shield :size="13" />
      <span><strong>Droits administrateur requis</strong> pour les opérations de partition (format, suppression, création). Lancez Nitrite en tant qu'administrateur si besoin.</span>
    </div>

    <!-- Toolbar -->
    <div class="toolbar">
      <NButton variant="ghost" size="sm" :loading="loading" @click="loadAll">
        <RefreshCw :size="13" /> Actualiser
      </NButton>
    </div>

    <div v-if="loading" class="loading-state"><NSpinner :size="20" /><span>Lecture des disques...</span></div>
    <template v-else>

      <!-- ══ SMART HEALTH ══ -->
      <div class="section-label">Santé des disques (SMART)</div>
      <div v-if="smart.length === 0" class="empty-small">Aucune donnée SMART disponible</div>
      <div v-else class="smart-grid">
        <div v-for="d in smart" :key="d.disk_index" class="smart-card">
          <div class="smart-header">
            <div class="smart-icon" :style="{ background: `color-mix(in srgb, ${smartColor(d.health)} 15%, transparent)`, color: smartColor(d.health) }">
              <HardDrive :size="18" />
            </div>
            <div class="smart-info">
              <span class="smart-name">{{ d.label || `Disque ${d.disk_index}` }}</span>
              <span class="smart-serial">{{ d.serial || '—' }}</span>
            </div>
            <NBadge :variant="healthVariant(d.health)" style="flex-shrink:0">{{ d.health || 'Unknown' }}</NBadge>
          </div>
          <div class="smart-stats">
            <div class="smart-stat">
              <span class="stat-label"><HardDrive :size="11" /> Type</span>
              <span class="stat-val">{{ d.media_type || 'HDD' }}</span>
            </div>
            <div class="smart-stat">
              <span class="stat-label"><Zap :size="11" /> Capacité</span>
              <span class="stat-val">{{ d.size_gb }} Go</span>
            </div>
            <div v-if="d.temperature !== undefined" class="smart-stat" :class="{ warn: d.temperature > 55 }">
              <span class="stat-label"><Thermometer :size="11" /> Temp.</span>
              <span class="stat-val">{{ d.temperature }}°C</span>
            </div>
            <div v-if="d.power_on_hours !== undefined" class="smart-stat">
              <span class="stat-label"><Clock :size="11" /> Heures</span>
              <span class="stat-val">{{ d.power_on_hours.toLocaleString() }}h</span>
            </div>
            <div v-if="d.wear_level !== undefined" class="smart-stat" :class="{ warn: d.wear_level < 20 }">
              <span class="stat-label"><Shield :size="11" /> Usure SSD</span>
              <span class="stat-val">{{ d.wear_level }}% restant</span>
            </div>
            <div v-if="d.reallocated_sectors !== undefined && d.reallocated_sectors > 0" class="smart-stat warn">
              <span class="stat-label"><AlertTriangle :size="11" /> Secteurs</span>
              <span class="stat-val">{{ d.reallocated_sectors }} réalloués</span>
            </div>
          </div>
        </div>
      </div>

      <!-- ══ PARTITIONS ══ -->
      <div class="section-label" style="margin-top:12px">Gestionnaire de partitions</div>
      <div v-if="diskGroups.length === 0" class="empty-small">Aucune partition détectée</div>
      <div v-else class="disk-groups">
        <div v-for="group in diskGroups" :key="group.idx" class="disk-group">
          <div class="disk-group-header">
            <HardDrive :size="14" />
            <span>Disque {{ group.idx }} — {{ group.label }}</span>
            <div class="disk-header-actions">
              <NButton variant="ghost" size="sm" @click="openMbr(group.idx, 'backup')" title="Sauvegarder MBR">
                <Download :size="11" /> MBR ↓
              </NButton>
              <NButton variant="ghost" size="sm" @click="openMbr(group.idx, 'restore')" title="Restaurer MBR">
                <Upload :size="11" /> MBR ↑
              </NButton>
              <NButton variant="ghost" size="sm" :loading="scanningLost && scannedDisk === group.idx" @click="doScanLost(group.idx)" title="Scanner partitions perdues">
                <Search :size="11" /> Partitions perdues
              </NButton>
              <NButton variant="ghost" size="sm" @click="openInit(group.idx, group.label)">
                <RotateCcw :size="11" /> Initialiser
              </NButton>
            </div>
          </div>
          <div class="parts-table">
            <div class="part-row hdr">
              <span>Lettre</span><span>Nom</span><span>Taille</span><span>Libre</span><span>FS</span><span>Type</span><span>Santé</span><span>Actions</span>
            </div>
            <div v-for="p in group.parts" :key="`${p.disk_index}-${p.part_index}`" class="part-row" :class="{ 'is-system': p.is_system || p.is_boot, 'is-unalloc': p.part_type === 'Unallocated' }">
              <span class="part-letter">{{ p.letter || '—' }}</span>
              <span class="part-name">
                {{ p.label || (p.part_type === 'Unallocated' ? 'Non alloué' : '—') }}
                <span v-if="p.is_boot" class="badge-mini boot">BOOT</span>
                <span v-if="p.is_system" class="badge-mini sys">SYS</span>
              </span>
              <span class="part-size">{{ formatGb(p.size_gb) }}</span>
              <span class="part-free">
                <template v-if="p.part_type !== 'Unallocated' && p.size_gb > 0">
                  {{ formatGb(p.free_gb) }}
                  <div class="mini-bar"><div class="mini-fill" :style="{ width: `${usedPct(p)}%`, background: usedPct(p) > 90 ? 'var(--danger)' : 'var(--accent-primary)' }" /></div>
                </template>
                <template v-else>—</template>
              </span>
              <span class="part-fs">{{ p.file_system || '—' }}</span>
              <span class="part-type" :style="{ color: typeColor(p.part_type) }">{{ typeLabel(p.part_type) }}</span>
              <span class="part-health">
                <NBadge :variant="healthVariant(p.health)" size="sm">{{ p.health }}</NBadge>
              </span>
              <span class="part-actions">
                <!-- Espace non alloué → Créer -->
                <template v-if="p.part_type === 'Unallocated'">
                  <NButton variant="primary" size="sm" @click="openCreate(p)"><Plus :size="11" /> Créer</NButton>
                </template>
                <!-- Partition existante → actions -->
                <template v-else>
                  <NButton v-if="!p.is_system && !p.is_boot && p.letter" variant="ghost" size="sm" @click="openFormat(p)" title="Formater"><Zap :size="11" /></NButton>
                  <NButton variant="ghost" size="sm" @click="openLetter(p)" title="Assigner une lettre"><Edit2 :size="11" /></NButton>
                  <NButton v-if="!p.is_system && !p.is_boot && p.file_system === 'NTFS'" variant="ghost" size="sm" @click="openResize(p)" title="Redimensionner"><Maximize2 :size="11" /></NButton>
                  <NButton v-if="!p.is_system && !p.is_boot" variant="ghost" size="sm" style="color:var(--danger)" @click="openDelete(p)" title="Supprimer"><Trash2 :size="11" /></NButton>
                </template>
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- ══ PARTITIONS PERDUES ══ -->
      <template v-if="lostPartitions.length > 0">
        <div class="section-label" style="margin-top:12px;color:var(--warning)">
          <AlertTriangle :size="12" /> Régions récupérables détectées (Disque {{ scannedDisk }})
        </div>
        <div class="lost-list">
          <div v-for="lp in lostPartitions" :key="lp.offset_bytes" class="lost-item">
            <Activity :size="14" style="color:var(--warning);flex-shrink:0" />
            <div class="lost-info">
              <span class="lost-desc">{{ lp.description }}</span>
              <span class="lost-meta">{{ lp.fs_hint }} — offset {{ fmtBytes(lp.offset_bytes) }} — taille {{ fmtBytes(lp.size_bytes) }}</span>
            </div>
            <NBadge variant="warning">{{ lp.signature }}</NBadge>
          </div>
        </div>
      </template>
    </template>

    <!-- ══ MODAL FORMAT ══ -->
    <Teleport to="body">
      <div v-if="modalFormat" class="modal-backdrop" @click.self="modalFormat = false">
        <div class="modal">
          <p class="modal-title"><Zap :size="14" /> Formater {{ targetPart?.letter }}</p>
          <p class="modal-warn">⚠ Toutes les données sur ce lecteur seront <strong>effacées définitivement</strong>.</p>
          <div class="field"><label>Système de fichiers</label>
            <select v-model="fmtFs" class="sel">
              <option>NTFS</option><option>FAT32</option><option>exFAT</option>
            </select>
          </div>
          <div class="field"><label>Nom du volume</label>
            <input v-model="fmtLabel" class="inp" placeholder="Nouveau volume" />
          </div>
          <div class="modal-actions">
            <NButton variant="ghost" @click="modalFormat = false">Annuler</NButton>
            <NButton variant="primary" :loading="opLoading" @click="doFormat">Formater</NButton>
          </div>
        </div>
      </div>

      <!-- ══ MODAL LETTRE ══ -->
      <div v-if="modalLetter" class="modal-backdrop" @click.self="modalLetter = false">
        <div class="modal">
          <p class="modal-title"><Edit2 :size="14" /> Assigner une lettre</p>
          <div class="field"><label>Nouvelle lettre (A-Z)</label>
            <input v-model="newLetter" class="inp" maxlength="1" placeholder="D" style="text-transform:uppercase" />
          </div>
          <div class="modal-actions">
            <NButton variant="ghost" @click="modalLetter = false">Annuler</NButton>
            <NButton variant="primary" :loading="opLoading" :disabled="!newLetter" @click="doAssignLetter">Assigner</NButton>
          </div>
        </div>
      </div>

      <!-- ══ MODAL CRÉER ══ -->
      <div v-if="modalCreate" class="modal-backdrop" @click.self="modalCreate = false">
        <div class="modal">
          <p class="modal-title"><Plus :size="14" /> Créer une partition</p>
          <p class="modal-info">Disque {{ targetPart?.disk_index }} — {{ formatGb(targetPart?.size_gb ?? 0) }} disponibles. Format NTFS automatique.</p>
          <div class="field"><label>Taille (Mo, vide = tout l'espace)</label>
            <input v-model.number="newSizeMb" type="number" class="inp" min="100" placeholder="Laisser vide pour maximum" />
          </div>
          <div class="modal-actions">
            <NButton variant="ghost" @click="modalCreate = false">Annuler</NButton>
            <NButton variant="primary" :loading="opLoading" @click="doCreate">Créer</NButton>
          </div>
        </div>
      </div>

      <!-- ══ MODAL SUPPRIMER ══ -->
      <div v-if="modalDelete" class="modal-backdrop" @click.self="modalDelete = false">
        <div class="modal">
          <p class="modal-title" style="color:var(--danger)"><Trash2 :size="14" /> Supprimer la partition</p>
          <p class="modal-warn">⚠ <strong>Action irréversible.</strong> Toutes les données sur {{ targetPart?.letter || `partition ${targetPart?.part_index}` }} seront perdues.</p>
          <div class="field"><label>Tapez <code>SUPPRIMER</code> pour confirmer</label>
            <input v-model="confirmTxt" class="inp" placeholder="SUPPRIMER" />
          </div>
          <div class="modal-actions">
            <NButton variant="ghost" @click="modalDelete = false">Annuler</NButton>
            <NButton variant="primary" style="background:var(--danger)" :loading="opLoading" :disabled="confirmTxt !== 'SUPPRIMER'" @click="doDelete">Supprimer</NButton>
          </div>
        </div>
      </div>

      <!-- ══ MODAL INITIALISER ══ -->
      <div v-if="modalInit" class="modal-backdrop" @click.self="modalInit = false">
        <div class="modal">
          <p class="modal-title" style="color:var(--warning)"><RotateCcw :size="14" /> Initialiser le disque {{ targetPart?.disk_index }}</p>
          <p class="modal-warn">⚠ <strong>Action irréversible.</strong> Toutes les données et partitions seront effacées.</p>
          <div class="field"><label>Table de partitions</label>
            <select v-model="initStyle" class="sel">
              <option value="GPT">GPT (recommandé — UEFI)</option>
              <option value="MBR">MBR (ancien BIOS)</option>
            </select>
          </div>
          <div class="field"><label>Tapez <code>INITIALISER</code> pour confirmer</label>
            <input v-model="confirmTxt" class="inp" placeholder="INITIALISER" />
          </div>
          <div class="modal-actions">
            <NButton variant="ghost" @click="modalInit = false">Annuler</NButton>
            <NButton variant="primary" style="background:var(--warning);color:#000" :loading="opLoading" :disabled="confirmTxt !== 'INITIALISER'" @click="doInit">Initialiser</NButton>
          </div>
        </div>
      </div>
      <!-- ══ MODAL RESIZE ══ -->
      <div v-if="modalResize" class="modal-backdrop" @click.self="modalResize = false">
        <div class="modal">
          <p class="modal-title"><Maximize2 :size="14" /> Redimensionner {{ targetPart?.letter }}</p>
          <div v-if="resizeLimits" class="resize-info">
            <span>Min : {{ Math.round(resizeLimits.min_bytes / 1_048_576) }} Mo</span>
            <span>Actuel : {{ Math.round(resizeLimits.current_bytes / 1_048_576) }} Mo</span>
            <span>Max : {{ Math.round(resizeLimits.max_bytes / 1_048_576) }} Mo</span>
          </div>
          <div v-if="resizeLimits" class="field">
            <label>Nouvelle taille (Mo)</label>
            <input
              v-model.number="resizeSizeMb" type="range"
              :min="Math.ceil(resizeLimits.min_bytes / 1_048_576)"
              :max="Math.floor(resizeLimits.max_bytes / 1_048_576)"
              step="100" class="range-slider"
            />
            <div class="range-val">{{ resizeSizeMb.toLocaleString() }} Mo ({{ (resizeSizeMb / 1024).toFixed(1) }} Go)</div>
          </div>
          <p class="modal-info">NTFS uniquement. L'opération préserve les données si vous réduisez sans dépasser l'espace utilisé.</p>
          <div class="modal-actions">
            <NButton variant="ghost" @click="modalResize = false">Annuler</NButton>
            <NButton variant="primary" :loading="opLoading" :disabled="!resizeLimits" @click="doResize">Appliquer</NButton>
          </div>
        </div>
      </div>

      <!-- ══ MODAL MBR ══ -->
      <div v-if="modalMbr" class="modal-backdrop" @click.self="modalMbr = false">
        <div class="modal">
          <p class="modal-title" :style="{ color: mbrMode === 'restore' ? 'var(--warning)' : 'var(--accent-primary)' }">
            <component :is="mbrMode === 'backup' ? Download : Upload" :size="14" />
            {{ mbrMode === 'backup' ? `Sauvegarder MBR — Disque ${mbrDiskIndex}` : `Restaurer MBR — Disque ${mbrDiskIndex}` }}
          </p>
          <p v-if="mbrMode === 'restore'" class="modal-warn">⚠ <strong>Opération critique.</strong> Restaurer un MBR incorrect peut rendre le disque non-démarrable. Confirmez que le fichier correspond bien à ce disque.</p>
          <div class="field">
            <label>{{ mbrMode === 'backup' ? 'Fichier de destination (.bin)' : 'Fichier MBR source (.bin)' }}</label>
            <input v-if="mbrMode === 'backup'" v-model="mbrOutputPath" class="inp" placeholder="C:\NiTriTe\MBR_backup.bin" />
            <input v-else v-model="mbrRestorePath" class="inp" placeholder="C:\NiTriTe\MBR_backup_disk0.bin" />
          </div>
          <div class="modal-actions">
            <NButton variant="ghost" @click="modalMbr = false">Annuler</NButton>
            <NButton variant="primary" :loading="opLoading" @click="doMbrOp">
              {{ mbrMode === 'backup' ? 'Sauvegarder' : 'Restaurer' }}
            </NButton>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.partition-mgr { display: flex; flex-direction: column; gap: 12px; }

.admin-banner {
  display: flex; gap: 8px; align-items: flex-start; padding: 10px 14px;
  background: color-mix(in srgb, var(--info) 10%, transparent);
  color: var(--info); border: 1px solid color-mix(in srgb, var(--info) 35%, transparent);
  border-radius: var(--radius-md); font-size: 12px; line-height: 1.5;
}
.toolbar { display: flex; align-items: center; gap: 8px; }
.loading-state { display: flex; align-items: center; gap: 10px; padding: 20px; font-size: 13px; color: var(--text-muted); }
.empty-small { padding: 14px; text-align: center; font-size: 12px; color: var(--text-muted); }
.section-label { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: .06em; color: var(--text-muted); }

/* SMART cards */
.smart-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 10px; }
.smart-card { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius-lg); padding: 12px 14px; display: flex; flex-direction: column; gap: 10px; transition: border-color 0.15s; }
.smart-card:hover { border-color: var(--text-muted); }
.smart-header { display: flex; align-items: center; gap: 10px; }
.smart-icon { width: 36px; height: 36px; border-radius: var(--radius-md); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.smart-info { flex: 1; display: flex; flex-direction: column; gap: 2px; overflow: hidden; }
.smart-name { font-size: 12px; font-weight: 700; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.smart-serial { font-size: 10px; color: var(--text-muted); font-family: monospace; }
.smart-stats { display: flex; flex-wrap: wrap; gap: 8px; }
.smart-stat { display: flex; flex-direction: column; gap: 2px; background: var(--bg-tertiary); padding: 5px 8px; border-radius: var(--radius-sm); min-width: 70px; }
.smart-stat.warn .stat-val { color: var(--warning); }
.stat-label { font-size: 10px; color: var(--text-muted); display: flex; align-items: center; gap: 3px; }
.stat-val { font-size: 12px; font-weight: 600; color: var(--text-primary); font-family: monospace; }

/* Partition groups */
.disk-groups { display: flex; flex-direction: column; gap: 12px; }
.disk-group { border: 1px solid var(--border); border-radius: var(--radius-lg); overflow: hidden; }
.disk-group-header {
  display: flex; align-items: center; gap: 8px; padding: 8px 14px;
  background: var(--bg-tertiary); font-size: 12px; font-weight: 700; color: var(--text-primary);
}
.parts-table { overflow-x: auto; }
.part-row {
  display: grid;
  grid-template-columns: 50px 1fr 75px 130px 55px 80px 85px auto;
  align-items: center; gap: 8px; padding: 7px 14px;
  border-top: 1px solid var(--border); font-size: 12px;
}
.part-row.hdr {
  background: var(--bg-tertiary); font-size: 10px; font-weight: 700;
  color: var(--text-muted); text-transform: uppercase; letter-spacing: .05em;
}
.part-row.is-system { background: color-mix(in srgb, var(--danger) 4%, transparent); }
.part-row.is-unalloc { background: color-mix(in srgb, var(--text-muted) 4%, transparent); opacity: .85; }
.part-letter { font-family: monospace; font-weight: 700; color: var(--accent-primary); }
.part-name { display: flex; align-items: center; gap: 5px; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.part-size, .part-free { font-family: monospace; color: var(--text-secondary); }
.part-free { display: flex; align-items: center; gap: 6px; }
.mini-bar { width: 36px; height: 4px; background: var(--bg-elevated); border-radius: 99px; overflow: hidden; flex-shrink: 0; }
.mini-fill { height: 100%; border-radius: 99px; transition: width 0.4s; }
.part-fs { color: var(--text-muted); font-family: monospace; font-size: 11px; }
.part-type { font-size: 11px; font-weight: 600; }
.part-health { }
.part-actions { display: flex; gap: 4px; flex-shrink: 0; }
.badge-mini { font-size: 9px; font-weight: 700; padding: 1px 4px; border-radius: 3px; }
.badge-mini.boot { background: var(--warning-muted); color: var(--warning); }
.badge-mini.sys  { background: var(--danger-muted);  color: var(--danger); }

/* Modals */
.modal-backdrop {
  position: fixed; inset: 0; background: rgba(0,0,0,.55); backdrop-filter: blur(4px);
  display: flex; align-items: center; justify-content: center; z-index: 9999;
}
.modal {
  background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius-xl);
  padding: 22px 24px; min-width: 380px; max-width: 480px; display: flex; flex-direction: column; gap: 14px;
  box-shadow: 0 20px 60px rgba(0,0,0,.4);
}
.modal-title { font-size: 15px; font-weight: 800; color: var(--text-primary); display: flex; align-items: center; gap: 8px; }
.modal-warn { font-size: 12px; color: var(--warning); background: var(--warning-muted); padding: 8px 12px; border-radius: var(--radius-md); border-left: 3px solid var(--warning); line-height: 1.5; }
.modal-info { font-size: 12px; color: var(--text-secondary); }
.field { display: flex; flex-direction: column; gap: 6px; }
.field label { font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; }
.inp, .sel {
  padding: 8px 10px; border: 1px solid var(--border); border-radius: var(--radius-md);
  background: var(--bg-tertiary); color: var(--text-primary); font-family: inherit; font-size: 13px;
  outline: none; transition: border-color 0.15s;
}
.inp:focus, .sel:focus { border-color: var(--accent-primary); }
code { font-family: monospace; background: var(--bg-tertiary); padding: 1px 5px; border-radius: 3px; }
.modal-actions { display: flex; justify-content: flex-end; gap: 8px; padding-top: 4px; }

/* Disk header actions */
.disk-header-actions { display: flex; gap: 4px; align-items: center; margin-left: auto; flex-wrap: wrap; }

/* Lost partitions */
.lost-list { display: flex; flex-direction: column; gap: 6px; }
.lost-item { display: flex; align-items: center; gap: 10px; background: color-mix(in srgb, var(--warning) 6%, transparent); border: 1px solid color-mix(in srgb, var(--warning) 25%, transparent); border-radius: var(--radius-md); padding: 10px 14px; }
.lost-info { flex: 1; display: flex; flex-direction: column; gap: 3px; }
.lost-desc { font-size: 13px; font-weight: 600; color: var(--text-primary); }
.lost-meta { font-size: 11px; color: var(--text-muted); font-family: monospace; }

/* Resize slider */
.resize-info { display: flex; gap: 16px; font-size: 11px; color: var(--text-muted); background: var(--bg-tertiary); padding: 8px 12px; border-radius: var(--radius-md); }
.range-slider { width: 100%; accent-color: var(--accent-primary); }
.range-val { text-align: center; font-size: 14px; font-weight: 700; color: var(--accent-primary); font-family: monospace; }
</style>
