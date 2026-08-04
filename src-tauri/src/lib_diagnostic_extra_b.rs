
/// Ouvre le gestionnaire de périphériques Windows filtré sur un type
#[tauri::command]
async fn open_device_manager(_device_class: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            // Ouvrir le gestionnaire de périphériques
            let _ = std::process::Command::new("devmgmt.msc")
                .creation_flags(0x08000000)
                .spawn();
            // Scanner les périphériques pour mises à jour
            let _ = std::process::Command::new("pnputil")
                .args(["/scan-devices"])
                .creation_flags(0x08000000)
                .spawn();
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Récupère les informations étendues BIOS (TPM, Secure Boot, type firmware)
#[tauri::command]
async fn get_bios_extended() -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let ps = r#"
$result = @{}
try {
    $bios = Get-WmiObject -Class Win32_BIOS -ErrorAction SilentlyContinue
    $result.bios_version = if ($bios) { [string]$bios.SMBIOSBIOSVersion } else { "" }
    $result.bios_date = if ($bios) { [string]$bios.ReleaseDate } else { "" }
    $result.bios_manufacturer = if ($bios) { [string]$bios.Manufacturer } else { "" }
    $result.bios_description = if ($bios) { [string]$bios.Description } else { "" }
} catch {}
try {
    # Type firmware UEFI ou BIOS legacy
    $isUEFI = try { Confirm-SecureBootUEFI -ErrorAction Stop; $true } catch { $false }
    $result.firmware_type = if ($isUEFI -or (Test-Path 'HKLM:\SYSTEM\CurrentControlSet\Control\SecureBoot\State')) { "UEFI" } else { "BIOS Legacy" }
} catch { $result.firmware_type = "Inconnu" }
try {
    # Secure Boot
    $sb = (Get-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\SecureBoot\State' -ErrorAction SilentlyContinue).UEFISecureBootEnabled
    $result.secure_boot = [bool]$sb
} catch { $result.secure_boot = $false }
try {
    # TPM
    $tpm = Get-WmiObject -Namespace "root\CIMV2\Security\MicrosoftTpm" -Class Win32_Tpm -ErrorAction SilentlyContinue
    if ($tpm) {
        $result.tpm_present = $true
        $result.tpm_enabled = [bool]$tpm.IsEnabled_InitialValue
        $result.tpm_activated = [bool]$tpm.IsActivated_InitialValue
        $result.tpm_version = if ($tpm.PhysicalPresenceVersionInfo) { [string]$tpm.PhysicalPresenceVersionInfo } else { "1.x" }
        $result.tpm_spec_version = try { [string]$tpm.SpecVersion.Split(',')[0].Trim() } catch { "" }
    } else {
        $result.tpm_present = $false; $result.tpm_enabled = $false; $result.tpm_activated = $false; $result.tpm_version = ""
        # Essai via Get-Tpm
        $tpm2 = try { Get-Tpm -ErrorAction SilentlyContinue } catch { $null }
        if ($tpm2) { $result.tpm_present = [bool]$tpm2.TpmPresent; $result.tpm_enabled = [bool]$tpm2.TpmEnabled }
    }
} catch { $result.tpm_present = $false; $result.tpm_enabled = $false }
try {
    # Chassis type
    $enclosure = Get-WmiObject -Class Win32_SystemEnclosure -ErrorAction SilentlyContinue | Select-Object -First 1
    $result.chassis_type = switch ([int]($enclosure.ChassisTypes | Select-Object -First 1)) {
        1{"Autre"} 2{"Inconnu"} 3{"Desktop"} 4{"Low Profile Desktop"} 5{"Pizza Box"}
        6{"Mini Tower"} 7{"Tower"} 8{"Portable"} 9{"Laptop"} 10{"Notebook"}
        11{"Handheld"} 12{"Docking Station"} 13{"All-in-One"} 14{"Sub-Notebook"}
        30{"Tablet"} 31{"Convertible"} 32{"Detachable"} default{"PC"}
    }
    $result.system_manufacturer = [string]$enclosure.Manufacturer
} catch { $result.chassis_type = "PC" }
try {
    # Fonctionnalités BIOS supplémentaires
    $result.wake_on_lan = try {
        $adapters = Get-NetAdapterAdvancedProperty -RegistryKeyword "*WakeOnMagicPacket" -ErrorAction SilentlyContinue
        [bool]($adapters | Where-Object { $_.DisplayValue -eq "Enabled" })
    } catch { $false }
} catch {}
try {
    $result.fast_boot = [int](Get-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Power" -Name "HiberbootEnabled" -ErrorAction SilentlyContinue).HiberbootEnabled -eq 1
} catch { $result.fast_boot = $false }
$result | ConvertTo-Json -Compress
"#;
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let text = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
            if text.is_empty() { return Ok(serde_json::Value::Null); }
            serde_json::from_str(&text).map_err(|e| format!("JSON parse: {} — raw: {}", e, text.chars().take(200).collect::<String>()))
        }
        #[cfg(not(target_os = "windows"))]
        Ok(serde_json::Value::Null)
    }).await.map_err(|e| e.to_string())?
}

