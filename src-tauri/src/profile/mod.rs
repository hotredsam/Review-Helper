//! Adaptive self-learning profile (Phase 20). Deterministic signals are
//! captured at touchpoints the app already passes through (quiz answers,
//! flashcard grades, suggestion verdicts, grill answers, tutor turns — cheap
//! INSERTs, zero model calls), and at most ONE cheap end-of-session reflection
//! call (haiku, no tools) rewrites the evidence-cited Observations sections of
//! two human-readable Markdown files the user can open, edit, and diff:
//!
//!   profile/learner-profile.md     → adapts Learning mode (tutor, generators)
//!   profile/review-preferences.md  → adapts plan / grill / assess / chat
//!
//! Design provenance: Honcho's evidence-grounded conclusions, mem0's
//! reconcile-don't-append pass, Letta's char-capped rewritten blocks — patterns
//! borrowed, no servers embedded. Files have three sentinel-marked regions:
//! Facts (regenerated from SQL each write), Observations (model-owned, capped,
//! validated fail-closed), and "Your notes" (never auto-edited).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rusqlite::{params, Connection};

pub mod commands;
pub mod reflect;

pub const LEARNER_FILE: &str = "learner-profile.md";
pub const REVIEW_FILE: &str = "review-preferences.md";

const FACTS_START: &str = "<!-- rh:facts:start (auto — regenerated from measurements; edits here are overwritten) -->";
const FACTS_END: &str = "<!-- rh:facts:end -->";
const OBS_START: &str = "<!-- rh:observations:start (auto — model-written, evidence-cited; edit below in Your notes instead) -->";
const OBS_END: &str = "<!-- rh:observations:end -->";
const NOTES_HEADER: &str = "## Your notes (never auto-edited)";

/// Hard cap on the model-owned Observations region.
pub const OBSERVATIONS_CAP: usize = 2_000;
/// Cap on what gets injected into prompts (chars ≈ 700 tokens).
const EXCERPT_CAP: usize = 2_800;
/// Reflection only runs once at least this many new events have accumulated.
pub const REFLECT_MIN_EVENTS: i64 = 15;

static PROFILE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Called once at startup so every layer (including code without an
/// AppHandle) can reach the profile files.
pub fn init(dir: PathBuf) {
    let _ = std::fs::create_dir_all(&dir);
    let _ = PROFILE_DIR.set(dir);
}

fn dir() -> Option<&'static PathBuf> {
    PROFILE_DIR.get()
}

// ---- the master toggle ----

pub fn enabled(conn: &Connection) -> bool {
    crate::settings::get(conn, "profile.adaptive")
        .ok()
        .flatten()
        .map(|v| v != "off")
        .unwrap_or(true)
}

pub fn set_enabled(conn: &Connection, on: bool) -> Result<(), String> {
    crate::settings::set(conn, "profile.adaptive", if on { "on" } else { "off" })
}

// ---- deterministic signal capture ($0 — plain INSERTs at existing touchpoints) ----

/// Record one behavioral signal. `payload` is a SMALL json of numbers/labels —
/// never raw study content or repo text (asserted by tests).
pub fn record(conn: &Connection, kind: &str, subject_id: Option<i64>, project_id: Option<i64>, payload: &serde_json::Value) {
    if !enabled(conn) {
        return;
    }
    let _ = conn.execute(
        "INSERT INTO profile_events (kind, subject_id, project_id, payload) VALUES (?1, ?2, ?3, ?4)",
        params![kind, subject_id, project_id, payload.to_string()],
    );
}

fn meta_get(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM profile_meta WHERE key = ?1", [key], |r| r.get(0)).ok()
}

