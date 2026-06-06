//! Suggestion commands. Phase 8 reads pending suggestions (created from chat);
//! Phase 9 adds approve/dismiss.

use tauri::State;

use super::{list, Suggestion};
use crate::db::Db;

#[tauri::command]
pub fn suggestions_list(
    db: State<'_, Db>,
    project_id: i64,
    status: Option<String>,
) -> Result<Vec<Suggestion>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list(&conn, project_id, status.as_deref())
}
