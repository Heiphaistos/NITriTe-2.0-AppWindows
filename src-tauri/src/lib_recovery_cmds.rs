
// === Clonage Système ===

#[tauri::command]
async fn get_disks_for_clone() -> Result<Vec<system::clone::DiskInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(system::clone::get_disks)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn start_system_image(target_drive: String, window: tauri::Window) -> Result<system::clone::CloneResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::clone::create_system_image(target_drive, &window))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn start_robocopy_clone(source_drive: String, target_drive: String, window: tauri::Window) -> Result<system::clone::CloneResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::clone::clone_with_robocopy(source_drive, target_drive, &window))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

// === Récupération de Données ===

#[tauri::command]
async fn list_shadow_copies() -> Result<Vec<system::data_recovery::ShadowCopy>, NiTriTeError> {
    tokio::task::spawn_blocking(system::data_recovery::list_shadow_copies)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn browse_shadow_copy(device_path: String, relative_path: String) -> Result<Vec<system::data_recovery::RecoveredFile>, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::data_recovery::browse_shadow_copy(device_path, relative_path))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn restore_from_shadow(source_path: String, target_folder: String) -> Result<system::data_recovery::RestoreResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::data_recovery::restore_from_shadow(source_path, target_folder))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn scan_recycle_bin() -> Result<Vec<system::data_recovery::RecoveredFile>, NiTriTeError> {
    tokio::task::spawn_blocking(system::data_recovery::scan_recycle_bin)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn restore_recycle_bin_item(original_path: String) -> Result<system::data_recovery::RestoreResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::data_recovery::restore_recycle_bin_item(original_path))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn scan_deleted_files(drive: String) -> Result<Vec<system::data_recovery::RecoveredFile>, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::data_recovery::scan_deleted_files(drive))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn search_shadow_copy(device_path: String, query: String, base_path: String) -> Result<Vec<system::data_recovery::RecoveredFile>, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::data_recovery::search_shadow_copy(device_path, query, base_path))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn restore_files_batch(files: Vec<String>, target_folder: String) -> Result<system::data_recovery::BatchRestoreResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::data_recovery::restore_files_batch(files, target_folder))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn list_connected_disks() -> Result<Vec<system::data_recovery::DiskInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(system::data_recovery::list_connected_disks)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn browse_disk_path(path: String) -> Result<Vec<system::data_recovery::DiskEntry>, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::data_recovery::browse_disk_path(path))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn recover_files_safe(
    files: Vec<String>,
    target_folder: String,
    safe_mode: bool,
    window: tauri::Window,
) -> Result<system::data_recovery::BatchRestoreResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::data_recovery::recover_files_safe(files, target_folder, safe_mode, &window))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn get_user_profile_folders() -> Result<Vec<system::data_recovery::UserFolder>, NiTriTeError> {
    tokio::task::spawn_blocking(system::data_recovery::get_user_profile_folders)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn backup_user_folders(
    folders: Vec<String>,
    target: String,
    window: tauri::Window,
) -> Result<system::data_recovery::BackupResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        system::data_recovery::backup_user_folders(folders, target, &window)
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))
}

// === Désinstallateur Propre ===

#[tauri::command]
async fn list_installed_apps_for_uninstall() -> Result<Vec<installer::uninstaller::InstalledApp>, NiTriTeError> {
    tokio::task::spawn_blocking(installer::uninstaller::list_installed_apps)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn uninstall_app_clean(
    app_name: String,
    uninstall_string: String,
    publisher: String,
    window: tauri::Window,
) -> Result<installer::uninstaller::UninstallResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        installer::uninstaller::uninstall_app_clean(app_name, uninstall_string, publisher, &window)
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))
}

// === Preview résidus (avant désinstallation) ===

