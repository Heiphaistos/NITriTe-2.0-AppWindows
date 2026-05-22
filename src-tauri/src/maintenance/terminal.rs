use serde::Serialize;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::error::NiTriTeError;

#[derive(Debug, Clone, Serialize)]
pub struct ShellInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShellResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub shell_used: String,
}

/// Detecte les shells disponibles sur le systeme
pub fn detect_shells() -> Vec<ShellInfo> {
    let candidates = vec![
        ("cmd", "CMD (Invite de commandes)", find_cmd()),
        ("powershell", "Windows PowerShell", find_in_path("powershell.exe")),
        ("pwsh", "PowerShell 7+", find_in_path("pwsh.exe")),
        ("gitbash", "Git Bash", find_git_bash()),
        ("wsl", "WSL (Linux)", find_in_path("wsl.exe")),
        ("nushell", "Nushell", find_in_path("nu.exe")),
    ];

    candidates
        .into_iter()
        .map(|(id, name, path)| {
            let available = path.is_some();
            ShellInfo {
                id: id.to_string(),
                name: name.to_string(),
                path: path.unwrap_or_default(),
                available,
            }
        })
        .collect()
}

/// Execute une commande dans le shell specifie avec timeout effectif
pub fn run_in_shell(shell_id: &str, command: &str, timeout_secs: u64) -> Result<ShellResult, NiTriTeError> {
    let (program, args) = match shell_id {
        "cmd" => ("cmd".to_string(), vec!["/C".to_string(), command.to_string()]),
        "powershell" => ("powershell".to_string(), vec![
            "-NoProfile".to_string(),
            "-ExecutionPolicy".to_string(), "RemoteSigned".to_string(),
            "-Command".to_string(), command.to_string(),
        ]),
        "pwsh" => ("pwsh".to_string(), vec![
            "-NoProfile".to_string(),
            "-ExecutionPolicy".to_string(), "RemoteSigned".to_string(),
            "-Command".to_string(), command.to_string(),
        ]),
        "gitbash" => {
            let bash = find_git_bash().unwrap_or_else(|| "bash".to_string());
            (bash, vec!["-c".to_string(), command.to_string()])
        }
        "wsl" => ("wsl".to_string(), vec!["-e".to_string(), "bash".to_string(), "-c".to_string(), command.to_string()]),
        "nushell" => ("nu".to_string(), vec!["-c".to_string(), command.to_string()]),
        _ => return Err(NiTriTeError::System(format!("Shell inconnu: {}", shell_id))),
    };

    let child = Command::new(&program)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| NiTriTeError::System(format!("Erreur lancement {}: {}", program, e)))?;

    let pid = child.id();
    let effective_timeout = Duration::from_secs(if timeout_secs == 0 { 120 } else { timeout_secs });
    let (tx, rx) = mpsc::channel::<std::io::Result<std::process::Output>>();

    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });

    let output = match rx.recv_timeout(effective_timeout) {
        Ok(result) => result.map_err(NiTriTeError::Io)?,
        Err(_) => {
            // Timeout — tuer le processus par PID
            #[cfg(target_os = "windows")]
            let _ = Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .creation_flags(0x08000000)
                .spawn();
            tracing::warn!("run_in_shell: timeout {}s dépassé (shell={}, pid={})", effective_timeout.as_secs(), shell_id, pid);
            return Err(NiTriTeError::Timeout(
                format!("Timeout {}s dépassé — processus terminé de force", effective_timeout.as_secs())
            ));
        }
    };

    // Tronquer la sortie si trop volumineuse
    let max_len = 100_000;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let stdout = if stdout.len() > max_len {
        format!("{}...\n[Sortie tronquée à {} caractères]", &stdout[..max_len], max_len)
    } else {
        stdout.to_string()
    };

    let stderr = if stderr.len() > max_len {
        format!("{}...\n[Erreur tronquée]", &stderr[..max_len])
    } else {
        stderr.to_string()
    };

    Ok(ShellResult {
        success: output.status.success(),
        stdout,
        stderr,
        exit_code: output.status.code().unwrap_or(-1),
        shell_used: shell_id.to_string(),
    })
}

fn find_cmd() -> Option<String> {
    let sys = std::env::var("SYSTEMROOT").unwrap_or_else(|_| "C:\\Windows".to_string());
    let path = PathBuf::from(&sys).join("System32").join("cmd.exe");
    if path.exists() { Some(path.to_string_lossy().to_string()) } else { None }
}

fn find_in_path(exe: &str) -> Option<String> {
    which_exe(exe)
}

fn find_git_bash() -> Option<String> {
    // Chercher dans les emplacements courants
    let candidates = vec![
        "C:\\Program Files\\Git\\bin\\bash.exe",
        "C:\\Program Files (x86)\\Git\\bin\\bash.exe",
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return Some(c.to_string());
        }
    }
    which_exe("bash.exe")
}

fn which_exe(name: &str) -> Option<String> {
    let output = Command::new("where").arg(name).creation_flags(0x08000000).output().ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout);
        path.lines().next().map(|s| s.trim().to_string())
    } else {
        None
    }
}
