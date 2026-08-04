use serde::Serialize;
use crate::utils::ps::ps;

#[derive(Serialize, Clone)]
pub struct DllEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub company: String,
    pub description: String,
    pub version: String,
    pub location: String,  // "System32" | "SysWOW64" | "ProgramFiles"
    pub category: String,  // "Système" | "Tiers" | "Pilote"
}

fn dll_category(company: &str, location: &str) -> String {
    let c = company.to_lowercase();
    if c.contains("microsoft") || c.contains("windows") {
        "Système".to_string()
    } else if location == "System32" || location == "SysWOW64" {
        // Une DLL non-Microsoft dans System32/SysWOW64 est ajoutée par un pilote/app tierce
        "Tiers (System32)".to_string()
    } else {
        "Application".to_string()
    }
}

fn scan_dlls_sync() -> Vec<DllEntry> {
    // On scanne System32 + SysWOW64 pour les DLLs tierces (non-Microsoft)
    // + ProgramFiles pour les DLLs racines d'applications
    let script = r#"
$results = @()
$sys = [System.Environment]::SystemDirectory
$sys32 = $sys
$sys64 = if (Test-Path 'C:\Windows\SysWOW64') { 'C:\Windows\SysWOW64' } else { $null }

function Scan-Dir($dir, $loc) {
    Get-ChildItem "$dir\*.dll" -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            $vi = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($_.FullName)
            $company = if ($vi.CompanyName) { $vi.CompanyName.Trim() } else { '' }
            $results += [PSCustomObject]@{
                name        = $_.Name
                path        = $_.FullName
                size        = $_.Length
                company     = $company
                description = if ($vi.FileDescription) { $vi.FileDescription.Trim() } else { '' }
                version     = if ($vi.FileVersion) { $vi.FileVersion.Trim() } else { '' }
                location    = $loc
            }
        } catch {}
    }
}

Scan-Dir $sys32 'System32'
if ($sys64) { Scan-Dir $sys64 'SysWOW64' }

# DLLs dans ProgramFiles (racine seulement, pas récursif — trop lent)
$pfPaths = @('C:\Program Files','C:\Program Files (x86)')
foreach ($pf in $pfPaths) {
    if (-not (Test-Path $pf)) { continue }
    Get-ChildItem $pf -ErrorAction SilentlyContinue | ForEach-Object {
        Get-ChildItem "$($_.FullName)\*.dll" -ErrorAction SilentlyContinue | Select-Object -First 30 | ForEach-Object {
            try {
                $vi = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($_.FullName)
                $company = if ($vi.CompanyName) { $vi.CompanyName.Trim() } else { '' }
                $results += [PSCustomObject]@{
                    name        = $_.Name
                    path        = $_.FullName
                    size        = $_.Length
                    company     = $company
                    description = if ($vi.FileDescription) { $vi.FileDescription.Trim() } else { '' }
                    version     = if ($vi.FileVersion) { $vi.FileVersion.Trim() } else { '' }
                    location    = 'ProgramFiles'
                }
            } catch {}
        }
    }
}

@($results) | ConvertTo-Json -Compress -Depth 2
"#;

    let out = ps(script).unwrap_or_default();
    if out.trim().is_empty() || out.trim() == "null" {
        return vec![];
    }
    let arr: Vec<serde_json::Value> = match serde_json::from_str(out.trim()) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    arr.iter().filter_map(|v| {
        let name     = v["name"].as_str().unwrap_or("").to_string();
        let path     = v["path"].as_str().unwrap_or("").to_string();
        let company  = v["company"].as_str().unwrap_or("").to_string();
        let location = v["location"].as_str().unwrap_or("").to_string();
        if name.is_empty() || path.is_empty() { return None; }
        let category = dll_category(&company, &location);
        Some(DllEntry {
            name,
            path,
            size:        v["size"].as_u64().unwrap_or(0),
            company,
            description: v["description"].as_str().unwrap_or("").to_string(),
            version:     v["version"].as_str().unwrap_or("").to_string(),
            location,
            category,
        })
    }).collect()
}

#[tauri::command]
pub async fn scan_dlls() -> Result<Vec<DllEntry>, String> {
    tokio::task::spawn_blocking(scan_dlls_sync)
        .await
        .map_err(|e| e.to_string())
}

