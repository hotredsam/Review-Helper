//! Grill — repo-specific interview questions, each with a recommended answer,
//! tagged by dimension. The bank (bank.json) supplies topics; the model writes
//! the question text + recommended answer grounded in the actual repo + plan.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;

pub mod commands;
pub mod generate;

pub use generate::{
    grill_user, parse_questions, select_topics, GenQuestion, GRILL_SYSTEM,
};

#[derive(Debug, Serialize, PartialEq)]
pub struct Question {
    pub id: i64,
    pub dimension: Option<String>,
    pub bank_topic: Option<String>,
    pub text: String,
    pub recommended_answer: Option<String>,
    pub ui_spec: Option<Value>,
    pub status: String,
    pub doc_ref: Option<String>,
}

/// Persist a generated batch as open questions, atomically. Returns the count
/// added (a mid-batch failure rolls back — no orphan rows / half-saved batch).
pub fn save_questions(conn: &mut Connection, project_id: i64, qs: &[GenQuestion]) -> Result<usize, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut added = 0;
    for q in qs {
        let ui_spec = q.ui_spec.as_ref().and_then(|s| serde_json::to_string(s).ok());
        tx.execute(
            "INSERT INTO questions (project_id, dimension, bank_topic, text, recommended_answer, ui_spec, doc_ref, status) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'open')",
            params![
                project_id,
                q.dimension.trim(),
                q.bank_topic.trim(),
                q.question.trim(),
                q.recommended_answer.trim(),
                ui_spec,
                q.doc_ref,
            ],
        )
        .map_err(|e| e.to_string())?;
        added += 1;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(added)
}

/// All non-deleted questions for a project, oldest first.
pub fn list_questions(conn: &Connection, project_id: i64) -> Result<Vec<Question>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, dimension, bank_topic, text, recommended_answer, ui_spec, doc_ref, status FROM questions \
             WHERE project_id = ?1 AND status != 'deleted' ORDER BY id",
        )
        .map_err(|e| e.to_string())?;
    stmt.query_map([project_id], |r| {
        Ok(Question {
            id: r.get(0)?,
            dimension: r.get(1)?,
            bank_topic: r.get(2)?,
            text: r.get(3)?,
            recommended_answer: r.get(4)?,
            ui_spec: r.get::<_, Option<String>>(5)?.and_then(|s| serde_json::from_str(&s).ok()),
            doc_ref: r.get(6)?,
            status: r.get(7)?,
        })
    })
    .and_then(Iterator::collect)
    .map_err(|e| e.to_string())
}

