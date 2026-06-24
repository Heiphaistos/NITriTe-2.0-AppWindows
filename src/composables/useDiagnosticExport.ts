/**
 * useDiagnosticExport — Scan système + génération de rapports (HTML/TXT/MD/JSON)
 * Extrait de DiagnosticPage.vue pour réduire la taille du fichier orchestrateur.
 */
import { ref, onUnmounted } from "vue";

const escHtml = (s: any): string =>
  String(s ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
import type { Ref } from "vue";
import { invoke, invokeRaw } from "@/utils/invoke";
import { exportScanHtml } from "@/composables/useScanExport";
import { cachedInvoke } from "@/composables/useCachedInvoke";
import { useNotificationStore } from "@/stores/notifications";
import type {
  SysInfo, BiosInfo, MoboDetailed, GpuDetailed, RamDetailed, CpuCache,
  StoragePhysical, NetworkAdapter, MonitorDetail, AudioDevice, UsbDevice,
  BatteryDetailed, PowerPlan, PrinterDetail, InstalledSoftware,
  EnvVar, StartupProgram, InstalledUpdate, WinLicense, ScanResult,
} from "@/types/diagnostic";

// ── Interface des refs partagés ───────────────────────────────────────────────
export interface DiagnosticState {
  sysInfo:          Ref<SysInfo | null>;
  biosInfo:         Ref<BiosInfo | null>;
  moboInfo:         Ref<MoboDetailed | null>;
  cpuCache:         Ref<CpuCache | null>;
  gpuList:          Ref<GpuDetailed[]>;
  ramData:          Ref<RamDetailed | null>;
  storageList:      Ref<StoragePhysical[]>;
  networkAdapters:  Ref<NetworkAdapter[]>;
  monitors:         Ref<MonitorDetail[]>;
  audioDevices:     Ref<AudioDevice[]>;
  usbDevices:       Ref<UsbDevice[]>;
  batteries:        Ref<BatteryDetailed[]>;
  powerPlans:       Ref<PowerPlan[]>;
  printers:         Ref<PrinterDetail[]>;
  softwareList:     Ref<InstalledSoftware[]>;
  envVars:          Ref<EnvVar[]>;
  startupPrograms:  Ref<StartupProgram[]>;
  updatesHistory:   Ref<InstalledUpdate[]>;
  licenseInfo:      Ref<WinLicense | null>;
  volumes:          Ref<unknown[]>;
  scanResult:       Ref<ScanResult | null>;
  scanProblems:     Ref<string[]>;
  loadedTabs:       Ref<Set<string>>;
}

// ── Helpers formatage ─────────────────────────────────────────────────────────
function kbStr(v: number) { return v >= 1024 ? `${(v / 1024).toFixed(0)} MB` : `${v} KB`; }
function badge(v: boolean, ok = "Activé", ko = "DÉSACTIVÉ") { return v ? ok : ko; }
function cls(v: boolean, good = true) { return (v === good) ? "ok" : "warn"; }

// ── Composable ────────────────────────────────────────────────────────────────
export function useDiagnosticExport(state: DiagnosticState, navigateToScan: () => void) {
  const notify = useNotificationStore();

  // [8] Guard onUnmounted — évite "Set on unmounted component" si l'user navigue pendant un scan
  let mounted = true;
  onUnmounted(() => { mounted = false; });

  // Scan UI state
  const scanning      = ref(false);
  const scanProgress  = ref(0);
  const scanStep      = ref("");

  // Export modal state
  const showExportModal = ref(false);
  const exportFormats   = ref<Set<string>>(new Set(["html"]));
  const exportRunning   = ref(false);
  const modalScanMode   = ref<"total" | "simple" | null>(null);

  // ── Modal ──────────────────────────────────────────────────────────────────
  function openExportModal(mode: "total" | "simple" | null) {
    modalScanMode.value = mode;
    showExportModal.value = true;
  }
  function toggleExportFormat(fmt: string) {
    if (exportFormats.value.has(fmt)) {
      if (exportFormats.value.size > 1) exportFormats.value.delete(fmt);
    } else {
      exportFormats.value.add(fmt);
    }
    exportFormats.value = new Set(exportFormats.value);
  }
  async function confirmScanLaunch() {
    if (!modalScanMode.value) return;
    await launchScanWithFormats(new Set(exportFormats.value));
  }
  async function runExportSelected() {
    exportRunning.value = true;
    await preloadExportData();
    const bde = await invoke<string>("get_bitlocker_report").catch(() => "");
    await Promise.allSettled([
      exportFormats.value.has("html") ? exportHtmlRaw() : Promise.resolve(),
      exportFormats.value.has("txt")  ? exportTxtRaw(bde) : Promise.resolve(),
      exportFormats.value.has("md")   ? exportMdRaw(bde)  : Promise.resolve(),
      exportFormats.value.has("json") ? exportJsonRaw(bde) : Promise.resolve(),
    ]);
    exportRunning.value = false;
    showExportModal.value = false;
  }

  // ── Preload export data ────────────────────────────────────────────────────
  async function preloadExportData() {
    const s = state;
    await Promise.all([
      cachedInvoke<SysInfo>("get_system_info").then(v => { if (v) s.sysInfo.value = v; }).catch(() => {}),
      cachedInvoke<BiosInfo>("get_bios_info").then(v => { if (v) s.biosInfo.value = v; }).catch(() => {}),
      cachedInvoke<MoboDetailed>("get_motherboard_detailed").then(v => { if (v) s.moboInfo.value = v; }).catch(() => {}),
      cachedInvoke<CpuCache>("get_cpu_cache_info").then(v => { if (v) s.cpuCache.value = v; }).catch(() => {}),
      cachedInvoke<GpuDetailed[]>("get_gpu_detailed").then(v => { if (v) s.gpuList.value = v; }).catch(() => {}),
      cachedInvoke<RamDetailed>("get_ram_detailed").then(v => { if (v) s.ramData.value = v; }).catch(() => {}),
      cachedInvoke<StoragePhysical[]>("get_storage_physical_info").then(v => { if (v) s.storageList.value = v; }).catch(() => {}),
      cachedInvoke<NetworkAdapter[]>("get_network_adapters_detailed").then(v => { if (v) s.networkAdapters.value = v; }).catch(() => {}),
      cachedInvoke<MonitorDetail[]>("get_monitor_info").then(v => { if (v) s.monitors.value = v; }).catch(() => {}),
      cachedInvoke<AudioDevice[]>("get_audio_devices").then(v => { if (v) s.audioDevices.value = v; }).catch(() => {}),
      cachedInvoke<UsbDevice[]>("get_usb_devices").then(v => { if (v) s.usbDevices.value = v; }).catch(() => {}),
      cachedInvoke<BatteryDetailed[]>("get_battery_detailed").then(v => { if (v) s.batteries.value = v; }).catch(() => {}),
      cachedInvoke<PowerPlan[]>("get_power_plans").then(v => { if (v) s.powerPlans.value = v; }).catch(() => {}),
      cachedInvoke<PrinterDetail[]>("get_printers").then(v => { if (v) s.printers.value = v; }).catch(() => {}),
      cachedInvoke<StartupProgram[]>("get_startup_programs_detailed").then(v => { if (v) s.startupPrograms.value = v; }).catch(() => {}),
      cachedInvoke<InstalledSoftware[]>("get_installed_software").then(v => { if (v) s.softwareList.value = v; }).catch(() => {}),
      cachedInvoke<EnvVar[]>("get_environment_variables").then(v => { if (v) s.envVars.value = v; }).catch(() => {}),
      cachedInvoke<InstalledUpdate[]>("get_installed_updates").then(v => { if (v) s.updatesHistory.value = v; }).catch(() => {}),
      cachedInvoke<WinLicense>("get_windows_license").then(v => { if (v) s.licenseInfo.value = v; }).catch(() => {}),
    ]);
  }

  // ── Problèmes scan ────────────────────────────────────────────────────────
  function computeProblems(sr: ScanResult) {
    const p: string[] = [];
    if (!sr.firewall_enabled) p.push("🛡 Pare-feu Windows désactivé");
    if (!sr.defender_enabled) p.push("🛡 Windows Defender (temps réel) désactivé ou inconnu");
    if (!sr.network_ok)       p.push("🌐 Pas de connectivité internet (8.8.8.8 injoignable)");
    if (sr.pending_reboot)    p.push("🔄 Redémarrage Windows en attente");
    if (sr.suspicious_processes.length > 0)
      p.push(`⚠ ${sr.suspicious_processes.length} processus suspect(s) hors chemins sécurisés`);
    if (sr.winget_upgradable.length > 0)
      p.push(`🔄 ${sr.winget_upgradable.length} mise(s) à jour WinGet disponible(s)`);
    if (sr.choco_upgradable.length > 0)
      p.push(`🔄 ${sr.choco_upgradable.length} mise(s) à jour Chocolatey disponible(s)`);
    if (sr.dism_status && ["avertissement", "erreur", "corrupt"].some(w => sr.dism_status.toLowerCase().includes(w)))
      p.push(`🔧 DISM: ${sr.dism_status}`);
    {
      const sfc = sr.sfc_status?.toLowerCase() ?? "";
      if (["corrompus", "corrupt", "fichiers corr"].some(w => sfc.includes(w))
       || (sfc.includes("avertissement") && !sfc.includes("non vérifié")))
        p.push(`🔧 SFC: ${sr.sfc_status}`);
    }
    for (const d of sr.disk_usage) {
      if (d.used_percent > 90) p.push(`💾 Disque ${d.drive}: espace critique (${d.used_percent.toFixed(0)}%)`);
      else if (d.used_percent > 80) p.push(`💾 Disque ${d.drive}: espace faible (${d.used_percent.toFixed(0)}%)`);
    }
    if (sr.ram_usage_percent > 85) p.push(`🧠 RAM critique: ${sr.ram_usage_percent.toFixed(0)}% utilisé`);
    for (const b of state.batteries.value) {
      if (b.battery_health_percent > 0 && b.battery_health_percent < 80)
        p.push(`🔋 Batterie "${b.name}": santé faible (${b.battery_health_percent.toFixed(0)}%)`);
      if (b.cycle_count > 400)
        p.push(`🔋 Batterie "${b.name}": ${b.cycle_count} cycles — remplacement recommandé`);
    }
    const winAct = sr.windows_activation || (state.licenseInfo.value?.activation_status ?? "");
    if (winAct && winAct !== "Activé" && winAct !== "Licencié")
      p.push(`🔑 Windows non activé: ${winAct}`);
    if (sr.recent_errors.length > 5)
      p.push(`📋 ${sr.recent_errors.length} erreurs/critiques dans les journaux (48h)`);
    if (sr.defender_definition_age_days > 7)
      p.push(`🛡 Définitions Defender: ${sr.defender_definition_age_days} jours sans mise à jour`);
    if (sr.last_update_days > 60)
      p.push(`🔄 Dernier KB Windows: il y a ${sr.last_update_days} jours`);
    if (sr.temp_folder_size_mb > 2048)
      p.push(`🗑 Fichiers temporaires volumineux: ${(sr.temp_folder_size_mb / 1024).toFixed(1)} GB`);
    if (sr.suspicious_services.length > 5)
      p.push(`⚙ ${sr.suspicious_services.length} services tiers en cours hors chemins sécurisés`);
    if (sr.autorun_entries.length > 8)
      p.push(`🚀 ${sr.autorun_entries.length} entrées Autorun tiers détectées`);
    if (sr.last_bsod && !sr.last_bsod.includes("Aucun"))
      p.push(`💥 Dernier BSOD détecté: ${sr.last_bsod}`);
    state.scanProblems.value = p;
  }

  // ── Scans ─────────────────────────────────────────────────────────────────
  async function runTotalScan() {
    scanning.value = true; scanProgress.value = 0; scanStep.value = "Démarrage...";
    state.scanResult.value = null; state.scanProblems.value = [];
    const { listen } = await import("@tauri-apps/api/event");
    try {
      scanStep.value = "Chargement composants..."; scanProgress.value = 5;
      await Promise.allSettled([
        state.batteries.value.length === 0
          ? invoke<BatteryDetailed[]>("get_battery_detailed").then(v => { state.batteries.value = v; state.loadedTabs.value.add("battery"); })
          : Promise.resolve(),
        !state.licenseInfo.value
          ? invoke<WinLicense>("get_windows_license").then(v => { state.licenseInfo.value = v; state.loadedTabs.value.add("license"); }).catch(() => {})
          : Promise.resolve(),
        state.storageList.value.length === 0
          ? invoke<StoragePhysical[]>("get_storage_physical_info").then(v => { state.storageList.value = v; state.loadedTabs.value.add("disks"); })
          : Promise.resolve(),
        !state.sysInfo.value
          ? invoke<SysInfo>("get_system_info").then(v => { state.sysInfo.value = v; })
          : Promise.resolve(),
      ]);
      scanProgress.value = 15;
      const unlisten = await listen<{ step: string; percent: number }>("scan-progress", (e) => {
        if (!mounted) return;
        scanProgress.value = 15 + Math.round(e.payload.percent * 0.85);
        scanStep.value = e.payload.step;
      });
      try { state.scanResult.value = await invokeRaw<ScanResult>("run_total_scan"); }
      finally { unlisten(); }
      if (mounted && state.scanResult.value) computeProblems(state.scanResult.value);
    } catch { if (mounted) notify.error("Scan échoué"); }
    finally { if (mounted) scanning.value = false; }
  }

  async function launchScanWithFormats(formats?: Set<string>) {
    const fmts: Set<string> = (formats && typeof formats.has === "function") ? formats : new Set(["html"]);
    showExportModal.value = false;
    navigateToScan();
    await runTotalScan();
    if (state.scanResult.value) {
      exportRunning.value = true;
      await preloadExportData();
      const bde = await invoke<string>("get_bitlocker_report").catch(() => "");
      await Promise.allSettled([
        fmts.has("html") ? exportScanHtml(state.scanResult.value, state.scanProblems.value, state.batteries.value, []) : Promise.resolve(),
        fmts.has("txt")  ? exportTxtRaw(bde) : Promise.resolve(),
        fmts.has("md")   ? exportMdRaw(bde)  : Promise.resolve(),
        fmts.has("json") ? exportJsonRaw(bde) : Promise.resolve(),
      ]);
      exportRunning.value = false;
    }
  }

  // ── Export I/O ────────────────────────────────────────────────────────────
  async function writeExport(defaultName: string, content: string, ext: string) {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const filePath = await save({
        defaultPath: defaultName,
        filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
      });
      if (!filePath) return;
      await invoke("save_content_to_path", { path: filePath, content });
      notify.success("Export sauvegardé", filePath);
      await invokeRaw("open_path", { path: filePath }).catch(() => {});
    } catch (e: unknown) { notify.error("Erreur export", String(e)); }
  }

  async function openExportFolder() {
    await invoke("open_path", { path: await invoke<string>("get_export_dir") });
  }

  // ── JSON Export ───────────────────────────────────────────────────────────
  async function exportJsonRaw(bde: string) {
    const s = state;
    const l = s.licenseInfo.value;
    const payload: Record<string, unknown> = {
      generated: new Date().toISOString(),
      system: s.sysInfo.value, bios: s.biosInfo.value, motherboard: s.moboInfo.value,
      cpu_cache: s.cpuCache.value, gpus: s.gpuList.value, ram: s.ramData.value,
      storage: s.storageList.value, volumes: s.volumes.value,
      network: s.networkAdapters.value, monitors: s.monitors.value,
      audio: s.audioDevices.value, usb: s.usbDevices.value,
      batteries: s.batteries.value, power_plans: s.powerPlans.value,
      printers: s.printers.value, software: s.softwareList.value,
      startup: s.startupPrograms.value, env_vars: s.envVars.value,
      updates: s.updatesHistory.value,
      license: l ? { ...l, windows_key_full: l.full_product_key, office_key_full: l.office_full_key || l.office_key } : null,
      bitlocker_raw: bde || null,
      bitlocker_volumes: s.scanResult.value?.bitlocker_volumes || null,
      scan: s.scanResult.value, problems: s.scanProblems.value,
    };
    await writeExport("diagnostic.json", JSON.stringify(payload, null, 2), "json");
  }
  async function exportJson() {
    await preloadExportData();
    const bde = await invoke<string>("get_bitlocker_report").catch(() => "");
    await exportJsonRaw(bde);
  }

  // ── TXT Export ────────────────────────────────────────────────────────────
  function buildTxtReport(bdeReport = ""): string {
    const s = state;
    const si = s.sysInfo.value;
    const lines: string[] = [
      "═══════════════════════════════════════════",
      "       RAPPORT DIAGNOSTIC — NiTriTe",
      `       Généré le ${new Date().toLocaleString()}`,
      "═══════════════════════════════════════════", "",
    ];
    if (si) {
      lines.push("[ SYSTÈME ]",
        `  OS         : ${si.os.name} ${si.os.version} (${si.os.architecture})`,
        `  Hostname   : ${si.os.hostname}`, `  Build      : ${si.os.build_number}`, "");
      lines.push("[ PROCESSEUR ]", `  Modèle     : ${si.cpu.name}`,
        `  Fabricant  : ${si.cpu.manufacturer}`,
        `  Cœurs      : ${si.cpu.cores} cores / ${si.cpu.threads} threads`,
        `  Fréquence  : ${(si.cpu.base_speed_mhz / 1000).toFixed(2)} GHz`, "");
      if (s.cpuCache.value) {
        const c = s.cpuCache.value;
        lines.push("[ CACHE CPU ]",
          `  L1i: ${kbStr(c.l1_instruction_kb)}  L1d: ${kbStr(c.l1_data_kb)}  L2: ${kbStr(c.l2_kb)}  L3: ${kbStr(c.l3_kb)}`, "");
      }
      lines.push("[ MÉMOIRE RAM ]",
        `  Total      : ${si.ram.total_gb.toFixed(1)} GB`,
        `  Utilisée   : ${si.ram.used_gb.toFixed(1)} GB (${Math.round(si.ram.usage_percent)}%)`, "");
    }
    if (s.ramData.value) {
      lines.push("[ SLOTS RAM ]", `  Slots: ${s.ramData.value.used_slots}/${s.ramData.value.total_slots} utilisés`);
      for (const sl of s.ramData.value.slots)
        lines.push(`  ${sl.device_locator} : ${sl.capacity_gb.toFixed(0)} GB ${sl.memory_type}-${sl.speed_mhz} (${sl.manufacturer}) [${sl.form_factor}] P/N: ${sl.part_number || "N/A"}`);
      lines.push("");
    }
    if (s.biosInfo.value) {
      const b = s.biosInfo.value;
      lines.push("[ BIOS ]", `  Fabricant  : ${b.manufacturer}`, `  Version    : ${b.version}`,
        `  Date       : ${b.release_date}`, `  SMBIOS     : ${b.smbios_version}`,
        `  N° Série   : ${b.serial_number || "N/A"}`, "");
    }
    if (s.moboInfo.value) {
      const m = s.moboInfo.value;
      lines.push("[ CARTE MÈRE ]", `  Fabricant  : ${m.manufacturer}`, `  Modèle     : ${m.product}`,
        `  Version    : ${m.version}`, `  N° Série   : ${m.serial_number || "N/A"}`, "");
    }
    if (s.gpuList.value.length) {
      lines.push("[ GPU ]");
      for (const g of s.gpuList.value)
        lines.push(`  ${g.name}`, `    VRAM: ${g.adapter_ram_mb >= 1024 ? (g.adapter_ram_mb / 1024).toFixed(0) + "GB" : g.adapter_ram_mb + "MB"} | Driver: ${g.driver_version} (${g.driver_date}) | ${g.current_resolution} @${g.current_refresh_rate}Hz`);
      lines.push("");
    }
    if (s.storageList.value.length) {
      lines.push("[ STOCKAGE ]");
      for (const d of s.storageList.value)
        lines.push(`  ${d.model} | ${d.size_gb.toFixed(0)} GB | ${d.interface_type} | ${d.media_type} | S/N: ${d.serial_number || "N/A"} | FW: ${d.firmware_revision || "N/A"} | Statut: ${d.status}`);
      lines.push("");
    }
    if (s.networkAdapters.value.length) {
      lines.push("[ RÉSEAU ]");
      for (const a of s.networkAdapters.value)
        lines.push(`  ${a.name} | MAC: ${a.mac_address} | IP: ${a.ip_addresses.join(",")} | ${a.speed_mbps} Mbps | GW: ${a.default_gateway.join(",")} | DNS: ${a.dns_servers.join(",")}`);
      lines.push("");
    }
    if (s.batteries.value.length) {
      lines.push("[ BATTERIE ]");
      for (const b of s.batteries.value)
        lines.push(`  ${b.name} | Charge: ${b.estimated_charge_remaining}% | Autonomie: ${b.estimated_run_time}`,
          `    Cap. originale: ${b.design_capacity} mWh | Cap. actuelle: ${b.full_charge_capacity} mWh`,
          `    Santé: ${b.battery_health_percent.toFixed(1)}% | Cycles: ${b.cycle_count} | Chimie: ${b.chemistry}`);
      lines.push("");
    }
    if (s.licenseInfo.value) {
      const l = s.licenseInfo.value;
      lines.push("[ LICENCE WINDOWS ]",
        `  Produit    : ${l.product_name}`, `  Statut     : ${l.activation_status}`,
        `  Clé OEM/UEFI : ${l.full_product_key || "(non disponible via OEM/UEFI)"}`,
        `  Clé partielle: XXXXX-XXXXX-XXXXX-XXXXX-${l.partial_product_key}`, "");
      if (l.office_name)
        lines.push("[ LICENCE OFFICE ]", `  Produit    : ${l.office_name}`, `  Statut     : ${l.office_status}`,
          `  Clé        : ${l.office_full_key || l.office_key || "(non disponible)"}`, "");
    }
    {
      const sr = s.scanResult.value;
      if (sr?.bitlocker_volumes?.length) {
        lines.push("[ BITLOCKER ]");
        for (const bv of sr.bitlocker_volumes)
          lines.push(`  ${bv.drive} | ${bv.protection_status} | ${bv.encryption_percent}% | Clé: ${bv.recovery_password || "—"}`);
        lines.push("");
      } else if (bdeReport) {
        lines.push("[ BITLOCKER ]", ...bdeReport.split("\n").map((l) => `  ${l}`), "");
      }
    }
    if (s.monitors.value.length) {
      lines.push("[ ÉCRANS ]");
      for (const m of s.monitors.value)
        lines.push(`  ${m.name} — ${m.screen_width}x${m.screen_height} — ${m.pixels_per_inch} PPI — ${m.manufacturer || "N/A"}`);
      lines.push("");
    }
    if (s.softwareList.value.length) {
      const sorted = [...s.softwareList.value].sort((a, b) => (b.install_date || "").localeCompare(a.install_date || "")).slice(0, 100);
      lines.push(`[ LOGICIELS — ${s.softwareList.value.length} total, 100 derniers ]`);
      for (const sw of sorted)
        lines.push(`  ${sw.name.padEnd(48)} | v${(sw.version || "—").padEnd(18)} | ${(sw.publisher || "—").padEnd(28)} | ${sw.install_date || "—"}`);
      lines.push("");
    }
    if (s.updatesHistory.value.length) {
      lines.push(`[ MISES À JOUR — ${s.updatesHistory.value.length} total ]`);
      for (const u of s.updatesHistory.value.slice(0, 30))
        lines.push(`  ${u.hotfix_id.padEnd(14)} | ${(u.description || "").padEnd(18)} | ${u.installed_on || "—"} | ${u.installed_by || "—"}`);
      lines.push("");
    }
    if (s.scanResult.value) {
      const sr = s.scanResult.value;
      lines.push("[ SCAN TOTAL SYSTÈME ]",
        `  DISM              : ${sr.dism_status}`,
        `  SFC               : ${sr.sfc_status}`,
        `  Pare-feu          : ${sr.firewall_enabled ? "Activé" : "DÉSACTIVÉ ⚠"}`,
        `  Defender          : ${sr.defender_enabled ? "Actif" : "INACTIF ⚠"}`,
        `  Connectivité      : ${sr.network_ok ? "OK" : "HORS LIGNE ⚠"}`,
        `  Reboot requis     : ${sr.pending_reboot ? "OUI ⚠" : "Non"}`,
        `  Uptime            : ${sr.uptime_hours >= 24 ? (sr.uptime_hours / 24).toFixed(1) + " jours" : sr.uptime_hours.toFixed(1) + " h"}`,
        `  Activation        : ${sr.windows_activation || "Inconnu"}`,
        `  Dernier BSOD      : ${sr.last_bsod || "Aucun"}`,
        `  WinGet MAJ        : ${sr.winget_upgradable.length}`,
        `  Processus suspects: ${sr.suspicious_processes.length}`,
        `  Ports ouverts     : ${sr.open_ports.join(", ") || "Aucun"}`, "");
    }
    if (s.scanProblems.value.length) {
      lines.push("[ PROBLÈMES DÉTECTÉS ]");
      for (const p of s.scanProblems.value) lines.push(`  ${p}`);
      lines.push("");
    } else if (s.scanResult.value) {
      lines.push("[ ÉTAT GLOBAL ]", "  ✓ Aucun problème critique détecté", "");
    }
    lines.push("═══════════════════════════════════════════",
      "  Fin du rapport — NiTriTe",
      `  Généré le ${new Date().toLocaleString()}`,
      "═══════════════════════════════════════════");
    return lines.join("\n");
  }

  async function exportTxtRaw(bde: string) { await writeExport("diagnostic.txt", buildTxtReport(bde), "txt"); }
  async function exportTxt() {
    await preloadExportData();
    const bde = await invoke<string>("get_bitlocker_report").catch(() => "");
    await exportTxtRaw(bde);
  }

  // ── HTML Export ───────────────────────────────────────────────────────────
  async function exportHtmlRaw() {
    const s = state;
    const si = s.sysInfo.value;
    const sr = s.scanResult.value;
    const CSS = `*{box-sizing:border-box;margin:0;padding:0}body{font-family:'Segoe UI',Arial,sans-serif;background:#0d0d1a;color:#c9d1e0;padding:24px;line-height:1.5}h1{color:#7c9af5;font-size:22px;margin-bottom:4px}h2{color:#a78bfa;font-size:15px;margin:18px 0 8px;border-bottom:1px solid #2a2a3e;padding-bottom:6px}h3{color:#94a3b8;font-size:12px;font-weight:600;margin:12px 0 5px}.header{text-align:center;background:#161625;padding:20px;border-radius:10px;margin-bottom:20px}.header p{color:#64748b;font-size:12px;margin-top:5px}.section{background:#161625;border-radius:8px;padding:16px;margin-bottom:12px;border:1px solid #1e1e35}table{width:100%;border-collapse:collapse;margin-top:6px}th{background:#1a1a2e;color:#7c9af5;padding:6px 10px;text-align:left;font-size:11px;font-weight:600}td{padding:5px 10px;border-bottom:1px solid #1e1e35;font-size:12px}tr:last-child td{border-bottom:none}.ok{background:#052e16;color:#4ade80;padding:1px 7px;border-radius:4px;font-size:11px;display:inline-block}.warn{background:#3b1f00;color:#fb923c;padding:1px 7px;border-radius:4px;font-size:11px;display:inline-block}.danger{background:#3b0000;color:#f87171;padding:1px 7px;border-radius:4px;font-size:11px;display:inline-block}code{font-family:Consolas,monospace;background:#1a1a2e;padding:1px 5px;border-radius:3px;font-size:11px;color:#7c9af5}.grid2{display:grid;grid-template-columns:1fr 1fr;gap:14px}.kv{display:flex;padding:4px 0;border-bottom:1px solid #1e1e35;font-size:12px}.kv .k{color:#94a3b8;min-width:200px;flex-shrink:0}.kv .v{word-break:break-all}`;
    let h = `<!DOCTYPE html><html lang="fr"><head><meta charset="UTF-8"><title>Rapport NiTriTe — ${new Date().toLocaleDateString()}</title><style>${CSS}</style></head><body>`;
    h += `<div class="header"><h1>🖥 Rapport Diagnostic — NiTriTe</h1><p>Généré le ${new Date().toLocaleString()}</p></div>`;
    if (s.scanProblems.value.length)
      h += `<div class="section"><h2>⚠ Problèmes Détectés (${s.scanProblems.value.length})</h2><ul style="list-style:none">${s.scanProblems.value.map(p => `<li style="padding:5px 0;border-bottom:1px solid #1e1e35;font-size:12px">${escHtml(p)}</li>`).join("")}</ul></div>`;
    else if (sr)
      h += `<div class="section"><p style="color:#4ade80">✅ Aucun problème critique détecté</p></div>`;
    if (si) {
      h += `<div class="section"><h2>Système</h2>
<div class="kv"><span class="k">OS</span><span class="v">${escHtml(si.os.name)} ${escHtml(si.os.version)} (${escHtml(si.os.architecture)})</span></div>
<div class="kv"><span class="k">Hostname</span><span class="v">${escHtml(si.os.hostname)}</span></div>
<div class="kv"><span class="k">Build</span><span class="v">${escHtml(si.os.build_number)}</span></div>
<h3>Processeur</h3>
<div class="kv"><span class="k">Modèle</span><span class="v">${escHtml(si.cpu.name)}</span></div>
<div class="kv"><span class="k">Cœurs / Threads</span><span class="v">${escHtml(si.cpu.cores)} / ${escHtml(si.cpu.threads)}</span></div>
<div class="kv"><span class="k">Fréquence</span><span class="v">${(si.cpu.base_speed_mhz / 1000).toFixed(2)} GHz</span></div>
<h3>RAM</h3>
<div class="kv"><span class="k">Total</span><span class="v">${si.ram.total_gb.toFixed(1)} GB</span></div>
<div class="kv"><span class="k">Utilisée</span><span class="v">${si.ram.used_gb.toFixed(1)} GB (${Math.round(si.ram.usage_percent)}%)</span></div></div>`;
    }
    if (s.ramData.value) {
      h += `<div class="section"><h2>RAM — ${escHtml(s.ramData.value.used_slots)}/${escHtml(s.ramData.value.total_slots)} slots — ${s.ramData.value.total_capacity_gb.toFixed(0)} GB</h2><table><tr><th>Slot</th><th>Capacité</th><th>Type</th><th>Vitesse</th><th>Fabricant</th><th>P/N</th></tr>`;
      for (const sl of s.ramData.value.slots)
        h += `<tr><td>${escHtml(sl.device_locator)}</td><td>${sl.capacity_gb.toFixed(0)} GB</td><td>${escHtml(sl.memory_type)}</td><td>${escHtml(sl.speed_mhz)} MHz</td><td>${escHtml(sl.manufacturer)}</td><td>${escHtml(sl.part_number || "—")}</td></tr>`;
      h += `</table></div>`;
    }
    if (s.gpuList.value.length) {
      h += `<div class="section"><h2>GPU (${s.gpuList.value.length})</h2><table><tr><th>Modèle</th><th>VRAM</th><th>Driver</th><th>Date driver</th><th>Résolution</th></tr>`;
      for (const g of s.gpuList.value)
        h += `<tr><td>${escHtml(g.name)}</td><td>${g.adapter_ram_mb >= 1024 ? (g.adapter_ram_mb / 1024).toFixed(0) + "GB" : g.adapter_ram_mb + "MB"}</td><td>${escHtml(g.driver_version)}</td><td>${escHtml(g.driver_date)}</td><td>${escHtml(g.current_resolution)} @${escHtml(g.current_refresh_rate)}Hz</td></tr>`;
      h += `</table></div>`;
    }
    if (s.storageList.value.length) {
      h += `<div class="section"><h2>Stockage Physique</h2><table><tr><th>Modèle</th><th>Taille</th><th>Interface</th><th>Type</th><th>Serial</th><th>Statut</th></tr>`;
      for (const d of s.storageList.value)
        h += `<tr><td>${escHtml(d.model)}</td><td>${d.size_gb.toFixed(0)} GB</td><td>${escHtml(d.interface_type)}</td><td>${escHtml(d.media_type)}</td><td><code>${escHtml(d.serial_number || "—")}</code></td><td>${escHtml(d.status)}</td></tr>`;
      h += `</table></div>`;
    }
    if (s.networkAdapters.value.length) {
      h += `<div class="section"><h2>Réseau</h2><table><tr><th>Nom</th><th>MAC</th><th>IP</th><th>Vitesse</th><th>DNS</th></tr>`;
      for (const a of s.networkAdapters.value.slice(0, 10))
        h += `<tr><td>${escHtml(a.name)}</td><td><code>${escHtml(a.mac_address)}</code></td><td>${a.ip_addresses.map(escHtml).join(", ")}</td><td>${escHtml(a.speed_mbps)} Mbps</td><td>${a.dns_servers.slice(0, 2).map(escHtml).join(", ")}</td></tr>`;
      h += `</table></div>`;
    }
    if (s.batteries.value.length) {
      h += `<div class="section"><h2>🔋 Batterie</h2><table><tr><th>Nom</th><th>Statut</th><th>Charge</th><th>Autonomie</th><th>Cap. originale</th><th>Cap. actuelle</th><th>Santé</th><th>Chimie</th><th>Cycles</th></tr>`;
      for (const b of s.batteries.value)
        h += `<tr><td>${escHtml(b.name)}</td><td>${escHtml(b.status)}</td><td>${escHtml(b.estimated_charge_remaining)}%</td><td>${escHtml(b.estimated_run_time)}</td><td>${escHtml(b.design_capacity)} mWh</td><td>${escHtml(b.full_charge_capacity)} mWh</td><td>${b.battery_health_percent.toFixed(1)}%</td><td>${escHtml(b.chemistry)}</td><td>${escHtml(b.cycle_count)}</td></tr>`;
      h += `</table></div>`;
    }
    if (s.licenseInfo.value) {
      const l = s.licenseInfo.value;
      h += `<div class="section"><h2>Licences</h2>
<div class="kv"><span class="k">Produit Windows</span><span class="v">${escHtml(l.product_name)}</span></div>
<div class="kv"><span class="k">Statut</span><span class="v"><span class="${l.activation_status === "Activé" ? "ok" : "danger"}">${escHtml(l.activation_status)}</span></span></div>
<div class="kv"><span class="k">Clé Windows</span><span class="v"><code style="color:#fb923c">${escHtml(l.full_product_key || "Non disponible via OEM/UEFI")}</code></span></div>
<div class="kv"><span class="k">Clé partielle</span><span class="v"><code>XXXXX-XXXXX-XXXXX-XXXXX-${escHtml(l.partial_product_key)}</code></span></div>
${l.office_name ? `<div class="kv"><span class="k">${escHtml(l.office_name)} — Clé</span><span class="v"><code style="color:#fb923c">${escHtml(l.office_full_key || l.office_key || "Non disponible")}</code></span></div>` : ""}
</div>`;
    }
    if (sr) {
      h += `<div class="section"><h2>Scan Total</h2><div class="grid2">
<div><h3>Sécurité</h3>
<div class="kv"><span class="k">Pare-feu</span><span class="v"><span class="${cls(sr.firewall_enabled)}">${badge(sr.firewall_enabled)}</span></span></div>
<div class="kv"><span class="k">Defender</span><span class="v"><span class="${cls(sr.defender_enabled)}">${badge(sr.defender_enabled, "Actif", "INACTIF")}</span></span></div>
<div class="kv"><span class="k">SMBv1</span><span class="v"><span class="${sr.smbv1_enabled ? "danger" : "ok"}">${sr.smbv1_enabled ? "Activé ⚠" : "Désactivé"}</span></span></div>
<div class="kv"><span class="k">TPM</span><span class="v"><span class="${cls(sr.tpm_present)}">${sr.tpm_present ? (sr.tpm_enabled ? "Présent & Activé" : "Présent (désactivé)") : "Absent"}</span> ${sr.tpm_version || ""}</span></div>
<div class="kv"><span class="k">Secure Boot</span><span class="v"><span class="${cls(sr.secure_boot)}">${badge(sr.secure_boot)}</span></span></div>
<div class="kv"><span class="k">RDP</span><span class="v"><span class="${sr.rdp_enabled ? "warn" : "ok"}">${badge(sr.rdp_enabled)}</span></span></div>
</div>
<div><h3>État Système</h3>
<div class="kv"><span class="k">DISM</span><span class="v">${sr.dism_status}</span></div>
<div class="kv"><span class="k">SFC</span><span class="v">${sr.sfc_status}</span></div>
<div class="kv"><span class="k">Connectivité</span><span class="v"><span class="${cls(sr.network_ok)}">${sr.network_ok ? "OK" : "Hors ligne"}</span></span></div>
<div class="kv"><span class="k">Reboot requis</span><span class="v"><span class="${sr.pending_reboot ? "warn" : "ok"}">${sr.pending_reboot ? "Oui" : "Non"}</span></span></div>
<div class="kv"><span class="k">Dernier BSOD</span><span class="v">${sr.last_bsod || "Aucun"}</span></div>
<div class="kv"><span class="k">WinGet MAJ</span><span class="v">${sr.winget_upgradable.length}</span></div>
</div></div></div>`;
    }
    if (s.softwareList.value.length) {
      const sorted = [...s.softwareList.value].sort((a, b) => (b.install_date || "").localeCompare(a.install_date || "")).slice(0, 50);
      h += `<div class="section"><h2>Logiciels (${s.softwareList.value.length} total — 50 récents)</h2><table><tr><th>Nom</th><th>Version</th><th>Éditeur</th><th>Date</th></tr>`;
      for (const sw of sorted)
        h += `<tr><td>${escHtml(sw.name)}</td><td>${escHtml(sw.version || "—")}</td><td>${escHtml(sw.publisher || "—")}</td><td>${escHtml(sw.install_date || "—")}</td></tr>`;
      h += `</table></div>`;
    }
    if (s.updatesHistory.value.length) {
      h += `<div class="section"><h2>Mises à jour (${s.updatesHistory.value.length})</h2><table><tr><th>KB</th><th>Description</th><th>Installé le</th></tr>`;
      for (const u of s.updatesHistory.value.slice(0, 30))
        h += `<tr><td><code>${escHtml(u.hotfix_id)}</code></td><td>${escHtml(u.description || "—")}</td><td>${escHtml(u.installed_on || "—")}</td></tr>`;
      h += `</table></div>`;
    }
    h += `<div style="text-align:center;margin-top:20px;color:#475569;font-size:11px">Rapport généré par NiTriTe — ${new Date().toLocaleString()}</div></body></html>`;
    await writeExport("diagnostic.html", h, "html");
  }
  async function exportHtml() { await preloadExportData(); await exportHtmlRaw(); }

  // ── Markdown Export ───────────────────────────────────────────────────────
  async function exportMdRaw(bde: string) {
    const s = state;
    const si = s.sysInfo.value;
    const lines = ["# Rapport Diagnostic — NiTriTe", `> ${new Date().toLocaleString()}`, ""];
    if (si) {
      lines.push("## Système", `| | |`, `|---|---|`,
        `| OS | ${si.os.name} ${si.os.version} (${si.os.architecture}) |`,
        `| Hostname | ${si.os.hostname} |`, `| Build | ${si.os.build_number} |`, "");
      lines.push("## Processeur",
        `**${si.cpu.name}** — ${si.cpu.cores}c/${si.cpu.threads}t @ ${(si.cpu.base_speed_mhz / 1000).toFixed(2)} GHz`, "");
    }
    if (s.cpuCache.value) {
      const c = s.cpuCache.value;
      lines.push(`Cache : L1i ${kbStr(c.l1_instruction_kb)} | L1d ${kbStr(c.l1_data_kb)} | L2 ${kbStr(c.l2_kb)} | L3 ${kbStr(c.l3_kb)}`, "");
    }
    if (s.ramData.value) {
      lines.push("## RAM",
        `${s.ramData.value.used_slots}/${s.ramData.value.total_slots} slots — ${s.ramData.value.total_capacity_gb.toFixed(0)} GB total`, "",
        `| Slot | GB | Type | Vitesse | Fabricant | P/N |`, `|---|---|---|---|---|---|`,
        ...s.ramData.value.slots.map(sl => `| ${sl.device_locator} | ${sl.capacity_gb.toFixed(0)} | ${sl.memory_type} | ${sl.speed_mhz} MHz | ${sl.manufacturer} | ${sl.part_number || "—"} |`), "");
    }
    if (s.gpuList.value.length) {
      lines.push("## GPU", `| Modèle | VRAM | Driver | Résolution | Hz |`, `|---|---|---|---|---|`,
        ...s.gpuList.value.map(g => `| ${g.name} | ${g.adapter_ram_mb >= 1024 ? (g.adapter_ram_mb / 1024).toFixed(0) + "GB" : g.adapter_ram_mb + "MB"} | ${g.driver_version} | ${g.current_resolution} | ${g.current_refresh_rate} |`), "");
    }
    if (s.storageList.value.length) {
      lines.push("## Stockage", `| Modèle | Taille | Interface | Type | Serial | Statut |`, `|---|---|---|---|---|---|`,
        ...s.storageList.value.map(d => `| ${d.model} | ${d.size_gb.toFixed(0)} GB | ${d.interface_type} | ${d.media_type} | ${d.serial_number || "—"} | ${d.status} |`), "");
    }
    if (s.batteries.value.length) {
      lines.push("## 🔋 Batterie", `| Nom | Statut | Charge | Autonomie | Cap. originale | Cap. actuelle | Santé | Chimie | Cycles |`, `|---|---|---|---|---|---|---|---|---|`,
        ...s.batteries.value.map(b => `| ${b.name} | ${b.status} | ${b.estimated_charge_remaining}% | ${b.estimated_run_time} | ${b.design_capacity} mWh | ${b.full_charge_capacity} mWh | ${b.battery_health_percent.toFixed(1)}% | ${b.chemistry} | ${b.cycle_count} |`), "");
    }
    if (s.licenseInfo.value) {
      const l = s.licenseInfo.value;
      lines.push("## Licences", `| | |`, `|---|---|`,
        `| Produit Windows | ${l.product_name} |`, `| Statut | ${l.activation_status} |`,
        `| Clé Windows | \`${l.full_product_key || "Non disponible"}\` |`,
        ...(l.office_name ? [`| ${l.office_name} | ${l.office_status} |`, `| Clé Office | \`${l.office_full_key || l.office_key || "Non disponible"}\` |`] : []), "");
    }
    {
      const sr = s.scanResult.value;
      if (sr?.bitlocker_volumes?.length) {
        lines.push("## BitLocker", `| Volume | Statut | Chiffrement | Clé |`, `|---|---|---|---|`,
          ...sr.bitlocker_volumes.map(bv => `| ${bv.drive} | ${bv.protection_status} | ${bv.encryption_percent}% | \`${bv.recovery_password || "N/A"}\` |`), "");
      } else if (bde) {
        lines.push("## BitLocker", "```", bde, "```", "");
      }
    }
    if (s.softwareList.value.length) {
      lines.push("## Logiciels installés (50 plus récents)",
        `| Nom | Version | Éditeur | Date |`, `|---|---|---|---|`,
        ...[...s.softwareList.value].sort((a, b) => (b.install_date || "").localeCompare(a.install_date || "")).slice(0, 50)
          .map(sw => `| ${sw.name} | ${sw.version || "—"} | ${sw.publisher || "—"} | ${sw.install_date || "—"} |`), "");
    }
    if (s.scanProblems.value.length)
      lines.push("## ⚠ Problèmes détectés", ...s.scanProblems.value.map(p => `- ${p}`), "");
    else if (s.scanResult.value)
      lines.push("## ✅ Aucun problème critique", "");
    await writeExport("diagnostic.md", lines.join("\n"), "md");
  }
  async function exportMd() {
    await preloadExportData();
    const bde = await invoke<string>("get_bitlocker_report").catch(() => "");
    await exportMdRaw(bde);
  }

  return {
    // Scan state
    scanning, scanProgress, scanStep,
    // Export modal state
    showExportModal, exportFormats, exportRunning, modalScanMode,
    // Modal actions
    openExportModal, confirmScanLaunch, toggleExportFormat, runExportSelected,
    // Scan actions
    runTotalScan, launchScanWithFormats,
    // Export actions
    exportHtml, exportTxt, exportMd, exportJson,
    openExportFolder,
  };
}
