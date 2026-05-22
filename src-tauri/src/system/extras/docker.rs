use serde::Serialize;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// ─── Docker Manager ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct DockerContainer {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub ports: String,
    pub created: String,
}

#[derive(Serialize)]
pub struct DockerImage {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub size: String,
    pub created: String,
}

#[derive(Serialize)]
pub struct DockerInfo {
    pub available: bool,
    pub version: String,
    pub containers: Vec<DockerContainer>,
    pub images: Vec<DockerImage>,
}

#[tauri::command]
pub fn get_docker_info() -> Result<DockerInfo, String> {
    let version_out = std::process::Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .creation_flags(0x08000000)
        .output();

    let (available, version) = match version_out {
        Ok(o) if o.status.success() => (true, String::from_utf8_lossy(&o.stdout).trim().to_string()),
        _ => return Ok(DockerInfo { available: false, version: String::new(), containers: vec![], images: vec![] }),
    };

    let containers = parse_docker_ps();
    let images = parse_docker_images();

    Ok(DockerInfo { available, version, containers, images })
}

fn parse_docker_ps() -> Vec<DockerContainer> {
    let out = std::process::Command::new("docker")
        .args(["ps", "-a", "--format", "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}\t{{.CreatedAt}}"])
        .creation_flags(0x08000000)
        .output()
        .ok();
    let text = out.map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    text.lines().filter(|l| !l.is_empty()).map(|line| {
        let parts: Vec<&str> = line.splitn(6, '\t').collect();
        DockerContainer {
            id: parts.first().unwrap_or(&"").to_string(),
            name: parts.get(1).unwrap_or(&"").to_string(),
            image: parts.get(2).unwrap_or(&"").to_string(),
            status: parts.get(3).unwrap_or(&"").to_string(),
            ports: parts.get(4).unwrap_or(&"").to_string(),
            created: parts.get(5).unwrap_or(&"").to_string(),
        }
    }).collect()
}

fn parse_docker_images() -> Vec<DockerImage> {
    let out = std::process::Command::new("docker")
        .args(["images", "--format", "{{.ID}}\t{{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedAt}}"])
        .creation_flags(0x08000000)
        .output()
        .ok();
    let text = out.map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    text.lines().filter(|l| !l.is_empty()).map(|line| {
        let parts: Vec<&str> = line.splitn(5, '\t').collect();
        DockerImage {
            id: parts.first().unwrap_or(&"").to_string(),
            repository: parts.get(1).unwrap_or(&"").to_string(),
            tag: parts.get(2).unwrap_or(&"").to_string(),
            size: parts.get(3).unwrap_or(&"").to_string(),
            created: parts.get(4).unwrap_or(&"").to_string(),
        }
    }).collect()
}

#[tauri::command]
pub fn docker_container_action(container_id: String, action: String) -> Result<String, String> {
    let valid_actions = ["start", "stop", "restart", "rm", "kill"];
    if !valid_actions.contains(&action.as_str()) {
        return Err(format!("Action invalide: {}", action));
    }
    let out = std::process::Command::new("docker")
        .args([action.as_str(), &container_id])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[tauri::command]
pub fn docker_image_remove(image_id: String) -> Result<String, String> {
    let out = std::process::Command::new("docker")
        .args(["rmi", &image_id])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[tauri::command]
pub fn docker_container_logs(container_id: String, lines: u32) -> Result<String, String> {
    let n = lines.min(500).to_string();
    let out = std::process::Command::new("docker")
        .args(["logs", "--tail", &n, "--timestamps", &container_id])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    Ok(if stdout.is_empty() { stderr } else { stdout })
}
