use serde::Serialize;
use std::process::Command;
use tauri::Emitter;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// ─── Événements streaming ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct CleanerProgress {
    pub scanned: u8,
    pub total: u8,
    pub item: Option<CleanTarget>,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CleanTarget {
    pub name: String,
    pub path: String,
    pub size_mb: f64,
    pub file_count: u32,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CleanResult {
    pub target: String,
    pub freed_mb: f64,
    pub files_deleted: u32,
    pub success: bool,
    pub message: String,
}

fn ps_run(ps: &str) -> Option<serde_json::Value> {
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",ps]).creation_flags(0x08000000).output().ok()?;
        let t = String::from_utf8_lossy(&o.stdout);
        serde_json::from_str(t.trim()).ok()
    }
    #[cfg(not(target_os = "windows"))]
    None
}

#[tauri::command]
pub fn get_clean_targets() -> Vec<CleanTarget> {
    let ps = r#"
$items = @(
    @{ name='%TEMP%'; path=$env:TEMP; cat='Temp' },
    @{ name='Windows\Temp'; path='C:\Windows\Temp'; cat='Temp' },
    @{ name='Prefetch'; path='C:\Windows\Prefetch'; cat='Système' },
    @{ name='Dumps mémoire'; path='C:\Windows\Minidump'; cat='Système' },
    @{ name='Logs CBS'; path='C:\Windows\Logs\CBS'; cat='Logs' },
    @{ name='Windows Error Reports'; path="$env:LOCALAPPDATA\Microsoft\Windows\WER\ReportArchive"; cat='Logs' },
    @{ name='Corbeille'; path=''; cat='Corbeille' },
    @{ name='Chrome Cache'; path="$env:LOCALAPPDATA\Google\Chrome\User Data\Default\Cache"; cat='Navigateurs' },
    @{ name='Edge Cache'; path="$env:LOCALAPPDATA\Microsoft\Edge\User Data\Default\Cache"; cat='Navigateurs' },
    @{ name='Firefox Cache'; path="$env:APPDATA\Mozilla\Firefox\Profiles"; cat='Navigateurs' },
    @{ name='Windows Update Cache'; path='C:\Windows\SoftwareDistribution\Download'; cat='Windows Update' },
    @{ name='Thumbnails DB'; path="$env:LOCALAPPDATA\Microsoft\Windows\Explorer"; cat='Cache système' }
)

@($items | ForEach-Object {
    $p = $_.path
    $sz = 0.0; $cnt = 0
    if ($p -and (Test-Path $p)) {
        try {
            $files = @(Get-ChildItem $p -Recurse -File -EA SilentlyContinue)
            $cnt = $files.Count
            $bytes = ($files | Measure-Object -Property Length -Sum -EA SilentlyContinue).Sum
            $sz = if($bytes){[math]::Round($bytes/1MB,2)}else{0}
        } catch {}
    } elseif ($_.cat -eq 'Corbeille') {
        try {
            $shell = New-Object -ComObject Shell.Application
            $rb = $shell.Namespace(0xA)
            $cnt = @($rb.Items()).Count
            $sz = 0.1 * $cnt
        } catch {}
    }
    @{ name=$_.name; path=$p; mb=$sz; count=$cnt; cat=$_.cat }
}) | ConvertTo-Json -Compress
"#;
    if let Some(v) = ps_run(ps) {
        let arr = match v.as_array() {
            Some(a) => a.clone(),
            None => if v.is_object() { vec![v] } else { vec![] }
        };
        return arr.iter().map(|r| CleanTarget {
            name: r["name"].as_str().unwrap_or("").to_string(),
            path: r["path"].as_str().unwrap_or("").to_string(),
            size_mb: r["mb"].as_f64().unwrap_or(0.0),
            file_count: r["count"].as_u64().unwrap_or(0) as u32,
            category: r["cat"].as_str().unwrap_or("").to_string(),
        }).collect();
    }
    vec![]
}

