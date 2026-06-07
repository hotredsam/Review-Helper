//! Plan commands: analyze a clone (read-only) or kick off a blank project from a
//! description, both into a first persisted plan with streamed progress.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::context::ProjectContext;
use crate::db::Db;
use crate::model::claude::ClaudeCodeProvider;
use crate::model::{ModelEvent, ModelProvider, ModelRequest};
use crate::plan::{ingest, parse, prompts, store};
use crate::projects;

/// Per-project gate serializing plan-version creation, so two concurrent
/// analyze/kickoff/update/rebuild runs can't both read MAX(version) and collide
/// on the UNIQUE(project_id, version) constraint (mirrors CardGate/GrillGate).
#[derive(Default)]
pub struct PlanGate(pub Mutex<HashMap<i64, Arc<Mutex<()>>>>);

/// Streamed plan-generation progress (channel: `analysis-event`).
#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnalysisEvent {
    Started { project_id: i64 },
    Tool { project_id: i64, name: String },
    Done {
        project_id: i64,
        version: i64,
        confidence: String,
        phases: usize,
        source: String,
    },
    Failed { project_id: i64, detail: String },
}

/// Analyze a project's clone (read-only) into a first plan, on a background thread.
#[tauri::command]
pub fn analyze_project(app: AppHandle, db: State<'_, Db>, project_id: i64) -> Result<(), String> {
    let project = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        projects::get(&conn, project_id)?.ok_or("Project not found.")?
    };
    let clone_path = project
        .clone_path
        .ok_or("This project has no local clone to analyze. Clone the repo first.")?;
    if !std::path::Path::new(&clone_path).join(".git").is_dir() {
        return Err("The clone is missing on disk. Refresh the clone, then analyze.".into());
    }
    let context = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        ProjectContext::assemble(&conn, project_id)?.to_prompt()
    };

    // Pre-read existing planning docs so the first plan provably reflects them.
    let docs = ingest::collect_existing_docs(&clone_path);
    let user = if docs.is_empty() {
        prompts::ANALYSIS_USER.to_string()
    } else {
        format!("{docs}\n\n{}", prompts::ANALYSIS_USER)
    };
    let mut req = ModelRequest::planning(user);
    req.system_append = Some(format!("{}\n\n{}", prompts::ANALYSIS_SYSTEM, context));
    req.cwd = Some(clone_path);

    let app = app.clone();
    let report = app.clone();
    crate::util::spawn_guarded(
        move || generate_plan(app, project_id, req, "analyze"),
        move || { let _ = report.emit("analysis-event", &AnalysisEvent::Failed { project_id, detail: "Analysis crashed unexpectedly.".into() }); },
    );
    Ok(())
}

/// Seed a blank project's first plan from a free-text description.
#[tauri::command]
pub fn kickoff_project(
    app: AppHandle,
    db: State<'_, Db>,
    project_id: i64,
    description: String,
) -> Result<(), String> {
    let description = description.trim().to_string();
    if description.is_empty() {
        return Err("Tell me what you're building first.".into());
    }
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        projects::get(&conn, project_id)?.ok_or("Project not found.")?;
        store_kickoff_answer(&conn, project_id, &description)?;
    }

    let mut req = ModelRequest::planning(prompts::kickoff_user(&description));
    req.system_append = Some(prompts::KICKOFF_SYSTEM.to_string());
    // No cwd: there is no repo to read.

    let app = app.clone();
    let report = app.clone();
    crate::util::spawn_guarded(
        move || generate_plan(app, project_id, req, "kickoff"),
        move || { let _ = report.emit("analysis-event", &AnalysisEvent::Failed { project_id, detail: "Plan generation crashed unexpectedly.".into() }); },
    );
    Ok(())
}

/// Record the kickoff description as an answered question so it grounds future
/// model calls and shows up in the project's panes.
fn store_kickoff_answer(conn: &Connection, project_id: i64, description: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO questions (project_id, text, status) VALUES (?1, 'What are you building?', 'answered')",
        params![project_id],
    )
    .map_err(|e| e.to_string())?;
    let qid = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO answers (question_id, project_id, body, source) VALUES (?1, ?2, ?3, 'typed')",
        params![qid, project_id, description],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Run a plan-generation request to completion: stream progress, parse the
