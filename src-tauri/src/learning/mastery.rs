//! Bayesian Knowledge Tracing (BKT) — the per-skill mastery estimate that makes
//! Learning mode adaptive. Each observation (a quiz answer, or a flashcard graded
//! Good/Easy vs Again/Hard) updates a probability that the learner *knows* the
//! skill. This is the evidence-based learner model the deep-research recommended
//! — NOT a "learning style" (that theory is debunked). A small fixed-parameter
//! HMM (Corbett & Anderson 1995), ported to Rust against SQLite.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

// Fixed BKT parameters. Per-skill fitting needs far more data than a single
// learner produces, so we use sensible global defaults (the research notes
// per-user fitting is unstable at this scale).
const P_INIT: f64 = 0.3; // prior P(known) before any evidence
const P_LEARN: f64 = 0.2; // P(transition unknown → known) per opportunity
const P_SLIP: f64 = 0.1; // P(slip): knows it but answers wrong
const P_GUESS: f64 = 0.25; // P(guess): doesn't know it but answers right

#[derive(Debug, Serialize, PartialEq)]
pub struct SkillMastery {
    pub skill: String,
    pub p_known: f64,
    pub n_obs: i64,
}

/// Apply one observation to a skill's mastery and persist it; returns the updated
/// P(known). A no-op (returns the prior) for an empty skill tag.
pub fn update(conn: &Connection, subject_id: i64, skill: &str, correct: bool) -> Result<f64, String> {
    let skill = skill.trim();
    if skill.is_empty() {
        return Ok(P_INIT);
    }
    let prior: f64 = conn
        .query_row(
            "SELECT p_known FROM learning_skill_mastery WHERE subject_id = ?1 AND skill = ?2",
            params![subject_id, skill],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(P_INIT);

    // Posterior P(known | observation), then the learning transition.
    let post = if correct {
        prior * (1.0 - P_SLIP) / (prior * (1.0 - P_SLIP) + (1.0 - prior) * P_GUESS)
    } else {
        prior * P_SLIP / (prior * P_SLIP + (1.0 - prior) * (1.0 - P_GUESS))
    };
    let p_new = (post + (1.0 - post) * P_LEARN).clamp(0.0, 1.0);

    conn.execute(
        "INSERT INTO learning_skill_mastery (subject_id, skill, p_known, n_obs) VALUES (?1, ?2, ?3, 1) \
         ON CONFLICT(subject_id, skill) DO UPDATE SET p_known = ?3, n_obs = n_obs + 1, updated_at = datetime('now')",
        params![subject_id, skill, p_new],
    )
    .map_err(|e| e.to_string())?;
    Ok(p_new)
}

pub fn list(conn: &Connection, subject_id: i64) -> Result<Vec<SkillMastery>, String> {
    let mut stmt = conn
        .prepare("SELECT skill, p_known, n_obs FROM learning_skill_mastery WHERE subject_id = ?1 ORDER BY skill")
        .map_err(|e| e.to_string())?;
    stmt.query_map([subject_id], |r| {
        Ok(SkillMastery { skill: r.get(0)?, p_known: r.get(1)?, n_obs: r.get(2)? })
    })
    .and_then(Iterator::collect)
    .map_err(|e| e.to_string())
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
    fn correct_answers_raise_mastery_wrong_ones_lower_it() {
        let conn = db();
        let p0 = update(&conn, 1, "vectors", true).unwrap();
        assert!(p0 > P_INIT, "a correct answer raises mastery above the prior");
        let p1 = update(&conn, 1, "vectors", true).unwrap();
        assert!(p1 > p0, "consecutive correct answers keep raising it");

        let before_wrong = p1;
        let p2 = update(&conn, 1, "vectors", false).unwrap();
        assert!(p2 < before_wrong, "a wrong answer pulls mastery back down");

        let listed = list(&conn, 1).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].n_obs, 3);
        assert!((0.0..=1.0).contains(&listed[0].p_known));
    }

    #[test]
    fn empty_skill_is_a_noop() {
        let conn = db();
        assert_eq!(update(&conn, 1, "  ", true).unwrap(), P_INIT);
        assert!(list(&conn, 1).unwrap().is_empty());
    }
}