#[tauri::command]
async fn preview_residuals(
    app_name: String,
    publisher: String,
) -> Result<Vec<String>, NiTriTeError> {
    tokio::task::spawn_blocking(move || installer::uninstaller::preview_residuals(app_name, publisher))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

// === Suppression définitive des résidus ===

#[tauri::command]
async fn delete_residuals(paths: Vec<String>) -> Result<installer::uninstaller::ResidualCleanResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || installer::uninstaller::delete_residuals(paths))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

// === Extraction + suppression des résidus ===

#[tauri::command]
async fn extract_residuals(paths: Vec<String>, target: String) -> Result<installer::uninstaller::ResidualCleanResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || installer::uninstaller::extract_residuals(paths, target))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

// === Comparaison Shadow Copy vs Système Actuel ===

#[tauri::command]
async fn compare_shadow_with_current(
    device_path: String,
    sub_path: String,
    live_path: String,
) -> Result<Vec<system::data_recovery::ComparedFile>, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::data_recovery::compare_shadow_with_current(device_path, sub_path, live_path))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

// === Scan USN Journal tous lecteurs NTFS ===

#[tauri::command]
async fn scan_all_deleted_files() -> Result<Vec<system::data_recovery::RecoveredFile>, NiTriTeError> {
    tokio::task::spawn_blocking(system::data_recovery::scan_all_deleted_files)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

// === Gestionnaire de Partitions ===

#[tauri::command]
async fn get_disks_smart() -> Result<Vec<system::partition::DiskSmartInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(system::partition::get_disks_smart)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn get_partition_list() -> Result<Vec<system::partition::PartitionDetail>, NiTriTeError> {
    tokio::task::spawn_blocking(system::partition::get_partition_list)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn format_partition_cmd(letter: String, fs: String, label: String) -> Result<String, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::partition::format_partition(letter, fs, label))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
        .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn assign_drive_letter_cmd(disk_index: u32, part_index: u32, letter: String) -> Result<String, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::partition::assign_drive_letter(disk_index, part_index, letter))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
        .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn create_partition_cmd(disk_index: u32, size_mb: Option<u32>) -> Result<String, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::partition::create_partition(disk_index, size_mb))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
        .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn delete_partition_cmd(disk_index: u32, part_index: u32) -> Result<String, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::partition::delete_partition(disk_index, part_index))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
        .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn initialize_disk_cmd(disk_index: u32, style: String) -> Result<String, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::partition::initialize_disk(disk_index, style))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
        .map_err(NiTriTeError::System)
}

// === Shadow Copy Gestion ===

#[tauri::command]
async fn create_shadow_copy_cmd(volume: String) -> Result<String, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::data_recovery::create_shadow_copy(volume))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
        .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn delete_shadow_copy_cmd(shadow_id: String) -> Result<String, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::data_recovery::delete_shadow_copy(shadow_id))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
        .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn open_in_explorer(path: String) -> Result<(), NiTriTeError> {
    system::data_recovery::open_in_explorer(path)
        .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn get_ntfs_drives() -> Result<Vec<String>, NiTriTeError> {
    tokio::task::spawn_blocking(system::data_recovery::get_ntfs_drives)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

// === Récupération avancée (image disque, surface test, scan MFT, rapport) ===

#[tauri::command]
async fn create_disk_image_cmd(
    disk_index: u32,
    output_path: String,
    window: tauri::Window,
) -> Result<system::advanced_recovery::DiskImageResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        system::advanced_recovery::create_disk_image(disk_index, output_path, &window)
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn surface_test_volume_cmd(
    drive_letter: String,
    window: tauri::Window,
) -> Result<system::advanced_recovery::SurfaceTestResult, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        system::advanced_recovery::surface_test_volume(drive_letter, &window)
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn deep_mft_scan_advanced_cmd(
    drive: String,
) -> Result<Vec<system::advanced_recovery::DeepMftFile>, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::advanced_recovery::deep_mft_scan_advanced(drive))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn generate_recovery_report_cmd(
    title: String,
    files_json: String,
    output_path: String,
) -> Result<String, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        system::advanced_recovery::generate_recovery_report(title, files_json, output_path)
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
    .map_err(NiTriTeError::System)
}

