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
    // Per-field ceilings (chars). Complements the coarse MAX_PAYLOAD gate: a
    // single oversized field can sit under 10KB total yet still bloat the merge
    // prompt once re-injected, so cap each field at the persistence boundary.
    let within = |k: &str, max: usize| {
        obj.get(k).and_then(Value::as_str).map(|s| s.chars().count() <= max).unwrap_or(true)
    };
    if !(within("topic", 200)
        && within("choice", 500)
        && within("rationale", 1500)
        && within("title", 200)
        && within("detail", 2000)
        && within("question", 1000)
        && within("answer", 2000))
    {
        return false;
    }
    match kind {
        "decision" => has("topic") && has("choice"),
        "feature" => has("title"),
        // pane must be a real stack pane (mirrors the schema CHECK + catalog).
        "stack" => {
            has("choice")
                && obj.get("pane").and_then(Value::as_str).map(|p| crate::stack::PANES.contains(&p)).unwrap_or(false)
        }
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

use rusqlite::OptionalExtension;

fn field(p: &Value, k: &str) -> String {
    p.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string()
}

/// Write the record for one pending suggestion + mark it approved, within an
/// existing transaction. Each kind writes ONLY its own table (stack is the
/// designed exception: it records a decision too, via stack::apply_one).
fn approve_in_tx(tx: &Connection, project_id: i64, suggestion_id: i64) -> Result<(), String> {
    let (kind, payload): (String, String) = tx
        .query_row(
            "SELECT kind, payload FROM suggestions WHERE id = ?1 AND project_id = ?2 AND status = 'pending'",
            params![suggestion_id, project_id],
            |r| Ok((r.get(0)?, r.get::<_, Option<String>>(1)?.unwrap_or_default())),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or("Suggestion not found or already handled.")?;
    let p: Value = serde_json::from_str(&payload).unwrap_or(Value::Null);

    match kind.as_str() {
        "decision" => {
            tx.execute(
                "INSERT INTO decisions (project_id, topic, choice, rationale, source_ref, status) \
                 VALUES (?1, ?2, ?3, ?4, 'chat', 'active')",
                params![project_id, field(&p, "topic"), field(&p, "choice"), field(&p, "rationale")],
            )
            .map_err(|e| e.to_string())?;
        }
        "feature" => {
            // features.source is CHECK(text|audio) — a chat-proposed feature is 'text'.
            tx.execute(
                "INSERT INTO features (project_id, title, detail, source, status) \
                 VALUES (?1, ?2, ?3, 'text', 'inbox')",
                params![project_id, field(&p, "title"), field(&p, "detail")],
            )
            .map_err(|e| e.to_string())?;
        }
        "stack" => {
            // Reuse the canonical path so an approved stack suggestion behaves
            // exactly like a direct selection: upsert + alternatives + a
            // (superseding) decision, tagged source_ref='chat'.
            crate::stack::apply_one(tx, project_id, &field(&p, "pane"), &field(&p, "choice"), "chat")?;
        }
        "answer" => {
            // Record the question + a LINKED answer, so ProjectContext (which
            // JOINs answers on question_id) actually surfaces it on later turns.
            tx.execute(
                "INSERT INTO questions (project_id, text, status) VALUES (?1, ?2, 'answered')",
                params![project_id, field(&p, "question")],
            )
            .map_err(|e| e.to_string())?;
            let qid = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO answers (question_id, project_id, body, source) VALUES (?1, ?2, ?3, 'chat')",
                params![qid, project_id, field(&p, "answer")],
            )
            .map_err(|e| e.to_string())?;
        }
        _ => return Err("Unknown suggestion kind.".into()),
    }
    tx.execute(
        "UPDATE suggestions SET status = 'approved' WHERE id = ?1 AND project_id = ?2",
        params![suggestion_id, project_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Approve a pending suggestion (its own transaction).
pub fn approve(conn: &mut Connection, project_id: i64, suggestion_id: i64) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    approve_in_tx(&tx, project_id, suggestion_id)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Dismiss a pending suggestion without writing anything to the record.
pub fn dismiss(conn: &Connection, project_id: i64, suggestion_id: i64) -> Result<(), String> {
    let n = conn
        .execute(
            "UPDATE suggestions SET status = 'dismissed' WHERE id = ?1 AND project_id = ?2 AND status = 'pending'",
            params![suggestion_id, project_id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("Suggestion not found or already handled.".into());
    }
    Ok(())
}

/// Approve every pending suggestion for a project in ONE transaction — all
/// succeed or none do (a mid-batch failure rolls the whole batch back).
pub fn approve_all(conn: &mut Connection, project_id: i64) -> Result<usize, String> {
    let ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT id FROM suggestions WHERE project_id = ?1 AND status = 'pending' ORDER BY id")
            .map_err(|e| e.to_string())?;
        stmt.query_map([project_id], |r| r.get(0))
            .and_then(Iterator::collect)
            .map_err(|e| e.to_string())?
    };
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for &id in &ids {
        approve_in_tx(&tx, project_id, id)?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(ids.len())
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
    // One fully-parameterized query: `?2 IS NULL` no-ops the status filter when
    // no status is given. Avoids building SQL with format!() (a fragile pattern
    // even when only constants are interpolated).
    let mut stmt = conn
        .prepare(
            "SELECT id, kind, payload, status, created_at FROM suggestions \
             WHERE project_id = ?1 AND (?2 IS NULL OR status = ?2) ORDER BY id DESC",
        )
        .map_err(|e| e.to_string())?;
    stmt.query_map(params![project_id, status], row)
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
            ParsedSuggestion { kind: "stack".into(), payload: r#"{"pane":"nope","choice":"X"}"#.into() }, // invalid pane
            ParsedSuggestion { kind: "decision".into(), payload: "{}".into() }, // empty object
            ParsedSuggestion { kind: "decision".into(), payload: oversized }, // too big
            ParsedSuggestion { kind: "feature".into(), payload: r#"{"title":"Good one"}"#.into() }, // valid
        ];
        let added = save(&mut conn, pid, &items).unwrap();
        assert_eq!(added, 1, "only the complete, bounded payload persists");
        let pending = list(&conn, pid, Some("pending")).unwrap();
        assert_eq!(pending[0].payload["title"], "Good one");
    }

    #[test]
    fn rejects_a_single_field_over_its_cap_even_under_max_payload() {
        let mut conn = db();
        let pid = project(&conn);
        // 9KB rationale: under the coarse MAX_PAYLOAD byte gate, but far over the
        // 1500-char per-field cap — must be rejected by the field check.
        let big = "z".repeat(9_000);
        let payload = format!(r#"{{"topic":"DB","choice":"SQLite","rationale":"{big}"}}"#);
        assert!(payload.len() < MAX_PAYLOAD, "stays under the coarse payload gate");
        let added = save(
            &mut conn,
            pid,
            &[ParsedSuggestion { kind: "decision".into(), payload }],
        )
        .unwrap();
        assert_eq!(added, 0, "oversized single field is rejected by the per-field cap");
    }

    #[test]
    fn list_filters_by_status_and_returns_all_when_none() {
        let mut conn = db();
        let pid = project(&conn);
        save(
            &mut conn,
            pid,
            &[
                ParsedSuggestion { kind: "decision".into(), payload: r#"{"topic":"DB","choice":"SQLite"}"#.into() },
                ParsedSuggestion { kind: "feature".into(), payload: r#"{"title":"Export"}"#.into() },
            ],
        )
        .unwrap();
        let feat_id = list(&conn, pid, Some("pending")).unwrap().iter().find(|s| s.kind == "feature").unwrap().id;
        dismiss(&conn, pid, feat_id).unwrap();

        // None returns every status; the filtered forms partition them.
        assert_eq!(list(&conn, pid, None).unwrap().len(), 2, "None returns all rows");
        assert_eq!(list(&conn, pid, Some("pending")).unwrap().len(), 1);
        assert_eq!(list(&conn, pid, Some("dismissed")).unwrap().len(), 1);
    }

    fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn approve_writes_the_right_table_dismiss_writes_nothing() {
        let mut conn = db();
        let pid = project(&conn);
        save(
            &mut conn,
            pid,
            &[
                ParsedSuggestion { kind: "decision".into(), payload: r#"{"topic":"DB","choice":"SQLite","rationale":"local"}"#.into() },
                ParsedSuggestion { kind: "feature".into(), payload: r#"{"title":"Export CSV","detail":"x"}"#.into() },
                // choice must be a real catalog option for the pane.
                ParsedSuggestion { kind: "stack".into(), payload: r#"{"pane":"frontend","choice":"React + Vite"}"#.into() },
            ],
        )
        .unwrap();
        let ids: Vec<i64> = list(&conn, pid, Some("pending")).unwrap().iter().map(|s| s.id).collect();

        // Approve the decision -> a decisions row, nothing else.
        let decision_id = list(&conn, pid, Some("pending")).unwrap().iter().find(|s| s.kind == "decision").unwrap().id;
        approve(&mut conn, pid, decision_id).unwrap();
        assert_eq!(count(&conn, "SELECT count(*) FROM decisions"), 1);
        assert_eq!(count(&conn, "SELECT count(*) FROM features"), 0);
        assert_eq!(count(&conn, "SELECT count(*) FROM stack_selections"), 0);

        // Dismiss one (a feature) -> writes nothing; it's gone from pending.
        let feat_id = list(&conn, pid, Some("pending")).unwrap().iter().find(|s| s.kind == "feature").unwrap().id;
        dismiss(&conn, pid, feat_id).unwrap();
        assert_eq!(count(&conn, "SELECT count(*) FROM features"), 0, "dismiss writes nothing");

        // Approve-all clears the rest of the queue (the stack one).
        let n = approve_all(&mut conn, pid).unwrap();
        assert_eq!(n, 1);
        assert_eq!(count(&conn, "SELECT count(*) FROM stack_selections"), 1);
        // A stack approval behaves like a direct selection: it records a
        // decision (source_ref='chat') and populates alternatives.
        assert_eq!(
            count(&conn, "SELECT count(*) FROM decisions WHERE topic='Stack: frontend' AND status='active' AND source_ref='chat'"),
            1
        );
        let alts: String = conn
            .query_row("SELECT alternatives FROM stack_selections WHERE pane='frontend'", [], |r| r.get(0))
            .unwrap();
        assert!(!alts.is_empty(), "alternatives populated from the catalog");
        assert!(list(&conn, pid, Some("pending")).unwrap().is_empty(), "queue cleared");

        // re-approving a handled suggestion errors.
        assert!(approve(&mut conn, pid, ids[0]).is_err());
    }

    #[test]
    fn approving_an_answer_links_it_so_context_surfaces_it() {
        let mut conn = db();
        let pid = project(&conn);
        save(
            &mut conn,
            pid,
            &[ParsedSuggestion { kind: "answer".into(), payload: r#"{"question":"Who is it for?","answer":"Solo builders."}"#.into() }],
        )
        .unwrap();
        let sid = list(&conn, pid, Some("pending")).unwrap()[0].id;
        approve(&mut conn, pid, sid).unwrap();
        let ctx = crate::context::ProjectContext::assemble(&conn, pid).unwrap();
        assert!(
            ctx.answers.iter().any(|a| a.question == "Who is it for?" && a.answer == "Solo builders."),
            "approved answer is question-linked and surfaced in the model context"
        );
    }

    #[test]
    fn approve_all_is_atomic_on_failure() {
        let mut conn = db();
        let pid = project(&conn);
        // A valid feature + a stack suggestion with a choice NOT in the catalog
        // (passes save's pane-enum check, fails apply_one's catalog guard).
        save(
            &mut conn,
            pid,
            &[
                ParsedSuggestion { kind: "feature".into(), payload: r#"{"title":"OK feature"}"#.into() },
                ParsedSuggestion { kind: "stack".into(), payload: r#"{"pane":"frontend","choice":"NotInCatalog"}"#.into() },
            ],
        )
        .unwrap();
        assert!(approve_all(&mut conn, pid).is_err(), "the bad stack choice fails the batch");
        // Atomic: the valid feature was NOT committed, nothing approved.
        assert_eq!(count(&conn, "SELECT count(*) FROM features"), 0, "rolled back");
        assert_eq!(list(&conn, pid, Some("pending")).unwrap().len(), 2, "both still pending");
    }
}
