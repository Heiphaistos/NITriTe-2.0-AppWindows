use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::utils::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub version: String,
    pub config: serde_json::Value,
}

fn profiles_dir() -> PathBuf {
    let dir = paths::config_dir().join("profiles");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

pub fn list_profiles() -> Vec<Profile> {
    let dir = profiles_dir();
    let mut profiles = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(profile) = serde_json::from_str::<Profile>(&content) {
                        profiles.push(profile);
                    }
                }
            }
        }
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    profiles
}

pub fn save_profile(profile: &Profile) -> Result<(), std::io::Error> {
    let dir = profiles_dir();
    let filename = sanitize_filename(&profile.name);
    let path = dir.join(format!("{}.json", filename));
    let json = serde_json::to_string_pretty(profile)
        .map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}

pub fn delete_profile(name: &str) -> Result<(), std::io::Error> {
    let dir = profiles_dir();
    let filename = sanitize_filename(name);
    let path = dir.join(format!("{}.json", filename));
    if path.exists() {
        std::fs::remove_file(path)
    } else {
        Ok(())
    }
}

pub fn profile_exists(name: &str) -> bool {
    let dir = profiles_dir();
    let filename = sanitize_filename(name);
    dir.join(format!("{}.json", filename)).exists()
}

pub fn export_profile_json(name: &str) -> Option<String> {
    let dir = profiles_dir();
    let filename = sanitize_filename(name);
    let path = dir.join(format!("{}.json", filename));
    std::fs::read_to_string(path).ok()
}

pub fn import_profile_from_json(json: &str) -> Result<Profile, String> {
    serde_json::from_str::<Profile>(json).map_err(|e| e.to_string())
}
