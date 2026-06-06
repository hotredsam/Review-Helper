//! Shallow-clone cache. Clones attached repos under the app data dir using a
//! GIT_ASKPASS helper so the token never appears in argv, the remote URL, or
//! .git/config. Refresh re-pulls the latest commit while staying shallow.

use std::path::{Path, PathBuf};
use std::process::Command;

const ASKPASS: &str = "git-askpass.sh";

/// Write (idempotently) the askpass helper. It returns `x-access-token` for the
/// username prompt and the token — read from the `GH_TOKEN` env var, never
/// embedded in the file — for the password prompt.
fn ensure_askpass(data_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let path = data_dir.join(ASKPASS);
    let script = "#!/bin/sh\ncase \"$1\" in\n  Username*) printf '%s' 'x-access-token' ;;\n  *) printf '%s' \"$GH_TOKEN\" ;;\nesac\n";
    std::fs::write(&path, script).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    Ok(path)
}

fn run_git(args: &[&str], cwd: Option<&Path>, token: &str, askpass: &Path) -> Result<(), String> {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .env("GIT_ASKPASS", askpass)
        .env("GH_TOKEN", token)
        .env("GIT_TERMINAL_PROMPT", "0");
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        // stderr can include the repo URL but never the token (askpass keeps it out).
        return Err(format!(
            "git {} failed: {}",
            args.first().copied().unwrap_or(""),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

/// Clone the repo shallowly into `dest`, or refresh it if already cloned.
pub fn clone_or_refresh(
    data_dir: &Path,
    clone_url: &str,
    dest: &Path,
    branch: &str,
    token: &str,
) -> Result<(), String> {
    let askpass = ensure_askpass(data_dir)?;
    if dest.join(".git").is_dir() {
        return refresh(dest, branch, token, &askpass);
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let dest_str = dest.to_str().ok_or("clone path is not valid UTF-8")?;
    run_git(
        &["clone", "--depth", "1", "--single-branch", clone_url, dest_str],
        None,
        token,
        &askpass,
    )
}

/// Re-pull the latest commit on `branch`, staying shallow and discarding any
/// local drift (the cache is read-only working state).
pub fn refresh(dest: &Path, branch: &str, token: &str, askpass: &Path) -> Result<(), String> {
    run_git(&["fetch", "--depth", "1", "origin", branch], Some(dest), token, askpass)?;
    run_git(&["reset", "--hard", "FETCH_HEAD"], Some(dest), token, askpass)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn askpass_helper_never_embeds_the_token() {
        let dir = std::env::temp_dir().join(format!("rh-askpass-{}", std::process::id()));
        let path = ensure_askpass(&dir).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("x-access-token"));
        assert!(content.contains("$GH_TOKEN")); // reads from env
        assert!(!content.contains("gho_")); // no literal token in the file
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[ignore = "clones a private repo + adds a commit to the throwaway smoketest repo; run: cargo test -- --ignored"]
    fn real_clone_and_refresh() {
        use crate::github::http_client;

        fn gh_token() -> String {
            let out = std::process::Command::new("gh")
                .args(["auth", "token"])
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        fn head(dest: &Path) -> String {
            let out = std::process::Command::new("git")
                .args(["-C", dest.to_str().unwrap(), "rev-parse", "HEAD"])
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }

        let token = gh_token();
        let base = std::env::temp_dir().join(format!("rh-clone-{}", std::process::id()));
        let dest = base.join("repo");
        let url = "https://github.com/hotredsam/rh-phase3-smoketest.git";

        clone_or_refresh(&base, url, &dest, "main", &token).unwrap();
        assert!(dest.join(".git").is_dir(), "clone should create a .git dir");
        // token must not leak into the saved remote config
        let cfg = std::fs::read_to_string(dest.join(".git").join("config")).unwrap();
        assert!(!cfg.contains(&token), "token leaked into .git/config");
        let before = head(&dest);

        // Create a new commit on the remote via the contents API.
        let path = format!("refresh-probe-{}.txt", std::process::id());
        let put = http_client()
            .unwrap()
            .put(format!(
                "https://api.github.com/repos/hotredsam/rh-phase3-smoketest/contents/{path}"
            ))
            .bearer_auth(&token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2026-03-10")
            .json(&serde_json::json!({"message":"refresh probe","content":"eA==","branch":"main"}))
            .send()
            .unwrap();
        assert!(put.status().is_success(), "PUT contents failed: {}", put.status());

        clone_or_refresh(&base, url, &dest, "main", &token).unwrap(); // refresh path
        let after = head(&dest);
        assert_ne!(before, after, "refresh should pull the new commit");

        std::fs::remove_dir_all(&base).ok();
    }
}
