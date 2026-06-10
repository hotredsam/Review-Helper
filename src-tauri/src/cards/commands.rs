//! Card commands for the Understand hub.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tauri::State;

use super::{generate_card, get as get_card, list as list_cards, normalize_domain, upsert, Card};
use crate::db::Db;
use crate::model::commands::provider_for;
use crate::settings::load_model_config;

/// Max length of a card term (chars) and a captured explanation (bytes).
/// Bounds token cost and DB growth on the "too-large" input path.
const MAX_TERM_CHARS: usize = 200;
const MAX_EXPLANATION_BYTES: usize = 10_000;

/// Per-term generation gate: serializes concurrent `card_explain` calls for the
/// same term so two requests don't both call the model (the result is idempotent
/// via UNIQUE(term), but a double call wastes credits). Different terms still
/// generate in parallel. Holds one tiny `Arc<Mutex<()>>` per distinct term
/// explained — bounded by the app's vocabulary.
#[derive(Default)]
pub struct CardGate(pub Mutex<HashMap<String, Arc<Mutex<()>>>>);

fn has_content(c: &Card) -> bool {
    c.what_md.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false)
}

#[tauri::command]
pub fn cards_list(db: State<'_, Db>) -> Result<Vec<Card>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list_cards(&conn)
}

#[tauri::command]
pub fn card_get(db: State<'_, Db>, term: String) -> Result<Option<Card>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    get_card(&conn, &term)
}

/// Return a card for `term`, generating + caching it if it has no content yet.
/// When `project_id` is given (the Understand hub on a project), the card is
/// associated with that project for the "This project" filter.
#[tauri::command]
pub async fn card_explain(
    db: State<'_, Db>,
    gate: State<'_, CardGate>,
    term: String,
    project_id: Option<i64>,
) -> Result<Card, String> {
    let term = term.trim().to_string();
    if term.is_empty() {
        return Err("Enter a term to explain.".into());
    }
    // Truncate an over-long term (e.g. a full composite stack choice) rather than
    // rejecting it, so "Why?" never dead-ends on a long choice.
    let term: String = term.chars().take(MAX_TERM_CHARS).collect();

    if let Some(pid) = project_id {
        if let Ok(conn) = db.0.lock() {
            let _ = super::study::record_project_card(&conn, pid, &term); // best-effort
        }
    }

    // Fast path: reuse a card that already has content.
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        if let Some(c) = get_card(&conn, &term)? {
            if has_content(&c) {
                return Ok(c);
            }
        }
    }

    // Serialize generation for this term so concurrent callers don't double-spend.
    let key = term.to_lowercase();
    let term_lock = {
        let mut map = gate.0.lock().map_err(|e| e.to_string())?;
        map.entry(key).or_default().clone()
    };
    // Recover from poisoning (a prior panic while held) so one crash doesn't
    // brick explaining this term forever; the gate's `()` carries no invariant.
    let _guard = term_lock.lock().unwrap_or_else(|e| e.into_inner());

    // Re-check under the gate: another caller may have just generated it.
    let existing_source = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        match get_card(&conn, &term)? {
            Some(c) if has_content(&c) => return Ok(c),
            Some(c) => c.source,
            None => None,
        }
    };

    // Generate without holding the DB lock across the model call.
    let provider = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        provider_for(&load_model_config(&conn))
    };
    let run_key = format!("card:{}", term.to_lowercase());
    let token = crate::model::registry::register(&run_key);
    let content = generate_card(provider.as_ref(), &term, &token);
    crate::model::registry::finish(&run_key);
    let content = content?;
    let source = match existing_source.as_deref() {
        Some("detected") => "detected",
        _ => "generated",
    };
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    upsert(
        &conn,
        &term,
        normalize_domain(&content.domain),
        &content.what,
        &content.when,
        &content.why,
        source,
    )
}