/// Version streaming : émet un événement `cleaner:progress` par cible.
/// Permet à l'UI de rester réactive pendant le scan.
#[tauri::command]
pub async fn scan_clean_targets_stream(app: tauri::AppHandle) {
    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        let targets: &[(&str, &str, &str)] = &[
            ("%TEMP%",               "",                                                                   "Temp"),
            ("Windows\\Temp",        "C:\\Windows\\Temp",                                                  "Temp"),
            ("Prefetch",             "C:\\Windows\\Prefetch",                                              "Système"),
            ("Dumps mémoire",        "C:\\Windows\\Minidump",                                             "Système"),
            ("Logs CBS",             "C:\\Windows\\Logs\\CBS",                                             "Logs"),
            ("Windows Error Reports","",                                                                   "Logs"),
            ("Corbeille",            "",                                                                    "Corbeille"),
            ("Chrome Cache",         "",                                                                   "Navigateurs"),
            ("Edge Cache",           "",                                                                   "Navigateurs"),
            ("Firefox Cache",        "",                                                                   "Navigateurs"),
            ("Windows Update Cache", "C:\\Windows\\SoftwareDistribution\\Download",                       "Windows Update"),
            ("Thumbnails DB",        "",                                                                   "Cache système"),
        ];
        let total = targets.len() as u8;
        for (idx, (name, path, cat)) in targets.iter().enumerate() {
            let ps_resolve = format!(r#"
$name='{name}';$path='{path}';$cat='{cat}'
if($name -eq '%TEMP%'){{$path=$env:TEMP}}
elseif($name -eq 'Windows Error Reports'){{$path="$env:LOCALAPPDATA\Microsoft\Windows\WER\ReportArchive"}}
elseif($name -eq 'Chrome Cache'){{$path="$env:LOCALAPPDATA\Google\Chrome\User Data\Default\Cache"}}
elseif($name -eq 'Edge Cache'){{$path="$env:LOCALAPPDATA\Microsoft\Edge\User Data\Default\Cache"}}
elseif($name -eq 'Firefox Cache'){{$path="$env:APPDATA\Mozilla\Firefox\Profiles"}}
$sz=0.0;$cnt=0
if($path -and (Test-Path $path)){{
    $files=@(Get-ChildItem $path -Recurse -File -EA SilentlyContinue)
    $cnt=$files.Count
    $bytes=($files|Measure-Object -Property Length -Sum -EA SilentlyContinue).Sum
    $sz=if($bytes){{[math]::Round($bytes/1MB,2)}}else{{0}}
}}elseif($name -eq 'Corbeille'){{
    try{{$rb=(New-Object -ComObject Shell.Application).Namespace(0xA);$cnt=@($rb.Items()).Count;$sz=0.1*$cnt}}catch{{}}
}}
@{{name=$name;path=[string]$path;mb=$sz;count=$cnt;cat=$cat}}|ConvertTo-Json -Compress
"#, name=name, path=path, cat=cat);

            let result = {
                #[cfg(target_os = "windows")]
                {
                    Command::new("powershell")
                        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_resolve])
                        .creation_flags(0x08000000)
                        .output()
                        .ok()
                        .and_then(|o| serde_json::from_str::<serde_json::Value>(
                            String::from_utf8_lossy(&o.stdout).trim()
                        ).ok())
                }
                #[cfg(not(target_os = "windows"))]
                { None::<serde_json::Value> }
            };

            let item = result.as_ref().map(|v| CleanTarget {
                name:       v["name"].as_str().unwrap_or(name).to_string(),
                path:       v["path"].as_str().unwrap_or("").to_string(),
                size_mb:    v["mb"].as_f64().unwrap_or(0.0),
                file_count: v["count"].as_u64().unwrap_or(0) as u32,
                category:   v["cat"].as_str().unwrap_or(cat).to_string(),
            }).or_else(|| Some(CleanTarget {
                name: name.to_string(),
                path: path.to_string(),
                size_mb: 0.0,
                file_count: 0,
                category: cat.to_string(),
            }));

            let _ = app_clone.emit("cleaner:progress", CleanerProgress {
                scanned: idx as u8 + 1,
                total,
                item,
                done: idx as u8 + 1 == total,
            });
        }
    }).await.ok();
}