/// Record an answer for a question and mark it answered. `source` is one of
/// typed/audio/chat (schema CHECK). Used by Submit (typed) and the chat
/// resolution path (chat) — a "Let's chat" outcome writes back into the card.
pub fn answer_question(
    conn: &mut Connection,
    project_id: i64,
    question_id: i64,
    body: &str,
    source: &str,
) -> Result<(), String> {
    let body = body.trim();
    if body.is_empty() {
        return Err("Write an answer first.".into());
    }
    if body.len() > 10_000 {
        return Err("Answer is too long (max 10000 characters).".into());
    }
    let exists = conn
        .query_row(
            "SELECT 1 FROM questions WHERE id = ?1 AND project_id = ?2 AND status != 'deleted'",
            params![question_id, project_id],
            |_| Ok(()),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .is_some();
    if !exists {
        return Err("Question not found.".into());
    }
    // Record the answer and flip the status atomically (no stranded open answer).
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO answers (question_id, project_id, body, source) VALUES (?1, ?2, ?3, ?4)",
        params![question_id, project_id, body, source],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE questions SET status = 'answered' WHERE id = ?1 AND project_id = ?2",
        params![question_id, project_id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Set a question's status (dismiss as not_relevant/unknown, soft-delete, or
/// reopen). Answered status is set via `answer_question`, not here.
pub fn set_status(conn: &Connection, project_id: i64, question_id: i64, status: &str) -> Result<(), String> {
    const ALLOWED: [&str; 4] = ["open", "not_relevant", "unknown", "deleted"];
    if !ALLOWED.contains(&status) {
        return Err("Invalid question status.".into());
    }
    let affected = conn
        .execute(
            "UPDATE questions SET status = ?1 WHERE id = ?2 AND project_id = ?3",
            params![status, question_id, project_id],
        )
        .map_err(|e| e.to_string())?;
    if affected == 0 {
        return Err("Question not found.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn
    }

    fn project(conn: &Connection) -> i64 {
        conn.execute("INSERT INTO projects (name, kind) VALUES ('G','new')", []).unwrap();
        conn.last_insert_rowid()
    }

    fn gen(dim: &str, topic: &str, q: &str, a: &str) -> GenQuestion {
        GenQuestion {
            dimension: dim.into(),
            bank_topic: topic.into(),
            question: q.into(),
            recommended_answer: a.into(),
            doc_ref: None,
            ui_spec: None,
        }
    }

    #[test]
    fn saves_and_lists_questions_with_recommended_answers_and_dimensions() {
        let mut conn = db();
        let pid = project(&conn);
        let added = save_questions(
            &mut conn,
            pid,
            &[
                gen("vision", "Core problem", "What problem does it solve?", "Tracks brisket cooks."),
                gen("users", "Primary user", "Who is it for?", "Solo pitmasters."),
            ],
        )
        .unwrap();
        assert_eq!(added, 2);

        let qs = list_questions(&conn, pid).unwrap();
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].dimension.as_deref(), Some("vision"));
        assert_eq!(qs[0].bank_topic.as_deref(), Some("Core problem"));
        assert_eq!(qs[0].recommended_answer.as_deref(), Some("Tracks brisket cooks."));
        assert_eq!(qs[0].status, "open");
    }

    #[test]
    fn answering_records_an_answer_and_marks_answered() {
        let mut conn = db();
        let pid = project(&conn);
        save_questions(&mut conn, pid, &[gen("vision", "Core problem", "Q?", "rec")]).unwrap();
        let qid = list_questions(&conn, pid).unwrap()[0].id;

        // Empty answer rejected.
        assert!(answer_question(&mut conn, pid, qid, "   ", "typed").is_err());

        answer_question(&mut conn, pid, qid, "Solo pitmasters tracking cooks.", "typed").unwrap();
        assert_eq!(list_questions(&conn, pid).unwrap()[0].status, "answered");
        let body: String = conn
            .query_row("SELECT body FROM answers WHERE question_id = ?1", [qid], |r| r.get(0))
            .unwrap();
        assert_eq!(body, "Solo pitmasters tracking cooks.");
    }

    #[test]
    fn lets_chat_resolution_writes_back_into_the_card() {
        let mut conn = db();
        let pid = project(&conn);
        save_questions(&mut conn, pid, &[gen("scope", "MVP boundary", "Q?", "rec")]).unwrap();
        let qid = list_questions(&conn, pid).unwrap()[0].id;

        // The chat resolution path stores a chat-sourced answer + marks answered.
        answer_question(&mut conn, pid, qid, "We settled on a read-only v1.", "chat").unwrap();
        assert_eq!(list_questions(&conn, pid).unwrap()[0].status, "answered");
        let src: String = conn
            .query_row("SELECT source FROM answers WHERE question_id = ?1", [qid], |r| r.get(0))
            .unwrap();
        assert_eq!(src, "chat");
    }

    #[test]
    fn dismiss_and_delete_behave() {
        let mut conn = db();
        let pid = project(&conn);
        save_questions(
            &mut conn,
            pid,
            &[gen("ux", "First run", "a", "r"), gen("ux", "Error states", "b", "r")],
        )
        .unwrap();
        let ids: Vec<i64> = list_questions(&conn, pid).unwrap().iter().map(|q| q.id).collect();

        set_status(&conn, pid, ids[0], "not_relevant").unwrap();
        assert_eq!(list_questions(&conn, pid).unwrap()[0].status, "not_relevant");

        set_status(&conn, pid, ids[1], "deleted").unwrap();
        let after = list_questions(&conn, pid).unwrap();
        assert_eq!(after.len(), 1, "deleted questions drop out of the list");

        assert!(set_status(&conn, pid, ids[0], "bogus").is_err(), "invalid status rejected");
        assert!(set_status(&conn, pid, 9999, "open").is_err(), "missing question rejected");
    }
}
