//! Feature inbox — capture ideas (text now, audio via a stub) into `features`,
//! triage them, and (Phase 10 T2) weave them into the plan. Pending count drives
//! a soft nudge; regeneration is never automatic.

use rusqlite::{params, Connection};
use serde::Serialize;

pub mod commands;

/// Placeholder returned by the audio-transcription stub. The mic button shows
/// this until a real provider is wired.

#[derive(Debug, Serialize, PartialEq)]
pub struct Feature {
    pub id: i64,
    pub title: String,
    pub detail: Option<String>,
    pub source: Option<String>,
    pub status: String,
    pub created_at: String,
}

/// Add a captured feature to the inbox. `source` is text|audio (schema CHECK).
pub fn add(
    conn: &Connection,
    project_id: i64,
    title: &str,
    detail: &str,
    source: &str,
) -> Result<Feature, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("A feature needs a title.".into());
    }
    if title.chars().count() > 200 {
        return Err("Title is too long (max 200 characters).".into());
    }
    if detail.len() > 10_000 {
        return Err("Detail is too long (max 10000 characters).".into());
    }
    let source = if source == "audio" { "audio" } else { "text" };
    conn.execute(
        "INSERT INTO features (project_id, title, detail, source, status) VALUES (?1, ?2, ?3, ?4, 'inbox')",
        params![project_id, title, detail.trim(), source],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    get(conn, project_id, id)?.ok_or_else(|| "feature vanished after insert".into())
}

fn get(conn: &Connection, project_id: i64, id: i64) -> Result<Option<Feature>, String> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        "SELECT id, title, detail, source, status, created_at FROM features WHERE id = ?1 AND project_id = ?2",
        params![id, project_id],
        row_to_feature,
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn row_to_feature(r: &rusqlite::Row) -> rusqlite::Result<Feature> {
    Ok(Feature {
        id: r.get(0)?,
        title: r.get(1)?,
        detail: r.get(2)?,
        source: r.get(3)?,
        status: r.get(4)?,
        created_at: r.get(5)?,
    })
}

/// All features for a project, newest first.
pub fn list(conn: &Connection, project_id: i64) -> Result<Vec<Feature>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, title, detail, source, status, created_at FROM features \
             WHERE project_id = ?1 ORDER BY id DESC",
        )
        .map_err(|e| e.to_string())?;
    stmt.query_map([project_id], row_to_feature)
        .and_then(Iterator::collect)
        .map_err(|e| e.to_string())
}

/// Set a feature's triage status (inbox/triaged/in_plan/rejected).
pub fn set_status(conn: &Connection, project_id: i64, id: i64, status: &str) -> Result<(), String> {
    const ALLOWED: [&str; 4] = ["inbox", "triaged", "in_plan", "rejected"];
    if !ALLOWED.contains(&status) {
        return Err("Invalid feature status.".into());
    }
    let n = conn
        .execute(
            "UPDATE features SET status = ?1 WHERE id = ?2 AND project_id = ?3",
            params![status, id, project_id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("Feature not found.".into());
    }
    Ok(())
}

/// Count of un-incorporated features (inbox + triaged) — drives the soft nudge.
pub fn pending_count(conn: &Connection, project_id: i64) -> Result<i64, String> {
    conn.query_row(
        "SELECT count(*) FROM features WHERE project_id = ?1 AND status IN ('inbox','triaged')",
        [project_id],
        |r| r.get(0),
    )
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
        conn.execute("INSERT INTO projects (name, kind) VALUES ('F','new')", []).unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn add_lists_and_triages_features() {
        let conn = db();
        let pid = project(&conn);
        assert!(add(&conn, pid, "  ", "", "text").is_err(), "empty title rejected");

        let f = add(&conn, pid, "CSV export", "download the table", "text").unwrap();
        assert_eq!(f.status, "inbox");
        assert_eq!(f.source.as_deref(), Some("text"));
        let f2 = add(&conn, pid, "Dark mode", "", "audio").unwrap();
        assert_eq!(f2.source.as_deref(), Some("audio"));

        assert_eq!(list(&conn, pid).unwrap().len(), 2);
        assert_eq!(pending_count(&conn, pid).unwrap(), 2);

        set_status(&conn, pid, f.id, "in_plan").unwrap();
        assert_eq!(pending_count(&conn, pid).unwrap(), 1, "in_plan no longer pending");
        assert!(set_status(&conn, pid, f.id, "bogus").is_err());
    }
}