/// Récupère les informations étendues de la carte mère (slots, chipset, socket)
#[tauri::command]
async fn get_motherboard_extended() -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let ps = r#"
$result = @{}
try {
    $board = Get-WmiObject -Class Win32_BaseBoard -ErrorAction SilentlyContinue
    $result.manufacturer = [string]$board.Manufacturer
    $result.product = [string]$board.Product
    $result.version = [string]$board.Version
    $result.serial = [string]$board.SerialNumber
    $result.tag = [string]$board.Tag
} catch {}
try {
    $sys = Get-WmiObject -Class Win32_ComputerSystem -ErrorAction SilentlyContinue
    $result.model = [string]$sys.Model
    $result.total_ram_slots_phys = [int]$sys.TotalPhysicalMemory
} catch {}
try {
    # Slots d'extension
    $slots = Get-WmiObject -Class Win32_SystemSlot -ErrorAction SilentlyContinue
    $result.expansion_slots = @($slots | ForEach-Object {
        @{
            name = [string]$_.SlotDesignation
            type = [string]$_.ConnectorType
            status = [string]$_.CurrentUsage
            max_data_width = [int]$_.MaxDataWidth
        }
    })
    $result.slot_count = $slots.Count
    $result.slot_available = ($slots | Where-Object { $_.CurrentUsage -eq 3 }).Count  # 3 = Available
} catch { $result.expansion_slots = @(); $result.slot_count = 0 }
try {
    # Socket CPU
    $proc = Get-WmiObject -Class Win32_Processor -ErrorAction SilentlyContinue | Select-Object -First 1
    $result.cpu_socket = [string]$proc.SocketDesignation
    $result.cpu_family = [string]$proc.Family
} catch {}
try {
    # Temperature via MSAcpi (peut échouer sur systèmes sans ACPI thermal)
    $temp_k = try {
        (Get-WmiObject -Namespace "root\WMI" -Class MSAcpi_ThermalZoneTemperature -ErrorAction SilentlyContinue |
        Select-Object -First 1).CurrentTemperature
    } catch { $null }
    $result.motherboard_temp_c = if ($temp_k -and $temp_k -gt 0) { [int]($temp_k / 10 - 273.15) } else { -1 }
} catch { $result.motherboard_temp_c = -1 }
$result | ConvertTo-Json -Compress -Depth 4
"#;
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let text = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
            if text.is_empty() { return Ok(serde_json::Value::Null); }
            serde_json::from_str(&text).map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "windows"))]
        Ok(serde_json::Value::Null)
    }).await.map_err(|e| e.to_string())?
}

/// Récupère la fréquence de rafraîchissement exacte de l'écran
#[tauri::command]
async fn get_monitor_refresh_rates() -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let ps = r#"
try {
    $result = @()
    $gpus = Get-WmiObject -Class Win32_VideoController -ErrorAction SilentlyContinue
    foreach ($gpu in $gpus) {
        if ($gpu.CurrentRefreshRate -gt 0) {
            $result += @{
                gpu_name = [string]$gpu.Name
                refresh_rate_hz = [int]$gpu.CurrentRefreshRate
                resolution = "$($gpu.CurrentHorizontalResolution)x$($gpu.CurrentVerticalResolution)"
                bits_per_pixel = [int]$gpu.CurrentBitsPerPixel
                video_mode = [string]$gpu.VideoModeDescription
            }
        }
    }
    if ($result.Count -eq 1) { $result[0] | ConvertTo-Json -Compress }
    else { $result | ConvertTo-Json -Compress }
} catch { Write-Output '{}' }
"#;
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let text = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
            if text.is_empty() { return Ok(serde_json::Value::Null); }
            let json_text = if text.starts_with('{') && !text.starts_with('[') {
                format!("[{}]", text)
            } else { text };
            serde_json::from_str(&json_text).map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "windows"))]
        Ok(serde_json::Value::Null)
    }).await.map_err(|e| e.to_string())?
}

