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

/// Fetch one repo's metadata (validates existence/access; used by link-by-URL).
pub fn get_repo(owner: &str, repo: &str) -> Result<RepoSummary, String> {
    let resp = http_client()?
        .get(format!("{API}/repos/{owner}/{repo}"))
        .bearer_auth(token()?)
        .header("Accept", ACCEPT)
        .header("X-GitHub-Api-Version", API_VERSION)
        .send()
        .map_err(|e| e.to_string())?;
    if resp.status().as_u16() == 404 {
        return Err(format!(
            "Repository {owner}/{repo} not found, or you don't have access."
        ));
    }
    if !resp.status().is_success() {
        return Err(status_error(resp.status()));
    }
    let repo: RepoJson = resp.json().map_err(|e| e.to_string())?;
    Ok(repo.into())
}

/// Create a new repo for the authenticated user (auto-initialized so it has a
/// first commit and can be cloned).
pub fn create_repo(name: &str, private: bool) -> Result<RepoSummary, String> {
    create_repo_with(&token()?, name, private)
}

/// Create a repo with an explicit token (lets tests/verification avoid the keychain).
pub fn create_repo_with(token: &str, name: &str, private: bool) -> Result<RepoSummary, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Repository name cannot be empty.".into());
    }
    let resp = http_client()?
        .post(format!("{API}/user/repos"))
        .bearer_auth(token)
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

// ---- Write primitives for sync-out (Phase 11) ----

/// Standard base64 (RFC 4648) — the Contents API wants base64 content, and a
/// 15-line encoder beats pulling in a dependency for it.
pub(crate) fn b64encode(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { T[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
    }
    out
}

/// Percent-encode a repo path, preserving `/` separators. Today's generated
/// paths are slug-safe, but this future-proofs against spaces/#/?/unicode.
fn enc_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.bytes() {
        match b {
            b'/' | b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn req_get(url: String) -> Result<reqwest::blocking::Response, String> {
    http_client()?
        .get(url)
        .bearer_auth(token()?)
        .header("Accept", ACCEPT)
        .header("X-GitHub-Api-Version", API_VERSION)
        .send()
        .map_err(|e| e.to_string())
}

/// The HEAD commit sha of a branch.
pub fn branch_head_sha(owner: &str, repo: &str, branch: &str) -> Result<String, String> {
    let resp = req_get(format!("{API}/repos/{owner}/{repo}/git/ref/heads/{branch}"))?;
    if !resp.status().is_success() {
        return Err(status_error(resp.status()));
    }
    let v: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
    v.get("object")
        .and_then(|o| o.get("sha"))
        .and_then(serde_json::Value::as_str)
        .map(String::from)
        .ok_or_else(|| "Could not read the branch head.".into())
}

/// Create a branch ref at `from_sha` if it doesn't already exist (idempotent).
pub fn ensure_branch(owner: &str, repo: &str, branch: &str, from_sha: &str) -> Result<(), String> {
    let resp = http_client()?
        .post(format!("{API}/repos/{owner}/{repo}/git/refs"))
        .bearer_auth(token()?)
        .header("Accept", ACCEPT)
        .header("X-GitHub-Api-Version", API_VERSION)
        .json(&serde_json::json!({ "ref": format!("refs/heads/{branch}"), "sha": from_sha }))
        .send()
        .map_err(|e| e.to_string())?;
    // 201 created, 422 already exists — both fine.
    if resp.status().is_success() || resp.status().as_u16() == 422 {
        Ok(())
    } else {
        Err(status_error(resp.status()))
    }
}

/// The blob sha of a file on a branch, or None if it doesn't exist.
pub fn file_sha(owner: &str, repo: &str, path: &str, branch: &str) -> Result<Option<String>, String> {
    let resp = req_get(format!("{API}/repos/{owner}/{repo}/contents/{}?ref={branch}", enc_path(path)))?;
    if resp.status().as_u16() == 404 {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Err(status_error(resp.status()));
    }
    let v: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
    Ok(v.get("sha").and_then(serde_json::Value::as_str).map(String::from))
}

/// Create or update a file on a branch (Contents API). `sha` is required to
/// update an existing file.
pub fn put_file(
    owner: &str,
    repo: &str,
    path: &str,
    content: &str,
    message: &str,
    branch: &str,
    sha: Option<&str>,
) -> Result<(), String> {
    let mut body = serde_json::json!({
        "message": message,
        "content": b64encode(content.as_bytes()),
        "branch": branch,
    });
    if let Some(s) = sha {
        body["sha"] = serde_json::json!(s);
    }
    let resp = http_client()?
        .put(format!("{API}/repos/{owner}/{repo}/contents/{}", enc_path(path)))
        .bearer_auth(token()?)
        .header("Accept", ACCEPT)
        .header("X-GitHub-Api-Version", API_VERSION)
        .json(&body)
        .send()
        .map_err(|e| e.to_string())?;
    if resp.status().as_u16() == 409 {
        return Err(format!("'{path}' was modified on GitHub since the preview — re-preview and try again."));
    }
    if !resp.status().is_success() {
        return Err(status_error(resp.status()));
    }
    Ok(())
}

/// A GitHub issue (subset) for reconciliation.
#[derive(Debug, Clone)]
pub struct GhIssue {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub state: String,
    pub labels: Vec<String>,
}

/// List a repo's issues (open + closed), excluding pull requests. Paginated so
/// repos with >100 issues don't hide owned issues (which would create dupes).
pub fn list_issues(owner: &str, repo: &str) -> Result<Vec<GhIssue>, String> {
    let mut all = Vec::new();
    for page in 1..=50u32 {
        let resp = req_get(format!("{API}/repos/{owner}/{repo}/issues?state=all&per_page=100&page={page}"))?;
        if !resp.status().is_success() {
            return Err(status_error(resp.status()));
        }
        let arr: Vec<serde_json::Value> = resp.json().map_err(|e| e.to_string())?;
        let n = arr.len();
        for v in arr {
            if v.get("pull_request").is_some() {
                continue; // the issues endpoint includes PRs
            }
            let Some(number) = v.get("number").and_then(serde_json::Value::as_u64) else { continue };
            all.push(GhIssue {
                number,
                title: v.get("title").and_then(serde_json::Value::as_str).unwrap_or("").to_string(),
                body: v.get("body").and_then(serde_json::Value::as_str).unwrap_or("").to_string(),
                state: v.get("state").and_then(serde_json::Value::as_str).unwrap_or("open").to_string(),
                labels: v
                    .get("labels")
                    .and_then(serde_json::Value::as_array)
                    .map(|ls| ls.iter().filter_map(|l| l.get("name").and_then(serde_json::Value::as_str).map(String::from)).collect())
                    .unwrap_or_default(),
            });
        }
        if n < 100 {
            break;
        }
    }
    Ok(all)
}

/// Create an issue. Returns its number.
pub fn create_issue(owner: &str, repo: &str, title: &str, body: &str, labels: &[&str]) -> Result<u64, String> {
    let resp = http_client()?
        .post(format!("{API}/repos/{owner}/{repo}/issues"))
        .bearer_auth(token()?)
        .header("Accept", ACCEPT)
        .header("X-GitHub-Api-Version", API_VERSION)
        .json(&serde_json::json!({ "title": title, "body": body, "labels": labels }))
        .send()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(status_error(resp.status()));
    }
    let v: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
    v.get("number").and_then(serde_json::Value::as_u64).ok_or_else(|| "Issue created but no number returned.".into())
}

/// Update an issue's title/body/state/labels (PATCH).
pub fn update_issue(
    owner: &str,
    repo: &str,
    number: u64,
    title: &str,
    body: &str,
    state: &str,
    labels: &[&str],
) -> Result<(), String> {
    let resp = http_client()?
        .patch(format!("{API}/repos/{owner}/{repo}/issues/{number}"))
        .bearer_auth(token()?)
        .header("Accept", ACCEPT)
        .header("X-GitHub-Api-Version", API_VERSION)
        .json(&serde_json::json!({ "title": title, "body": body, "state": state, "labels": labels }))
        .send()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(status_error(resp.status()));
    }
    Ok(())
}

/// Close an issue without touching its title/body (state only).
pub fn close_issue(owner: &str, repo: &str, number: u64) -> Result<(), String> {
    let resp = http_client()?
        .patch(format!("{API}/repos/{owner}/{repo}/issues/{number}"))
        .bearer_auth(token()?)
        .header("Accept", ACCEPT)
        .header("X-GitHub-Api-Version", API_VERSION)
        .json(&serde_json::json!({ "state": "closed" }))
        .send()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(status_error(resp.status()));
    }
    Ok(())
}

