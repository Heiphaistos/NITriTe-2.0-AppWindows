use std::path::PathBuf;

/// Retourne le dossier racine de l'application (a cote de l'exe)
pub fn app_root_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Dossier de configuration portable
pub fn config_dir() -> PathBuf {
    let root = app_root_dir();
    let portable = root.join("config");
    if portable.exists() || std::fs::create_dir_all(&portable).is_ok() {
        portable
    } else {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nitrite")
    }
}

/// Dossier de logs
pub fn logs_dir() -> PathBuf {
    let dir = app_root_dir().join(".logs");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Dossier de backups (Documents/NiTriTe/backups/)
pub fn backups_dir() -> PathBuf {
    let dir = dirs::document_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Documents")))
        .unwrap_or_else(app_root_dir)
        .join("NiTriTe")
        .join("backups");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Dossier de telechargements
pub fn downloads_dir() -> PathBuf {
    let dir = app_root_dir().join("downloads");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Dossier des logiciels portables
pub fn portables_dir() -> PathBuf {
    let dir = app_root_dir().join("logiciel");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_root_dir_is_absolute() {
        let p = app_root_dir();
        assert!(p.is_absolute(), "app_root_dir() should be absolute");
    }

    #[test]
    fn logs_dir_ends_with_logs() {
        let p = logs_dir();
        assert!(p.ends_with(".logs"), "logs_dir should end with .logs");
    }

    #[test]
    fn downloads_dir_ends_with_downloads() {
        let p = downloads_dir();
        assert!(p.ends_with("downloads"));
    }

    #[test]
    fn portables_dir_ends_with_logiciel() {
        let p = portables_dir();
        assert!(p.ends_with("logiciel"));
    }
}
