//! Stack panes — the five build choices (frontend, backend, database,
//! deployment, pipes). A static catalog (catalog.json) supplies a recommendation
//! + alternatives + rationale per pane; pre-made stacks (premade.json) fill all
//! five at once. Every selection is also recorded as a decision, superseding the
//! prior active stack decision for that pane so history is kept.

use std::collections::HashMap;
use std::sync::OnceLock;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

pub mod commands;

pub const PANES: [&str; 5] = ["frontend", "backend", "database", "deployment", "pipes"];

const CATALOG_JSON: &str = include_str!("catalog.json");
const PREMADE_JSON: &str = include_str!("premade.json");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatalogOption {
    pub choice: String,
    pub rationale: String,
}

pub fn catalog() -> &'static HashMap<String, Vec<CatalogOption>> {
    static C: OnceLock<HashMap<String, Vec<CatalogOption>>> = OnceLock::new();
    C.get_or_init(|| serde_json::from_str(CATALOG_JSON).expect("catalog.json is valid"))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PremadeStack {
    pub name: String,
    pub summary: String,
    pub panes: HashMap<String, String>,
}

pub fn premade() -> &'static [PremadeStack] {
    static P: OnceLock<Vec<PremadeStack>> = OnceLock::new();
    P.get_or_init(|| serde_json::from_str(PREMADE_JSON).expect("premade.json is valid"))
}

fn rationale_for(pane: &str, choice: &str) -> String {
    catalog()
        .get(pane)
        .and_then(|opts| opts.iter().find(|o| o.choice == choice))
        .map(|o| o.rationale.clone())
        .unwrap_or_default()
}