/// Récupère les licences de logiciels tiers depuis le registre
#[tauri::command]
async fn get_third_party_licenses() -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let ps = r#"
# Sortie UTF-8 : statuts accentués codés en dur (« Licencié », « Grâce étendue »,
# « Abonnement Microsoft 365 ») + Description WMI → sinon mojibake garanti.
$OutputEncoding = [System.Text.Encoding]::UTF8; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$result = @()
# Logiciels avec licences dans le registre
$regPaths = @(
    'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\SoftwareProtectionPlatform',
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows NT\CurrentVersion\SoftwareProtectionPlatform'
)
# Chercher des logiciels communs avec clés de licence connues
$softwareChecks = @(
    @{ name="Adobe Acrobat"; reg="HKLM:\SOFTWARE\Adobe\Adobe Acrobat"; key="Serial" },
    @{ name="AutoCAD"; reg="HKLM:\SOFTWARE\Autodesk\AutoCAD"; key="SERIALNUMBER" },
    @{ name="EaseUS"; reg="HKLM:\SOFTWARE\EaseUS"; key="LicenseKey" }
)

# Licence Windows depuis SLP
try {
    $slp = Get-WmiObject -Query "SELECT Name,LicenseStatus,PartialProductKey,Description FROM SoftwareLicensingProduct WHERE PartialProductKey IS NOT NULL" -ErrorAction SilentlyContinue
    foreach ($lic in $slp) {
        $status = switch ($lic.LicenseStatus) {
            0{"Non licencié"} 1{"Licencié"} 2{"Grâce OOB"} 3{"Grâce OOT"}
            4{"Non-authentique"} 5{"Notification"} 6{"Grâce étendue"} default{"Inconnu"}
        }
        $result += @{
            software = [string]$lic.Name
            status = $status
            partial_key = [string]$lic.PartialProductKey
            type = "Windows/Office"
            description = [string]$lic.Description
        }
    }
} catch {}

# Office Click-to-Run
try {
    $c2r = Get-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Office\ClickToRun\Configuration' -ErrorAction SilentlyContinue
    if ($c2r) {
        $result += @{
            software = "Microsoft Office (C2R)"
            status = if ($c2r.LicensingTenantId) { "Abonnement Microsoft 365" } else { "Installé" }
            partial_key = ""
            type = "Office C2R"
            description = if ($c2r.ProductReleaseIds) { [string]$c2r.ProductReleaseIds } else { "Office Click-to-Run" }
        }
    }
} catch {}

if ($result.Count -eq 0) { Write-Output '[]' } else { $result | ConvertTo-Json -Compress -Depth 3 }
"#;
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let text = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
            if text.is_empty() || text == "[]" { return Ok(serde_json::json!([])); }
            let json_text = if text.starts_with('{') { format!("[{}]", text) } else { text };
            serde_json::from_str(&json_text).map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "windows"))]
        Ok(serde_json::json!([]))
    }).await.map_err(|e| e.to_string())?
}

/// Ouvre les paramètres Windows d'activation
#[tauri::command]
async fn open_activation_settings() -> Result<(), String> {
    open::that("ms-settings:activation").map_err(|e| e.to_string())
}

