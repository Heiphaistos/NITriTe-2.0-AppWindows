
/// Termine un processus par son PID
#[tauri::command]
async fn kill_process(pid: u32) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let out = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            if out.status.success() {
                Ok(format!("Processus {} terminé", pid))
            } else {
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Contrôle un service Windows (start/stop/restart)
#[tauri::command]
async fn control_service(name: String, action: String) -> Result<String, String> {
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
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
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
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
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
            let ps = format!(
                "[System.Environment]::SetEnvironmentVariable('{}', '{}', [System.EnvironmentVariableTarget]::{}); Write-Output 'OK'",
                name.replace('\'', "''"),
                value.replace('\'', "''"),
                ps_scope
            );
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if stdout.contains("OK") || out.status.success() {
                Ok(format!("Variable '{}' définie ({})", name, ps_scope))
            } else {
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
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
            let ps = format!(
                "[System.Environment]::SetEnvironmentVariable('{}', $null, [System.EnvironmentVariableTarget]::{}); Write-Output 'OK'",
                name.replace('\'', "''"),
                ps_scope
            );
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if stdout.contains("OK") || out.status.success() {
                Ok(format!("Variable '{}' supprimée ({})", name, ps_scope))
            } else {
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
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

            let ps = if enable {
                format!(
                    "Set-ItemProperty -Path '{}' -Name '{}' -Value '{}' -Force; Write-Output 'OK'",
                    reg_path.replace('\'', "''"),
                    name.replace('\'', "''"),
                    command.replace('\'', "''")
                )
            } else {
                // Déplacer vers Disabled\Run plutôt que supprimer
                format!(
                    "$disPath = '{}\\Disabled\\Run'; if(-not (Test-Path $disPath)){{ New-Item $disPath -Force | Out-Null }}; \
                     $val = try{{ (Get-ItemProperty -Path '{}' -Name '{}' -ErrorAction Stop).'{}' }} catch {{ '{}' }}; \
                     Set-ItemProperty -Path $disPath -Name '{}' -Value $val -Force; \
                     Remove-ItemProperty -Path '{}' -Name '{}' -ErrorAction SilentlyContinue; Write-Output 'OK'",
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
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if stdout.contains("OK") || out.status.success() {
                Ok(if enable { format!("'{}' activé au démarrage", name) } else { format!("'{}' désactivé au démarrage", name) })
            } else {
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
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
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
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
                _ => "New-ScheduledTaskTrigger -AtStartup".to_string(),
            };
            let safe_name = task_name.replace('\'', "''");
            let safe_cmd = command.replace('\'', "''");
            let safe_desc = description.replace('\'', "''");
            let ps = format!(
                r#"
$action = New-ScheduledTaskAction -Execute '{safe_cmd}'
$trigger = {trigger_ps}
$settings = New-ScheduledTaskSettingsSet -RunOnlyIfNetworkAvailable:$false
Register-ScheduledTask -TaskName '{safe_name}' -Action $action -Trigger $trigger -Settings $settings -Description '{safe_desc}' -Force
Write-Output 'OK'
"#
            );
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if stdout.contains("OK") || out.status.success() {
                Ok(format!("Tâche '{}' créée avec succès", task_name))
            } else {
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
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
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                let out_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
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
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
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
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
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
            let ps = format!(
                "(New-Object -ComObject WScript.Network).SetDefaultPrinter('{}'); Write-Output 'OK'",
                printer_name.replace('\'', "''")
            );
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if stdout.contains("OK") || out.status.success() {
                Ok(format!("Imprimante '{}' définie par défaut", printer_name))
            } else {
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
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
            let ps = match manager.as_str() {
                "scoop" => r#"
Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
Invoke-RestMethod -Uri 'https://get.scoop.sh' | Invoke-Expression
Write-Output 'Scoop installé !'
"#,
                "chocolatey" => r#"
Set-ExecutionPolicy Bypass -Scope Process -Force
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
Invoke-Expression ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
Write-Output 'Chocolatey installé !'
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
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let _ = window.emit("pkg-manager-install-done", &stdout);
            if out.status.success() {
                Ok(stdout)
            } else {
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
            }
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}
