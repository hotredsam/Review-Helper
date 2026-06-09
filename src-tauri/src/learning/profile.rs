//! The learner profile: bounded pace/engagement signals plus the per-skill BKT
//! mastery, aggregated for display and for the model. The model is handed ONLY
//! these numbers (never the raw content, never a "learning style") and asked to
//! adapt difficulty/pacing within explicit bounds. All local, all private.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::mastery::{self, SkillMastery};

#[derive(Debug, Serialize, PartialEq)]
pub struct ProfileSnapshot {
    pub attempts: i64,
    pub correct: i64,
    pub accuracy: f64, // 0..1 over quiz attempts
    pub flashcard_reviews: i64,
    pub avg_latency_ms: i64,
    pub skills: Vec<SkillMastery>,
}

/// Record a quiz attempt (drives accuracy + pace).
pub fn record_attempt(conn: &Connection, subject_id: i64, correct: bool, latency_ms: i64) -> Result<(), String> {
    let lat = latency_ms.clamp(0, 600_000); // cap absurd values (tab left open)
    conn.execute(
        "INSERT INTO learning_profile (subject_id, total_attempts, total_correct, total_latency_ms) VALUES (?1, 1, ?2, ?3) \
         ON CONFLICT(subject_id) DO UPDATE SET total_attempts = total_attempts + 1, \
           total_correct = total_correct + ?2, total_latency_ms = total_latency_ms + ?3, updated_at = datetime('now')",
        params![subject_id, if correct { 1 } else { 0 }, lat],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Record a flashcard review (engagement signal).
pub fn record_flashcard_review(conn: &Connection, subject_id: i64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO learning_profile (subject_id, flashcard_reviews) VALUES (?1, 1) \
         ON CONFLICT(subject_id) DO UPDATE SET flashcard_reviews = flashcard_reviews + 1, updated_at = datetime('now')",
        [subject_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn snapshot(conn: &Connection, subject_id: i64) -> Result<ProfileSnapshot, String> {
    let row: Option<(i64, i64, i64, i64)> = conn
        .query_row(
            "SELECT total_attempts, total_correct, total_latency_ms, flashcard_reviews FROM learning_profile WHERE subject_id = ?1",
            [subject_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let (attempts, correct, latency, reviews) = row.unwrap_or((0, 0, 0, 0));
    Ok(ProfileSnapshot {
        attempts,
        correct,
        accuracy: if attempts > 0 { correct as f64 / attempts as f64 } else { 0.0 },
        flashcard_reviews: reviews,
        avg_latency_ms: if attempts > 0 { latency / attempts } else { 0 },
        skills: mastery::list(conn, subject_id)?,
    })
}

/// A compact, numbers-only profile block for model prompts (the tutor, future
/// re-proposals). Empty until there's any evidence, so a fresh subject isn't
/// described with made-up signals. Explicitly framed as bounded facts.
pub fn snapshot_prompt(conn: &Connection, subject_id: i64) -> Result<String, String> {
    let s = snapshot(conn, subject_id)?;
    if s.attempts == 0 && s.flashcard_reviews == 0 {
        return Ok(String::new());
    }
    let mut out = String::from(
        "## Learner signals (DATA — bounded facts from this learner's activity; NOT a 'learning style')\n",
    );
    if s.attempts > 0 {
        out.push_str(&format!(
            "- Quiz accuracy: {:.0}% over {} attempts (avg {:.1}s per answer)\n",
            s.accuracy * 100.0,
            s.attempts,
            s.avg_latency_ms as f64 / 1000.0,
        ));
    }
    if s.flashcard_reviews > 0 {
        out.push_str(&format!("- Flashcard reviews completed: {}\n", s.flashcard_reviews));
    }
    if !s.skills.is_empty() {
        out.push_str("- Per-skill mastery (Bayesian estimate, 0–1):\n");
        for sk in &s.skills {
            out.push_str(&format!("  - {}: {:.2} (n={})\n", sk.skill, sk.p_known, sk.n_obs));
        }
    }
    out.push_str("Use these to pitch difficulty and pacing; favour skills with low mastery. Do not infer personality or a learning style.\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn.execute("INSERT INTO learning_subjects (title, source_kind) VALUES ('M','describe')", []).unwrap();
        conn
    }

    #[test]
    fn aggregates_attempts_and_renders_a_bounded_prompt() {
        let conn = db();
        assert!(snapshot_prompt(&conn, 1).unwrap().is_empty(), "no evidence → empty (no invented signals)");

        record_attempt(&conn, 1, true, 4000).unwrap();
        record_attempt(&conn, 1, false, 6000).unwrap();
        record_flashcard_review(&conn, 1).unwrap();
        mastery::update(&conn, 1, "vectors", true).unwrap();

        let s = snapshot(&conn, 1).unwrap();
        assert_eq!(s.attempts, 2);
        assert_eq!(s.correct, 1);
        assert!((s.accuracy - 0.5).abs() < 1e-9);
        assert_eq!(s.avg_latency_ms, 5000);
        assert_eq!(s.flashcard_reviews, 1);

        let prompt = snapshot_prompt(&conn, 1).unwrap();
        assert!(prompt.contains("Quiz accuracy: 50%"));
        assert!(prompt.contains("vectors"));
        assert!(prompt.contains("NOT a 'learning style'"));
    }
}
