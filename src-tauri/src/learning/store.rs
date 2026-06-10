//! Learning-mode persistence (v6): subjects and their lifecycle stage. A
//! *subject* is one thing the user wants to study, independent of code projects.
//! Stage advances intake → proposed → ready as the user grills, picks modules,
//! and generates materials. Higher layers (intake/propose/materials) build on
//! this; the adaptive learner model lives in `mastery`/`profile`.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct Subject {
    pub id: i64,
    pub title: String,
    pub source_kind: String,
    pub stage: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct SubjectDetail {
    pub id: i64,
    pub title: String,
    pub source_kind: String,
    pub source_text: Option<String>,
    pub stage: String,
    /// Per-subject opt-in: may the tutor answer from the web (labeled) when
    /// the materials don't cover a question?
    pub web_fallback: bool,
}

/// Create a subject from a described goal or extracted upload text. `source_text`
/// is bounded by the caller (commands) before it reaches here.
pub fn create_subject(
    conn: &Connection,
    title: &str,
    source_kind: &str,
    source_text: &str,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO learning_subjects (title, source_kind, source_text) VALUES (?1, ?2, ?3)",
        params![title, source_kind, source_text],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn list_subjects(conn: &Connection) -> Result<Vec<Subject>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, title, source_kind, stage, created_at, updated_at \
             FROM learning_subjects ORDER BY updated_at DESC, id DESC",
        )
        .map_err(|e| e.to_string())?;
    stmt.query_map([], |r| {
        Ok(Subject {
            id: r.get(0)?,
            title: r.get(1)?,
            source_kind: r.get(2)?,
            stage: r.get(3)?,
            created_at: r.get(4)?,
            updated_at: r.get(5)?,
        })
    })
    .and_then(Iterator::collect)
    .map_err(|e| e.to_string())
}

pub fn get_subject(conn: &Connection, id: i64) -> Result<Option<SubjectDetail>, String> {
    conn.query_row(
        "SELECT id, title, source_kind, source_text, stage, web_fallback FROM learning_subjects WHERE id = ?1",
        [id],
        |r| {
            Ok(SubjectDetail {
                id: r.get(0)?,
                title: r.get(1)?,
                source_kind: r.get(2)?,
                source_text: r.get(3)?,
                stage: r.get(4)?, web_fallback: r.get::<_, i64>(5)? != 0 })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// The bounded study source (described goal / upload excerpt) for prompts.
pub fn source_text(conn: &Connection, id: i64) -> Result<String, String> {
    let s: Option<String> = conn
        .query_row("SELECT source_text FROM learning_subjects WHERE id = ?1", [id], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();
    Ok(s.unwrap_or_default())
}

pub fn stage(conn: &Connection, id: i64) -> Result<String, String> {
    conn.query_row("SELECT stage FROM learning_subjects WHERE id = ?1", [id], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Subject not found.".into())
}

pub fn set_stage(conn: &Connection, id: i64, stage: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE learning_subjects SET stage = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![stage, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn touch(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute(
        "UPDATE learning_subjects SET updated_at = datetime('now') WHERE id = ?1",
        [id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_subject(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM learning_subjects WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;

    pub fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn
    }

    #[test]
    fn creates_lists_and_advances_subjects() {
        let conn = db();
        let id = create_subject(&conn, "Spanish A1", "describe", "I want conversational basics").unwrap();
        let list = list_subjects(&conn).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].stage, "intake");
        assert_eq!(list[0].source_kind, "describe");

        set_stage(&conn, id, "proposed").unwrap();
        assert_eq!(stage(&conn, id).unwrap(), "proposed");
        assert_eq!(source_text(&conn, id).unwrap(), "I want conversational basics");

        delete_subject(&conn, id).unwrap();
        assert!(list_subjects(&conn).unwrap().is_empty());
    }
}

/// Toggle the per-subject web-fallback opt-in.
pub fn set_web_fallback(conn: &Connection, id: i64, on: bool) -> Result<(), String> {
    conn.execute(
        "UPDATE learning_subjects SET web_fallback = ?1 WHERE id = ?2",
        rusqlite::params![if on { 1 } else { 0 }, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
