//! GitHub sync-out — render the planning package (the `.planning/` docs + a
//! per-project CLAUDE.md) from the current plan/decisions/stack, then push it
//! and the per-phase issues behind a confirmed preview (idempotent). T1 is the
//! pure package renderer; T2–T4 add the GitHub writes.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub mod commands;
pub mod issues;

use crate::plan::store::{get_plan, PhaseView, PlanView};
use issues::{reconcile, IssueAction, IssueRef, PhasePlan};

/// One file in the planning package: a repo-relative path + its full contents.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PackageFile {
    pub path: String,
    pub content: String,
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.extend(ch.to_lowercase());
            prev_dash = false;
        } else if !out.is_empty() && !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let t = out.trim_matches('-');
    if t.is_empty() { "phase".into() } else { t.chars().take(40).collect() }
}

fn marker(status: &str) -> &'static str {
    match status {
        "done" => "[x]",
        "in_progress" => "[~]",
        _ => "[ ]",
    }
}

fn phase_filename(i: usize, title: &str) -> String {
    format!(".planning/phases/phase-{:02}-{}.md", i + 1, slug(title))
}

/// The package files for the current plan. Returns an empty Vec if there's no
/// plan yet (nothing to sync).
pub fn package(conn: &Connection, project_id: i64) -> Result<Vec<PackageFile>, String> {
    let project_name: String = conn
        .query_row("SELECT name FROM projects WHERE id = ?1", [project_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let plan = match get_plan(conn, project_id)? {
        Some(p) => p,
        None => return Ok(vec![]),
    };
    Ok(render_package(&project_name, &plan))
}

/// Render the package from an in-memory plan (pure — easy to test).
pub fn render_package(project_name: &str, plan: &PlanView) -> Vec<PackageFile> {
    let mut files = vec![PackageFile {
        path: ".planning/PLAN.md".into(),
        content: render_plan_md(project_name, plan),
    }];
    for (i, ph) in plan.phases.iter().enumerate() {
        files.push(PackageFile {
            path: phase_filename(i, &ph.title),
            content: render_phase_md(i, ph),
        });
    }
    files.push(PackageFile {
        path: "CLAUDE.md".into(),
        content: render_claude_md(project_name, plan),
    });
    files
}

fn render_plan_md(project_name: &str, plan: &PlanView) -> String {
    let resume = plan
        .phases
        .iter()
        .find(|p| p.status != "done")
        .map(|p| format!("Resume at “{}”.", p.title))
        .unwrap_or_else(|| "All phases complete.".into());

    let mut s = format!("# Plan — {project_name}\n\n");
    s.push_str(&format!("> {resume} Plan version {}. Managed by Review Helper.\n\n", plan.version));
    if let Some(cs) = plan.current_state.as_deref().filter(|c| !c.trim().is_empty()) {
        s.push_str(&format!("## Current state\n\n{}\n\n", cs.trim()));
    }
    if let Some(body) = plan.body_md.as_deref().filter(|b| !b.trim().is_empty()) {
        s.push_str(&format!("## Overview\n\n{}\n\n", body.trim()));
    }
    s.push_str("## Phases\n\n| # | Phase | Status |\n|---|-------|--------|\n");
    for (i, ph) in plan.phases.iter().enumerate() {
        s.push_str(&format!("| {} | {} | {} {} |\n", i + 1, ph.title, marker(&ph.status), ph.status));
    }
    if !plan.decisions.is_empty() {
        s.push_str("\n## Decisions\n\n");
        for d in &plan.decisions {
            s.push_str(&format!("- **{}**: {}", d.topic, d.choice));
            if let Some(r) = d.rationale.as_deref().filter(|r| !r.is_empty()) {
                s.push_str(&format!(" — {r}"));
            }
            s.push('\n');
        }
    }
    let stack: Vec<&crate::plan::store::StackView> = plan.stack.iter().filter(|s| s.choice.is_some()).collect();
    if !stack.is_empty() {
        s.push_str("\n## Stack\n\n");
        for st in stack {
            s.push_str(&format!("- {}: {}\n", st.pane, st.choice.as_deref().unwrap_or("")));
        }
    }
    s
}

fn render_phase_md(i: usize, ph: &PhaseView) -> String {
    let mut s = format!("# Phase {}: {}\nStatus: {}\n", i + 1, ph.title, ph.status);
    if let Some(g) = ph.goal.as_deref().filter(|g| !g.trim().is_empty()) {
        s.push_str(&format!("\n{}\n", g.trim()));
    }
    s.push_str("\n## Tasks\n");
    for t in &ph.tasks {
        s.push_str(&format!("- {} {}", marker(&t.status), t.title));
        if let Some(v) = t.verification.as_deref().filter(|v| !v.trim().is_empty()) {
            s.push_str(&format!(" — Done when: {}", v.trim()));
        }
        s.push('\n');
    }
    s
}

/// Parse `owner` + `repo` from a GitHub https/ssh URL (or `owner/repo`).
pub fn parse_owner_repo(url: &str) -> Option<(String, String)> {
    let s = url.trim().trim_end_matches(".git");
    let rest = match s.split_once("github.com") {
        Some((_, r)) => r.trim_start_matches([':', '/']),
        None => s,
    };
    let mut parts = rest.split('/').filter(|p| !p.is_empty());
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    (!owner.is_empty() && !repo.is_empty()).then_some((owner, repo))
}

fn owner_repo(conn: &Connection, project_id: i64) -> Result<(String, String, String), String> {
    let project = crate::projects::get(conn, project_id)?.ok_or("Project not found.")?;
    let url = project.github_repo_url.ok_or("Connect a GitHub repo first.")?;
    let (owner, repo) = parse_owner_repo(&url).ok_or("Couldn't parse the repo from its URL.")?;
    let default_branch = project.default_branch.unwrap_or_else(|| "main".into());
    Ok((owner, repo, default_branch))
}

/// Write the package files to a branch (idempotent — update in place by sha).
fn push_files(owner: &str, repo: &str, branch: &str, files: &[PackageFile]) -> Result<(), String> {
    for f in files {
        let sha = crate::github::api::file_sha(owner, repo, &f.path, branch)?;
        crate::github::api::put_file(
            owner,
            repo,
            &f.path,
            &f.content,
            &format!("Review Helper: sync {}", f.path),
            branch,
            sha.as_deref(),
        )?;
    }
    Ok(())
}

/// Push the planning package to the `planning` branch (created from the default
/// branch head if missing). Idempotent.
pub fn push_planning_branch(conn: &Connection, project_id: i64) -> Result<usize, String> {
    let (owner, repo, default_branch) = owner_repo(conn, project_id)?;
    let files = package(conn, project_id)?;
    if files.is_empty() {
        return Err("No plan to sync yet — analyze or kick off a plan first.".into());
    }
    let head = crate::github::api::branch_head_sha(&owner, &repo, &default_branch)?;
    crate::github::api::ensure_branch(&owner, &repo, "planning", &head)?;
    push_files(&owner, &repo, "planning", &files)?;
    Ok(files.len())
}

fn latest_version(conn: &Connection, project_id: i64) -> Result<Option<i64>, String> {
    conn.query_row("SELECT MAX(version) FROM plans WHERE project_id = ?1", [project_id], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())
        .map(Option::flatten)
}

/// The phases of the latest plan version as PhasePlans (for issue sync),
/// including any recorded issue number so matching survives renames.
fn phase_plans(conn: &Connection, project_id: i64) -> Result<Vec<PhasePlan>, String> {
    let Some(version) = latest_version(conn, project_id)? else { return Ok(vec![]) };
    let rows: Vec<(i64, String, String, Option<String>, String, Option<i64>)> = {
        let mut stmt = conn
            .prepare("SELECT id, marker, title, goal, status, github_issue_number FROM phases WHERE project_id = ?1 AND plan_version = ?2 ORDER BY idx")
            .map_err(|e| e.to_string())?;
        stmt.query_map(params![project_id, version], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
        })
        .and_then(Iterator::collect)
        .map_err(|e| e.to_string())?
    };
    let mut out = Vec::new();
    for (id, marker, title, goal, status, issue) in rows {
        let mut ts = conn
            .prepare("SELECT title, status FROM tasks WHERE phase_id = ?1 ORDER BY idx")
            .map_err(|e| e.to_string())?;
        let tasks: Vec<(String, String)> = ts
            .query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))
            .and_then(Iterator::collect)
            .map_err(|e| e.to_string())?;
        out.push(PhasePlan { marker, title, goal, status, tasks, issue_number: issue.map(|n| n as u64) });
    }
    Ok(out)
}