#[tauri::command]
pub fn clean_target(target_name: String) -> CleanResult {
    let ps = match target_name.as_str() {
        "%TEMP%"               => "$b=0;$c=0;@(Get-ChildItem $env:TEMP -Recurse -File -EA SilentlyContinue)|ForEach-Object{$b+=$_.Length;$c++;Remove-Item $_.FullName -Force -EA SilentlyContinue};@{freed=[math]::Round($b/1MB,2);count=$c;ok=$true}|ConvertTo-Json -Compress".to_string(),
        "Windows\\Temp"        => "$b=0;$c=0;@(Get-ChildItem 'C:\\Windows\\Temp' -Recurse -File -EA SilentlyContinue)|ForEach-Object{$b+=$_.Length;$c++;Remove-Item $_.FullName -Force -EA SilentlyContinue};@{freed=[math]::Round($b/1MB,2);count=$c;ok=$true}|ConvertTo-Json -Compress".to_string(),
        "Prefetch"             => "$b=0;$c=0;@(Get-ChildItem 'C:\\Windows\\Prefetch\\*.pf' -EA SilentlyContinue)|ForEach-Object{$b+=$_.Length;$c++;Remove-Item $_.FullName -Force -EA SilentlyContinue};@{freed=[math]::Round($b/1MB,2);count=$c;ok=$true}|ConvertTo-Json -Compress".to_string(),
        "Dumps mémoire"        => "$b=0;$c=0;@(Get-ChildItem 'C:\\Windows\\Minidump\\*.dmp' -EA SilentlyContinue)+@(Get-Item 'C:\\Windows\\MEMORY.DMP' -EA SilentlyContinue)|ForEach-Object{$b+=$_.Length;$c++;Remove-Item $_.FullName -Force -EA SilentlyContinue};@{freed=[math]::Round($b/1MB,2);count=$c;ok=$true}|ConvertTo-Json -Compress".to_string(),
        "Corbeille"            => "Clear-RecycleBin -Force -EA SilentlyContinue;@{freed=0;count=0;ok=$true}|ConvertTo-Json -Compress".to_string(),
        "Chrome Cache"         => "$p=\"$env:LOCALAPPDATA\\Google\\Chrome\\User Data\\Default\\Cache\";$b=0;$c=0;if(Test-Path $p){@(Get-ChildItem $p -Recurse -File -EA SilentlyContinue)|ForEach-Object{$b+=$_.Length;$c++;Remove-Item $_.FullName -Force -EA SilentlyContinue}};@{freed=[math]::Round($b/1MB,2);count=$c;ok=$true}|ConvertTo-Json -Compress".to_string(),
        "Edge Cache"           => "$p=\"$env:LOCALAPPDATA\\Microsoft\\Edge\\User Data\\Default\\Cache\";$b=0;$c=0;if(Test-Path $p){@(Get-ChildItem $p -Recurse -File -EA SilentlyContinue)|ForEach-Object{$b+=$_.Length;$c++;Remove-Item $_.FullName -Force -EA SilentlyContinue}};@{freed=[math]::Round($b/1MB,2);count=$c;ok=$true}|ConvertTo-Json -Compress".to_string(),
        "Windows Update Cache" => "net stop wuauserv /y 2>&1|Out-Null;net stop bits /y 2>&1|Out-Null;Start-Sleep -Seconds 2;$b=0;$c=0;@(Get-ChildItem 'C:\\Windows\\SoftwareDistribution\\Download' -Recurse -File -EA SilentlyContinue)|ForEach-Object{$b+=$_.Length;$c++;Remove-Item $_.FullName -Force -EA SilentlyContinue};net start bits 2>&1|Out-Null;net start wuauserv 2>&1|Out-Null;@{freed=[math]::Round($b/1MB,2);count=$c;ok=$true}|ConvertTo-Json -Compress".to_string(),
        "Thumbnails DB"        => "$b=0;$c=0;try{Stop-Process -Name explorer -Force -EA SilentlyContinue;Start-Sleep -Milliseconds 500;$p=\"$env:LOCALAPPDATA\\Microsoft\\Windows\\Explorer\";@(Get-ChildItem $p -Filter 'thumbcache_*.db' -EA SilentlyContinue)|ForEach-Object{$b+=$_.Length;$c++;Remove-Item $_.FullName -Force -EA SilentlyContinue};@(Get-ChildItem $p -Filter 'iconcache_*.db' -EA SilentlyContinue)|ForEach-Object{$b+=$_.Length;$c++;Remove-Item $_.FullName -Force -EA SilentlyContinue}}finally{if(-not(Get-Process explorer -EA SilentlyContinue)){Start-Process explorer}};@{freed=[math]::Round($b/1MB,2);count=$c;ok=$true}|ConvertTo-Json -Compress".to_string(),
        _ => return CleanResult { target: target_name, success: false, message: "Cible inconnue".to_string(), ..Default::default() },
    };
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",&ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(t.trim()) {
                return CleanResult {
                    target: target_name,
                    freed_mb: v["freed"].as_f64().unwrap_or(0.0),
                    files_deleted: v["count"].as_u64().unwrap_or(0) as u32,
                    success: v["ok"].as_bool().unwrap_or(false),
                    message: String::new(),
                };
            }
        }
    }
    CleanResult { target: target_name, success: false, message: "Erreur".to_string(), ..Default::default() }
}

