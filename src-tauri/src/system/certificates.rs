use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Default)]
pub struct CertEntry {
    pub subject: String,
    pub issuer: String,
    pub thumbprint: String,
    pub not_before: String,
    pub not_after: String,
    pub store: String,
    pub is_expired: bool,
    pub has_private_key: bool,
}

#[derive(Debug, Default, Serialize)]
pub struct CertsData {
    pub certs: Vec<CertEntry>,
    pub total: u32,
    pub expired_count: u32,
    pub expiring_soon_count: u32,
}

// Anti-freeze : PowerShell est bloquant — jamais inline sur le thread de commande.
#[tauri::command]
pub async fn get_certificates() -> CertsData {
    tokio::task::spawn_blocking(get_certificates_blocking)
        .await
        .unwrap_or_default()
}

fn get_certificates_blocking() -> CertsData {
    let ps = r#"
# Sortie UTF-8 : Subject/Issuer de certificats peuvent contenir des accents
# (organisations FR : « O=Société … ») → sinon mojibake côté Rust.
$OutputEncoding = [System.Text.Encoding]::UTF8; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$out = @{}
$all = @()
$stores = @(
    @{name='LocalMachine\My';      label='Machine\Perso'},
    @{name='LocalMachine\Root';    label='Machine\Racine'},
    @{name='LocalMachine\CA';      label='Machine\CA'},
    @{name='LocalMachine\TrustedPublisher'; label='Machine\Editeurs'},
    @{name='CurrentUser\My';       label='User\Perso'},
    @{name='CurrentUser\Root';     label='User\Racine'}
)
$now = Get-Date
$soon = $now.AddDays(30)
foreach ($s in $stores) {
    try {
        $store = [System.Security.Cryptography.X509Certificates.X509Store]::new($s.name.Split('\')[1], $s.name.Split('\')[0])
        $store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadOnly)
        foreach ($c in $store.Certificates) {
            $exp  = $c.NotAfter -lt $now
            $pk   = $c.HasPrivateKey
            $all += @{
                subject   = [string]$c.Subject
                issuer    = [string]$c.Issuer
                thumb     = [string]$c.Thumbprint
                before    = $c.NotBefore.ToString('yyyy-MM-dd')
                after     = $c.NotAfter.ToString('yyyy-MM-dd')
                store     = [string]$s.label
                expired   = [bool]$exp
                pk        = [bool]$pk
            }
        }
        $store.Close()
    } catch {}
}
$out.Certs       = $all
$out.Total       = [int]$all.Count
$out.ExpiredCt   = [int]($all | Where-Object { $_['expired'] }).Count
$out.ExpireSoon  = [int]($all | Where-Object { -not $_['expired'] -and [datetime]$_['after'] -lt $soon }).Count
$out | ConvertTo-Json -Depth 4 -Compress
"#;

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .creation_flags(0x08000000)
            .output();

        if let Ok(o) = output {
            let text = String::from_utf8_lossy(&o.stdout);
            let v: serde_json::Value = match serde_json::from_str(text.trim()) {
                Ok(v) => v,
                Err(_) => return CertsData::default(),
            };

            let certs = v["Certs"].as_array().map(|arr| {
                arr.iter().map(|c| CertEntry {
                    subject: c["subject"].as_str().unwrap_or("").to_string(),
                    issuer: c["issuer"].as_str().unwrap_or("").to_string(),
                    thumbprint: c["thumb"].as_str().unwrap_or("").to_string(),
                    not_before: c["before"].as_str().unwrap_or("").to_string(),
                    not_after: c["after"].as_str().unwrap_or("").to_string(),
                    store: c["store"].as_str().unwrap_or("").to_string(),
                    is_expired: c["expired"].as_bool().unwrap_or(false),
                    has_private_key: c["pk"].as_bool().unwrap_or(false),
                }).collect()
            }).unwrap_or_default();

            return CertsData {
                total: v["Total"].as_u64().unwrap_or(0) as u32,
                expired_count: v["ExpiredCt"].as_u64().unwrap_or(0) as u32,
                expiring_soon_count: v["ExpireSoon"].as_u64().unwrap_or(0) as u32,
                certs,
            };
        }
    }
    CertsData::default()
}
