use std::sync::atomic::{AtomicBool, Ordering};
use tracing_appender::rolling;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use super::paths;

// Protection contre la double initialisation (ex: tests, rechargement Tauri)
static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn init_logger() {
    // swap retourne l'ancienne valeur : si déjà true, on sort immédiatement
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return; // Already initialized
    }

    let logs_dir = paths::logs_dir();

    let file_appender = rolling::daily(&logs_dir, "nitrite.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Leak le guard pour qu'il vive toute la duree du programme
    std::mem::forget(_guard);

    tracing_subscriber::registry()
        .with(EnvFilter::new("info,nitrite_lib=debug,tao=error,wry=error"))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .with(fmt::layer().with_writer(std::io::stdout))
        .init();

    tracing::info!("NiTriTe V26.0 — Logger initialise");
}
