use serde::Serialize;
use wmi::WMIConnection;

#[derive(Debug, Serialize, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub state: String,
    pub start_mode: String,
    pub path_name: String,
    pub description: String,
    pub process_id: u32,
    pub service_type: String,
    pub started: bool,
    pub account: String,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct WmiService {
    Name: Option<String>,
    DisplayName: Option<String>,
    State: Option<String>,
    StartMode: Option<String>,
    PathName: Option<String>,
    Description: Option<String>,
    ProcessId: Option<u32>,
    ServiceType: Option<String>,
    Started: Option<bool>,
    StartName: Option<String>,
}

fn wmi_con() -> Result<WMIConnection, String> {
    WMIConnection::new().map_err(|e| format!("WMI: {}", e))
}

pub fn collect_services() -> Result<Vec<ServiceInfo>, String> {
    let wmi = wmi_con()?;
    let results: Vec<WmiService> = wmi
        .raw_query("SELECT Name, DisplayName, State, StartMode, PathName, Description, ProcessId, ServiceType, Started, StartName FROM Win32_Service")
        .map_err(|e| e.to_string())?;

    let mut list: Vec<ServiceInfo> = results.into_iter().map(|s| ServiceInfo {
        name: s.Name.unwrap_or_default().trim().to_string(),
        display_name: s.DisplayName.unwrap_or_default().trim().to_string(),
        state: s.State.unwrap_or_default().trim().to_string(),
        start_mode: s.StartMode.unwrap_or_default().trim().to_string(),
        path_name: s.PathName.unwrap_or_default().trim().to_string(),
        description: s.Description.unwrap_or_default().trim().to_string(),
        process_id: s.ProcessId.unwrap_or(0),
        service_type: s.ServiceType.unwrap_or_default().trim().to_string(),
        started: s.Started.unwrap_or(false),
        account: s.StartName.unwrap_or_default().trim().to_string(),
    }).collect();

    list.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(list)
}
