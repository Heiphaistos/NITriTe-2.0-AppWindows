use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use super::{SuspiciousService, AutorunEntry};
use crate::maintenance::commands::decode_output;

// === Advanced Security ===

#[derive(Default)]
pub struct AdvancedSecurityInfo {
    pub antivirus: String,
    pub def_age_days: i64,
    pub last_bsod: String,
    pub last_update_days: i64,
    pub temp_size_mb: f64,
    pub vmem_total_mb: u64,
    pub vmem_available_mb: u64,
}

pub fn collect_advanced_security() -> AdvancedSecurityInfo {
    let ps = r#"
$out = [ordered]@{}

# Antivirus tiers via SecurityCenter2
try {
    $av = Get-WmiObject -Namespace root\SecurityCenter2 -Class AntiVirusProduct -ErrorAction SilentlyContinue |
          Where-Object { $_.displayName -notmatch "Windows Defender" } | Select-Object -First 1
    $out['AV'] = if ($av) { $av.displayName } else { "Windows Defender uniquement" }
} catch { $out['AV'] = "Inconnu" }

# Age definitions Defender (jours)
try {
    $mp = Get-MpComputerStatus -ErrorAction SilentlyContinue
    if ($mp) {
        $age = ((Get-Date) - $mp.AntivirusSignatureLastUpdated).Days
        $out['DefAge'] = $age
    } else { $out['DefAge'] = -1 }
} catch { $out['DefAge'] = -1 }

# Dernier BSOD
try {
    $bsod = Get-WinEvent -FilterHashtable @{LogName='System';Id=41} -MaxEvents 1 -ErrorAction SilentlyContinue |
            Select-Object -First 1
    $out['BSOD'] = if ($bsod) { $bsod.TimeCreated.ToString('yyyy-MM-dd HH:mm') } else { "Aucun BSOD recent" }
} catch { $out['BSOD'] = "Aucun BSOD recent" }

# Jours depuis dernier KB
try {
    $kb = Get-HotFix | Sort-Object InstalledOn -Descending | Select-Object -First 1
    if ($kb -and $kb.InstalledOn) {
        $out['KBDays'] = ((Get-Date) - $kb.InstalledOn).Days
    } else { $out['KBDays'] = -1 }
} catch { $out['KBDays'] = -1 }

# Taille dossiers temp
try {
    $t1 = (Get-ChildItem $env:TEMP -Recurse -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
    $t2 = (Get-ChildItem "$env:windir\Temp" -Recurse -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
    $out['TempMB'] = [math]::Round(($t1 + $t2) / 1MB, 1)
} catch { $out['TempMB'] = 0 }

# Memoire virtuelle (pagefile)
try {
    $cs = Get-WmiObject -Class Win32_ComputerSystem -ErrorAction SilentlyContinue
    $out['VMemTotal'] = [math]::Round($cs.TotalVirtualMemorySize / 1024)
    $out['VMemAvail'] = [math]::Round($cs.FreeVirtualMemory / 1024)
} catch { $out['VMemTotal'] = 0; $out['VMemAvail'] = 0 }

$out | ConvertTo-Json -Compress
"#;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps])
        .creation_flags(0x08000000)
        .output();

    if let Ok(o) = output {
        // decode_output : displayName d'un antivirus tiers peut être localisé FR.
        let text = decode_output(&o.stdout);
        let v: serde_json::Value = serde_json::from_str(text.trim()).unwrap_or_default();
        return AdvancedSecurityInfo {
            antivirus: v["AV"].as_str().unwrap_or("Inconnu").to_string(),
            def_age_days: v["DefAge"].as_i64().unwrap_or(-1),
            last_bsod: v["BSOD"].as_str().unwrap_or("").to_string(),
            last_update_days: v["KBDays"].as_i64().unwrap_or(-1),
            temp_size_mb: v["TempMB"].as_f64().unwrap_or(0.0),
            vmem_total_mb: v["VMemTotal"].as_u64().unwrap_or(0),
            vmem_available_mb: v["VMemAvail"].as_u64().unwrap_or(0),
        };
    }
    AdvancedSecurityInfo::default()
}

// === Suspicious Services ===

pub fn scan_suspicious_services() -> Vec<SuspiciousService> {
    let ps = r#"
Get-WmiObject -Class Win32_Service -ErrorAction SilentlyContinue |
Where-Object { $_.State -eq 'Running' -and $_.PathName -and
    $_.PathName -notmatch 'System32|SysWOW64|Microsoft|Windows\\' -and
    $_.StartMode -ne 'Disabled' } |
Select-Object Name, DisplayName, State, PathName |
Select-Object -First 20 |
ConvertTo-Json -Compress -Depth 1
"#;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps])
        .creation_flags(0x08000000)
        .output();
    // decode_output : DisplayName Win32_Service est fréquemment localisé FR
    // (ex. « Service de plateforme de données agrégées »), confirmé en direct
    // sur cette machine — from_utf8_lossy corrompt ces noms en mojibake.
    let text = match output { Ok(o) => decode_output(&o.stdout), Err(_) => return vec![] };
    let val: serde_json::Value = serde_json::from_str(text.trim()).unwrap_or(serde_json::Value::Array(vec![]));
    let arr = match val {
        serde_json::Value::Array(a) => a,
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => return vec![],
    };
    arr.iter().map(|v| SuspiciousService {
        name: v["Name"].as_str().unwrap_or("").to_string(),
        display_name: v["DisplayName"].as_str().unwrap_or("").to_string(),
        state: v["State"].as_str().unwrap_or("").to_string(),
        path: v["PathName"].as_str().unwrap_or("").chars().take(100).collect(),
    }).collect()
}

// === Autoruns ===

pub fn scan_autoruns() -> Vec<AutorunEntry> {
    let ps = r#"
$entries = @()
$runKeys = @(
    'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run',
    'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run',
    'HKCU:\Software\Microsoft\Windows\CurrentVersion\RunOnce',
    'HKLM:\Software\Microsoft\Windows\CurrentVersion\RunOnce'
)
foreach ($key in $runKeys) {
    try {
        $props = Get-ItemProperty -Path $key -ErrorAction SilentlyContinue
        if ($props) {
            $props.PSObject.Properties | Where-Object { $_.Name -notmatch '^PS' } | ForEach-Object {
                $val = $_.Value -replace '"',''
                if ($val -notmatch 'Windows|System32|Microsoft') {
                    $entries += [PSCustomObject]@{ Name=$_.Name; Path=$val; Location=$key -replace 'HKCU:','HKCU' -replace 'HKLM:','HKLM' }
                }
            }
        }
    } catch {}
}
$entries | Select-Object -First 25 | ConvertTo-Json -Compress -Depth 1
"#;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps])
        .creation_flags(0x08000000)
        .output();
    // decode_output : valeurs de clés Run peuvent être un chemin/label accentué.
    let text = match output { Ok(o) => decode_output(&o.stdout), Err(_) => return vec![] };
    let val: serde_json::Value = serde_json::from_str(text.trim()).unwrap_or(serde_json::Value::Array(vec![]));
    let arr = match val {
        serde_json::Value::Array(a) => a,
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => return vec![],
    };
    arr.iter().map(|v| AutorunEntry {
        name: v["Name"].as_str().unwrap_or("").to_string(),
        path: v["Path"].as_str().unwrap_or("").to_string(),
        location: v["Location"].as_str().unwrap_or("").to_string(),
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Live, non-CI (ignorée par défaut) : confirme sur une vraie machine Windows
    // que scan_suspicious_services() ne produit plus de mojibake sur un
    // DisplayName Win32_Service réel (ex. « Intel® Graphics Software Service »,
    // « Service de plateforme de données agrégées » — corruption confirmée avant
    // le fix decode_output via un repro Rust direct sur cette machine).
    #[test]
    #[ignore]
    fn scan_suspicious_services_no_mojibake_on_real_services() {
        let services = scan_suspicious_services();
        let bad = services.iter().find(|s| s.display_name.contains('\u{FFFD}') || s.name.contains('\u{FFFD}'));
        assert!(bad.is_none(), "mojibake détecté dans un service réel : {:?}", bad);
    }
}