/// Vérifie qu'un chemin canonique appartient à System32/SysWOW64/Program Files.
/// Comparaison sur la représentation texte (pas Path::starts_with) : sur Windows,
/// canonicalize() retourne le préfixe verbatim étendu `\\?\C:\...`
/// (Prefix::VerbatimDisk), qui n'est JAMAIS égal au composant `Prefix::Disk`
/// d'un chemin `C:\...` classique pour Path::starts_with — la whitelist
/// rejetait donc TOUJOURS toute suppression, y compris les DLL légitimes.
fn is_dll_path_allowed(canonical: &std::path::Path) -> bool {
    let raw = canonical.to_string_lossy().to_lowercase().replace('/', "\\");
    let path_lower = raw.strip_prefix(r"\\?\").unwrap_or(&raw);
    let allowed = [
        r"c:\windows\system32",
        r"c:\windows\syswow64",
        r"c:\program files",
        r"c:\program files (x86)",
    ];
    allowed.iter().any(|base| path_lower.starts_with(base))
}

#[tauri::command]
pub async fn delete_dll(path: String) -> Result<(), String> {
    // Sécurité : autoriser uniquement les .dll dans System32/SysWOW64/Program Files
    let p = std::path::Path::new(&path);
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if ext != "dll" {
        return Err("Suppression refusée : extension non autorisée.".to_string());
    }
    let canonical = p.canonicalize().map_err(|e| e.to_string())?;
    if !is_dll_path_allowed(&canonical) {
        return Err("Suppression refusée : chemin hors des répertoires autorisés.".to_string());
    }
    tokio::task::spawn_blocking(move || {
        std::fs::remove_file(&canonical)
            .map_err(|e| format!("Impossible de supprimer : {}", e))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn microsoft_company_is_system() {
        assert_eq!(dll_category("Microsoft Corporation", "System32"), "Système");
        assert_eq!(dll_category("Microsoft Windows", "SysWOW64"), "Système");
    }

    #[test]
    fn windows_in_company_is_system() {
        assert_eq!(dll_category("Windows Components", "anywhere"), "Système");
    }

    #[test]
    fn third_party_in_system32_is_tiers() {
        assert_eq!(dll_category("NVidia Corporation", "System32"), "Tiers (System32)");
        assert_eq!(dll_category("Intel Inc", "SysWOW64"), "Tiers (System32)");
    }

    #[test]
    fn third_party_elsewhere_is_application() {
        assert_eq!(dll_category("Adobe Systems", "ProgramFiles"), "Application");
        assert_eq!(dll_category("Unknown Vendor", "SomeOtherLocation"), "Application");
    }

    #[test]
    fn empty_company_in_system32_is_tiers() {
        assert_eq!(dll_category("", "System32"), "Tiers (System32)");
    }

    // ── is_dll_path_allowed ────────────────────────────────────────────────────
    // Régression : canonicalize() renvoie le préfixe verbatim `\\?\` sur Windows,
    // que Path::starts_with ne reconnaît jamais comme préfixe `C:\...` classique.
    // Sans le fix, TOUTES ces assertions "allowed" échoueraient (delete_dll
    // refusait 100% des suppressions, y compris légitimes).

    #[test]
    fn verbatim_system32_path_is_allowed() {
        let p = std::path::Path::new(r"\\?\C:\Windows\System32\example.dll");
        assert!(is_dll_path_allowed(p));
    }

    #[test]
    fn verbatim_syswow64_path_is_allowed() {
        let p = std::path::Path::new(r"\\?\C:\Windows\SysWOW64\example.dll");
        assert!(is_dll_path_allowed(p));
    }

    #[test]
    fn verbatim_program_files_path_is_allowed() {
        let p = std::path::Path::new(r"\\?\C:\Program Files\SomeApp\example.dll");
        assert!(is_dll_path_allowed(p));
    }

    #[test]
    fn verbatim_path_outside_whitelist_is_rejected() {
        let p = std::path::Path::new(r"\\?\C:\Users\Momo\Documents\example.dll");
        assert!(!is_dll_path_allowed(p));
    }

    #[test]
    fn non_verbatim_path_still_works() {
        // Comportement conservé même sans préfixe verbatim (ex: appel direct hors canonicalize()).
        assert!(is_dll_path_allowed(std::path::Path::new(r"C:\Windows\System32\example.dll")));
        assert!(!is_dll_path_allowed(std::path::Path::new(r"C:\Users\Momo\example.dll")));
    }

    #[test]
    fn empty_company_elsewhere_is_application() {
        assert_eq!(dll_category("", "ProgramFiles"), "Application");
    }
}
