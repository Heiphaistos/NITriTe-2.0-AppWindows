use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Default)]
pub struct RegEntry {
    pub hive: String,
    pub key: String,
    pub name: String,
    pub value: String,
    pub suspicious: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RegistryPersistence {
    pub run_hklm: Vec<RegEntry>,
    pub run_hkcu: Vec<RegEntry>,
    pub run_once: Vec<RegEntry>,
    pub appinit_dlls: Vec<String>,
    pub ifeo_debuggers: Vec<RegEntry>,
    pub winlogon: Vec<RegEntry>,
    pub lsa_packages: Vec<String>,
    pub browser_hijack: Vec<RegEntry>,
    pub total_suspicious: u32,
}

// Anti-freeze : parcours registre est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn get_registry_persistence() -> RegistryPersistence {
    tokio::task::spawn_blocking(get_registry_persistence_blocking)
        .await
        .unwrap_or_default()
}

fn get_registry_persistence_blocking() -> RegistryPersistence {
    let ps = r#"
$out = @{}
$safe = @('Windows','System32','SysWOW64','Program Files','Microsoft','MsEdge','OneDrive')

function IsSusp($v) {
    if (-not $v) { return $false }
    $v = [string]$v
    ($v -match 'Temp|AppData\\Local\\[^P]|cmd\.exe|powershell\.exe|wscript|cscript|mshta|rundll32' -and
     ($safe | Where-Object { $v -match $_ } | Measure-Object).Count -eq 0)
}

function Get-RunEntries($path, $hive) {
    try {
        $props = Get-ItemProperty -Path $path -ErrorAction SilentlyContinue
        if ($props) {
            return @($props.PSObject.Properties | Where-Object { $_.Name -notmatch '^PS' } | ForEach-Object {
                $v = [string]$_.Value
                @{hive=$hive; key=$path; name=$_.Name; value=$v; suspicious=[bool](IsSusp $v)}
            })
        }
    } catch {}
    return @()
}

$out.RunHKLM   = Get-RunEntries 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run' 'HKLM'
$out.RunHKCU   = Get-RunEntries 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run' 'HKCU'
$out.RunOnce   = @(
    (Get-RunEntries 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce' 'HKLM') +
    (Get-RunEntries 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce' 'HKCU')
)

# AppInit DLLs (malware classique)
try {
    $ai = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Windows' -ErrorAction SilentlyContinue).AppInit_DLLs
    $out.AppInitDLLs = if ($ai -and $ai.Trim()) { @($ai -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ }) } else { @() }
} catch { $out.AppInitDLLs = @() }

# Image File Execution Options (debugger hijack)
try {
    $ifeo = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\*' `
        -ErrorAction SilentlyContinue | Where-Object { $_.Debugger -and $_.Debugger -notmatch 'vsjitdebugger|drwtsn32|windbg' }
    $out.IFEO = @($ifeo | Select-Object -First 20 | ForEach-Object {
        $key = $_.PSPath -replace 'Microsoft.PowerShell.Core\\Registry::',''
        @{hive='HKLM'; key=$key; name='Debugger'; value=[string]$_.Debugger; suspicious=$true}
    })
} catch { $out.IFEO = @() }

# Winlogon (userinit, shell hijack)
try {
    $wl = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon' -ErrorAction SilentlyContinue
    $wlEntries = @()
    if ($wl.Shell -and $wl.Shell -ne 'explorer.exe') {
        $wlEntries += @{hive='HKLM'; key='Winlogon'; name='Shell'; value=[string]$wl.Shell; suspicious=$true}
    }
    if ($wl.Userinit -and $wl.Userinit -notmatch 'userinit\.exe') {
        $wlEntries += @{hive='HKLM'; key='Winlogon'; name='Userinit'; value=[string]$wl.Userinit; suspicious=$true}
    }
    $out.Winlogon = $wlEntries
} catch { $out.Winlogon = @() }

# LSA Auth Packages
try {
    $lsa = (Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Lsa' -ErrorAction SilentlyContinue)
    $pkgs = @()
    if ($lsa.AuthenticationPackages) { $pkgs += $lsa.AuthenticationPackages }
    if ($lsa.SecurityPackages) { $pkgs += $lsa.SecurityPackages }
    $out.LsaPackages = @($pkgs | Where-Object { $_ -and $_ -notmatch '^(msv1_0|kerberos|wdigest|tspkg|pku2u|CloudAP)$' } | Select-Object -First 10)
} catch { $out.LsaPackages = @() }

# Proxy + search hijack navigateurs
try {
    $proxy = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings' -ErrorAction SilentlyContinue
    $hijack = @()
    if ($proxy.ProxyEnable -eq 1 -and $proxy.ProxyServer) {
        $hijack += @{hive='HKCU'; key='InetSettings'; name='ProxyServer'; value=[string]$proxy.ProxyServer; suspicious=$false}
    }
    $out.BrowserHijack = $hijack
} catch { $out.BrowserHijack = @() }

# Total suspect
$total = 0
$total += ($out.RunHKLM | Where-Object { $_.suspicious }).Count
$total += ($out.RunHKCU | Where-Object { $_.suspicious }).Count
$total += ($out.RunOnce | Where-Object { $_.suspicious }).Count
$total += $out.AppInitDLLs.Count
$total += $out.IFEO.Count
$total += $out.Winlogon.Count
$out.TotalSusp = $total

$out | ConvertTo-Json -Depth 4 -Compress
"#;

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output();

        if let Ok(o) = output {
            // decode_output : valeurs Run/IFEO/Winlogon peuvent contenir des
            // chemins utilisateur accentués (ex: C:\Users\Jérôme\...).
            let text = crate::maintenance::commands::decode_output(&o.stdout);
            let v: serde_json::Value = match serde_json::from_str(text.trim()) {
                Ok(val) => val, Err(_) => return RegistryPersistence::default(),
            };

            let parse_entries = |key: &str| -> Vec<RegEntry> {
                v[key].as_array().map(|a| a.iter().map(|e| RegEntry {
                    hive: e["hive"].as_str().unwrap_or("").to_string(),
                    key: e["key"].as_str().unwrap_or("").to_string(),
                    name: e["name"].as_str().unwrap_or("").to_string(),
                    value: e["value"].as_str().unwrap_or("").chars().take(200).collect(),
                    suspicious: e["suspicious"].as_bool().unwrap_or(false),
                }).collect()).unwrap_or_default()
            };

            let total_suspicious = v["TotalSusp"].as_u64().unwrap_or(0) as u32;

            return RegistryPersistence {
                run_hklm: parse_entries("RunHKLM"),
                run_hkcu: parse_entries("RunHKCU"),
                run_once: parse_entries("RunOnce"),
                appinit_dlls: v["AppInitDLLs"].as_array().map(|a| a.iter()
                    .filter_map(|s| s.as_str().map(|x| x.to_string())).collect()).unwrap_or_default(),
                ifeo_debuggers: parse_entries("IFEO"),
                winlogon: parse_entries("Winlogon"),
                lsa_packages: v["LsaPackages"].as_array().map(|a| a.iter()
                    .filter_map(|s| s.as_str().map(|x| x.to_string())).collect()).unwrap_or_default(),
                browser_hijack: parse_entries("BrowserHijack"),
                total_suspicious,
            };
        }
    }
    RegistryPersistence::default()
}

// ── Navigateur de registre ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Default)]
pub struct RegBrowseResult {
    pub path: String,
    pub subkeys: Vec<String>,
    pub values: Vec<RegValue>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RegValue {
    pub name: String,
    pub kind: String,
    pub data: String,
}

// Anti-freeze : accès registre est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn registry_browse(path: String) -> RegBrowseResult {
    tokio::task::spawn_blocking(move || registry_browse_blocking(path))
        .await
        .unwrap_or_default()
}

fn registry_browse_blocking(path: String) -> RegBrowseResult {
    let ps = format!(
        r#"
$ErrorActionPreference = 'Stop'
try {{
    $key = Get-Item -LiteralPath 'Registry::{}' -ErrorAction Stop
    $subkeys = $key.GetSubKeyNames() | Sort-Object
    $values = @()
    foreach ($v in $key.GetValueNames()) {{
        $kind = $key.GetValueKind($v).ToString()
        $raw = $key.GetValue($v)
        $data = if ($raw -eq $null) {{ '' }} elseif ($raw -is [byte[]]) {{ ($raw | ForEach-Object {{ '{{0:X2}}' -f $_ }}) -join ' ' }} elseif ($raw -is [string[]]) {{ $raw -join ' | ' }} else {{ $raw.ToString() }}
        $values += [PSCustomObject]@{{ Name = if ($v -eq '') {{ '(Default)' }} else {{ $v }}; Kind = $kind; Data = $data }}
    }}
    [PSCustomObject]@{{ Path = '{}'; Subkeys = $subkeys; Values = $values; Error = $null }} | ConvertTo-Json -Depth 3
}} catch {{
    [PSCustomObject]@{{ Path = '{}'; Subkeys = @(); Values = @(); Error = $_.Exception.Message }} | ConvertTo-Json
}}
"#,
        path.replace('\'', "''"),
        path.replace('\'', "''"),
        path.replace('\'', "''")
    );

    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    if let Ok(out) = cmd.output() {
        let stdout = crate::maintenance::commands::decode_output(&out.stdout);
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&stdout) {
            let subkeys = v["Subkeys"].as_array()
                .map(|a| a.iter().filter_map(|s| s.as_str().map(|x| x.to_string())).collect())
                .unwrap_or_default();
            let values = v["Values"].as_array()
                .map(|a| a.iter().filter_map(|item| {
                    Some(RegValue {
                        name: item["Name"].as_str()?.to_string(),
                        kind: item["Kind"].as_str().unwrap_or("Unknown").to_string(),
                        data: item["Data"].as_str().unwrap_or("").to_string(),
                    })
                }).collect())
                .unwrap_or_default();
            let error = v["Error"].as_str().filter(|s| !s.is_empty()).map(|s| s.to_string());
            return RegBrowseResult { path, subkeys, values, error };
        }
    }

    RegBrowseResult { path: path.clone(), error: Some("Accès refusé ou chemin invalide".to_string()), ..Default::default() }
}

