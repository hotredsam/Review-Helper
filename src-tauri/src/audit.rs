//! Plan audit log — a source→version trail for every plan generation
//! (analyze / kickoff / update / rebuild). Stored in the generic `settings`
//! kv (the schema is fixed; no dedicated table), one JSON array per project.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub mod commands;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditEntry {
    pub version: i64,
    pub source: String,
    pub at: String,
}

fn key(project_id: i64) -> String {
    format!("audit:{project_id}")
}

/// Append a "source produced plan version N" entry.
pub fn record(conn: &Connection, project_id: i64, version: i64, source: &str) -> Result<(), String> {
    let mut entries = list(conn, project_id)?;
    let at: String = conn
        .query_row("SELECT datetime('now')", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    entries.push(AuditEntry { version, source: source.to_string(), at });
    // FIFO cap so the kv cell + read-modify-write stay bounded.
    const MAX_ENTRIES: usize = 50;
    if entries.len() > MAX_ENTRIES {
        let drop = entries.len() - MAX_ENTRIES;
        entries.drain(0..drop);
    }
    let json = serde_json::to_string(&entries).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key(project_id), json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// The audit trail for a project (oldest first), or empty.
pub fn list(conn: &Connection, project_id: i64) -> Result<Vec<AuditEntry>, String> {
    let raw: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", params![key(project_id)], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?;
    match raw {
        Some(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
        None => Ok(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;

    #[test]
    fn records_and_lists_source_to_version() {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        assert!(list(&conn, 1).unwrap().is_empty());

        record(&conn, 1, 1, "analyze").unwrap();
        record(&conn, 1, 2, "update").unwrap();
        record(&conn, 2, 1, "kickoff").unwrap(); // a different project

        let p1 = list(&conn, 1).unwrap();
        assert_eq!(p1.len(), 2);
        assert_eq!(p1[0], AuditEntry { version: 1, source: "analyze".into(), at: p1[0].at.clone() });
        assert_eq!(p1[1].version, 2);
        assert_eq!(p1[1].source, "update");
        assert_eq!(list(&conn, 2).unwrap().len(), 1, "scoped per project");
    }
}
