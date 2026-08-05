use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::Emitter;

use crate::error::NiTriTeError;

#[derive(Debug, Clone, Serialize)]
pub struct WingetPackage {
    pub name: String,
    pub id: String,
    pub version: String,
    pub available: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallResult {
    pub success: bool,
    pub app_id: String,
    pub message: String,
}

/// Découpe une ligne de table winget en colonnes. winget sépare les colonnes
/// par 2 espaces ou plus ; un nom de paquet peut contenir un espace simple,
/// d'où ce parsing par double-espace plutôt qu'un `split_whitespace` qui
/// éclaterait les noms composés. L'indexation ASCII est sûre : la coupe ne se
/// fait que sur des espaces (frontières de caractères valides).
fn split_winget_columns(line: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let bytes = line.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b' ' && bytes[i + 1] == b' ' {
            let p = line[start..i].trim();
            if !p.is_empty() { parts.push(p); }
            while i < bytes.len() && bytes[i] == b' ' { i += 1; }
            start = i;
        } else { i += 1; }
    }
    let last = line[start..].trim();
    if !last.is_empty() { parts.push(last); }
    parts
}

pub fn check_winget() -> bool {
    Command::new("winget").arg("--version")
        .stdout(Stdio::null()).stderr(Stdio::null())
        .creation_flags(0x08000000)
        .status().is_ok()
}

pub fn list_upgradable() -> Result<Vec<WingetPackage>, NiTriTeError> {
    let output = Command::new("winget")
        .args(["upgrade", "--include-unknown", "--accept-source-agreements"])
        .creation_flags(0x08000000).output()?;

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let mut packages = Vec::new();
    let mut past_separator = false;

    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        if !past_separator {
            if t.chars().all(|c| c == '-' || c == ' ') && t.len() > 10 { past_separator = true; }
            continue;
        }
        let lower = t.to_lowercase();
        // Fin de la table = phrase de pied de page winget. On matche la PHRASE
        // (EN « N package(s) have version numbers that cannot be determined »,
        // « X upgrades available » / FR « … ne peuvent pas être déterminés »,
        // « … mises à niveau disponibles ») et NON le mot nu « package », qui
        // apparaît légitimement dans des noms (« NuGet Package Explorer ») et
        // aurait tronqué la liste prématurément.
        if lower.contains("cannot be determined")
            || lower.contains("détermin") || lower.contains("determin")
            || lower.contains("upgrades available")
            || (lower.contains("niveau") && lower.contains("disponible"))
        {
            break;
        }
        let parts = split_winget_columns(t);
        if parts.len() >= 3 {
            packages.push(WingetPackage {
                name: parts[0].to_string(),
                id: parts.get(1).unwrap_or(&"").to_string(),
                version: parts.get(2).unwrap_or(&"").to_string(),
                available: parts.get(3).unwrap_or(&"").to_string(),
                source: "winget".to_string(),
            });
        }
    }

    Ok(packages)
}

pub fn install_package(
    package_id: &str,
    window: &tauri::Window,
) -> Result<InstallResult, NiTriTeError> {
    let mut child = Command::new("winget")
        .args([
            "install", "--id", package_id, "--exact", "--silent",
            "--accept-source-agreements", "--accept-package-agreements",
            "--disable-interactivity",
        ])
        .stdout(Stdio::piped())
        // stderr non lu ici : le piper sans le drainer bloquerait winget (deadlock)
        // dès que son buffer stderr se remplit. On le jette ; le verdict vient du
        // code de sortie et la sortie utile de winget passe par stdout.
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            // Classification EN + FR, insensible à la casse : winget est localisé,
            // donc « erreur »/« Réussi » sur Windows FR — sans ça toutes les lignes
            // tombaient en « info » (aucune coloration rouge/verte).
            let ll = line.to_lowercase();
            let level = if ll.contains("error") || ll.contains("erreur") || ll.contains("échou") || ll.contains("failed") || ll.contains("échec") {
                "error"
            } else if ll.contains("successfully") || ll.contains("réussi") || ll.contains("succès") {
                "success"
            } else {
                "info"
            };
            let _ = window.emit("install-log", serde_json::json!({
                "app_id": package_id,
                "line": line,
                "level": level,
            }));
        }
    }

    let status = child.wait()?;

    Ok(InstallResult {
        success: status.success(),
        app_id: package_id.to_string(),
        message: if status.success() { "Installation reussie".into() } else { format!("Code: {}", status.code().unwrap_or(-1)) },
    })
}