// === Partition avancé (resize, MBR, lost partitions) ===

#[tauri::command]
async fn get_partition_resize_limits_cmd(
    disk_index: u32,
    part_index: u32,
) -> Result<system::partition::PartitionSizeLimits, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        system::partition::get_partition_resize_limits(disk_index, part_index)
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
    .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn resize_partition_cmd(
    disk_index: u32,
    part_index: u32,
    new_size_mb: u64,
) -> Result<String, NiTriTeError> {
    tokio::task::spawn_blocking(move || {
        system::partition::resize_partition_ps(disk_index, part_index, new_size_mb)
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
    .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn backup_mbr_cmd(disk_index: u32, output_path: String) -> Result<String, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::partition::backup_mbr(disk_index, output_path))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
        .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn restore_mbr_cmd(disk_index: u32, mbr_path: String) -> Result<String, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::partition::restore_mbr(disk_index, mbr_path))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
        .map_err(NiTriTeError::System)
}

#[tauri::command]
async fn scan_lost_partitions_cmd(
    disk_index: u32,
) -> Result<Vec<system::partition::LostPartition>, NiTriTeError> {
    tokio::task::spawn_blocking(move || system::partition::scan_lost_partitions(disk_index))
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

// === Save content to path (dialog-driven export) ===
// Le chemin vient toujours d'une save dialog Tauri — le choix utilisateur est le consentement.

#[tauri::command]
async fn save_content_to_path(path: String, content: String) -> Result<(), NiTriTeError> {
    // Extensions exécutables interdites
    let blocked_exts = ["exe", "dll", "bat", "cmd", "ps1", "vbs", "hta", "scr", "msi", "inf", "com", "pif", "reg"];

    // Canonicalize préalable pour résoudre les .. et symlinks
    // On canonicalize le dossier parent (le fichier n'existe pas encore)
    let raw_path = std::path::Path::new(&path);

    if let Some(ext) = raw_path.extension().and_then(|e| e.to_str()) {
        if blocked_exts.contains(&ext.to_lowercase().as_str()) {
            return Err(NiTriTeError::CommandDenied(
                format!("Extension exécutable interdite: .{}", ext),
            ));
        }
    }

    // Canonicalize le répertoire parent pour résoudre les traversals (..)
    let canonical_parent = if let Some(parent) = raw_path.parent() {
        if parent.as_os_str().is_empty() {
            return Err(NiTriTeError::CommandDenied("Chemin sans répertoire parent".into()));
        }
        // Crée le dossier parent si nécessaire avant canonicalize
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| NiTriTeError::System(format!("Impossible de créer le dossier: {}", e)))?;
        std::fs::canonicalize(parent)
            .map_err(|e| NiTriTeError::System(format!("Chemin invalide: {}", e)))?
    } else {
        return Err(NiTriTeError::CommandDenied("Chemin sans répertoire parent".into()));
    };

    let canonical_str = canonical_parent.to_string_lossy().to_lowercase();

    // Whitelist des répertoires autorisés (résolu après canonicalize)
    // Couvre Desktop, Documents, Downloads, répertoire de données de l'app
    let user_profile = std::env::var("USERPROFILE").unwrap_or_default().to_lowercase();
    let appdata_local = std::env::var("LOCALAPPDATA").unwrap_or_default().to_lowercase();
    let app_data_dir = format!(r"{}\nitrite", appdata_local);

    let allowed_prefixes: &[&str] = &[
        // Dossiers utilisateur classiques
        &format!(r"{}\desktop", user_profile),
        &format!(r"{}\documents", user_profile),
        &format!(r"{}\downloads", user_profile),
        // Dossier de données de l'application
        &app_data_dir,
    ];

    let in_allowed = allowed_prefixes.iter().any(|prefix| {
        !prefix.is_empty() && canonical_str.starts_with(prefix.as_ref() as &str)
    });

    // Autorise aussi les chemins sur d'autres lecteurs (clé USB) hors C:\ système
    // mais bloque explicitement les répertoires système Windows
    let is_system_path = canonical_str.starts_with(r"c:\windows")
        || canonical_str.starts_with(r"c:\program files")
        || canonical_str.starts_with(r"c:\programdata");

    if is_system_path {
        return Err(NiTriTeError::CommandDenied(
            format!("Écriture dans un répertoire système interdite: {}", canonical_parent.display()),
        ));
    }

    if !in_allowed {
        // Pour les chemins hors whitelist (ex: clé USB D:\, E:\),
        // on accepte uniquement si ce n'est pas un chemin système
        // et si la lettre de lecteur n'est pas C: (ou si c'est C: vérifié ci-dessus)
        let first_two: String = canonical_str.chars().take(2).collect();
        let is_c_drive = first_two == "c:";
        if is_c_drive {
            return Err(NiTriTeError::CommandDenied(
                format!(
                    "Chemin hors des répertoires autorisés (Bureau, Documents, Téléchargements, données app): {}",
                    canonical_parent.display()
                ),
            ));
        }
        // Autre lecteur (clé USB, disque externe) → autorisé
    }

    let final_path = canonical_parent.join(
        raw_path.file_name().ok_or_else(|| NiTriTeError::CommandDenied("Nom de fichier manquant".into()))?
    );

    tokio::fs::write(&final_path, content.as_bytes())
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))
}

