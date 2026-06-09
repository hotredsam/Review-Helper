//! Learning-mode Tauri commands. L0 covers subject lifecycle (create/list/get/
//! delete); later sub-phases add intake-grill, module proposal, materials, and
//! the adaptive engine. A per-subject gate serializes the background model work
//! (one generation per subject at a time), mirroring GrillGate/PlanGate.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::State;

use super::intake::{self, IntakeItem};
use super::materials::{self, Flashcard, QuizQuestion};
use super::profile::{self, ProfileSnapshot};
use super::propose::{self, ProposedModule};
use super::store::{self, Subject, SubjectDetail};
use super::tutor::{self, TutorMsg};
use super::{mastery, schedule};
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

/// Load a module's subject + identity for generation (under a short lock).
fn module_grounding(db: &State<'_, Db>, module_id: i64) -> Result<(SubjectDetail, materials::ModuleRow), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let m = materials::module_row(&conn, module_id)?;
    let subject = store::get_subject(&conn, m.subject_id)?.ok_or("Subject not found.")?;
    Ok((subject, m))
}

/// A module's notes, generated + cached on first open. Same lock discipline as
/// the other generators (cache + per-module gate + lock-free model call + save).
#[tauri::command]
pub fn learning_notes(db: State<'_, Db>, gate: State<'_, LearningGate>, module_id: i64) -> Result<String, String> {
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        if let Some(body) = materials::notes_get(&conn, module_id)? {
            return Ok(body);
        }
    }
    let glock = gate.for_subject(module_id);
    let _g = glock.lock().unwrap_or_else(|e| e.into_inner());
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        if let Some(body) = materials::notes_get(&conn, module_id)? {
            return Ok(body);
        }
    }
    let (subject, m) = module_grounding(&db, module_id)?;
    let body = materials::fetch_notes(&subject, &m)?; // model call, no DB lock
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    materials::notes_save(&conn, module_id, &body)?;
    materials::set_module_status(&conn, module_id, "ready")?;
    Ok(body)
}

/// A module's flashcards, generated + cached on first open.
#[tauri::command]
pub fn learning_flashcards(db: State<'_, Db>, gate: State<'_, LearningGate>, module_id: i64) -> Result<Vec<Flashcard>, String> {
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let existing = materials::flashcards_list(&conn, module_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
    }
    let glock = gate.for_subject(module_id);
    let _g = glock.lock().unwrap_or_else(|e| e.into_inner());
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let existing = materials::flashcards_list(&conn, module_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
    }
    let (subject, m) = module_grounding(&db, module_id)?;
    let cards = materials::fetch_flashcards(&subject, &m)?; // model call, no DB lock
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    materials::flashcards_save(&conn, module_id, m.subject_id, m.skill.as_deref().unwrap_or(""), &cards)?;
    materials::set_module_status(&conn, module_id, "ready")?;
    materials::flashcards_list(&conn, module_id)
}

/// A module's quiz questions, generated + cached on first open.
#[tauri::command]
pub fn learning_quiz(db: State<'_, Db>, gate: State<'_, LearningGate>, module_id: i64) -> Result<Vec<QuizQuestion>, String> {
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let existing = materials::quiz_list(&conn, module_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
    }
    let glock = gate.for_subject(module_id);
    let _g = glock.lock().unwrap_or_else(|e| e.into_inner());
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let existing = materials::quiz_list(&conn, module_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
    }
    let (subject, m) = module_grounding(&db, module_id)?;
    let questions = materials::fetch_quiz(&subject, &m)?; // model call, no DB lock
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    materials::quiz_save(&conn, module_id, m.subject_id, m.skill.as_deref().unwrap_or(""), &questions)?;
    materials::set_module_status(&conn, module_id, "ready")?;
    materials::quiz_list(&conn, module_id)
}

// ---- L4: the adaptive engine (FSRS scheduling + BKT mastery + pace) ----