/// Everything a push-to-main would change — issue actions AND file deletions —
/// so the user previews ALL destructive operations before confirming.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncPreview {
    pub issue_actions: Vec<IssueAction>,
    pub file_deletions: Vec<String>,
}

/// The outcome of an applied sync (partial-success aware).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SyncResult {
    pub files_pushed: usize,
    pub issues_applied: usize,
    pub files_deleted: usize,
    pub failures: Vec<String>,
}

/// Read-only preview of the push-to-main: issue reconciliation + the stale
/// phase docs that would be pruned. Nothing is written.
pub fn preview_main_sync(conn: &Connection, project_id: i64) -> Result<SyncPreview, String> {
    let (owner, repo, default_branch) = owner_repo(conn, project_id)?;
    let existing: Vec<IssueRef> = crate::github::api::list_issues(&owner, &repo)?
        .into_iter()
        .map(|g| IssueRef { number: g.number, title: g.title, body: g.body, state: g.state, labels: g.labels })
        .collect();
    let issue_actions = reconcile(&existing, &phase_plans(conn, project_id)?);

    let files = package(conn, project_id)?;
    let keep: std::collections::HashSet<&str> = files.iter().map(|f| f.path.as_str()).collect();
    let file_deletions: Vec<String> = crate::github::api::list_dir(&owner, &repo, ".planning/phases", &default_branch)?
        .into_iter()
        .map(|(p, _)| p)
        .filter(|p| !keep.contains(p.as_str()))
        .collect();
    Ok(SyncPreview { issue_actions, file_deletions })
}