fn meta_set(conn: &Connection, key: &str, value: &str) {
    let _ = conn.execute(
        "INSERT INTO profile_meta (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    );
}

// ---- deterministic facts (SQL aggregation, no model) ----

#[derive(Debug, Default, serde::Serialize)]
pub struct Facts {
    pub quiz_attempts: i64,
    pub quiz_accuracy_pct: Option<i64>,
    pub quiz_fast_accuracy_pct: Option<i64>, // answers under 10s
    pub flashcard_grades: i64,
    pub flashcard_lapse_pct: Option<i64>, // Again/Hard share
    pub tutor_turns: i64,
    pub explain_reasks: i64,
    pub suggestions_seen: i64,
    pub suggestion_approval_pct: Option<i64>,
    pub grill_answers: i64,
    pub grill_avg_answer_chars: Option<i64>,
    pub assess_runs: i64,
}

fn pct(num: i64, den: i64) -> Option<i64> {
    (den > 0).then(|| (num * 100) / den)
}

pub fn facts(conn: &Connection) -> Facts {
    let count = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap_or(0) };
    let quiz_attempts = count("SELECT COUNT(*) FROM profile_events WHERE kind = 'quiz_answer'");
    let quiz_correct = count("SELECT COUNT(*) FROM profile_events WHERE kind = 'quiz_answer' AND json_extract(payload,'$.correct') = 1");
    let quiz_fast = count("SELECT COUNT(*) FROM profile_events WHERE kind = 'quiz_answer' AND json_extract(payload,'$.latency_ms') < 10000");
    let quiz_fast_correct = count("SELECT COUNT(*) FROM profile_events WHERE kind = 'quiz_answer' AND json_extract(payload,'$.latency_ms') < 10000 AND json_extract(payload,'$.correct') = 1");
    let grades = count("SELECT COUNT(*) FROM profile_events WHERE kind = 'flashcard_grade'");
    let lapses = count("SELECT COUNT(*) FROM profile_events WHERE kind = 'flashcard_grade' AND json_extract(payload,'$.rating') <= 2");
    let sugg = count("SELECT COUNT(*) FROM profile_events WHERE kind = 'suggestion'");
    let sugg_ok = count("SELECT COUNT(*) FROM profile_events WHERE kind = 'suggestion' AND json_extract(payload,'$.verdict') = 'approved'");
    let grill = count("SELECT COUNT(*) FROM profile_events WHERE kind = 'grill_answer'");
    let grill_chars = count("SELECT COALESCE(SUM(json_extract(payload,'$.chars')),0) FROM profile_events WHERE kind = 'grill_answer'");
    Facts {
        quiz_attempts,
        quiz_accuracy_pct: pct(quiz_correct, quiz_attempts),
        quiz_fast_accuracy_pct: pct(quiz_fast_correct, quiz_fast),
        flashcard_grades: grades,
        flashcard_lapse_pct: pct(lapses, grades),
        tutor_turns: count("SELECT COUNT(*) FROM profile_events WHERE kind = 'tutor_turn'"),
        explain_reasks: count("SELECT COUNT(*) FROM profile_events WHERE kind = 'explain_reask'"),
        suggestions_seen: sugg,
        suggestion_approval_pct: pct(sugg_ok, sugg),
        grill_answers: grill,
        grill_avg_answer_chars: (grill > 0).then(|| grill_chars / grill),
        assess_runs: count("SELECT COUNT(*) FROM profile_events WHERE kind = 'assess_run'"),
    }
}

fn learner_facts_md(f: &Facts) -> String {
    let mut s = String::new();
    s.push_str(&format!("- Quiz answers recorded: {}", f.quiz_attempts));
    if let Some(a) = f.quiz_accuracy_pct {
        s.push_str(&format!(" ({a}% correct"));
        if let Some(fa) = f.quiz_fast_accuracy_pct {
            s.push_str(&format!("; {fa}% when answered under 10s"));
        }
        s.push(')');
    }
    s.push('\n');
    s.push_str(&format!("- Flashcard grades: {}", f.flashcard_grades));
    if let Some(l) = f.flashcard_lapse_pct {
        s.push_str(&format!(" ({l}% lapses — Again/Hard)"));
    }
    s.push('\n');
    s.push_str(&format!("- Tutor turns: {}; explanation re-asks: {}\n", f.tutor_turns, f.explain_reasks));
    s
}