/// Exécute slmgr et retourne le résultat
#[tauri::command]
async fn run_slmgr(arg: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            let safe_arg = match arg.as_str() {
                "/xpr" | "/dlv" | "/dli" | "/ato" => arg.as_str(),
                _ => return Err("Argument non autorisé".to_string()),
            };
            // [Console]::OutputEncoding=UTF8 : la sortie de slmgr.vbs est localisée
            // et accentuée sur Windows FR (« Le PC est activé de façon permanente »,
            // « période de grâce »…) → sinon mojibake dans le statut d'activation.
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command",
                    &format!("[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; $r = cscript.exe //Nologo $env:SystemRoot\\System32\\slmgr.vbs {} 2>&1; $r -join \"`n\"", safe_arg)])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            Ok(crate::maintenance::commands::decode_output(&out.stdout).trim().to_string())
        }
        #[cfg(not(target_os = "windows"))]
        Err("Non supporté".to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Récupère infos audio étendues (fréquence, améliorations)
#[tauri::command]
async fn get_audio_extended() -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let ps = r#"
$result = @()
try {
    $devices = Get-WmiObject -Class Win32_SoundDevice -ErrorAction SilentlyContinue
    foreach ($d in $devices) {
        $item = @{
            name = [string]$d.Name
            manufacturer = [string]$d.Manufacturer
            status = [string]$d.Status
            device_id = [string]$d.DeviceID
            pnp_device_id = [string]$d.PNPDeviceID
        }
        # Essai de récupérer le format audio depuis le registre
        try {
            $regBase = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio"
            # Chercher dans Render et Capture
            foreach ($subkey in @("Render", "Capture")) {
                $path = "$regBase\$subkey"
                if (Test-Path $path) {
                    Get-ChildItem $path -ErrorAction SilentlyContinue | ForEach-Object {
                        $propPath = "$($_.PSPath)\Properties"
                        if (Test-Path $propPath) {
                            $props = Get-ItemProperty $propPath -ErrorAction SilentlyContinue
                            $desc = if ($props."{a45c254e-df1c-4efd-8020-67d146a850e0},2") { $props."{a45c254e-df1c-4efd-8020-67d146a850e0},2" } else { "" }
                            if ($desc -and [string]$desc -like "*$([string]$d.Name.Split(' ')[0])*") {
                                $item.audio_type = $subkey
                            }
                        }
                    }
                }
            }
        } catch {}
        $result += $item
    }
} catch {}
if ($result.Count -eq 0) { Write-Output '[]' } else { $result | ConvertTo-Json -Compress -Depth 3 }
"#;
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let text = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
            if text.is_empty() || text == "[]" { return Ok(serde_json::json!([])); }
            let json_text = if text.starts_with('{') { format!("[{}]", text) } else { text };
            serde_json::from_str(&json_text).map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "windows"))]
        Ok(serde_json::json!([]))
    }).await.map_err(|e| e.to_string())?
}

/// Récupère les processus avec utilisation GPU et disque
#[tauri::command]
async fn get_processes_extended() -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let ps = format!("{}{}", crate::utils::ps::LOC_COUNTER_PRELUDE, r#"
try {
    $procs = Get-Process -ErrorAction SilentlyContinue | Select-Object -First 200
    $result = @()
    foreach ($p in $procs) {
        $result += @{
            pid = [int]$p.Id
            name = [string]$p.Name
            cpu_percent = 0.0
            memory_mb = [double]($p.WorkingSet64 / 1MB)
            disk_io_read_kb = try { [double]($p.ReadTransferCount / 1KB) } catch { 0.0 }
            disk_io_write_kb = try { [double]($p.WriteTransferCount / 1KB) } catch { 0.0 }
        }
    }
    # GPU usage via Get-Counter (meilleures infos si dispo)
    try {
        $pGpu = "\{0}(*)\{1}" -f (Loc-Counter 'GPU Engine'), (Loc-Counter 'Utilization Percentage')
        $gpuCounters = Get-Counter $pGpu -ErrorAction SilentlyContinue -MaxSamples 1
        if ($gpuCounters) {
            $gpuByPid = @{}
            $gpuCounters.CounterSamples | Where-Object { $_.CookedValue -gt 0 } | ForEach-Object {
                if ($_.InstanceName -match 'pid_(\d+)') {
                    $pid2 = [int]$matches[1]
                    if (-not $gpuByPid[$pid2] -or $gpuByPid[$pid2] -lt $_.CookedValue) {
                        $gpuByPid[$pid2] = [double]$_.CookedValue
                    }
                }
            }
            $result = $result | ForEach-Object {
                $pid3 = $_.pid
                $_.gpu_percent = if ($gpuByPid[$pid3]) { [double]$gpuByPid[$pid3] } else { 0.0 }
                $_
            }
        }
    } catch {}
    $result | ConvertTo-Json -Compress -Depth 2
} catch { Write-Output '[]' }
"#);
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps.as_str()])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            let text = crate::maintenance::commands::decode_output(&out.stdout).trim().to_string();
            if text.is_empty() || text == "[]" { return Ok(serde_json::json!([])); }
            let json_text = if text.starts_with('{') { format!("[{}]", text) } else { text };
            serde_json::from_str(&json_text).map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "windows"))]
        Ok(serde_json::json!([]))
    }).await.map_err(|e| e.to_string())?
}

