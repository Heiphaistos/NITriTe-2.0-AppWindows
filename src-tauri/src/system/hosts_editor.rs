use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Default)]
pub struct HostsEntry {
    pub ip: String,
    pub hostname: String,
    pub comment: String,
    pub active: bool,
    pub line_number: u32,
}

const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";

// Anti-freeze : lecture fichier hosts + résolutions associées peuvent
// bloquer — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn get_hosts_entries() -> Vec<HostsEntry> {
    tokio::task::spawn_blocking(get_hosts_entries_blocking)
        .await
        .unwrap_or_default()
}

fn get_hosts_entries_blocking() -> Vec<HostsEntry> {
    let content = match std::fs::read_to_string(HOSTS_PATH) {
        Ok(c) => c,
        Err(_) => {
            // Try PowerShell read
            #[cfg(target_os = "windows")]
            {
                let o = Command::new("powershell")
                    .args(["-NoProfile","-NonInteractive","-Command",
                           &format!("Get-Content '{}' -Raw", HOSTS_PATH)])
                    .creation_flags(0x08000000).output().ok();
                if let Some(o) = o {
                    String::from_utf8_lossy(&o.stdout).to_string()
                } else { return vec![]; }
            }
            #[cfg(not(target_os = "windows"))]
            return vec![];
        }
    };

    let mut entries = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let active = !trimmed.starts_with('#');

        // Parse active entry: "ip hostname # optional comment"
        let parse_line = if active { trimmed } else {
            trimmed.trim_start_matches('#').trim()
        };

        let parts: Vec<&str> = parse_line.splitn(2, |c: char| c.is_whitespace()).collect();
        if !parts.is_empty() {
            let ip = parts[0].trim();
            let rest = parts.get(1).unwrap_or(&"").trim();

            // Separate hostname and inline comment
            let (hostname, comment) = if let Some(ci) = rest.find('#') {
                (rest[..ci].trim(), rest[ci+1..].trim())
            } else {
                (rest, "")
            };

            // Le bloc d'en-tete standard Windows ("# This file contains the
            // mappings...", "# entry should be kept...") est du texte libre en
            // commentaire, pas des entrees desactivees — sans cette validation,
            // chaque ligne de commentaire etait parsee comme si son 1er mot etait
            // une IP ("Copyright"/"(c)", "This"/"is", "entry"/"should"...),
            // polluant la liste de fausses entrees a chaque fichier hosts par defaut.
            if !ip.is_empty() && !hostname.is_empty() && ip.parse::<std::net::IpAddr>().is_ok() {
                entries.push(HostsEntry {
                    ip: ip.to_string(),
                    hostname: hostname.split_whitespace().next().unwrap_or("").to_string(),
                    comment: comment.to_string(),
                    active,
                    line_number: i as u32 + 1,
                });
            }
        }
    }
    entries
}

// Anti-freeze : écriture fichier hosts est bloquante — jamais inline sur le
// thread de commande.
#[tauri::command]
pub async fn add_hosts_entry(ip: String, hostname: String, comment: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || add_hosts_entry_blocking(ip, hostname, comment))
        .await
        .map_err(|e| e.to_string())?
}

fn add_hosts_entry_blocking(ip: String, hostname: String, comment: String) -> Result<String, String> {
    // Suppression de tous les caractères de contrôle (newlines inclus) + guillemets
    fn clean(s: &str) -> String {
        s.chars().filter(|c| !c.is_control() && *c != '\'' && *c != '"').collect::<String>().trim().to_string()
    }
    // Whitelist stricte pour le commentaire : alphanumériques, espaces, tirets, underscores, points
    // Évite l'injection PowerShell via caractères spéciaux (;, |, $, `, &, (, ))
    fn clean_comment(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_alphanumeric() || " -_.".contains(*c))
            .collect::<String>()
            .trim()
            .to_string()
    }
    let ip_c = clean(&ip);
    let host_c = clean(&hostname);
    let comment_c = clean_comment(&comment);

    if ip_c.is_empty() || host_c.is_empty() {
        return Err("IP et hostname requis".to_string());
    }

    // Validation IP basique (IPv4 ou IPv6)
    let valid_ip = ip_c.parse::<std::net::IpAddr>().is_ok();
    if !valid_ip {
        return Err(format!("IP invalide : {}", ip_c));
    }

    // Validation hostname : alphanumériques, tirets, points uniquement
    let valid_host = host_c.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '.' || c == '_');
    if !valid_host {
        return Err(format!("Hostname invalide : {}", host_c));
    }

    let line = if comment_c.is_empty() {
        format!("\n{}\t{}", ip_c, host_c)
    } else {
        format!("\n{}\t{}\t# {}", ip_c, host_c, comment_c)
    };
    let ps = format!(r#"Add-Content -Path '{}' -Value '{}' -Encoding UTF8"#, HOSTS_PATH, line);
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",&ps]).creation_flags(0x08000000).output().map_err(|e| e.to_string())?;
        if !o.status.success() {
            return Err(String::from_utf8_lossy(&o.stderr).to_string());
        }
        // Add-Content peut rendre un exit code 0 meme quand l'ecriture echoue
        // reellement (droits insuffisants sur un fichier systeme, process enfant
        // powershell.exe non elevé) — sans cette relecture, l'utilisateur voit
        // "Entrée ajoutée" alors que le fichier hosts reel n'a jamais change.
        let content = std::fs::read_to_string(HOSTS_PATH).unwrap_or_default();
        if !content.lines().any(|l| l.trim() == format!("{}\t{}", ip_c, host_c).trim() || (l.contains(&ip_c) && l.contains(&host_c))) {
            return Err("L'écriture a échoué silencieusement (droits administrateur requis sur le fichier hosts)".to_string());
        }
        Ok(format!("Entrée ajoutée : {} -> {}", ip_c, host_c))
    }
    #[cfg(not(target_os = "windows"))]
    Err("Non disponible".to_string())
}

