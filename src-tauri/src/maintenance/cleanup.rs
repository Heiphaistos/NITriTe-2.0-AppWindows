use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::error::NiTriTeError;

#[derive(Debug, Clone, Serialize)]
pub struct CleanupResult {
    pub action: String,
    pub success: bool,
    pub freed_mb: f64,
    pub message: String,
}

pub fn empty_recycle_bin() -> Result<CleanupResult, NiTriTeError> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", "Clear-RecycleBin -Force -ErrorAction SilentlyContinue"])
        .creation_flags(0x08000000).output()?;

    Ok(CleanupResult {
        action: "Vider la corbeille".into(),
        success: output.status.success(),
        freed_mb: 0.0,
        message: if output.status.success() { "Corbeille videe".into() } else { "Erreur".into() },
    })
}

pub fn clean_temp_files() -> Result<CleanupResult, NiTriTeError> {
    let temp_dir = std::env::temp_dir();

    // Validation : %TEMP% doit pointer vers un répertoire utilisateur sûr
    // pour éviter qu'un %TEMP% malicieux ne supprime des fichiers système
    let temp_str = temp_dir.to_string_lossy().to_lowercase();
    let is_safe = temp_str.contains(r"\appdata\local\temp")
        || temp_str.contains(r"\temp")
        || temp_str.contains(r"\tmp");
    if !is_safe {
        return Err(NiTriTeError::CommandDenied(
            format!(
                "Répertoire temporaire suspect (non autorisé pour suppression): {}",
                temp_dir.display()
            ),
        ));
    }

    let mut freed: u64 = 0;

    if let Ok(entries) = std::fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    freed += meta.len();
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }

    Ok(CleanupResult {
        action: "Supprimer fichiers temporaires".into(),
        success: true,
        freed_mb: freed as f64 / 1_048_576.0,
        message: format!("{:.1} MB liberes", freed as f64 / 1_048_576.0),
    })
}

pub fn run_disk_cleanup() -> Result<CleanupResult, NiTriTeError> {
    let status = Command::new("cleanmgr").arg("/d").arg("C:").creation_flags(0x08000000).status()?;

    Ok(CleanupResult {
        action: "Nettoyage disque Windows".into(),
        success: status.success(),
        freed_mb: 0.0,
        message: "Nettoyage disque lance".into(),
    })
}

pub fn get_startup_programs() -> Result<Vec<StartupEntry>, NiTriTeError> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command",
            "Get-CimInstance Win32_StartupCommand | Select-Object Name, Command, Location, User | ConvertTo-Json"])
        .creation_flags(0x08000000).output()?;

    let text = String::from_utf8_lossy(&output.stdout);
    let trimmed = text.trim();
    // PowerShell retourne un objet JSON unique si un seul programme est trouvé
    let entries: Vec<serde_json::Value> = serde_json::from_str(trimmed)
        .or_else(|_| {
            serde_json::from_str::<serde_json::Value>(trimmed)
                .map(|obj| if obj.is_object() { vec![obj] } else { vec![] })
        })
        .unwrap_or_default();

    Ok(entries.iter().map(|e| StartupEntry {
        name: e["Name"].as_str().unwrap_or("").to_string(),
        command: e["Command"].as_str().unwrap_or("").to_string(),
        location: e["Location"].as_str().unwrap_or("").to_string(),
        user: e["User"].as_str().unwrap_or("").to_string(),
        enabled: true,
    }).collect())
}

#[derive(Debug, Clone, Serialize)]
pub struct StartupEntry {
    pub name: String,
    pub command: String,
    pub location: String,
    pub user: String,
    pub enabled: bool,
}

/// Desactive un programme au demarrage via le registre
pub fn disable_startup_program(name: &str, location: &str) -> Result<CleanupResult, NiTriTeError> {
    // Determiner la ruche (HKCU ou HKLM) et le chemin
    let reg_path = if location.starts_with("HKLM") || location.starts_with("HKU") {
        return Err(NiTriTeError::ElevationRequired(
            "La desactivation de programmes systeme necessite les droits administrateur".into(),
        ));
    } else {
        location
    };

    // Valider reg_path : uniquement HKCU Run/RunOnce autorisé
    let allowed_reg_prefixes = [
        "HKCU\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
        "HKCU:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
        "HKEY_CURRENT_USER\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
    ];
    if !allowed_reg_prefixes.iter().any(|prefix| reg_path.starts_with(prefix)) {
        return Err(NiTriTeError::CommandDenied(
            format!("Chemin registre non autorisé pour désactivation démarrage: {}", reg_path),
        ));
    }

    // Valider name : pas de métacaractères PS/shell
    if name.contains('\'') || name.contains('"') || name.contains('`') || name.contains('$') || name.is_empty() {
        return Err(NiTriTeError::CommandDenied(
            "Nom de programme invalide pour désactivation démarrage".into(),
        ));
    }

    // Passer reg_path et name comme arguments séparés (pas par concaténation)
    let ps_script = format!(
        r#"$regPath = '{}'
$entryName = '{}'
Remove-ItemProperty -Path $regPath -Name $entryName -ErrorAction Stop"#,
        reg_path.replace('\'', "''"),
        name.replace('\'', "''")
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .creation_flags(0x08000000).output()?;

    if output.status.success() {
        Ok(CleanupResult {
            action: format!("Desactiver {}", name),
            success: true,
            freed_mb: 0.0,
            message: format!("{} retire du demarrage", name),
        })
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(NiTriTeError::System(format!("Impossible de desactiver {}: {}", name, err.trim())))
    }
}
