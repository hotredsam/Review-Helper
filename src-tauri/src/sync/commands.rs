//! Sync commands. T1: preview the planning package (no GitHub writes).

use tauri::State;

use super::issues::IssueAction;
use super::{apply_issue_sync, package, preview_issue_sync, push_main, push_planning_branch, PackageFile};
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

/// Preview the issue reconciliation (read-only) — the actions the user confirms.
#[tauri::command]
pub fn sync_issue_preview(db: State<'_, Db>, project_id: i64) -> Result<Vec<IssueAction>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    preview_issue_sync(&conn, project_id)
}

/// Apply the issue reconciliation (after a confirmed preview). Returns the count.
#[tauri::command]
pub fn sync_issue_apply(db: State<'_, Db>, project_id: i64) -> Result<usize, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    apply_issue_sync(&conn, project_id)
}

/// Push the package to main + prune stale phase docs (after a confirmed preview).
#[tauri::command]
pub fn sync_push_main(db: State<'_, Db>, project_id: i64) -> Result<usize, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    push_main(&conn, project_id)
}
