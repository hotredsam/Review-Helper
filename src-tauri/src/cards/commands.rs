//! Card commands for the Understand hub.

use tauri::State;

use super::{generate_card, get as get_card, list as list_cards, normalize_domain, upsert, Card};
use crate::db::Db;

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
#[tauri::command]
pub fn card_explain(db: State<'_, Db>, term: String) -> Result<Card, String> {
    let term = term.trim().to_string();
    if term.is_empty() {
        return Err("Enter a term to explain.".into());
    }
    // Reuse a card that already has content; remember a detected stub's source.
    let existing_source = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        match get_card(&conn, &term)? {
            Some(c) if c.what_md.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false) => {
                return Ok(c)
            }
            Some(c) => c.source,
            None => None,
        }
    };
    // Generate without holding the DB lock across the model call.
    let content = generate_card(&term)?;
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
) -> Result<Card, String> {
    let term = term.trim().to_string();
    if term.is_empty() {
        return Err("A card needs a term.".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    super::capture(&conn, &term, &explanation, domain.as_deref().unwrap_or("other"))
}
