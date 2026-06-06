//! GitHub REST client. Reads the token from the keychain, sends the current
//! auth + version headers, and maps failures to friendly messages. A 401 clears
//! the stored token so the UI re-prompts to connect.

use serde::{Deserialize, Serialize};

use super::{http_client, keychain};

const API: &str = "https://api.github.com";
const ACCEPT: &str = "application/vnd.github+json";
const API_VERSION: &str = "2026-03-10";

/// A repository as the UI needs it.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RepoSummary {
    pub full_name: String,
    pub name: String,
    pub clone_url: String,
    pub private: bool,
    pub default_branch: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
struct RepoJson {
    full_name: String,
    name: String,
    clone_url: String,
    private: bool,
    #[serde(default = "default_branch")]
    default_branch: String,
    description: Option<String>,
}

fn default_branch() -> String {
    "main".to_string()
}

impl From<RepoJson> for RepoSummary {
    fn from(r: RepoJson) -> Self {
        RepoSummary {
            full_name: r.full_name,
            name: r.name,
            clone_url: r.clone_url,
            private: r.private,
            default_branch: r.default_branch,
            description: r.description,
        }
    }
}

fn token() -> Result<String, String> {
    keychain::get_token()?.ok_or_else(|| "Not connected to GitHub.".to_string())
}

/// Translate a non-success status into a friendly error. A 401 also clears the
/// stored token (so the app falls back to a disconnected state).
fn status_error(status: reqwest::StatusCode) -> String {
    match status.as_u16() {
        401 => {
            let _ = keychain::delete_token();
            "GitHub authorization failed (401). Please reconnect.".to_string()
        }
        403 | 429 => "GitHub rate limit reached. Try again shortly.".to_string(),
        other => format!("GitHub API error ({other})."),
    }
}

/// Resolve the login for a given token (used to validate + label a connection).
pub fn get_login_with(token: &str) -> Result<String, String> {
    let resp = http_client()?
        .get(format!("{API}/user"))
        .bearer_auth(token)
        .header("Accept", ACCEPT)
        .header("X-GitHub-Api-Version", API_VERSION)
        .send()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(if resp.status().as_u16() == 401 {
            "GitHub rejected the token (401).".to_string()
        } else {
            format!("GitHub API error ({}).", resp.status().as_u16())
        });
    }
    let v: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
    v.get("login")
        .and_then(|l| l.as_str())
        .map(String::from)
        .ok_or_else(|| "No login in the GitHub response.".to_string())
}

/// List the authenticated user's repos (owner + collaborator + org member).
pub fn list_repos() -> Result<Vec<RepoSummary>, String> {
    list_repos_with(&token()?)
}

/// List repos with an explicit token (lets tests avoid the keychain).
pub fn list_repos_with(token: &str) -> Result<Vec<RepoSummary>, String> {
    let client = http_client()?;
    let mut repos = Vec::new();
    for page in 1..=5 {
        let url = format!(
            "{API}/user/repos?per_page=100&sort=full_name&affiliation=owner,collaborator,organization_member&page={page}"
        );
        let resp = client
            .get(url)
            .bearer_auth(&token)
            .header("Accept", ACCEPT)
            .header("X-GitHub-Api-Version", API_VERSION)
            .send()
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(status_error(resp.status()));
        }
        let batch: Vec<RepoJson> = resp.json().map_err(|e| e.to_string())?;
        let full = batch.len() == 100;
        repos.extend(batch.into_iter().map(RepoSummary::from));
        if !full {
            break;
        }
    }
    Ok(repos)
}

/// Create a new repo for the authenticated user (auto-initialized so it has a
/// first commit and can be cloned). Wired into the "create from app" path in T2.
#[allow(dead_code)]
pub fn create_repo(name: &str, private: bool) -> Result<RepoSummary, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Repository name cannot be empty.".into());
    }
    let token = token()?;
    let resp = http_client()?
        .post(format!("{API}/user/repos"))
        .bearer_auth(&token)
        .header("Accept", ACCEPT)
        .header("X-GitHub-Api-Version", API_VERSION)
        .json(&serde_json::json!({ "name": name, "private": private, "auto_init": true }))
        .send()
        .map_err(|e| e.to_string())?;
    if resp.status().as_u16() == 422 {
        return Err(format!(
            "GitHub couldn't create '{name}' — the name may already be taken."
        ));
    }
    if !resp.status().is_success() {
        return Err(status_error(resp.status()));
    }
    let repo: RepoJson = resp.json().map_err(|e| e.to_string())?;
    Ok(repo.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gh_token() -> Option<String> {
        let out = std::process::Command::new("gh")
            .args(["auth", "token"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (!t.is_empty()).then_some(t)
    }

    #[test]
    #[ignore = "hits the real GitHub API with the gh token; run: cargo test -- --ignored"]
    fn real_login_and_list_repos() {
        let token = gh_token().expect("gh auth token");
        let login = get_login_with(&token).unwrap();
        assert!(!login.is_empty(), "login should be non-empty");

        let repos = list_repos_with(&token).unwrap();
        assert!(!repos.is_empty(), "expected some repos");
        assert!(
            repos.iter().any(|r| r.full_name.ends_with("/Review-Helper")),
            "Review-Helper not in the repo list"
        );
    }
}