/// Quarantine: move files to %LOCALAPPDATA%\NiTriTe\quarantine\ instead of deleting
#[tauri::command]
pub fn quarantine_target(target_name: String) -> CleanResult {
    let src_ps = match target_name.as_str() {
        "%TEMP%"        => "$env:TEMP".to_string(),
        "Windows\\Temp" => "'C:\\Windows\\Temp'".to_string(),
        "Prefetch"      => "'C:\\Windows\\Prefetch'".to_string(),
        "Chrome Cache"  => "\"$env:LOCALAPPDATA\\Google\\Chrome\\User Data\\Default\\Cache\"".to_string(),
        "Edge Cache"    => "\"$env:LOCALAPPDATA\\Microsoft\\Edge\\User Data\\Default\\Cache\"".to_string(),
        _ => return CleanResult { target: target_name, success: false, message: "Quarantaine non supportée pour cette cible".to_string(), ..Default::default() },
    };
    let safe_name = target_name.replace(['\\', '/', ':', '*', '?', '"', '<', '>', '|'], "_");
    let ps = format!(r#"
$src = {src}
$qDir = "$env:LOCALAPPDATA\NiTriTe\quarantine\{name}"
New-Item -ItemType Directory -Force -Path $qDir | Out-Null
$b=0; $c=0
if (Test-Path $src) {{
    @(Get-ChildItem $src -Recurse -File -EA SilentlyContinue) | ForEach-Object {{
        $b += $_.Length; $c++
        $dst = Join-Path $qDir $_.Name
        try {{ Move-Item $_.FullName -Destination $dst -Force -EA SilentlyContinue }} catch {{}}
    }}
}}
@{{freed=[math]::Round($b/1MB,2);count=$c;ok=$true}} | ConvertTo-Json -Compress
"#, src = src_ps, name = safe_name);
    if let Some(v) = ps_run(&ps) {
        return CleanResult {
            target: target_name,
            freed_mb: v["freed"].as_f64().unwrap_or(0.0),
            files_deleted: v["count"].as_u64().unwrap_or(0) as u32,
            success: v["ok"].as_bool().unwrap_or(false),
            message: "Mis en quarantaine".to_string(),
        };
    }
    CleanResult { target: target_name, success: false, message: "Erreur quarantaine".to_string(), ..Default::default() }
}

/// List quarantine entries
#[tauri::command]
pub fn list_quarantine() -> Vec<serde_json::Value> {
    let ps = r#"
$qBase = "$env:LOCALAPPDATA\NiTriTe\quarantine"
if (!(Test-Path $qBase)) { @() | ConvertTo-Json -Compress; return }
@(Get-ChildItem $qBase -Directory -EA SilentlyContinue | ForEach-Object {
    $files = @(Get-ChildItem $_.FullName -Recurse -File -EA SilentlyContinue)
    $size = ($files | Measure-Object -Property Length -Sum).Sum
    @{ name=$_.Name; path=$_.FullName; file_count=$files.Count; size_mb=[math]::Round($size/1MB,2) }
}) | ConvertTo-Json -Compress"#;
    if let Some(v) = ps_run(ps) {
        if let Some(arr) = v.as_array() { return arr.clone(); }
        return vec![v];
    }
    vec![]
}

/// Clear quarantine (permanently delete quarantine folder contents)
#[tauri::command]
pub fn clear_quarantine(entry_name: Option<String>) -> bool {
    let ps = if let Some(name) = entry_name {
        // Whitelist stricte : alphanum, espace, tiret, underscore, point uniquement
        let safe: String = name.chars().map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' || c == '.' { c } else { '_' }
        }).collect();
        format!("$p=\"$env:LOCALAPPDATA\\NiTriTe\\quarantine\\{safe}\";if(Test-Path $p){{Remove-Item $p -Recurse -Force -EA SilentlyContinue}};$true")
    } else {
        "$p=\"$env:LOCALAPPDATA\\NiTriTe\\quarantine\";if(Test-Path $p){Remove-Item $p -Recurse -Force -EA SilentlyContinue};$true".to_string()
    };
    ps_run(&ps).is_some()
}

