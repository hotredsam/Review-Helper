//! Persist a parsed plan (a new version + phases/tasks/decisions/stack) in one
//! transaction, and read the latest plan back for the UI. On any failure the
//! transaction rolls back — no partial or orphaned rows.

use std::collections::HashSet;

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

/// Save a generated plan as a new version (own transaction). Returns the version.
/// Test/seed convenience wrapper around `save_into_tx`; production paths use the
/// atomic commit_fresh/commit_merge helpers.
#[cfg(test)]
pub fn save_generated_plan(
    conn: &mut Connection,
    project_id: i64,
    plan: &GeneratedPlan,
) -> Result<i64, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let version = save_into_tx(&tx, project_id, plan)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(version)
}

/// Insert a plan version + its phases/tasks/decisions/stack within a caller's
/// transaction (so an update can save + carry status atomically).
pub(crate) fn save_into_tx(tx: &Connection, project_id: i64, plan: &GeneratedPlan) -> Result<i64, String> {
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
        // Marker is the title slug ONLY (no index) so a phase's carry-status
        // identity survives reordering/insertion. Position carry uses `idx`.
        let marker = format!("phase-{}", slug(&ph.title));
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
        let topic = d.topic.trim();
        if topic.is_empty() {
            continue;
        }
        // Upsert by topic: a plan update refreshes the active record instead of
        // re-inserting every decision as a new row on every rebuild. A topic the
        // user superseded stays superseded — the plan never resurrects it.
        let active: Option<i64> = tx
            .query_row(
                "SELECT id FROM decisions WHERE project_id = ?1 AND status = 'active' AND lower(topic) = lower(?2)",
                params![project_id, topic],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(id) = active {
            tx.execute(
                "UPDATE decisions SET choice = ?1, rationale = ?2, alternatives = ?3, consequences = ?4, plan_version = ?5 \
                 WHERE id = ?6",
                params![d.choice, opt(&d.rationale), opt(&d.alternatives), opt(&d.consequences), version, id],
            )
            .map_err(|e| e.to_string())?;
            continue;
        }
        let superseded: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM decisions WHERE project_id = ?1 AND lower(topic) = lower(?2))",
                params![project_id, topic],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if superseded {
            continue;
        }
        tx.execute(
            "INSERT INTO decisions (project_id, topic, choice, rationale, alternatives, consequences, plan_version) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![project_id, topic, d.choice, opt(&d.rationale), opt(&d.alternatives), opt(&d.consequences), version],
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
    Ok(version)
}

/// Carry completion forward (own transaction). Returns any loss warnings.
/// Test convenience wrapper; production uses carry_into_tx inside commit_merge.
#[cfg(test)]
pub fn carry_status(conn: &mut Connection, project_id: i64, new_version: i64) -> Result<Vec<String>, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let warnings = carry_into_tx(&tx, project_id, new_version)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(warnings)
}

#[derive(Clone)]
struct PriorPhase {
    id: i64,
    idx: i64,
    title: String,
    status: String,
    issue: Option<i64>,
}

/// Carry per-phase (and per-task) completion from the prior plan version into
/// `new_version`, within a caller's transaction. Matches each new phase to a
/// prior phase by stable `marker` first, then falls back to position (`idx`) so
/// a renamed phase doesn't silently reset — the core guard against the
/// "restart at Phase 1" bug. Returns warnings for any completed prior phase
/// that couldn't be matched (so the merge can surface a heads-up).
pub(crate) fn carry_into_tx(tx: &Connection, project_id: i64, new_version: i64) -> Result<Vec<String>, String> {
    let prior = new_version - 1;
    let mut warnings = Vec::new();
    if prior < 1 {
        return Ok(warnings);
    }

    let mut prior_phases: Vec<PriorPhase> = {
        let mut stmt = tx
            .prepare("SELECT id, idx, title, status, github_issue_number FROM phases WHERE project_id = ?1 AND plan_version = ?2 ORDER BY idx")
            .map_err(|e| e.to_string())?;
        stmt.query_map(params![project_id, prior], |r| {
            Ok(PriorPhase {
                id: r.get(0)?,
                idx: r.get(1)?,
                title: r.get(2)?,
                status: r.get(3)?,
                issue: r.get(4)?,
            })
        })
        .and_then(Iterator::collect)
        .map_err(|e| e.to_string())?
    };
    let prior_markers: Vec<(i64, String)> = {
        let mut stmt = tx
            .prepare("SELECT id, marker FROM phases WHERE project_id = ?1 AND plan_version = ?2")
            .map_err(|e| e.to_string())?;
        stmt.query_map(params![project_id, prior], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .and_then(Iterator::collect)
            .map_err(|e| e.to_string())?
    };
    let new_phases: Vec<(i64, i64, String)> = {
        let mut stmt = tx
            .prepare("SELECT id, idx, marker FROM phases WHERE project_id = ?1 AND plan_version = ?2 ORDER BY idx")
            .map_err(|e| e.to_string())?;
        stmt.query_map(params![project_id, new_version], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?))
        })
        .and_then(Iterator::collect)
        .map_err(|e| e.to_string())?
    };

    let mut consumed: HashSet<i64> = HashSet::new();
    for (new_id, new_idx, marker) in &new_phases {
        // Exact marker match (by id, unconsumed), else fall back to same position.
        let prior_id_by_marker = prior_markers
            .iter()
            .find(|(pid, m)| m == marker && !consumed.contains(pid))
            .map(|(pid, _)| *pid);
        let matched = prior_id_by_marker
            .and_then(|pid| prior_phases.iter().find(|p| p.id == pid))
            .or_else(|| prior_phases.iter().find(|p| p.idx == *new_idx && !consumed.contains(&p.id)))
            .cloned();

        if let Some(pp) = matched {
            consumed.insert(pp.id);
            tx.execute(
                "UPDATE phases SET status = ?1, github_issue_number = ?2 WHERE id = ?3",
                params![pp.status, pp.issue, new_id],
            )
            .map_err(|e| e.to_string())?;
            carry_tasks(tx, pp.id, *new_id)?;
        }
    }

    // Loss-detection: any completed/in-progress prior phase that wasn't matched.
    prior_phases.retain(|p| !consumed.contains(&p.id));
    for p in &prior_phases {
        if p.status != "not_started" {
            warnings.push(format!(
                "Phase “{}” was {} but the updated plan dropped it — its progress was not carried.",
                p.title, p.status
            ));
        }
    }
    Ok(warnings)
}

