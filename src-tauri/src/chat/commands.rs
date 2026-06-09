//! Chat commands: persisted transcripts, a past-chats list, and a turn run on a
//! background thread (streaming tokens). Each turn injects the ProjectContext +
//! the full text of all prior chats (cross-chat memory); messages persist to the
//! DB. Inferred updates (T2) become pending suggestions.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use super::store::{self, ChatMessage, Transcript};
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
    Started { project_id: i64, transcript_id: i64 },
    Token { project_id: i64, transcript_id: i64, text: String },
    Tool { project_id: i64, transcript_id: i64, name: String },
    Done { project_id: i64, transcript_id: i64, reply: String, suggestions: usize },
    Failed { project_id: i64, transcript_id: i64, detail: String },
}

/// Start a fresh chat for a project; returns the new transcript id.
#[tauri::command]
pub fn chat_new(db: State<'_, Db>, project_id: i64) -> Result<i64, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    projects::get(&conn, project_id)?.ok_or("Project not found.")?;
    store::new_transcript(&conn, project_id)
}

/// The project's past chats (newest first) for the history rail.
#[tauri::command]
pub fn chat_transcripts(db: State<'_, Db>, project_id: i64) -> Result<Vec<Transcript>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store::list_transcripts(&conn, project_id)
}

/// The messages of one transcript (to render when a past chat is opened).
#[tauri::command]
pub fn chat_messages(db: State<'_, Db>, transcript_id: i64) -> Result<Vec<ChatMessage>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store::list_messages(&conn, transcript_id)
}

#[tauri::command]
pub fn chat_delete(db: State<'_, Db>, transcript_id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store::delete_transcript(&conn, transcript_id)
}

#[tauri::command]
pub fn chat_send(
    app: AppHandle,
    db: State<'_, Db>,
    project_id: i64,
    transcript_id: i64,
    message: String,
) -> Result<(), String> {
    let message = message.trim().to_string();
    if message.is_empty() {
        return Err("Type a message first.".into());
    }
    if message.chars().count() > 20_000 {
        return Err("Message is too long (max 20000 characters).".into());
    }
    // Read routing + grounding + history under one lock, then persist the user
    // message (history is gathered first so it holds the PRIOR turns only).
    let (config, context, history, clone_path) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let project = projects::get(&conn, project_id)?.ok_or("Project not found.")?;
        let config = load_model_config(&conn);
        let context = ProjectContext::assemble(&conn, project_id)
            .map(|c| c.to_prompt())
            .unwrap_or_default();
        store::set_title_if_empty(&conn, transcript_id, &message)?;
        let history = store::history_context(&conn, project_id, transcript_id)?;
        store::add_message(&conn, transcript_id, "user", &message)?;
        (config, context, history, project.clone_path)
    };
    let app = app.clone();
    let report = app.clone();
    crate::util::spawn_guarded(
        move || run_chat(app, project_id, transcript_id, message, config, context, history, clone_path),
        move || {
            let _ = report.emit(
                "chat-event",
                &ChatEvent::Failed { project_id, transcript_id, detail: "The chat crashed unexpectedly.".into() },
            );
        },
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_chat(
    app: AppHandle,
    project_id: i64,
    transcript_id: i64,
    message: String,
    config: ModelConfig,
    context: String,
    history: String,
    clone_path: Option<String>,
) {
    let emit = |ev: ChatEvent| {
        let _ = app.emit("chat-event", &ev);
    };
    emit(ChatEvent::Started { project_id, transcript_id });

    let mut sys = format!("{CHAT_SYSTEM}\n\n{context}");
    if !history.trim().is_empty() {
        sys.push_str("\n\n");
        sys.push_str(&history);
    }
    let mut req = ModelRequest::planning(message);
    req.system_append = Some(sys);
    // No session_id: the full chat history is injected each turn, so memory
    // survives restarts and never depends on a live CLI session.
    if let Some(cp) = clone_path {
        req.cwd = Some(cp);
    }

    let mut final_text: Option<String> = None;
    let mut failure: Option<String> = None;
    provider_for(&config).run(&req, &mut |event: ModelEvent| match event {
        ModelEvent::AssistantText { text } => emit(ChatEvent::Token { project_id, transcript_id, text }),
        ModelEvent::ToolUse { name } => emit(ChatEvent::Tool { project_id, transcript_id, name }),
        ModelEvent::Completed { text, .. } => final_text = Some(text),
        ModelEvent::Unavailable { detail, .. } | ModelEvent::Failed { detail } => failure = Some(detail),
        _ => {}
    });

    if let Some(detail) = failure {
        return emit(ChatEvent::Failed { project_id, transcript_id, detail });
    }
    let text = match final_text {
        Some(t) => t,
        None => {
            return emit(ChatEvent::Failed {
                project_id,
                transcript_id,
                detail: "The model produced no result.".into(),
            })
        }
    };

    let (mut reply, parsed) = super::parse_suggestions(&text);
    if reply.is_empty() && !parsed.is_empty() {
        reply = "Recorded the suggestions below.".to_string();
    }

    // Persist the assistant reply, then save any inferred suggestions as pending.
    let db = app.state::<Db>();
    let saved = {
        let mut conn = match db.0.lock() {
            Ok(c) => c,
            Err(e) => return emit(ChatEvent::Failed { project_id, transcript_id, detail: e.to_string() }),
        };
        let _ = store::add_message(&conn, transcript_id, "assistant", &reply);
        if parsed.is_empty() {
            0
        } else {
            match crate::suggestions::save(&mut conn, project_id, &parsed) {
                Ok(n) => n,
                Err(detail) => {
                    eprintln!("chat: failed to save suggestions: {detail}");
                    return emit(ChatEvent::Failed {
                        project_id,
                        transcript_id,
                        detail: format!("Could not save suggestions: {detail}"),
                    });
                }
            }
        }
    };
    emit(ChatEvent::Done { project_id, transcript_id, reply, suggestions: saved });
}
