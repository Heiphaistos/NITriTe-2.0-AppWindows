
// === AI ===

#[tauri::command]
async fn ai_check(state: tauri::State<'_, AppState>) -> Result<bool, NiTriTeError> {
    let url = {
        let config = state.config.lock().await;
        config.ollama_url.clone()
    };
    Ok(ai::ollama::check_ollama(&url).await)
}

#[tauri::command]
async fn ai_list_models(state: tauri::State<'_, AppState>) -> Result<Vec<ai::ollama::OllamaModel>, NiTriTeError> {
    let url = {
        let config = state.config.lock().await;
        config.ollama_url.clone()
    };
    ai::ollama::list_models(&url).await
}

#[tauri::command]
async fn ai_query(
    prompt: String,
    model: Option<String>,
    system_prompt: Option<String>,
    history: Option<Vec<serde_json::Value>>,   // [{role, content}]
    state: tauri::State<'_, AppState>,
) -> Result<String, NiTriTeError> {
    let (backend, port, ollama_url, default_model) = {
        let config = state.config.lock().await;
        (config.ai_backend.clone(), config.llamacpp_port, config.ollama_url.clone(), config.ollama_model.clone())
    };
    let temp = 0.7_f64;
    let m = model.unwrap_or(default_model);

    // Construire les messages
    let mut messages_ollama: Vec<ai::ollama::OllamaChatMessage> = vec![];
    let mut messages_llama: Vec<ai::llamacpp::ChatMessage> = vec![];

    let add_msg = |role: &str, content: &str,
                   mo: &mut Vec<ai::ollama::OllamaChatMessage>,
                   ml: &mut Vec<ai::llamacpp::ChatMessage>| {
        mo.push(ai::ollama::OllamaChatMessage { role: role.into(), content: content.into() });
        ml.push(ai::llamacpp::ChatMessage   { role: role.into(), content: content.into() });
    };

    if let Some(sys) = &system_prompt {
        if !sys.trim().is_empty() {
            add_msg("system", sys.trim(), &mut messages_ollama, &mut messages_llama);
        }
    }
    if let Some(hist) = history {
        for msg in hist {
            let role = msg["role"].as_str().unwrap_or("user").to_string();
            let content = msg["content"].as_str().unwrap_or("").to_string();
            if !content.is_empty() && (role == "user" || role == "assistant") {
                add_msg(&role, &content, &mut messages_ollama, &mut messages_llama);
            }
        }
    }
    add_msg("user", &prompt, &mut messages_ollama, &mut messages_llama);

    if backend == "llamacpp" {
        ai::llamacpp::chat(port, &m, messages_llama, temp).await
    } else {
        ai::ollama::chat(&ollama_url, &m, messages_ollama, temp).await
    }
}

#[tauri::command]
async fn ai_execute_command(command: String) -> Result<ai::tool_calling::SafeCommandResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || ai::tool_calling::execute_safe(&command))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn ai_start_llamacpp(
    model_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let (server_path, port) = {
        let config = state.config.lock().await;
        (config.llamacpp_server_path.clone(), config.llamacpp_port)
    };
    // Détecter le binaire si chemin non configuré
    let exe_dir = std::env::current_exe()
        .ok().and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let bin = if !server_path.is_empty() && std::path::Path::new(&server_path).exists() {
        server_path
    } else {
        ai::llamacpp::find_server_binary(&exe_dir)
            .ok_or_else(|| "llama-server.exe introuvable. Placez-le dans logiciel/AI/".to_string())?
    };

    // Tuer l'ancien processus si présent
    {
        let mut proc = state.llamacpp_process.lock().await;
        if let Some(mut child) = proc.take() {
            let _ = child.kill().await; // tokio::process::Child::kill() est async
        }
    }

    let child = ai::llamacpp::start_server(&bin, &model_path, port)
        .map_err(|e| e.to_string())?;

    {
        let mut proc = state.llamacpp_process.lock().await;
        *proc = Some(child);
    }
    // Sauvegarder le chemin du modèle
    {
        let mut config = state.config.lock().await;
        config.llamacpp_model_path = model_path;
        config.llamacpp_server_path = bin;
        let _ = config.save();
    }
    Ok(())
}

