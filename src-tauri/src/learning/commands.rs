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
use super::embed::{Embedder, OllamaEmbedder};
use super::retrieve;
use super::{mastery, schedule};
use crate::db::Db;
use crate::model::commands::provider_for;
use crate::settings::load_model_config;

const MAX_TITLE_CHARS: usize = 200;
/// Hard cap with a loud error — never a silent cut (chunked ingest covers the
/// whole document; the old 40k truncation meant materials ignored most of it).
const MAX_SOURCE_CHARS: usize = 2_000_000;

/// Per-subject async gate: a background generation locks its subject's mutex so
/// two generations for the same subject can't interleave, while different
/// subjects still run concurrently.
#[derive(Default)]
pub struct LearningGate(pub Mutex<HashMap<i64, Arc<Mutex<()>>>>);

impl LearningGate {
    pub fn for_subject(&self, subject_id: i64) -> Arc<Mutex<()>> {
        // Recover from poisoning: the map carries no invariant, and a panic in
        // one generation must not brick the gate for every later one.
        let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        map.entry(subject_id).or_default().clone()
    }
}

/// Create a study subject from a described goal (`describe`) or extracted upload
/// text (`upload`). Validates the title and bounds the source text.
#[tauri::command]
pub async fn subject_create(
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
    if source_text.chars().count() > MAX_SOURCE_CHARS {
        return Err("That material is enormous (over 2M characters). Split it and upload the part you're studying.".into());
    }
    let source: String = source_text.trim().to_string();
    let id = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        store::create_subject(&conn, title, &source_kind, &source)?
    };
    if source_kind == "upload" && !source.is_empty() {
        // Index for retrieval: chunk + embed lock-free, then one short
        // transaction. Ollama down ⇒ keyword-only chunks (backfilled later).
        let embedder = OllamaEmbedder::default();
        let doc = retrieve::prepare_document(
            title,
            "upload",
            &source,
            embedder.available().then_some(&embedder as &dyn Embedder),
        );
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let _ = retrieve::store_document(&conn, id, &doc);
    }
    Ok(id)
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
pub async fn learning_intake(
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

    let run_key = format!("learning:{subject_id}");
    let token = crate::model::registry::register(&run_key);
    let provider = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        provider_for(&load_model_config(&conn))
    };
    let questions = intake::fetch_questions(provider.as_ref(), &subject, &token); // model call, no DB lock held
    crate::model::registry::finish(&run_key);
    let questions = questions?;
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
pub async fn learning_propose(
    app: tauri::AppHandle,
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

    let run_key = format!("learning:{subject_id}");
    let token = crate::model::registry::register(&run_key);
    let provider = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        provider_for(&load_model_config(&conn))
    };
    let emit_progress = |done: usize, total: usize| {
        use tauri::Emitter;
        let _ = app.emit(
            "learning-progress",
            serde_json::json!({ "subject_id": subject_id, "stage": "propose", "done": done, "total": total }),
        );
    };
    let learner = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        crate::profile::excerpt(&conn, crate::profile::LEARNER_FILE)
    };
    let modules = propose::fetch_modules(provider.as_ref(), &subject, &intake, &learner, &token, emit_progress); // model call, no DB lock
    crate::model::registry::finish(&run_key);
    let modules = modules?;
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
pub async fn learning_notes(db: State<'_, Db>, gate: State<'_, LearningGate>, module_id: i64) -> Result<String, String> {
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        if let Some(body) = materials::notes_get(&conn, module_id)? {
            return Ok(body);
        }
    }
    // Load the module first so the gate keys on its SUBJECT — the documented
    // per-subject serialization (gating on module_id let two modules of one
    // subject generate concurrently).
    let (subject, m) = module_grounding(&db, module_id)?;
    let glock = gate.for_subject(m.subject_id);
    let _g = glock.lock().unwrap_or_else(|e| e.into_inner());
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        if let Some(body) = materials::notes_get(&conn, module_id)? {
            return Ok(body);
        }
    }
    let run_key = format!("learning:{module_id}");
    let token = crate::model::registry::register(&run_key);
    let provider = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        provider_for(&load_model_config(&conn))
    };
    let learner = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        crate::profile::excerpt(&conn, crate::profile::LEARNER_FILE)
    };
    let body = materials::fetch_notes(provider.as_ref(), &subject, &m, &learner, &token); // model call, no DB lock
    crate::model::registry::finish(&run_key);
    let body = body?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    materials::notes_save(&conn, module_id, &body)?;
    materials::set_module_status(&conn, module_id, "ready")?;
    Ok(body)
}

