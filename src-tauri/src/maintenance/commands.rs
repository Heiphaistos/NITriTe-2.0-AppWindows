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
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
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