#[tauri::command]
async fn ai_stop_llamacpp(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut proc = state.llamacpp_process.lock().await;
    if let Some(mut child) = proc.take() {
        child.kill().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn ai_llamacpp_status(state: tauri::State<'_, AppState>) -> Result<bool, NiTriTeError> {
    let port = { state.config.lock().await.llamacpp_port };
    Ok(ai::llamacpp::is_server_ready(port).await)
}

#[tauri::command]
async fn ai_list_gguf_models() -> Result<Vec<ai::llamacpp::GgufModel>, NiTriTeError> {
    let exe_dir = std::env::current_exe()
        .ok().and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let dir = ai::llamacpp::models_dir(&exe_dir);
    Ok(ai::llamacpp::list_gguf_models(&dir))
}

#[tauri::command]
async fn ai_find_llamacpp_server() -> Result<Option<String>, NiTriTeError> {
    let exe_dir = std::env::current_exe()
        .ok().and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()))
        .unwrap_or_default();
    Ok(ai::llamacpp::find_server_binary(&exe_dir))
}

#[tauri::command]
fn ai_model_catalog() -> Vec<ai::llamacpp::ModelCatalogEntry> {
    ai::llamacpp::model_catalog()
}

#[tauri::command]
async fn ai_download_server(app: tauri::AppHandle) -> Result<String, String> {
    let exe_dir = std::env::current_exe()
        .ok().and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let app_clone = app.clone();
    ai::llamacpp::download_server(&exe_dir, move |progress| {
        let _ = app_clone.emit("ai:download-progress", &progress);
    }).await
}

#[tauri::command]
async fn ai_download_model(
    app: tauri::AppHandle,
    url: String,
    filename: String,
) -> Result<String, String> {
    let exe_dir = std::env::current_exe()
        .ok().and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let app_clone = app.clone();
    ai::llamacpp::download_model_file(&url, &filename, &exe_dir, move |progress| {
        let _ = app_clone.emit("ai:download-progress", &progress);
    }).await
}

// === Scripts ===

#[tauri::command]
async fn get_builtin_scripts() -> Result<Vec<scripts::executor::ScriptEntry>, NiTriTeError> {
    Ok(scripts::executor::get_builtin_scripts())
}

#[tauri::command]
async fn execute_script(
    content: String,
    script_type: String,
    window: tauri::Window,
) -> Result<scripts::executor::ScriptResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || scripts::executor::execute_script(&content, &script_type, &window))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// === Logs ===

#[derive(serde::Serialize)]
struct ReportInfo {
    name: String,
    path: String,
    size_bytes: u64,
    created: String,
}

#[derive(serde::Serialize)]
struct AppLogEntry {
    timestamp: String,
    level: String,
    message: String,
}

#[tauri::command]
async fn get_app_logs() -> Result<Vec<AppLogEntry>, NiTriTeError> {
    tokio::task::spawn_blocking(|| {
        let logs_dir = utils::paths::logs_dir();
        let mut entries: Vec<AppLogEntry> = Vec::new();

        // Lire tous les fichiers .log dans le dossier
        let mut log_files: Vec<_> = std::fs::read_dir(&logs_dir)
            .map_err(NiTriTeError::Io)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .to_str()
                    .map(|s| s.contains("nitrite.log"))
                    .unwrap_or(false)
            })
            .collect();

        // Trier par date de modification (plus recent en dernier)
        log_files.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());

        // Garder les 3 derniers fichiers max
        let files_to_read: Vec<_> = log_files.into_iter().rev().take(3).collect();

        for entry in files_to_read.into_iter().rev() {
            let content = match std::fs::read_to_string(entry.path()) {
                Ok(c) => c,
                Err(_) => continue,
            };

            for line in content.lines() {
                // Format tracing: 2026-03-01T10:00:00.123Z  INFO nitrite_lib: message
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // Extraire timestamp, level, message
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() < 2 {
                    entries.push(AppLogEntry {
                        timestamp: String::new(),
                        level: "INFO".to_string(),
                        message: line.to_string(),
                    });
                    continue;
                }

                let timestamp = parts[0].to_string();
                let rest = parts[1].trim();

                // Trouver le level (INFO, WARN, ERROR, DEBUG, TRACE)
                let (level, msg) = if let Some(s) = rest.strip_prefix("INFO") {
                    ("INFO", s.trim().trim_start_matches(|c: char| !c.is_alphabetic()))
                } else if let Some(s) = rest.strip_prefix("WARN") {
                    ("WARN", s.trim().trim_start_matches(|c: char| !c.is_alphabetic()))
                } else if let Some(s) = rest.strip_prefix("ERROR") {
                    ("ERROR", s.trim().trim_start_matches(|c: char| !c.is_alphabetic()))
                } else if let Some(s) = rest.strip_prefix("DEBUG") {
                    ("INFO", s.trim().trim_start_matches(|c: char| !c.is_alphabetic()))
                } else {
                    ("INFO", rest)
                };

                // Nettoyer le prefix module (ex: "nitrite_lib: ")
                let message = msg
                    .trim_start_matches(|c: char| c.is_alphanumeric() || c == '_' || c == ':')
                    .trim()
                    .to_string();

                let message = if message.is_empty() { msg.to_string() } else { message };

                entries.push(AppLogEntry {
                    timestamp,
                    level: level.to_string(),
                    message,
                });
            }
        }

        // Limiter a 500 entrees
        if entries.len() > 500 {
            entries = entries.split_off(entries.len() - 500);
        }

        Ok(entries)
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

