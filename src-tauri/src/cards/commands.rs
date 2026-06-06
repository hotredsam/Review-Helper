//! Card commands for the Understand hub.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tauri::State;

use super::{generate_card, get as get_card, list as list_cards, normalize_domain, upsert, Card};
use crate::db::Db;

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
#[tauri::command]
pub fn card_explain(db: State<'_, Db>, gate: State<'_, CardGate>, term: String) -> Result<Card, String> {
    let term = term.trim().to_string();
    if term.is_empty() {
        return Err("Enter a term to explain.".into());
    }
    if term.chars().count() > MAX_TERM_CHARS {
        return Err(format!("Term is too long (max {MAX_TERM_CHARS} characters)."));
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
    let _guard = term_lock.lock().map_err(|_| "card generation lock poisoned".to_string())?;

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
    if term.chars().count() > MAX_TERM_CHARS {
        return Err(format!("Term is too long (max {MAX_TERM_CHARS} characters)."));
    }
    if explanation.len() > MAX_EXPLANATION_BYTES {
        return Err("Explanation is too long (max 10000 characters).".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    super::capture(&conn, &term, &explanation, domain.as_deref().unwrap_or("other"))
}
