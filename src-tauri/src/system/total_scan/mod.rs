use serde::Serialize;
use tauri::Emitter;
#[cfg(target_os = "windows")]
use crate::error::NiTriTeError;
use super::extended_info::{get_bios_info, get_battery_extended};
use super::scan_supplement::{BitlockerVolume, StorageItem, collect_scan_supplement};
use super::scan_extra::{TopProcess, SuspTask, WmiSubscriptionDetail, collect_scan_extra};

pub mod system_checks;
pub mod security_checks;
pub mod hw_extended;

use system_checks::*;
use security_checks::*;
use hw_extended::*;

// === Types publics ===

#[derive(Debug, Clone, Serialize, Default)]
pub struct ScanResult {
    pub bios_ok: bool,
    pub bios_info: Option<String>,
    pub battery_present: bool,
    pub battery_health: f64,
    pub battery_cycles: i64,
    pub suspicious_processes: Vec<SuspiciousProcess>,
    pub disk_usage: Vec<DiskUsage>,
    pub winget_upgradable: Vec<WingetUpgrade>,
    pub choco_upgradable: Vec<String>,
    pub dism_status: String,
    pub dism_details: String,
    pub sfc_status: String,
    pub sfc_details: String,
    pub scan_errors: Vec<String>,
    pub uptime_hours: f64,
    pub cpu_name: String,
    pub cpu_cores: u32,
    pub cpu_usage_percent: f32,
    pub ram_total_gb: f64,
    pub ram_used_gb: f64,
    pub ram_usage_percent: f32,
    pub windows_version: String,
    pub windows_activation: String,
    pub firewall_enabled: bool,
    pub defender_enabled: bool,
    pub startup_count: usize,
    pub pending_reboot: bool,
    pub recent_errors: Vec<EventEntry>,
    pub network_ok: bool,
    pub open_ports: Vec<u16>,
    pub antivirus_installed: String,
    pub defender_definition_age_days: i64,
    pub last_bsod: String,
    pub last_update_days: i64,
    pub temp_folder_size_mb: f64,
    pub suspicious_services: Vec<SuspiciousService>,
    pub autorun_entries: Vec<AutorunEntry>,
    pub virtual_memory_total_mb: u64,
    pub virtual_memory_available_mb: u64,
    pub gpu_name: String,
    pub gpu_vram_mb: u64,
    pub all_gpus: Vec<GpuScanItem>,
    pub screen_resolution: String,
    pub power_plan: String,
    pub installed_software_count: u32,
    pub services_running: u32,
    pub services_stopped: u32,
    pub network_adapters_summary: String,
    pub cpu_temperature: String,
    pub windows_product_key: String,
    pub office_product_key: String,
    pub office_name: String,
    pub bitlocker_volumes: Vec<BitlockerVolume>,
    pub motherboard: String,
    pub ram_detail: String,
    pub ram_slots: Vec<String>,
    pub cpu_threads: u32,
    pub cpu_frequency_ghz: f64,
    pub cpu_socket: String,
    pub cpu_l3_mb: u32,
    pub storage_items: Vec<StorageItem>,
    pub monitors_detail: String,
    pub tpm_present: bool,
    pub tpm_enabled: bool,
    pub tpm_version: String,
    pub secure_boot: bool,
    pub uac_level: String,
    pub rdp_enabled: bool,
    pub smbv1_enabled: bool,
    pub wmi_subscriptions: u32,
    pub wmi_subscription_details: Vec<WmiSubscriptionDetail>,
    pub local_admins: Vec<String>,
    pub guest_enabled: bool,
    pub system_manufacturer: String,
    pub system_model: String,
    pub system_serial: String,
    pub bios_manufacturer: String,
    pub bios_version: String,
    pub bios_date: String,
    pub license_type: String,
    pub last_restore_point: String,
    pub pending_updates_cached: i32,
    pub top_cpu: Vec<TopProcess>,
    pub top_ram: Vec<TopProcess>,
    pub susp_tasks_count: u32,
    pub susp_tasks: Vec<SuspTask>,
    pub windows_updates_pending: Vec<String>,
    pub scoop_upgradable: Vec<String>,
    pub activation_type: String,
    pub office_activation_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuScanItem {
    pub name: String,
    pub vram_mb: u64,
    pub is_integrated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WingetUpgrade {
    pub name: String,
    pub id: String,
    pub current_version: String,
    pub available_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuspiciousService {
    pub name: String,
    pub display_name: String,
    pub state: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutorunEntry {
    pub name: String,
    pub path: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuspiciousProcess {
    pub name: String,
    pub pid: u32,
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskUsage {
    pub drive: String,
    pub total_gb: f64,
    pub free_gb: f64,
    pub used_percent: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventEntry {
    pub time: String,
    pub source: String,
    pub message: String,
    pub level: String,
}

#[derive(Debug, Clone, Serialize)]
struct ScanProgressEvent {
    step: String,
    percent: u32,
}

// === Main ===

pub async fn run_total_scan(window: tauri::Window) -> Result<ScanResult, NiTriTeError> {
    let mut result = ScanResult {
        // Valeurs non-Default : -1 signifie "non mesuré" (distinct de 0)
        battery_cycles: -1,
        defender_definition_age_days: -1,
        last_update_days: -1,
        pending_updates_cached: -1,
        ..ScanResult::default()
    };

    // 1. BIOS + Battery (0→10%)
    emit_progress(&window, "BIOS & Batterie...", 5);
    match tokio::task::spawn_blocking(get_bios_info).await {
        Ok(Ok(info)) => {
            result.bios_ok = true;
            result.bios_info = Some(format!("{} {} ({})", info.manufacturer, info.version, info.release_date));
        }
        Ok(Err(e)) => result.scan_errors.push(format!("BIOS: {}", e)),
        Err(_) => {}
    }
    if let Ok(Ok(Some(bat))) = tokio::task::spawn_blocking(get_battery_extended).await {
        result.battery_present = true;
        result.battery_health = bat.health_percent;
        result.battery_cycles = bat.estimated_runtime_minutes;
    }

    // 2. Infos système (CPU, RAM, OS, uptime) (10→20%)
    emit_progress(&window, "Informations système...", 10);
    let sys_info = tokio::task::spawn_blocking(collect_system_summary).await.unwrap_or_default();
    result.uptime_hours = sys_info.uptime_hours;
    result.cpu_name = sys_info.cpu_name;
    result.cpu_cores = sys_info.cpu_cores;
    result.cpu_usage_percent = sys_info.cpu_usage;
    result.ram_total_gb = sys_info.ram_total;
    result.ram_used_gb = sys_info.ram_used;
    result.ram_usage_percent = sys_info.ram_pct;
    result.windows_version = sys_info.win_version;

    // 3. Activation Windows (20→25%)
    emit_progress(&window, "Vérification licence Windows...", 20);
    result.windows_activation = tokio::task::spawn_blocking(check_windows_activation)
        .await.unwrap_or_else(|_| "Inconnu".to_string());

    // 4. Sécurité : Firewall + Defender (25→35%)
    emit_progress(&window, "Sécurité — Firewall & Defender...", 25);
    let (fw, def) = tokio::task::spawn_blocking(check_security).await.unwrap_or((false, false));
    result.firewall_enabled = fw;
    result.defender_enabled = def;

    // 5. Processus suspects (35→50%)
    emit_progress(&window, "Analyse des processus...", 35);
    result.suspicious_processes = tokio::task::spawn_blocking(scan_suspicious_processes).await.unwrap_or_default();

    // 6. Espace disque (50→60%)
    emit_progress(&window, "Vérification espace disque...", 50);
    result.disk_usage = tokio::task::spawn_blocking(get_disk_usage).await.unwrap_or_default();

    // 7. Démarrage + Reboot pending (60→65%)
    emit_progress(&window, "Programmes au démarrage...", 60);
    let (sc, pr) = tokio::task::spawn_blocking(check_startup_and_reboot).await.unwrap_or((0, false));
    result.startup_count = sc;
    result.pending_reboot = pr;

    // 8. Événements d'erreur récents (65→70%)
    emit_progress(&window, "Journal d'événements...", 65);
    result.recent_errors = tokio::task::spawn_blocking(get_recent_errors).await.unwrap_or_default();

    // 9. WinGet upgrades (70→78%)
    emit_progress(&window, "Mises à jour WinGet...", 70);
    result.winget_upgradable = tokio::task::spawn_blocking(list_winget_upgradable).await.unwrap_or_default();

    // 10. Chocolatey (78→82%)
    emit_progress(&window, "Vérification Chocolatey...", 78);
    if is_chocolatey_installed() {
        result.choco_upgradable = tokio::task::spawn_blocking(list_choco_upgradable).await.unwrap_or_default();
    }

    // 10.5. Scoop (80→82%)
    emit_progress(&window, "Vérification Scoop...", 80);
    if is_scoop_installed() {
        result.scoop_upgradable = tokio::task::spawn_blocking(list_scoop_upgradable).await.unwrap_or_default();
    }

    // 11. Connectivité réseau (82→86%)
    emit_progress(&window, "Test connectivité réseau...", 82);
    result.network_ok = tokio::task::spawn_blocking(test_network).await.unwrap_or(false);

    // 12. Ports ouverts (86→88%)
    emit_progress(&window, "Vérification ports...", 86);
    result.open_ports = tokio::task::spawn_blocking(get_listening_ports).await.unwrap_or_default();

    // 13. DISM /CheckHealth (88→94%)
    emit_progress(&window, "Intégrité système (DISM)...", 88);
    let (dism_s, dism_d) = tokio::task::spawn_blocking(run_dism_check).await.unwrap_or_else(|_| ("Erreur".to_string(), String::new()));
    result.dism_status = dism_s;
    result.dism_details = dism_d;

    // 14. SFC /verifyonly (88→93%)
    emit_progress(&window, "Vérification SFC...", 88);
    let (sfc_s, sfc_d) = tokio::task::spawn_blocking(run_sfc_verify).await.unwrap_or_else(|_| ("Erreur".to_string(), String::new()));
    result.sfc_status = sfc_s;
    result.sfc_details = sfc_d;

    // 15. Antivirus + Defender defs + BSOD + KB age + Temp size (93→97%)
    emit_progress(&window, "Antivirus, services & sécurité avancée...", 93);
    let adv = tokio::task::spawn_blocking(collect_advanced_security).await.unwrap_or_default();
    result.antivirus_installed = adv.antivirus;
    result.defender_definition_age_days = adv.def_age_days;
    result.last_bsod = adv.last_bsod;
    result.last_update_days = adv.last_update_days;
    result.temp_folder_size_mb = adv.temp_size_mb;
    result.virtual_memory_total_mb = adv.vmem_total_mb;
    result.virtual_memory_available_mb = adv.vmem_available_mb;

    // 16. Services suspects + Autoruns (94→97%)
    emit_progress(&window, "Services & autoruns suspects...", 94);
    result.suspicious_services = tokio::task::spawn_blocking(scan_suspicious_services).await.unwrap_or_default();
    result.autorun_entries = tokio::task::spawn_blocking(scan_autoruns).await.unwrap_or_default();

    // 17. Matériel étendu (97→100%)
    emit_progress(&window, "Matériel & logiciels...", 97);
    let hw = tokio::task::spawn_blocking(collect_hw_extended).await.unwrap_or_default();
    result.gpu_name = hw.gpu_name;
    result.gpu_vram_mb = hw.gpu_vram_mb;
    result.all_gpus = hw.all_gpus;
    result.screen_resolution = hw.screen_resolution;
    result.power_plan = hw.power_plan;
    result.installed_software_count = hw.installed_software_count;
    result.services_running = hw.services_running;
    result.services_stopped = hw.services_stopped;
    result.network_adapters_summary = hw.network_adapters_summary;
    result.cpu_temperature = hw.cpu_temperature;

    // 18. Licences, BitLocker, composants (97→100%)
    emit_progress(&window, "Licences, BitLocker & composants...", 97);
    let sup = tokio::task::spawn_blocking(collect_scan_supplement).await.unwrap_or_default();
    result.windows_product_key = sup.windows_product_key;
    result.office_product_key = sup.office_product_key;
    result.office_name = sup.office_name;
    result.bitlocker_volumes = sup.bitlocker_volumes;
    result.motherboard = sup.motherboard;
    result.ram_detail = sup.ram_detail;
    result.ram_slots = sup.ram_slots;
    result.cpu_threads = sup.cpu_threads;
    result.cpu_frequency_ghz = sup.cpu_frequency_ghz;
    result.cpu_socket = sup.cpu_socket;
    result.cpu_l3_mb = sup.cpu_l3_mb;
    result.storage_items = sup.storage_items;
    result.monitors_detail = sup.monitors_detail;

    // 19. TPM, Secure Boot, UAC, RDP, SMBv1, admins, BIOS, processus, tâches (97→100%)
    emit_progress(&window, "Sécurité avancée & identité système...", 98);
    let extra = tokio::task::spawn_blocking(collect_scan_extra).await.unwrap_or_default();
    result.tpm_present = extra.tpm_present;
    result.tpm_enabled = extra.tpm_enabled;
    result.tpm_version = extra.tpm_version;
    result.secure_boot = extra.secure_boot;
    result.uac_level = extra.uac_level;
    result.rdp_enabled = extra.rdp_enabled;
    result.smbv1_enabled = extra.smbv1_enabled;
    result.wmi_subscriptions = extra.wmi_subscriptions;
    result.wmi_subscription_details = extra.wmi_subscription_details;
    result.local_admins = extra.local_admins;
    result.guest_enabled = extra.guest_enabled;
    result.system_manufacturer = extra.system_manufacturer;
    result.system_model = extra.system_model;
    result.system_serial = extra.system_serial;
    result.bios_manufacturer = extra.bios_manufacturer;
    result.bios_version = extra.bios_version;
    result.bios_date = extra.bios_date;
    result.license_type = extra.license_type;
    result.last_restore_point = extra.last_restore_point;
    result.pending_updates_cached = extra.pending_updates_cached;
    result.top_cpu = extra.top_cpu;
    result.top_ram = extra.top_ram;
    result.susp_tasks_count = extra.susp_tasks_count;
    result.susp_tasks = extra.susp_tasks;
    result.windows_updates_pending = extra.windows_updates_pending;
    result.activation_type = extra.activation_type;
    result.office_activation_type = extra.office_activation_type;

    emit_progress(&window, "Scan terminé ✓", 100);
    Ok(result)
}

fn emit_progress(window: &tauri::Window, step: &str, percent: u32) {
    let _ = window.emit("scan-progress", ScanProgressEvent { step: step.to_string(), percent });
}

/// Scan léger : vérifie l'essentiel sans DISM/SFC (quelques secondes au lieu de minutes).
#[tauri::command]
pub async fn run_simple_scan(window: tauri::Window) -> Result<ScanResult, String> {
    let mut result = ScanResult {
        dism_status: "Non vérifié (scan simple)".to_string(),
        sfc_status:  "Non vérifié (scan simple)".to_string(),
        battery_cycles: -1,
        defender_definition_age_days: -1,
        last_update_days: -1,
        pending_updates_cached: -1,
        ..ScanResult::default()
    };

    emit_progress(&window, "Batterie & système...", 5);
    if let Ok(Ok(Some(bat))) = tokio::task::spawn_blocking(get_battery_extended).await {
        result.battery_present = true;
        result.battery_health = bat.health_percent;
        result.battery_cycles = bat.estimated_runtime_minutes;
    }

    emit_progress(&window, "Informations système...", 15);
    let sys = tokio::task::spawn_blocking(collect_system_summary).await.unwrap_or_default();
    result.uptime_hours = sys.uptime_hours;
    result.cpu_name = sys.cpu_name;
    result.cpu_cores = sys.cpu_cores;
    result.cpu_usage_percent = sys.cpu_usage;
    result.ram_total_gb = sys.ram_total;
    result.ram_used_gb = sys.ram_used;
    result.ram_usage_percent = sys.ram_pct;
    result.windows_version = sys.win_version;

    emit_progress(&window, "Activation Windows...", 25);
    result.windows_activation = tokio::task::spawn_blocking(check_windows_activation)
        .await.unwrap_or_else(|_| "Inconnu".to_string());

    emit_progress(&window, "Sécurité — Firewall & Defender...", 35);
    let (fw, def) = tokio::task::spawn_blocking(check_security).await.unwrap_or((false, false));
    result.firewall_enabled = fw;
    result.defender_enabled = def;

    emit_progress(&window, "Espace disque...", 50);
    result.disk_usage = tokio::task::spawn_blocking(get_disk_usage).await.unwrap_or_default();

    emit_progress(&window, "Démarrage & reboot...", 60);
    let (sc, pr) = tokio::task::spawn_blocking(check_startup_and_reboot).await.unwrap_or((0, false));
    result.startup_count = sc;
    result.pending_reboot = pr;

    emit_progress(&window, "Connectivité réseau...", 70);
    result.network_ok = tokio::task::spawn_blocking(test_network).await.unwrap_or(false);

    emit_progress(&window, "Vérifications avancées rapides...", 80);
    let adv = tokio::task::spawn_blocking(collect_advanced_security).await.unwrap_or_default();
    result.antivirus_installed = adv.antivirus;
    result.last_bsod = adv.last_bsod;
    result.last_update_days = adv.last_update_days;
    result.temp_folder_size_mb = adv.temp_size_mb;
    result.virtual_memory_total_mb = adv.vmem_total_mb;
    result.virtual_memory_available_mb = adv.vmem_available_mb;

    emit_progress(&window, "Scan simple terminé ✓", 100);
    Ok(result)
}
