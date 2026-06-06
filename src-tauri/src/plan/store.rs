//! Persist a parsed plan (a new version + phases/tasks/decisions/stack) in one
//! transaction, and read the latest plan back for the UI. On any failure the
//! transaction rolls back — no partial or orphaned rows.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::parse::GeneratedPlan;

fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.extend(ch.to_lowercase());
            prev_dash = false;
        } else if !out.is_empty() && !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "phase".to_string()
    } else {
        trimmed.chars().take(40).collect()
    }
}

fn opt(s: &str) -> Option<&str> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Save a generated plan as a new version. Returns the version number.
pub fn save_generated_plan(
    conn: &mut Connection,
    project_id: i64,
    plan: &GeneratedPlan,
) -> Result<i64, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let version: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM plans WHERE project_id = ?1",
            [project_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO plans (project_id, version, current_state, body_md) VALUES (?1, ?2, ?3, ?4)",
        params![project_id, version, plan.current_state, plan.body_md],
    )
    .map_err(|e| e.to_string())?;

    for (i, ph) in plan.phases.iter().enumerate() {
        let marker = format!("phase-{:02}-{}", i + 1, slug(&ph.title));
        tx.execute(
            "INSERT INTO phases (project_id, plan_version, idx, title, goal, marker) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![project_id, version, i as i64, ph.title, opt(&ph.goal), marker],
        )
        .map_err(|e| e.to_string())?;
        let phase_id = tx.last_insert_rowid();
        for (j, t) in ph.tasks.iter().enumerate() {
            tx.execute(
                "INSERT INTO tasks (phase_id, idx, title, body_md, verification) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![phase_id, j as i64, t.title, opt(&t.body), opt(&t.verification)],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    for d in &plan.decisions {
        tx.execute(
            "INSERT INTO decisions (project_id, topic, choice, rationale, alternatives, consequences, plan_version) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![project_id, d.topic, d.choice, opt(&d.rationale), opt(&d.alternatives), opt(&d.consequences), version],
        )
        .map_err(|e| e.to_string())?;
    }

    for (pane, choice) in [
        ("frontend", &plan.stack.frontend),
        ("backend", &plan.stack.backend),
        ("database", &plan.stack.database),
        ("deployment", &plan.stack.deployment),
        ("pipes", &plan.stack.pipes),
    ] {
        if let Some(c) = choice.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
            tx.execute(
                "INSERT INTO stack_selections (project_id, pane, choice) VALUES (?1, ?2, ?3) \
                 ON CONFLICT(project_id, pane) DO UPDATE SET choice = excluded.choice",
                params![project_id, pane, c],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(version)
}

// ---- Read the latest plan for the UI ----

#[derive(Debug, Serialize)]
pub struct TaskView {
    pub id: i64,
    pub idx: i64,
    pub title: String,
    pub body_md: Option<String>,
    pub verification: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct PhaseView {
    pub id: i64,
    pub idx: i64,
    pub title: String,
    pub goal: Option<String>,
    pub status: String,
    pub tasks: Vec<TaskView>,
}

#[derive(Debug, Serialize)]
pub struct DecisionView {
    pub topic: String,
    pub choice: String,
    pub rationale: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StackView {
    pub pane: String,
    pub choice: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlanView {
    pub version: i64,
    pub current_state: Option<String>,
    pub body_md: Option<String>,
    pub phases: Vec<PhaseView>,
    pub decisions: Vec<DecisionView>,
    pub stack: Vec<StackView>,
}

/// The latest plan for a project, or None if it has no plan yet.
pub fn get_plan(conn: &Connection, project_id: i64) -> Result<Option<PlanView>, String> {
    let head = conn
        .query_row(
            "SELECT version, current_state, body_md FROM plans WHERE project_id = ?1 ORDER BY version DESC LIMIT 1",
            [project_id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let (version, current_state, body_md) = match head {
        Some(v) => v,
        None => return Ok(None),
    };

    let mut stmt = conn
        .prepare(
            "SELECT id, idx, title, goal, status FROM phases \
             WHERE project_id = ?1 AND plan_version = ?2 ORDER BY idx",
        )
        .map_err(|e| e.to_string())?;
    let phase_rows = stmt
        .query_map(params![project_id, version], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
        .map_err(|e| e.to_string())?;

    let mut phases = Vec::new();
    for (id, idx, title, goal, status) in phase_rows {
        let mut tstmt = conn
            .prepare(
                "SELECT id, idx, title, body_md, verification, status FROM tasks \
                 WHERE phase_id = ?1 ORDER BY idx",
            )
            .map_err(|e| e.to_string())?;
        let tasks = tstmt
            .query_map([id], |r| {
                Ok(TaskView {
                    id: r.get(0)?,
                    idx: r.get(1)?,
                    title: r.get(2)?,
                    body_md: r.get(3)?,
                    verification: r.get(4)?,
                    status: r.get(5)?,
                })
            })
            .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
            .map_err(|e| e.to_string())?;
        phases.push(PhaseView {
            id,
            idx,
            title,
            goal,
            status,
            tasks,
        });
    }

    let mut dstmt = conn
        .prepare(
            "SELECT topic, choice, rationale FROM decisions \
             WHERE project_id = ?1 AND status = 'active' ORDER BY created_at, id",
        )
        .map_err(|e| e.to_string())?;
    let decisions = dstmt
        .query_map([project_id], |r| {
            Ok(DecisionView {
                topic: r.get(0)?,
                choice: r.get(1)?,
                rationale: r.get(2)?,
            })
        })
        .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
        .map_err(|e| e.to_string())?;

    let mut sstmt = conn
        .prepare("SELECT pane, choice FROM stack_selections WHERE project_id = ?1 ORDER BY pane")
        .map_err(|e| e.to_string())?;
    let stack = sstmt
        .query_map([project_id], |r| {
            Ok(StackView {
                pane: r.get(0)?,
                choice: r.get(1)?,
            })
        })
        .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
        .map_err(|e| e.to_string())?;

    Ok(Some(PlanView {
        version,
        current_state,
        body_md,
        phases,
        decisions,
        stack,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;
    use crate::plan::parse::{GenDecision, GenPhase, GenStack, GenTask, GeneratedPlan};

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn
    }

    fn project(conn: &Connection) -> i64 {
        conn.execute("INSERT INTO projects (name, kind) VALUES ('Demo', 'new')", [])
            .unwrap();
        conn.last_insert_rowid()
    }

    fn sample_plan() -> GeneratedPlan {
        GeneratedPlan {
            current_state: "A scaffold.".into(),
            body_md: "## Plan".into(),
            confidence: "low".into(),
            notes: "from README".into(),
            phases: vec![GenPhase {
                title: "Set up".into(),
                goal: "Runnable".into(),
                tasks: vec![GenTask {
                    title: "Init".into(),
                    body: "do it".into(),
                    verification: "it runs".into(),
                }],
            }],
            decisions: vec![GenDecision {
                topic: "DB".into(),
                choice: "SQLite".into(),
                rationale: "simple".into(),
                alternatives: "pg".into(),
                consequences: "".into(),
            }],
            stack: GenStack {
                frontend: None,
                backend: Some("Rust".into()),
                database: Some("SQLite".into()),
                deployment: None,
                pipes: None,
            },
        }
    }

    #[test]
    fn saves_and_reads_back_a_plan() {
        let mut conn = db();
        let pid = project(&conn);
        let version = save_generated_plan(&mut conn, pid, &sample_plan()).unwrap();
        assert_eq!(version, 1);

        let view = get_plan(&conn, pid).unwrap().unwrap();
        assert_eq!(view.version, 1);
        assert_eq!(view.current_state.as_deref(), Some("A scaffold."));
        assert_eq!(view.phases.len(), 1);
        assert_eq!(view.phases[0].title, "Set up");
        assert_eq!(view.phases[0].tasks[0].verification.as_deref(), Some("it runs"));
        assert_eq!(view.phases[0].status, "not_started");
        assert_eq!(view.decisions[0].choice, "SQLite");
        // stack: only the two non-null panes
        assert_eq!(view.stack.len(), 2);
        assert!(view.stack.iter().any(|s| s.pane == "backend" && s.choice.as_deref() == Some("Rust")));
    }

    #[test]
    fn versions_increment() {
        let mut conn = db();
        let pid = project(&conn);
        assert_eq!(save_generated_plan(&mut conn, pid, &sample_plan()).unwrap(), 1);
        assert_eq!(save_generated_plan(&mut conn, pid, &sample_plan()).unwrap(), 2);
        assert_eq!(get_plan(&conn, pid).unwrap().unwrap().version, 2);
    }

    #[test]
    fn no_plan_yields_none() {
        let conn = db();
        let pid = project(&conn);
        assert!(get_plan(&conn, pid).unwrap().is_none());
    }
}
