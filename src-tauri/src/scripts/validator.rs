use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    pub risk_level: String, // "safe" | "warning" | "danger"
    pub warnings: Vec<String>,
    pub info: Vec<String>,
    pub line_count: usize,
}

/// Analyse un script PowerShell ou Batch et retourne un rapport de risque
#[tauri::command]
pub fn validate_script(code: String, language: String) -> ValidationResult {
    let mut warnings: Vec<String> = Vec::new();
    let mut info: Vec<String> = Vec::new();
    let lines: Vec<&str> = code.lines().collect();
    let lower = code.to_lowercase();

    // Patterns dangereux communs
    let danger_patterns: &[(&str, &str)] = &[
        ("format-volume", "Formatage de volume détecté"),
        ("remove-item -recurse", "Suppression récursive détectée"),
        ("rd /s /q c:", "Suppression récursive de C: détectée"),
        ("del /f /q", "Suppression forcée détectée"),
        ("net user /add", "Création de compte utilisateur détectée"),
        ("net localgroup administrators", "Modification groupe Admins détectée"),
        ("bcdedit", "Modification boot loader détectée"),
        ("reg add hklm\\sam", "Modification SAM registry détectée"),
        ("invoke-expression", "Exécution dynamique (IEX) détectée"),
        ("iex (", "Exécution dynamique (IEX) détectée"),
        ("downloadstring", "Téléchargement + exécution détecté"),
        ("downloadfile", "Téléchargement de fichier détecté"),
        ("-encodedcommand", "Commande encodée Base64 détectée"),
        // Shorts pour -EncodedCommand : bypassent facilement le filtre
        (" -enc ", "Commande encodée Base64 (court) détectée"),
        (" -e ", "Paramètre encodé suspect détecté"),
        ("disable-windowsoptionalfeature", "Désactivation de fonctionnalité Windows"),
        ("set-executionpolicy unrestricted", "Politique exécution permissive"),
        // Téléchargement depuis internet — promu de warning à danger
        ("invoke-webrequest", "Téléchargement depuis internet (Invoke-WebRequest)"),
        ("iwr ", "Téléchargement depuis internet (iwr)"),
        // Lancement de processus externe — danger si script utilisateur
        ("start-process", "Lancement de processus externe (Start-Process)"),
    ];

    let warning_patterns: &[(&str, &str)] = &[
        ("stop-service", "Arrêt de service"),
        ("disable-netadapter", "Désactivation d'adaptateur réseau"),
        ("clear-eventlog", "Effacement des journaux d'événements"),
        ("curl ", "Téléchargement depuis internet"),
        ("wget ", "Téléchargement depuis internet"),
        ("regedit", "Modification du registre"),
        ("taskkill", "Terminaison de processus"),
        ("sc config", "Modification de configuration service"),
        ("netsh", "Modification configuration réseau"),
        ("wmic", "Requête WMI système"),
        ("powershell -w hidden", "Exécution cachée PowerShell"),
        ("-windowstyle hidden", "Fenêtre cachée"),
    ];

    let mut is_danger = false;
    for (pattern, msg) in danger_patterns {
        if lower.contains(pattern) {
            warnings.push(format!("DANGER: {}", msg));
            is_danger = true;
        }
    }
    for (pattern, msg) in warning_patterns {
        if lower.contains(pattern) {
            warnings.push(format!("Attention: {}", msg));
        }
    }

    // Info générales
    if language.to_lowercase().contains("powershell") || language.to_lowercase().contains("ps1") {
        let has_error_handling = lower.contains("try {") || lower.contains("trap {") || lower.contains("-erroraction");
        if has_error_handling { info.push("Gestion d'erreur présente".to_string()); }

        let uses_sudo = lower.contains("[security.principal.") || lower.contains("runas");
        if uses_sudo { info.push("Demande d'élévation détectée".to_string()); }
    }

    let risk_level = if is_danger {
        "danger"
    } else if !warnings.is_empty() {
        "warning"
    } else {
        "safe"
    };

    ValidationResult {
        risk_level: risk_level.to_string(),
        warnings,
        info,
        line_count: lines.len(),
    }
}