// === Portables ===

#[tauri::command]
async fn get_portable_apps() -> Result<Vec<installer::portables::PortableApp>, NiTriTeError> {
    Ok(installer::portables::get_all_portables())
}

#[tauri::command]
async fn check_portable_installed(app_id: String) -> Result<bool, NiTriTeError> {
    // Validation : interdire traversal de chemin
    if app_id.contains("..") || app_id.contains('/') || app_id.contains('\\') {
        return Ok(false);
    }
    let portables_dir = utils::paths::portables_dir();
    let app_dir = portables_dir.join(&app_id);
    Ok(app_dir.exists())
}

#[tauri::command]
async fn launch_portable(app_id: String) -> Result<(), NiTriTeError> {
    let portables = installer::portables::get_all_portables();
    let app = portables.iter().find(|a| a.id == app_id)
        .ok_or_else(|| NiTriTeError::System(format!("Application {} non trouvee", app_id)))?;

    let portables_dir = utils::paths::portables_dir();
    let exe_path = portables_dir.join(&app.folder_name).join(&app.exe_name);

    if !exe_path.exists() {
        return Err(NiTriTeError::System(format!(
            "Executable non trouve: {}. Telechargez d'abord l'application.",
            exe_path.display()
        )));
    }

    std::process::Command::new(&exe_path)
        .spawn()
        .map_err(|e| NiTriTeError::System(format!("Impossible de lancer {}: {}", app.name, e)))?;

    Ok(())
}

/// Vérifie si LibreHardwareMonitor.exe portable existe dans logiciel/
#[tauri::command]
async fn check_lhm_portable() -> bool {
    let dir = utils::paths::portables_dir();
    // Cherche dans logiciel/LibreHardwareMonitor/ ou logiciel/LHM/
    for folder in &["LibreHardwareMonitor", "LHM", "libre-hardware-monitor"] {
        let exe = dir.join(folder).join("LibreHardwareMonitor.exe");
        if exe.exists() { return true; }
    }
    false
}