/// Lance une commande winget et streame stdout ligne par ligne via `upgrade-log`.
fn stream_winget_upgrade(args: &[&str], window: &tauri::Window) -> Result<(), NiTriTeError> {
    let mut child = Command::new("winget")
        .args(args)
        .stdout(Stdio::piped())
        // stderr non lu ici : le piper sans le drainer bloquerait winget (deadlock)
        // dès que son buffer stderr se remplit. On le jette ; le verdict vient du
        // code de sortie et la sortie utile de winget passe par stdout.
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let _ = window.emit("upgrade-log", &line);
        }
    }

    // child.wait()? seul ne propage QUE les erreurs d'E/S (process introuvable
    // etc.) — un winget qui echoue reellement (paquet verrouille, telechargement
    // interrompu, confirmation interactive malgre --silent) renvoie un
    // ExitStatus valide avec success()=false, jamais un Err. upgrade_all()
    // renvoyait donc toujours Ok(()) meme quand AUCUNE mise a jour n'avait
    // reellement abouti, et les deux appelants (UpdatesPage.vue, DiagTabUpdates.vue)
    // font confiance a ce succes pour afficher un toast "termine" et vider la
    // liste des mises a jour en attente.
    let status = child.wait()?;
    if !status.success() {
        return Err(NiTriTeError::System(format!(
            "winget a échoué (code {})",
            status.code().unwrap_or(-1)
        )));
    }
    Ok(())
}

pub fn upgrade_all(excluded_ids: Vec<String>, window: &tauri::Window) -> Result<(), NiTriTeError> {
    // winget upgrade ne supporte PAS d'option d'exclusion : `--all` mettrait à
    // jour TOUS les paquets, y compris ceux que l'utilisateur a exclus. Quand une
    // exclusion existe, on met donc à jour chaque paquet non-exclu individuellement.
    if excluded_ids.is_empty() {
        return stream_winget_upgrade(
            &["upgrade", "--all", "--silent", "--accept-source-agreements", "--accept-package-agreements"],
            window,
        );
    }

    let pkgs = list_upgradable()?;
    for p in pkgs {
        if p.id.is_empty() || excluded_ids.iter().any(|e| e.eq_ignore_ascii_case(&p.id)) {
            let _ = window.emit("upgrade-log", format!("Exclu : {} ({})", p.name, p.id));
            continue;
        }
        let _ = window.emit("upgrade-log", format!("Mise à jour de {} ({})…", p.name, p.id));
        stream_winget_upgrade(
            &["upgrade", "--id", p.id.as_str(), "--exact", "--silent",
              "--accept-source-agreements", "--accept-package-agreements", "--disable-interactivity"],
            window,
        )?;
    }
    Ok(())
}

pub fn search_packages(query: &str) -> Result<Vec<WingetPackage>, NiTriTeError> {
    let output = Command::new("winget")
        .args(["search", query, "--accept-source-agreements"])
        .creation_flags(0x08000000).output()?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut packages = Vec::new();

    let lines: Vec<&str> = text.lines().collect();
    let header_idx = lines.iter().position(|l| l.contains("----"));

    if let Some(idx) = header_idx {
        for line in &lines[idx + 1..] {
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }
            // Colonnes winget search : Name | Id | Version | [Match] | Source.
            // On indexe par le DÉBUT : Name/Id/Version sont toujours les 3
            // premières, quelles que soient les colonnes de fin (Match/Source).
            let parts = split_winget_columns(trimmed);
            if parts.len() >= 3 {
                packages.push(WingetPackage {
                    name: parts[0].to_string(),
                    id: parts[1].to_string(),
                    version: parts[2].to_string(),
                    available: String::new(),
                    source: parts.get(parts.len() - 1).filter(|_| parts.len() > 3).map_or_else(|| "winget".to_string(), |s| s.to_string()),
                });
            }
        }
    }

    Ok(packages)
}
