use serde::Serialize;
#[cfg(target_os = "windows")]
use super::{parse_json_arr, ps};

// ─── Disk Space Visualizer ────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct DiskNode {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub is_dir: bool,
    pub children: Vec<DiskNode>,
}

#[tauri::command]
pub async fn get_disk_tree(path: String, max_depth: u32) -> Result<DiskNode, String> {
    tokio::task::spawn_blocking(move || get_disk_tree_sync(path, max_depth))
        .await
        .map_err(|e| e.to_string())?
}

fn get_disk_tree_sync(path: String, max_depth: u32) -> Result<DiskNode, String> {
    let p = std::path::Path::new(&path);
    Ok(disk_build_node(p, 0, max_depth.min(4)))
}

fn disk_dir_size(p: &std::path::Path) -> u64 {
    std::fs::read_dir(p).ok().map(|rd| {
        rd.filter_map(|e| e.ok()).map(|e| {
            if e.path().is_dir() { disk_dir_size(&e.path()) }
            else { e.metadata().map(|m| m.len()).unwrap_or(0) }
        }).sum()
    }).unwrap_or(0)
}

fn disk_build_node(p: &std::path::Path, depth: u32, max: u32) -> DiskNode {
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.to_string_lossy().to_string());
    let is_dir = p.is_dir();
    if !is_dir {
        let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        return DiskNode { name, path: p.to_string_lossy().to_string(), size_bytes: size, is_dir: false, children: vec![] };
    }
    if depth >= max {
        let size = disk_dir_size(p);
        return DiskNode { name, path: p.to_string_lossy().to_string(), size_bytes: size, is_dir: true, children: vec![] };
    }
    let mut children: Vec<DiskNode> = std::fs::read_dir(p).ok().map(|rd| {
        rd.filter_map(|e| e.ok()).map(|e| disk_build_node(&e.path(), depth + 1, max)).collect()
    }).unwrap_or_default();
    children.sort_by_key(|a| std::cmp::Reverse(a.size_bytes));
    children.truncate(20);
    let size: u64 = children.iter().map(|c| c.size_bytes).sum();
    DiskNode { name, path: p.to_string_lossy().to_string(), size_bytes: size, is_dir: true, children }
}

// ─── Big Files Finder ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct BigFile {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub extension: String,
    pub modified: String,
}

#[tauri::command]
pub fn get_big_files(path: String, count: u32, excluded_folders: Option<Vec<String>>) -> Result<Vec<BigFile>, String> {
    let n = count.min(200);
    let exclusions = excluded_folders.unwrap_or_default();
    let exclude_filter = if exclusions.is_empty() {
        String::new()
    } else {
        let patterns: Vec<String> = exclusions.iter()
            .map(|p| format!("$_.FullName -notlike '{}*'", p.replace('\'', "''")))
            .collect();
        format!("| Where-Object {{ {} }}", patterns.join(" -and "))
    };
    let script = format!(r#"
Get-ChildItem -Path '{path}' -Recurse -File -ErrorAction SilentlyContinue {exclude_filter}|
    Sort-Object Length -Descending | Select-Object -First {n} |
    ForEach-Object {{
        [PSCustomObject]@{{
            name=$_.Name
            path=$_.FullName
            size=$_.Length
            ext=$_.Extension
            modified=$_.LastWriteTime.ToString('yyyy-MM-dd HH:mm')
        }}
    }} | ConvertTo-Json -Compress
"#, path = path.replace('\'', "''"), n = n, exclude_filter = exclude_filter);
    let out = ps(&script)?;
    if out.is_empty() { return Ok(vec![]); }
    let arr: Vec<serde_json::Value> = parse_json_arr(&out);
    Ok(arr.iter().map(|v| BigFile {
        name: v["name"].as_str().unwrap_or("").to_string(),
        path: v["path"].as_str().unwrap_or("").to_string(),
        size_bytes: v["size"].as_u64().unwrap_or(0),
        extension: v["ext"].as_str().unwrap_or("").to_string(),
        modified: v["modified"].as_str().unwrap_or("").to_string(),
    }).collect())
}

// ─── Duplicate Finder ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct DuplicateGroup {
    pub hash: String,
    pub size_bytes: u64,
    pub count: u32,
    pub files: Vec<String>,
    pub wasted_bytes: u64,
}

#[tauri::command]
pub async fn find_duplicates(path: String, min_size_kb: u64) -> Result<Vec<DuplicateGroup>, String> {
    tokio::task::spawn_blocking(move || find_duplicates_sync(path, min_size_kb))
        .await
        .map_err(|e| e.to_string())?
}

