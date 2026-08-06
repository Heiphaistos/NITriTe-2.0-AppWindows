use serde::Serialize;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::error::NiTriTeError;

#[derive(Debug, Clone, Serialize)]
pub struct CommandResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Décode la sortie d'une commande. UTF-8 d'abord (PowerShell, ASCII : chemin
/// rapide, aucun changement), puis repli sur le codepage OEM — les outils console
/// (driverquery, systeminfo…) écrivent en OEM sur Windows FR, et `from_utf8_lossy`
/// transformait leurs accents en mojibake ("Contrôleur" → "Contr�leur").
#[cfg(target_os = "windows")]
pub(crate) fn decode_output(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    if bytes.is_empty() {
        return String::new();
    }
    use windows::Win32::Globalization::{GetOEMCP, MultiByteToWideChar, MULTI_BYTE_TO_WIDE_CHAR_FLAGS};
    unsafe {
        let cp = GetOEMCP();
        let len = MultiByteToWideChar(cp, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), bytes, None);
        if len <= 0 {
            return String::from_utf8_lossy(bytes).to_string();
        }
        let mut buf = vec![0u16; len as usize];
        let written = MultiByteToWideChar(cp, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), bytes, Some(&mut buf));
        if written <= 0 {
            return String::from_utf8_lossy(bytes).to_string();
        }
        String::from_utf16_lossy(&buf[..written as usize])
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn decode_output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Execute une commande systeme avec timeout effectif.
/// `timeout_secs` : 0 = pas de timeout (attente illimitée).
pub fn execute_system_command(cmd: &str, args: &[&str], timeout_secs: u64) -> Result<CommandResult, NiTriTeError> {
    let child = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| NiTriTeError::System(format!("Erreur lancement {}: {}", cmd, e)))?;

    let pid = child.id();
    let (tx, rx) = mpsc::channel::<std::io::Result<std::process::Output>>();

    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });

    let output = if timeout_secs == 0 {
        rx.recv().map_err(|_| NiTriTeError::System("Thread error".into()))??
    } else {
        match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
            Ok(result) => result?,
            Err(_) => {
                #[cfg(target_os = "windows")]
                let _ = Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .creation_flags(0x08000000)
                    .spawn();
                tracing::warn!("execute_system_command: timeout {}s dépassé (cmd={}, pid={})", timeout_secs, cmd, pid);
                return Err(NiTriTeError::Timeout(
                    format!("Commande '{}' interrompue après {}s", cmd, timeout_secs)
                ));
            }
        }
    };

    Ok(CommandResult {
        success: output.status.success(),
        stdout: decode_output(&output.stdout),
        stderr: decode_output(&output.stderr),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

/// Lance SFC /scannow
pub fn run_sfc() -> Result<CommandResult, NiTriTeError> {
    execute_system_command("sfc", &["/scannow"], 300)
}

/// Lance DISM RestoreHealth — chemin complet, minuscules, avec timeout
pub fn run_dism_restore() -> Result<CommandResult, NiTriTeError> {
    execute_system_command(
        "C:\\Windows\\System32\\Dism.exe",
        &["/Online", "/Cleanup-Image", "/RestoreHealth"],
        600,
    )
}

/// Liste les drivers installes
pub fn list_drivers() -> Result<CommandResult, NiTriTeError> {
    execute_system_command("driverquery", &["/v", "/fo", "csv"], 30)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prouve le mecanisme exploite par le bug DiagTabDriverUpdater.vue::chooseSdiLocation :
    /// execute_system_command ATTEND la fin du process (wait_with_output), et au-dela du
    /// timeout le tue de force (taskkill /F) plutot que de simplement abandonner l'attente.
    /// Lancer un executable GUI interactif long (comme SDI.exe) directement via cette
    /// fonction — au lieu de Start-Process fire-and-forget — le ferait donc tuer de force
    /// apres le timeout, meme en pleine utilisation par l'utilisateur.
    #[test]
    #[ignore] // lance un vrai process powershell, à exécuter manuellement (--ignored)
    fn execute_system_command_kills_process_on_timeout() {
        let start = std::time::Instant::now();
        let result = execute_system_command(
            "powershell",
            &["-NoProfile", "-Command", "Start-Sleep -Seconds 30"],
            2, // timeout volontairement plus court que le sleep de 30s
        );
        let elapsed = start.elapsed();

        assert!(result.is_err(), "un process encore vivant après le timeout doit renvoyer une erreur, pas un faux succès");
        assert!(matches!(result, Err(NiTriTeError::Timeout(_))));
        // La fonction ne doit pas attendre les 30s complètes du sleep : le timeout
        // + taskkill doivent interrompre l'attente bien avant.
        assert!(elapsed.as_secs() < 10, "execute_system_command a attendu {}s, le taskkill sur timeout ne fonctionne pas", elapsed.as_secs());
    }
}
