//! Grill — repo-specific interview questions, each with a recommended answer,
//! tagged by dimension. The bank (bank.json) supplies topics; the model writes
//! the question text + recommended answer grounded in the actual repo + plan.

use rusqlite::{params, Connection};
use serde::Serialize;

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
    pub status: String,
}

/// Persist a generated batch as open questions. Returns the count added.
pub fn save_questions(conn: &Connection, project_id: i64, qs: &[GenQuestion]) -> Result<usize, String> {
    let mut added = 0;
    for q in qs {
        conn.execute(
            "INSERT INTO questions (project_id, dimension, bank_topic, text, recommended_answer, status) \
             VALUES (?1, ?2, ?3, ?4, ?5, 'open')",
            params![
                project_id,
                q.dimension.trim(),
                q.bank_topic.trim(),
                q.question.trim(),
                q.recommended_answer.trim(),
            ],
        )
        .map_err(|e| e.to_string())?;
        added += 1;
    }
    Ok(added)
}

/// All non-deleted questions for a project, oldest first.
pub fn list_questions(conn: &Connection, project_id: i64) -> Result<Vec<Question>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, dimension, bank_topic, text, recommended_answer, status FROM questions \
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
            status: r.get(5)?,
        })
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
        }
    }

    #[test]
    fn saves_and_lists_questions_with_recommended_answers_and_dimensions() {
        let conn = db();
        let pid = project(&conn);
        let added = save_questions(
            &conn,
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
}