/// List a directory's files on a branch: (path, blob sha). Empty if the dir
/// doesn't exist.
pub fn list_dir(owner: &str, repo: &str, path: &str, branch: &str) -> Result<Vec<(String, String)>, String> {
    let resp = req_get(format!("{API}/repos/{owner}/{repo}/contents/{}?ref={branch}", enc_path(path)))?;
    if resp.status().as_u16() == 404 {
        return Ok(vec![]);
    }
    if !resp.status().is_success() {
        return Err(status_error(resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().map_err(|e| e.to_string())?;
    Ok(arr
        .into_iter()
        .filter(|v| v.get("type").and_then(serde_json::Value::as_str) == Some("file"))
        .filter_map(|v| {
            Some((
                v.get("path")?.as_str()?.to_string(),
                v.get("sha")?.as_str()?.to_string(),
            ))
        })
        .collect())
}

/// Delete a file on a branch (Contents API DELETE).
pub fn delete_file(owner: &str, repo: &str, path: &str, sha: &str, message: &str, branch: &str) -> Result<(), String> {
    let resp = http_client()?
        .delete(format!("{API}/repos/{owner}/{repo}/contents/{}", enc_path(path)))
        .bearer_auth(token()?)
        .header("Accept", ACCEPT)
        .header("X-GitHub-Api-Version", API_VERSION)
        .json(&serde_json::json!({ "message": message, "sha": sha, "branch": branch }))
        .send()
        .map_err(|e| e.to_string())?;
    // 404 = already gone — treat as success so a re-run can make progress.
    if resp.status().as_u16() != 404 && !resp.status().is_success() {
        return Err(status_error(resp.status()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b64encode_matches_rfc4648() {
        assert_eq!(b64encode(b""), "");
        assert_eq!(b64encode(b"f"), "Zg==");
        assert_eq!(b64encode(b"fo"), "Zm8=");
        assert_eq!(b64encode(b"foo"), "Zm9v");
        assert_eq!(b64encode(b"hello"), "aGVsbG8=");
    }

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

    #[test]
    #[ignore = "creates a REAL private repo on GitHub (user-authorized throwaway); run: cargo test -- --ignored"]
    fn real_create_private_repo() {
        let token = gh_token().expect("gh auth token");
        let repo = create_repo_with(&token, "rh-phase3-smoketest", true).unwrap();
        assert!(repo.clone_url.contains("rh-phase3-smoketest"));
        assert!(!repo.default_branch.is_empty(), "a fresh auto-init repo has a default branch");
        eprintln!(
            "CREATED {} (private, branch {}) -> {}",
            repo.full_name, repo.default_branch, repo.clone_url
        );
    }
}
