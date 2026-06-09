//! FSRS flashcard scheduling. On each grade (Again/Hard/Good/Easy) the card's
//! scheduling state (stability/difficulty/due) is advanced by the FSRS algorithm
//! and persisted; the next due date drives the review queue. FSRS is the
//! best-evidenced scheduler (beats SM-2-era heuristics on a ~350M-review
//! benchmark); we use its default parameters, which are stable for one learner.

use chrono::Utc;
use rs_fsrs::{Card, Rating, FSRS};
use rusqlite::{params, Connection};

/// Outcome of grading a card: where it lands next + whether it counts as a
/// "correct" recall (Good/Easy) for the skill's mastery update.
pub struct Graded {
    pub subject_id: i64,
    pub skill: String,
    pub due: String,
    pub correct: bool,
}

fn rating_from(n: i64) -> Rating {
    match n {
        1 => Rating::Again,
        2 => Rating::Hard,
        3 => Rating::Good,
        _ => Rating::Easy,
    }
}

/// Advance one flashcard by a grade and persist its new FSRS state. Pure DB +
/// arithmetic (no model call), so it runs under the normal command lock.
pub fn grade(conn: &Connection, flashcard_id: i64, rating_num: i64) -> Result<Graded, String> {
    let (subject_id, skill, json): (i64, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT subject_id, skill, fsrs_json FROM learning_flashcards WHERE id = ?1",
            [flashcard_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| "Flashcard not found.".to_string())?;

    // Resume the card's scheduling state, or start fresh on its first review.
    let card: Card = json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_else(Card::new);

    let rating = rating_from(rating_num);
    let log = FSRS::default().repeat(card, Utc::now());
    let info = log
        .get(&rating)
        .ok_or("The scheduler returned no card for that rating.")?;
    let next = info.card.clone();

    let new_json = serde_json::to_string(&next).map_err(|e| e.to_string())?;
    let due = next.due.to_rfc3339();
    conn.execute(
        "UPDATE learning_flashcards SET fsrs_json = ?1, due = ?2, reps = reps + 1 WHERE id = ?3",
        params![new_json, due, flashcard_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(Graded { subject_id, skill: skill.unwrap_or_default(), due, correct: rating_num >= 3 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn.execute("INSERT INTO learning_subjects (title, source_kind) VALUES ('M','describe')", []).unwrap();
        conn.execute("INSERT INTO learning_modules (subject_id, idx, kind, title, skill) VALUES (1,0,'flashcards','F','vocab')", []).unwrap();
        conn.execute(
            "INSERT INTO learning_flashcards (module_id, subject_id, skill, front, back) VALUES (1,1,'vocab','hola','hello')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn grading_advances_due_and_persists_fsrs_state() {
        let conn = db();
        let g = grade(&conn, 1, 3).unwrap(); // Good
        assert_eq!(g.subject_id, 1);
        assert_eq!(g.skill, "vocab");
        assert!(g.correct, "Good counts as a correct recall");
        assert!(!g.due.is_empty());

        // State persisted: a second grade resumes from the stored card.
        let (json, reps): (Option<String>, i64) = conn
            .query_row("SELECT fsrs_json, reps FROM learning_flashcards WHERE id = 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert!(json.is_some(), "FSRS state is saved");
        assert_eq!(reps, 1);

        let again = grade(&conn, 1, 1).unwrap(); // Again
        assert!(!again.correct, "Again counts as a lapse");
    }
}