/// result, and persist it. Shared by analyze + kickoff.
fn generate_plan(app: AppHandle, project_id: i64, req: ModelRequest, source: &str) {
    let emit = |ev: AnalysisEvent| {
        let _ = app.emit("analysis-event", &ev);
    };
    emit(AnalysisEvent::Started { project_id });

    // Serialize version creation per project (no MAX(version) collision).
    let gate = app.state::<PlanGate>();
    let plock = match gate.0.lock() {
        Ok(mut m) => m.entry(project_id).or_default().clone(),
        Err(e) => return emit(AnalysisEvent::Failed { project_id, detail: e.to_string() }),
    };
    // Recover from poisoning: a prior run that panicked while holding the gate
    // would otherwise lock out all future plan generation for this project. The
    // gate guards only version-creation serialization (its `()` carries no
    // invariant), so reusing the poisoned guard is safe; spawn_guarded is the net.
    let _plan_guard = plock.lock().unwrap_or_else(|e| e.into_inner());

    let mut final_text: Option<String> = None;
    let mut failure: Option<String> = None;
    ClaudeCodeProvider::new().run(&req, &mut |event: ModelEvent| match event {
        ModelEvent::ToolUse { name } => emit(AnalysisEvent::Tool { project_id, name }),
        ModelEvent::Completed { text, .. } => final_text = Some(text),
        ModelEvent::Unavailable { detail, .. } | ModelEvent::Failed { detail } => {
            failure = Some(detail)
        }
        _ => {}
    });

    if let Some(detail) = failure {
        emit(AnalysisEvent::Failed { project_id, detail });
        return;
    }
    let text = match final_text {
        Some(t) => t,
        None => {
            emit(AnalysisEvent::Failed {
                project_id,
                detail: "The model produced no result.".into(),
            });
            return;
        }
    };

    let plan = match parse::parse_plan(&text) {
        Ok(p) => p,
        Err(detail) => {
            emit(AnalysisEvent::Failed { project_id, detail });
            return;
        }
    };
    let confidence = plan.confidence.clone();
    let phases = plan.phases.len();

    let db = app.state::<Db>();
    let saved = match db.0.lock() {
        Ok(mut conn) => commit_fresh(&mut conn, project_id, &plan, source),
        Err(e) => Err(e.to_string()),
    };
    match saved {
        Ok(version) => emit(AnalysisEvent::Done {
            project_id,
            version,
            confidence,
            phases,
            source: source.to_string(),
        }),
        Err(detail) => emit(AnalysisEvent::Failed { project_id, detail }),
    }
}

/// Save a fresh plan version + record its audit entry atomically (analyze /
/// kickoff / rebuild — no status carry).
fn commit_fresh(conn: &mut Connection, project_id: i64, plan: &parse::GeneratedPlan, source: &str) -> Result<i64, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let version = store::save_into_tx(&tx, project_id, plan)?;
    crate::audit::record(&tx, project_id, version, source)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(version)
}

/// Save the merged plan, carry completion forward, mark incorporated features
/// in_plan, and record the audit entry — all in ONE transaction so a failure
/// can't leave a persisted plan with reset progress or a stale inbox.
fn commit_merge(conn: &mut Connection, project_id: i64, plan: &parse::GeneratedPlan, feature_ids: &[i64]) -> Result<i64, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let version = store::save_into_tx(&tx, project_id, plan)?;
    let _warnings = store::carry_into_tx(&tx, project_id, version)?;
    for fid in feature_ids {
        crate::features::set_status(&tx, project_id, *fid, "in_plan")?;
    }
    crate::audit::record(&tx, project_id, version, "update")?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(version)
}

