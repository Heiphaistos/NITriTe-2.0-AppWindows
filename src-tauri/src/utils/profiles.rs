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

/// Nom de fichier final pour un profil : sanitize_filename() n'est PAS injective
/// (tout caractère spécial devient '_'), donc deux noms distincts et plausibles
/// ("Bureau/Pro" et "Bureau Pro") produiraient le même fichier -> écrasement
/// silencieux du premier profil par le second. Un suffixe de hash déterministe
/// du nom ORIGINAL (pas du nom sanitisé) désambiguïse sans casser la lisibilité.
fn profile_filename(name: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    format!("{}_{:016x}.json", sanitize_filename(name), hasher.finish())
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
    let path = dir.join(profile_filename(&profile.name));
    let json = serde_json::to_string_pretty(profile)
        .map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}

pub fn delete_profile(name: &str) -> Result<(), std::io::Error> {
    let dir = profiles_dir();
    let path = dir.join(profile_filename(name));
    if path.exists() {
        std::fs::remove_file(path)
    } else {
        Ok(())
    }
}

pub fn profile_exists(name: &str) -> bool {
    let dir = profiles_dir();
    dir.join(profile_filename(name)).exists()
}

pub fn export_profile_json(name: &str) -> Option<String> {
    let dir = profiles_dir();
    let path = dir.join(profile_filename(name));
    std::fs::read_to_string(path).ok()
}

pub fn import_profile_from_json(json: &str) -> Result<Profile, String> {
    serde_json::from_str::<Profile>(json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_alphanumeric_unchanged() {
        assert_eq!(sanitize_filename("myProfile123"), "myProfile123");
        assert_eq!(sanitize_filename("default-profile"), "default-profile");
        assert_eq!(sanitize_filename("my_profile"), "my_profile");
    }

    #[test]
    fn sanitize_path_traversal_blocked() {
        // "../evil" → 3 replaced chars (./.) + "evil"
        assert_eq!(sanitize_filename("../evil"), "___evil");
        // "../../etc/passwd" → 6 replaced chars (../..) + etc + _ + passwd
        assert_eq!(sanitize_filename("../../etc/passwd"), "______etc_passwd");
    }

    #[test]
    fn sanitize_spaces_replaced() {
        assert_eq!(sanitize_filename("my profile name"), "my_profile_name");
    }

    #[test]
    fn sanitize_special_chars_replaced() {
        assert_eq!(sanitize_filename("profile<>|*"), "profile____");
        assert_eq!(sanitize_filename("name.json"), "name_json");
    }

    #[test]
    fn sanitize_empty_stays_empty() {
        assert_eq!(sanitize_filename(""), "");
    }

    #[test]
    fn profile_filename_distinct_names_no_collision() {
        // Régression : "Bureau/Pro" et "Bureau Pro" sanitisent au même résultat
        // ("Bureau_Pro") via sanitize_filename seul -> sans le hash de
        // désambiguïsation, save_profile("Bureau Pro") écraserait silencieusement
        // le fichier de "Bureau/Pro" (et vice-versa).
        assert_eq!(sanitize_filename("Bureau/Pro"), sanitize_filename("Bureau Pro"));
        assert_ne!(profile_filename("Bureau/Pro"), profile_filename("Bureau Pro"));
    }

    #[test]
    fn profile_filename_same_name_deterministic() {
        // Doit être stable pour que delete/export retrouvent bien le fichier
        // écrit par save_profile avec le même nom.
        assert_eq!(profile_filename("Config Dev"), profile_filename("Config Dev"));
    }

    #[test]
    fn import_valid_json_profile() {
        let json = r#"{"name":"Test","description":"desc","created_at":"2026-01-01","version":"1.0","config":{}}"#;
        let p = import_profile_from_json(json).unwrap();
        assert_eq!(p.name, "Test");
        assert_eq!(p.version, "1.0");
    }

    #[test]
    fn import_invalid_json_returns_error() {
        assert!(import_profile_from_json("{not valid json}").is_err());
    }
}
