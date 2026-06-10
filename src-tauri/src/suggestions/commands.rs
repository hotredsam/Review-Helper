//! Suggestion commands. Phase 8 reads pending suggestions (created from chat);
//! Phase 9 adds approve/dismiss.

use tauri::State;

use super::{approve, approve_all, dismiss, list, Suggestion};
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

#[tauri::command]
pub fn suggestion_approve(db: State<'_, Db>, project_id: i64, suggestion_id: i64) -> Result<(), String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let out = approve(&mut conn, project_id, suggestion_id);
    if out.is_ok() {
        crate::profile::record(&conn, "suggestion", None, Some(project_id), &serde_json::json!({ "verdict": "approved" }));
    }
    out
}

#[tauri::command]
pub fn suggestion_dismiss(db: State<'_, Db>, project_id: i64, suggestion_id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let out = dismiss(&conn, project_id, suggestion_id);
    if out.is_ok() {
        crate::profile::record(&conn, "suggestion", None, Some(project_id), &serde_json::json!({ "verdict": "dismissed" }));
    }
    out
}

#[tauri::command]
pub fn suggestions_approve_all(db: State<'_, Db>, project_id: i64) -> Result<usize, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    approve_all(&mut conn, project_id)
}
