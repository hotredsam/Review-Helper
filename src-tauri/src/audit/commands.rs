//! Audit-log command.

use tauri::State;

use super::{list, AuditEntry};
use crate::db::Db;

#[tauri::command]
pub fn audit_list(db: State<'_, Db>, project_id: i64) -> Result<Vec<AuditEntry>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list(&conn, project_id)
}