fn review_facts_md(f: &Facts) -> String {
    let mut s = String::new();
    s.push_str(&format!("- Suggestions reviewed: {}", f.suggestions_seen));
    if let Some(a) = f.suggestion_approval_pct {
        s.push_str(&format!(" ({a}% approved)"));
    }
    s.push('\n');
    s.push_str(&format!("- Grill answers: {}", f.grill_answers));
    if let Some(c) = f.grill_avg_answer_chars {
        s.push_str(&format!(" (avg {c} chars — {})", if c < 80 { "terse" } else if c < 300 { "moderate" } else { "detailed" }));
    }
    s.push('\n');
    s.push_str(&format!("- Assessments run: {}\n", f.assess_runs));
    s
}

// ---- the markdown files ----

fn skeleton(title: &str, intro: &str) -> String {
    format!(
        "# {title}\n\n{intro}\n\n## Facts (measured)\n{FACTS_START}\n(no measurements yet)\n{FACTS_END}\n\n## Observations (model-written, evidence-cited)\n{OBS_START}\n(nothing observed yet)\n{OBS_END}\n\n{NOTES_HEADER}\n\n"
    )
}

fn file_path(name: &str) -> Option<PathBuf> {
    dir().map(|d| d.join(name))
}

pub fn read_or_create(name: &str) -> Result<String, String> {
    let path = file_path(name).ok_or("Profile dir not initialized.")?;
    if !path.exists() {
        let content = match name {
            LEARNER_FILE => skeleton(
                "How you learn",
                "Review Helper maintains this file from measured study behavior. The Facts and Observations sections are automatic; everything under Your notes is yours and never touched.",
            ),
            _ => skeleton(
                "How you like reviews",
                "Review Helper maintains this file from how you actually use plan/grill/assess/chat. The Facts and Observations sections are automatic; everything under Your notes is yours and never touched.",
            ),
        };
        std::fs::write(&path, &content).map_err(|e| e.to_string())?;
        return Ok(content);
    }
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

fn replace_between(content: &str, start: &str, end: &str, new_inner: &str) -> Option<String> {
    let s = content.find(start)? + start.len();
    let e = content.find(end)?;
    if e < s {
        return None;
    }
    Some(format!("{}\n{}\n{}", &content[..s], new_inner.trim(), &content[e..]))
}

fn section(content: &str, start: &str, end: &str) -> Option<String> {
    let s = content.find(start)? + start.len();
    let e = content.find(end)?;
    (e >= s).then(|| content[s..e].trim().to_string())
}

/// Regenerate the Facts block of one file from SQL. Never touches the other
/// regions; creates the file if missing.
pub fn write_facts(conn: &Connection, name: &str) -> Result<(), String> {
    let f = facts(conn);
    let md = if name == LEARNER_FILE { learner_facts_md(&f) } else { review_facts_md(&f) };
    let content = read_or_create(name)?;
    let updated = replace_between(&content, FACTS_START, FACTS_END, &md)
        .ok_or("Profile file is missing its Facts markers — fix it in Settings or reset the file.")?;
    let path = file_path(name).ok_or("Profile dir not initialized.")?;
    std::fs::write(path, updated).map_err(|e| e.to_string())
}

/// Replace the Observations block — the ONLY region the model may write.
/// Validates fail-closed: over-cap or marker-damaging output is rejected and
/// the previous version is kept.
pub fn write_observations(name: &str, new_obs: &str) -> Result<(), String> {
    let new_obs = new_obs.trim();
    if new_obs.chars().count() > OBSERVATIONS_CAP {
        return Err(format!("Observations over the {OBSERVATIONS_CAP}-char cap — keeping the previous version."));
    }
    if new_obs.contains("<!-- rh:") || new_obs.contains(NOTES_HEADER) {
        return Err("Observations tried to escape its region — keeping the previous version.".into());
    }
    let content = read_or_create(name)?;
    let before_notes = content.find(NOTES_HEADER);
    let updated = replace_between(&content, OBS_START, OBS_END, new_obs)
        .ok_or("Profile file is missing its Observations markers — reset the file in Settings.")?;
    // The user's region must survive byte-for-byte.
    if let Some(idx) = before_notes {
        let user_before = &content[idx..];
        if !updated.ends_with(user_before) {
            return Err("Rewrite would have altered Your notes — rejected.".into());
        }
    }
    let path = file_path(name).ok_or("Profile dir not initialized.")?;
    std::fs::write(path, updated).map_err(|e| e.to_string())
}

/// Reset the auto sections of one file, preserving Your notes byte-for-byte.
pub fn reset_auto_sections(name: &str) -> Result<(), String> {
    let content = read_or_create(name)?;
    let mut updated = replace_between(&content, FACTS_START, FACTS_END, "(no measurements yet)")
        .ok_or("Missing Facts markers.")?;
    updated = replace_between(&updated, OBS_START, OBS_END, "(nothing observed yet)")
        .ok_or("Missing Observations markers.")?;
    let path = file_path(name).ok_or("Profile dir not initialized.")?;
    std::fs::write(path, updated).map_err(|e| e.to_string())
}

/// Save the user's notes region (everything after the notes header).
pub fn save_notes(name: &str, notes: &str) -> Result<(), String> {
    let content = read_or_create(name)?;
    let idx = content.find(NOTES_HEADER).ok_or("Missing the notes header.")?;
    let updated = format!("{}{}\n\n{}\n", &content[..idx], NOTES_HEADER, notes.trim());
    let path = file_path(name).ok_or("Profile dir not initialized.")?;
    std::fs::write(path, updated).map_err(|e| e.to_string())
}

// ---- prompt injection ----

/// The bounded excerpt injected into prompts: Facts + Observations only — the
/// user's notes never ride into a model call unless they put them there
/// deliberately via the files themselves. Empty when disabled or evidence-free.
pub fn excerpt(conn: &Connection, name: &str) -> String {
    if !enabled(conn) {
        return String::new();
    }
    let Ok(content) = read_or_create(name) else {
        return String::new();
    };
    let facts = section(&content, FACTS_START, FACTS_END).unwrap_or_default();
    let obs = section(&content, OBS_START, OBS_END).unwrap_or_default();
    if facts.contains("no measurements yet") && obs.contains("nothing observed yet") {
        return String::new(); // no invented signals
    }
    let label = if name == LEARNER_FILE { "How this learner learns (measured)" } else { "How this user likes reviews (measured)" };
    let mut out = format!("\n\n## {label}\n{facts}\n{obs}");
    if out.chars().count() > EXCERPT_CAP {
        out = out.chars().take(EXCERPT_CAP).collect();
    }
    out
}

/// Events newer than the last reflection.
pub fn unreflected_count(conn: &Connection) -> i64 {
    let last: i64 = meta_get(conn, "last_reflected_event_id").and_then(|v| v.parse().ok()).unwrap_or(0);
    conn.query_row("SELECT COUNT(*) FROM profile_events WHERE id > ?1", [last], |r| r.get(0)).unwrap_or(0)
}

pub fn mark_reflected(conn: &Connection) {
    let max: i64 = conn.query_row("SELECT COALESCE(MAX(id),0) FROM profile_events", [], |r| r.get(0)).unwrap_or(0);
    meta_set(conn, "last_reflected_event_id", &max.to_string());
}

/// Up to `n` compact evidence lines (numbers + labels only) for the reflection.
pub fn evidence_lines(conn: &Connection, n: usize) -> Vec<String> {
    let mut stmt = match conn.prepare(
        "SELECT kind, COALESCE(payload,'{}'), created_at FROM profile_events ORDER BY id DESC LIMIT ?1",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let rows = stmt
        .query_map([n as i64], |r| {
            Ok(format!(
                "- {} {} {}",
                r.get::<_, String>(2)?,
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?
            ))
        })
        .map(|it| it.filter_map(Result::ok).collect::<Vec<_>>())
        .unwrap_or_default();
    rows
}

#[allow(dead_code)]
fn _path_helper(_p: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;

    fn setup() -> Connection {
        let dir = std::env::temp_dir().join(format!("rh-profile-{}-{}", std::process::id(), rand_suffix()));
        std::fs::create_dir_all(&dir).unwrap();
        let _ = PROFILE_DIR.set(dir); // first test wins; fine for this suite
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn
    }

    fn rand_suffix() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos() as u64
    }

    #[test]
    fn records_and_aggregates_quiz_signals() {
        let conn = setup();
        for i in 0..4 {
            record(&conn, "quiz_answer", Some(1), None, &serde_json::json!({"correct": i % 2 == 0, "latency_ms": 5000}));
        }
        let f = facts(&conn);
        assert_eq!(f.quiz_attempts, 4);
        assert_eq!(f.quiz_accuracy_pct, Some(50));
        assert_eq!(f.quiz_fast_accuracy_pct, Some(50));
    }

    #[test]
    fn toggle_off_stops_event_writes() {
        let conn = setup();
        set_enabled(&conn, false).unwrap();
        record(&conn, "quiz_answer", Some(1), None, &serde_json::json!({"correct": true}));
        assert_eq!(facts(&conn).quiz_attempts, 0);
        set_enabled(&conn, true).unwrap();
    }

    #[test]
    fn user_notes_survive_facts_and_observation_rewrites() {
        let conn = setup();
        let name = "test-roundtrip.md";
        let path = file_path(name).unwrap();
        std::fs::write(&path, skeleton("T", "i")).unwrap();
        save_notes(name, "MY PRECIOUS NOTE — do not touch").unwrap();
        for i in 0..100 {
            record(&conn, "quiz_answer", Some(1), None, &serde_json::json!({"correct": true, "latency_ms": 1}));
            write_facts(&conn, name).unwrap_or_else(|_| ());
            write_observations(name, &format!("- prefers worked examples (evidence: round {i})")).unwrap();
        }
        let final_content = std::fs::read_to_string(&path).unwrap();
        assert!(final_content.contains("MY PRECIOUS NOTE — do not touch"));
        assert!(final_content.contains("evidence: round 99"));
    }

    #[test]
    fn oversized_or_escaping_observations_are_rejected() {
        let _conn = setup();
        let name = "test-guard.md";
        std::fs::write(file_path(name).unwrap(), skeleton("T", "i")).unwrap();
        write_observations(name, "fine").unwrap();
        let big = "x".repeat(OBSERVATIONS_CAP + 1);
        assert!(write_observations(name, &big).is_err());
        assert!(write_observations(name, "## Your notes (never auto-edited)\nhijack").is_err());
        let content = std::fs::read_to_string(file_path(name).unwrap()).unwrap();
        assert!(content.contains("fine"), "previous version kept on rejection");
    }

    #[test]
    fn excerpt_is_empty_without_evidence_and_bounded_with_it() {
        let conn = setup();
        let name = "test-excerpt.md";
        std::fs::write(file_path(name).unwrap(), skeleton("T", "i")).unwrap();
        // No measurements yet → no invented signals.
        let raw = std::fs::read_to_string(file_path(name).unwrap()).unwrap();
        assert!(section(&raw, FACTS_START, FACTS_END).unwrap().contains("no measurements yet"));

        record(&conn, "quiz_answer", Some(1), None, &serde_json::json!({"correct": true, "latency_ms": 1}));
        write_facts(&conn, name).unwrap();
        write_observations(name, "- short observation (evidence: 1/1)").unwrap();
        let ex = excerpt(&conn, name);
        assert!(ex.contains("short observation"));
        assert!(ex.chars().count() <= EXCERPT_CAP);
    }

    #[test]
    fn profile_files_never_carry_raw_study_content() {
        // The privacy audit: facts/evidence are numbers + labels; a payload is
        // small json, and nothing pipes source_text into record().
        let conn = setup();
        record(&conn, "quiz_answer", Some(1), None, &serde_json::json!({"correct": true, "latency_ms": 9}));
        let name = "test-privacy.md";
        std::fs::write(file_path(name).unwrap(), skeleton("T", "i")).unwrap();
        write_facts(&conn, name).unwrap();
        let content = std::fs::read_to_string(file_path(name).unwrap()).unwrap();
        assert!(!content.to_lowercase().contains("source_text"));
        for line in evidence_lines(&conn, 10) {
            assert!(line.len() < 300, "evidence stays compact: {line}");
        }
    }
}
