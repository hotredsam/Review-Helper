//! Plan commands: analyze a clone (read-only) or kick off a blank project from a
//! description, both into a first persisted plan with streamed progress.

use rusqlite::{params, Connection};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::context::ProjectContext;
use crate::db::Db;
use crate::model::claude::ClaudeCodeProvider;
use crate::model::{ModelEvent, ModelProvider, ModelRequest};
use crate::plan::{parse, prompts, store};
use crate::projects;

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

    let mut req = ModelRequest::planning(prompts::ANALYSIS_USER);
    req.system_append = Some(format!("{}\n\n{}", prompts::ANALYSIS_SYSTEM, context));
    req.cwd = Some(clone_path);

    let app = app.clone();
    std::thread::spawn(move || generate_plan(app, project_id, req));
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
    std::thread::spawn(move || generate_plan(app, project_id, req));
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
fn generate_plan(app: AppHandle, project_id: i64, req: ModelRequest) {
    let emit = |ev: AnalysisEvent| {
        let _ = app.emit("analysis-event", &ev);
    };
    emit(AnalysisEvent::Started { project_id });

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
        Ok(mut conn) => store::save_generated_plan(&mut conn, project_id, &plan),
        Err(e) => Err(e.to_string()),
    };
    match saved {
        Ok(version) => emit(AnalysisEvent::Done {
            project_id,
            version,
            confidence,
            phases,
        }),
        Err(detail) => emit(AnalysisEvent::Failed { project_id, detail }),
    }
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
