//! USER-REQUESTED helper: seed real GitHub repos into the LOCAL app DB to
//! exercise the pipeline end-to-end (clone -> detect-tech cards -> analysis plan
//! -> assessment). Pushes NOTHING to GitHub; only clones + writes locally.
//!
//! Owner/repos are read from env so no personal repo list is committed:
//!   TEST_GITHUB_OWNER=hotredsam \
//!   TEST_GITHUB_REPOS="repo-a,repo-b:main,repo-c:dev" \
//!   cargo test -- --ignored seed_recent_repos --nocapture
//! Each entry is `name` or `name:branch` (branch defaults to `main`).

use rusqlite::{params, Connection};

use crate::model::claude::ClaudeCodeProvider;
use crate::model::{ModelEvent, ModelProvider, ModelRequest};

/// Parse `TEST_GITHUB_OWNER` + `TEST_GITHUB_REPOS`. Returns None (skip) if unset.
fn repos_from_env() -> Option<(String, Vec<(String, String)>)> {
    let owner = std::env::var("TEST_GITHUB_OWNER").ok().filter(|s| !s.trim().is_empty())?;
    let spec = std::env::var("TEST_GITHUB_REPOS").ok().filter(|s| !s.trim().is_empty())?;
    let repos = spec
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|entry| match entry.split_once(':') {
            Some((name, branch)) => (name.trim().to_string(), branch.trim().to_string()),
            None => (entry.to_string(), "main".to_string()),
        })
        .collect();
    Some((owner, repos))
}

fn gh_token() -> String {
    let out = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn run_model(req: &ModelRequest) -> Option<String> {
    let mut text = None;
    let mut failure: Option<String> = None;
    ClaudeCodeProvider::new().run(req, &mut |e: ModelEvent| match e {
        ModelEvent::Completed { text: t, .. } => text = Some(t),
        ModelEvent::Unavailable { detail, .. } | ModelEvent::Failed { detail } => {
            failure = Some(detail)
        }
        _ => {}
    });
    if let Some(detail) = failure {
        eprintln!("  model FAILED: {detail}"); // surface offline/credit/errored runs
    }
    text
}

#[test]
#[ignore = "USER-REQUESTED: clones real repos (TEST_GITHUB_OWNER/TEST_GITHUB_REPOS) + runs analysis+assessment into the LOCAL app DB (sonnet). Pushes nothing. Run: cargo test -- --ignored seed_recent_repos --nocapture"]
fn seed_recent_repos() {
    let Some((owner, repos)) = repos_from_env() else {
        eprintln!(
            "skipped: set TEST_GITHUB_OWNER and TEST_GITHUB_REPOS (e.g. \"repo-a,repo-b:main\") to run."
        );
        return;
    };

    let home = std::env::var("HOME").unwrap();
    let data_dir =
        std::path::PathBuf::from(format!("{home}/Library/Application Support/com.reviewhelper.app"));
    std::fs::create_dir_all(&data_dir).unwrap();
    let mut conn = Connection::open(data_dir.join("review-helper.db")).unwrap();
    crate::db::init_connection(&conn).unwrap();
    let _ = crate::cards::seed(&conn);
    let token = gh_token();
    assert!(!token.is_empty(), "need a gh token to clone repos");

    for (name, branch) in &repos {
        let clone_url = format!("https://github.com/{owner}/{name}.git");
        eprintln!("\n=== {name} ===");

        // Project row (reuse if already attached, so re-runs don't duplicate).
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM projects WHERE github_repo_url = ?1",
                params![clone_url],
                |r| r.get(0),
            )
            .ok();
        let pid = match existing {
            Some(id) => id,
            None => {
                crate::projects::insert_attached(&conn, name, "imported", Some(&clone_url), Some(branch))
                    .unwrap()
                    .id
            }
        };

        // Shallow clone into the app cache (token via askpass, never in argv/config).
        let dest = data_dir.join("clones").join(pid.to_string());
        if let Err(e) = crate::github::clone::clone_or_refresh(&data_dir, &clone_url, &dest, branch, &token) {
            eprintln!("  clone FAILED: {e}");
            continue;
        }
        let dest_str = dest.to_string_lossy().to_string();
        crate::projects::set_clone_path(&conn, pid, &dest_str).unwrap();

        // Detected-tech cards.
        let added = crate::cards::detect_tech_in_clone(&conn, &dest_str).unwrap_or(0);
        eprintln!("  detected-tech cards added: {added}");

        // Analysis -> plan.
        let docs = crate::plan::ingest::collect_existing_docs(&dest_str);
        let context = crate::context::ProjectContext::assemble(&conn, pid)
            .map(|c| c.to_prompt())
            .unwrap_or_default();
        let user = if docs.is_empty() {
            crate::plan::prompts::ANALYSIS_USER.to_string()
        } else {
            format!("{docs}\n\n{}", crate::plan::prompts::ANALYSIS_USER)
        };
        let mut req = ModelRequest::planning(user);
        req.system_append = Some(format!(
            "{}\n\n{}",
            crate::plan::prompts::ANALYSIS_SYSTEM,
            context
        ));
        req.cwd = Some(dest_str.clone());
        req.model = Some("sonnet".into());
        match run_model(&req).and_then(|t| crate::plan::parse::parse_plan(&t).ok()) {
            Some(plan) => {
                let v = crate::plan::store::save_generated_plan(&mut conn, pid, &plan).unwrap();
                eprintln!(
                    "  plan v{v}: {} phases, confidence {}",
                    plan.phases.len(),
                    plan.confidence
                );
            }
            None => eprintln!("  plan: FAILED to generate/parse"),
        }

        // Assessment (scan + scoring).
        match crate::assess::run_scan(&dest_str) {
            Ok(scan) => {
                let mut areq = ModelRequest::planning(crate::assess::assess_user(&scan));
                areq.system_append = Some(crate::assess::ASSESS_SYSTEM.to_string());
                areq.cwd = Some(dest_str.clone());
                areq.model = Some("sonnet".into());
                match run_model(&areq).and_then(|t| crate::assess::parse_assessment(&t).ok()) {
                    Some(a) => {
                        crate::assess::save_assessment(&conn, pid, &a).unwrap();
                        let overall = crate::assess::overall_score(&a, "dimensions_overall", "dimensions");
                        eprintln!("  assessment: overall {overall}");
                    }
                    None => eprintln!("  assessment: FAILED"),
                }
            }
            Err(e) => eprintln!("  scan FAILED: {e}"),
        }
    }

    // Summary.
    let projects: i64 = conn
        .query_row("SELECT count(*) FROM projects", [], |r| r.get(0))
        .unwrap();
    let plans: i64 = conn
        .query_row("SELECT count(*) FROM plans", [], |r| r.get(0))
        .unwrap();
    let assessments: i64 = conn
        .query_row("SELECT count(*) FROM assessments", [], |r| r.get(0))
        .unwrap();
    let cards: i64 = conn
        .query_row("SELECT count(*) FROM learning_cards", [], |r| r.get(0))
        .unwrap();
    let detected: i64 = conn
        .query_row(
            "SELECT count(*) FROM learning_cards WHERE source = 'detected'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    eprintln!(
        "\n=== SUMMARY === projects={projects} plans={plans} assessments={assessments} cards={cards} (detected={detected})"
    );
}
