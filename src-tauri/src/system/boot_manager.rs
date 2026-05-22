use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Default)]
pub struct BcdEntry {
    pub id: String,
    pub description: String,
    pub entry_type: String,
    pub device: String,
    pub path: String,
    pub locale: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct BootConfig {
    pub entries: Vec<BcdEntry>,
    pub default_id: String,
    pub timeout_secs: u32,
    pub safe_mode: bool,
    pub debug_mode: bool,
}

fn ps_out(ps: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o { return String::from_utf8_lossy(&o.stdout).to_string(); }
    }
    String::new()
}

#[tauri::command]
pub fn get_boot_config() -> BootConfig {
    let ps = r#"
try {
    $bcd = & bcdedit /enum ALL 2>&1
    $entries = @()
    $current = $null
    $defaultId = ''
    $timeout = 30
    $bcd | ForEach-Object {
        $line = $_
        if ($line -match '^-+$') {
            if ($current) { $entries += $current }
            $current = @{ id=''; desc=''; type=''; device=''; path=''; locale=''; default=$false }
        } elseif ($line -match '^identifier\s+(\S+)') {
            if ($current) { $current.id = $Matches[1] }
        } elseif ($line -match '^description\s+(.+)') {
            if ($current) { $current.desc = $Matches[1].Trim() }
        } elseif ($line -match '^device\s+(.+)') {
            if ($current) { $current.device = $Matches[1].Trim() }
        } elseif ($line -match '^path\s+(.+)') {
            if ($current) { $current.path = $Matches[1].Trim() }
        } elseif ($line -match '^locale\s+(.+)') {
            if ($current) { $current.locale = $Matches[1].Trim() }
        }
    }
    if ($current) { $entries += $current }

    # Get default
    $defLine = $bcd | Where-Object { $_ -match 'default\s+(\{[^\}]+\})' } | Select-Object -First 1
    if ($defLine -match '\{[^\}]+\}') { $defaultId = $Matches[0] }

    # Get timeout
    $toLine = $bcd | Where-Object { $_ -match 'timeout\s+(\d+)' } | Select-Object -First 1
    if ($toLine -match '\d+') { $timeout = [int]$Matches[0] }

    @{ entries=$entries; default=$defaultId; timeout=$timeout; safe=$false; debug=$false } | ConvertTo-Json -Depth 4 -Compress
} catch {
    @{ entries=@(); default=''; timeout=30; safe=$false; debug=$false } | ConvertTo-Json -Compress
}
"#;
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(t.trim()) {
                let entries = v["entries"].as_array().map(|arr| arr.iter().map(|e| {
                    let id = e["id"].as_str().unwrap_or("").to_string();
                    let default_id = v["default"].as_str().unwrap_or("").to_string();
                    BcdEntry {
                        is_default: id == default_id,
                        id,
                        description: e["desc"].as_str().unwrap_or("").to_string(),
                        entry_type: e["type"].as_str().unwrap_or("osloader").to_string(),
                        device: e["device"].as_str().unwrap_or("").to_string(),
                        path: e["path"].as_str().unwrap_or("").to_string(),
                        locale: e["locale"].as_str().unwrap_or("").to_string(),
                    }
                }).collect()).unwrap_or_default();
                return BootConfig {
                    entries,
                    default_id: v["default"].as_str().unwrap_or("").to_string(),
                    timeout_secs: v["timeout"].as_u64().unwrap_or(30) as u32,
                    safe_mode: false,
                    debug_mode: false,
                };
            }
        }
    }
    BootConfig::default()
}

#[tauri::command]
pub fn set_boot_timeout(seconds: u32) -> Result<String, String> {
    let s = seconds.min(999);
    let out = ps_out(&format!("bcdedit /timeout {}", s));
    if out.to_lowercase().contains("successfully") || out.to_lowercase().contains("success") {
        Ok(format!("Timeout défini à {} secondes", s))
    } else {
        Err(out)
    }
}

#[tauri::command]
pub fn set_default_boot(entry_id: String) -> Result<String, String> {
    let id = entry_id.replace(['"', '\''], "");
    let out = ps_out(&format!("bcdedit /default {{{}}}", id.trim_matches(|c| c == '{' || c == '}')));
    if out.to_lowercase().contains("successfully") {
        Ok(format!("Entrée de démarrage par défaut définie : {}", id))
    } else {
        Err(out)
    }
}

#[tauri::command]
pub fn boot_to_recovery() -> String {
    ps_out("shutdown /r /o /f /t 0")
}
