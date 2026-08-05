use serde::Serialize;
use tauri::Emitter;

#[cfg(target_os = "windows")]
use std::io::{BufRead, BufReader};
#[cfg(target_os = "windows")]
use std::process::Stdio;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub index: u32,
    pub label: String,
    pub size_gb: f64,
    pub partitions: Vec<PartitionInfo>,
    pub disk_type: String,
    pub bus_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PartitionInfo {
    pub letter: String,
    pub label: String,
    pub size_gb: f64,
    pub free_gb: f64,
    pub file_system: String,
    pub is_system: bool,
    pub is_boot: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CloneResult {
    pub success: bool,
    pub method: String,
    pub message: String,
    pub duration_secs: u64,
}

// ════════════════════════════════════════════════════════
//  Détection des disques
// ════════════════════════════════════════════════════════

pub fn get_disks() -> Vec<DiskInfo> {
    let ps = r#"
try {
    $disks = Get-Disk -ErrorAction SilentlyContinue
    if (-not $disks) { Write-Output '[]'; exit }
    $result = @()
    foreach ($disk in $disks) {
        $partitions = @()
        $parts = Get-Partition -DiskNumber $disk.Number -ErrorAction SilentlyContinue
        foreach ($part in $parts) {
            $vol = $null
            try { $vol = Get-Volume -Partition $part -ErrorAction SilentlyContinue } catch {}
            $letter = if ($part.DriveLetter) { "$($part.DriveLetter):" } else { "" }
            $partitions += @{
                letter      = $letter
                label       = if ($vol) { [string]$vol.FileSystemLabel } else { "" }
                size_gb     = [math]::Round([double]$part.Size / 1GB, 2)
                free_gb     = if ($vol) { [math]::Round([double]$vol.SizeRemaining / 1GB, 2) } else { 0 }
                file_system = if ($vol) { [string]$vol.FileSystem } else { "RAW" }
                is_system   = [bool]$part.IsSystem
                is_boot     = [bool]$part.IsBoot
            }
        }
        $result += @{
            index      = [int]$disk.Number
            label      = [string]$disk.FriendlyName
            size_gb    = [math]::Round([double]$disk.Size / 1GB, 2)
            disk_type  = [string]$disk.MediaType
            bus_type   = [string]$disk.BusType
            partitions = $partitions
        }
    }
    $result | ConvertTo-Json -Depth 4 -Compress
} catch { Write-Output '[]' }
"#;

    run_ps(ps)
        .and_then(|txt| {
            let txt = txt.trim();
            if txt.is_empty() || txt == "[]" { return Some(vec![]); }
            let json = if txt.starts_with('{') { format!("[{}]", txt) } else { txt.to_string() };
            serde_json::from_str::<Vec<serde_json::Value>>(&json).ok()
        })
        .map(|vals| vals.into_iter().filter_map(parse_disk).collect())
        .unwrap_or_default()
}

fn parse_disk(v: serde_json::Value) -> Option<DiskInfo> {
    let parts: Vec<PartitionInfo> = v["partitions"]
        .as_array()
        .map(|arr| arr.iter().filter_map(parse_part).collect())
        .unwrap_or_default();
    Some(DiskInfo {
        index: v["index"].as_u64()? as u32,
        label: v["label"].as_str().unwrap_or("Disque inconnu").to_string(),
        size_gb: v["size_gb"].as_f64().unwrap_or(0.0),
        disk_type: v["disk_type"].as_str().unwrap_or("HDD").to_string(),
        bus_type: v["bus_type"].as_str().unwrap_or("").to_string(),
        partitions: parts,
    })
}

fn parse_part(v: &serde_json::Value) -> Option<PartitionInfo> {
    Some(PartitionInfo {
        letter: v["letter"].as_str().unwrap_or("").to_string(),
        label: v["label"].as_str().unwrap_or("").to_string(),
        size_gb: v["size_gb"].as_f64().unwrap_or(0.0),
        free_gb: v["free_gb"].as_f64().unwrap_or(0.0),
        file_system: v["file_system"].as_str().unwrap_or("").to_string(),
        is_system: v["is_system"].as_bool().unwrap_or(false),
        is_boot: v["is_boot"].as_bool().unwrap_or(false),
    })
}

// ════════════════════════════════════════════════════════
//  Image Système (wbadmin)
// ════════════════════════════════════════════════════════

pub fn create_system_image(target_drive: String, window: &tauri::Window) -> CloneResult {
    let start = std::time::Instant::now();
    emit(window, "start", 5, "Vérification des droits administrateur...");

    #[cfg(target_os = "windows")]
    {
        // ── Droits admin obligatoires pour wbadmin ──────────────
        if !is_admin() {
            return CloneResult {
                success: false,
                method: "wbadmin".into(),
                message: "Droits administrateur requis. Clic droit sur Nitrite → \
                          Exécuter en tant qu'administrateur, puis réessayez.".into(),
                duration_secs: 0,
            };
        }

        // ── Validation du lecteur cible ─────────────────────────
        let target_letter = format!("{}:", target_drive.trim_end_matches(':').trim());
        let target_path   = format!("{}\\", target_letter);
        if !std::path::Path::new(&target_path).exists() {
            return CloneResult {
                success: false,
                method: "wbadmin".into(),
                message: format!(
                    "Lecteur cible {} introuvable. Assurez-vous qu'il est connecté, \
                     monté et formaté en NTFS.",
                    target_letter
                ),
                duration_secs: 0,
            };
        }

        emit(window, "preparing", 10,
             "Lancement de Windows Backup — cette opération peut durer 15-60 min...");

        // ── Lancement wbadmin avec capture stdout ───────────────
        let mut child = match std::process::Command::new("wbadmin")
            .args([
                "start", "backup",
                &format!("-backuptarget:{}:", target_drive.trim_end_matches(':')),
                "-allCritical",
                "-quiet",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(0x08000000)
            .spawn()
        {
            Ok(c) => c,
            Err(e) => return CloneResult {
                success: false,
                method: "wbadmin".into(),
                message: format!(
                    "Impossible de lancer wbadmin: {}. \
                     Vérifiez que le service Windows Backup est actif.",
                    e
                ),
                duration_secs: start.elapsed().as_secs(),
            },
        };

        // ── Thread de lecture stdout → progression réelle ───────
        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => return CloneResult {
                success: false,
                method: "wbadmin".into(),
                message: "Impossible de lire la sortie du processus wbadmin.".into(),
                duration_secs: 0,
            },
        };
        let window2 = window.clone();
        let reader_thread = std::thread::spawn(move || {
            // Lecture par octets + décodage OEM : wbadmin sort en codepage OEM
            // (CP850 FR) ; lines() échouerait sur la 1re ligne accentuée et
            // figerait la progression.
            let mut br = BufReader::new(stdout);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                match br.read_until(b'\n', &mut buf) {
                    Ok(0) => break,
                    Ok(_) => {
                        while matches!(buf.last(), Some(b'\n') | Some(b'\r')) { buf.pop(); }
                        let line = decode_console(&buf);
                        // wbadmin : "…, copied (X%)." (EN) / "…, copié (X %)." (FR)
                        if let Some(pct) = parse_wbadmin_pct(&line) {
                            // Mappe 0-100% sur la plage 10-95%
                            let mapped = ((pct as f64 * 0.85) as u32).saturating_add(10).min(95);
                            let _ = window2.emit("clone-progress", serde_json::json!({
                                "step": "progress",
                                "percent": mapped,
                                "message": format!("Sauvegarde du volume système : {}% terminé...", pct)
                            }));
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Drainer stderr entièrement (octets bruts) : le piper sans le vider
        // bloquerait wbadmin (deadlock) dès que son buffer stderr se remplit.
        let stderr_thread = child.stderr.take().map(|mut stderr| {
            std::thread::spawn(move || {
                let _ = std::io::copy(&mut stderr, &mut std::io::sink());
            })
        });

        // ── Attente de fin de processus ─────────────────────────
        match child.wait() {
            Ok(status) => {
                let _ = reader_thread.join();
                if let Some(t) = stderr_thread { let _ = t.join(); }
                let ok   = status.success();
                let code = status.code().unwrap_or(-1);
                let step = if ok { "done" } else { "error" };
                let msg_emit = if ok {
                    "Image système créée avec succès ✓".to_string()
                } else {
                    format!("Échec wbadmin (code {})", code)
                };
                emit(window, step, 100, &msg_emit);
                CloneResult {
                    success: ok,
                    method: "Windows Backup (wbadmin)".into(),
                    message: if ok {
                        format!(
                            "Image système complète sauvegardée sur {} en {}min {}s.",
                            target_letter,
                            start.elapsed().as_secs() / 60,
                            start.elapsed().as_secs() % 60
                        )
                    } else {
                        format!(
                            "wbadmin a échoué (code {}). Causes fréquentes : \
                             disque non NTFS, espace insuffisant (doit être ≥ taille de C:), \
                             droits insuffisants.",
                            code
                        )
                    },
                    duration_secs: start.elapsed().as_secs(),
                }
            }
            Err(e) => {
                let _ = reader_thread.join();
                CloneResult {
                    success: false,
                    method: "wbadmin".into(),
                    message: format!("Erreur d'attente du processus wbadmin: {}", e),
                    duration_secs: start.elapsed().as_secs(),
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    CloneResult {
        success: false,
        method: "wbadmin".into(),
        message: "Non supporté hors Windows.".into(),
        duration_secs: 0,
    }
}

/// Parse "copied (X%)" depuis la sortie wbadmin
/// Décode une ligne console brute (UTF-8 d'abord, repli OEM sur Windows FR).
#[cfg(target_os = "windows")]
fn decode_console(bytes: &[u8]) -> String {
    crate::maintenance::commands::decode_output(bytes)
}
#[cfg(not(target_os = "windows"))]
fn decode_console(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn parse_wbadmin_pct(line: &str) -> Option<u32> {
    // Indépendant de la locale : wbadmin traduit le texte autour ("copied" EN /
    // "copié" FR) mais le pourcentage reste "N%" ou "N %" (espace insécable FR).
    // On repère le '%' et on remonte sur les espaces puis les chiffres.
    let bytes = line.as_bytes();
    let pct = line.find('%')?;
    let mut i = pct;
    while i > 0 && bytes[i - 1] == b' ' { i -= 1; }
    let end = i;
    while i > 0 && bytes[i - 1].is_ascii_digit() { i -= 1; }
    if i == end { return None; }
    line[i..end].parse::<u32>().ok().filter(|&p| p <= 100)
}

/// Vérifie que le processus courant tourne en tant qu'administrateur (élevé)
fn is_admin() -> bool {
    #[cfg(target_os = "windows")]
    {
        // S-1-16-12288 = High Mandatory Level = token élevé (admin réel)
        std::process::Command::new("whoami")
            .args(["/groups"])
            .creation_flags(0x08000000)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("S-1-16-12288"))
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    false
}

// ════════════════════════════════════════════════════════
//  Clone disque-à-disque (Robocopy)
// ════════════════════════════════════════════════════════

pub fn clone_with_robocopy(
    source_drive: String,
    target_drive: String,
    window: &tauri::Window,
) -> CloneResult {
    let start = std::time::Instant::now();

    let src = format!("{}:\\", source_drive.trim_end_matches([':', '\\']));
    let dst = format!("{}:\\", target_drive.trim_end_matches([':', '\\']));

    // ── Vérifications de sécurité ───────────────────────────────
    // Windows n'est pas toujours installé sur C: (installs personnalisés,
    // systèmes multi-boot, certaines configurations OEM) — comparer à la
    // vraie lettre système (%SystemDrive%), jamais à "C:" en dur, sous peine
    // de laisser passer un robocopy qui écrase le Windows en cours d'exécution
    // sur une machine où celui-ci n'est pas sur C:.
    let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
    if is_system_drive(&dst, &system_drive) {
        return CloneResult {
            success: false,
            method: "Robocopy".into(),
            message: format!("Destination {} interdite — écrasement du système Windows impossible.", system_drive),
            duration_secs: 0,
        };
    }
    if src.eq_ignore_ascii_case(&dst) {
        return CloneResult {
            success: false,
            method: "Robocopy".into(),
            message: "Source et destination sont identiques.".into(),
            duration_secs: 0,
        };
    }

    #[cfg(target_os = "windows")]
    {
        if !std::path::Path::new(&src).exists() {
            return CloneResult {
                success: false,
                method: "Robocopy".into(),
                message: format!("Lecteur source {} introuvable.", src),
                duration_secs: 0,
            };
        }
        if !std::path::Path::new(&dst).exists() {
            return CloneResult {
                success: false,
                method: "Robocopy".into(),
                message: format!("Lecteur destination {} introuvable.", dst),
                duration_secs: 0,
            };
        }

        emit(window, "start", 5, &format!("Démarrage Robocopy : {} → {}...", src, dst));

        // ── Lancement Robocopy ──────────────────────────────────
        // /E       : copie tous les sous-dossiers y compris vides
        // /B       : mode backup, contourne les ACL restrictives
        // /DCOPY:DA: attributs + timestamps des dossiers
        // /COPY:DAT: données + attributs + timestamps des fichiers
        // /R:2 /W:3: 2 tentatives, 3s d'attente (évite les blocages sur fichiers verrouillés)
        // /MT:8    : 8 threads parallèles
        let mut child = match std::process::Command::new("robocopy")
            .args([
                src.as_str(), dst.as_str(),
                "/E",
                "/B",
                "/DCOPY:DA",
                "/COPY:DAT",
                "/R:2", "/W:3",
                "/MT:8",
                "/NFL", "/NDL", "/NP",
            ])
            .creation_flags(0x08000000)
            .spawn()
        {
            Ok(c) => c,
            Err(e) => return CloneResult {
                success: false,
                method: "Robocopy".into(),
                message: format!("Impossible de lancer Robocopy: {}", e),
                duration_secs: start.elapsed().as_secs(),
            },
        };

        // ── Polling avec progression basée sur le temps ─────────
        // Robocopy ne fournit pas de pourcentage global facilement parsable.
        // Estimation : 0→90% sur 30 minutes (1800s), stable à 90% ensuite.
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            match child.try_wait() {
                Ok(Some(status)) => {
                    let code     = status.code().unwrap_or(-1);
                    let ok       = code < 8;
                    let step     = if ok { "done" } else { "error" };
                    let msg_emit = if ok {
                        "Clonage terminé ✓".to_string()
                    } else {
                        format!("Terminé avec erreurs (code {})", code)
                    };
                    emit(window, step, 100, &msg_emit);
                    return CloneResult {
                        success: ok,
                        method: "Robocopy /E /B".into(),
                        message: robocopy_message(code, &src, &dst),
                        duration_secs: start.elapsed().as_secs(),
                    };
                }
                Ok(None) => {
                    let elapsed = start.elapsed().as_secs();
                    let pct = ((elapsed as f64 / 1800.0) * 88.0).min(90.0) as u32 + 5;
                    emit(window, "progress", pct, &format!(
                        "Copie en cours... {}min {}s écoulées",
                        elapsed / 60, elapsed % 60
                    ));
                }
                Err(e) => return CloneResult {
                    success: false,
                    method: "Robocopy".into(),
                    message: format!("Erreur de suivi du processus: {}", e),
                    duration_secs: start.elapsed().as_secs(),
                },
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    CloneResult {
        success: false,
        method: "Robocopy".into(),
        message: "Non supporté hors Windows.".into(),
        duration_secs: 0,
    }
}

/// Vérifie que `dst` ne pointe pas vers le lecteur système (celui contenant
/// Windows) — écraser ce lecteur via robocopy corromprait l'installation en
/// cours d'exécution.
fn is_system_drive(dst: &str, system_drive: &str) -> bool {
    let dst_letter = dst.trim_end_matches(['\\', '/']).to_uppercase();
    let sys_letter = system_drive.trim_end_matches(['\\', '/']).to_uppercase();
    !sys_letter.is_empty() && dst_letter.starts_with(&sys_letter)
}

/// Traduit les codes de sortie Robocopy en message clair
fn robocopy_message(code: i32, src: &str, dst: &str) -> String {
    let detail = match code {
        0  => "Aucune différence — source et destination déjà identiques.",
        1  => "Tous les fichiers copiés avec succès.",
        2  => "Fichiers supplémentaires en destination (non présents dans source).",
        3  => "Fichiers copiés + extras en destination.",
        4  => "Discordances de fichiers détectées (sans copie).",
        5..=7 => "Copie réussie avec avertissements mineurs.",
        8  => "Certains fichiers n'ont pas pu être copiés (fichiers verrouillés ou accès refusé).",
        16 => "Erreur fatale — aucun fichier copié. Vérifiez source et destination.",
        _  => "Opération terminée.",
    };
    format!("{} → {} — {}", src, dst, detail)
}

// ════════════════════════════════════════════════════════
//  Utilitaires internes
// ════════════════════════════════════════════════════════

fn emit(window: &tauri::Window, step: &str, pct: u32, msg: &str) {
    let _ = window.emit("clone-progress", serde_json::json!({
        "step": step, "percent": pct, "message": msg
    }));
}

fn run_ps(script: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let o = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(0x08000000)
            .output().ok()?;
        // decode_output : labels de disque/volume souvent nommés par l'utilisateur
        // en FR accentué ("Données", "Système") — sinon mojibake dans get_disks.
        Some(crate::maintenance::commands::decode_output(&o.stdout))
    }
    #[cfg(not(target_os = "windows"))]
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_wbadmin_pct ─────────────────────────────────────────────────────

    #[test]
    fn parse_wbadmin_pct_extracts_percentage() {
        // wbadmin output format: "xxx copied (25%)"
        let line = "Creating a backup of volume C:, copied (25%)";
        assert_eq!(parse_wbadmin_pct(line), Some(25));
    }

    #[test]
    fn parse_wbadmin_pct_100_percent() {
        let line = "Backup operations completed, copied (100%)";
        assert_eq!(parse_wbadmin_pct(line), Some(100));
    }

    #[test]
    fn parse_wbadmin_pct_no_match_returns_none() {
        assert_eq!(parse_wbadmin_pct("No percentage here"), None);
        assert_eq!(parse_wbadmin_pct(""), None);
        // '%' sans chiffre devant
        assert_eq!(parse_wbadmin_pct("100 percent (%)"), None);
    }

    #[test]
    fn parse_wbadmin_pct_french_output() {
        // Sortie FR : "copié (25 %)" avec espace avant le %
        assert_eq!(parse_wbadmin_pct("Création d'une sauvegarde du volume (C:), copié (25 %)."), Some(25));
        assert_eq!(parse_wbadmin_pct("copié (100 %)"), Some(100));
    }

    // ── is_system_drive ───────────────────────────────────────────────────────
    // Régression : l'ancien check comparait dst à "C:" en dur, laissant passer
    // un robocopy destructeur vers le lecteur système sur une machine où
    // Windows n'est pas installé sur C: (installs personnalisés, multi-boot).

    #[test]
    fn system_drive_blocked_when_matches_env() {
        assert!(is_system_drive("C:\\", "C:"));
    }

    #[test]
    fn non_system_drive_allowed() {
        assert!(!is_system_drive("D:\\", "C:"));
    }

    #[test]
    fn custom_system_drive_letter_blocked() {
        // Windows installé sur D: (pas C:) — la vraie faille corrigée ici.
        assert!(is_system_drive("D:\\", "D:"));
        assert!(!is_system_drive("C:\\", "D:"));
    }

    #[test]
    fn case_insensitive_match() {
        assert!(is_system_drive("c:\\", "C:"));
    }

    // ── robocopy_message ──────────────────────────────────────────────────────

    #[test]
    fn robocopy_message_code_0_no_difference() {
        let msg = robocopy_message(0, "C:", "D:");
        assert!(msg.contains("identiques"), "Got: {}", msg);
        assert!(msg.contains("C:") && msg.contains("D:"));
    }

    #[test]
    fn robocopy_message_code_1_success() {
        let msg = robocopy_message(1, "C:", "D:");
        assert!(msg.contains("succès"), "Got: {}", msg);
    }

    #[test]
    fn robocopy_message_code_8_error() {
        let msg = robocopy_message(8, "C:", "D:");
        assert!(msg.contains("n'ont pas pu"), "Got: {}", msg);
    }

    #[test]
    fn robocopy_message_code_16_fatal() {
        let msg = robocopy_message(16, "C:", "D:");
        assert!(msg.contains("fatale"), "Got: {}", msg);
    }

    #[test]
    fn robocopy_message_unknown_code_terminee() {
        let msg = robocopy_message(42, "C:", "D:");
        assert!(msg.contains("terminée"), "Got: {}", msg);
    }
}
