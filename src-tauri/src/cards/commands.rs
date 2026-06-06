//! Card commands for the Understand hub.

use tauri::State;

use super::{get as get_card, list as list_cards, Card};
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
