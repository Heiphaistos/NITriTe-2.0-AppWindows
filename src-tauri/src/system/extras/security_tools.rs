use serde::Serialize;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use super::{parse_json_arr, ps, ps_ok};

// ─── Security Quick Actions ───────────────────────────────────────────────────

// Anti-freeze : PowerShell/Set-MpPreference est bloquant — jamais inline
// sur le thread de commande.
#[tauri::command]
pub async fn toggle_defender_realtime(enable: bool) -> Result<String, String> {
    tokio::task::spawn_blocking(move || toggle_defender_realtime_blocking(enable))
        .await
        .map_err(|e| e.to_string())?
}

fn toggle_defender_realtime_blocking(enable: bool) -> Result<String, String> {
    let val = if enable { "$false" } else { "$true" };
    // -ErrorAction Stop + try/catch : Set-MpPreference est routinièrement bloqué
    // par la Tamper Protection de Defender (erreur non-terminante). Sans ça, le
    // '; OK' s'imprime malgré l'échec et ps() ne vérifie pas le code de sortie
    // → faux succès. On distingue explicitement OK / ERR.
    let script = format!(
        "try {{ Set-MpPreference -DisableRealtimeMonitoring {val} -ErrorAction Stop; 'OK' }} catch {{ \"ERR:$($_.Exception.Message)\" }}"
    );
    let out = ps(&script)?;
    let t = out.trim();
    if t.starts_with("OK") {
        Ok(if enable { "Protection temps réel activée".into() } else { "Protection temps réel désactivée".into() })
    } else {
        Err(t.trim_start_matches("ERR:").trim().to_string())
    }
}

// Anti-freeze : PowerShell est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn update_defender_signatures() -> Result<String, String> {
    tokio::task::spawn_blocking(update_defender_signatures_blocking)
        .await
        .map_err(|e| e.to_string())?
}

fn update_defender_signatures_blocking() -> Result<String, String> {
    // ps_ok() (pas ps()) : meme piege que quick_uninstall_software_blocking —
    // ps() ignore le code de sortie, donc un echec reel de Update-MpSignature
    // (pas de connexion, service Defender arrete, etc.) etait rapporte comme
    // un succes malgre -ErrorAction Stop. Verifie en direct : une erreur
    // terminante non rattrapee via -ErrorAction Stop sort bien avec un code
    // non-nul (1), que ps_ok() detecte correctement.
    ps_ok("Update-MpSignature -ErrorAction Stop; 'Définitions mises à jour'")
}

// Anti-freeze : PowerShell est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn enable_firewall_all_profiles() -> Result<String, String> {
    tokio::task::spawn_blocking(enable_firewall_all_profiles_blocking)
        .await
        .map_err(|e| e.to_string())?
}

fn enable_firewall_all_profiles_blocking() -> Result<String, String> {
    let out = std::process::Command::new("netsh")
        .args(["advfirewall", "set", "allprofiles", "state", "on"])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| e.to_string())?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() { Ok(s) } else { Err(String::from_utf8_lossy(&out.stderr).to_string()) }
}

// ─── Software Uninstall (quick) ───────────────────────────────────────────────

// Anti-freeze : PowerShell/winget est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn quick_uninstall_software(name: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || quick_uninstall_software_blocking(name))
        .await
        .map_err(|e| e.to_string())?
}

