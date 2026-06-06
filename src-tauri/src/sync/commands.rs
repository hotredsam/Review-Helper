//! Sync commands. Preview is read-only; apply replays the CONFIRMED preview so
//! GitHub is only ever changed to match what the user saw.

use tauri::State;

use super::{
    apply_main_sync, package, preview_main_sync, push_planning_branch, PackageFile, SyncPreview, SyncResult,
};
use crate::db::Db;

/// Render the planning package for preview (the files that would be pushed).
#[tauri::command]
pub fn sync_package(db: State<'_, Db>, project_id: i64) -> Result<Vec<PackageFile>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    package(&conn, project_id)
}

/// Push the package to the `planning` branch (non-destructive). Returns the count.
#[tauri::command]
pub fn sync_push_planning(db: State<'_, Db>, project_id: i64) -> Result<usize, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    push_planning_branch(&conn, project_id)
}

/// Preview the push-to-main: every issue change + every file deletion. Read-only.
#[tauri::command]
pub fn sync_main_preview(db: State<'_, Db>, project_id: i64) -> Result<SyncPreview, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    preview_main_sync(&conn, project_id)
}

/// Apply the confirmed preview (push docs + replay issue actions + delete the
/// shown files). The exact `preview` the user confirmed is passed back in.
#[tauri::command]
pub fn sync_main_apply(db: State<'_, Db>, project_id: i64, preview: SyncPreview) -> Result<SyncResult, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    apply_main_sync(&mut conn, project_id, preview)
}
