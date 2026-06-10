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
pub async fn sync_push_planning(db: State<'_, Db>, project_id: i64) -> Result<usize, String> {
    push_planning_branch(db.inner(), project_id)
}

/// Preview the push-to-main: every issue change + every file deletion. Read-only.
#[tauri::command]
pub async fn sync_main_preview(db: State<'_, Db>, project_id: i64) -> Result<SyncPreview, String> {
    preview_main_sync(db.inner(), project_id)
}

/// Apply the confirmed preview (push docs + replay issue actions + delete the
/// shown files). The exact `preview` the user confirmed is passed back in.
///
/// `apply_main_sync` takes the `Db` itself (not a held lock) so the DB mutex is
/// only locked for the brief reads/writes around the GitHub network I/O, never
/// across it — a slow or hung sync can't freeze the rest of the app.
#[tauri::command]
pub async fn sync_main_apply(db: State<'_, Db>, project_id: i64, preview: SyncPreview) -> Result<SyncResult, String> {
    apply_main_sync(db.inner(), project_id, preview)
}
