pub mod error;
pub mod state;
pub mod system;
pub mod utils;
pub mod installer;
pub mod maintenance;
pub mod backup;
pub mod ai;
pub mod scripts;
pub mod logging;
use std::sync::atomic::Ordering;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::Emitter;
use crate::error::NiTriTeError;
use crate::state::{AppState, CacheEntry};
use crate::system::info;
use crate::utils::config::AppConfig;
use crate::utils::platform::PlatformInfo;

// === Helper : spawn_blocking avec timeout 30s (anti-freeze WMI) ===
// Défini dans utils::wmi — re-exporté ici pour que les include!() y aient accès.
// NOTE ARCH : les lib_*.rs utilisent include!() (même scope que lib.rs).
// Migration future → pub mod + generate_handler![lib_core_cmds::cmd, ...] quand Tauri le simplifie.
use crate::utils::wmi::wmi_timeout;


// === Nouvelles commandes Diagnostic Amélioré ===

// ─── Commandes extraites (include! = même scope module) ──────────────────────
include!("lib_core_cmds.rs");
include!("lib_ai_scripts_cmds.rs");
include!("lib_recovery_cmds.rs");
include!("lib_extended_cmds.rs");
include!("lib_diagnostic_extra_a.rs");
include!("lib_diagnostic_extra_b.rs");
// ─────────────────────────────────────────────────────────────────────────────

// === Profils ===

#[tauri::command]
fn list_profiles() -> Vec<utils::profiles::Profile> {
    utils::profiles::list_profiles()
}