/// Incrementally UPDATE the plan: weave approved answers + pending features into
/// the existing plan as a new version, preserving completed phases (carry_status)
/// and marking the incorporated features in_plan.
#[tauri::command]
pub fn update_plan(app: AppHandle, db: State<'_, Db>, project_id: i64) -> Result<(), String> {
    let (req, feature_ids) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let project = projects::get(&conn, project_id)?.ok_or("Project not found.")?;
        let plan = store::get_plan(&conn, project_id)?
            .ok_or("No plan to update yet — analyze or kick off a plan first.")?;

        let mut summary = String::new();
        if let Some(cs) = plan.current_state.as_deref().filter(|s| !s.trim().is_empty()) {
            summary.push_str(&format!("Current state: {cs}\n\n"));
        }
        summary.push_str("Phases:\n");
        for ph in &plan.phases {
            summary.push_str(&format!("- [{}] {}: {}\n", ph.status, ph.title, ph.goal.as_deref().unwrap_or("")));
        }

        let ctx = ProjectContext::assemble(&conn, project_id)?;
        let answers: Vec<(String, String)> =
            ctx.answers.iter().map(|a| (a.question.clone(), a.answer.clone())).collect();

        let feats: Vec<crate::features::Feature> = crate::features::list(&conn, project_id)?
            .into_iter()
            .filter(|f| f.status == "inbox" || f.status == "triaged")
            .collect();
        let feature_lines: Vec<String> = feats
            .iter()
            .map(|f| match f.detail.as_deref().filter(|d| !d.trim().is_empty()) {
                Some(d) => format!("{} — {}", f.title, d),
                None => f.title.clone(),
            })
            .collect();
        let feature_ids: Vec<i64> = feats.iter().map(|f| f.id).collect();

        if feature_lines.is_empty() && answers.is_empty() {
            return Err("No new ideas or answers to incorporate — add to the inbox or answer questions first.".into());
        }

        let mut req = ModelRequest::planning(prompts::merge_user(&summary, &answers, &feature_lines));
        req.system_append = Some(format!("{}\n\n{}", prompts::MERGE_SYSTEM, ctx.to_prompt()));
        if let Some(cp) = project.clone_path.as_deref() {
            if std::path::Path::new(cp).join(".git").is_dir() {
                req.cwd = Some(cp.to_string());
            }
        }
        (req, feature_ids)
    };

    let app = app.clone();
    let report = app.clone();
    crate::util::spawn_guarded(
        move || run_merge(app, project_id, req, feature_ids),
        move || { let _ = report.emit("analysis-event", &AnalysisEvent::Failed { project_id, detail: "Plan update crashed unexpectedly.".into() }); },
    );
    Ok(())
}

/// Like generate_plan, but after saving the new version it carries completion
/// forward and marks the incorporated features in_plan.
fn run_merge(app: AppHandle, project_id: i64, req: ModelRequest, feature_ids: Vec<i64>) {
    let emit = |ev: AnalysisEvent| {
        let _ = app.emit("analysis-event", &ev);
    };
    emit(AnalysisEvent::Started { project_id });

    // Serialize version creation per project (no MAX(version) collision).
    let gate = app.state::<PlanGate>();
    let plock = match gate.0.lock() {
        Ok(mut m) => m.entry(project_id).or_default().clone(),
        Err(e) => return emit(AnalysisEvent::Failed { project_id, detail: e.to_string() }),
    };
    // Recover from poisoning: a prior run that panicked while holding the gate
    // would otherwise lock out all future plan generation for this project. The
    // gate guards only version-creation serialization (its `()` carries no
    // invariant), so reusing the poisoned guard is safe; spawn_guarded is the net.
    let _plan_guard = plock.lock().unwrap_or_else(|e| e.into_inner());

    let mut final_text: Option<String> = None;
    let mut failure: Option<String> = None;
    ClaudeCodeProvider::new().run(&req, &mut |event: ModelEvent| match event {
        ModelEvent::ToolUse { name } => emit(AnalysisEvent::Tool { project_id, name }),
        ModelEvent::Completed { text, .. } => final_text = Some(text),
        ModelEvent::Unavailable { detail, .. } | ModelEvent::Failed { detail } => failure = Some(detail),
        _ => {}
    });
    if let Some(detail) = failure {
        return emit(AnalysisEvent::Failed { project_id, detail });
    }
    let text = match final_text {
        Some(t) => t,
        None => return emit(AnalysisEvent::Failed { project_id, detail: "The model produced no result.".into() }),
    };
    let plan = match parse::parse_plan(&text) {
        Ok(p) => p,
        Err(detail) => return emit(AnalysisEvent::Failed { project_id, detail }),
    };
    let confidence = plan.confidence.clone();
    let phases = plan.phases.len();

    let db = app.state::<Db>();
    let result = match db.0.lock() {
        Ok(mut conn) => commit_merge(&mut conn, project_id, &plan, &feature_ids),
        Err(e) => Err(e.to_string()),
    };
    match result {
        Ok(version) => emit(AnalysisEvent::Done {
            project_id,
            version,
            confidence,
            phases,
            source: "update".to_string(),
        }),
        Err(detail) => emit(AnalysisEvent::Failed { project_id, detail }),
    }
}

