//! GitHub Tauri commands. The token is read/written only via the keychain; the
//! login name is cached in settings so status checks need no network.

use serde::Serialize;
use tauri::State;

use crate::db::Db;
use crate::github::{api, device, keychain};
use crate::settings;

#[derive(Debug, Serialize)]
pub struct GithubStatus {
    pub connected: bool,
    pub login: Option<String>,
}

const LOGIN_KEY: &str = "github.login";
const CLIENT_ID_KEY: &str = "github.client_id";

fn stored_login(db: &State<Db>) -> Option<String> {
    let conn = db.0.lock().ok()?;
    settings::get(&conn, LOGIN_KEY)
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
}

fn set_login(db: &State<Db>, login: &str) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    settings::set(&conn, LOGIN_KEY, login)
}

#[tauri::command]
pub fn github_status(db: State<Db>) -> Result<GithubStatus, String> {
    let connected = keychain::get_token()?.is_some();
    let login = if connected { stored_login(&db) } else { None };
    Ok(GithubStatus { connected, login })
}

/// Import the token from the already-authenticated `gh` CLI into the app
/// keychain (the v1 connect path; no browser auth needed).
#[tauri::command]
pub fn github_connect_gh(db: State<Db>) -> Result<GithubStatus, String> {
    let output = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "GitHub CLI (`gh`) not found. Install it, or sign in with the device flow.".to_string()
            } else {
                e.to_string()
            }
        })?;
    if !output.status.success() {
        return Err("`gh` is not signed in. Run `gh auth login` first.".into());
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        return Err("`gh` returned an empty token.".into());
    }
    let login = api::get_login_with(&token)?; // validate before storing
    keychain::save_token(&token)?;
    set_login(&db, &login)?;
    Ok(GithubStatus {
        connected: true,
        login: Some(login),
    })
}

#[tauri::command]
pub fn github_sign_out(db: State<Db>) -> Result<(), String> {
    keychain::delete_token()?;
    let _ = set_login(&db, "");
    Ok(())
}

#[tauri::command]
pub fn github_list_repos() -> Result<Vec<api::RepoSummary>, String> {
    api::list_repos()
}

// ---- Device flow: built now, active once a client_id is configured ----

fn client_id(db: &State<Db>) -> Result<String, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    settings::get(&conn, CLIENT_ID_KEY)
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "No GitHub OAuth client_id configured. Connect via the gh CLI, or set one in Settings.".to_string()
        })
}

#[tauri::command]
pub fn github_device_start(db: State<Db>) -> Result<device::DeviceCode, String> {
    device::request_device_code(&client_id(&db)?)
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DevicePollResult {
    Authorized { login: String },
    Pending,
    SlowDown,
    Denied,
    Expired,
    Error { detail: String },
}

#[tauri::command]
pub fn github_device_poll(db: State<Db>, device_code: String) -> Result<DevicePollResult, String> {
    let client_id = client_id(&db)?;
    Ok(match device::poll_token(&client_id, &device_code) {
        device::PollOutcome::Authorized(token) => {
            let login = api::get_login_with(&token)?;
            keychain::save_token(&token)?;
            set_login(&db, &login)?;
            DevicePollResult::Authorized { login }
        }
        device::PollOutcome::Pending => DevicePollResult::Pending,
        device::PollOutcome::SlowDown => DevicePollResult::SlowDown,
        device::PollOutcome::Denied => DevicePollResult::Denied,
        device::PollOutcome::Expired => DevicePollResult::Expired,
        device::PollOutcome::Error(detail) => DevicePollResult::Error { detail },
    })
}
