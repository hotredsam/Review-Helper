//! Sync commands. T1: preview the planning package (no GitHub writes).

use tauri::State;

use super::{package, PackageFile};
use crate::db::Db;

/// Render the planning package for preview (the files that would be pushed).
#[tauri::command]
pub fn sync_package(db: State<'_, Db>, project_id: i64) -> Result<Vec<PackageFile>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    package(&conn, project_id)
}