/// Rebuild the plan from scratch (warned in the UI) — fresh analysis for a repo,
/// or a fresh plan from the stored description otherwise. Does NOT carry status
/// forward (that's what "update" is for); records an audit entry source=rebuild.
#[tauri::command]
pub fn rebuild_plan(app: AppHandle, db: State<'_, Db>, project_id: i64) -> Result<(), String> {
    let req = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let project = projects::get(&conn, project_id)?.ok_or("Project not found.")?;
        let context = ProjectContext::assemble(&conn, project_id)?.to_prompt();
        match project.clone_path.as_deref().filter(|cp| std::path::Path::new(cp).join(".git").is_dir()) {
            Some(cp) => {
                let docs = ingest::collect_existing_docs(cp);
                let user = if docs.is_empty() {
                    prompts::ANALYSIS_USER.to_string()
                } else {
                    format!("{docs}\n\n{}", prompts::ANALYSIS_USER)
                };
                let mut req = ModelRequest::planning(user);
                req.system_append = Some(format!("{}\n\n{}", prompts::ANALYSIS_SYSTEM, context));
                req.cwd = Some(cp.to_string());
                req
            }
            None => {
                let desc = stored_description(&conn, project_id)?;
                let mut req = ModelRequest::planning(prompts::kickoff_user(&desc));
                // Include context so a rebuild keeps prior answers/decisions, not blank.
                req.system_append = Some(format!("{}\n\n{}", prompts::KICKOFF_SYSTEM, context));
                req
            }
        }
    };
    let app = app.clone();
    let report = app.clone();
    crate::util::spawn_guarded(
        move || generate_plan(app, project_id, req, "rebuild"),
        move || { let _ = report.emit("analysis-event", &AnalysisEvent::Failed { project_id, detail: "Plan rebuild crashed unexpectedly.".into() }); },
    );
    Ok(())
}

