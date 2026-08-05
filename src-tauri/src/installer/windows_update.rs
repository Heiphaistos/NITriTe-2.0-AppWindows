use serde::Serialize;
use crate::error::NiTriTeError;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Serialize, Clone)]
pub struct WuUpdate {
    pub title: String,
    pub kbs: String,
    pub severity: String,
    pub size_mb: f64,
    pub category: String,
}

#[tauri::command]
pub async fn search_pending_updates() -> Result<Vec<WuUpdate>, NiTriTeError> {
    tokio::task::spawn_blocking(|| {
        let ps = r#"
try {
    $Session = New-Object -ComObject Microsoft.Update.Session
    $Searcher = $Session.CreateUpdateSearcher()
    $Result = $Searcher.Search("IsInstalled=0 and IsHidden=0")
    $updates = @($Result.Updates) | ForEach-Object {
        [PSCustomObject]@{
            Title    = $_.Title
            KBs      = if ($_.KBArticleIDs.Count -gt 0) { $_.KBArticleIDs -join "," } else { "" }
            Severity = if ($_.MsrcSeverity) { $_.MsrcSeverity } else { "N/A" }
            SizeMB   = [math]::Round($_.MaxDownloadSize / 1MB, 1)
            Category = if ($_.Categories.Count -gt 0) { $_.Categories.Item(0).Name } else { "Autre" }
        }
    }
    if ($updates) { $updates | ConvertTo-Json -Compress -Depth 2 } else { Write-Output "[]" }
} catch {
    Write-Output "[]"
}
"#;
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| NiTriTeError::System(e.to_string()))?;

        // Meme piege que install_windows_updates ci-dessous : PowerShell redirige
        // stdout en codepage OEM, un titre de MAJ accentue (KB FR) casserait le JSON.
        let text = crate::maintenance::commands::decode_output(&output.stdout).trim().to_string();
        if text.is_empty() || text == "[]" {
            return Ok(vec![]);
        }

        let json: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or(serde_json::Value::Array(vec![]));

        let arr = match json {
            serde_json::Value::Array(arr) => arr,
            obj @ serde_json::Value::Object(_) => vec![obj],
            _ => return Ok(vec![]),
        };

        Ok(arr
            .iter()
            .map(|v| WuUpdate {
                title: v["Title"].as_str().unwrap_or("").to_string(),
                kbs: v["KBs"].as_str().unwrap_or("").to_string(),
                severity: v["Severity"].as_str().unwrap_or("N/A").to_string(),
                size_mb: v["SizeMB"].as_f64().unwrap_or(0.0),
                category: v["Category"].as_str().unwrap_or("Autre").to_string(),
            })
            .collect())
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[tauri::command]
pub async fn install_windows_updates(window: tauri::Window) -> Result<bool, NiTriTeError> {
    use tauri::Emitter;

    let ps = r#"
try {
    Write-Output "[INFO] Initialisation Windows Update Agent..."
    $Session = New-Object -ComObject Microsoft.Update.Session
    $Searcher = $Session.CreateUpdateSearcher()
    Write-Output "[INFO] Recherche des mises a jour disponibles..."
    $Result = $Searcher.Search("IsInstalled=0 and IsHidden=0")
    $Count = $Result.Updates.Count
    Write-Output "[INFO] $Count mise(s) a jour trouvee(s)"
    if ($Count -eq 0) { Write-Output "[OK] Systeme deja a jour."; exit 0 }

    Write-Output "[TELECHARGEMENT] Demarrage..."
    $Downloader = $Session.CreateUpdateDownloader()
    $Downloader.Updates = $Result.Updates
    $DL = $Downloader.Download()
    Write-Output "[TELECHARGEMENT] Code resultat: $($DL.ResultCode)"

    Write-Output "[INSTALLATION] Demarrage..."
    $Installer = $Session.CreateUpdateInstaller()
    $Installer.Updates = $Result.Updates
    $Inst = $Installer.Install()
    Write-Output "[INSTALLATION] Code resultat: $($Inst.ResultCode)"
    if ($Inst.RebootRequired) { Write-Output "[ATTENTION] Redemarrage requis apres installation" }

    $failCount = 0
    for ($i = 0; $i -lt $Count; $i++) {
        $upd = $Result.Updates.Item($i)
        $code = $Inst.GetUpdateResult($i).ResultCode
        $status = switch ($code) { 2 { "OK" } 3 { "ERREUR" } 4 { "EN COURS" } 5 { "ABANDONNE" } default { "CODE $code" } }
        if ($code -ne 2) { $failCount++ }
        Write-Output "[$status] $($upd.Title)"
    }
    Write-Output "[TERMINE] Installation terminee"
    # Le code de sortie du script ne refletait avant que "a-t-il leve une
    # exception ?", pas "les mises a jour ont-elles reellement reussi ?" —
    # une installation ou CHAQUE mise a jour echoue (ResultCode 3/5) se
    # terminait quand meme normalement, exit 0, faux succes. Le frontend
    # (UpdatesPage.vue) fait confiance au booleen : succes => toast "Installation
    # terminee" ET la liste des MAJ en attente est videe, alors que rien n'a
    # ete installe.
    if ($failCount -gt 0) { exit 1 }
} catch {
    Write-Output "[ERREUR] $($_.Exception.Message)"
    exit 1
}
"#;

    let win = window.clone();
    tokio::task::spawn_blocking(move || -> Result<bool, NiTriTeError> {
        use std::io::{BufRead, BufReader};

        let mut child = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| NiTriTeError::System(e.to_string()))?;

        if let Some(stdout) = child.stdout.take() {
            // Lecture octets + décodage OEM : PowerShell 5.1 redirige stdout dans
            // le codepage OEM (pas UTF-8), donc un titre de MAJ accentué (KB FR)
            // ferait échouer lines() et tronquerait le reste du log.
            let mut reader = BufReader::new(stdout);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf) {
                    Ok(0) => break,
                    Ok(_) => {
                        while matches!(buf.last(), Some(b'\n') | Some(b'\r')) { buf.pop(); }
                        let _ = win.emit("wu-log", crate::maintenance::commands::decode_output(&buf));
                    }
                    Err(_) => break,
                }
            }
        }

        let status = child.wait().map_err(|e| NiTriTeError::System(e.to_string()))?;
        Ok(status.success())
    })
    .await
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}