#[tauri::command]
fn save_profile_cmd(profile: utils::profiles::Profile) -> Result<(), String> {
    utils::profiles::save_profile(&profile).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_profile_cmd(name: String) -> Result<(), String> {
    utils::profiles::delete_profile(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_profile_json(name: String) -> Option<String> {
    utils::profiles::export_profile_json(&name)
}

#[tauri::command]
fn import_profile_json(json: String) -> Result<utils::profiles::Profile, String> {
    utils::profiles::import_profile_from_json(&json)
}

// === Gestionnaire de Dépendances ===

#[tauri::command]
async fn check_all_dependencies() -> Vec<system::dependencies::Dependency> {
    tokio::task::spawn_blocking(system::dependencies::check_all)
        .await
        .unwrap_or_default()
}

/// Streaming : émet `deps:progress` par dépendance vérifiée.
#[tauri::command]
async fn scan_dependencies_stream(app: tauri::AppHandle) {
    tokio::task::spawn_blocking(move || {
        system::dependencies::check_all_streaming(&app);
    }).await.ok();
}


#[tauri::command]
async fn install_dependency(winget_id: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        system::dependencies::install_via_winget(&winget_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

// === Setup Tauri ===

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    utils::logger::init_logger();
    logging::init_log_dir();
    logging::log_internal("INFO", "SYSTEM", "NiTriTe démarré — init logging", None);
    tracing::info!("Demarrage NiTriTe 8.113.0");

    let config = AppConfig::load();
    let app_state = AppState::new(config);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // Systeme
            get_system_info,
            get_platform_info,
            // Monitoring
            start_monitoring,
            stop_monitoring,
            // Reseau
            get_network_overview,
            get_connections,
            ping_host,
            // Installer
            get_apps,
            get_tools,
            install_app,
            uninstall_app,
            check_winget,
            list_upgradable,
            upgrade_all,
            // Maintenance
            empty_recycle_bin,
            clean_temp_files,
            run_disk_cleanup,
            get_startup_programs,
            run_system_command,
            disable_startup_program,
            // Terminal
            detect_shells,
            run_in_shell,
            // Browser Cleanup
            get_browser_cache_sizes,
            clean_browser_cache,
            // Backup
            create_backup,
            list_backups,
            // AI
            ai_check,
            ai_list_models,
            ai_query,
            ai_execute_command,
            ai_start_llamacpp,
            ai_stop_llamacpp,
            ai_llamacpp_status,
            ai_list_gguf_models,
            ai_find_llamacpp_server,
            ai_model_catalog,
            ai_download_server,
            ai_download_model,
            // Scripts
            get_builtin_scripts,
            execute_script,
            execute_builtin_script,
            // Logs
            get_app_logs,
            // Portables
            get_portable_apps,
            check_portable_installed,
            launch_portable,
            check_lhm_portable,
            launch_lhm_portable,
            launch_sdi,
            open_portables_dir,
            launch_exe,
            run_battery_report,
            // Config
            get_config,
            save_config,
            // Event Logs
            get_event_logs,
            // Drivers
            get_recommended_drivers,
            // Scripts files
            list_script_files,
            read_script_file,
            save_script_file,
            // Reports
            list_reports,
            // Exit cleanup
            cleanup_on_exit,
            // Utils
            open_url,
            open_path,
            execute_tool,
            get_export_dir,
            save_export_file,
            save_content_to_path,
            // Désinstallateur
            list_installed_apps_for_uninstall,
            uninstall_app_clean,
            preview_residuals,
            delete_residuals,
            extract_residuals,
            // Clonage
            get_disks_for_clone,
            start_system_image,
            start_robocopy_clone,
            // Récupération données
            list_shadow_copies,
            browse_shadow_copy,
            restore_from_shadow,
            scan_recycle_bin,
            restore_recycle_bin_item,
            scan_deleted_files,
            search_shadow_copy,
            restore_files_batch,
            list_connected_disks,
            browse_disk_path,
            recover_files_safe,
            get_user_profile_folders,
            backup_user_folders,
            compare_shadow_with_current,
            scan_all_deleted_files,
            // Partitions & SMART
            get_disks_smart,
            get_partition_list,
            format_partition_cmd,
            assign_drive_letter_cmd,
            create_partition_cmd,
            delete_partition_cmd,
            initialize_disk_cmd,
            // Shadow Copy gestion
            create_shadow_copy_cmd,
            delete_shadow_copy_cmd,
            open_in_explorer,
            get_ntfs_drives,
            // Récupération avancée
            create_disk_image_cmd,
            surface_test_volume_cmd,
            deep_mft_scan_advanced_cmd,
            generate_recovery_report_cmd,
            // Partition avancé
            get_partition_resize_limits_cmd,
            resize_partition_cmd,
            backup_mbr_cmd,
            restore_mbr_cmd,
            scan_lost_partitions_cmd,
            // Extended Info
            get_bios_info,
            get_battery_extended,
            get_folder_sizes,
            run_total_scan,
            system::total_scan::run_simple_scan,
            // Debloat
            debloat_disable_telemetry,
            debloat_disable_cortana,
            debloat_disable_xbox,
            debloat_disable_superfetch,
            debloat_disable_tips,
            debloat_optimize_power,
            debloat_disable_visual_effects,
            debloat_clear_event_logs,
            debloat_clear_wu_cache,
            debloat_flush_dns,
            debloat_reset_network,
            debloat_enable_trim,
            debloat_remove_bloatware,
            maintenance::debloat::debloat_run_extra,
            // Chocolatey
            check_chocolatey,
            list_chocolatey_upgrades,
            upgrade_chocolatey_all,
            // Windows Updates
            check_windows_updates,
            installer::windows_update::search_pending_updates,
            installer::windows_update::install_windows_updates,
            scan_pending_windows_updates,
            trigger_windows_update,
            open_mas_window,
            get_network_extended,
            check_scoop,
            list_scoop_upgrades,
            upgrade_scoop_all,
            // Detailed Diagnostics
            system::system_detailed::get_motherboard_detailed,
            system::system_detailed::get_gpu_detailed,
            system::system_detailed::get_ram_detailed,
            system::system_detailed::get_audio_devices,
            system::system_detailed::get_usb_devices,
            system::system_detailed::get_battery_detailed,
            system::system_detailed::get_monitor_info,
            system::system_detailed::get_power_plans,
            system::system_detailed::get_printers,
            system::system_detailed::get_environment_variables,
            system::system_detailed::get_windows_license,
            system::system_detailed::get_bitlocker_report,
            system::system_detailed::get_installed_updates,
            system::system_detailed_extra::get_storage_physical_info,
            system::system_detailed_extra::get_network_adapters_detailed,
            system::system_detailed_extra::get_cpu_cache_info,
            system::system_detailed_extra::get_installed_software,
            // Nouveaux diagnostics
            get_running_processes,
            get_windows_services,
            get_security_status,
            get_scheduled_tasks,
            get_active_connections,
            get_wifi_status,
            get_logical_volumes,
            get_cpu_extended,
            get_os_extended,
            get_folder_sizes_detailed,
            get_startup_programs_detailed,
            get_smart_info,
            // Nouveaux onglets diagnostics
            system::accounts::get_user_accounts,
            system::firewall_rules::get_firewall_rules,
            system::shares::get_network_shares,
            system::registry_persist::get_registry_persistence,
            system::registry_persist::registry_browse,
            system::registry_persist::registry_set_value,
            system::registry_persist::registry_delete_value,
            system::sys_history::get_system_history,
            // Nouveaux modules 10x
            system::sys_drivers::get_sys_drivers_list,
            system::certificates::get_certificates,
            system::perf_snapshot::get_perf_snapshot,
            system::net_tools::run_ping,
            system::net_tools::run_traceroute,
            system::net_tools::run_nslookup,
            system::net_tools::get_ip_config,
            system::net_tools::get_arp_table,
            system::net_tools::get_route_table,
            system::net_tools::scan_ports,
            system::net_tools::get_wifi_networks,
            system::net_tools::get_local_open_ports,
            system::net_tools::check_http,
            system::net_tools::get_net_shares,
            system::net_tools::test_bandwidth,
            system::repair::check_system_health,
            system::repair::run_repair_command,
            system::driver_updater::get_hardware_devices,
            system::driver_updater::scan_driver_folder,
            system::driver_updater::install_driver,
            system::driver_updater::check_driver_updates_winupdate,
            system::driver_updater::install_driver_winupdate,
            system::driver_updater::install_all_driver_updates,
            system::driver_updater::get_all_hardware_ids,
            // Benchmark
            system::benchmark::run_cpu_bench,
            system::benchmark::run_ram_bench,
            system::benchmark::run_disk_bench,
            system::benchmark::run_cpu_mt_bench,
            system::benchmark::run_crypto_bench,
            system::benchmark::run_compression_bench,
            // Cleaner
            system::cleaner::get_clean_targets,
            system::cleaner::scan_clean_targets_stream,
            system::cleaner::clean_target,
            system::cleaner::quarantine_target,
            system::cleaner::list_quarantine,
            system::cleaner::clear_quarantine,
            system::cleaner::get_large_files,
            // BSOD Analyzer
            system::bsod_analyzer::get_bsod_history,
            system::bsod_analyzer::get_bugcheck_description,
            // Hosts Editor
            system::hosts_editor::get_hosts_entries,
            system::hosts_editor::add_hosts_entry,
            system::hosts_editor::delete_hosts_entry,
            system::hosts_editor::toggle_hosts_entry,
            system::hosts_editor::backup_hosts,
            system::hosts_editor::get_hosts_raw,
            system::hosts_editor::resolve_hostname,
            system::hosts_editor::import_hosts_blocklist,
            // Boot Manager
            system::boot_manager::get_boot_config,
            system::boot_manager::set_boot_timeout,
            system::boot_manager::set_default_boot,
            system::boot_manager::boot_to_recovery,
            // WSL
            system::wsl_info::get_wsl_info,
            system::wsl_info::wsl_run_command,
            system::wsl_info::wsl_set_default_version,
            // Bluetooth
            system::bluetooth::get_bluetooth_info,
            system::bluetooth::toggle_bluetooth,
            // Perf History
            system::perf_history::get_perf_history,
            system::perf_history::get_top_processes_by_cpu,
            // ============ Nouvelles commandes Diagnostics Améliorés ============
            // Processus
            kill_process,
            get_processes_extended,
            // Services
            control_service,
            set_service_start_mode,
            // Variables d'environnement
            set_environment_variable,
            delete_environment_variable,
            // Démarrage
            toggle_startup_program,
            remove_startup_program,
            // Tâches planifiées
            create_scheduled_task,
            delete_scheduled_task,
            run_scheduled_task_now,
            // Énergie
            set_power_plan,
            // Imprimantes
            set_default_printer,
            // Batterie
            open_battery_report_html,
            // Gestionnaires de paquets
            install_package_manager,
            // Périphériques
            open_device_manager,
            // BIOS / Carte mère (étendus)
            get_bios_extended,
            get_motherboard_extended,
            // Écran taux rafraîchissement
            get_monitor_refresh_rates,
            // Audio étendu
            get_audio_extended,
            // Licences
            get_third_party_licenses,
            // Activation
            open_activation_settings,
            run_slmgr,
            // Points de restauration
            system::restore_points::list_restore_points_cmd,
            system::restore_points::create_restore_point_cmd,
            system::restore_points::delete_restore_point_cmd,
            system::restore_points::rollback_last_windows_update_cmd,
            // Regedit navigation
            open_in_regedit,
            // Script Generator
            installer::script_generator::generate_deploy_script,
            // Script Validator
            scripts::validator::validate_script,
            // Favorites & Install History
            installer::favorites::get_favorites_data,
            installer::favorites::toggle_favorite,
            installer::favorites::log_install,
            installer::favorites::clear_install_history,
            // Usage Stats
            utils::stats::get_app_stats,
            utils::stats::log_action,
            utils::stats::reset_stats,
            // Report Generator
            utils::report_generator::generate_html_report,
            utils::report_generator::generate_md_report,
            // Profils
            list_profiles,
            save_profile_cmd,
            delete_profile_cmd,
            export_profile_json,
            import_profile_json,
            // Gestionnaire Dépendances
            check_all_dependencies,
            scan_dependencies_stream,
            install_dependency,
            // ═══ Nouvelles fonctionnalités v26.46 ═══
            system::extras::hash_file,
            system::extras::get_dns_presets,
            system::extras::get_network_adapters_for_dns,
            system::extras::switch_dns,
            system::extras::flush_dns_cache,
            system::extras::get_local_ports,
            system::extras::get_disk_tree,
            system::extras::get_big_files,
            system::extras::trash_file,
            system::extras::find_duplicates,
            system::extras::get_temperatures,
            system::extras::get_nearby_wifi,
            system::extras::apply_turbo_mode,
            system::extras::get_docker_info,
            system::extras::docker_container_action,
            system::extras::docker_image_remove,
            system::extras::docker_container_logs,
            system::extras::delete_file,
            system::extras::toggle_defender_realtime,
            system::extras::update_defender_signatures,
            system::extras::enable_firewall_all_profiles,
            system::extras::quick_uninstall_software,
            system::extras::get_browser_cache_info,
            system::extras::clean_browser_cache_path,
            system::extras::ping_dns,
            system::extras::hash_folder,
            system::extras::get_cpu_core_temps,
            system::extras::get_gpu_temps,
            system::extras::enable_hidden_power_plans,
            system::extras::get_all_product_keys,
            system::extras::get_problem_devices,
            system::extras::run_quick_optimization,
            // ═══ WinPE v26.57 ═══
            system::winpe::is_winpe_mode,
            system::winpe::get_pe_system_info,
            system::winpe::get_pe_drives,
            system::winpe::repair_mbr,
            system::winpe::repair_boot,
            system::winpe::rebuild_bcd,
            system::winpe::scan_os_installations,
            system::winpe::run_chkdsk_pe,
            system::winpe::run_sfc_offline,
            system::winpe::run_dism_offline_repair,
            system::winpe::list_offline_users,
            system::winpe::reset_user_password,
            system::winpe::disk_wipe,
            system::winpe::detect_windows_installs,
            system::winpe::get_bitlocker_status,
            system::winpe::unlock_bitlocker,
            system::winpe::clear_offline_password,
            system::winpe::enable_offline_account,
            system::winpe::winpe_run_command,
            // ═══ Capteurs complets (sensors.rs) ═══
            system::sensors::get_all_sensors,
            // ═══ Scanner DLL (dll_scanner.rs) ═══
            system::dll_scanner::scan_dlls,
            system::dll_scanner::delete_dll,
            // ═══ Logging structuré v26.57 ═══
            logging::log_entry,
            logging::get_recent_logs,
            logging::clear_logs,
            logging::get_log_stats,
            logging::list_log_archives,
            logging::get_log_file_path,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur lors du lancement de NiTriTe");
}