/// The text a description-only project was planned from (latest "what are you
/// building?" answer, else the latest plan's current_state).
fn stored_description(conn: &Connection, project_id: i64) -> Result<String, String> {
    let answer: Option<String> = conn
        .query_row(
            "SELECT a.body FROM answers a JOIN questions q ON a.question_id = q.id \
             WHERE q.project_id = ?1 AND q.text = 'What are you building?' ORDER BY a.id DESC LIMIT 1",
            params![project_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(d) = answer.filter(|s| !s.trim().is_empty()) {
        return Ok(d);
    }
    let state: Option<String> = conn
        .query_row(
            "SELECT current_state FROM plans WHERE project_id = ?1 ORDER BY version DESC LIMIT 1",
            params![project_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();
    state
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Nothing to rebuild from — analyze or describe the project first.".into())
}

#[tauri::command]
pub fn get_plan(db: State<'_, Db>, project_id: i64) -> Result<Option<store::PlanView>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store::get_plan(&conn, project_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;

    #[test]
    fn kickoff_answer_is_stored_and_in_context() {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn.execute("INSERT INTO projects (name, kind) VALUES ('Blank', 'new')", [])
            .unwrap();
        let pid = conn.last_insert_rowid();

        store_kickoff_answer(&conn, pid, "A markdown note CLI").unwrap();
        let ctx = ProjectContext::assemble(&conn, pid).unwrap();
        assert_eq!(ctx.answers.len(), 1);
        assert_eq!(ctx.answers[0].answer, "A markdown note CLI");
        assert!(ctx.to_prompt().contains("What are you building?"));
    }

    #[test]
    #[ignore = "runs a real model analysis ingesting a PLANNING.md; needs auth + uses credits. Run: cargo test -- --ignored"]
    fn real_analysis_reflects_existing_planning_md() {
        let dir = std::env::temp_dir().join(format!("rh-ingest-real-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "# recipes").unwrap();
        std::fs::write(
            dir.join("PLANNING.md"),
            "# Roadmap\nThis project is a recipe-sharing web app.\n\
             Phase 1: user accounts with email login.\n\
             Phase 2: recipe CRUD with photo uploads.\n\
             Phase 3: search recipes by ingredient.",
        )
        .unwrap();

        let docs = ingest::collect_existing_docs(dir.to_str().unwrap());
        assert!(!docs.is_empty(), "ingest should find the PLANNING.md");
        let user = format!("{docs}\n\n{}", prompts::ANALYSIS_USER);
        let mut req = ModelRequest::planning(user);
        req.system_append = Some(prompts::ANALYSIS_SYSTEM.to_string());
        req.cwd = Some(dir.to_string_lossy().to_string());
        req.model = Some("sonnet".into());

        let mut text = None;
        ClaudeCodeProvider::new().run(&req, &mut |e: ModelEvent| {
            if let ModelEvent::Completed { text: t, .. } = e {
                text = Some(t);
            }
        });
        let plan = parse::parse_plan(&text.expect("model returned a result")).expect("plan parses");
        let blob = format!(
            "{} {} {}",
            plan.current_state,
            plan.body_md,
            plan.phases
                .iter()
                .map(|p| format!("{} {}", p.title, p.goal))
                .collect::<Vec<_>>()
                .join(" ")
        )
        .to_lowercase();
        assert!(
            blob.contains("recipe") || blob.contains("ingredient"),
            "plan should reflect the PLANNING.md content, got: {blob}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[ignore = "runs a real model kickoff from a description; needs auth + uses credits. Run: cargo test -- --ignored"]
    fn real_kickoff_yields_a_parseable_plan() {
        let mut req = ModelRequest::planning(prompts::kickoff_user(
            "A macOS menu-bar timer that tracks how long I spend in each app and shows a weekly chart.",
        ));
        req.system_append = Some(prompts::KICKOFF_SYSTEM.to_string());
        req.model = Some("sonnet".into());

        let mut text = None;
        ClaudeCodeProvider::new().run(&req, &mut |e: ModelEvent| {
            if let ModelEvent::Completed { text: t, .. } = e {
                text = Some(t);
            }
        });
        let plan = parse::parse_plan(&text.expect("model returned a result")).expect("plan parses");
        assert!(!plan.current_state.trim().is_empty());
        assert!(!plan.phases.is_empty(), "a described project should yield phases");
    }

    #[test]
    #[ignore = "runs a real model analysis on a tiny temp repo; needs auth + uses credits. Run: cargo test -- --ignored"]
    fn real_analysis_yields_a_parseable_plan() {
        let dir = std::env::temp_dir().join(format!("rh-analyze-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("README.md"),
            "# notecli\nA fast local note-taking CLI that stores notes as Markdown files.",
        )
        .unwrap();
        std::fs::write(
            dir.join("package.json"),
            r#"{"name":"notecli","version":"0.0.1","bin":{"notecli":"index.js"}}"#,
        )
        .unwrap();

        let mut req = ModelRequest::planning(prompts::ANALYSIS_USER);
        req.system_append = Some(prompts::ANALYSIS_SYSTEM.to_string());
        req.cwd = Some(dir.to_string_lossy().to_string());
        req.model = Some("sonnet".into());

        let mut text = None;
        ClaudeCodeProvider::new().run(&req, &mut |e: ModelEvent| {
            if let ModelEvent::Completed { text: t, .. } = e {
                text = Some(t);
            }
        });
        let text = text.expect("model returned a result");
        let plan = parse::parse_plan(&text).expect("plan should parse");
        assert!(!plan.current_state.trim().is_empty());
        assert!(["high", "medium", "low"].contains(&plan.confidence.as_str()));

        std::fs::remove_dir_all(&dir).ok();
    }
}