/// Carry task completion within a matched phase: exact title first, then position.
fn carry_tasks(tx: &Connection, prior_phase_id: i64, new_phase_id: i64) -> Result<(), String> {
    let prior: Vec<(i64, String, String)> = {
        let mut stmt = tx
            .prepare("SELECT idx, title, status FROM tasks WHERE phase_id = ?1 ORDER BY idx")
            .map_err(|e| e.to_string())?;
        stmt.query_map([prior_phase_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
        })
        .and_then(Iterator::collect)
        .map_err(|e| e.to_string())?
    };
    let new: Vec<(i64, i64, String)> = {
        let mut stmt = tx
            .prepare("SELECT id, idx, title FROM tasks WHERE phase_id = ?1 ORDER BY idx")
            .map_err(|e| e.to_string())?;
        stmt.query_map([new_phase_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?))
        })
        .and_then(Iterator::collect)
        .map_err(|e| e.to_string())?
    };
    let mut used: HashSet<i64> = HashSet::new(); // prior idx values consumed
    for (tid, tidx, title) in &new {
        let m = prior
            .iter()
            .find(|(pidx, pt, _)| pt == title && !used.contains(pidx))
            .or_else(|| prior.iter().find(|(pidx, _, _)| pidx == tidx && !used.contains(pidx)));
        if let Some((pidx, _, status)) = m {
            used.insert(*pidx);
            if status != "not_started" {
                tx.execute("UPDATE tasks SET status = ?1 WHERE id = ?2", params![status, tid])
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

// ---- Read the latest plan for the UI ----

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

    #[test]
    fn carry_status_preserves_completion_across_a_merge() {
        let mut conn = db();
        let pid = project(&conn);
        let v1 = save_generated_plan(&mut conn, pid, &sample_plan()).unwrap();
        // The builder completed the v1 phase + its task.
        conn.execute("UPDATE phases SET status='done' WHERE project_id=?1 AND plan_version=?2", params![pid, v1]).unwrap();
        conn.execute(
            "UPDATE tasks SET status='done' WHERE phase_id IN (SELECT id FROM phases WHERE project_id=?1 AND plan_version=?2)",
            params![pid, v1],
        )
        .unwrap();

        // An update produces v2 with the same phase title (same marker).
        let v2 = save_generated_plan(&mut conn, pid, &sample_plan()).unwrap();
        assert_eq!(get_plan(&conn, pid).unwrap().unwrap().phases[0].status, "not_started", "fresh v2 starts not_started");

        let warnings = carry_status(&mut conn, pid, v2).unwrap();
        assert!(warnings.is_empty(), "nothing lost on an identical merge");
        let view = get_plan(&conn, pid).unwrap().unwrap();
        assert_eq!(view.version, v2);
        assert_eq!(view.phases[0].status, "done", "phase completion carried over by marker");
        assert_eq!(view.phases[0].tasks[0].status, "done", "task completion carried over by title");
    }

    // A multi-phase plan helper for reorder/rename tests.
    fn plan_with(phases: &[(&str, &[&str])]) -> GeneratedPlan {
        GeneratedPlan {
            current_state: "s".into(),
            body_md: "b".into(),
            confidence: "low".into(),
            notes: "".into(),
            phases: phases
                .iter()
                .map(|(title, tasks)| GenPhase {
                    title: (*title).into(),
                    goal: "g".into(),
                    tasks: tasks
                        .iter()
                        .map(|t| GenTask { title: (*t).into(), body: "".into(), verification: "v".into() })
                        .collect(),
                })
                .collect(),
            decisions: vec![],
            stack: GenStack { frontend: None, backend: None, database: None, deployment: None, pipes: None },
        }
    }

    fn phase_status(conn: &Connection, pid: i64, title: &str) -> String {
        let view = get_plan(conn, pid).unwrap().unwrap();
        view.phases.iter().find(|p| p.title == title).unwrap().status.clone()
    }

    #[test]
    fn carry_status_survives_phase_reorder_and_rename() {
        let mut conn = db();
        let pid = project(&conn);
        // v1: [Setup, Build]; complete "Build".
        let v1 = save_generated_plan(&mut conn, pid, &plan_with(&[("Setup", &["a"]), ("Build", &["x"])])).unwrap();
        conn.execute(
            "UPDATE phases SET status='done' WHERE project_id=?1 AND plan_version=?2 AND title='Build'",
            params![pid, v1],
        )
        .unwrap();

        // v2 REORDERS to [Build, Setup] (markers are index-free) — Build stays done.
        let v2 = save_generated_plan(&mut conn, pid, &plan_with(&[("Build", &["x"]), ("Setup", &["a"])])).unwrap();
        let w2 = carry_status(&mut conn, pid, v2).unwrap();
        assert!(w2.is_empty(), "reorder loses nothing: {w2:?}");
        assert_eq!(phase_status(&conn, pid, "Build"), "done", "reordered done phase preserved");

        // v3 RENAMES "Build" -> "Build it" at the same position — idx fallback carries it.
        let v3 = save_generated_plan(&mut conn, pid, &plan_with(&[("Build it", &["x"]), ("Setup", &["a"])])).unwrap();
        let w3 = carry_status(&mut conn, pid, v3).unwrap();
        assert!(w3.is_empty(), "rename at same position loses nothing: {w3:?}");
        assert_eq!(phase_status(&conn, pid, "Build it"), "done", "renamed done phase carried by position");
    }

    #[test]
    fn carry_status_warns_when_a_completed_phase_is_dropped() {
        let mut conn = db();
        let pid = project(&conn);
        let v1 = save_generated_plan(&mut conn, pid, &plan_with(&[("Keep", &["k"]), ("Done thing", &["d"])])).unwrap();
        conn.execute(
            "UPDATE phases SET status='done' WHERE project_id=?1 AND plan_version=?2 AND title='Done thing'",
            params![pid, v1],
        )
        .unwrap();
        // v2 drops "Done thing" entirely (only one phase, so no position match).
        let v2 = save_generated_plan(&mut conn, pid, &plan_with(&[("Keep", &["k"])])).unwrap();
        let warnings = carry_status(&mut conn, pid, v2).unwrap();
        assert_eq!(warnings.len(), 1, "the dropped completed phase is surfaced");
        assert!(warnings[0].contains("Done thing"));
    }

    #[test]
    fn plan_updates_upsert_decisions_instead_of_duplicating() {
        let mut conn = db();
        conn.execute("INSERT INTO projects (name, kind) VALUES ('P','new')", []).unwrap();
        let pid = conn.last_insert_rowid();
        let mut plan = GeneratedPlan {
            current_state: "x".into(),
            body_md: "b".into(),
            confidence: "low".into(),
            notes: "".into(),
            phases: vec![],
            decisions: vec![GenDecision { topic: "Database".into(), choice: "SQLite".into(), rationale: "simple".into(), alternatives: "".into(), consequences: "".into() }],
            stack: GenStack { frontend: None, backend: None, database: None, deployment: None, pipes: None },
        };
        save_generated_plan(&mut conn, pid, &plan).unwrap();
        plan.decisions[0].choice = "Postgres".into();
        save_generated_plan(&mut conn, pid, &plan).unwrap();

        let (count, choice): (i64, String) = conn
            .query_row("SELECT COUNT(*), MAX(choice) FROM decisions WHERE project_id = ?1", [pid], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!(count, 1, "rebuild must not duplicate the decision");
        assert_eq!(choice, "Postgres", "the active row is refreshed in place");

        // A superseded topic stays superseded — the plan never resurrects it.
        conn.execute("UPDATE decisions SET status = 'superseded' WHERE project_id = ?1", [pid]).unwrap();
        save_generated_plan(&mut conn, pid, &plan).unwrap();
        let (count, status): (i64, String) = conn
            .query_row("SELECT COUNT(*), MAX(status) FROM decisions WHERE project_id = ?1", [pid], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!((count, status.as_str()), (1, "superseded"));
    }
}
