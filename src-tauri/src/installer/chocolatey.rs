use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::Emitter;

use crate::error::NiTriTeError;
use crate::installer::winget::InstallResult;
use crate::maintenance::commands::decode_output;

#[derive(Debug, Clone, Serialize)]
pub struct ChocoPackage {
    pub name: String,
    pub current_version: String,
    pub available_version: String,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChocoUpgradeResult {
    pub success: bool,
    pub upgraded_count: u32,
    pub message: String,
}

/// Chemin fixe et connu de choco.exe (toujours le meme, poste par
/// l'installeur officiel). Necessaire car un bootstrap effectue PENDANT la
/// session Nitrite en cours ne met a jour le PATH que dans le registre — le
/// PATH du process Nitrite deja demarre reste celui capture au lancement,
/// donc `Command::new(choco_exe())` resterait "introuvable" tant que Nitrite
/// n'est pas relance, meme juste apres un bootstrap reussi.
pub fn choco_exe() -> String {
    const KNOWN: &str = r"C:\ProgramData\chocolatey\bin\choco.exe";
    if std::path::Path::new(KNOWN).exists() { KNOWN.to_string() } else { "choco".to_string() }
}

pub fn check_chocolatey() -> bool {
    Command::new(choco_exe())
        .arg("--version")
        .creation_flags(0x08000000)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn list_chocolatey_upgrades() -> Result<Vec<ChocoPackage>, NiTriTeError> {
    let output = Command::new(choco_exe())
        .args(["outdated", "-r", "--no-color"])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| NiTriTeError::System(format!("Chocolatey introuvable: {}", e)))?;

    // decode_output : par cohérence avec bootstrap_chocolatey() (cassé confirmé
    // en direct sur cette machine) — substitution sûre même si non déclenché
    // par les paquets choco actuels (majoritairement anglophones).
    let text = decode_output(&output.stdout);
    let mut packages = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Format choco -r : "packagename|currentversion|availableversion|ispinned"
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() >= 3 {
            let pinned = parts.get(3).map(|v| *v == "true").unwrap_or(false);
            packages.push(ChocoPackage {
                name: parts[0].to_string(),
                current_version: parts[1].to_string(),
                available_version: parts[2].to_string(),
                pinned,
            });
        }
    }

    Ok(packages)
}

pub fn upgrade_chocolatey_all(excluded: Vec<String>) -> Result<ChocoUpgradeResult, NiTriTeError> {
    let mut args: Vec<String> = vec!["upgrade".into(), "all".into(), "-y".into(), "--no-color".into()];
    // Exclusions : choco supporte `--except="pkg1,pkg2"`. Sans ça, les paquets
    // exclus par l'utilisateur étaient quand même mis à jour. Les noms non-choco
    // (ids winget de la liste d'exclusion partagée) sont simplement ignorés.
    let cleaned: Vec<String> = excluded.into_iter().map(|s| s.replace(['"', ','], "")).filter(|s| !s.trim().is_empty()).collect();
    if !cleaned.is_empty() {
        args.push(format!("--except=\"{}\"", cleaned.join(",")));
    }
    let output = Command::new(choco_exe())
        .args(&args)
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| NiTriTeError::System(format!("Erreur upgrade choco: {}", e)))?;

    let stdout = decode_output(&output.stdout);
    let success = output.status.success();

    // Nombre de paquets mis à jour : choco termine par une ligne récapitulative
    // "Chocolatey upgraded 2/3 packages." — on en extrait N (fiable), plutôt que
    // de compter les lignes "upgraded"/"successful" qui sur-comptaient (la ligne
    // récap + une ligne par paquet, et "0/3" comptait comme 1).
    let upgraded = stdout
        .lines()
        .find_map(|l| {
            let ll = l.to_lowercase();
            if !ll.contains('/') {
                return None;
            }
            ll.split_once("upgraded ")
                .and_then(|(_, rest)| rest.split('/').next())
                .and_then(|n| n.trim().parse::<u32>().ok())
        })
        .unwrap_or_else(|| {
            // Repli si le format récap change : compter les lignes "successful".
            stdout.lines().filter(|l| l.to_lowercase().contains("successful")).count() as u32
        });

    let message = if success {
        format!("{} paquet(s) mis à jour", upgraded)
    } else {
        let stderr = decode_output(&output.stderr);
        format!("Erreur: {}", stderr.lines().next().unwrap_or("inconnue"))
    };

    Ok(ChocoUpgradeResult {
        success,
        upgraded_count: upgraded,
        message,
    })
}