// === Config ===

#[tauri::command]
async fn get_config(state: tauri::State<'_, AppState>) -> Result<AppConfig, NiTriTeError> {
    let config = state.config.lock().await;
    Ok(config.clone())
}

#[tauri::command]
async fn save_config(
    config: AppConfig,
    state: tauri::State<'_, AppState>,
) -> Result<(), NiTriTeError> {
    config.save().map_err(|e| NiTriTeError::System(e.to_string()))?;
    let mut current = state.config.lock().await;
    *current = config;
    Ok(())
}

// === Open URL/Path ===

#[tauri::command]
async fn open_url(url: String) -> Result<(), NiTriTeError> {
    let allowed = url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("ms-settings:")
        || url.starts_with("ms-windows-store:");
    if !allowed {
        return Err(NiTriTeError::CommandDenied(
            format!("Schéma d'URL non autorisé: {}", url),
        ));
    }
    open::that(&url).map_err(|e| NiTriTeError::System(e.to_string()))
}

#[tauri::command]
async fn open_path(path: String) -> Result<(), NiTriTeError> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err(NiTriTeError::System(format!("Chemin introuvable: {}", path)));
    }
    // Refuser les exécutables (extension connue dangereuse)
    let blocked_exts = ["exe", "bat", "cmd", "ps1", "vbs", "hta", "scr", "com", "pif", "cpl", "msi", "dll"];
    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        if blocked_exts.contains(&ext.to_lowercase().as_str()) {
            return Err(NiTriTeError::CommandDenied(
                format!("Ouverture d'exécutable interdite: .{}", ext),
            ));
        }
    } else if p.is_file() {
        // Fichier sans extension : vérifier les magic bytes pour détecter les exécutables Windows
        // MZ (0x4D 0x5A) = en-tête PE (EXE, DLL, COM natif 32/64 bits)
        let magic = std::fs::read(&path)
            .ok()
            .and_then(|b| if b.len() >= 2 { Some([b[0], b[1]]) } else { None });
        if magic == Some([0x4D, 0x5A]) {
            return Err(NiTriTeError::CommandDenied(
                "Ouverture d'exécutable Windows (signature MZ) interdite.".into(),
            ));
        }
    }
    open::that(&path).map_err(|e| NiTriTeError::System(e.to_string()))
}

// === Execute tool command (cmd or ms-settings) ===

