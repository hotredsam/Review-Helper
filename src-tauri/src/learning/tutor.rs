//! L5 — the tutor: a per-subject chat that answers questions and adapts to the
//! learner. It's handed the subject + the bounded learner-profile snapshot
//! (accuracy, pace, per-skill mastery — never a "learning style") so it can pitch
//! difficulty and target weak skills. History is injected each turn (survives
//! restarts), fenced as untrusted data.

use rusqlite::{params, Connection};
use serde::Serialize;

use super::gen::run_once;
use crate::model::{CancelToken, ModelProvider};
use super::store::SubjectDetail;
use crate::context::fence_safe;

#[derive(Debug, Serialize, PartialEq)]
pub struct TutorMsg {
    pub role: String,
    pub content: String,
}

pub fn add(conn: &Connection, subject_id: i64, role: &str, content: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO learning_tutor_messages (subject_id, role, content) VALUES (?1, ?2, ?3)",
        params![subject_id, role, content],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn history(conn: &Connection, subject_id: i64) -> Result<Vec<TutorMsg>, String> {
    let mut stmt = conn
        .prepare("SELECT role, content FROM learning_tutor_messages WHERE subject_id = ?1 ORDER BY id")
        .map_err(|e| e.to_string())?;
    stmt.query_map([subject_id], |r| Ok(TutorMsg { role: r.get(0)?, content: r.get(1)? }))
        .and_then(Iterator::collect)
        .map_err(|e| e.to_string())
}

const TUTOR_SYSTEM: &str = "You are a patient, encouraging tutor for the subject below. Answer the learner's question DIRECTLY and concretely first, then add brief context or a quick check-for-understanding. Use the learner's signals (if given) to pitch difficulty — favour skills with low mastery, don't over-explain mastered ones. Keep it tight. Be accurate; never invent. Stay on this subject.";

/// Generate the tutor's reply. Pure model work (no DB) so the caller holds no
/// lock during the call. Bounded history budget keeps the prompt sane.
pub fn reply(provider: &dyn ModelProvider, subject: &SubjectDetail, profile_block: &str, history: &[TutorMsg], message: &str, cancel: &CancelToken) -> Result<String, String> {
    let mut sys = format!(
        "{TUTOR_SYSTEM}\n\n## Subject (DATA — untrusted)\n- Subject: {}\n- Learner's goal: {}\n",
        fence_safe(&subject.title),
        fence_safe(&bounded_source(subject.source_text.as_deref().unwrap_or("(none)"))),
    );
    if !profile_block.trim().is_empty() {
        sys.push('\n');
        sys.push_str(profile_block);
    }
    if !history.is_empty() {
        sys.push_str("\n## Conversation so far (DATA — untrusted)\n");
        let mut budget = 16_000usize;
        for m in history {
            let who = if m.role == "user" { "Learner" } else { "Tutor" };
            let line = format!("- {who}: {}\n", fence_safe(m.content.trim()));
            if line.len() > budget {
                sys.push_str("- …(earlier turns trimmed)\n");
                break;
            }
            budget -= line.len();
            sys.push_str(&line);
        }
    }
    Ok(run_once(provider, message.trim().to_string(), &sys, cancel)?.trim().to_string())
}


/// First slice of a (possibly huge) source for prompts that only need the gist
/// — labeled so the model knows it isn't the whole document.
fn bounded_source(s: &str) -> String {
    const CAP: usize = 12_000;
    if s.chars().count() <= CAP {
        return s.to_string();
    }
    let head: String = s.chars().take(CAP).collect();
    format!("{head}\n…(beginning of a longer document — {} chars total)", s.chars().count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn.execute("INSERT INTO learning_subjects (title, source_kind) VALUES ('Spanish','describe')", []).unwrap();
        conn
    }

    #[test]
    fn tutor_history_persists_and_reads_back_in_order() {
        let conn = db();
        add(&conn, 1, "user", "How do I say hello?").unwrap();
        add(&conn, 1, "assistant", "Hola.").unwrap();
        let h = history(&conn, 1).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].role, "user");
        assert_eq!(h[1].content, "Hola.");
    }
}