fn quick_uninstall_software_blocking(name: String) -> Result<String, String> {
    let safe = name.replace('\'', "''");
    // ps_ok() (pas ps()) : ps() renvoie Ok des que powershell.exe demarre, quel
    // que soit le code de sortie du script — inutilisable ici. Sans ca, ni les
    // throw ("app introuvable"/"chaine introuvable") ni un desinstalleur qui
    // echoue reellement (code de sortie de Start-Process -PassThru auparavant
    // jete via Out-Null) ne remontaient jamais comme un echec : la fonction
    // rapportait toujours "Desinstallation lancee" telle quelle.
    let script = format!(r#"
$paths = @(
    'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
$pat = '*' + [Management.Automation.WildcardPattern]::Escape('{safe}') + '*'
$app = Get-ItemProperty $paths -ErrorAction SilentlyContinue | Where-Object {{ $_.DisplayName -like $pat }} | Select-Object -First 1
if (-not $app) {{ Write-Output 'Application non trouvée dans le registre'; exit 1 }}
$us = $app.UninstallString
if (-not $us) {{ Write-Output 'Chaîne de désinstallation introuvable'; exit 1 }}
if ($us -match 'MsiExec') {{
    $guid = [regex]::Match($us, '\{{[^}}]+\}}').Value
    $proc = Start-Process msiexec.exe -ArgumentList "/x $guid /quiet /norestart" -Wait -PassThru
    $code = $proc.ExitCode
}} else {{
    $proc = Start-Process cmd.exe -ArgumentList "/c `"$us`" /S /SILENT /VERYSILENT /NORESTART" -Wait -PassThru
    $code = $proc.ExitCode
}}
if ($code -eq 0 -or $code -eq 3010) {{
    "Désinstallation réussie : $($app.DisplayName)"
}} else {{
    Write-Output "Désinstallation possiblement échouée (code $code) : $($app.DisplayName)"
    exit 1
}}
"#);
    ps_ok(&script)
}

// ─── All Product Keys ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ProductKey {
    pub product: String,
    pub key: String,
    pub key_type: String,
}

// Anti-freeze : requêtes registre/WMI sont bloquantes — jamais inline sur
// le thread de commande.
#[tauri::command]
pub async fn get_all_product_keys() -> Result<Vec<ProductKey>, String> {
    tokio::task::spawn_blocking(get_all_product_keys_blocking)
        .await
        .map_err(|e| e.to_string())?
}

fn get_all_product_keys_blocking() -> Result<Vec<ProductKey>, String> {
    let script = r#"
function Decode-DigitalProductId($dpid) {
    if (-not $dpid -or $dpid.Length -lt 67) { return $null }
    $kOff = 52
    $isW8 = ([int]($dpid[$kOff+14]) / 6) -band 1
    $dpid[$kOff+14] = ($dpid[$kOff+14] -band 0xF7) -bor (($isW8 -band 2) * 4)
    $ch = 'BCDFGHJKMPQRTVWXY2346789'; $ko = ''
    for ($i = 24; $i -ge 0; $i--) {
        $cu = 0
        for ($j = 14; $j -ge 0; $j--) { $cu = $cu*256+$dpid[$kOff+$j]; $dpid[$kOff+$j]=[int]($cu/24); $cu=$cu%24 }
        $ko = $ch[$cu] + $ko
    }
    $k = ''; for ($i=0;$i-lt 25;$i++){if($i-gt 0-and $i%5-eq 0){$k+='-'};$k+=$ko[$i]}
    return $k
}
$keys = @()
# Methode 1 : Cle OEM/BIOS
try {
    $sls = Get-WmiObject -Query 'SELECT OA3xOriginalProductKey FROM SoftwareLicensingService' -ErrorAction SilentlyContinue
    if ($sls -and $sls.OA3xOriginalProductKey -and $sls.OA3xOriginalProductKey.Length -gt 10) {
        $pn = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').ProductName
        $keys += [PSCustomObject]@{ product="$pn (OEM/BIOS)"; key=$sls.OA3xOriginalProductKey; type='Windows' }
    }
} catch {}
# Methode 2 : Windows via DigitalProductId
try {
    $reg = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
    $dpid = (Get-ItemProperty $reg -ErrorAction Stop).DigitalProductId
    $k = Decode-DigitalProductId $dpid
    if ($k -and $k -notmatch '^[B]+$' -and -not ($keys | Where-Object { $_.type -eq 'Windows' })) {
        $pn = (Get-ItemProperty $reg).ProductName
        $keys += [PSCustomObject]@{ product=$pn; key=$k; type='Windows' }
    }
} catch {}
# Methode 3 : Cle partielle via SLMgr
try {
    $pkr = (Get-WmiObject -Query 'SELECT PartialProductKey,Name FROM SoftwareLicensingProduct WHERE PartialProductKey IS NOT NULL AND ApplicationId="55c92734-d682-4d71-983e-d6ec3f16059f"' -ErrorAction SilentlyContinue) | Where-Object { $_.Name -match 'Windows' } | Select-Object -First 1
    if ($pkr -and $pkr.PartialProductKey -and -not ($keys | Where-Object { $_.type -eq 'Windows' })) {
        $keys += [PSCustomObject]@{ product=$pkr.Name; key="XXXXX-XXXXX-XXXXX-XXXXX-$($pkr.PartialProductKey)"; type='Windows' }
    }
} catch {}
# Office : DigitalProductId dans Registration
try {
    $ofPaths = @('HKLM:\SOFTWARE\Microsoft\Office','HKLM:\SOFTWARE\WOW6432Node\Microsoft\Office')
    foreach ($op in $ofPaths) {
        if (-not (Test-Path $op)) { continue }
        Get-ChildItem $op -ErrorAction SilentlyContinue | ForEach-Object {
            $rp = "$($_.PSPath)\Registration"
            if (-not (Test-Path $rp)) { return }
            Get-ChildItem $rp -ErrorAction SilentlyContinue | ForEach-Object {
                $rd = Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue
                if ($rd.ProductName -and $rd.DigitalProductId) {
                    $k2 = Decode-DigitalProductId $rd.DigitalProductId
                    if ($k2) { $keys += [PSCustomObject]@{ product=$rd.ProductName; key=$k2; type='Office' } }
                }
            }
        }
    }
} catch {}
# Office M365
try {
    $ctr = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Office\ClickToRun\Configuration' -ErrorAction SilentlyContinue
    if ($ctr -and $ctr.ProductReleaseIds) {
        $keys += [PSCustomObject]@{ product="Microsoft 365 (ClickToRun)"; key=$ctr.ProductReleaseIds; type='Office' }
    }
} catch {}
# Logiciels
try {
    $swPaths = @(
        'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'
    )
    Get-ItemProperty $swPaths -ErrorAction SilentlyContinue |
        Where-Object { ($_.SerialNumber -or $_.ProductKey) -and $_.DisplayName -and $_.DisplayName -notmatch '^KB\d+|Security Update|Hotfix|Update for' } |
        ForEach-Object {
            $k = if ($_.ProductKey) { $_.ProductKey } else { $_.SerialNumber }
            if ($k -and ([string]$k).Length -gt 5) {
                $keys += [PSCustomObject]@{ product=$_.DisplayName; key=[string]$k; type='Software' }
            }
        }
} catch {}
@($keys) | ConvertTo-Json -Compress -Depth 2"#;
    let out = ps(script)?;
    if out.is_empty() { return Ok(vec![]); }
    let arr: Vec<serde_json::Value> = parse_json_arr(&out);
    Ok(arr.iter().map(|v| ProductKey {
        product:  v["product"].as_str().unwrap_or("").to_string(),
        key:      v["key"].as_str().unwrap_or("").to_string(),
        key_type: v["type"].as_str().unwrap_or("Software").to_string(),
    }).filter(|k| !k.product.is_empty() && !k.key.is_empty()).collect())
}

// ─── Problem Devices (driver errors) ─────────────────────────────────────────

#[derive(Serialize)]
pub struct ProblemDevice {
    pub name: String,
    pub device_id: String,
    pub error_code: u32,
    pub error_description: String,
    pub class: String,
    pub status: String,
}

// Anti-freeze : WMI est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn get_problem_devices() -> Result<Vec<ProblemDevice>, String> {
    tokio::task::spawn_blocking(get_problem_devices_blocking)
        .await
        .map_err(|e| e.to_string())?
}

fn get_problem_devices_blocking() -> Result<Vec<ProblemDevice>, String> {
    let script = r#"
$errDesc = @{
    1='Non configuré correctement'; 2='Impossible de charger le pilote'; 3='Pilote corrompu';
    10='Impossible de démarrer'; 12='Ressources insuffisantes'; 14='Redémarrage requis';
    16='Non identifié complètement'; 18='Réinstaller les pilotes'; 19='Registre corrompu';
    21='Suppression en cours'; 22='Désactivé par l utilisateur'; 24='Périphérique absent';
    28='Pilotes non installés'; 29='Désactivé dans le BIOS'; 31='Ne fonctionne pas correctement';
    32='Service de pilote désactivé'; 33='Ressources non déterminées'; 37='Pilote a retourné une erreur';
    38='Pilote chargé deux fois'; 39='Pilote manquant ou corrompu'; 40='Problème de registre';
    41='Impossible de charger le pilote'; 42='Périphérique dupliqué'
}
Get-WmiObject Win32_PnPEntity -ErrorAction SilentlyContinue |
    Where-Object { $_.ConfigManagerErrorCode -ne 0 -and $_.Name } |
    ForEach-Object {
        $code = [int]$_.ConfigManagerErrorCode
        $desc = if ($errDesc.ContainsKey($code)) { $errDesc[$code] } else { "Erreur code $code" }
        [PSCustomObject]@{
            name=$_.Name; device_id=$_.DeviceID; error_code=$code;
            error_description=$desc; class=($_.PNPClass ?? ''); status=($_.Status ?? '')
        }
    } | ConvertTo-Json -Compress"#;
    let out = ps(script)?;
    if out.is_empty() { return Ok(vec![]); }
    let arr: Vec<serde_json::Value> = parse_json_arr(&out);
    Ok(arr.iter().filter_map(|v| {
        let name = v["name"].as_str().unwrap_or("").to_string();
        if name.is_empty() { return None; }
        Some(ProblemDevice {
            name,
            device_id:         v["device_id"].as_str().unwrap_or("").to_string(),
            error_code:        v["error_code"].as_u64().unwrap_or(0) as u32,
            error_description: v["error_description"].as_str().unwrap_or("").to_string(),
            class:             v["class"].as_str().unwrap_or("").to_string(),
            status:            v["status"].as_str().unwrap_or("").to_string(),
        })
    }).collect())
}
