import { invoke, invokeRaw, useNotificationStore, fullRegPath, oneLine, type Solution } from "./scanExportHelpers";
import type { ScanResult, BatteryDetailed } from "@/types/diagnostic";

export async function exportScanTxt(
  scanResult: ScanResult | null,
  scanProblems: string[],
  batteries: BatteryDetailed[],
  scanSolutions: Solution[]
) {
  if (!scanResult) return;
  const sr = scanResult;
  const W = 78;
  const SEP  = "=".repeat(W);
  const THIN = "-".repeat(W);

  function sec(title: string): string {
    const t = `  ${title}  `;
    const r = W - t.length;
    const l = Math.floor(r / 2);
    return "=".repeat(l) + t + "=".repeat(r - l);
  }
  function kv(label: string, value: string): string {
    return `  ${(label + " :").padEnd(26)} ${value}`;
  }
  function bar(pct: number, w = 20): string {
    const filled = Math.round(pct / 100 * w);
    return "[" + "#".repeat(filled) + ".".repeat(w - filled) + "]";
  }

  const lines: string[] = [
    SEP,
    sec("RAPPORT SCAN TOTAL — NiTriTe"),
    `${"  Generated :".padEnd(28)} ${new Date().toLocaleString()}`,
    SEP, "",
  ];

  lines.push(sec("=== COMPOSANTS DU PC ==="), "");

  lines.push(sec("IDENTITE SYSTEME & BIOS"), THIN,
    kv("Fabricant",             sr.system_manufacturer || "N/A"),
    kv("Modele",                sr.system_model        || "N/A"),
    kv("N. Serie",              sr.system_serial       || "N/A"),
    kv("BIOS Fabricant",        sr.bios_manufacturer   || "N/A"),
    kv("BIOS Version",          sr.bios_version        || "N/A"),
    kv("BIOS Date",             sr.bios_date           || "N/A"),
    kv("BIOS Sante",            sr.bios_ok ? "OK" : "Erreur detection"),
    "");

  lines.push(sec("COMPOSANTS MATERIELS"), THIN,
    kv("CPU",                   `${sr.cpu_name}  --  ${sr.cpu_cores} coeurs / ${sr.cpu_threads || "?"} threads @ ${sr.cpu_frequency_ghz || "?"} GHz`),
    kv("CPU Utilisation",       `${sr.cpu_usage_percent?.toFixed(1) || "?"}%`),
    kv("CPU Temperature",       sr.cpu_temperature || "N/A"),
    kv("RAM",                   `${sr.ram_used_gb?.toFixed(1) || "?"} / ${sr.ram_total_gb?.toFixed(0) || "?"} GB (${sr.ram_usage_percent?.toFixed(0) || "?"}%)`),
    kv("RAM Detail",            sr.ram_detail || "N/A"),
    kv("Memoire virtuelle",     `${sr.virtual_memory_total_mb || "?"} MB total  /  ${sr.virtual_memory_available_mb || "?"} MB libre`),
    ...(sr.all_gpus?.length
      ? sr.all_gpus.map(g => kv(`GPU ${g.is_integrated ? "(intégré)" : "(dédié)"}`, `${g.name}  --  VRAM: ${g.vram_mb >= 1024 ? (g.vram_mb / 1024).toFixed(1) + " GB" : g.vram_mb + " MB"}`))
      : [kv("GPU", `${sr.gpu_name || "N/A"}  --  VRAM: ${sr.gpu_vram_mb >= 1024 ? (sr.gpu_vram_mb / 1024).toFixed(1) + " GB" : (sr.gpu_vram_mb || 0) + " MB"}`)]),
    kv("Carte mere",            sr.motherboard || "N/A"),
    kv("Ecrans",                sr.monitors_detail || sr.screen_resolution || "N/A"),
    kv("Plan alimentation",     sr.power_plan || "N/A"),
    "");

  if (sr.storage_items?.length) {
    lines.push(sec("STOCKAGE PHYSIQUE"), THIN);
    for (const s of sr.storage_items) {
      lines.push(kv(s.model || "N/A", `${s.size_gb} GB  --  ${s.media_type}  --  ${s.interface_type}  --  ${s.health}`));
    }
    lines.push("");
  }
  lines.push(sec("ESPACE DISQUE (VOLUMES)"), THIN);
  for (const d of sr.disk_usage || []) {
    lines.push(kv(d.drive, `${d.used_percent.toFixed(0).padStart(3)}%  ${bar(d.used_percent)}  ${d.free_gb.toFixed(1)} GB libres / ${d.total_gb.toFixed(0)} GB`));
  }
  lines.push("");

  lines.push(sec("RESEAU"), THIN,
    kv("Connectivite",          sr.network_ok ? "OK (8.8.8.8 joignable)" : "!!! HORS LIGNE !!!"),
    kv("Adaptateurs actifs",    sr.network_adapters_summary || "N/A"),
  );
  if (sr.open_ports?.length) {
    lines.push(kv("Ports ecoute", sr.open_ports.slice(0, 25).join(", ") + (sr.open_ports.length > 25 ? `  (+${sr.open_ports.length - 25})` : "")));
  }
  lines.push("");

  if (batteries?.length) {
    lines.push(sec("BATTERIE"), THIN);
    for (const b of batteries) {
      lines.push(kv("Nom",               b.name               || "N/A"));
      lines.push(kv("Statut",            b.status             || "N/A"));
      lines.push(kv("Charge",            b.estimated_charge_remaining != null ? `${b.estimated_charge_remaining}%` : "N/A"));
      lines.push(kv("Autonomie estimee", b.estimated_run_time || "N/A"));
      lines.push(kv("Capacite originale", b.design_capacity       != null ? `${b.design_capacity} mWh` : "N/A"));
      lines.push(kv("Capacite actuelle",  b.full_charge_capacity  != null ? `${b.full_charge_capacity} mWh` : "N/A"));
      lines.push(kv("Sante batterie",    b.battery_health_percent != null ? `${b.battery_health_percent.toFixed(0)}%` : "N/A"));
      lines.push(kv("Chimie",            b.chemistry          || "N/A"));
      lines.push(kv("Cycles",            b.cycle_count        != null ? String(b.cycle_count) : "N/A"));
      if (batteries.length > 1) lines.push(THIN);
    }
    lines.push("");
  }

  lines.push(sec("=== INFORMATIONS WINDOWS ==="), "");

  lines.push(sec("SYSTEME WINDOWS"), THIN,
    kv("OS",                    sr.windows_version || "N/A"),
    kv("Uptime",                sr.uptime_hours >= 24 ? `${(sr.uptime_hours / 24).toFixed(1)} jours` : `${sr.uptime_hours?.toFixed(1) || "?"} heures`),
    kv("Activation",            sr.windows_activation || "Inconnu"),
    kv("Type licence",          sr.license_type        || "N/A"),
    kv("Dernier KB (MAJ)",      sr.last_update_days  >= 0 ? `il y a ${sr.last_update_days} jours`  : "Inconnu"),
    kv("Reboot requis",         sr.pending_reboot ? "!!! OUI !!!" : "Non"),
    kv("MAJ en attente",        sr.pending_updates_cached >= 0 ? String(sr.pending_updates_cached) : "N/A"),
    kv("Logiciels installes",   String(sr.installed_software_count || 0)),
    kv("Services",              `${sr.services_running || 0} actifs  /  ${sr.services_stopped || 0} arretes`),
    kv("Prog. demarrage",       String(sr.startup_count || 0)),
    kv("Dossiers Temp",         `${sr.temp_folder_size_mb?.toFixed(0) || "?"} MB`),
    "");

  lines.push(sec("LICENCES & CHIFFREMENT"), THIN,
    kv("Cle Windows",           sr.windows_product_key || "Non disponible"),
    kv("Office",                `${sr.office_name || "N/A"}  --  ${sr.office_product_key || "N/A"}`),
    kv("Type activation Win.", sr.activation_type || "N/A"),
    kv("Type activation Office", sr.office_activation_type || "N/A"),
  );
  if (sr.bitlocker_volumes?.length) {
    for (const bv of sr.bitlocker_volumes) {
      const prot = bv.protection_status === "On" || bv.protection_status === "1" ? "Protege" : "Non protege";
      lines.push(kv(`BitLocker ${bv.drive}`, prot));
      if (bv.recovery_password) lines.push(kv("  Cle recuperation", bv.recovery_password));
    }
  } else { lines.push(kv("BitLocker", "Aucun volume chiffre")); }
  lines.push("");

  lines.push(sec("INTEGRITE SYSTEME"), THIN,
    kv("DISM",                  sr.dism_status || "N/A"),
    kv("SFC",                   sr.sfc_status  || "N/A"),
  );
  if (sr.sfc_details && sr.sfc_details.trim()) {
    lines.push("  Sortie SFC :");
    for (const l of sr.sfc_details.split("\n").slice(0, 15)) lines.push(`    ${l}`);
  }
  if (sr.dism_details && !sr.dism_status?.toLowerCase().includes("sain")) {
    lines.push("  Sortie DISM :");
    for (const l of sr.dism_details.split("\n").slice(0, 15)) lines.push(`    ${l}`);
  }
  lines.push("");

  lines.push(sec("MISES A JOUR — GESTIONNAIRES DE PAQUETS"), THIN);
  if (sr.winget_upgradable?.length) {
    lines.push(`  WinGet : ${sr.winget_upgradable.length} logiciel(s) a mettre a jour`);
    for (const u of sr.winget_upgradable.slice(0, 20)) {
      const name = (u.name || "").padEnd(32).substring(0, 32);
      lines.push(`    - ${name}  ${u.current_version || "?"} -> ${u.available_version || "?"}  [${u.id || ""}]`);
    }
    if (sr.winget_upgradable.length > 20) lines.push(`    ... (+${sr.winget_upgradable.length - 20} autres)`);
  } else {
    lines.push(kv("WinGet", "Aucune mise a jour disponible"));
  }
  if (sr.choco_upgradable?.length) {
    lines.push(`  Chocolatey : ${sr.choco_upgradable.length} paquet(s) a mettre a jour`);
    for (const u of sr.choco_upgradable.slice(0, 12)) lines.push(`    - ${u}`);
    if (sr.choco_upgradable.length > 12) lines.push(`    ... (+${sr.choco_upgradable.length - 12} autres)`);
  } else {
    lines.push(kv("Chocolatey", "Aucune mise a jour disponible"));
  }
  if (sr.scoop_upgradable?.length) {
    lines.push(`  Scoop : ${sr.scoop_upgradable.length} paquet(s) a mettre a jour`);
    for (const u of sr.scoop_upgradable.slice(0, 12)) lines.push(`    - ${u}`);
    if (sr.scoop_upgradable.length > 12) lines.push(`    ... (+${sr.scoop_upgradable.length - 12} autres)`);
  } else {
    lines.push(kv("Scoop", "Aucune mise a jour disponible"));
  }
  if (sr.windows_updates_pending?.length) {
    lines.push(`  Windows Update : ${sr.windows_updates_pending.length} KB en attente`);
    for (const u of sr.windows_updates_pending.slice(0, 15)) lines.push(`    - ${u}`);
    if (sr.windows_updates_pending.length > 15) lines.push(`    ... (+${sr.windows_updates_pending.length - 15} autres)`);
  } else {
    lines.push(kv("Windows Update", "Aucune mise a jour en attente"));
  }
  lines.push("");

  if (sr.top_cpu?.length || sr.top_ram?.length) {
    lines.push(sec("TOP PROCESSUS (snapshot)"), THIN);
    if (sr.top_cpu?.length) {
      lines.push("  CPU :");
      for (const p of sr.top_cpu) lines.push(`    [${String(p.pid).padEnd(6)}] ${oneLine(p.name).padEnd(32)} ${p.value}s CPU`);
    }
    if (sr.top_ram?.length) {
      lines.push("  RAM :");
      for (const p of sr.top_ram) lines.push(`    [${String(p.pid).padEnd(6)}] ${oneLine(p.name).padEnd(32)} ${p.value} MB`);
    }
    lines.push("");
  }

  lines.push(sec("=== PROBLEMES & SECURITE ==="), "");

  if (scanProblems?.length) {
    lines.push(sec(`!!! PROBLEMES DETECTES : ${scanProblems.length} !!!`), THIN);
    for (const p of scanProblems) lines.push(`  !! ${p}`);
    lines.push("");
  } else {
    lines.push(sec("BILAN"), THIN, "  Aucun probleme critique detecte.", "");
  }

  lines.push(sec("SECURITE"), THIN,
    kv("Pare-feu Windows",      sr.firewall_enabled ? "Actif" : "!!! DESACTIVE !!!"),
    kv("Windows Defender",      sr.defender_enabled  ? "Actif" : "!!! INACTIF !!!"),
    kv("Antivirus tiers",       sr.antivirus_installed || "Aucun (Defender)"),
    kv("Defs Defender",         sr.defender_definition_age_days >= 0 ? `${sr.defender_definition_age_days} jours` : "N/A"),
    kv("Dernier BSOD",          sr.last_bsod || "Aucun BSOD recent"),
    kv("Connexion Internet",    sr.network_ok ? "OK" : "!!! HORS LIGNE !!!"),
    "");

  lines.push(sec("SECURITE AVANCEE"), THIN,
    kv("TPM",                   sr.tpm_present ? (sr.tpm_enabled ? `Present & Actif (v${sr.tpm_version || "?"})` : "Present (desactive)") : "Absent"),
    kv("Secure Boot",           sr.secure_boot ? "Actif" : "Desactive"),
    kv("UAC",                   sr.uac_level || "Inconnu"),
    kv("RDP (Bureau a dist.)",  sr.rdp_enabled  ? "!!! Actif (risque) !!!" : "Desactive"),
    kv("SMBv1",                 sr.smbv1_enabled ? "!!! Actif (risque) !!!" : "Desactive"),
    kv("Abonnements WMI",       String(sr.wmi_subscriptions ?? 0)),
    kv("Compte Invite",         sr.guest_enabled ? "!!! Actif !!!" : "Desactive"),
    kv("Admins locaux",         sr.local_admins?.join(", ") || "N/A"),
    kv("Point restauration",    sr.last_restore_point || "N/A"),
  );
  if (sr.wmi_subscription_details?.length) {
    lines.push("  Details abonnements WMI :");
    for (const sub of sr.wmi_subscription_details) {
      lines.push(`    [${oneLine(sub.consumer_type)}] ${oneLine(sub.name || "(sans nom)")}`);
      lines.push(`      Chemin : ${oneLine(sub.path)}`);
    }
  }
  lines.push("");

  if (sr.suspicious_processes?.length) {
    lines.push(sec(`PROCESSUS HORS CHEMINS SECURISES (${sr.suspicious_processes.length})`), THIN);
    for (const p of sr.suspicious_processes) {
      lines.push(`  [PID ${p.pid}] ${oneLine(p.name)}`);
      lines.push(`           Raison : ${oneLine(p.reason)}`);
      lines.push(`           Chemin : ${oneLine(p.path)}`);
    }
    lines.push("");
  }

  if (sr.suspicious_services?.length) {
    lines.push(sec(`SERVICES TIERS ACTIFS (${sr.suspicious_services.length})`), THIN);
    for (const s of sr.suspicious_services.slice(0, 15)) {
      lines.push(`  ${oneLine(s.name)}  (${oneLine(s.state)})`);
      lines.push(`    ${oneLine(s.display_name)}`);
      lines.push(`    ${oneLine(s.path)}`);
    }
    lines.push("");
  }

  if (sr.autorun_entries?.length) {
    lines.push(sec(`ENTREES AUTORUN TIERS (${sr.autorun_entries.length})`), THIN);
    for (const a of sr.autorun_entries.slice(0, 25)) {
      lines.push(`  ${oneLine(a.name).padEnd(36)} [${oneLine(a.location)}]`);
      lines.push(`    Exec   : ${oneLine(a.path)}`);
      lines.push(`    RegKey : ${oneLine(fullRegPath(a.location, a.name))}`);
    }
    lines.push("");
  }

  if (sr.susp_tasks?.length) {
    lines.push(sec(`TACHES PLANIFIEES SUSPECTES (${sr.susp_tasks_count})`), THIN);
    for (const t of sr.susp_tasks) {
      lines.push(`  ${oneLine(t.name)}  (${oneLine(t.path)})`);
      lines.push(`    Executable : ${oneLine(t.exec)}`);
    }
    lines.push("");
  }

  if (sr.recent_errors?.length) {
    lines.push(sec(`ERREURS RECENTES (${sr.recent_errors.length} dans 48h)`), THIN);
    for (const e of sr.recent_errors.slice(0, 10)) {
      lines.push(`  [${(e.level || "?").padEnd(8)}] ${e.time}  --  ${e.source}`);
      const msg = (e.message || "").replace(/\r?\n/g, " ").trim();
      lines.push(`    ${msg.substring(0, 100)}${msg.length > 100 ? "..." : ""}`);
    }
    lines.push("");
  }

  if (sr.scan_errors?.length) {
    lines.push(sec(`ERREURS DE SCAN (${sr.scan_errors.length})`), THIN);
    // Un \n brut dans l'erreur (exception PowerShell/WMI multi-ligne) injectait
    // des lignes non préfixées "!!" dans le rapport, pouvant visuellement usurper
    // un en-tête de section — même garde déjà appliquée juste au-dessus pour
    // recent_errors.message, manquait ici pour ce champ jumeau.
    for (const e of sr.scan_errors) lines.push(`  !! ${e.replace(/\r?\n/g, " ")}`);
    lines.push("");
  }

  lines.push(SEP, sec("FIN DU RAPPORT"), SEP);

  try {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const filePath = await save({ defaultPath: "scan_total.txt", filters: [{ name: "TXT", extensions: ["txt"] }] });
    if (!filePath) return;
    await invoke("save_content_to_path", { path: filePath, content: "\uFEFF" + lines.join("\n") });
    useNotificationStore().success("Scan exporte (.txt)", filePath);
    await invokeRaw("open_path", { path: filePath }).catch(() => {});
  } catch (e: unknown) { useNotificationStore().error("Erreur export", String(e)); }
}