// Anti-freeze : écriture registre est bloquante — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn registry_set_value(path: String, name: String, data: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || registry_set_value_blocking(path, name, data))
        .await
        .map_err(|e| e.to_string())?
}

fn registry_set_value_blocking(path: String, name: String, data: String) -> Result<String, String> {
    let real_name = if name == "(Default)" { "" } else { &name };
    let ps = format!(
        r#"Set-ItemProperty -LiteralPath 'Registry::{}' -Name '{}' -Value '{}' -ErrorAction Stop; 'OK'"#,
        path.replace('\'', "''"),
        real_name.replace('\'', "''"),
        data.replace('\'', "''")
    );
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok("Valeur mise à jour".to_string()) }
    else { Err(crate::maintenance::commands::decode_output(&out.stderr).trim().to_string()) }
}

// Anti-freeze : écriture registre est bloquante — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn registry_delete_value(path: String, name: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || registry_delete_value_blocking(path, name))
        .await
        .map_err(|e| e.to_string())?
}

fn registry_delete_value_blocking(path: String, name: String) -> Result<String, String> {
    let ps = format!(
        r#"Remove-ItemProperty -LiteralPath 'Registry::{}' -Name '{}' -ErrorAction Stop; 'OK'"#,
        path.replace('\'', "''"),
        name.replace('\'', "''")
    );
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok("Valeur supprimée".to_string()) }
    else { Err(crate::maintenance::commands::decode_output(&out.stderr).trim().to_string()) }
}
