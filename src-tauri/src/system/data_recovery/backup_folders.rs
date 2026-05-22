use serde::Serialize;
use tauri::Emitter;
#[cfg(target_os = "windows")]
use super::run_ps;

#[derive(Debug, Clone, Serialize)]
pub struct UserFolder {
    pub name: String,
    pub path: String,
    pub size_mb: u64,
    pub shadow_relative: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupResult {
    pub success: bool,
    pub message: String,
    pub duration_secs: u64,
    pub folders_count: usize,
}

/// Retourne les dossiers standard du profil utilisateur avec leur taille et chemin VSS relatif
pub fn get_user_profile_folders() -> Vec<UserFolder> {
    let ps = r#"
try {
    $result = @()
    $keys = @(
        @{ name='Documents';       path=[Environment]::GetFolderPath('Personal') },
        @{ name='Bureau';          path=[Environment]::GetFolderPath('Desktop') },
        @{ name='Images';          path=[Environment]::GetFolderPath('MyPictures') },
        @{ name='Videos';          path=[Environment]::GetFolderPath('MyVideos') },
        @{ name='Musique';         path=[Environment]::GetFolderPath('MyMusic') },
        @{ name='Telechargements'; path=(Join-Path ([Environment]::GetFolderPath('UserProfile')) 'Downloads') }
    )
    foreach ($k in $keys) {
        if (Test-Path $k.path) {
            $sum = 0
            try {
                $items = Get-ChildItem $k.path -Recurse -Force -ErrorAction SilentlyContinue
                if ($items) {
                    $measure = $items | Measure-Object -Property Length -Sum -ErrorAction SilentlyContinue
                    if ($measure.Sum) { $sum = $measure.Sum }
                }
            } catch {}
            $rel = $k.path -replace '^[A-Za-z]:\\', ''
            $result += @{
                name            = $k.name
                path            = $k.path
                size_mb         = [math]::Round($sum / 1MB, 0)
                shadow_relative = $rel
            }
        }
    }
    if ($result.Count -eq 0) { Write-Output '[]' }
    else { $result | ConvertTo-Json -Compress -Depth 2 }
} catch { Write-Output '[]' }
"#;
    run_ps(ps)
        .and_then(|t| {
            let t = t.trim();
            if t.is_empty() || t == "[]" { return Some(vec![]); }
            let j = if t.starts_with('{') { format!("[{}]", t) } else { t.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()
        })
        .map(|vals| vals.into_iter().filter_map(|v| {
            Some(UserFolder {
                name:            v["name"].as_str()?.to_string(),
                path:            v["path"].as_str().unwrap_or("").to_string(),
                size_mb:         v["size_mb"].as_u64().unwrap_or(0),
                shadow_relative: v["shadow_relative"].as_str().unwrap_or("").to_string(),
            })
        }).collect())
        .unwrap_or_default()
}

/// Sauvegarde les dossiers sélectionnés vers un dossier cible via Robocopy.
pub fn backup_user_folders(
    folders: Vec<String>,
    target: String,
    window: &tauri::Window,
) -> BackupResult {
    let start = std::time::Instant::now();
    let total = folders.len();

    if total == 0 {
        return BackupResult {
            success: false,
            message: "Aucun dossier sélectionné.".into(),
            duration_secs: 0,
            folders_count: 0,
        };
    }

    if let Err(e) = std::fs::create_dir_all(&target) {
        return BackupResult {
            success: false,
            message: format!(
                "Impossible de créer le dossier de destination: {}. Vérifiez le chemin et les permissions.",
                e
            ),
            duration_secs: 0,
            folders_count: 0,
        };
    }
    let test_file = std::path::Path::new(&target).join(".nitrite_write_test");
    if let Err(e) = std::fs::write(&test_file, b"ok") {
        return BackupResult {
            success: false,
            message: format!(
                "Destination non accessible en écriture ({}). Vérifiez les droits sur le dossier cible.",
                e
            ),
            duration_secs: 0,
            folders_count: 0,
        };
    }
    let _ = std::fs::remove_file(&test_file);

    let mut succeeded = 0usize;
    let mut errors: Vec<String> = Vec::new();

    for (i, folder_path) in folders.iter().enumerate() {
        let folder_name = std::path::Path::new(folder_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("dossier_{}", i));

        let pct = ((i as f32 / total as f32) * 88.0) as u32 + 5;
        let _ = window.emit("backup-profile-progress", serde_json::json!({
            "folder": &folder_name,
            "percent": pct,
            "message": format!("({}/{}) Copie de {}...", i + 1, total, folder_name)
        }));

        if !std::path::Path::new(folder_path).exists() {
            errors.push(format!("{} : dossier source introuvable", folder_name));
            continue;
        }

        let dst = format!("{}\\{}", target.trim_end_matches('\\'), folder_name);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let status = std::process::Command::new("robocopy")
                .args([
                    folder_path.as_str(),
                    dst.as_str(),
                    "/E",
                    "/B",
                    "/DCOPY:DA",
                    "/COPY:DAT",
                    "/R:1", "/W:2",
                    "/MT:4",
                    "/NFL", "/NDL", "/NP",
                ])
                .creation_flags(0x08000000)
                .status();

            match status {
                Ok(s) if s.code().map(|c| c < 8).unwrap_or(false) => {
                    succeeded += 1;
                }
                Ok(s) => {
                    let code = s.code().unwrap_or(-1);
                    if code == 8 {
                        succeeded += 1;
                        errors.push(format!("{} : copié avec avertissements (fichiers verrouillés ignorés)", folder_name));
                    } else {
                        errors.push(format!("{} : échec robocopy code {}", folder_name, code));
                    }
                }
                Err(e) => {
                    errors.push(format!("{} : {}", folder_name, e));
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            errors.push(format!("{} : non supporté hors Windows", folder_name));
        }
    }

    let _ = window.emit("backup-profile-progress", serde_json::json!({
        "folder": "done",
        "percent": 100,
        "message": if errors.is_empty() {
            format!("{}/{} dossier(s) sauvegardé(s) avec succès", succeeded, total)
        } else {
            format!("{}/{} succès — {} avertissement(s)/erreur(s)", succeeded, total, errors.len())
        }
    }));

    let all_failed = succeeded == 0 && !errors.is_empty();
    BackupResult {
        success: !all_failed,
        message: if errors.is_empty() {
            format!("{} dossier(s) sauvegardé(s) dans «{}».", succeeded, target)
        } else {
            format!("{}/{} succès. Détails : {}", succeeded, total, errors.join(" | "))
        },
        duration_secs: start.elapsed().as_secs(),
        folders_count: succeeded,
    }
}
