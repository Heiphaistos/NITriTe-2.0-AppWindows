use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstallRecord {
    pub app_id: String,
    pub app_name: String,
    pub installed_at: String,
    pub success: bool,
    pub method: String, // "winget" | "choco" | "url"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FavoritesData {
    pub favorites: Vec<String>,       // app_ids
    pub history: Vec<InstallRecord>,  // dernières installations
}

fn data_path() -> PathBuf {
    // Portable d'abord (dossier config a cote de l'exe) — sans ca, favoris et
    // historique d'installation atterrissaient dans %LOCALAPPDATA%\NiTriTe,
    // une trace sur le PC client que la version portable est censee eviter.
    crate::utils::paths::config_dir().join("favorites.json")
}

fn load_data() -> FavoritesData {
    let path = data_path();
    if let Ok(raw) = fs::read_to_string(&path) {
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        FavoritesData::default()
    }
}

fn save_data(data: &FavoritesData) -> Result<(), String> {
    let path = data_path();
    let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

/// Retourne les favoris et l'historique
// Anti-freeze : I/O fichier (load_data) est bloquant — jamais inline sur
// le thread de commande.
#[tauri::command]
pub async fn get_favorites_data() -> FavoritesData {
    tokio::task::spawn_blocking(get_favorites_data_blocking)
        .await
        .unwrap_or_default()
}

fn get_favorites_data_blocking() -> FavoritesData {
    load_data()
}

/// Ajoute ou retire un app_id des favoris
// Anti-freeze : I/O fichier est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn toggle_favorite(app_id: String) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || toggle_favorite_blocking(app_id))
        .await
        .map_err(|e| e.to_string())?
}

fn toggle_favorite_blocking(app_id: String) -> Result<bool, String> {
    let mut data = load_data();
    if let Some(pos) = data.favorites.iter().position(|f| f == &app_id) {
        data.favorites.remove(pos);
        save_data(&data)?;
        Ok(false)
    } else {
        data.favorites.push(app_id);
        save_data(&data)?;
        Ok(true)
    }
}

/// Enregistre une installation dans l'historique
// Anti-freeze : I/O fichier est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn log_install(app_id: String, app_name: String, success: bool, method: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || log_install_blocking(app_id, app_name, success, method))
        .await
        .map_err(|e| e.to_string())?
}

fn log_install_blocking(app_id: String, app_name: String, success: bool, method: String) -> Result<(), String> {
    let mut data = load_data();
    let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    data.history.insert(0, InstallRecord {
        app_id,
        app_name,
        installed_at: now,
        success,
        method,
    });
    // Garder 200 entrées max
    data.history.truncate(200);
    save_data(&data)
}

/// Efface l'historique d'installation
// Anti-freeze : I/O fichier est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn clear_install_history() -> Result<(), String> {
    tokio::task::spawn_blocking(clear_install_history_blocking)
        .await
        .map_err(|e| e.to_string())?
}

fn clear_install_history_blocking() -> Result<(), String> {
    let mut data = load_data();
    data.history.clear();
    save_data(&data)
}
