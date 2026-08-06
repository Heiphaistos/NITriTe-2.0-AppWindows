import { invoke, invokeRaw, useNotificationStore, fullRegPath, mdCell, type Solution } from "./scanExportHelpers";
import type { ScanResult, BatteryDetailed } from "@/types/diagnostic";

export async function exportScanMd(
  scanResult: ScanResult | null,
  scanProblems: string[],
  batteries: BatteryDetailed[],
  scanSolutions: Solution[]
) {
  if (!scanResult) return;
  const sr = scanResult;
  const now = new Date().toLocaleString();
  const row = (...cells: string[]) => `| ${cells.map(mdCell).join(" | ")} |`;
  const head = (...cols: string[]) => [row(...cols), `|${cols.map(()=>"---").join("|")}|`];

  const L: string[] = [
    `# 🔍 Rapport Scan Total — NiTriTe`,
    ``,
    `> Généré le **${now}**`,
    ``,
    `---`,
    ``,
    `## 🖥️ Partie 1 — Composants du PC`,
    ``,
    `### 🔧 Identité Système & BIOS`,
    ``,
    ...head("Propriété","Valeur"),
    row("Fabricant",         sr.system_manufacturer||"N/A"),
    row("Modèle",            sr.system_model||"N/A"),
    row("N° Série",          sr.system_serial||"N/A"),
    row("BIOS Fabricant",    sr.bios_manufacturer||"N/A"),
    row("BIOS Version",      sr.bios_version||"N/A"),
    row("BIOS Date",         sr.bios_date||"N/A"),
    row("BIOS Santé",        sr.bios_ok ? "✅ OK" : "⚠ Erreur détection"),
    ``,
    `### 🔩 Composants Matériels`,
    ``,
    ...head("Composant","Détail"),
    row("CPU",               `${sr.cpu_name} — ${sr.cpu_cores} cœurs / ${sr.cpu_threads||"?"}T @ ${sr.cpu_frequency_ghz||"?"}GHz`),
    row("CPU Utilisation",   `${sr.cpu_usage_percent?.toFixed(1)||"?"}%`),
    row("CPU Température",   sr.cpu_temperature||"N/A"),
    row("RAM",               `${sr.ram_used_gb?.toFixed(1)} / ${sr.ram_total_gb?.toFixed(0)} GB (${sr.ram_usage_percent?.toFixed(0)}%)`),
    row("Config RAM",        sr.ram_detail||"N/A"),
    row("Mémoire virtuelle", `${sr.virtual_memory_total_mb||"?"} MB total / ${sr.virtual_memory_available_mb||"?"} MB libre`),
    ...(sr.all_gpus?.length
      ? sr.all_gpus.map(g => row(`GPU ${g.is_integrated?"(intégré)":"(dédié)"}`, `${g.name} — VRAM: ${g.vram_mb>=1024?(g.vram_mb/1024).toFixed(1)+"GB":g.vram_mb+"MB"}`))
      : [row("GPU", `${sr.gpu_name||"N/A"} — VRAM: ${sr.gpu_vram_mb>=1024?(sr.gpu_vram_mb/1024).toFixed(1)+"GB":sr.gpu_vram_mb+"MB"}`)]),
    row("Carte mère",        sr.motherboard||"N/A"),
    row("Écrans",            sr.monitors_detail||sr.screen_resolution||"N/A"),
    row("Plan d'alim.",      sr.power_plan||"N/A"),
    ``,
  ];

  if (sr.storage_items?.length) {
    L.push(`### 💾 Stockage Physique`, ``);
    L.push(...head("Modèle","Type","Interface","Taille","Santé"));
    for (const s of sr.storage_items) L.push(row(s.model||"—", s.media_type, s.interface_type, s.size_gb+" GB", s.health));
    L.push(``);
  }

  L.push(`### 💽 Espace Disque (Volumes)`, ``);
  L.push(...head("Lecteur","Utilisé","Libre","Total"));
  for (const d of sr.disk_usage||[]) L.push(row(d.drive, d.used_percent.toFixed(0)+"%", d.free_gb.toFixed(1)+" GB", d.total_gb.toFixed(0)+" GB"));
  L.push(``);

  L.push(`### 🌐 Réseau`, ``);
  L.push(...head("Indicateur","Valeur"),
    row("Connectivité",       sr.network_ok?"✅ OK (8.8.8.8)":"❌ Hors ligne"),
    row("Adaptateurs actifs", sr.network_adapters_summary||"N/A"),
    row("Ports en écoute",    sr.open_ports?.length ? sr.open_ports.slice(0,30).join(", ")+(sr.open_ports.length>30?" …":"") : "Aucun"),
    ``);

  if (batteries?.length) {
    L.push(`### 🔋 Batterie`, ``);
    L.push(...head("Propriété","Valeur"));
    for (const b of batteries) {
      L.push(row("Nom",               b.name               || "N/A"));
      L.push(row("Statut",            b.status             || "N/A"));
      L.push(row("Charge",            b.estimated_charge_remaining != null ? `${b.estimated_charge_remaining}%` : "N/A"));
      L.push(row("Autonomie estimée", b.estimated_run_time || "N/A"));
      L.push(row("Capacité originale",  b.design_capacity      != null ? `${b.design_capacity} mWh` : "N/A"));
      L.push(row("Capacité actuelle",   b.full_charge_capacity != null ? `${b.full_charge_capacity} mWh` : "N/A"));
      L.push(row("Santé batterie",    b.battery_health_percent != null ? `${b.battery_health_percent.toFixed(0)}%` : "N/A"));
      L.push(row("Chimie",            b.chemistry          || "N/A"));
      L.push(row("Cycles",            b.cycle_count        != null ? String(b.cycle_count) : "N/A"));
      if (batteries.length > 1) L.push(``, `---`, ``);
    }
    L.push(``);
  }

  L.push(`---`, ``, `## 🪟 Partie 2 — Informations Windows`, ``);

  L.push(`### 🪟 Système Windows`, ``);
  L.push(...head("Indicateur","Valeur"),
    row("Version OS",               sr.windows_version||"N/A"),
    row("Uptime",                   sr.uptime_hours>=24?(sr.uptime_hours/24).toFixed(1)+" jours":sr.uptime_hours?.toFixed(1)+" heures"),
    row("Activation",               sr.windows_activation||"Inconnu"),
    row("Type licence",             sr.license_type||"N/A"),
    row("Dernier KB",               sr.last_update_days>=0?"il y a "+sr.last_update_days+" jours":"N/A"),
    row("Redémarrage requis",       sr.pending_reboot?"⚠ Oui":"Non"),
    row("MAJ en attente (cache)",   sr.pending_updates_cached>=0?String(sr.pending_updates_cached):"N/A"),
    row("Logiciels installés",      String(sr.installed_software_count||0)),
    row("Services",                 `${sr.services_running||0} actifs / ${sr.services_stopped||0} arrêtés`),
    row("Prog. démarrage",          String(sr.startup_count||0)),
    row("Fichiers TEMP",            sr.temp_folder_size_mb>=1024?(sr.temp_folder_size_mb/1024).toFixed(1)+"GB":sr.temp_folder_size_mb?.toFixed(0)+" MB"),
    row("Dernier BSOD",             sr.last_bsod||"Aucun BSOD récent"),
    ``);

  L.push(`### 🔑 Licences & Chiffrement`, ``);
  L.push(...head("Propriété","Valeur"),
    row("Clé Windows",              sr.windows_product_key ? `\`${sr.windows_product_key}\`` : "Non disponible"),
    row(sr.office_name||"Office",   sr.office_product_key ? `\`${sr.office_product_key}\`` : "N/A"),
    row("Type activation Windows",  sr.activation_type || "N/A"),
    row("Type activation Office",   sr.office_activation_type || "N/A"),
  );
  if (sr.bitlocker_volumes?.length) {
    for (const bv of sr.bitlocker_volumes) {
      const prot = bv.protection_status==="On"||bv.protection_status==="1";
      L.push(row(`BitLocker ${bv.drive}`, prot?"🔒 Protégé":"🔓 Non protégé"));
      if (bv.recovery_password) L.push(row("  Clé récupération", `\`${bv.recovery_password}\``));
    }
  } else { L.push(row("BitLocker","Non configuré")); }
  L.push(``);

  L.push(`### 🛡️ Intégrité Système (DISM / SFC)`, ``);
  L.push(...head("Outil","Statut"),
    row("DISM", sr.dism_status||"N/A"),
    row("SFC",  sr.sfc_status||"N/A"),
    ``);
  if (sr.sfc_details?.trim()) {
    L.push(`**Sortie SFC :**`, ``, "```", sr.sfc_details.split("\n").slice(0,20).join("\n"), "```", ``);
  }
  if (sr.dism_details && !sr.dism_status?.toLowerCase().includes("sain")) {
    L.push(`**Sortie DISM :**`, ``, "```", sr.dism_details.split("\n").slice(0,20).join("\n"), "```", ``);
  }

  L.push(`### 📦 Mises à Jour — Gestionnaires de Paquets`, ``);
  L.push(...head("Gestionnaire","Statut"),
    row("WinGet",         sr.winget_upgradable?.length ? `⚠ ${sr.winget_upgradable.length} logiciel(s) à mettre à jour` : "✅ Aucune mise à jour"),
    row("Chocolatey",     sr.choco_upgradable?.length  ? `⚠ ${sr.choco_upgradable.length} paquet(s) à mettre à jour`  : "✅ Aucune mise à jour"),
    row("Scoop",          sr.scoop_upgradable?.length  ? `⚠ ${sr.scoop_upgradable.length} paquet(s) à mettre à jour`  : "✅ Aucune mise à jour"),
    row("Windows Update", sr.windows_updates_pending?.length ? `⚠ ${sr.windows_updates_pending.length} KB en attente` : sr.pending_updates_cached > 0 ? `⚠ ${sr.pending_updates_cached} (cache)` : "✅ Aucune mise à jour"),
    ``);
  L.push(`**WinGet**`, ``);
  if (sr.winget_upgradable?.length) {
    L.push(...head("Nom","ID","Version actuelle","Disponible"));
    for (const u of sr.winget_upgradable) L.push(row(u.name, u.id, u.current_version||"?", u.available_version||"?"));
  } else { L.push(`*Aucune mise à jour disponible*`); }
  L.push(``);
  L.push(`**Chocolatey**`, ``);
  if (sr.choco_upgradable?.length) {
    for (const u of sr.choco_upgradable) L.push(`- ${u}`);
  } else { L.push(`*Aucune mise à jour disponible*`); }
  L.push(``);
  L.push(`**Scoop**`, ``);
  if (sr.scoop_upgradable?.length) {
    for (const u of sr.scoop_upgradable) L.push(`- ${u}`);
  } else { L.push(`*Aucune mise à jour disponible*`); }
  L.push(``);
  L.push(`**Windows Update**`, ``);
  if (sr.windows_updates_pending?.length) {
    for (const u of sr.windows_updates_pending.slice(0, 20)) L.push(`- ${u}`);
    if (sr.windows_updates_pending.length > 20) L.push(`- *(+${sr.windows_updates_pending.length - 20} autres)*`);
  } else { L.push(`*Aucune mise à jour en attente*`); }
  L.push(``);

  if (sr.top_cpu?.length || sr.top_ram?.length) {
    L.push(`### 📊 Top 5 Processus (snapshot)`, ``);
    if (sr.top_cpu?.length) {
      L.push(`**CPU :**`, ``,...head("PID","Processus","CPU (s)"));
      for (const p of sr.top_cpu) L.push(row(String(p.pid), p.name, String(p.value)));
      L.push(``);
    }
    if (sr.top_ram?.length) {
      L.push(`**RAM :**`, ``,...head("PID","Processus","RAM (MB)"));
      for (const p of sr.top_ram) L.push(row(String(p.pid), p.name, String(p.value)));
      L.push(``);
    }
  }

  L.push(`---`, ``, `## 🚨 Partie 3 — Problèmes & Sécurité`, ``);

  if (scanProblems.length) {
    L.push(`### ⚠ Problèmes Détectés (${scanProblems.length})`, ``);
    for (const p of scanProblems) L.push(`- ⚠ ${p}`);
    L.push(``);
  } else {
    L.push(`> ✅ **Aucun problème critique détecté**`, ``);
  }

  if (scanSolutions.length) {
    L.push(`### 💡 Solutions Recommandées`, ``);
    L.push(...head("Problème","Action","Sévérité"));
    for (const s of scanSolutions) {
      const sev = s.severity==="critical"?"🔴 Critique":s.severity==="warning"?"🟡 Attention":"🔵 Info";
      L.push(row(s.problem, s.action, sev));
    }
    L.push(``);
  }

  L.push(`### 🔒 Sécurité`, ``);
  L.push(...head("Indicateur","Valeur"),
    row("Pare-feu Windows",   sr.firewall_enabled?"✅ Activé":"❌ DÉSACTIVÉ"),
    row("Windows Defender",   sr.defender_enabled?"✅ Actif":"❌ INACTIF"),
    row("Antivirus tiers",    sr.antivirus_installed||"Aucun (Defender)"),
    row("Defs Defender",      sr.defender_definition_age_days>=0?sr.defender_definition_age_days+" jours":"N/A"),
    row("Connectivité",       sr.network_ok?"✅ OK":"❌ Hors ligne"),
    row("Dernier BSOD",       sr.last_bsod||"Aucun"),
    ``);

  L.push(`### 🛡️ Sécurité Avancée`, ``);
  L.push(...head("Indicateur","Valeur"),
    row("TPM",                sr.tpm_present?(sr.tpm_enabled?`✅ Présent & Actif (v${sr.tpm_version||"?"})`:"⚠ Présent (désactivé)"):"❌ Absent"),
    row("Secure Boot",        sr.secure_boot?"✅ Activé":"⚠ Désactivé"),
    row("Niveau UAC",         sr.uac_level||"Inconnu"),
    row("RDP",                sr.rdp_enabled?"⚠ Activé":"✅ Désactivé"),
    row("SMBv1",              sr.smbv1_enabled?"❌ Activé (risque)":"✅ Désactivé"),
    row("Abonnements WMI",    String(sr.wmi_subscriptions??0)),
    row("Compte Invité",      sr.guest_enabled?"⚠ Activé":"✅ Désactivé"),
    row("Admins locaux",      sr.local_admins?.join(", ")||"N/A"),
    row("Dernier pt. restauration", sr.last_restore_point||"N/A"),
    ``);

  if (sr.wmi_subscriptions > 0) {
    L.push(`> ⚠ **${sr.wmi_subscriptions} abonnement(s) WMI suspects** — indicateur de malware potentiel`, ``);
    if (sr.wmi_subscription_details?.length) {
      L.push(...head("Nom","Type","Chemin"));
      for (const s of sr.wmi_subscription_details) L.push(row(s.name||"(sans nom)", s.consumer_type, s.path));
      L.push(``);
    }
    L.push(`**Commande suppression :** \`Get-WmiObject -Namespace root\\subscription -Class __EventConsumer | Remove-WmiObject\``, ``);
  }

  if (sr.suspicious_processes?.length) {
    L.push(`### 🔍 Processus Hors Chemins Sécurisés (${sr.suspicious_processes.length})`, ``);
    L.push(...head("PID","Nom","Raison","Chemin"));
    for (const p of sr.suspicious_processes) L.push(row(String(p.pid), p.name, p.reason, p.path));
    L.push(``);
  }

  if (sr.suspicious_services?.length) {
    L.push(`### ⚙️ Services Tiers Actifs (${sr.suspicious_services.length})`, ``);
    L.push(...head("Nom","Affichage","État","Chemin"));
    for (const s of (sr.suspicious_services||[]).slice(0,20)) L.push(row(s.name, s.display_name, s.state, s.path));
    L.push(``);
  }

  if (sr.autorun_entries?.length) {
    L.push(`### 🚀 Entrées Autorun Tiers (${sr.autorun_entries.length})`, ``);
    L.push(...head("Nom","Clé Registre","Exécutable"));
    for (const a of (sr.autorun_entries||[]).slice(0,30)) L.push(row(a.name, fullRegPath(a.location, a.name), a.path));
    L.push(``);
  }

  if (sr.susp_tasks?.length) {
    L.push(`### 📅 Tâches Planifiées Suspectes (${sr.susp_tasks_count})`, ``);
    L.push(...head("Tâche","Chemin","Exécutable"));
    for (const t of sr.susp_tasks) L.push(row(t.name, t.path, t.exec));
    L.push(``);
  }

  if (sr.recent_errors?.length) {
    L.push(`### 🔴 Erreurs Récentes (${sr.recent_errors.length} — 48h)`, ``);
    L.push(...head("Heure","Niveau","Source","Message"));
    for (const ev of (sr.recent_errors||[]).slice(0,20))
      L.push(row(ev.time, ev.level||"?", ev.source, (ev.message||"").substring(0,120)));
    L.push(``);
  }

  if (sr.scan_errors?.length) {
    L.push(`### ⚙️ Erreurs de Scan (${sr.scan_errors.length})`, ``);
    // Historiquement `for (const e of ...)` masquait la fonction d'échappement
    // locale (alors nommée "e"), laissant sr.scan_errors (texte d'erreur brut
    // PowerShell/WMI, potentiellement porteur de backtick/pipe) passer
    // complètement non échappé, cassant la structure du span de code inline en
    // Markdown — corrigé en renommant la variable de boucle et en appelant
    // mdCell() explicitement.
    for (const err of sr.scan_errors) L.push(`- \`${mdCell(err)}\``);
    L.push(``);
  }

  L.push(`---`, `*Rapport complet généré par **NiTriTe v${__APP_VERSION__}***`);

  try {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const filePath = await save({ defaultPath: "scan_total.md", filters: [{ name: "Markdown", extensions: ["md"] }] });
    if (!filePath) return;
    await invoke("save_content_to_path", { path: filePath, content: L.join("\n") });
    useNotificationStore().success("Scan exporté (Markdown)", filePath);
    await invokeRaw("open_path", { path: filePath }).catch(() => {});
  } catch (e: unknown) { useNotificationStore().error("Erreur export", String(e)); }
}
