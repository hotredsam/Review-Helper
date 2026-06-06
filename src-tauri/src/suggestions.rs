//! Pending suggestions — inferred updates (from chat, T2) that the user
//! approves before anything reaches the record. Phase 9 adds the approve/dismiss
//! pane; this module owns the data layer + creation.

use rusqlite::{params, Connection};
use serde::Serialize;
use serde_json::Value;

pub mod commands;

/// A parsed-but-not-yet-stored suggestion (kind + raw JSON payload string).
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedSuggestion {
    pub kind: String,
    pub payload: String,
}

const VALID_KINDS: [&str; 4] = ["decision", "answer", "feature", "stack"];

/// Max stored payload size (bytes). Bounds DB growth on untrusted model output;
/// real payloads are a few hundred bytes.
const MAX_PAYLOAD: usize = 10_000;

pub fn is_valid_kind(kind: &str) -> bool {
    VALID_KINDS.contains(&kind)
}

/// Whether a payload has the non-empty string fields its kind requires. Keeps
/// half-formed suggestions out of the record so Phase 9 approval can trust them.
fn valid_payload(kind: &str, payload: &str) -> bool {
    let v: Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let obj = match v.as_object() {
        Some(o) => o,
        None => return false,
    };
    let has = |k: &str| obj.get(k).and_then(Value::as_str).map(|s| !s.trim().is_empty()).unwrap_or(false);
    match kind {
        "decision" => has("topic") && has("choice"),
        "feature" => has("title"),
        "stack" => has("pane") && has("choice"),
        "answer" => has("question") && has("answer"),
        _ => false,
    }
}

/// Persist parsed suggestions as pending, atomically. Skips invalid kinds,
/// oversized payloads, and payloads missing required fields. Returns the count
/// added (only well-formed rows).
pub fn save(conn: &mut Connection, project_id: i64, items: &[ParsedSuggestion]) -> Result<usize, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut added = 0;
    for it in items {
        if !is_valid_kind(&it.kind) || it.payload.len() > MAX_PAYLOAD || !valid_payload(&it.kind, &it.payload) {
            continue;
        }
        tx.execute(
            "INSERT INTO suggestions (project_id, kind, payload, status) VALUES (?1, ?2, ?3, 'pending')",
            params![project_id, it.kind, it.payload],
        )
        .map_err(|e| e.to_string())?;
        added += 1;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(added)
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Suggestion {
    pub id: i64,
    pub kind: String,
    pub payload: Value,
    pub status: String,
    pub created_at: String,
}

fn payload_value(s: Option<String>) -> Value {
    match s {
        Some(x) => serde_json::from_str(&x).unwrap_or_else(|e| {
            eprintln!("suggestions: stored payload is not valid JSON: {e}");
            Value::Null
        }),
        None => Value::Null,
    }
}

/// List a project's suggestions, optionally filtered by status, newest first.
pub fn list(conn: &Connection, project_id: i64, status: Option<&str>) -> Result<Vec<Suggestion>, String> {
    let row = |r: &rusqlite::Row| {
        Ok(Suggestion {
            id: r.get(0)?,
            kind: r.get(1)?,
            payload: payload_value(r.get::<_, Option<String>>(2)?),
            status: r.get(3)?,
            created_at: r.get(4)?,
        })
    };
    let base = "SELECT id, kind, payload, status, created_at FROM suggestions WHERE project_id = ?1";
    match status {
        Some(s) => {
            let mut stmt = conn
                .prepare(&format!("{base} AND status = ?2 ORDER BY id DESC"))
                .map_err(|e| e.to_string())?;
            stmt.query_map(params![project_id, s], row)
                .and_then(Iterator::collect)
                .map_err(|e| e.to_string())
        }
        None => {
            let mut stmt = conn
                .prepare(&format!("{base} ORDER BY id DESC"))
                .map_err(|e| e.to_string())?;
            stmt.query_map(params![project_id], row)
                .and_then(Iterator::collect)
                .map_err(|e| e.to_string())
        }
    }
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
        conn.execute("INSERT INTO projects (name, kind) VALUES ('S','new')", []).unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn saves_valid_kinds_as_pending_and_lists_them() {
        let mut conn = db();
        let pid = project(&conn);
        let items = vec![
            ParsedSuggestion { kind: "decision".into(), payload: r#"{"topic":"DB","choice":"SQLite"}"#.into() },
            ParsedSuggestion { kind: "feature".into(), payload: r#"{"title":"Export CSV"}"#.into() },
            ParsedSuggestion { kind: "bogus".into(), payload: "{}".into() }, // skipped
        ];
        let added = save(&mut conn, pid, &items).unwrap();
        assert_eq!(added, 2, "invalid kind skipped");

        let pending = list(&conn, pid, Some("pending")).unwrap();
        assert_eq!(pending.len(), 2);
        // newest first; payload parsed back to structured JSON.
        assert_eq!(pending[0].kind, "feature");
        assert_eq!(pending[1].payload["choice"], "SQLite");
        assert_eq!(pending[0].status, "pending");
    }

    #[test]
    fn skips_payloads_missing_required_fields_or_oversized() {
        let mut conn = db();
        let pid = project(&conn);
        let oversized = format!(r#"{{"topic":"{}","choice":"x"}}"#, "y".repeat(MAX_PAYLOAD));
        let items = vec![
            ParsedSuggestion { kind: "decision".into(), payload: r#"{"choice":"only"}"#.into() }, // no topic
            ParsedSuggestion { kind: "feature".into(), payload: r#"{"detail":"no title"}"#.into() }, // no title
            ParsedSuggestion { kind: "stack".into(), payload: r#"{"pane":"frontend"}"#.into() }, // no choice
            ParsedSuggestion { kind: "decision".into(), payload: "{}".into() }, // empty object
            ParsedSuggestion { kind: "decision".into(), payload: oversized }, // too big
            ParsedSuggestion { kind: "feature".into(), payload: r#"{"title":"Good one"}"#.into() }, // valid
        ];
        let added = save(&mut conn, pid, &items).unwrap();
        assert_eq!(added, 1, "only the complete, bounded payload persists");
        let pending = list(&conn, pid, Some("pending")).unwrap();
        assert_eq!(pending[0].payload["title"], "Good one");
    }
}
