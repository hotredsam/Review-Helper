//! Assessment commands: run the scan + model scoring on a background thread,
//! streaming progress, and read the latest assessment back.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use super::{
    assess_user, get_assessment as get_assessment_fn, parse_assessment, run_scan, save_assessment,
    AssessmentView, ASSESS_SYSTEM,
};
use crate::db::Db;
use crate::model::claude::ClaudeCodeProvider;
use crate::model::{ModelEvent, ModelProvider, ModelRequest};
use crate::projects;

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssessmentEvent {
    Started { project_id: i64 },
    Tool { project_id: i64, name: String },
    Done { project_id: i64, overall: i64 },
    Failed { project_id: i64, detail: String },
}

#[tauri::command]
pub fn assess_project(app: AppHandle, db: State<'_, Db>, project_id: i64) -> Result<(), String> {
    let project = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        projects::get(&conn, project_id)?.ok_or("Project not found.")?
    };
    let clone_path = project.clone_path.ok_or("Clone the repo first, then assess.")?;
    if !std::path::Path::new(&clone_path).join(".git").is_dir() {
        return Err("The clone is missing on disk. Refresh the clone, then assess.".into());
    }

    // Deterministic scan first (fast subprocess), then hand the facts to the model.
    let scan = run_scan(&clone_path)?;
    let mut req = ModelRequest::planning(assess_user(&scan));
    req.system_append = Some(ASSESS_SYSTEM.to_string());
    req.cwd = Some(clone_path);

    let app = app.clone();
    std::thread::spawn(move || run_assessment(app, project_id, req));
    Ok(())
}

fn run_assessment(app: AppHandle, project_id: i64, req: ModelRequest) {
    let emit = |ev: AssessmentEvent| {
        let _ = app.emit("assessment-event", &ev);
    };
    emit(AssessmentEvent::Started { project_id });

    let mut final_text: Option<String> = None;
    let mut failure: Option<String> = None;
    ClaudeCodeProvider::new().run(&req, &mut |event: ModelEvent| match event {
        ModelEvent::ToolUse { name } => emit(AssessmentEvent::Tool { project_id, name }),
        ModelEvent::Completed { text, .. } => final_text = Some(text),
        ModelEvent::Unavailable { detail, .. } | ModelEvent::Failed { detail } => {
            failure = Some(detail)
        }
        _ => {}
    });

    if let Some(detail) = failure {
        emit(AssessmentEvent::Failed { project_id, detail });
        return;
    }
    let text = match final_text {
        Some(t) => t,
        None => {
            emit(AssessmentEvent::Failed {
                project_id,
                detail: "The model produced no result.".into(),
            });
            return;
        }
    };
    let assessment = match parse_assessment(&text) {
        Ok(a) => a,
        Err(detail) => {
            emit(AssessmentEvent::Failed { project_id, detail });
            return;
        }
    };
    let overall = assessment
        .get("dimensions_overall")
        .and_then(|x| x.as_i64())
        .unwrap_or(0)
        .clamp(0, 100);

    let db = app.state::<Db>();
    let saved = match db.0.lock() {
        Ok(conn) => save_assessment(&conn, project_id, &assessment),
        Err(e) => Err(e.to_string()),
    };
    match saved {
        Ok(()) => emit(AssessmentEvent::Done { project_id, overall }),
        Err(detail) => emit(AssessmentEvent::Failed { project_id, detail }),
    }
}

#[tauri::command]
pub fn get_assessment(db: State<'_, Db>, project_id: i64) -> Result<Option<AssessmentView>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    get_assessment_fn(&conn, project_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "runs a real model assessment on a tiny repo; needs auth + uses credits. Run: cargo test -- --ignored"]
    fn real_assessment_scores_a_repo() {
        let dir = std::env::temp_dir().join(format!("rh-assess-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "# demo app").unwrap();
        std::fs::write(dir.join("main.py"), "print('hi')\n").unwrap();

        let scan = run_scan(dir.to_str().unwrap()).unwrap();
        let mut req = ModelRequest::planning(assess_user(&scan));
        req.system_append = Some(ASSESS_SYSTEM.to_string());
        req.cwd = Some(dir.to_string_lossy().to_string());
        req.model = Some("sonnet".into());

        let mut text = None;
        ClaudeCodeProvider::new().run(&req, &mut |e: ModelEvent| {
            if let ModelEvent::Completed { text: t, .. } = e {
                text = Some(t);
            }
        });
        let a = parse_assessment(&text.expect("model returned a result")).expect("parses");
        let arch = a["dimensions"]["architecture"]["score"]
            .as_i64()
            .expect("architecture score present");
        assert!((0..=100).contains(&arch));
        assert!(!a["top_fixes"].as_array().unwrap().is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }
}
