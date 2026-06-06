//! Decision commands: read the record and supersede.

use tauri::State;

use super::{list, supersede, Decision};
use crate::db::Db;

#[tauri::command]
pub fn decisions_list(db: State<'_, Db>, project_id: i64) -> Result<Vec<Decision>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list(&conn, project_id)
}

#[tauri::command]
pub fn decision_supersede(db: State<'_, Db>, project_id: i64, decision_id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    supersede(&conn, project_id, decision_id)
}
