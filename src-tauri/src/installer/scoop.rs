use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::Emitter;

use crate::error::NiTriTeError;
use crate::installer::winget::InstallResult;
use crate::maintenance::commands::decode_output;

/// Chemin connu de scoop.exe/scoop.cmd (shim sous le profil de l'utilisateur
/// courant). Meme raison que pour Chocolatey : un bootstrap effectue pendant
/// la session Nitrite en cours ne rend pas "scoop" trouvable via le PATH deja
/// capture au lancement du process, tant que Nitrite n'est pas relance.
///
/// Les installations Scoop récentes ne posent PAS de "scoop.exe" dans shims/
/// — seulement "scoop" (sans extension) et "scoop.cmd" (confirmé en direct
/// sur cette machine). `Command::new("scoop")` (repli precedent) echoue
/// silencieusement (NotFound) car Rust ne fait pas de resolution PATHEXT
/// comme un vrai shell, contrairement a "scoop.cmd" avec son chemin complet
/// qui fonctionne (Rust gere nativement l'invocation .cmd/.bat sur Windows).
pub fn scoop_exe() -> String {
    if let Some(home) = dirs::home_dir() {
        let shims = home.join("scoop").join("shims");
        let known_exe = shims.join("scoop.exe");
        if known_exe.exists() {
            return known_exe.to_string_lossy().to_string();
        }
        let known_cmd = shims.join("scoop.cmd");
        if known_cmd.exists() {
            return known_cmd.to_string_lossy().to_string();
        }
    }
    "scoop".to_string()
}

pub fn check_scoop() -> bool {
    Command::new(scoop_exe())
        .arg("--version")
        .creation_flags(0x08000000)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Installe Scoop lui-meme via le script officiel (idempotent). Scoop refuse
/// par design de s'installer sous un compte administrateur ("Scoop is not
/// designed to be run as an administrator") sauf avec `-RunAsAdmin` — Nitrite
/// tournant deja elevé pour le reste de ses fonctions, ce flag est necessaire
/// ici (pas de contournement de securite : simplement l'option officielle
/// prevue pour ce cas).
pub fn bootstrap_scoop() -> Result<(), NiTriTeError> {
    if check_scoop() {
        return Ok(());
    }
    let ps = "Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force; \
        $s = irm get.scoop.sh; \
        & ([scriptblock]::Create($s)) -RunAsAdmin";
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| NiTriTeError::System(format!("Erreur bootstrap Scoop: {}", e)))?;
    if !output.status.success() || !check_scoop() {
        // decode_output : message d'erreur PowerShell réel (ex. « Impossible de
        // se connecter au serveur distant ») corrompu par from_utf8_lossy —
        // confirmé en direct sur cette machine.
        let stderr = decode_output(&output.stderr);
        return Err(NiTriTeError::System(format!("Installation de Scoop echouee: {}", stderr.lines().next().unwrap_or("inconnue"))));
    }
    Ok(())
}

fn normalize_pkg_name(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

/// Recherche un id Scoop par nom. `scoop search` interroge les buckets connus
/// (main/extras/versions...) sans qu'ils soient ajoutes localement (recherche
/// distante depuis 2023). Correspondance normalisee (espaces/casse ignores)
/// uniquement — jamais le premier resultat au hasard, qui installerait le
/// mauvais logiciel si le nom ne correspond qu'approximativement.
pub fn search_scoop_id(name: &str) -> Option<String> {
    let output = Command::new(scoop_exe())
        .args(["search", name])
        .creation_flags(0x08000000)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    // decode_output : par cohérence avec bootstrap_scoop() — les buckets Scoop
    // communautaires peuvent inclure des noms/paquets non-ASCII, substitution
    // sûre (repli UTF-8 en premier) même si non déclenché sur cette machine.
    let text = decode_output(&output.stdout);
    let normalized_query = normalize_pkg_name(name);
    // Format : lignes "'<bucket>' bucket:\n    <name> (<version>) ..."
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.ends_with("bucket:") { continue; }
        if let Some(pkg) = trimmed.split_whitespace().next() {
            if normalize_pkg_name(pkg) == normalized_query {
                return Some(pkg.to_string());
            }
        }
    }
    None
}

/// Installe un paquet via Scoop, en streamant la sortie comme winget/choco.
pub fn install_via_scoop(package_id: &str, window: &tauri::Window) -> Result<InstallResult, NiTriTeError> {
    let mut child = Command::new(scoop_exe())
        .args(["install", package_id])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let ll = line.to_lowercase();
            let level = if ll.contains("error") || ll.contains("couldn't find") {
                "error"
            } else if ll.contains("was installed successfully") || ll.contains("is already installed") {
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
        message: if status.success() { "Installation reussie (Scoop)".into() } else { format!("Code: {}", status.code().unwrap_or(-1)) },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Live, non-CI (ignorée par défaut) : confirme sur une vraie machine
    // Windows avec Scoop installé que scoop_exe() résout un chemin réellement
    // exécutable. Cette machine n'a qu'un "scoop.cmd" (pas de "scoop.exe"),
    // ce qui faisait échouer silencieusement Command::new("scoop") (repli
    // précédent, pas de résolution PATHEXT côté Rust) — check_scoop()
    // retournait toujours false même avec Scoop réellement installé.
    #[test]
    #[ignore]
    fn scoop_exe_resolves_to_working_executable() {
        let exe = scoop_exe();
        assert_ne!(exe, "scoop", "scoop_exe() ne devrait pas retourner le repli générique quand un shim réel existe : {}", exe);
        assert!(check_scoop(), "check_scoop() devrait réussir avec le chemin résolu : {}", exe);
    }
}
