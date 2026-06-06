//! Decisions record — the ADR-style log (topic, choice, rationale, alternatives,
//! consequences, source, status). Rows are created by approving suggestions
//! (Phase 9 T1) and by analysis; here we read them and supersede (keeping history).

use rusqlite::{params, Connection};
use serde::Serialize;

pub mod commands;

#[derive(Debug, Serialize, PartialEq)]
pub struct Decision {
    pub id: i64,
    pub topic: String,
    pub choice: String,
    pub rationale: Option<String>,
    pub alternatives: Option<String>,
    pub consequences: Option<String>,
    pub source_ref: Option<String>,
    pub status: String,
    pub created_at: String,
}

/// All decisions for a project, newest first (active and superseded — the
/// record keeps history).
pub fn list(conn: &Connection, project_id: i64) -> Result<Vec<Decision>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, topic, choice, rationale, alternatives, consequences, source_ref, status, created_at \
             FROM decisions WHERE project_id = ?1 ORDER BY id DESC",
        )
        .map_err(|e| e.to_string())?;
    stmt.query_map([project_id], |r| {
        Ok(Decision {
            id: r.get(0)?,
            topic: r.get(1)?,
            choice: r.get(2)?,
            rationale: r.get(3)?,
            alternatives: r.get(4)?,
            consequences: r.get(5)?,
            source_ref: r.get(6)?,
            status: r.get(7)?,
            created_at: r.get(8)?,
        })
    })
    .and_then(Iterator::collect)
    .map_err(|e| e.to_string())
}

/// Mark an active decision superseded — the row stays (history is preserved).
pub fn supersede(conn: &Connection, project_id: i64, decision_id: i64) -> Result<(), String> {
    let n = conn
        .execute(
            "UPDATE decisions SET status = 'superseded' WHERE id = ?1 AND project_id = ?2 AND status = 'active'",
            params![decision_id, project_id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("Decision not found or already superseded.".into());
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
        conn.execute("INSERT INTO projects (name, kind) VALUES ('D','new')", []).unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn lists_all_fields_and_supersede_keeps_history() {
        let conn = db();
        let pid = project(&conn);
        conn.execute(
            "INSERT INTO decisions (project_id, topic, choice, rationale, alternatives, consequences, source_ref, status) \
             VALUES (?1, 'Database', 'SQLite', 'local + simple', 'Postgres; files', 'no server', 'chat', 'active')",
            [pid],
        )
        .unwrap();
        let did = conn.last_insert_rowid();

        let decisions = list(&conn, pid).unwrap();
        assert_eq!(decisions.len(), 1);
        let d = &decisions[0];
        assert_eq!(d.topic, "Database");
        assert_eq!(d.choice, "SQLite");
        assert_eq!(d.rationale.as_deref(), Some("local + simple"));
        assert_eq!(d.alternatives.as_deref(), Some("Postgres; files"));
        assert_eq!(d.consequences.as_deref(), Some("no server"));
        assert_eq!(d.status, "active");

        supersede(&conn, pid, did).unwrap();
        let after = list(&conn, pid).unwrap();
        assert_eq!(after.len(), 1, "superseding keeps the row (history)");
        assert_eq!(after[0].status, "superseded");

        // re-superseding an already-superseded decision errors.
        assert!(supersede(&conn, pid, did).is_err());
    }
}
