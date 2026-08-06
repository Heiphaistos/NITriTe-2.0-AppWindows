/// wmi.rs — Helper anti-freeze pour les requêtes WMI/blocking
///
/// `wmi_timeout` wraps une closure bloquante dans spawn_blocking + tokio::time::timeout.
/// Toute commande WMI doit passer par ce wrapper pour éviter les freezes
/// causés par des appels WMI qui ne répondent pas (drivers corrompus, services morts, etc.)
use crate::error::NiTriTeError;

/// Timeout global pour les opérations WMI bloquantes (30 secondes).
pub const WMI_TIMEOUT_SECS: u64 = 30;

/// Exécute `f` dans un thread bloquant avec timeout de 30s.
/// Retourne `NiTriTeError::System("WMI timeout")` si dépassé.
///
/// ATTENTION : `tokio::time::timeout` n'annule PAS le thread bloquant sous-jacent —
/// il abandonne seulement l'attente côté appelant. Un appel WMI réellement figé
/// (driver corrompu, service mort) continue de tourner indéfiniment sur son thread
/// du pool blocking de Tokio, orphelin, sans jamais être récupéré (prouvé par le
/// test `timeout_does_not_cancel_the_blocking_task` ci-dessous). Ce n'est pas
/// contournable proprement : un appel WMI/COM bloquant ne peut pas être interrompu
/// depuis l'extérieur comme un process externe (contrairement à
/// `execute_system_command` qui peut faire un `taskkill /F` sur le PID enfant).
pub async fn wmi_timeout<T, F>(f: F) -> Result<T, NiTriTeError>
where
    F: FnOnce() -> Result<T, NiTriTeError> + Send + 'static,
    T: Send + 'static,
{
    tokio::time::timeout(
        std::time::Duration::from_secs(WMI_TIMEOUT_SECS),
        tokio::task::spawn_blocking(f),
    )
    .await
    .map_err(|_| {
        tracing::warn!(
            "wmi_timeout: timeout {}s dépassé — l'appel WMI sous-jacent continue de tourner en arrière-plan (thread orphelin, non annulable)",
            WMI_TIMEOUT_SECS
        );
        NiTriTeError::System(format!("WMI timeout ({}s)", WMI_TIMEOUT_SECS))
    })?
    .map_err(|e| NiTriTeError::System(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// Prouve que `tokio::time::timeout` autour de `spawn_blocking` n'annule pas le
    /// thread bloquant sous-jacent : le travail continue et se termine APRÈS que le
    /// timeout ait déjà renvoyé une erreur à l'appelant — ce n'est donc jamais un
    /// vrai "abandon", juste un abandon de l'attente côté appelant. Justifie le
    /// commentaire d'avertissement ajouté sur wmi_timeout.
    #[tokio::test]
    async fn timeout_does_not_cancel_the_blocking_task() {
        let completed = Arc::new(AtomicBool::new(false));
        let completed2 = completed.clone();
        let handle = tokio::task::spawn_blocking(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            completed2.store(true, Ordering::SeqCst);
        });

        let result = tokio::time::timeout(std::time::Duration::from_millis(50), handle).await;
        assert!(result.is_err(), "doit timeout avant la fin du sleep de 300ms");
        assert!(!completed.load(Ordering::SeqCst), "pas encore terminé au moment du timeout");

        // Laisse le temps au sleep de 300ms de se terminer réellement, alors que
        // l'appelant a déjà reçu son erreur de timeout depuis longtemps.
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        assert!(
            completed.load(Ordering::SeqCst),
            "le thread bloquant a continué de tourner après le timeout côté appelant — preuve qu'il n'est jamais annulé"
        );
    }
}
