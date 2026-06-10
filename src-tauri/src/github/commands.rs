//! GitHub Tauri commands. The token is read/written only via the keychain; the
//! login name is cached in settings so status checks need no network.

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::db::Db;
use crate::github::{api, clone, keychain};
use crate::projects::{self, Project};
use crate::settings;

#[derive(Debug, Serialize)]
pub struct GithubStatus {
    pub connected: bool,
    pub login: Option<String>,
}

const LOGIN_KEY: &str = "github.login";

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
                "GitHub CLI (`gh`) not found. Install it (brew install gh), run `gh auth login`, then retry.".to_string()
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

// ---- Add-project paths (import / link / create-from-app) ----

/// Parse a GitHub repo reference: `owner/repo`, an https URL, or an ssh URL.
fn parse_repo_ref(input: &str) -> Option<(String, String)> {
    let s = input.trim().trim_end_matches('/');
    let s = s.strip_suffix(".git").unwrap_or(s);
    if let Some(rest) = s.strip_prefix("git@github.com:") {
        return split_owner_repo(rest);
    }
    // HTTPS or SSH only — reject plaintext http:// so a link/clone can't be
    // silently downgraded to a MITM-able transport. (Bare github.com/ resolves
    // to HTTPS at clone time.)
    for prefix in ["https://github.com/", "github.com/"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            return split_owner_repo(rest);
        }
    }
    if s.matches('/').count() == 1 && !s.contains(char::is_whitespace) {
        return split_owner_repo(s);
    }
    None
}

fn split_owner_repo(rest: &str) -> Option<(String, String)> {
    let mut parts = rest.split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

/// Path 1: import a repo picked from the connected user's list.

#[tauri::command]
pub fn project_import_repo(
    db: State<Db>,
    full_name: String,
    clone_url: String,
    default_branch: String,
) -> Result<Project, String> {
    let name = full_name.rsplit('/').next().unwrap_or(&full_name).to_string();
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    projects::insert_attached(&conn, &name, "imported", Some(&clone_url), Some(&default_branch))
}

/// Path 3: link a repo by URL. Validates it exists (and pulls its metadata).
#[tauri::command]
pub fn project_link_url(db: State<Db>, url: String) -> Result<Project, String> {
    let (owner, repo) = parse_repo_ref(&url)
        .ok_or("That doesn't look like a GitHub repo (expected github.com/owner/repo).")?;
    let summary = api::get_repo(&owner, &repo)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    projects::insert_attached(
        &conn,
        &summary.name,
        "imported",
        Some(&summary.clone_url),
        Some(&summary.default_branch),
    )
}

/// Path 4: create a brand-new repo on GitHub and attach it.
#[tauri::command]
pub fn project_create_repo(db: State<Db>, name: String, private: bool) -> Result<Project, String> {
    let summary = api::create_repo(&name, private)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    projects::insert_attached(
        &conn,
        &summary.name,
        "new",
        Some(&summary.clone_url),
        Some(&summary.default_branch),
    )
}

/// Clone (or refresh) a project's repo into the app-data clone cache. Idempotent:
/// the first call clones, later calls re-pull. Used on attach and on "refresh".
#[tauri::command]
pub async fn project_clone(app: AppHandle, db: State<'_, Db>, project_id: i64) -> Result<Project, String> {
    let project = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        projects::get(&conn, project_id)?.ok_or("Project not found.")?
    };
    let url = project
        .github_repo_url
        .ok_or("This project isn't linked to a GitHub repo.")?;
    let branch = project.default_branch.unwrap_or_else(|| "main".to_string());
    let token = keychain::get_token()?.ok_or("Not connected to GitHub.")?;

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dest = data_dir.join("clones").join(project_id.to_string());
    clone::clone_or_refresh(&data_dir, &url, &dest, &branch, &token)?;

    let dest_str = dest.to_string_lossy().to_string();
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    projects::set_clone_path(&conn, project_id, &dest_str)?;
    // Add Understand-hub cards for any tech detected in the freshly cloned repo,
    // and associate them with this project (for the "This project" filter).
    let _ = crate::cards::detect_tech_in_clone(&conn, &dest_str, Some(project_id));
    projects::get(&conn, project_id)?.ok_or_else(|| "Project vanished after clone.".to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_repo_ref;

    #[test]
    fn parses_repo_refs_in_several_forms() {
        let expected = Some(("hotredsam".to_string(), "Review-Helper".to_string()));
        assert_eq!(parse_repo_ref("hotredsam/Review-Helper"), expected);
        assert_eq!(parse_repo_ref("https://github.com/hotredsam/Review-Helper"), expected);
        assert_eq!(parse_repo_ref("https://github.com/hotredsam/Review-Helper.git"), expected);
        assert_eq!(parse_repo_ref("https://github.com/hotredsam/Review-Helper/"), expected);
        assert_eq!(parse_repo_ref("git@github.com:hotredsam/Review-Helper.git"), expected);
        assert_eq!(parse_repo_ref("github.com/hotredsam/Review-Helper"), expected);
    }

    #[test]
    fn rejects_bad_refs() {
        assert_eq!(parse_repo_ref("not a url"), None);
        assert_eq!(parse_repo_ref("https://gitlab.com/a/b"), None);
        assert_eq!(parse_repo_ref("just-a-name"), None);
        assert_eq!(parse_repo_ref("a/b/c"), None);
    }

    #[test]
    fn rejects_plaintext_http_github_urls() {
        // http:// is MITM-able; only https/ssh/bare forms are accepted.
        assert_eq!(parse_repo_ref("http://github.com/hotredsam/Review-Helper"), None);
        // The secure forms still parse.
        let expected = Some(("hotredsam".to_string(), "Review-Helper".to_string()));
        assert_eq!(parse_repo_ref("https://github.com/hotredsam/Review-Helper"), expected);
        assert_eq!(parse_repo_ref("hotredsam/Review-Helper"), expected);
    }
}