/// Vérifie que la commande ne contient pas de métacaractères CMD/PS dangereux.
/// Autorise %VAR_NAME% (variables d'environnement Windows légitimes).
fn has_shell_metacharacters(cmd: &str) -> bool {
    // Bloquants absolus : délimiteurs de commandes, redirections, injections PS
    let dangerous: &[&str] = &["&", "|", ";", ">", "<", "`", "$(", "!", "\n", "\r"];
    if dangerous.iter().any(|c| cmd.contains(c)) {
        return true;
    }
    // % dangereux uniquement si ce n'est PAS une variable env Windows légale (%WORD%)
    // Ex: %TEMP%, %PROGRAMFILES%, %USERPROFILE% → autorisé
    // %1, %%var, % seul, nombre impair de % → bloqué
    if cmd.contains('%') {
        // Algorithme : parcourt la chaîne en cherchant des paires %NAME%
        // Tout % qui ne fait pas partie d'une paire %ALPHANUM_NAME% valide est dangereux
        let bytes = cmd.as_bytes();
        let mut i = 0;
        let mut has_dangerous_pct = false;
        while i < bytes.len() {
            if bytes[i] != b'%' {
                i += 1;
                continue;
            }
            // Cherche le % fermant
            let start = i;
            i += 1;
            let mut end = None;
            while i < bytes.len() {
                if bytes[i] == b'%' {
                    end = Some(i);
                    break;
                }
                // Le nom de variable ne doit contenir que alphanum et underscore
                if !bytes[i].is_ascii_alphanumeric() && bytes[i] != b'_' {
                    break; // caractère invalide dans le nom → % ouvrant dangereux
                }
                i += 1;
            }
            match end {
                Some(close) if close > start + 1 => {
                    // %VALID_NAME% trouvé → accepté, avance après le %
                    i = close + 1;
                }
                _ => {
                    // % isolé, %% ou nom invalide → dangereux
                    has_dangerous_pct = true;
                    break;
                }
            }
        }
        if has_dangerous_pct {
            return true;
        }
    }
    false
}

#[tauri::command]
async fn execute_tool(command: String, is_url: bool) -> Result<(), NiTriTeError> {
    if is_url || command.starts_with("ms-settings:") || command.starts_with("http") {
        // Déléguer à open_url pour la validation de schéma
        let allowed = command.starts_with("http://")
            || command.starts_with("https://")
            || command.starts_with("ms-settings:")
            || command.starts_with("ms-windows-store:");
        if !allowed {
            return Err(NiTriTeError::CommandDenied(
                format!("Schéma non autorisé: {}", command),
            ));
        }
        open::that(&command).map_err(|e| NiTriTeError::System(e.to_string()))
    } else {
        if has_shell_metacharacters(&command) {
            return Err(NiTriTeError::CommandDenied(
                "Métacaractères shell détectés dans la commande".into(),
            ));
        }
        tokio::task::spawn_blocking(move || {
            // Ouvre un terminal visible avec /K pour garder la fenêtre ouverte après la commande
            std::process::Command::new("cmd")
                .args(["/C", "start", "cmd", "/K", &command])
                .creation_flags(0x08000000)
                .spawn()
                .map_err(|e| NiTriTeError::System(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
    }
}

// === Extended Info (BIOS, Batterie, Dossiers) ===

#[tauri::command]
async fn get_bios_info() -> Result<system::extended_info::BiosInfo, NiTriTeError> {
    tokio::task::spawn_blocking(system::extended_info::get_bios_info)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn get_battery_extended() -> Result<Option<system::extended_info::BatteryInfo>, NiTriTeError> {
    tokio::task::spawn_blocking(system::extended_info::get_battery_extended)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn get_folder_sizes() -> Result<Vec<system::extended_info::FolderSizeEntry>, NiTriTeError> {
    tokio::task::spawn_blocking(system::extended_info::get_folder_sizes)
        .await
        .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
async fn run_total_scan(window: tauri::Window) -> Result<system::total_scan::ScanResult, NiTriTeError> {
    system::total_scan::run_total_scan(window).await
}

