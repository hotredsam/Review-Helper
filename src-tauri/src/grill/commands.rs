//! Grill commands: generate a batch of repo-specific questions on a background
//! thread (streaming progress through events), and read questions back.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use super::{
    grill_user, list_questions, parse_questions, save_questions, select_topics, Question, GRILL_SYSTEM,
};
use crate::context::ProjectContext;
use crate::db::Db;
use crate::model::claude::ClaudeCodeProvider;
use crate::model::{ModelEvent, ModelProvider, ModelRequest};
use crate::projects;

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GrillEvent {
    Started { project_id: i64 },
    Tool { project_id: i64, name: String },
    Done { project_id: i64, added: usize },
    Failed { project_id: i64, detail: String },
}

#[tauri::command]
pub fn grill_generate(
    app: AppHandle,
    db: State<'_, Db>,
    project_id: i64,
    depth: i64,
) -> Result<(), String> {
    // Validate the project exists before spawning; everything heavy is off-thread.
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        projects::get(&conn, project_id)?.ok_or("Project not found.")?;
    }
    let app = app.clone();
    std::thread::spawn(move || run_grill(app, project_id, depth.clamp(1, 5)));
    Ok(())
}

fn run_grill(app: AppHandle, project_id: i64, depth: i64) {
    let emit = |ev: GrillEvent| {
        let _ = app.emit("grill-event", &ev);
    };
    emit(GrillEvent::Started { project_id });
    let db = app.state::<Db>();

    // Pick uncovered topics + assemble grounding context under one lock, then
    // release it before the (slow) model call.
    let (topics, context, clone_path) = {
        let conn = match db.0.lock() {
            Ok(c) => c,
            Err(e) => return emit(GrillEvent::Failed { project_id, detail: e.to_string() }),
        };
        let topics = match select_topics(&conn, project_id, depth) {
            Ok(t) => t,
            Err(detail) => return emit(GrillEvent::Failed { project_id, detail }),
        };
        if topics.is_empty() {
            // Target already met — nothing to ask. Not a failure.
            return emit(GrillEvent::Done { project_id, added: 0 });
        }
        let context = ProjectContext::assemble(&conn, project_id)
            .map(|c| c.to_prompt())
            .unwrap_or_default();
        let clone_path = projects::get(&conn, project_id)
            .ok()
            .flatten()
            .and_then(|p| p.clone_path);
        (topics, context, clone_path)
    };

    let mut req = ModelRequest::planning(grill_user(&topics));
    req.system_append = Some(format!("{GRILL_SYSTEM}\n\n{context}"));
    if let Some(cp) = clone_path {
        req.cwd = Some(cp);
    }

    let mut final_text: Option<String> = None;
    let mut failure: Option<String> = None;
    ClaudeCodeProvider::new().run(&req, &mut |event: ModelEvent| match event {
        ModelEvent::ToolUse { name } => emit(GrillEvent::Tool { project_id, name }),
        ModelEvent::Completed { text, .. } => final_text = Some(text),
        ModelEvent::Unavailable { detail, .. } | ModelEvent::Failed { detail } => {
            failure = Some(detail)
        }
        _ => {}
    });

    if let Some(detail) = failure {
        return emit(GrillEvent::Failed { project_id, detail });
    }
    let text = match final_text {
        Some(t) => t,
        None => {
            return emit(GrillEvent::Failed {
                project_id,
                detail: "The model produced no result.".into(),
            })
        }
    };
    let questions = match parse_questions(&text) {
        Ok(q) => q,
        Err(detail) => return emit(GrillEvent::Failed { project_id, detail }),
    };

    let saved = match db.0.lock() {
        Ok(conn) => save_questions(&conn, project_id, &questions),
        Err(e) => Err(e.to_string()),
    };
    match saved {
        Ok(added) => emit(GrillEvent::Done { project_id, added }),
        Err(detail) => emit(GrillEvent::Failed { project_id, detail }),
    }
}

#[tauri::command]
pub fn grill_list(db: State<'_, Db>, project_id: i64) -> Result<Vec<Question>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list_questions(&conn, project_id)
}

#[cfg(test)]
mod tests {
    use super::super::generate::bank;
    use super::*;

    #[test]
    #[ignore = "runs a real model question-generation on a tiny repo; needs auth + uses credits. Run: cargo test -- --ignored"]
    fn real_question_generation() {
        let dir = std::env::temp_dir().join(format!("rh-grill-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "# Brisket Helpline\nA BBQ cook tracker + AI/human helpline.").unwrap();

        let topics: Vec<&_> = bank().iter().take(3).collect();
        let mut req = ModelRequest::planning(grill_user(&topics));
        req.system_append = Some(GRILL_SYSTEM.to_string());
        req.cwd = Some(dir.to_string_lossy().to_string());
        req.model = Some("sonnet".into());

        let mut text = None;
        ClaudeCodeProvider::new().run(&req, &mut |e: ModelEvent| {
            if let ModelEvent::Completed { text: t, .. } = e {
                text = Some(t);
            }
        });
        let qs = parse_questions(&text.expect("model returned a result")).expect("parses");
        assert!(!qs.is_empty());
        assert!(qs.iter().all(|q| !q.recommended_answer.trim().is_empty()), "each has a recommended answer");
        assert!(qs.iter().all(|q| !q.dimension.trim().is_empty()), "each tagged by dimension");

        std::fs::remove_dir_all(&dir).ok();
    }
}