/// Grade a flashcard (1=Again…4=Easy): advances its FSRS schedule, nudges the
/// skill's mastery, and records the review. Returns the next due date (RFC3339).
#[tauri::command]
pub fn learning_flashcard_grade(db: State<'_, Db>, flashcard_id: i64, rating: i64) -> Result<String, String> {
    if !(1..=4).contains(&rating) {
        return Err("Invalid grade.".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let g = schedule::grade(&conn, flashcard_id, rating)?;
    let _ = mastery::update(&conn, g.subject_id, &g.skill, g.correct);
    profile::record_flashcard_review(&conn, g.subject_id)?;
    Ok(g.due)
}

#[derive(Serialize)]
pub struct QuizResult {
    pub correct: bool,
    pub answer_idx: i64,
    pub explanation: Option<String>,
    pub p_known: f64,
}

/// Submit a quiz answer (the chosen option index): records the attempt, updates
/// the skill's BKT mastery + pace profile, and returns the correct answer +
/// explanation so the UI can give immediate feedback (retrieval practice).
#[tauri::command]
pub fn learning_quiz_answer(
    db: State<'_, Db>,
    question_id: i64,
    choice_idx: i64,
    latency_ms: Option<i64>,
) -> Result<QuizResult, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let (subject_id, skill, answer_idx, explanation): (i64, Option<String>, i64, Option<String>) = conn
        .query_row(
            "SELECT subject_id, skill, answer_idx, explanation FROM learning_quiz_questions WHERE id = ?1",
            [question_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|_| "Question not found.".to_string())?;
    let correct = choice_idx == answer_idx;
    conn.execute(
        "INSERT INTO learning_quiz_attempts (question_id, subject_id, correct, latency_ms) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![question_id, subject_id, correct as i64, latency_ms],
    )
    .map_err(|e| e.to_string())?;
    let p_known = mastery::update(&conn, subject_id, skill.as_deref().unwrap_or(""), correct)?;
    profile::record_attempt(&conn, subject_id, correct, latency_ms.unwrap_or(0))?;
    Ok(QuizResult { correct, answer_idx, explanation, p_known })
}

/// The learner profile for a subject (pace + per-skill mastery) for the progress
/// view and the "how you learn best" summary.
#[tauri::command]
pub fn learning_progress(db: State<'_, Db>, subject_id: i64) -> Result<ProfileSnapshot, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    profile::snapshot(&conn, subject_id)
}

// ---- L5: the tutor (adaptive per-subject chat) ----

#[tauri::command]
pub fn learning_tutor_history(db: State<'_, Db>, subject_id: i64) -> Result<Vec<TutorMsg>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    tutor::history(&conn, subject_id)
}

/// Send a message to the subject's tutor. Loads the subject + bounded learner
/// profile + prior history under a brief lock, persists the user message, then
/// makes the model call WITHOUT the lock and persists the reply. The profile is
/// numbers-only (no "learning style"); the model adapts difficulty from it.
#[tauri::command]
pub fn learning_tutor_send(db: State<'_, Db>, subject_id: i64, message: String) -> Result<String, String> {
    let message = message.trim().to_string();
    if message.is_empty() {
        return Err("Type a message first.".into());
    }
    if message.chars().count() > 20_000 {
        return Err("Message is too long (max 20000 characters).".into());
    }
    let (subject, profile_block, hist) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let subject = store::get_subject(&conn, subject_id)?.ok_or("Subject not found.")?;
        let profile_block = profile::snapshot_prompt(&conn, subject_id)?;
        let hist = tutor::history(&conn, subject_id)?; // prior turns (before this message)
        tutor::add(&conn, subject_id, "user", &message)?;
        (subject, profile_block, hist)
    };
    let reply = tutor::reply(&subject, &profile_block, &hist, &message)?; // model call, no DB lock
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    tutor::add(&conn, subject_id, "assistant", &reply)?;
    Ok(reply)
}

// ---- L6: upload ingest (PDF → text; text/markdown are read in the frontend) ----

/// Extract text from an uploaded PDF's bytes to seed a subject. Bounded + panic-
/// safe; degrades to a clear "paste the text instead" error on failure.
#[tauri::command]
pub fn learning_extract_pdf(bytes: Vec<u8>) -> Result<String, String> {
    if bytes.len() > 25_000_000 {
        return Err("That PDF is too large (max 25 MB). Paste the relevant text instead.".into());
    }
    super::ingest::extract_pdf(&bytes)
}
