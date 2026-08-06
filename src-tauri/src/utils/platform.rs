use serde::Serialize;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize)]
pub struct PlatformInfo {
    pub os_version: String,
    pub arch: String,
    pub build_number: u32,
    pub edition: String,
    pub has_winget: bool,
    pub has_chocolatey: bool,
}

impl PlatformInfo {
    pub fn detect() -> Self {
        let os_version = os_version();
        let build_number = extract_build_number(&os_version);

        Self {
            os_version,
            arch: if cfg!(target_arch = "x86_64") {
                "x64".to_string()
            } else {
                "x86".to_string()
            },
            build_number,
            edition: windows_edition(),
            has_winget: check_command("winget", &["--version"]),
            has_chocolatey: check_command("choco", &["--version"]),
        }
    }
}

fn os_version() -> String {
    format!(
        "{} {}",
        sysinfo::System::name().unwrap_or_default(),
        sysinfo::System::os_version().unwrap_or_default()
    )
}

fn extract_build_number(version: &str) -> u32 {
    // Cherche le dernier nombre dans la version (ex: "10.0.26100" -> 26100)
    version
        .split('.')
        .next_back()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn windows_edition() -> String {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    hklm.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
        .ok()
        .and_then(|key| key.get_value::<String, _>("EditionID").ok())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn check_command(cmd: &str, args: &[&str]) -> bool {
    check_command_with_timeout(cmd, args, 10)
}

/// `.status()` seul n'a aucun timeout — si winget/choco se fige (ex: winget
/// réindexant ses sources au premier lancement, un scénario réel documenté),
/// ce thread bloquant (get_platform_info tourne via spawn_blocking, rappelé
/// toutes les 5 min par le cache frontend, cf. dataCache.ts) reste bloqué à
/// vie sans jamais se libérer. Même pattern timeout+taskkill que
/// execute_system_command : on tue le process de force plutôt que d'attendre
/// indéfiniment.
fn check_command_with_timeout(cmd: &str, args: &[&str], timeout_secs: u64) -> bool {
    let mut child = match std::process::Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x08000000)
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    let pid = child.id();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait());
    });
    match rx.recv_timeout(std::time::Duration::from_secs(timeout_secs)) {
        // Même sémantique que l'ancien `.status().is_ok()` : seul un échec de
        // lancement (binaire introuvable) compte comme false, pas un exit code non-nul.
        Ok(Ok(_status)) => true,
        _ => {
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .creation_flags(0x08000000)
                .spawn();
            tracing::warn!(
                "check_command: timeout {}s dépassé pour '{}' — process tué de force (outil potentiellement figé)",
                timeout_secs, cmd
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_build_number_standard() {
        assert_eq!(extract_build_number("Windows 10 10.0.19041"), 19041);
    }

    #[test]
    fn extract_build_number_win11() {
        assert_eq!(extract_build_number("Windows 11 10.0.26100"), 26100);
    }

    #[test]
    fn extract_build_number_empty() {
        assert_eq!(extract_build_number(""), 0);
    }

    #[test]
    fn extract_build_number_no_dot() {
        assert_eq!(extract_build_number("Windows"), 0);
    }

    #[test]
    fn extract_build_number_trailing_space() {
        assert_eq!(extract_build_number("10.0.22621 "), 22621);
    }

    #[test]
    fn check_command_with_timeout_kills_hung_process() {
        let start = std::time::Instant::now();
        let result = check_command_with_timeout(
            "powershell",
            &["-NoProfile", "-Command", "Start-Sleep -Seconds 30"],
            2,
        );
        let elapsed = start.elapsed();
        assert!(!result, "un process encore vivant après le timeout doit renvoyer false");
        assert!(
            elapsed.as_secs() < 10,
            "check_command_with_timeout a attendu {}s, le timeout+taskkill ne fonctionne pas",
            elapsed.as_secs()
        );
    }

    #[test]
    fn check_command_with_timeout_returns_true_for_quick_command() {
        assert!(check_command_with_timeout("cmd", &["/c", "exit", "0"], 10));
    }

    #[test]
    fn check_command_with_timeout_returns_true_even_on_nonzero_exit() {
        // Même sémantique que l'ancien .status().is_ok() : un exit code non-nul
        // compte comme "commande lancée avec succès", pas comme un échec.
        assert!(check_command_with_timeout("cmd", &["/c", "exit", "1"], 10));
    }

    #[test]
    fn check_command_with_timeout_returns_false_for_missing_binary() {
        assert!(!check_command_with_timeout("this_binary_does_not_exist_xyz123", &[], 5));
    }
}