fn alternatives_for(pane: &str, choice: &str) -> String {
    catalog()
        .get(pane)
        .map(|opts| {
            opts.iter()
                .filter(|o| o.choice != choice)
                .map(|o| o.choice.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default()
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Selection {
    pub pane: String,
    pub choice: Option<String>,
    pub alternatives: Option<String>,
    pub rationale: Option<String>,
}

/// All five panes with the current selection (choice None if unset).
pub fn list_selections(conn: &Connection, project_id: i64) -> Result<Vec<Selection>, String> {
    let mut current: HashMap<String, (Option<String>, Option<String>, Option<String>)> = HashMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT pane, choice, alternatives, rationale FROM stack_selections WHERE project_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([project_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            })
            .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
            .map_err(|e| e.to_string())?;
        for (pane, choice, alts, rat) in rows {
            current.insert(pane, (choice, alts, rat));
        }
    }
    Ok(PANES
        .iter()
        .map(|&p| {
            let (choice, alternatives, rationale) = current.remove(p).unwrap_or((None, None, None));
            Selection { pane: p.to_string(), choice, alternatives, rationale }
        })
        .collect())
}

/// Apply one pane selection: upsert the selection row + record a decision
/// (superseding the prior active stack decision for that pane). `source_ref`
/// tags the decision's origin ('stack' for direct selection, 'chat' when an
/// approved suggestion drives it). Caller wraps it in a transaction (a single
/// override, a 5-pane pre-made apply, or a suggestion approval). Validates the
/// pane + that the choice exists in the catalog so a bad value can't slip in.
pub fn apply_one(
    conn: &Connection,
    project_id: i64,
    pane: &str,
    choice: &str,
    source_ref: &str,
) -> Result<(), String> {
    if !PANES.contains(&pane) {
        return Err("Unknown stack pane.".into());
    }
    let rationale = rationale_for(pane, choice);
    if rationale.is_empty() {
        return Err(format!("Stack choice '{choice}' is not in the catalog for '{pane}'."));
    }
    let alternatives = alternatives_for(pane, choice);
    conn.execute(
        "INSERT INTO stack_selections (project_id, pane, choice, alternatives, rationale) VALUES (?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT(project_id, pane) DO UPDATE SET choice = excluded.choice, alternatives = excluded.alternatives, rationale = excluded.rationale",
        params![project_id, pane, choice, alternatives, rationale],
    )
    .map_err(|e| e.to_string())?;

    let topic = format!("Stack: {pane}");
    conn.execute(
        "UPDATE decisions SET status = 'superseded' \
         WHERE project_id = ?1 AND topic = ?2 AND status = 'active' AND source_ref IN ('stack', 'chat')",
        params![project_id, topic],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO decisions (project_id, topic, choice, rationale, alternatives, source_ref, status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active')",
        params![project_id, topic, choice, rationale, alternatives, source_ref],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Override a single pane.
pub fn set_selection(conn: &mut Connection, project_id: i64, pane: &str, choice: &str) -> Result<(), String> {
    if !PANES.contains(&pane) {
        return Err("Unknown stack pane.".into());
    }
    if choice.trim().is_empty() {
        return Err("Choose an option.".into());
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    apply_one(&tx, project_id, pane, choice, "stack")?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Apply a named pre-made stack to all five panes at once.
pub fn apply_premade(conn: &mut Connection, project_id: i64, name: &str) -> Result<(), String> {
    let stack = premade().iter().find(|s| s.name == name).ok_or("Unknown stack.")?;
    let missing: Vec<&str> = PANES.iter().copied().filter(|p| !stack.panes.contains_key(*p)).collect();
    if !missing.is_empty() {
        return Err(format!("Pre-made stack '{name}' is missing panes: {missing:?}"));
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for &pane in PANES.iter() {
        if let Some(choice) = stack.panes.get(pane) {
            apply_one(&tx, project_id, pane, choice, "stack")?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
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
        conn.execute("INSERT INTO projects (name, kind) VALUES ('K','new')", []).unwrap();
        conn.last_insert_rowid()
    }
    fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn configs_parse_and_cover_all_panes() {
        for p in PANES {
            assert!(!catalog().get(p).unwrap().is_empty(), "catalog has options for {p}");
        }
        assert!(premade().len() >= 2);
        let expected: std::collections::HashSet<_> = PANES.iter().copied().collect();
        for s in premade() {
            let keys: std::collections::HashSet<_> = s.panes.keys().map(String::as_str).collect();
            assert_eq!(keys, expected, "premade {} keys must equal PANES", s.name);
        }
    }

    #[test]
    fn list_returns_all_five_panes_even_when_empty() {
        let conn = db();
        let pid = project(&conn);
        let sels = list_selections(&conn, pid).unwrap();
        assert_eq!(sels.len(), 5);
        assert!(sels.iter().all(|s| s.choice.is_none()));
    }

    #[test]
    fn premade_fills_five_override_persists_and_selections_become_decisions() {
        let mut conn = db();
        let pid = project(&conn);

        apply_premade(&mut conn, pid, "Local-first desktop").unwrap();
        let sels = list_selections(&conn, pid).unwrap();
        assert!(sels.iter().all(|s| s.choice.is_some()), "all five filled");
        assert_eq!(count(&conn, "SELECT count(*) FROM stack_selections"), 5);
        // Selections appear as active decisions (one per pane).
        assert_eq!(count(&conn, "SELECT count(*) FROM decisions WHERE status='active' AND source_ref='stack'"), 5);

        // Override one pane: it persists and supersedes the prior stack decision.
        set_selection(&mut conn, pid, "database", "PostgreSQL").unwrap();
        let db_sel = list_selections(&conn, pid).unwrap().into_iter().find(|s| s.pane == "database").unwrap();
        assert_eq!(db_sel.choice.as_deref(), Some("PostgreSQL"));
        assert_eq!(count(&conn, "SELECT count(*) FROM stack_selections"), 5, "still five (upsert, not insert)");
        assert_eq!(
            count(&conn, "SELECT count(*) FROM decisions WHERE topic='Stack: database' AND status='active'"),
            1,
            "exactly one active database decision"
        );
        assert_eq!(
            count(&conn, "SELECT count(*) FROM decisions WHERE topic='Stack: database' AND status='superseded'"),
            1,
            "the prior one is superseded (history kept)"
        );

        assert!(set_selection(&mut conn, pid, "bogus", "x").is_err());
    }
}
