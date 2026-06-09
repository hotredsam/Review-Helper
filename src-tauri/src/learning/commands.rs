//! Learning-mode Tauri commands. L0 covers subject lifecycle (create/list/get/
//! delete); later sub-phases add intake-grill, module proposal, materials, and
//! the adaptive engine. A per-subject gate serializes the background model work
//! (one generation per subject at a time), mirroring GrillGate/PlanGate.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tauri::State;

use super::intake::{self, IntakeItem};
use super::propose::{self, ProposedModule};
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

/// The subject's intake (scoping) questions, generated + cached on first call.
/// Cache check and persistence happen under a brief DB lock; the model call runs
/// WITHOUT the lock (so it never blocks the rest of the app), serialized per
/// subject by the gate so a double-click can't double-generate.
#[tauri::command]
pub fn learning_intake(
    db: State<'_, Db>,
    gate: State<'_, LearningGate>,
    subject_id: i64,
) -> Result<Vec<IntakeItem>, String> {
    let subject = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let existing = intake::list(&conn, subject_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
        store::get_subject(&conn, subject_id)?.ok_or("Subject not found.")?
    };

    let glock = gate.for_subject(subject_id);
    let _g = glock.lock().unwrap_or_else(|e| e.into_inner());

    // Another waiter may have generated while we blocked on the gate.
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let existing = intake::list(&conn, subject_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
    }

    let questions = intake::fetch_questions(&subject)?; // model call, no DB lock held
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    intake::save(&conn, subject_id, &questions)?;
    intake::list(&conn, subject_id)
}

/// Save (or clear) the answer to one intake question.
#[tauri::command]
pub fn learning_intake_answer(db: State<'_, Db>, intake_id: i64, answer: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    intake::set_answer(&conn, intake_id, &answer)
}

/// Propose a study plan (the module manifest) from the scoping answers, caching
/// it and advancing the subject to the `proposed` stage. Same lock discipline as
/// intake: cache-check + gate + lock-free model call + save.
#[tauri::command]
pub fn learning_propose(
    db: State<'_, Db>,
    gate: State<'_, LearningGate>,
    subject_id: i64,
) -> Result<Vec<ProposedModule>, String> {
    let (subject, intake) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let existing = propose::list_modules(&conn, subject_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
        let subject = store::get_subject(&conn, subject_id)?.ok_or("Subject not found.")?;
        let intake = intake::list(&conn, subject_id)?;
        (subject, intake)
    };

    let glock = gate.for_subject(subject_id);
    let _g = glock.lock().unwrap_or_else(|e| e.into_inner());

    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let existing = propose::list_modules(&conn, subject_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
    }

    let modules = propose::fetch_modules(&subject, &intake)?; // model call, no DB lock
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    propose::save_modules(&conn, subject_id, &modules)?;
    store::set_stage(&conn, subject_id, "proposed")?;
    propose::list_modules(&conn, subject_id)
}

#[tauri::command]
pub fn learning_modules(db: State<'_, Db>, subject_id: i64) -> Result<Vec<ProposedModule>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    propose::list_modules(&conn, subject_id)
}

#[tauri::command]
pub fn learning_module_set_included(db: State<'_, Db>, module_id: i64, included: bool) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    propose::set_included(&conn, module_id, included)
}

/// Lock in the edited plan and move to studying. Requires at least one included
/// module (an empty plan has nothing to generate).
#[tauri::command]
pub fn learning_confirm_plan(db: State<'_, Db>, subject_id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    if propose::included_count(&conn, subject_id)? == 0 {
        return Err("Keep at least one module to study.".into());
    }
    store::set_stage(&conn, subject_id, "ready")
}
