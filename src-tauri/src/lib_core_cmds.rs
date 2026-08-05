
// === Commandes Systeme ===

/// get_system_info — résultat caché 30s côté Rust (évite WMI répété)
/// Le cache stocke la valeur JSON brute et la renvoie directement au frontend.
#[tauri::command]
async fn get_system_info(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, NiTriTeError> {
    const KEY: &str = "get_system_info";
    const TTL: u64 = 30;
    // Vérification cache
    {
        let cache = state.wmi_cache.lock().await;
        if let Some(entry) = cache.get(KEY) {
            if entry.is_fresh(TTL) {
                return Ok(entry.data.clone());
            }
        }
    }
    // Cache miss — WMI query avec timeout anti-freeze
    let result = wmi_timeout(info::collect_system_info).await?;
    let json = serde_json::to_value(&result)
        .map_err(|e| NiTriTeError::System(e.to_string()))?;
    // Mise en cache
    {
        let mut cache = state.wmi_cache.lock().await;
        cache.insert(KEY.to_string(), CacheEntry {
            data: json.clone(),
            acquired_at: std::time::Instant::now(),
        });
    }
    Ok(json)
}

#[tauri::command]
async fn get_platform_info() -> Result<PlatformInfo, NiTriTeError> {
    tokio::task::spawn_blocking(PlatformInfo::detect)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

// === Monitoring ===

#[tauri::command]
async fn start_monitoring(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> Result<(), NiTriTeError> {
    let (interval, process_count) = {
        let config = state.config.lock().await;
        (config.monitor_interval_ms, config.process_count)
    };
    let running = state.monitor_running.clone();
    system::monitor::start_monitoring(window, running, interval, process_count);
    Ok(())
}

#[tauri::command]
async fn stop_monitoring(state: tauri::State<'_, AppState>) -> Result<(), NiTriTeError> {
    state.monitor_running.store(false, Ordering::SeqCst);
    Ok(())
}

// === Reseau ===

#[tauri::command]
async fn get_network_overview() -> Result<system::network::NetworkOverview, NiTriTeError> {
    tokio::task::spawn_blocking(system::network::get_network_overview)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn get_connections() -> Result<Vec<system::network::ConnectionInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(system::network::get_connections)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn ping_host(host: String) -> Result<system::network::PingResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::network::ping_host(&host))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// === Installer ===

#[tauri::command]
async fn get_apps() -> Result<Vec<installer::manager::AppEntry>, NiTriTeError> {
    Ok(installer::manager::get_default_apps())
}

#[tauri::command]
async fn get_tools() -> Result<Vec<installer::manager::ToolEntry>, NiTriTeError> {
    Ok(installer::manager::get_tools())
}

/// Reconstruit l'AppEntry complet (winget_id + choco_id + url) depuis le
/// catalogue via app_id, avec repli synthetique si absent — pour que la
/// chaine de fallback (winget -> chocolatey -> scoop -> telechargement
/// direct) ait toutes les methodes disponibles, pas seulement winget.
fn resolve_app_entry(app_id: &Option<String>, winget_id: &Option<String>) -> installer::manager::AppEntry {
    let entry = app_id.as_ref().and_then(|aid| {
        installer::manager::get_default_apps().into_iter().find(|a| &a.id == aid)
    });
    entry.unwrap_or_else(|| installer::manager::AppEntry {
        id: app_id.clone().unwrap_or_default(),
        name: app_id.clone().unwrap_or_default(),
        description: String::new(),
        category: String::new(),
        winget_id: winget_id.clone().filter(|w| !w.is_empty()),
        choco_id: None,
        url: None,
        icon: String::new(),
    })
}

#[tauri::command]
async fn install_app(app_id: Option<String>, winget_id: Option<String>, window: tauri::Window) -> Result<installer::winget::InstallResult, NiTriTeError> {
    let entry = resolve_app_entry(&app_id, &winget_id);
    tokio::task::spawn_blocking(move || installer::smart_install::install_app_smart(&entry, &window))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn uninstall_app(app_id: Option<String>, winget_id: Option<String>) -> Result<installer::winget::InstallResult, NiTriTeError> {
    let entry = resolve_app_entry(&app_id, &winget_id);
    tokio::task::spawn_blocking(move || installer::smart_install::uninstall_app_smart(&entry))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn check_winget() -> Result<bool, NiTriTeError> {
    Ok(installer::winget::check_winget())
}

#[tauri::command]
async fn list_upgradable() -> Result<Vec<installer::winget::WingetPackage>, NiTriTeError> {
    tokio::task::spawn_blocking(installer::winget::list_upgradable)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn upgrade_all(excluded_ids: Option<Vec<String>>, window: tauri::Window) -> Result<(), NiTriTeError> {
    let excluded = excluded_ids.unwrap_or_default();
    tokio::task::spawn_blocking(move || installer::winget::upgrade_all(excluded, &window))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// === Terminal multi-shell ===

#[tauri::command]
async fn detect_shells() -> Result<Vec<maintenance::terminal::ShellInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::terminal::detect_shells)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn run_in_shell(shell_id: String, command: String) -> Result<maintenance::terminal::ShellResult, NiTriTeError> {
    // Sécurité : longueur maximale de la commande pour éviter les abus
    if command.len() > 8192 {
        return Err(NiTriTeError::System("Commande trop longue (max 8192 caractères)".into()));
    }
    // Sécurité : bloquer les caractères nuls qui peuvent tronquer les arguments
    if command.contains('\0') {
        return Err(NiTriTeError::System("Commande contient des caractères nuls invalides".into()));
    }
    tokio::task::spawn_blocking(move || maintenance::terminal::run_in_shell(&shell_id, &command, 120))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// === Browser Cleanup ===

#[tauri::command]
async fn get_browser_cache_sizes() -> Result<Vec<maintenance::browser_cleanup::BrowserCacheInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::browser_cleanup::get_browser_cache_sizes)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn clean_browser_cache(browser_ids: Vec<String>) -> Result<Vec<maintenance::browser_cleanup::CleanupResult>, NiTriTeError> {
    tokio::task::spawn_blocking(move || maintenance::browser_cleanup::clean_browser_cache(browser_ids))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// === Event Logs ===

#[tauri::command]
async fn get_event_logs(log_name: String, count: u32) -> Result<Vec<system::eventlog::EventLogEntry>, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::eventlog::get_event_logs(&log_name, count))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// === Drivers Recommandes ===

#[tauri::command]
async fn get_recommended_drivers() -> Result<Vec<system::drivers::DriverStatus>, NiTriTeError> {
    tokio::task::spawn_blocking(system::drivers::get_recommended_drivers)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// === Scripts File Management ===

#[tauri::command]
async fn list_script_files(dir: String) -> Result<Vec<scripts::executor::ScriptFileInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(move || scripts::executor::list_script_files(&dir))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn read_script_file(path: String) -> Result<String, NiTriTeError> {
    tokio::task::spawn_blocking(move || scripts::executor::read_script_file(&path))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn save_script_file(path: String, content: String) -> Result<(), NiTriTeError> {
    tokio::task::spawn_blocking(move || scripts::executor::save_script_file(&path, &content))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// === Reports ===

#[tauri::command]
async fn list_reports() -> Result<Vec<ReportInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(|| {
        let backup_dir = utils::paths::backups_dir();
        let mut reports = Vec::new();

        if backup_dir.exists() {
            for entry in std::fs::read_dir(&backup_dir).map_err(NiTriTeError::Io)? {
                let entry = entry.map_err(NiTriTeError::Io)?;
                let meta = entry.metadata().map_err(NiTriTeError::Io)?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") || name.ends_with(".txt") {
                    let modified_secs = meta.modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let created = chrono::DateTime::from_timestamp(modified_secs as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "Inconnu".to_string());
                    reports.push(ReportInfo {
                        name,
                        path: entry.path().to_string_lossy().to_string(),
                        size_bytes: meta.len(),
                        created,
                    });
                }
            }
        }

        reports.sort_by(|a, b| b.created.cmp(&a.created));
        Ok(reports)
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// === Maintenance ===

#[tauri::command]
async fn empty_recycle_bin() -> Result<maintenance::cleanup::CleanupResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::cleanup::empty_recycle_bin)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn clean_temp_files() -> Result<maintenance::cleanup::CleanupResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::cleanup::clean_temp_files)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn run_disk_cleanup() -> Result<maintenance::cleanup::CleanupResult, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::cleanup::run_disk_cleanup)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn get_startup_programs() -> Result<Vec<maintenance::cleanup::StartupEntry>, NiTriTeError> {
    tokio::task::spawn_blocking(maintenance::cleanup::get_startup_programs)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn run_system_command(cmd: String, args: Vec<String>) -> Result<maintenance::commands::CommandResult, NiTriTeError> {
    validate_ui_command(&cmd, &args)?;
    tokio::task::spawn_blocking(move || {
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        maintenance::commands::execute_system_command(&cmd, &args_refs, 60)
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

/// Validation pour les commandes UI/système (plus permissive que is_safe() qui cible l'IA).
/// Bloque les opérations destructives, détecte l'injection shell sur les args passés à cmd.exe.
fn validate_ui_command(cmd: &str, args: &[String]) -> Result<(), NiTriTeError> {
    let cmd_lower = cmd.to_lowercase();
    let cmd_base = std::path::Path::new(&cmd_lower)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&cmd_lower)
        .to_string();

    // Commandes vraiment destructives — jamais autorisées
    const BLOCKED: &[&str] = &[
        "format", "fdisk", "dd", "mkfs", "deltree",
        "cipher /w", "sdelete",
    ];
    if BLOCKED.iter().any(|b| cmd_base == *b) {
        return Err(NiTriTeError::CommandDenied(format!("Commande interdite: {}", cmd)));
    }

    // Quand cmd.exe shell-exécute des args, vérifier l'injection
    if cmd_base == "cmd" {
        let shell_args: String = args.iter()
            .skip_while(|a| a.to_lowercase() == "/c" || a.to_lowercase() == "/k")
            .cloned().collect::<Vec<_>>().join(" ");
        let dangerous_patterns = [
            "rm -rf", "del /f /s", "format ", "rmdir /s /q",
            "rd /s /q", "> nul", "reg delete", "bcdedit",
        ];
        let shell_lower = shell_args.to_lowercase();
        for pat in dangerous_patterns {
            if shell_lower.contains(pat) {
                return Err(NiTriTeError::CommandDenied(
                    format!("Argument cmd.exe potentiellement destructif: {}", pat)
                ));
            }
        }
    }

    // PowerShell — bloquer opérations matérielles destructives
    if cmd_base == "powershell" || cmd_base == "pwsh" {
        let ps_args: String = args.iter().cloned().collect::<Vec<_>>().join(" ");
        let ps_lower = ps_args.to_lowercase();
        let dangerous_ps = [
            "format-volume", "clear-disk", "initialize-disk",
            "stop-computer", "restart-computer",
        ];
        for pat in &dangerous_ps {
            if ps_lower.contains(pat) {
                return Err(NiTriTeError::CommandDenied(
                    format!("Argument PowerShell destructif bloqué: {}", pat)
                ));
            }
        }
    }

    Ok(())
}

#[tauri::command]
async fn disable_startup_program(name: String, location: String) -> Result<maintenance::cleanup::CleanupResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || maintenance::cleanup::disable_startup_program(&name, &location))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

/// `%TEMP%` est un dossier PARTAGÉ par toutes les applications de la machine
/// (pas propre à NiTriTe) — le nettoyage de sortie n'y supprime QUE ce qui
/// porte le nom de l'app elle-même. Le mot générique "tauri" matchait aussi
/// les artefacts temp d'autres applications Tauri installées sur la même
/// machine, un risque de suppression collatérale de données appartenant à
/// une application tierce (plusieurs autres apps de ce parc sont elles aussi
/// construites avec Tauri).
fn is_own_temp_entry(name: &str) -> bool {
    name.contains("nitrite")
}

// === Nettoyage sortie ===

#[tauri::command]
async fn cleanup_on_exit(app: tauri::AppHandle) -> Result<(), NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        // 1. Logs applicatif (.logs/ à la racine du projet — dev uniquement)
        let logs_candidates = [
            std::env::current_dir().ok().map(|d| d.join(".logs")),
            std::env::var("APPDATA").ok().map(|a| std::path::PathBuf::from(a).join("com.nitrite.tool").join("logs")),
        ];
        for candidate in logs_candidates.iter().flatten() {
            if candidate.exists() {
                let _ = std::fs::remove_dir_all(candidate);
            }
        }

        // 2. Fichiers temp Tauri dans %LOCALAPPDATA%\com.nitrite.tool\EBWebView\...
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let tauri_cache = std::path::PathBuf::from(&local).join("com.nitrite.tool");
            for sub in &["EBWebView", "logs", "temp"] {
                let p = tauri_cache.join(sub);
                if p.exists() { let _ = std::fs::remove_dir_all(&p); }
            }
        }

        // 3. Dossier temp interne de l'app (rapports, fichiers temporaires)
        let app_temp = crate::utils::paths::temp_dir();
        if app_temp.exists() { let _ = std::fs::remove_dir_all(&app_temp); }

        // 5. Fichiers temp Windows portant "nitrite" dans le nom
        if let Ok(temp) = std::env::var("TEMP") {
            if let Ok(entries) = std::fs::read_dir(&temp) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if is_own_temp_entry(&name) {
                        if entry.path().is_dir() {
                            let _ = std::fs::remove_dir_all(entry.path());
                        } else {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }

        // 6. WebView2 data (cookies, cache) du dossier EBWebView
        if let Ok(appdata) = std::env::var("APPDATA") {
            let webview_cache = std::path::PathBuf::from(&appdata).join("com.nitrite.tool");
            for sub in &["logs", "tmp", "cache"] {
                let p = webview_cache.join(sub);
                if p.exists() { let _ = std::fs::remove_dir_all(&p); }
            }
        }
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?;

    // Forcer la fermeture après nettoyage
    app.exit(0);
    Ok(())
}

// === Backup ===

#[tauri::command]
async fn create_backup(items: Vec<String>, format: String, custom_path: Option<String>) -> Result<backup::collector::BackupManifest, NiTriTeError> {
    tokio::task::spawn_blocking(move || backup::collector::create_backup(items, format, custom_path))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn list_backups() -> Result<Vec<backup::collector::BackupEntryInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(backup::collector::list_backups)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_commands_are_rejected() {
        assert!(validate_ui_command("format", &[]).is_err());
        assert!(validate_ui_command("fdisk", &[]).is_err());
        assert!(validate_ui_command("deltree", &[]).is_err());
        assert!(validate_ui_command("sdelete", &[]).is_err());
    }

    #[test]
    fn safe_commands_are_allowed() {
        assert!(validate_ui_command("ipconfig", &[]).is_ok());
        assert!(validate_ui_command("netstat", &["-an".to_string()]).is_ok());
        assert!(validate_ui_command("sfc", &["/scannow".to_string()]).is_ok());
    }

    #[test]
    fn cmd_with_destructive_arg_blocked() {
        let args = vec!["/c".to_string(), "del /f /s C:\\important".to_string()];
        assert!(validate_ui_command("cmd", &args).is_err());
        let args2 = vec!["/c".to_string(), "rmdir /s /q C:\\data".to_string()];
        assert!(validate_ui_command("cmd", &args2).is_err());
        let args3 = vec!["/c".to_string(), "reg delete HKLM\\SOFTWARE".to_string()];
        assert!(validate_ui_command("cmd", &args3).is_err());
    }

    #[test]
    fn cmd_with_safe_arg_allowed() {
        let args = vec!["/c".to_string(), "ipconfig /all".to_string()];
        assert!(validate_ui_command("cmd", &args).is_ok());
    }

    #[test]
    fn blocked_command_with_full_path_rejected() {
        assert!(validate_ui_command("C:\\Windows\\format.exe", &[]).is_err());
        assert!(validate_ui_command("C:\\bin\\fdisk.exe", &[]).is_err());
    }

    // ── is_own_temp_entry ─────────────────────────────────────────────────────

    #[test]
    fn own_app_temp_entries_matched() {
        assert!(is_own_temp_entry("nitrite-report-1234.tmp"));
        assert!(is_own_temp_entry("nitrite_export_20260805"));
    }

    #[test]
    fn other_tauri_apps_temp_entries_not_matched() {
        // Régression : %TEMP% est partagé par toutes les apps de la machine.
        // "tauri" seul matchait aussi les dossiers temp d'autres apps Tauri
        // (ForgeChat, ForgeEmu, PlexMetaForge...) installées sur ce parc.
        assert!(!is_own_temp_entry("forgechat-tauri-updater"));
        assert!(!is_own_temp_entry("tauri-webview-cache"));
    }

    #[test]
    fn unrelated_temp_entries_not_matched() {
        assert!(!is_own_temp_entry("chrome_cache_1234"));
        assert!(!is_own_temp_entry("random_folder"));
    }
}
