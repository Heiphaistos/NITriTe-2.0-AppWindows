
/// PID/nom de processus dont la terminaison plante immédiatement Windows
/// (BSOD ou déconnexion de session forcée) — ce n'est pas juste "risqué",
/// c'est un comportement noyau volontaire (protection anti-sabotage intégrée
/// depuis Windows XP/Vista : csrss.exe déclenche un arrêt système s'il reçoit
/// une demande de terminaison). PID 0/4 sont fixes par le noyau (System Idle
/// Process / System). Liste volontairement étroite (contrairement au
/// `$protected` plus large de temps_wifi_turbo.rs, pensé pour un auto-kill de
/// process "inutiles") : ici l'utilisateur cible un PID précis avec
/// confirmation explicite, on ne bloque que ce qui plante l'OS à coup sûr.
fn is_critical_process(pid: u32, name: &str) -> bool {
    if pid == 0 || pid == 4 { return true; }
    let n = name.trim().to_lowercase();
    let n = n.strip_suffix(".exe").unwrap_or(&n);
    matches!(n, "csrss" | "wininit" | "winlogon" | "services" | "lsass" | "smss")
}

/// Termine un processus par son PID
#[tauri::command]
async fn kill_process(pid: u32) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let name = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command",
                    &format!("(Get-Process -Id {} -ErrorAction SilentlyContinue).ProcessName", pid)])
                .creation_flags(0x08000000)
                .output()
                .ok()
                .map(|o| crate::maintenance::commands::decode_output(&o.stdout).trim().to_string())
                .unwrap_or_default();
            if is_critical_process(pid, &name) {
                return Err(format!(
                    "Processus système critique ({}) — terminaison refusée (plante immédiatement Windows).",
                    if name.is_empty() { format!("PID {}", pid) } else { name }
                ));
            }
            let out = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            if out.status.success() {
                Ok(format!("Processus {} terminé", pid))
            } else {
                Err(crate::maintenance::commands::decode_output(&out.stderr).trim().to_string())
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod diagnostic_extra_a_tests {
    use super::*;

    #[test]
    fn system_idle_process_pid_is_critical() {
        assert!(is_critical_process(0, ""));
    }

    #[test]
    fn system_pid_is_critical() {
        assert!(is_critical_process(4, "System"));
    }

    #[test]
    fn csrss_name_is_critical_regardless_of_pid() {
        assert!(is_critical_process(1234, "csrss.exe"));
        assert!(is_critical_process(1234, "csrss"));
        assert!(is_critical_process(1234, "CSRSS.EXE"));
    }

    #[test]
    fn lsass_and_winlogon_are_critical() {
        assert!(is_critical_process(999, "lsass.exe"));
        assert!(is_critical_process(999, "winlogon.exe"));
    }

    #[test]
    fn ordinary_process_is_not_critical() {
        assert!(!is_critical_process(1234, "chrome.exe"));
        assert!(!is_critical_process(1234, "notepad.exe"));
        assert!(!is_critical_process(1234, "svchost.exe"));
    }

    #[test]
    fn unresolved_name_on_ordinary_pid_is_not_critical() {
        // PID inconnu/déjà terminé, ni 0 ni 4 — laisser taskkill échouer
        // naturellement plutôt que refuser sur un nom vide.
        assert!(!is_critical_process(99999, ""));
    }

    // ── is_critical_service_action ────────────────────────────────────────────

    #[test]
    fn stopping_rpcss_is_blocked() {
        assert!(is_critical_service_action("RpcSs", "stop"));
        assert!(is_critical_service_action("rpcss", "stop"));
    }

    #[test]
    fn restarting_dcomlaunch_is_blocked() {
        assert!(is_critical_service_action("DcomLaunch", "restart"));
    }

    #[test]
    fn starting_critical_service_is_never_blocked() {
        assert!(!is_critical_service_action("RpcSs", "start"));
        assert!(!is_critical_service_action("DcomLaunch", "start"));
    }

    #[test]
    fn ordinary_service_is_not_blocked() {
        assert!(!is_critical_service_action("WinDefend", "stop"));
        assert!(!is_critical_service_action("Spooler", "stop"));
        assert!(!is_critical_service_action("wuauserv", "restart"));
    }
}

/// Services dont l'arrêt/redémarrage rend Windows non-fonctionnel (nécessite
/// généralement un reboot) — RpcSs (RPC Endpoint Mapper) et DcomLaunch (DCOM
/// Server Process Launcher) sont la plomberie de base dont dépend presque tout
/// le reste du système (documenté par Microsoft). Liste volontairement étroite
/// (contrairement au `$protected` plus large de temps_wifi_turbo.rs) : on ne
/// bloque QUE ce qui bloque le système à coup sûr, pas les services qu'un
/// outil de debloat légitime désactive intentionnellement (WinDefend, etc.).
/// `start` n'est jamais bloqué (toujours sans risque).
fn is_critical_service_action(name: &str, action: &str) -> bool {
    if action == "start" { return false; }
    matches!(name.trim().to_lowercase().as_str(), "rpcss" | "dcomlaunch")
}

/// Contrôle un service Windows (start/stop/restart)
#[tauri::command]
async fn control_service(name: String, action: String) -> Result<String, String> {
    if is_critical_service_action(&name, &action) {
        return Err(format!(
            "Service système critique ({}) — {} refusé (bloquerait le système, nécessiterait un redémarrage).",
            name, if action == "stop" { "arrêt" } else { "redémarrage" }
        ));
    }
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let cmd = match action.as_str() {
                "start" => "Start-Service",
                "stop" => "Stop-Service",
                "restart" => "Restart-Service",
                _ => return Err(format!("Action inconnue: {}", action)),
            };
            let ps = format!(
                "{} -Name '{}' -ErrorAction Stop 2>&1; Write-Output 'OK'",
                cmd, name.replace('\'', "''")
            );
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            // decode_output (repli OEM) : le message d'erreur PowerShell FR
            // (« Impossible de démarrer le service… ») serait sinon mojibake.
            let stdout = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
            let stderr = crate::maintenance::commands::decode_output(&out.stderr).trim().to_string();
            if stdout.contains("OK") || out.status.success() {
                Ok(format!("Service '{}' : {} effectué", name, action))
            } else {
                Err(if !stderr.is_empty() { stderr } else { stdout })
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Change le mode de démarrage d'un service Windows
#[tauri::command]
async fn set_service_start_mode(name: String, mode: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            // mode: "Auto", "Manual", "Disabled", "Automatic (Delayed Start)"
            let sc_mode = match mode.as_str() {
                "Auto" | "Automatic" => "auto",
                "Manual" => "demand",
                "Disabled" => "disabled",
                "Automatic (Delayed Start)" => "delayed-auto",
                _ => return Err(format!("Mode inconnu: {}", mode)),
            };
            let out = std::process::Command::new("sc")
                .args(["config", &name, &format!("start= {}", sc_mode)])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            if out.status.success() {
                Ok(format!("Service '{}' : mode '{}' appliqué", name, mode))
            } else {
                Err(crate::maintenance::commands::decode_output(&out.stderr).trim().to_string())
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Définit ou modifie une variable d'environnement
#[tauri::command]
async fn set_environment_variable(name: String, value: String, scope: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let ps_scope = match scope.as_str() {
                "Système" | "System" | "Machine" => "Machine",
                _ => "User",
            };
            // try/catch obligatoire : une exception de méthode .NET (ex. scope
            // Machine sans admin) n'arrête PAS le script — 'OK' était imprimé
            // quand même → faux-succès.
            let ps = format!(
                "try {{ [System.Environment]::SetEnvironmentVariable('{}', '{}', [System.EnvironmentVariableTarget]::{}); Write-Output 'OK' }} catch {{ Write-Output $_.Exception.Message; exit 1 }}",
                name.replace('\'', "''"),
                value.replace('\'', "''"),
                ps_scope
            );
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let stdout = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
            if out.status.success() && stdout.contains("OK") {
                Ok(format!("Variable '{}' définie ({})", name, ps_scope))
            } else {
                let stderr = crate::maintenance::commands::decode_output(&out.stderr).trim().to_string();
                Err(if !stdout.is_empty() { stdout } else { stderr })
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Supprime une variable d'environnement
#[tauri::command]
async fn delete_environment_variable(name: String, scope: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let ps_scope = match scope.as_str() {
                "Système" | "System" | "Machine" => "Machine",
                _ => "User",
            };
            // Même protocole que set_environment_variable : exception .NET
            // non-terminante → try/catch + exit 1, sinon faux-succès sans admin.
            let ps = format!(
                "try {{ [System.Environment]::SetEnvironmentVariable('{}', $null, [System.EnvironmentVariableTarget]::{}); Write-Output 'OK' }} catch {{ Write-Output $_.Exception.Message; exit 1 }}",
                name.replace('\'', "''"),
                ps_scope
            );
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let stdout = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
            if out.status.success() && stdout.contains("OK") {
                Ok(format!("Variable '{}' supprimée ({})", name, ps_scope))
            } else {
                let stderr = crate::maintenance::commands::decode_output(&out.stderr).trim().to_string();
                Err(if !stdout.is_empty() { stdout } else { stderr })
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Active ou désactive un programme de démarrage dans le registre
#[tauri::command]
async fn toggle_startup_program(name: String, location: String, command: String, enable: bool) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let hive = if location.contains("HKCU") { "HKCU:" } else { "HKLM:" };
            let reg_path = if location.contains("x86") {
                format!("{}\\SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run", hive)
            } else if location.contains("RunOnce") {
                format!("{}\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\RunOnce", hive)
            } else {
                format!("{}\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run", hive)
            };

            // try/catch + -ErrorAction Stop : les échecs de Set-ItemProperty
            // (HKLM sans admin) sont non-terminants → 'OK' sortait quand même.
            let ps = if enable {
                format!(
                    "try {{ Set-ItemProperty -Path '{}' -Name '{}' -Value '{}' -Force -ErrorAction Stop; Write-Output 'OK' }} catch {{ Write-Output $_.Exception.Message; exit 1 }}",
                    reg_path.replace('\'', "''"),
                    name.replace('\'', "''"),
                    command.replace('\'', "''")
                )
            } else {
                // Déplacer vers un stash plutôt que supprimer. Stash sous
                // SOFTWARE\NiTriTe\DisabledRun : l'ancien '{hive}\Disabled\Run'
                // créait une clé à la RACINE du hive (pollution registre).
                // Le stash n'est jamais relu (la réactivation passe par le
                // paramètre `command` du frontend) → déplacement sans migration.
                format!(
                    "try {{ \
                     $disPath = '{}\\SOFTWARE\\NiTriTe\\DisabledRun'; if(-not (Test-Path $disPath)){{ New-Item $disPath -Force -ErrorAction Stop | Out-Null }}; \
                     $val = try{{ (Get-ItemProperty -Path '{}' -Name '{}' -ErrorAction Stop).'{}' }} catch {{ '{}' }}; \
                     Set-ItemProperty -Path $disPath -Name '{}' -Value $val -Force -ErrorAction Stop; \
                     Remove-ItemProperty -Path '{}' -Name '{}' -ErrorAction Stop; Write-Output 'OK' \
                     }} catch {{ Write-Output $_.Exception.Message; exit 1 }}",
                    hive, // $disPath
                    &reg_path.replace('\'', "''"),
                    &name.replace('\'', "''"),
                    &name.replace('\'', "''"),
                    &command.replace('\'', "''"),
                    &name.replace('\'', "''"),
                    &reg_path.replace('\'', "''"),
                    &name.replace('\'', "''"),
                )
            };
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let stdout = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
            if out.status.success() && stdout.contains("OK") {
                Ok(if enable { format!("'{}' activé au démarrage", name) } else { format!("'{}' désactivé au démarrage", name) })
            } else {
                let stderr = crate::maintenance::commands::decode_output(&out.stderr).trim().to_string();
                Err(if !stdout.is_empty() { stdout } else { stderr })
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Supprime complètement une entrée de démarrage du registre
#[tauri::command]
async fn remove_startup_program(name: String, location: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let hive = if location.contains("HKCU") { "HKCU:" } else { "HKLM:" };
            let reg_path = if location.contains("x86") {
                format!("{}\\SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run", hive)
            } else {
                format!("{}\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run", hive)
            };
            let ps = format!(
                "Remove-ItemProperty -Path '{}' -Name '{}' -ErrorAction SilentlyContinue; Write-Output 'OK'",
                reg_path.replace('\'', "''"),
                name.replace('\'', "''")
            );
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            if out.status.success() {
                Ok(format!("Entrée '{}' supprimée du démarrage", name))
            } else {
                Err(crate::maintenance::commands::decode_output(&out.stderr).trim().to_string())
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Crée une tâche planifiée Windows simple
#[tauri::command]
async fn create_scheduled_task(task_name: String, command: String, trigger: String, description: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            // trigger: "startup", "logon", "daily HH:MM", "hourly N"
            let trigger_ps: String = match trigger.as_str() {
                "startup" => "New-ScheduledTaskTrigger -AtStartup".to_string(),
                "logon"   => "New-ScheduledTaskTrigger -AtLogOn".to_string(),
                t if t.starts_with("daily ") => {
                    let time = t.trim_start_matches("daily ").trim();
                    // Validation : format HH:MM strict (heures 0-23, minutes 0-59)
                    let valid = time.len() == 5
                        && time.chars().nth(2) == Some(':')
                        && time[..2].parse::<u8>().map(|h| h < 24).unwrap_or(false)
                        && time[3..].parse::<u8>().map(|m| m < 60).unwrap_or(false);
                    if !valid {
                        return Err("Format d'heure invalide. Attendu: HH:MM (ex: 14:30)".to_string());
                    }
                    format!("New-ScheduledTaskTrigger -Daily -At '{}'", time)
                }
                t if t.starts_with("hourly ") => {
                    let n = t.trim_start_matches("hourly ")
                        .trim()
                        .parse::<u32>()
                        .unwrap_or(1)
                        .max(1);
                    format!(
                        "New-ScheduledTaskTrigger -RepetitionInterval (New-TimeSpan -Hours {n}) -Once -At (Get-Date)"
                    )
                }
                _ => "New-ScheduledTaskTrigger -AtStartup".to_string(),
            };
            let safe_name = task_name.replace('\'', "''");
            let safe_cmd = command.replace('\'', "''");
            let safe_desc = description.replace('\'', "''");
            // try/catch : un échec de Register-ScheduledTask (droits, nom déjà
            // pris par une tâche protégée…) est non-terminant → 'OK' sortait
            // quand même et l'UI annonçait la tâche créée à tort.
            let ps = format!(
                r#"
try {{
    $action = New-ScheduledTaskAction -Execute '{safe_cmd}'
    $trigger = {trigger_ps}
    $settings = New-ScheduledTaskSettingsSet -RunOnlyIfNetworkAvailable:$false
    Register-ScheduledTask -TaskName '{safe_name}' -Action $action -Trigger $trigger -Settings $settings -Description '{safe_desc}' -Force -ErrorAction Stop | Out-Null
    Write-Output 'OK'
}} catch {{ Write-Output $_.Exception.Message; exit 1 }}
"#
            );
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let stdout = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
            if out.status.success() && stdout.contains("OK") {
                Ok(format!("Tâche '{}' créée avec succès", task_name))
            } else {
                let stderr = crate::maintenance::commands::decode_output(&out.stderr).trim().to_string();
                Err(if !stdout.is_empty() { stdout } else { stderr })
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Supprime une tâche planifiée Windows
#[tauri::command]
async fn delete_scheduled_task(task_name: String, task_path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let full_name = if task_path.is_empty() || task_path == "\\" {
                task_name.clone()
            } else {
                format!("{}\\{}", task_path.trim_end_matches('\\'), task_name)
            };
            let out = std::process::Command::new("schtasks")
                .args(["/Delete", "/TN", &full_name, "/F"])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            if out.status.success() {
                Ok(format!("Tâche '{}' supprimée", task_name))
            } else {
                // decode_output : schtasks écrit ses erreurs en codepage OEM sur FR.
                let err = crate::maintenance::commands::decode_output(&out.stderr).trim().to_string();
                let out_str = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
                Err(if !err.is_empty() { err } else { out_str })
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Exécute maintenant une tâche planifiée
#[tauri::command]
async fn run_scheduled_task_now(task_name: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let out = std::process::Command::new("schtasks")
                .args(["/Run", "/TN", &task_name])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            if out.status.success() {
                Ok(format!("Tâche '{}' démarrée", task_name))
            } else {
                Err(crate::maintenance::commands::decode_output(&out.stderr).trim().to_string())
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Active un plan d'alimentation Windows par GUID
#[tauri::command]
async fn set_power_plan(guid: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let out = std::process::Command::new("powercfg")
                .args(["/setactive", &guid])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            if out.status.success() {
                Ok(format!("Plan d'alimentation {} activé", guid))
            } else {
                Err(crate::maintenance::commands::decode_output(&out.stderr).trim().to_string())
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Définit l'imprimante par défaut
#[tauri::command]
async fn set_default_printer(printer_name: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            // try/catch : SetDefaultPrinter lève une exception COM non-terminante
            // (imprimante inexistante) → 'OK' sortait quand même.
            let ps = format!(
                "try {{ (New-Object -ComObject WScript.Network).SetDefaultPrinter('{}'); Write-Output 'OK' }} catch {{ Write-Output $_.Exception.Message; exit 1 }}",
                printer_name.replace('\'', "''")
            );
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let stdout = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
            if out.status.success() && stdout.contains("OK") {
                Ok(format!("Imprimante '{}' définie par défaut", printer_name))
            } else {
                let stderr = crate::maintenance::commands::decode_output(&out.stderr).trim().to_string();
                Err(if !stdout.is_empty() { stdout } else { stderr })
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Génère et ouvre le rapport HTML de batterie
#[tauri::command]
async fn open_battery_report_html() -> Result<(), String> {
    let output_path = crate::utils::paths::temp_dir().join("nitrite-battery-report.html");
    let output_str = output_path.to_string_lossy().to_string();
    let out = output_str.clone();
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("powercfg")
                .args(["/batteryreport", "/output", &out])
                .creation_flags(0x08000000)
                .status();
        }
    }).await.map_err(|e| e.to_string())?;
    if output_path.exists() {
        open::that(&output_path).map_err(|e| e.to_string())
    } else {
        Err("Rapport batterie non généré (pas de batterie ?)".to_string())
    }
}

/// Ouvre Regedit positionné sur une clé de registre précise
#[tauri::command]
async fn open_in_regedit(key_path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            // Normaliser le chemin : HKCU\ -> HKEY_CURRENT_USER\, etc.
            let full_path = key_path
                .replace("HKCU\\", "HKEY_CURRENT_USER\\")
                .replace("HKLM\\", "HKEY_LOCAL_MACHINE\\")
                .replace("HKCR\\", "HKEY_CLASSES_ROOT\\")
                .replace("HKU\\", "HKEY_USERS\\")
                .replace("HKCC\\", "HKEY_CURRENT_CONFIG\\");

            // Écrire la clé de navigation regedit dans le registre
            let set_ps = format!(
                "Set-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Applets\\Regedit' -Name 'LastKey' -Value '{}' -Force -ErrorAction SilentlyContinue",
                full_path.replace('\'', "''")
            );
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &set_ps])
                .creation_flags(0x08000000)
                .status();

            // Ouvrir regedit
            std::process::Command::new("regedit.exe")
                .creation_flags(0x00000001) // Ouvrir visible (pas de CREATE_NO_WINDOW ici)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Installe un gestionnaire de paquets (winget/scoop/chocolatey)
#[tauri::command]
async fn install_package_manager(manager: String, window: tauri::Window) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            // try/catch : un échec réseau/installeur (DownloadString lève une
            // exception non-terminante) laissait le message « installé ! »
            // s'imprimer quand même → faux-succès.
            let ps = match manager.as_str() {
                "scoop" => r#"
try {
    Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force -ErrorAction Stop
    Invoke-RestMethod -Uri 'https://get.scoop.sh' -ErrorAction Stop | Invoke-Expression
    Write-Output 'Scoop installé !'
} catch { Write-Output $_.Exception.Message; exit 1 }
"#,
                "chocolatey" => r#"
try {
    Set-ExecutionPolicy Bypass -Scope Process -Force -ErrorAction Stop
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
    Invoke-Expression ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
    Write-Output 'Chocolatey installé !'
} catch { Write-Output $_.Exception.Message; exit 1 }
"#,
                "winget" => {
                    // Winget s'installe via le Microsoft Store / App Installer
                    let _ = std::process::Command::new("cmd")
                        .args(["/C", "start ms-windows-store://pdp/?productid=9NBLGGH4NNS1"])
                        .creation_flags(0x08000000)
                        .spawn();
                    return Ok("Microsoft Store ouvert — recherchez 'App Installer' pour installer WinGet".to_string());
                }
                _ => return Err(format!("Gestionnaire inconnu: {}", manager)),
            };
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let stdout = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
            let _ = window.emit("pkg-manager-install-done", &stdout);
            if out.status.success() {
                Ok(stdout)
            } else {
                let stderr = crate::maintenance::commands::decode_output(&out.stderr).trim().to_string();
                Err(if !stdout.is_empty() { stdout } else { stderr })
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}