/// Installe Chocolatey lui-meme via le script officiel (idempotent : ne fait
/// rien si deja present). Necessite les droits admin (deja garantis par le
/// contexte d'installation d'apps).
pub fn bootstrap_chocolatey() -> Result<(), NiTriTeError> {
    if check_chocolatey() {
        return Ok(());
    }
    let ps = "Set-ExecutionPolicy Bypass -Scope Process -Force; \
        [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; \
        iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))";
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| NiTriTeError::System(format!("Erreur bootstrap Chocolatey: {}", e)))?;
    if !output.status.success() || !check_chocolatey() {
        // decode_output : message d'erreur PowerShell réel (ex. « Impossible
        // de se connecter au serveur distant ») corrompu par from_utf8_lossy —
        // confirmé en direct sur cette machine.
        let stderr = decode_output(&output.stderr);
        return Err(NiTriTeError::System(format!("Installation de Chocolatey echouee: {}", stderr.lines().next().unwrap_or("inconnue"))));
    }
    Ok(())
}

/// Recherche un id Chocolatey par nom quand programs.json n'en fournit pas.
/// `choco search` interroge le depot communautaire (des milliers de paquets,
/// bien plus large que winget pour les outils niche/portables).
fn normalize_pkg_name(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

/// Cherche un id exact d'abord (rapide, precis quand le nom correspond deja a
/// l'id chocolatey), puis une recherche floue avec correspondance normalisee
/// (espaces/casse ignores) : les ids chocolatey n'ont jamais d'espace
/// ("googlechrome" pour "Google Chrome"), donc `--exact` seul rate presque
/// tout. Ne retourne un match flou QUE si le nom normalise correspond
/// exactement a un id — jamais le "premier resultat" au hasard, qui
/// installerait le mauvais logiciel (`choco search chrome` retourne aussi
/// "1password-chrome", "adblockpluschrome"...).
pub fn search_choco_id(name: &str) -> Option<String> {
    let run_search = |query: &str, exact: bool| -> Option<Vec<(String, String)>> {
        let mut args = vec!["search", query, "--limit-output", "--no-progress"];
        if exact { args.push("--exact"); }
        let output = Command::new(choco_exe()).args(&args).creation_flags(0x08000000).output().ok()?;
        if !output.status.success() { return None; }
        let text = decode_output(&output.stdout);
        Some(text.lines().filter_map(|l| {
            let mut parts = l.splitn(2, '|');
            let id = parts.next()?.trim().to_string();
            let ver = parts.next().unwrap_or("").trim().to_string();
            if id.is_empty() { None } else { Some((id, ver)) }
        }).collect())
    };

    let normalized_query = normalize_pkg_name(name);

    if let Some(results) = run_search(name, true) {
        if let Some((id, _)) = results.into_iter().next() {
            return Some(id);
        }
    }
    // Convention Chocolatey : l'id du paquet est le nom sans espaces/casse
    // ("googlechrome" pour "Google Chrome") — chercher directement avec le nom
    // normalise est bien plus fiable que le plein-texte flou ci-dessous.
    if !normalized_query.is_empty() {
        if let Some(results) = run_search(&normalized_query, true) {
            if let Some((id, _)) = results.into_iter().next() {
                return Some(id);
            }
        }
    }

    let results = run_search(name, false)?;
    results.into_iter().find(|(id, _)| normalize_pkg_name(id) == normalized_query).map(|(id, _)| id)
}

/// Installe un paquet via Chocolatey, en streamant la sortie comme winget.
pub fn install_via_chocolatey(package_id: &str, window: &tauri::Window) -> Result<InstallResult, NiTriTeError> {
    let mut child = Command::new(choco_exe())
        .args(["install", package_id, "-y", "--no-progress", "--no-color"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let ll = line.to_lowercase();
            let level = if ll.contains("error") || ll.contains("erreur") || ll.contains("failed") {
                "error"
            } else if ll.contains("successfully installed") || ll.contains("already installed") {
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
        message: if status.success() { "Installation reussie (Chocolatey)".into() } else { format!("Code: {}", status.code().unwrap_or(-1)) },
    })
}