/// Capture an explanation (e.g. from a chat answer) as a retrievable card.
/// The chat calls this to turn an in-conversation explanation into a card;
/// existing `when`/`why` on a richer card are preserved, not blanked.
#[tauri::command]
pub fn card_capture(
    db: State<'_, Db>,
    term: String,
    explanation: String,
    domain: Option<String>,
    project_id: Option<i64>,
) -> Result<Card, String> {
    let term = term.trim().to_string();
    if term.is_empty() {
        return Err("A card needs a term.".into());
    }
    if term.chars().count() > MAX_TERM_CHARS {
        return Err(format!("Term is too long (max {MAX_TERM_CHARS} characters)."));
    }
    if explanation.len() > MAX_EXPLANATION_BYTES {
        return Err("Explanation is too long (max 10000 characters).".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    if let Some(pid) = project_id {
        let _ = super::study::record_project_card(&conn, pid, &term);
    }
    super::capture(&conn, &term, &explanation, domain.as_deref().unwrap_or("other"))
}

/// Terms of cards that belong to this project (for the "This project" filter).
#[tauri::command]
pub fn card_project_terms(db: State<'_, Db>, project_id: i64) -> Result<Vec<String>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    super::study::project_terms(&conn, project_id)
}

/// Fix the spelling/grammar of a typed term before it's explained + carded.
#[tauri::command]
pub async fn card_clean_term(db: State<'_, Db>, term: String) -> Result<String, String> {
    let t = term.trim();
    if t.is_empty() {
        return Err("Enter a term first.".into());
    }
    let provider = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        provider_for(&load_model_config(&conn))
    };
    super::study::clean_term(provider.as_ref(), t, &crate::model::CancelToken::new())
}

/// 5–10 starter questions for a card; cached after the first generation.
#[tauri::command]
pub async fn card_premade_questions(db: State<'_, Db>, term: String) -> Result<Vec<String>, String> {
    let term = term.trim().to_string();
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let cached = super::study::cached_questions(&conn, &term)?;
        if !cached.is_empty() {
            return Ok(cached);
        }
    }
    let provider = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        provider_for(&load_model_config(&conn))
    };
    let qs = super::study::generate_questions(provider.as_ref(), &term, &crate::model::CancelToken::new())?; // model call, no lock held
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    super::study::save_questions(&conn, &term, &qs)?;
    Ok(qs)
}

/// Read a card's inline chat history (per project + term).
#[tauri::command]
pub fn card_chat_history(
    db: State<'_, Db>,
    project_id: i64,
    term: String,
) -> Result<Vec<super::study::CardMsg>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    super::study::chat_history(&conn, project_id, &term)
}

/// Send a message in a card's inline chat; persists both sides, returns the reply.
#[tauri::command]
pub async fn card_chat_send(
    db: State<'_, Db>,
    project_id: i64,
    term: String,
    message: String,
) -> Result<String, String> {
    let message = message.trim().to_string();
    if message.is_empty() {
        return Err("Type a question first.".into());
    }
    if message.chars().count() > 4_000 {
        return Err("Message is too long.".into());
    }
    // Gather the card content + history, persist the user message, then call out.
    let (what, why, history) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let card = get_card(&conn, &term)?;
        let (what, why) = card
            .map(|c| (c.what_md.unwrap_or_default(), c.why_md.unwrap_or_default()))
            .unwrap_or_default();
        let history = super::study::chat_history(&conn, project_id, &term)?;
        super::study::chat_add(&conn, project_id, &term, "user", &message)?;
        (what, why, history)
    };
    let provider = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        provider_for(&load_model_config(&conn))
    };
    let run_key = format!("cardchat:{project_id}:{}", term.to_lowercase());
    let token = crate::model::registry::register(&run_key);
    let reply = super::study::chat_reply(provider.as_ref(), &term, &what, &why, &history, &message, &token);
    crate::model::registry::finish(&run_key);
    let reply = reply?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    super::study::chat_add(&conn, project_id, &term, "assistant", &reply)?;
    Ok(reply)
}
