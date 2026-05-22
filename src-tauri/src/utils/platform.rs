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
    std::process::Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x08000000)
        .status()
        .is_ok()
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
}
