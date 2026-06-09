//! Learning-mode Tauri commands. L0 covers subject lifecycle (create/list/get/
//! delete); later sub-phases add intake-grill, module proposal, materials, and
//! the adaptive engine. A per-subject gate serializes the background model work
//! (one generation per subject at a time), mirroring GrillGate/PlanGate.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tauri::State;

use super::store::{self, Subject, SubjectDetail};
use crate::db::Db;

const MAX_TITLE_CHARS: usize = 200;
const MAX_SOURCE_CHARS: usize = 40_000;

/// Per-subject async gate: a background generation locks its subject's mutex so
/// two generations for the same subject can't interleave, while different
/// subjects still run concurrently.
#[derive(Default)]
pub struct LearningGate(pub Mutex<HashMap<i64, Arc<Mutex<()>>>>);

impl LearningGate {
    pub fn for_subject(&self, subject_id: i64) -> Arc<Mutex<()>> {
        let mut map = self.0.lock().unwrap();
        map.entry(subject_id).or_default().clone()
    }
}

/// Create a study subject from a described goal (`describe`) or extracted upload
/// text (`upload`). Validates the title and bounds the source text.
#[tauri::command]
pub fn subject_create(
    db: State<'_, Db>,
    title: String,
    source_kind: String,
    source_text: String,
) -> Result<i64, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("Give the subject a name.".into());
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(format!("Title is too long (max {MAX_TITLE_CHARS} characters)."));
    }
    if source_kind != "describe" && source_kind != "upload" {
        return Err("Unknown subject source.".into());
    }
    // Bound the source text to a safe budget (uploads can be large).
    let source: String = source_text.trim().chars().take(MAX_SOURCE_CHARS).collect();
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store::create_subject(&conn, title, &source_kind, &source)
}

#[tauri::command]
pub fn subjects_list(db: State<'_, Db>) -> Result<Vec<Subject>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store::list_subjects(&conn)
}

#[tauri::command]
pub fn subject_get(db: State<'_, Db>, subject_id: i64) -> Result<SubjectDetail, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store::get_subject(&conn, subject_id)?.ok_or_else(|| "Subject not found.".into())
}

#[tauri::command]
pub fn subject_delete(db: State<'_, Db>, subject_id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store::delete_subject(&conn, subject_id)
}
