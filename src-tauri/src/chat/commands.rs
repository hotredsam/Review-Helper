//! Chat commands: run a grounded conversation turn on a background thread,
//! streaming tokens, resuming the prior session, and (T2) turning inferred
//! updates into pending suggestions.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use super::CHAT_SYSTEM;
use crate::context::ProjectContext;
use crate::db::Db;
use crate::model::commands::provider_for;
use crate::model::{ModelEvent, ModelRequest};
use crate::projects;
use crate::settings::{load_model_config, ModelConfig};

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEvent {
    Started { project_id: i64 },
    Token { project_id: i64, text: String },
    Tool { project_id: i64, name: String },
    Done {
        project_id: i64,
        session_id: Option<String>,
        reply: String,
        suggestions: usize,
    },
    Failed { project_id: i64, detail: String },
}

#[tauri::command]
pub fn chat_send(
    app: AppHandle,
    db: State<'_, Db>,
    project_id: i64,
    message: String,
    session_id: Option<String>,
) -> Result<(), String> {
    let message = message.trim().to_string();
    if message.is_empty() {
        return Err("Type a message first.".into());
    }
    if message.chars().count() > 20_000 {
        return Err("Message is too long (max 20000 characters).".into());
    }
    // Validate the project + capture routing, grounding, and clone path up front.
    let (config, context, clone_path) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let project = projects::get(&conn, project_id)?.ok_or("Project not found.")?;
        let config = load_model_config(&conn);
        let context = ProjectContext::assemble(&conn, project_id)
            .map(|c| c.to_prompt())
            .unwrap_or_default();
        (config, context, project.clone_path)
    };
    let app = app.clone();
    let report = app.clone();
    crate::util::spawn_guarded(
        move || run_chat(app, project_id, message, session_id, config, context, clone_path),
        move || {
            let _ = report.emit(
                "chat-event",
                &ChatEvent::Failed { project_id, detail: "The chat crashed unexpectedly.".into() },
            );
        },
    );
    Ok(())
}

fn run_chat(
    app: AppHandle,
    project_id: i64,
    message: String,
    session_id: Option<String>,
    config: ModelConfig,
    context: String,
    clone_path: Option<String>,
) {
    let emit = |ev: ChatEvent| {
        let _ = app.emit("chat-event", &ev);
    };
    emit(ChatEvent::Started { project_id });

    let prior_session = session_id.clone();
    let mut req = ModelRequest::planning(message);
    req.system_append = Some(format!("{CHAT_SYSTEM}\n\n{context}"));
    req.session_id = session_id; // resume the prior turn's session, if any
    if let Some(cp) = clone_path {
        req.cwd = Some(cp);
    }

    let mut final_text: Option<String> = None;
    // Default to the prior session so a None on completion doesn't break resume.
    let mut new_session: Option<String> = prior_session.clone();
    let mut failure: Option<String> = None;
    provider_for(&config).run(&req, &mut |event: ModelEvent| match event {
        ModelEvent::AssistantText { text } => emit(ChatEvent::Token { project_id, text }),
        ModelEvent::ToolUse { name } => emit(ChatEvent::Tool { project_id, name }),
        ModelEvent::Completed { text, session_id } => {
            final_text = Some(text);
            if session_id.is_some() {
                new_session = session_id;
            }
        }
        ModelEvent::Unavailable { detail, .. } | ModelEvent::Failed { detail } => {
            failure = Some(detail)
        }
        _ => {}
    });

    if let Some(detail) = failure {
        return emit(ChatEvent::Failed { project_id, detail });
    }
    let text = match final_text {
        Some(t) => t,
        None => {
            return emit(ChatEvent::Failed {
                project_id,
                detail: "The model produced no result.".into(),
            })
        }
    };

    // Split inferred updates out of the prose, persist them as PENDING
    // suggestions (the user approves later), and show the clean reply.
    let (mut reply, parsed) = super::parse_suggestions(&text);
    if reply.is_empty() && !parsed.is_empty() {
        reply = "Recorded the suggestions below.".to_string();
    }
    let saved = if parsed.is_empty() {
        0
    } else {
        let db = app.state::<Db>();
        let result = match db.0.lock() {
            Ok(mut conn) => crate::suggestions::save(&mut conn, project_id, &parsed),
            Err(e) => Err(e.to_string()),
        };
        match result {
            Ok(n) => n,
            // Surface a save failure instead of silently swallowing it (mirrors grill).
            Err(detail) => {
                eprintln!("chat: failed to save suggestions: {detail}");
                return emit(ChatEvent::Failed {
                    project_id,
                    detail: format!("Could not save suggestions: {detail}"),
                });
            }
        }
    };
    emit(ChatEvent::Done {
        project_id,
        session_id: new_session,
        reply,
        suggestions: saved,
    });
}
