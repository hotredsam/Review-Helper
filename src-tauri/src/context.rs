//! ProjectContext — the bundle injected into every model call so the model is
//! grounded in current state: the latest plan, active decisions, answered
//! questions, and the chosen stack. Assembled fresh before each call.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ContextDecision {
    pub topic: String,
    pub choice: String,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ContextAnswer {
    pub question: String,
    pub answer: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ContextStack {
    pub pane: String,
    pub choice: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProjectContext {
    pub project_name: String,
    pub current_state: Option<String>,
    pub plan_body: Option<String>,
    pub decisions: Vec<ContextDecision>,
    pub answers: Vec<ContextAnswer>,
    pub stack: Vec<ContextStack>,
}

impl ProjectContext {
    /// Assemble the bundle from the database for a project.
    pub fn assemble(conn: &Connection, project_id: i64) -> Result<ProjectContext, String> {
        let project_name: String = conn
            .query_row("SELECT name FROM projects WHERE id = ?1", [project_id], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;

        let (current_state, plan_body) = conn
            .query_row(
                "SELECT current_state, body_md FROM plans WHERE project_id = ?1 ORDER BY version DESC LIMIT 1",
                [project_id],
                |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or((None, None));

        let mut stmt = conn
            .prepare(
                "SELECT topic, choice, rationale FROM decisions \
                 WHERE project_id = ?1 AND status = 'active' ORDER BY created_at, id",
            )
            .map_err(|e| e.to_string())?;
        let decisions = stmt
            .query_map([project_id], |r| {
                Ok(ContextDecision {
                    topic: r.get(0)?,
                    choice: r.get(1)?,
                    rationale: r.get(2)?,
                })
            })
            .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
            .map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT q.text, a.body FROM questions q JOIN answers a ON a.question_id = q.id \
                 WHERE q.project_id = ?1 AND q.status = 'answered' ORDER BY q.created_at, a.created_at",
            )
            .map_err(|e| e.to_string())?;
        let answers = stmt
            .query_map([project_id], |r| {
                Ok(ContextAnswer {
                    question: r.get(0)?,
                    answer: r.get(1)?,
                })
            })
            .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
            .map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT pane, choice FROM stack_selections \
                 WHERE project_id = ?1 AND choice IS NOT NULL ORDER BY pane",
            )
            .map_err(|e| e.to_string())?;
        let stack = stmt
            .query_map([project_id], |r| {
                Ok(ContextStack {
                    pane: r.get(0)?,
                    choice: r.get(1)?,
                })
            })
            .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
            .map_err(|e| e.to_string())?;

        Ok(ProjectContext {
            project_name,
            current_state,
            plan_body,
            decisions,
            answers,
            stack,
        })
    }

    /// Render the bundle as a text block for injection into a model system prompt.
    pub fn to_prompt(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("## Project context: {}\n\n", self.project_name));

        s.push_str("### Current state\n");
        s.push_str(
            self.current_state
                .as_deref()
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .unwrap_or("Not assessed yet."),
        );

        s.push_str("\n\n### Decisions\n");
        if self.decisions.is_empty() {
            s.push_str("None recorded.\n");
        } else {
            for d in &self.decisions {
                s.push_str(&format!("- {}: {}", d.topic, d.choice));
                if let Some(r) = d.rationale.as_deref().filter(|r| !r.is_empty()) {
                    s.push_str(&format!(" — {r}"));
                }
                s.push('\n');
            }
        }

        s.push_str("\n### Answered questions\n");
        if self.answers.is_empty() {
            s.push_str("None yet.\n");
        } else {
            for a in &self.answers {
                s.push_str(&format!("- Q: {}\n  A: {}\n", a.question, a.answer));
            }
        }

        s.push_str("\n### Stack\n");
        if self.stack.is_empty() {
            s.push_str("Not chosen yet.\n");
        } else {
            for st in &self.stack {
                s.push_str(&format!("- {}: {}\n", st.pane, st.choice));
            }
        }

        if let Some(body) = self.plan_body.as_deref().filter(|b| !b.trim().is_empty()) {
            s.push_str("\n### Current plan\n");
            s.push_str(body.trim());
            s.push('\n');
        }

        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;
    use rusqlite::params;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn
    }

    fn new_project(conn: &Connection) -> i64 {
        conn.execute("INSERT INTO projects (name, kind) VALUES ('Demo', 'new')", [])
            .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn empty_project_yields_an_empty_bundle() {
        let conn = db();
        let pid = new_project(&conn);
        let ctx = ProjectContext::assemble(&conn, pid).unwrap();
        assert_eq!(ctx.project_name, "Demo");
        assert!(ctx.current_state.is_none());
        assert!(ctx.decisions.is_empty() && ctx.answers.is_empty() && ctx.stack.is_empty());

        let prompt = ctx.to_prompt();
        assert!(prompt.contains("Not assessed yet."));
        assert!(prompt.contains("None recorded."));
        assert!(prompt.contains("Not chosen yet."));
    }

    #[test]
    fn assembles_from_seeded_rows_excluding_inactive() {
        let conn = db();
        let pid = new_project(&conn);
        conn.execute(
            "INSERT INTO plans (project_id, version, current_state, body_md) VALUES (?1, 1, 'A todo app, early.', '# Plan\nphases')",
            [pid],
        ).unwrap();
        conn.execute(
            "INSERT INTO decisions (project_id, topic, choice, rationale, status) VALUES (?1, 'DB', 'SQLite', 'simple', 'active')",
            [pid],
        ).unwrap();
        conn.execute(
            "INSERT INTO decisions (project_id, topic, choice, status) VALUES (?1, 'old', 'x', 'superseded')",
            [pid],
        ).unwrap();
        conn.execute(
            "INSERT INTO questions (project_id, text, status) VALUES (?1, 'Who is it for?', 'answered')",
            [pid],
        ).unwrap();
        let qid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO answers (question_id, project_id, body, source) VALUES (?1, ?2, 'Solo devs', 'typed')",
            params![qid, pid],
        ).unwrap();
        conn.execute(
            "INSERT INTO questions (project_id, text, status) VALUES (?1, 'still open', 'open')",
            [pid],
        ).unwrap();
        conn.execute(
            "INSERT INTO stack_selections (project_id, pane, choice) VALUES (?1, 'database', 'SQLite')",
            [pid],
        ).unwrap();

        let ctx = ProjectContext::assemble(&conn, pid).unwrap();
        assert_eq!(ctx.current_state.as_deref(), Some("A todo app, early."));
        assert_eq!(ctx.decisions.len(), 1, "only active decisions");
        assert_eq!(ctx.decisions[0].choice, "SQLite");
        assert_eq!(ctx.answers.len(), 1, "only answered questions");
        assert_eq!(ctx.answers[0].answer, "Solo devs");
        assert_eq!(ctx.stack.len(), 1);

        let prompt = ctx.to_prompt();
        assert!(prompt.contains("A todo app, early."));
        assert!(prompt.contains("DB: SQLite — simple"));
        assert!(prompt.contains("Q: Who is it for?"));
        assert!(prompt.contains("database: SQLite"));
        assert!(prompt.contains("### Current plan"));
    }
}
