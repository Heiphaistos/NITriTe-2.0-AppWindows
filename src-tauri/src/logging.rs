/// logging.rs — Système de logs structurés NiTriTe
/// Format fichier : JSON Lines (.jsonl) — une entrée par ligne
/// Rotation automatique à 1 MB → .logs/archive/
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use chrono::Local;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

// ── Constantes ────────────────────────────────────────────────────────────────

const LOG_FILENAME: &str  = "nitrite.log";
const MAX_SIZE_BYTES: u64 = 1_048_576; // 1 MB
const RECENT_DEFAULT: usize = 500;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id:           String,
    pub timestamp:    String,
    pub level:        String,
    pub source:       String,
    pub message:      String,
    pub details:      Option<String>,
    pub duration_ms:  Option<u64>,
    pub session_id:   String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogStats {
    pub total:    usize,
    pub debug:    usize,
    pub info:     usize,
    pub warn:     usize,
    pub error:    usize,
    pub critical: usize,
    pub file_size_kb: u64,
}

// ── Mutex global (accès fichier thread-safe) ──────────────────────────────────

pub static LOG_MUTEX: Mutex<()> = Mutex::new(());

// ── Chemin du fichier log — absolu basé sur le répertoire de l'exe ────────────
// Un chemin relatif comme ".logs/" se résoudrait selon le CWD du processus,
// qui peut varier selon le contexte de lancement (Tauri, terminal, test).
// On ancre toujours les logs dans le même répertoire que l'exécutable.

fn logs_base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".logs")
}

fn log_path() -> PathBuf {
    logs_base_dir().join(LOG_FILENAME)
}

fn archive_dir() -> PathBuf {
    logs_base_dir().join("archive")
}

// ── Initialisation (crée le dossier .logs/ si absent) ────────────────────────

pub fn init_log_dir() {
    let _ = fs::create_dir_all(logs_base_dir());
    let _ = fs::create_dir_all(archive_dir());
}

// ── Rotation 1 MB ─────────────────────────────────────────────────────────────

fn rotate_if_needed() {
    let path = log_path();
    if let Ok(meta) = fs::metadata(&path) {
        if meta.len() >= MAX_SIZE_BYTES {
            let ts = Local::now().format("%Y%m%d_%H%M%S");
            let archive = archive_dir().join(format!("nitrite_{ts}.log"));
            let _ = fs::rename(&path, archive);
        }
    }
}

// ── Écriture d'une entrée ─────────────────────────────────────────────────────

fn write_entry(entry: &LogEntry) -> std::io::Result<()> {
    // Récupère le lock même si un thread précédent a paniqué (mutex poisonné)
    let _guard = LOG_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    rotate_if_needed();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())?;
    let line = serde_json::to_string(entry)
        .unwrap_or_else(|_| format!(r#"{{"error":"serialize_fail","timestamp":"{}"}}"#, entry.timestamp));
    writeln!(file, "{line}")?;
    Ok(())
}

// ── Écriture interne (Rust → log, sans command Tauri) ────────────────────────

pub fn log_internal(level: &str, source: &str, message: &str, details: Option<String>) {
    let entry = LogEntry {
        id:          format!("{}-rust", Local::now().timestamp_millis()),
        timestamp:   Local::now().to_rfc3339(),
        level:       level.to_string(),
        source:      source.to_string(),
        message:     message.to_string(),
        details,
        duration_ms: None,
        session_id:  "rust".to_string(),
    };
    let _ = write_entry(&entry);
}

// ── Commandes Tauri ───────────────────────────────────────────────────────────

/// Reçoit une entrée de log depuis le frontend et la persiste
#[tauri::command]
pub async fn log_entry(
    entry: LogEntry,
    app: tauri::AppHandle,
) -> Result<(), String> {
    write_entry(&entry).map_err(|e| e.to_string())?;
    // Broadcast temps réel → LogsPage
    let _ = app.emit("log:entry", &entry);
    Ok(())
}

/// Retourne les N dernières entrées du fichier de log
#[tauri::command]
pub async fn get_recent_logs(count: Option<usize>) -> Result<Vec<LogEntry>, String> {
    let n = count.unwrap_or(RECENT_DEFAULT);
    let path = log_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let file = fs::File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut entries: Vec<LogEntry> = Vec::new();
    for line in reader.lines().map_while(Result::ok) {
        if let Ok(entry) = serde_json::from_str::<LogEntry>(&line) {
            entries.push(entry);
        }
    }
    // Retourner les N plus récentes (fin de fichier)
    let total = entries.len();
    if total > n {
        Ok(entries.into_iter().skip(total - n).collect())
    } else {
        Ok(entries)
    }
}

/// Vide le fichier de log courant (archive d'abord)
#[tauri::command]
pub async fn clear_logs() -> Result<(), String> {
    let path = log_path();
    if path.exists() {
        // Archive avant suppression
        let ts = Local::now().format("%Y%m%d_%H%M%S");
        let archive = archive_dir().join(format!("nitrite_{ts}_cleared.log"));
        let _ = fs::copy(&path, archive);
        fs::write(&path, "").map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Statistiques du fichier de log
#[tauri::command]
pub async fn get_log_stats() -> Result<LogStats, String> {
    let path = log_path();
    if !path.exists() {
        return Ok(LogStats { total: 0, debug: 0, info: 0, warn: 0, error: 0, critical: 0, file_size_kb: 0 });
    }
    let file_size_kb = fs::metadata(&path).map(|m| m.len() / 1024).unwrap_or(0);
    let file = fs::File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut stats = LogStats { total: 0, debug: 0, info: 0, warn: 0, error: 0, critical: 0, file_size_kb };
    for line in reader.lines().map_while(Result::ok) {
        if let Ok(entry) = serde_json::from_str::<LogEntry>(&line) {
            stats.total += 1;
            match entry.level.as_str() {
                "DEBUG"    => stats.debug    += 1,
                "INFO"     => stats.info     += 1,
                "WARN"     => stats.warn     += 1,
                "ERROR"    => stats.error    += 1,
                "CRITICAL" => stats.critical += 1,
                _ => {}
            }
        }
    }
    Ok(stats)
}

/// Liste les fichiers d'archive
#[tauri::command]
pub async fn list_log_archives() -> Result<Vec<String>, String> {
    let dir = archive_dir();
    if !dir.exists() { return Ok(vec![]); }
    let mut files = vec![];
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            if let Some(name) = e.file_name().to_str() {
                files.push(name.to_string());
            }
        }
    }
    files.sort_by(|a, b| b.cmp(a)); // plus récent en premier
    Ok(files)
}

/// Retourne le chemin absolu du fichier de log
#[tauri::command]
pub async fn get_log_file_path() -> Result<String, String> {
    let abs = fs::canonicalize(log_path())
        .unwrap_or_else(|_| log_path());
    Ok(abs.to_string_lossy().to_string())
}
