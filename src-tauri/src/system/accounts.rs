use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Default)]
pub struct UserAccount {
    pub name: String,
    pub full_name: String,
    pub description: String,
    pub enabled: bool,
    pub last_logon: String,
    pub password_required: bool,
    pub is_admin: bool,
    pub account_type: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct LocalGroup {
    pub name: String,
    pub description: String,
    pub member_count: u32,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PasswordPolicy {
    pub min_length: u32,
    pub max_age_days: i32,
    pub min_age_days: i32,
    pub complexity: bool,
    pub lockout_threshold: u32,
    pub lockout_duration: i32,
    pub history_count: u32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct AccountsInfo {
    pub users: Vec<UserAccount>,
    pub groups: Vec<LocalGroup>,
    pub policy: PasswordPolicy,
    pub total_enabled: u32,
    pub total_admin: u32,
}

// Anti-freeze : Get-LocalGroupMember/Get-LocalUser via PowerShell est bloquant
// (subprocess synchrone) — jamais l'exécuter inline sur le thread de commande,
// voir utils::wmi pour le même principe appliqué aux requêtes WMI.
#[tauri::command]
pub async fn get_user_accounts() -> AccountsInfo {
    tokio::task::spawn_blocking(get_user_accounts_blocking)
        .await
        .unwrap_or_default()
}

fn get_user_accounts_blocking() -> AccountsInfo {
    let ps = r#"
# Sortie UTF-8 : noms/descriptions de comptes et groupes accentués (FR
# « Administrateurs », « Compte d'utilisateur invité »…) seraient sinon mojibake.
$OutputEncoding = [System.Text.Encoding]::UTF8; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$out = @{}
$admins = @(Get-LocalGroupMember -Group 'Administrators' -ErrorAction SilentlyContinue |
    Select-Object -ExpandProperty Name)

$out.Users = @(Get-LocalUser -ErrorAction SilentlyContinue | ForEach-Object {
    $n = $_.Name
    $isAdmin = [bool]($admins | Where-Object { $_ -like "*\$n" -or $_ -eq $n } | Select-Object -First 1)
    $acctType = if ($_.PrincipalSource -eq 'MicrosoftAccount') { 'Microsoft' }
                elseif ($_.PrincipalSource -eq 'AzureAD') { 'Azure AD' }
                else { 'Local' }
    @{
        name=$n; fullName=[string]$_.FullName; description=[string]$_.Description;
        enabled=[bool]$_.Enabled;
        lastLogon=if($_.LastLogon){$_.LastLogon.ToString('yyyy-MM-dd HH:mm')}else{'Jamais'};
        passwordRequired=[bool]$_.PasswordRequired;
        isAdmin=$isAdmin; accountType=$acctType
    }
})

$out.Groups = @(Get-LocalGroup -ErrorAction SilentlyContinue | Select-Object -First 25 | ForEach-Object {
    $members = @(Get-LocalGroupMember -Group $_.Name -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty Name | Select-Object -First 20)
    @{name=$_.Name; description=[string]$_.Description; memberCount=$members.Count; members=@($members)}
})

try {
    # secedit /export : clés INF INVARIANTES (MinimumPasswordLength…), contrairement
    # à `net accounts` dont les libellés sont traduits — sur Windows FR aucun regex
    # anglais ne matchait et toute la politique tombait à 0/-1. secedit requiert
    # l'élévation ; à défaut on garde les valeurs par défaut (fallback catch).
    $inf = Join-Path $env:TEMP ("nitrite_secpol_" + [guid]::NewGuid().ToString('N') + ".inf")
    secedit /export /cfg $inf /areas SECURITYPOLICY 2>$null | Out-Null
    if (Test-Path $inf) {
        $c = Get-Content $inf -ErrorAction SilentlyContinue
        function Sec($key, $def) {
            $m = $c | Where-Object { $_ -match ("^\s*" + $key + "\s*=\s*(-?\d+)") } | Select-Object -First 1
            if ($m -and $m -match '=\s*(-?\d+)') { [int]$matches[1] } else { $def }
        }
        $out.Policy = @{
            MinLen  = Sec 'MinimumPasswordLength' 0
            MaxAge  = Sec 'MaximumPasswordAge' -1
            MinAge  = Sec 'MinimumPasswordAge' 0
            Lockout = Sec 'LockoutBadCount' 0
            LockDur = Sec 'LockoutDuration' -1
            History = Sec 'PasswordHistorySize' 0
            Complex = [bool]((Sec 'PasswordComplexity' 0) -eq 1)
        }
        Remove-Item $inf -Force -ErrorAction SilentlyContinue
    } else {
        $out.Policy = @{MinLen=0;MaxAge=-1;MinAge=0;Lockout=0;LockDur=-1;History=0;Complex=$false}
    }
} catch {
    $out.Policy = @{MinLen=0;MaxAge=-1;MinAge=0;Lockout=0;LockDur=-1;History=0;Complex=$false}
}

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
                Ok(val) => val, Err(_) => return AccountsInfo::default(),
            };

            let users: Vec<UserAccount> = v["Users"].as_array().map(|a| a.iter().map(|u| UserAccount {
                name: u["name"].as_str().unwrap_or("").to_string(),
                full_name: u["fullName"].as_str().unwrap_or("").to_string(),
                description: u["description"].as_str().unwrap_or("").to_string(),
                enabled: u["enabled"].as_bool().unwrap_or(false),
                last_logon: u["lastLogon"].as_str().unwrap_or("Jamais").to_string(),
                password_required: u["passwordRequired"].as_bool().unwrap_or(false),
                is_admin: u["isAdmin"].as_bool().unwrap_or(false),
                account_type: u["accountType"].as_str().unwrap_or("Local").to_string(),
            }).collect()).unwrap_or_default();

            let groups: Vec<LocalGroup> = v["Groups"].as_array().map(|a| a.iter().map(|g| LocalGroup {
                name: g["name"].as_str().unwrap_or("").to_string(),
                description: g["description"].as_str().unwrap_or("").to_string(),
                member_count: g["memberCount"].as_u64().unwrap_or(0) as u32,
                members: g["members"].as_array().map(|m| m.iter()
                    .filter_map(|s| s.as_str().map(|x| x.to_string())).collect())
                    .unwrap_or_default(),
            }).collect()).unwrap_or_default();

            let p = &v["Policy"];
            let policy = PasswordPolicy {
                min_length: p["MinLen"].as_u64().unwrap_or(0) as u32,
                max_age_days: p["MaxAge"].as_i64().unwrap_or(-1) as i32,
                min_age_days: p["MinAge"].as_i64().unwrap_or(0) as i32,
                complexity: p["Complex"].as_bool().unwrap_or(false),
                lockout_threshold: p["Lockout"].as_u64().unwrap_or(0) as u32,
                lockout_duration: p["LockDur"].as_i64().unwrap_or(-1) as i32,
                history_count: p["History"].as_u64().unwrap_or(0) as u32,
            };

            let total_enabled = users.iter().filter(|u| u.enabled).count() as u32;
            let total_admin = users.iter().filter(|u| u.is_admin).count() as u32;

            return AccountsInfo { users, groups, policy, total_enabled, total_admin };
        }
    }
    AccountsInfo::default()
}