/// Apply the CONFIRMED preview: push the package, replay the exact issue actions
/// the user saw, and delete exactly the files shown. Re-runs/partials are safe
/// (idempotent); failures are collected, not silently swallowed.
pub fn apply_main_sync(conn: &mut Connection, project_id: i64, preview: SyncPreview) -> Result<SyncResult, String> {
    let (owner, repo, default_branch) = owner_repo(conn, project_id)?;
    let version = latest_version(conn, project_id)?.ok_or("No plan to sync.")?;
    let files = package(conn, project_id)?;
    if files.is_empty() {
        return Err("No plan to sync yet — analyze or kick off a plan first.".into());
    }
    let mut failures = Vec::new();

    // 1. Push the package docs (create/update — non-destructive).
    let mut files_pushed = 0;
    for f in &files {
        let sha = crate::github::api::file_sha(&owner, &repo, &f.path, &default_branch)?;
        match crate::github::api::put_file(&owner, &repo, &f.path, &f.content, &format!("Review Helper: sync {}", f.path), &default_branch, sha.as_deref()) {
            Ok(()) => files_pushed += 1,
            Err(e) => failures.push(format!("push {}: {e}", f.path)),
        }
    }

    // 2. Replay the confirmed issue actions; record numbers atomically after.
    let mut recorded: Vec<(String, u64)> = Vec::new();
    let mut issues_applied = 0;
    for a in &preview.issue_actions {
        match a {
            IssueAction::Create { marker, title, body, state, labels } => {
                let lab: Vec<&str> = labels.iter().map(String::as_str).collect();
                match crate::github::api::create_issue(&owner, &repo, title, body, &lab) {
                    Ok(num) => {
                        recorded.push((marker.clone(), num)); // record even if the close below fails
                        if state == "closed" {
                            if let Err(e) = crate::github::api::close_issue(&owner, &repo, num) {
                                failures.push(format!("close new #{num}: {e}"));
                            }
                        }
                        issues_applied += 1;
                    }
                    Err(e) => failures.push(format!("create '{title}': {e}")),
                }
            }
            IssueAction::Update { number, marker, title, body, state, labels } => {
                let lab: Vec<&str> = labels.iter().map(String::as_str).collect();
                match crate::github::api::update_issue(&owner, &repo, *number, title, body, state, &lab) {
                    Ok(()) => { recorded.push((marker.clone(), *number)); issues_applied += 1; }
                    Err(e) => failures.push(format!("update #{number}: {e}")),
                }
            }
            IssueAction::Close { number, .. } => match crate::github::api::close_issue(&owner, &repo, *number) {
                Ok(()) => issues_applied += 1,
                Err(e) => failures.push(format!("close #{number}: {e}")),
            },
        }
    }
    if !recorded.is_empty() {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        for (marker, number) in &recorded {
            tx.execute(
                "UPDATE phases SET github_issue_number = ?1 WHERE project_id = ?2 AND plan_version = ?3 AND marker = ?4",
                params![*number as i64, project_id, version, marker],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
    }

    // 3. Delete exactly the previewed-and-confirmed stale files.
    let mut files_deleted = 0;
    for path in &preview.file_deletions {
        match crate::github::api::file_sha(&owner, &repo, path, &default_branch)? {
            Some(sha) => match crate::github::api::delete_file(&owner, &repo, path, &sha, &format!("Review Helper: remove stale {path}"), &default_branch) {
                Ok(()) => files_deleted += 1,
                Err(e) => failures.push(format!("delete {path}: {e}")),
            },
            None => files_deleted += 1, // already gone
        }
    }

    Ok(SyncResult { files_pushed, issues_applied, files_deleted, failures })
}

fn render_claude_md(project_name: &str, plan: &PlanView) -> String {
    let mut s = format!(
        "# CLAUDE.md — {project_name}\n\nStanding rules for building this project. Generated by Review Helper from the plan; re-read before each task.\n\n"
    );
    s.push_str("## How to work\n\n- Work one phase at a time, in order; see `.planning/PLAN.md` and the phase files.\n- Tick a task only when its \"Done when\" check passes. Keep commits atomic.\n- Small, single-responsibility files; handle the unhappy paths as you build.\n\n");
    let stack: Vec<&crate::plan::store::StackView> = plan.stack.iter().filter(|s| s.choice.is_some()).collect();
    if !stack.is_empty() {
        s.push_str("## Stack\n\n");
        for st in stack {
            s.push_str(&format!("- {}: {}\n", st.pane, st.choice.as_deref().unwrap_or("")));
        }
        s.push('\n');
    }
    if !plan.decisions.is_empty() {
        s.push_str("## Key decisions\n\n");
        for d in &plan.decisions {
            s.push_str(&format!("- {}: {}", d.topic, d.choice));
            if let Some(r) = d.rationale.as_deref().filter(|r| !r.is_empty()) {
                s.push_str(&format!(" — {r}"));
            }
            s.push('\n');
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;
    use crate::plan::parse::{GenDecision, GenPhase, GenStack, GenTask, GeneratedPlan};
    use crate::plan::store::save_generated_plan;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn
    }

    #[test]
    fn package_reflects_plan_and_carries_status_markers() {
        let mut conn = db();
        conn.execute("INSERT INTO projects (name, kind) VALUES ('Demo','new')", []).unwrap();
        let pid = conn.last_insert_rowid();
        let plan = GeneratedPlan {
            current_state: "Early scaffold.".into(),
            body_md: "## Arc".into(),
            confidence: "low".into(),
            notes: "".into(),
            phases: vec![
                GenPhase { title: "Setup".into(), goal: "Runs".into(), tasks: vec![GenTask { title: "Init".into(), body: "".into(), verification: "it runs".into() }] },
                GenPhase { title: "Build".into(), goal: "Core".into(), tasks: vec![GenTask { title: "Core".into(), body: "".into(), verification: "works".into() }] },
            ],
            decisions: vec![GenDecision { topic: "DB".into(), choice: "SQLite".into(), rationale: "simple".into(), alternatives: "".into(), consequences: "".into() }],
            stack: GenStack { frontend: Some("React".into()), backend: None, database: Some("SQLite".into()), deployment: None, pipes: None },
        };
        let v = save_generated_plan(&mut conn, pid, &plan).unwrap();
        // Complete the first phase so the package shows a status marker + resume.
        conn.execute("UPDATE phases SET status='done' WHERE project_id=?1 AND plan_version=?2 AND title='Setup'", rusqlite::params![pid, v]).unwrap();

        let files = package(&conn, pid).unwrap();
        let plan_md = &files.iter().find(|f| f.path == ".planning/PLAN.md").unwrap().content;
        assert!(plan_md.contains("[x] done"), "done marker present");
        assert!(plan_md.contains("[ ] not_started"), "not-started marker present");
        assert!(plan_md.contains("Resume at “Build”."), "resume points to first not-done phase");
        assert!(plan_md.contains("DB") && plan_md.contains("SQLite"));

        assert!(files.iter().any(|f| f.path == ".planning/phases/phase-01-setup.md"));
        assert!(files.iter().any(|f| f.path == ".planning/phases/phase-02-build.md"));
        let claude = &files.iter().find(|f| f.path == "CLAUDE.md").unwrap().content;
        assert!(claude.contains("Stack") && claude.contains("React"));
    }

    #[test]
    fn parses_owner_repo_from_various_urls() {
        assert_eq!(parse_owner_repo("https://github.com/hotredsam/Review-Helper.git"), Some(("hotredsam".into(), "Review-Helper".into())));
        assert_eq!(parse_owner_repo("https://github.com/o/r"), Some(("o".into(), "r".into())));
        assert_eq!(parse_owner_repo("git@github.com:o/r.git"), Some(("o".into(), "r".into())));
        assert_eq!(parse_owner_repo("o/r"), Some(("o".into(), "r".into())));
        assert_eq!(parse_owner_repo("not a url"), None);
    }

    #[test]
    fn no_plan_yields_empty_package() {
        let conn = db();
        conn.execute("INSERT INTO projects (name, kind) VALUES ('Empty','new')", []).unwrap();
        let pid = conn.last_insert_rowid();
        assert!(package(&conn, pid).unwrap().is_empty());
    }
}