fn find_duplicates_sync(path: String, min_size_kb: u64) -> Result<Vec<DuplicateGroup>, String> {
    let min_bytes = (min_size_kb * 1024).max(1024);
    let script = format!(r#"
$files = Get-ChildItem -Path '{path}' -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object {{ $_.Length -ge {min} }}
$groups = $files | Group-Object Length | Where-Object {{ $_.Count -gt 1 }}
$dupes = @()
foreach ($g in $groups) {{
    $hashed = $g.Group | ForEach-Object {{
        [PSCustomObject]@{{ path=$_.FullName; hash=(Get-FileHash $_.FullName -Algorithm MD5).Hash; size=$_.Length }}
    }}
    $hashGroups = $hashed | Group-Object hash | Where-Object {{ $_.Count -gt 1 }}
    foreach ($hg in $hashGroups) {{
        $dupes += [PSCustomObject]@{{
            hash=$hg.Name
            size=$hg.Group[0].size
            count=$hg.Count
            files=($hg.Group | ForEach-Object {{ $_.path }}) -join '|'
        }}
    }}
}}
$dupes | ConvertTo-Json -Compress
"#, path = path.replace('\'', "''"), min = min_bytes);
    let out = ps(&script)?;
    if out.is_empty() { return Ok(vec![]); }
    let arr: Vec<serde_json::Value> = parse_json_arr(&out);
    let mut groups: Vec<DuplicateGroup> = arr.iter().map(|v| {
        let size = v["size"].as_u64().unwrap_or(0);
        let count = v["count"].as_u64().unwrap_or(2) as u32;
        let files: Vec<String> = v["files"].as_str().unwrap_or("").split('|').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect();
        DuplicateGroup { hash: v["hash"].as_str().unwrap_or("").to_string(), size_bytes: size, count, wasted_bytes: size * (count as u64 - 1), files }
    }).collect();
    groups.sort_by_key(|a| std::cmp::Reverse(a.wasted_bytes));
    Ok(groups)
}

// ─── Delete File ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn delete_file(path: String) -> Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| format!("{}: {}", path, e))
}

#[tauri::command]
pub fn trash_file(path: String) -> Result<(), String> {
    let safe = path.replace('\'', "''");
    let script = format!(r#"
Add-Type -AssemblyName Microsoft.VisualBasic
[Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile(
    '{safe}',
    [Microsoft.VisualBasic.FileIO.UIOption]::OnlyErrorDialogs,
    [Microsoft.VisualBasic.FileIO.RecycleOption]::SendToRecycleBin
)"#);
    ps(&script)?;
    Ok(())
}

// ─── Browser Cache ────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct BrowserCacheInfo {
    pub browser: String,
    pub path: String,
    pub size_mb: f64,
    pub exists: bool,
}

#[tauri::command]
pub fn get_browser_cache_info() -> Result<Vec<BrowserCacheInfo>, String> {
    let script = r#"
$localApp = [Environment]::GetFolderPath('LocalApplicationData')
$roaming   = [Environment]::GetFolderPath('ApplicationData')
$caches = @(
    @{ browser='Google Chrome';    path="$localApp\Google\Chrome\User Data\Default\Cache" },
    @{ browser='Microsoft Edge';   path="$localApp\Microsoft\Edge\User Data\Default\Cache" },
    @{ browser='Firefox';          path="$roaming\Mozilla\Firefox\Profiles" },
    @{ browser='Opera';            path="$roaming\Opera Software\Opera Stable\Cache" },
    @{ browser='Brave';            path="$localApp\BraveSoftware\Brave-Browser\User Data\Default\Cache" },
    @{ browser='Vivaldi';          path="$localApp\Vivaldi\User Data\Default\Cache" }
)
$result = $caches | ForEach-Object {
    $p = $_.path
    $exists = Test-Path $p
    $size = 0.0
    if ($exists) {
        $size = [math]::Round((Get-ChildItem $p -Recurse -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum / 1MB, 2)
    }
    [PSCustomObject]@{ browser=$_.browser; path=$p; size_mb=$size; exists=[bool]$exists }
}
$result | ConvertTo-Json -Compress"#;
    let out = ps(script)?;
    if out.is_empty() { return Ok(vec![]); }
    let arr: Vec<serde_json::Value> = parse_json_arr(&out);
    Ok(arr.iter().map(|v| BrowserCacheInfo {
        browser:  v["browser"].as_str().unwrap_or("").to_string(),
        path:     v["path"].as_str().unwrap_or("").to_string(),
        size_mb:  v["size_mb"].as_f64().unwrap_or(0.0),
        exists:   v["exists"].as_bool().unwrap_or(false),
    }).collect())
}

#[tauri::command]
pub fn clean_browser_cache_path(browser_path: String) -> Result<f64, String> {
    if browser_path.is_empty() || browser_path.contains("..") {
        return Err("Chemin invalide".to_string());
    }
    let safe = browser_path.replace('\'', "''");
    let script = format!(r#"
$before = (Get-ChildItem '{safe}' -Recurse -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum
Remove-Item '{safe}\*' -Recurse -Force -ErrorAction SilentlyContinue
$after  = (Get-ChildItem '{safe}' -Recurse -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum
[math]::Round(($before - $after) / 1MB, 2)"#);
    let out = ps(&script)?;
    Ok(out.trim().parse::<f64>().unwrap_or(0.0))
}
