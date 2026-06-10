//! Feature inbox commands. The audio path returns a stub placeholder for now.

use tauri::State;

use super::{add, list, pending_count, set_status, Feature};
use crate::db::Db;

#[tauri::command]
pub fn features_list(db: State<'_, Db>, project_id: i64) -> Result<Vec<Feature>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list(&conn, project_id)
}

#[tauri::command]
pub fn feature_add(
    db: State<'_, Db>,
    project_id: i64,
    title: String,
    detail: Option<String>,
    source: Option<String>,
) -> Result<Feature, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    add(&conn, project_id, &title, detail.as_deref().unwrap_or(""), source.as_deref().unwrap_or("text"))
}

#[tauri::command]
pub fn feature_set_status(db: State<'_, Db>, project_id: i64, feature_id: i64, status: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    set_status(&conn, project_id, feature_id, &status)
}

#[tauri::command]
pub fn features_pending_count(db: State<'_, Db>, project_id: i64) -> Result<i64, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    pending_count(&conn, project_id)
}


