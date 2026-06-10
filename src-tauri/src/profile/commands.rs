//! Profile commands: the Settings surface (view/edit/reset/toggle) and the
//! end-of-session reflection trigger the frontend fires opportunistically
//! (mode switches, app start) — gating makes stray calls free.

use tauri::State;

use crate::db::Db;

#[derive(serde::Serialize)]
pub struct ProfileFile {
    pub name: String,
    pub content: String,
}

#[derive(serde::Serialize)]
pub struct ProfileStatus {
    pub enabled: bool,
    pub unreflected_events: i64,
    pub files: Vec<ProfileFile>,
}

#[tauri::command]
pub fn profile_get(db: State<'_, Db>) -> Result<ProfileStatus, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    // Keep facts fresh every time Settings opens — it's free SQL.
    let _ = super::write_facts(&conn, super::LEARNER_FILE);
    let _ = super::write_facts(&conn, super::REVIEW_FILE);
    Ok(ProfileStatus {
        enabled: super::enabled(&conn),
        unreflected_events: super::unreflected_count(&conn),
        files: [super::LEARNER_FILE, super::REVIEW_FILE]
            .into_iter()
            .map(|n| {
                Ok(ProfileFile { name: n.to_string(), content: super::read_or_create(n)? })
            })
            .collect::<Result<Vec<_>, String>>()?,
    })
}

#[tauri::command]
pub fn profile_set_enabled(db: State<'_, Db>, enabled: bool) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    super::set_enabled(&conn, enabled)
}

#[tauri::command]
pub fn profile_save_notes(name: String, notes: String) -> Result<(), String> {
    if name != super::LEARNER_FILE && name != super::REVIEW_FILE {
        return Err("Unknown profile file.".into());
    }
    super::save_notes(&name, &notes)
}

#[tauri::command]
pub fn profile_reset(name: String) -> Result<(), String> {
    if name != super::LEARNER_FILE && name != super::REVIEW_FILE {
        return Err("Unknown profile file.".into());
    }
    super::reset_auto_sections(&name)
}

/// Fire-and-forget from the frontend at session boundaries. Async: the haiku
/// call must never sit on the main thread.
#[tauri::command]
pub async fn profile_reflect(db: State<'_, Db>) -> Result<String, String> {
    super::reflect::run(db.inner())
}