#[tauri::command]
pub async fn get_large_files(folder: String, min_size_mb: f64) -> Vec<serde_json::Value> {
    tokio::task::spawn_blocking(move || get_large_files_sync(folder, min_size_mb))
        .await
        .unwrap_or_default()
}

fn get_large_files_sync(folder: String, min_size_mb: f64) -> Vec<serde_json::Value> {
    // Whitelist stricte pour le chemin : alphanum, séparateurs de chemin Windows, tiret, underscore, point, espace
    // Rejette tout ce qui pourrait injecter du PS ($, `, ;, |, &, (, ), {, }, etc.)
    let f: String = folder.chars().map(|c| {
        if c.is_alphanumeric() || matches!(c, '\\' | '/' | ':' | ' ' | '-' | '_' | '.' | '(' | ')') {
            // Parenthèses autorisées dans les noms de dossiers Windows (ex: Program Files (x86))
            // mais on bloque $ et ` qui sont les vecteurs PS réels
            c
        } else {
            '_'
        }
    }).collect::<String>();
    let f = f.trim().to_string();
    // Validation supplémentaire : le chemin doit commencer par une lettre de lecteur (ex: C:\)
    let valid_path = {
        let mut chars = f.chars();
        let first = chars.next();
        let colon = chars.next();
        first.map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
            && colon == Some(':')
    };
    if !valid_path || f.is_empty() {
        return vec![];
    }
    let min_bytes = (min_size_mb * 1048576.0) as u64;
    let ps = format!(r#"
@(Get-ChildItem '{folder}' -Recurse -File -EA SilentlyContinue |
    Where-Object {{ $_.Length -ge {min} }} |
    Sort-Object Length -Descending |
    Select-Object -First 100 |
    ForEach-Object {{ @{{ name=$_.Name; path=$_.FullName; mb=[math]::Round($_.Length/1MB,1); ext=$_.Extension; mod=[string]$_.LastWriteTime.ToString('yyyy-MM-dd') }} }}) | ConvertTo-Json -Compress
"#, folder=f, min=min_bytes);
    #[cfg(target_os = "windows")]
    {
        let o = Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",&ps]).creation_flags(0x08000000).output();
        if let Ok(o) = o {
            let t = String::from_utf8_lossy(&o.stdout); let t = t.trim();
            let arr_t = if t.starts_with('{') { format!("[{}]",t) } else { t.to_string() };
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&arr_t) { return arr; }
        }
    }
    vec![]
}
