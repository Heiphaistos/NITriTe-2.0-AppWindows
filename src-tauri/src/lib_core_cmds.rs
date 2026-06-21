
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
    let interval = {
        let config = state.config.lock().await;
        config.monitor_interval_ms
    };
    let running = state.monitor_running.clone();
    system::monitor::start_monitoring(window, running, interval);
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

#[tauri::command]
async fn install_app(app_id: Option<String>, winget_id: Option<String>, window: tauri::Window) -> Result<installer::winget::InstallResult, NiTriTeError> {
    // Resoudre l'ID winget : soit fourni directement, soit lookup depuis programs.json via app_id
    let resolved_id = if let Some(wid) = winget_id.filter(|w| !w.is_empty()) {
        wid
    } else if let Some(aid) = app_id {
        let apps = installer::manager::get_default_apps();
        apps.iter()
            .find(|a| a.id == aid)
            .and_then(|a| a.winget_id.clone())
            .unwrap_or(aid)
    } else {
        return Err(NiTriTeError::System("Aucun identifiant d'application fourni".into()));
    };
    tokio::task::spawn_blocking(move || installer::winget::install_package(&resolved_id, &window))
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
async fn upgrade_all(window: tauri::Window) -> Result<(), NiTriTeError> {
    tokio::task::spawn_blocking(move || installer::winget::upgrade_all(&window))
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
    // Valider via whitelist avant toute exécution
    let full_cmd = if args.is_empty() {
        cmd.clone()
    } else {
        format!("{} {}", cmd, args.join(" "))
    };
    ai::tool_calling::is_safe(&full_cmd)?;
    tokio::task::spawn_blocking(move || {
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        maintenance::commands::execute_system_command(&cmd, &args_refs, 60)
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn disable_startup_program(name: String, location: String) -> Result<maintenance::cleanup::CleanupResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || maintenance::cleanup::disable_startup_program(&name, &location))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
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
                    if name.contains("nitrite") || name.contains("tauri") {
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

