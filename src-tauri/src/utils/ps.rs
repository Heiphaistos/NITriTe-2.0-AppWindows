#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Lance un script PowerShell sans fenêtre CMD visible.
///
/// Décode via `decode_output` (UTF-8 d'abord, repli codepage OEM) plutôt que
/// `from_utf8_lossy` : de nombreux scripts émettent du texte FR accentué
/// (« Activé », « non trouvée »…) que PowerShell encode en OEM (CP850 FR) faute
/// de `$OutputEncoding` — from_utf8_lossy le transformait en mojibake. Les
/// scripts déjà UTF-8/JSON restent inchangés (fast path UTF-8, aucune régression).
pub fn ps(script: &str) -> Result<String, String> {
    ps_with_timeout(script, 10)
}

/// Comme `ps()`, mais avec un timeout explicite en secondes. `.output()` seul
/// (comportement d'origine) n'a aucune limite — un script PowerShell figé
/// (namespace WMI tiers non répondant, ex: LibreHardwareMonitor/OpenHardwareMonitor
/// crashé mais toujours "enregistré") bloquait le thread appelant indéfiniment.
/// Ce helper est partagé par `sensors.rs` (interrogé toutes les 3s par
/// TemperaturesPage.vue tant que la page reste ouverte — un utilisateur qui
/// surveille les températures pendant un stress test verrait l'affichage se
/// figer silencieusement, sans le moindre message d'erreur, exactement quand
/// cette information compte le plus) et `dll_scanner.rs`. Même pattern
/// timeout+taskkill que `execute_system_command`/`check_command_with_timeout`.
pub fn ps_with_timeout(script: &str, timeout_secs: u64) -> Result<String, String> {
    let child = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| e.to_string())?;
    let pid = child.id();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    let out = match rx.recv_timeout(std::time::Duration::from_secs(timeout_secs)) {
        Ok(result) => result.map_err(|e| e.to_string())?,
        Err(_) => {
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .creation_flags(0x08000000)
                .spawn();
            tracing::warn!("ps(): timeout {}s dépassé — script PowerShell tué de force", timeout_secs);
            return Err(format!("PowerShell timeout ({}s)", timeout_secs));
        }
    };
    #[cfg(target_os = "windows")]
    { Ok(crate::maintenance::commands::decode_output(&out.stdout).trim().to_string()) }
    #[cfg(not(target_os = "windows"))]
    { Ok(String::from_utf8_lossy(&out.stdout).trim().to_string()) }
}

/// Préambule PowerShell définissant la fonction `Loc-Counter` : traduit un nom
/// de compteur de performance ANGLAIS vers son libellé LOCALISÉ via l'index
/// perflib du registre.
///
/// Indispensable car `Get-Counter` n'accepte que des chemins localisés : les
/// chemins anglais codés en dur (`\GPU Engine(*)\Utilization Percentage`)
/// échouent silencieusement sur Windows non-anglophone (FR : « Moteur GPU »,
/// « Pourcentage d'utilisation »), renvoyant 0 partout. Sur Windows anglais la
/// traduction renvoie le nom d'origine (aucune régression) ; si le registre est
/// illisible on retombe aussi sur le nom d'origine.
///
/// Usage : préfixer le script avec ce préambule, puis construire le chemin avec
/// `"\{0}(*)\{1}" -f (Loc-Counter 'GPU Engine'), (Loc-Counter 'Utilization Percentage')`.
pub const LOC_COUNTER_PRELUDE: &str = r#"
$__pe = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Perflib\009' -EA SilentlyContinue).Counter
$__pl = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Perflib\CurrentLanguage' -EA SilentlyContinue).Counter
function Loc-Counter($n) {
    if (-not $__pe -or -not $__pl) { return $n }
    for ($i = 1; $i -lt $__pe.Count; $i += 2) {
        if ($__pe[$i] -ieq $n) {
            $id = $__pe[$i-1]
            for ($j = 0; $j -lt ($__pl.Count - 1); $j += 2) {
                if ($__pl[$j] -eq $id) { return $__pl[$j+1] }
            }
            return $n
        }
    }
    return $n
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ps_with_timeout_kills_hung_script() {
        let start = std::time::Instant::now();
        let result = ps_with_timeout("Start-Sleep -Seconds 30", 2);
        let elapsed = start.elapsed();

        assert!(result.is_err(), "un script encore en cours après le timeout doit renvoyer une erreur, pas un faux succès silencieux");
        assert!(
            elapsed.as_secs() < 10,
            "ps_with_timeout a attendu {}s, le timeout+taskkill ne fonctionne pas",
            elapsed.as_secs()
        );
    }

    #[test]
    fn ps_with_timeout_returns_output_for_quick_script() {
        let result = ps_with_timeout("Write-Output 'hello'", 10);
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn ps_default_timeout_still_works_for_quick_script() {
        let result = ps("Write-Output 'ok'");
        assert_eq!(result.unwrap(), "ok");
    }
}