/// A module's flashcards, generated + cached on first open.
#[tauri::command]
pub async fn learning_flashcards(db: State<'_, Db>, gate: State<'_, LearningGate>, module_id: i64) -> Result<Vec<Flashcard>, String> {
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let existing = materials::flashcards_list(&conn, module_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
    }
    let (subject, m) = module_grounding(&db, module_id)?;
    let glock = gate.for_subject(m.subject_id);
    let _g = glock.lock().unwrap_or_else(|e| e.into_inner());
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let existing = materials::flashcards_list(&conn, module_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
    }
    let run_key = format!("learning:{module_id}");
    let token = crate::model::registry::register(&run_key);
    let provider = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        provider_for(&load_model_config(&conn))
    };
    let learner = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        crate::profile::excerpt(&conn, crate::profile::LEARNER_FILE)
    };
    let cards = materials::fetch_flashcards(provider.as_ref(), &subject, &m, &learner, &token); // model call, no DB lock
    crate::model::registry::finish(&run_key);
    let cards = cards?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    materials::flashcards_save(&conn, module_id, m.subject_id, m.skill.as_deref().unwrap_or(""), &cards)?;
    materials::set_module_status(&conn, module_id, "ready")?;
    materials::flashcards_list(&conn, module_id)
}

/// A module's quiz questions, generated + cached on first open.
#[tauri::command]
pub async fn learning_quiz(db: State<'_, Db>, gate: State<'_, LearningGate>, module_id: i64) -> Result<Vec<QuizQuestion>, String> {
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let existing = materials::quiz_list(&conn, module_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
    }
    let (subject, m) = module_grounding(&db, module_id)?;
    let glock = gate.for_subject(m.subject_id);
    let _g = glock.lock().unwrap_or_else(|e| e.into_inner());
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let existing = materials::quiz_list(&conn, module_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
    }
    let run_key = format!("learning:{module_id}");
    let token = crate::model::registry::register(&run_key);
    let provider = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        provider_for(&load_model_config(&conn))
    };
    let learner = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        crate::profile::excerpt(&conn, crate::profile::LEARNER_FILE)
    };
    let questions = materials::fetch_quiz(provider.as_ref(), &subject, &m, &learner, &token); // model call, no DB lock
    crate::model::registry::finish(&run_key);
    let questions = questions?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    materials::quiz_save(&conn, module_id, m.subject_id, m.skill.as_deref().unwrap_or(""), &questions)?;
    materials::set_module_status(&conn, module_id, "ready")?;
    materials::quiz_list(&conn, module_id)
}

/// The study queue for a module: due cards first, then new cards up to the
/// session cap, plus the next future due date for the empty state.
#[tauri::command]
pub fn learning_flashcards_queue(db: State<'_, Db>, module_id: i64) -> Result<materials::StudyQueue, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    materials::flashcards_queue(&conn, module_id)
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
    crate::profile::record(&conn, "flashcard_grade", Some(g.subject_id), None, &serde_json::json!({ "rating": rating }));
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
    crate::profile::record(&conn, "quiz_answer", Some(subject_id), None, &serde_json::json!({ "correct": correct, "latency_ms": latency_ms.unwrap_or(0) }));
    Ok(QuizResult { correct, answer_idx, explanation, p_known })
}

/// The learner profile for a subject (pace + per-skill mastery) for the progress
/// view and the "how you learn best" summary.
#[tauri::command]
pub fn learning_progress(db: State<'_, Db>, subject_id: i64) -> Result<ProfileSnapshot, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    profile::snapshot(&conn, subject_id)
}

