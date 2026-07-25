use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppStats {
    pub installs_count: u64,
    pub cleanups_count: u64,
    pub scans_count: u64,
    pub scripts_run: u64,
    pub exports_count: u64,
    pub actions_today: u64,
    pub last_action: String,
    pub action_counts: HashMap<String, u64>,
}

fn stats_path() -> PathBuf {
    // Portable d'abord — meme raison que favorites.rs : %LOCALAPPDATA%\NiTriTe
    // etait une trace laissee sur le PC client, hors du dossier de Nitrite.
    crate::utils::paths::config_dir().join("stats.json")
}

fn load_stats() -> AppStats {
    let path = stats_path();
    if let Ok(raw) = fs::read_to_string(&path) {
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        AppStats::default()
    }
}

fn save_stats(stats: &AppStats) -> Result<(), String> {
    let json = serde_json::to_string_pretty(stats).map_err(|e| e.to_string())?;
    fs::write(stats_path(), json).map_err(|e| e.to_string())
}

/// Retourne les statistiques d'utilisation
// Anti-freeze : I/O fichier (load_stats) est bloquant — jamais inline
// sur le thread de commande.
#[tauri::command]
pub async fn get_app_stats() -> AppStats {
    tokio::task::spawn_blocking(get_app_stats_blocking)
        .await
        .unwrap_or_default()
}

fn get_app_stats_blocking() -> AppStats {
    load_stats()
}

/// Incrémente un compteur nommé
// Anti-freeze : I/O fichier est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn log_action(action: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || log_action_blocking(action))
        .await
        .map_err(|e| e.to_string())?
}

fn log_action_blocking(action: String) -> Result<(), String> {
    let mut stats = load_stats();
    let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    stats.last_action = now;

    *stats.action_counts.entry(action.clone()).or_insert(0) += 1;

    match action.as_str() {
        "install" => stats.installs_count += 1,
        "cleanup" => stats.cleanups_count += 1,
        "scan"    => stats.scans_count += 1,
        "script"  => stats.scripts_run += 1,
        "export"  => stats.exports_count += 1,
        _ => {}
    }
    stats.actions_today += 1;

    save_stats(&stats)
}

/// Remet les statistiques à zéro
// Anti-freeze : I/O fichier est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn reset_stats() -> Result<(), String> {
    tokio::task::spawn_blocking(reset_stats_blocking)
        .await
        .map_err(|e| e.to_string())?
}

fn reset_stats_blocking() -> Result<(), String> {
    save_stats(&AppStats::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_stats_default_zeroed() {
        let s = AppStats::default();
        assert_eq!(s.installs_count, 0);
        assert_eq!(s.cleanups_count, 0);
        assert_eq!(s.scans_count, 0);
        assert_eq!(s.scripts_run, 0);
        assert_eq!(s.exports_count, 0);
        assert_eq!(s.actions_today, 0);
        assert!(s.action_counts.is_empty());
    }

    #[test]
    fn action_counts_increments() {
        let mut s = AppStats::default();
        *s.action_counts.entry("scan".to_string()).or_insert(0) += 1;
        *s.action_counts.entry("scan".to_string()).or_insert(0) += 1;
        assert_eq!(s.action_counts["scan"], 2);
    }

    #[test]
    fn stats_serializes_to_valid_json() {
        let s = AppStats {
            installs_count: 3,
            scans_count: 7,
            last_action: "2026-04-01T12:00:00".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&s).expect("serialization failed");
        assert!(json.contains("\"installs_count\":3"));
        assert!(json.contains("\"scans_count\":7"));
    }
}