/// Lance LibreHardwareMonitor portable depuis logiciel/ (en tant qu'admin via runas)
#[tauri::command]
async fn launch_lhm_portable() -> Result<(), NiTriTeError> {
    let dir = utils::paths::portables_dir();
    let mut exe_path = None;
    for folder in &["LibreHardwareMonitor", "LHM", "libre-hardware-monitor"] {
        let p = dir.join(folder).join("LibreHardwareMonitor.exe");
        if p.exists() { exe_path = Some(p); break; }
    }
    let exe = exe_path.ok_or_else(|| NiTriTeError::System(
        "LibreHardwareMonitor.exe introuvable. Placez-le dans logiciel/LibreHardwareMonitor/".to_string()
    ))?;

    #[cfg(target_os = "windows")]
    {
        // Lancement avec élévation UAC (requis pour accès WMI matériel)
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command",
                &format!("Start-Process -FilePath '{}' -Verb RunAs", exe.display())])
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| NiTriTeError::System(e.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
async fn launch_sdi() -> Result<(), NiTriTeError> {
    // Cherche SDI en remontant depuis l'exe (prod: a cote de l'exe, dev: jusqu'a la racine projet)
    let exe_dir = utils::paths::app_root_dir();
    let sdi_rel = std::path::Path::new("Drivers").join("SDI_RUS").join("SDI_x64_R2601.exe");

    let mut search_dir = exe_dir.clone();
    let mut found: Option<std::path::PathBuf> = None;
    for _ in 0..5 {
        let candidate = search_dir.join(&sdi_rel);
        if candidate.exists() { found = Some(candidate); break; }
        match search_dir.parent() {
            Some(p) => search_dir = p.to_path_buf(),
            None => break,
        }
    }

    let exe_path = found.ok_or_else(|| NiTriTeError::System(
        format!("SDI introuvable (cherche depuis {})", exe_dir.display())
    ))?;

    let sdi_dir = exe_path.parent().unwrap_or(exe_path.as_path());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // Start-Process cree un processus completement isole de Tauri
        // → fermer SDI n'affecte jamais Nitrite
        let cmd = format!(
            "Start-Process -FilePath '{}' -WorkingDirectory '{}'",
            exe_path.display(),
            sdi_dir.display()
        );
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &cmd])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| NiTriTeError::System(format!("Impossible de lancer SDI: {}", e)))?;
    }
    Ok(())
}

#[tauri::command]
async fn open_portables_dir() -> Result<(), NiTriTeError> {
    let dir = utils::paths::portables_dir();
    open::that(&dir).map_err(|e| NiTriTeError::System(e.to_string()))
}

/// Lance un .exe par chemin relatif depuis le dossier logiciel/
#[tauri::command]
async fn launch_exe(relative_path: String) -> Result<(), NiTriTeError> {
    let base = utils::paths::portables_dir();
    let full = base.join(std::path::Path::new(&relative_path));
    let canonical_base = base.canonicalize()
        .unwrap_or(base.clone());
    let canonical_full = full.canonicalize()
        .unwrap_or_else(|_| full.clone());
    if !canonical_full.starts_with(&canonical_base) {
        return Err(NiTriTeError::CommandDenied(
            "Chemin hors du répertoire portables autorisé".into(),
        ));
    }
    if !full.exists() {
        return Err(NiTriTeError::System(format!("Fichier non trouvé: {}", full.display())));
    }
    std::process::Command::new(&full)
        .spawn()
        .map_err(|e| NiTriTeError::System(format!("Lancement impossible: {}", e)))?;
    Ok(())
}

/// Génère un rapport batterie via powercfg et retourne le chemin HTML
#[tauri::command]
async fn run_battery_report() -> Result<String, NiTriTeError> {
    let output_path = std::env::temp_dir().join("nitrite-battery-report.html");
    let output_str = output_path.to_string_lossy().to_string();
    let out = output_str.clone();
    tokio::task::spawn_blocking(move || {
        let _ = std::process::Command::new("powercfg")
            .args(["/batteryreport", "/output", &out])
            .creation_flags(0x08000000)
            .status();
        Ok::<(), NiTriTeError>(())
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))??;
    Ok(output_str)
}

// === Exports (dev + prod) ===

fn nitrite_export_dir() -> Result<std::path::PathBuf, NiTriTeError> {
    let docs = dirs::document_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Documents")))
        .ok_or_else(|| NiTriTeError::System("Dossier Documents introuvable".into()))?;
    let dir = docs.join("NiTriTe").join("exports");
    std::fs::create_dir_all(&dir)
        .map_err(|e| NiTriTeError::System(format!("mkdir exports: {}", e)))?;
    Ok(dir)
}

#[tauri::command]
async fn get_export_dir() -> Result<String, NiTriTeError> {
    let dir = tokio::task::spawn_blocking(nitrite_export_dir)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))??;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
async fn save_export_file(filename: String, content: String) -> Result<String, NiTriTeError> {
    let file_path = tokio::task::spawn_blocking(move || -> Result<std::path::PathBuf, NiTriTeError> {
        let dir = nitrite_export_dir()?;
        let path = dir.join(&filename);
        std::fs::write(&path, content.as_bytes())
            .map_err(|e| NiTriTeError::System(format!("Écriture {}: {}", filename, e)))?;
        Ok(path)
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))??;
    Ok(file_path.to_string_lossy().to_string())
}

