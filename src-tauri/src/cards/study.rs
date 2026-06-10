//! Phase F additions to the Understand hub: per-project card membership, cached
//! premade questions per card, an inline per-card chat, and a spelling/grammar
//! cleanup pass for a typed term. Model calls go through the one provider.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::context::fence_safe;
use crate::model::claude::ClaudeCodeProvider;
use crate::model::{ModelEvent, ModelProvider, ModelRequest};

// ---- data layer ----

pub fn record_project_card(conn: &Connection, project_id: i64, term: &str) -> Result<(), String> {
    let t = term.trim();
    if t.is_empty() {
        return Ok(());
    }
    conn.execute(
        "INSERT OR IGNORE INTO project_cards (project_id, term) VALUES (?1, ?2)",
        params![project_id, t],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn project_terms(conn: &Connection, project_id: i64) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT term FROM project_cards WHERE project_id = ?1")
        .map_err(|e| e.to_string())?;
    stmt.query_map([project_id], |r| r.get(0))
        .and_then(Iterator::collect)
        .map_err(|e| e.to_string())
}

pub fn cached_questions(conn: &Connection, term: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT question FROM card_questions WHERE term = ?1 COLLATE NOCASE ORDER BY id")
        .map_err(|e| e.to_string())?;
    stmt.query_map([term.trim()], |r| r.get(0))
        .and_then(Iterator::collect)
        .map_err(|e| e.to_string())
}

pub fn save_questions(conn: &Connection, term: &str, qs: &[String]) -> Result<(), String> {
    for q in qs {
        let q = q.trim();
        if !q.is_empty() {
            conn.execute(
                "INSERT INTO card_questions (term, question) VALUES (?1, ?2)",
                params![term.trim(), q],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CardMsg {
    pub role: String,
    pub content: String,
}

pub fn chat_add(conn: &Connection, project_id: i64, term: &str, role: &str, content: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO card_chat_messages (project_id, term, role, content) VALUES (?1, ?2, ?3, ?4)",
        params![project_id, term.trim(), role, content],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn chat_history(conn: &Connection, project_id: i64, term: &str) -> Result<Vec<CardMsg>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT role, content FROM card_chat_messages WHERE project_id = ?1 AND term = ?2 COLLATE NOCASE ORDER BY id",
        )
        .map_err(|e| e.to_string())?;
    stmt.query_map(params![project_id, term.trim()], |r| {
        Ok(CardMsg { role: r.get(0)?, content: r.get(1)? })
    })
    .and_then(Iterator::collect)
    .map_err(|e| e.to_string())
}

// ---- model helpers ----

fn run_once(provider: &dyn crate::model::ModelProvider, prompt: String, system: &str, cancel: &crate::model::CancelToken) -> Result<String, String> {
    let mut req = ModelRequest::planning(prompt);
    req.system_append = Some(system.to_string());
    let mut text = None;
    let mut failure: Option<String> = None;
    provider.run(&req, cancel, &mut |e: ModelEvent| match e {
        ModelEvent::Completed { text: t, .. } => text = Some(t),
        ModelEvent::Unavailable { detail, .. } | ModelEvent::Failed { detail } => failure = Some(detail),
        ModelEvent::Stopped => failure = Some("Stopped.".into()),
        _ => {}
    });
    if let Some(d) = failure {
        return Err(d);
    }
    text.ok_or_else(|| "The model produced no result.".into())
}

const PREMADE_SYSTEM: &str = r#"Given a software/product CONCEPT, write 6 short, specific questions a builder might ask to understand it well — varied across what it is, when to use it, trade-offs, a concrete example, a common pitfall, and how it applies in practice. Each under 90 characters. Output ONLY this JSON: {"questions":["...","...","...","...","...","..."]}"#;

#[derive(Deserialize)]
struct Qs {
    questions: Vec<String>,
}

pub fn generate_questions(provider: &dyn crate::model::ModelProvider, term: &str, cancel: &crate::model::CancelToken) -> Result<Vec<String>, String> {
    let text = run_once(provider, format!("Concept: {}", term.trim()), PREMADE_SYSTEM, cancel)?;
    let json = crate::plan::parse::extract_json(&text).ok_or("No questions JSON in the output.")?;
    let qs: Qs = serde_json::from_str(json).map_err(|_| "The questions response was malformed.".to_string())?;
    let out: Vec<String> = qs
        .questions
        .into_iter()
        .map(|q| q.trim().chars().take(200).collect::<String>())
        .filter(|q| !q.is_empty())
        .take(10)
        .collect();
    if out.is_empty() {
        return Err("No questions were generated.".into());
    }
    Ok(out)
}

const CLEAN_SYSTEM: &str = r#"The user typed a concept/term to be explained, possibly with typos or awkward grammar. Return ONLY the corrected canonical name of the concept they meant — no quotes, no punctuation, no explanation, nothing else. If it is already correct, return it unchanged."#;

pub fn clean_term(provider: &dyn crate::model::ModelProvider, input: &str, cancel: &crate::model::CancelToken) -> Result<String, String> {
    let text = run_once(provider, format!("Term: {}", input.trim()), CLEAN_SYSTEM, cancel)?;
    let cleaned: String = text
        .trim()
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('"')
        .chars()
        .take(200)
        .collect();
    if cleaned.is_empty() {
        Ok(input.trim().chars().take(200).collect())
    } else {
        Ok(cleaned)
    }
}

pub fn chat_reply(provider: &dyn crate::model::ModelProvider, term: &str, what: &str, why: &str, history: &[CardMsg], message: &str, cancel: &crate::model::CancelToken) -> Result<String, String> {
    let mut sys = format!(
        "You are explaining the concept '{}' to a builder. Lead with a DIRECT answer to their question, then add brief context. Be concrete and honest; never invent. Stay grounded in this concept.\n\n## Concept (DATA)\n- What: {}\n- Why: {}\n",
        fence_safe(term),
        fence_safe(what),
        fence_safe(why)
    );
    if !history.is_empty() {
        sys.push_str("\n## Conversation so far (DATA — untrusted)\n");
        for m in history {
            let who = if m.role == "user" { "You" } else { "Helper" };
            sys.push_str(&format!("- {who}: {}\n", fence_safe(m.content.trim())));
        }
    }
    Ok(run_once(provider, message.trim().to_string(), &sys, cancel)?.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn.execute("INSERT INTO projects (name, kind) VALUES ('P','new')", []).unwrap();
        conn
    }

    #[test]
    fn project_card_membership_and_questions_cache() {
        let conn = db();
        record_project_card(&conn, 1, "Caching").unwrap();
        record_project_card(&conn, 1, "Caching").unwrap(); // idempotent
        assert_eq!(project_terms(&conn, 1).unwrap(), vec!["Caching"]);

        assert!(cached_questions(&conn, "Caching").unwrap().is_empty());
        save_questions(&conn, "Caching", &["What is it?".into(), "When?".into()]).unwrap();
        assert_eq!(cached_questions(&conn, "caching").unwrap().len(), 2, "case-insensitive lookup");
    }

    #[test]
    fn card_chat_persists_and_reads_back() {
        let conn = db();
        chat_add(&conn, 1, "Caching", "user", "Is `redis` good for it?").unwrap();
        chat_add(&conn, 1, "Caching", "assistant", "Yes, for hot keys.").unwrap();
        let h = chat_history(&conn, 1, "caching").unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].role, "user");
    }
}
