//! Plan commands: run read-only analysis of a clone into a first plan (streaming
//! progress to the UI), and read the persisted plan back.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::context::ProjectContext;
use crate::db::Db;
use crate::model::claude::ClaudeCodeProvider;
use crate::model::{ModelEvent, ModelProvider, ModelRequest};
use crate::plan::{parse, prompts, store};
use crate::projects;

/// Streamed analysis progress (channel: `analysis-event`).
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

/// Analyze a project's clone (read-only) into a first plan, on a background
/// thread. Streams `analysis-event`s; persists the plan on success.
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

    let app = app.clone();
    std::thread::spawn(move || run_analysis(app, project_id, clone_path, context));
    Ok(())
}

fn run_analysis(app: AppHandle, project_id: i64, clone_path: String, context: String) {
    let emit = |ev: AnalysisEvent| {
        let _ = app.emit("analysis-event", &ev);
    };
    emit(AnalysisEvent::Started { project_id });

    let mut req = ModelRequest::planning(prompts::ANALYSIS_USER);
    req.system_append = Some(format!("{}\n\n{}", prompts::ANALYSIS_SYSTEM, context));
    req.cwd = Some(clone_path);

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

    let saved = {
        let db = app.state::<Db>();
        let locked = db.0.lock();
        match locked {
            Ok(mut conn) => store::save_generated_plan(&mut conn, project_id, &plan),
            Err(e) => Err(e.to_string()),
        }
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

    #[test]
    #[ignore = "runs a real model analysis on a tiny temp repo; needs auth + uses credits. Run: cargo test -- --ignored"]
    fn real_analysis_yields_a_parseable_plan() {
        // A controlled tiny "repo" the model reads read-only.
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
        req.model = Some("sonnet".into()); // cheaper for the test

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