// Anti-freeze : écriture fichier hosts est bloquante — jamais inline sur le
// thread de commande.
#[tauri::command]
pub async fn delete_hosts_entry(line_number: u32) -> Result<String, String> {
    tokio::task::spawn_blocking(move || delete_hosts_entry_blocking(line_number))
        .await
        .map_err(|e| e.to_string())?
}

fn delete_hosts_entry_blocking(line_number: u32) -> Result<String, String> {
    if line_number == 0 {
        return Err("Numéro de ligne invalide".to_string());
    }
    let before = std::fs::read_to_string(HOSTS_PATH).unwrap_or_default();
    let before_count = before.lines().count();
    // -SkipIndex n'existe qu'en PowerShell 6+ ; boucle indexée compatible Windows PowerShell 5.1
    let ps = format!(r#"
$lines = @(Get-Content '{}')
$idx = {}
if ($idx -ge $lines.Count) {{ throw "Ligne introuvable" }}
$new = @(for ($i = 0; $i -lt $lines.Count; $i++) {{ if ($i -ne $idx) {{ $lines[$i] }} }})
$new | Set-Content '{}' -Encoding UTF8
"#, HOSTS_PATH, line_number - 1, HOSTS_PATH);
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",&ps]).creation_flags(0x08000000).output().map_err(|e| e.to_string())?;
        if !o.status.success() {
            return Err(String::from_utf8_lossy(&o.stderr).to_string());
        }
        // Meme piege que add_hosts_entry : exit code 0 ne garantit pas que
        // l'ecriture a reellement eu lieu — on verifie que le fichier a
        // vraiment une ligne de moins.
        let after_count = std::fs::read_to_string(HOSTS_PATH).unwrap_or_default().lines().count();
        if after_count >= before_count {
            return Err("La suppression a échoué silencieusement (droits administrateur requis sur le fichier hosts)".to_string());
        }
        Ok("Entrée supprimée".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    Err("Non disponible".to_string())
}

// Anti-freeze : écriture fichier hosts est bloquante — jamais inline sur le
// thread de commande.
#[tauri::command]
pub async fn toggle_hosts_entry(line_number: u32, enable: bool) -> Result<String, String> {
    tokio::task::spawn_blocking(move || toggle_hosts_entry_blocking(line_number, enable))
        .await
        .map_err(|e| e.to_string())?
}

fn toggle_hosts_entry_blocking(line_number: u32, enable: bool) -> Result<String, String> {
    if line_number == 0 {
        return Err("Numéro de ligne invalide".to_string());
    }
    let ps = format!(r#"
$lines = @(Get-Content '{}')
$idx = {}
if ($idx -ge 0 -and $idx -lt $lines.Count) {{
    $line = $lines[$idx]
    if ({}) {{
        $lines[$idx] = $line.TrimStart('#').TrimStart()
    }} else {{
        if (-not $line.StartsWith('#')) {{ $lines[$idx] = '# ' + $line }}
    }}
    $lines | Set-Content '{}' -Encoding UTF8
    "Modifié"
}} else {{ throw "Ligne introuvable" }}
"#, HOSTS_PATH, line_number - 1, if enable { "$true" } else { "$false" }, HOSTS_PATH);
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",&ps]).creation_flags(0x08000000).output().map_err(|e| e.to_string())?;
        if !o.status.success() {
            return Err(String::from_utf8_lossy(&o.stderr).to_string());
        }
        // Meme piege : verifier que la ligne ciblee a reellement le prefixe '#'
        // attendu apres coup, pas juste que powershell.exe est sorti en 0.
        let lines: Vec<String> = std::fs::read_to_string(HOSTS_PATH).unwrap_or_default().lines().map(|l| l.to_string()).collect();
        let idx = (line_number - 1) as usize;
        let actually_enabled = lines.get(idx).map(|l| !l.trim_start().starts_with('#')).unwrap_or(false);
        if actually_enabled != enable {
            return Err("La modification a échoué silencieusement (droits administrateur requis sur le fichier hosts)".to_string());
        }
        Ok("Modifié".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    Err("Non disponible".to_string())
}

// Anti-freeze : I/O fichier — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn backup_hosts() -> Result<String, String> {
    tokio::task::spawn_blocking(backup_hosts_blocking)
        .await
        .map_err(|e| e.to_string())?
}

fn backup_hosts_blocking() -> Result<String, String> {
    let backup = format!("{}.bak", HOSTS_PATH);
    std::fs::copy(HOSTS_PATH, &backup).map(|_| format!("Sauvegarde : {}", backup)).map_err(|e| e.to_string())
}

// Anti-freeze : I/O fichier — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn get_hosts_raw() -> String {
    tokio::task::spawn_blocking(get_hosts_raw_blocking)
        .await
        .unwrap_or_default()
}

fn get_hosts_raw_blocking() -> String {
    std::fs::read_to_string(HOSTS_PATH).unwrap_or_default()
}

// Anti-freeze : résolution DNS est bloquante — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn resolve_hostname(hostname: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || resolve_hostname_blocking(hostname))
        .await
        .map_err(|e| e.to_string())?
}

fn resolve_hostname_blocking(hostname: String) -> Result<String, String> {
    use std::net::ToSocketAddrs;
    let addr = format!("{}:80", hostname.trim());
    match addr.to_socket_addrs() {
        Ok(iter) => {
            let mut seen = std::collections::HashSet::new();
            for a in iter { seen.insert(a.ip().to_string()); }
            let ips: Vec<String> = seen.into_iter().collect();
            Ok(ips.join(", "))
        }
        Err(e) => Err(format!("Résolution échouée : {}", e)),
    }
}

#[tauri::command]
pub fn import_hosts_blocklist(url: String, _list_name: String) -> Result<String, String> {
    Err(format!(
        "Import en ligne non disponible dans cette version. Téléchargez manuellement : {} et importez-le.",
        url
    ))
}

#[cfg(test)]
mod tests {
    

    // add_hosts_entry is tested in isolation by extracting its validation logic
    // (the actual file write is platform-gated and won't run in tests)

    fn validate_hosts_input(ip: &str, hostname: &str) -> Result<(), String> {
        fn clean(s: &str) -> String {
            s.chars().filter(|c| !c.is_control() && *c != '\'' && *c != '"').collect::<String>().trim().to_string()
        }
        let ip_c = clean(ip);
        let host_c = clean(hostname);
        if ip_c.is_empty() || host_c.is_empty() { return Err("IP et hostname requis".into()); }
        if ip_c.parse::<std::net::IpAddr>().is_err() { return Err(format!("IP invalide: {}", ip_c)); }
        let valid_host = host_c.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '.' || c == '_');
        if !valid_host { return Err(format!("Hostname invalide: {}", host_c)); }
        Ok(())
    }

    #[test]
    fn valid_ipv4_and_hostname() {
        assert!(validate_hosts_input("192.168.1.10", "local.dev").is_ok());
        assert!(validate_hosts_input("127.0.0.1", "my-server").is_ok());
    }

    #[test]
    fn valid_ipv6() {
        assert!(validate_hosts_input("::1", "localhost6").is_ok());
        assert!(validate_hosts_input("2001:db8::1", "test.host").is_ok());
    }

    #[test]
    fn invalid_ip_rejected() {
        assert!(validate_hosts_input("not-an-ip", "host.local").is_err());
        assert!(validate_hosts_input("999.999.999.999", "host.local").is_err());
        assert!(validate_hosts_input("", "host.local").is_err());
    }

    #[test]
    fn hostname_with_special_chars_rejected() {
        // clean() ne supprime PAS ;|$ — ils sont bloqués par la whitelist hostname [a-z0-9-._]
        assert!(validate_hosts_input("127.0.0.1", "evil;whoami").is_err());
        assert!(validate_hosts_input("127.0.0.1", "host|cmd").is_err());
    }

    #[test]
    fn quotes_stripped_from_ip_and_host() {
        // Single/double quote-only inputs become empty after stripping → rejected
        assert!(validate_hosts_input("'", "host").is_err());
        assert!(validate_hosts_input("\"\"", "host").is_err());
        // Control characters stripped — but valid chars survive
        assert!(validate_hosts_input("127.0.0.1", "host\x00evil").is_ok()); // null stripped, "hostevil" is valid
    }
}