/// Per-subject opt-in for labeled web fallback in the tutor.
#[tauri::command]
pub fn subject_set_web(db: State<'_, Db>, subject_id: i64, enabled: bool) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store::set_web_fallback(&conn, subject_id, enabled)
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
#[derive(Serialize)]
pub struct TutorReply {
    pub reply: String,
    /// "local" | "web" | "mixed" | "none"
    pub grounding: String,
    /// Citation labels for the [n] markers (doc › section).
    pub sources: Vec<String>,
}

#[tauri::command]
pub async fn learning_tutor_send(db: State<'_, Db>, subject_id: i64, message: String) -> Result<TutorReply, String> {
    let message = message.trim().to_string();
    if message.is_empty() {
        return Err("Type a message first.".into());
    }
    if message.chars().count() > 20_000 {
        return Err("Message is too long (max 20000 characters).".into());
    }
    // The user message is NOT persisted yet: a failed model call must leave no
    // orphaned turn (retry would duplicate it in history and in the prompt).
    let (subject, profile_block, hist) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let subject = store::get_subject(&conn, subject_id)?.ok_or("Subject not found.")?;
        let mut profile_block = profile::snapshot_prompt(&conn, subject_id)?;
        profile_block.push_str(&crate::profile::excerpt(&conn, crate::profile::LEARNER_FILE));
        let hist = tutor::history(&conn, subject_id)?; // prior turns (before this message)
        (subject, profile_block, hist)
    };

    // ---- retrieval (RAG): embed lock-free, then search under a short lock ----
    let query = {
        // Deterministic follow-up rewrite: a short message borrows the previous
        // user turn for context — no extra LLM call on the common path.
        let prev = hist.iter().rev().find(|m| m.role == "user").map(|m| m.content.clone());
        match prev {
            Some(p) if message.chars().count() < 60 => format!("{p}\n{message}"),
            _ => message.clone(),
        }
    };
    let embedder = OllamaEmbedder::default();
    let query_vec = embedder.available().then(|| embedder.embed_query(&query).ok()).flatten();
    let retrieval = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        retrieve::search(&conn, query_vec.as_deref(), subject_id, &query)
    };

    let run_key = format!("tutor:{subject_id}");
    let token = crate::model::registry::register(&run_key);
    let provider = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        provider_for(&load_model_config(&conn))
    };

    // CRAG-lite: only when retrieval looks shaky does ONE cheap haiku call
    // grade sufficiency — the confident path adds zero extra LLM calls.
    let mut sufficient = !retrieval.hits.is_empty();
    if sufficient && retrieval.confidence < 0.55 {
        let summaries: String = retrieval
            .hits
            .iter()
            .enumerate()
            .map(|(i, h)| format!("[{}] {} — {}\n", i + 1, h.header, h.body.chars().take(200).collect::<String>()))
            .collect();
        let mut grade = crate::model::ModelRequest::grounded(format!(
            "Question: {query}\n\nExcerpts:\n{summaries}\nCan the question be answered from these excerpts alone? Reply with exactly SUFFICIENT or INSUFFICIENT."
        ));
        grade.allowed_tools = vec![];
        grade.model = Some("haiku".into());
        if let Ok(v) = super::gen::run_req(provider.as_ref(), grade, &token) {
            sufficient = !v.to_uppercase().contains("INSUFFICIENT");
        }
    }

    let web_path = subject.web_fallback && (!sufficient || retrieval.hits.is_empty());
    let (excerpts, sources) = if retrieval.hits.is_empty() {
        (String::new(), vec![])
    } else {
        retrieve::excerpts_block(&retrieval.hits)
    };

    let reply = tutor::reply(
        provider.as_ref(),
        &subject,
        &profile_block,
        &hist,
        &message,
        &excerpts,
        web_path,
        &token,
    ); // model call, no DB lock
    crate::model::registry::finish(&run_key);
    let reply = reply?;

    let grounding = if web_path {
        if retrieval.hits.is_empty() { "web" } else { "mixed" }
    } else if retrieval.hits.is_empty() {
        "none"
    } else {
        "local"
    };

    let conn = db.0.lock().map_err(|e| e.to_string())?;
    // Both sides of the turn persist atomically — never an orphaned half.
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tutor::add_with_grounding(&tx, subject_id, "user", &message, "local")?;
    tutor::add_with_grounding(&tx, subject_id, "assistant", &reply, grounding)?;
    tx.commit().map_err(|e| e.to_string())?;
    crate::profile::record(&conn, "tutor_turn", Some(subject_id), None, &serde_json::json!({ "chars": reply.chars().count(), "grounding": grounding }));
    Ok(TutorReply { reply, grounding: grounding.to_string(), sources })
}

// ---- L6: upload ingest (PDF → text; text/markdown are read in the frontend) ----

/// Extract text from an uploaded PDF (base64-encoded) to seed a subject.
/// Base64 because a 25 MB file as a JSON number array froze the webview while
/// serializing. Bounded + panic-safe; degrades to a clear "paste the text
/// instead" error on failure.
#[tauri::command]
pub async fn learning_extract_pdf(bytes_b64: String) -> Result<String, String> {
    use base64::Engine;
    if bytes_b64.len() > 34_000_000 {
        // ~25 MB binary at 4/3 base64 expansion.
        return Err("That PDF is too large (max 25 MB). Paste the relevant text instead.".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(bytes_b64.trim())
        .map_err(|_| "That upload couldn't be decoded. Try the file again, or paste the text instead.")?;
    if bytes.len() > 25_000_000 {
        return Err("That PDF is too large (max 25 MB). Paste the relevant text instead.".into());
    }
    super::ingest::extract_pdf(&bytes)
}
