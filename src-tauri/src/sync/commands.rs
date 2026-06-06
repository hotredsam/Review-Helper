//! Sync commands. T1: preview the planning package (no GitHub writes).

use tauri::State;

use super::{package, push_planning_branch, PackageFile};
use crate::db::Db;

/// Render the planning package for preview (the files that would be pushed).
#[tauri::command]
pub fn sync_package(db: State<'_, Db>, project_id: i64) -> Result<Vec<PackageFile>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    package(&conn, project_id)
}

/// Push the package to the `planning` branch. Returns the number of files written.
#[tauri::command]
pub fn sync_push_planning(db: State<'_, Db>, project_id: i64) -> Result<usize, String> {
    // Hold the lock across the (network) push: pushes are explicit + infrequent,
    // and serializing them avoids interleaved writes to the same branch.
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    push_planning_branch(&conn, project_id)
}
