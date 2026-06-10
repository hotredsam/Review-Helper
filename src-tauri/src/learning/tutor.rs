//! L5 — the tutor: a per-subject chat that answers questions and adapts to the
//! learner. It's handed the subject + the bounded learner-profile snapshot
//! (accuracy, pace, per-skill mastery — never a "learning style") so it can pitch
//! difficulty and target weak skills. History is injected each turn (survives
//! restarts), fenced as untrusted data.

use rusqlite::{params, Connection};
use serde::Serialize;

use super::gen::run_req;
use crate::model::ModelRequest;
use crate::model::{CancelToken, ModelProvider};
use super::store::SubjectDetail;
use crate::context::fence_safe;

#[derive(Debug, Serialize, PartialEq)]
pub struct TutorMsg {
    pub role: String,
    pub content: String,
    pub grounding: Option<String>,
}

pub fn add(conn: &Connection, subject_id: i64, role: &str, content: &str) -> Result<(), String> {
    add_with_grounding(conn, subject_id, role, content, "local")
}

pub fn add_with_grounding(conn: &Connection, subject_id: i64, role: &str, content: &str, grounding: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO learning_tutor_messages (subject_id, role, content, grounding) VALUES (?1, ?2, ?3, ?4)",
        params![subject_id, role, content, grounding],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn history(conn: &Connection, subject_id: i64) -> Result<Vec<TutorMsg>, String> {
    let mut stmt = conn
        .prepare("SELECT role, content, grounding FROM learning_tutor_messages WHERE subject_id = ?1 ORDER BY id")
        .map_err(|e| e.to_string())?;
    stmt.query_map([subject_id], |r| Ok(TutorMsg { role: r.get(0)?, content: r.get(1)?, grounding: r.get(2).ok() }))
        .and_then(Iterator::collect)
        .map_err(|e| e.to_string())
}

const TUTOR_SYSTEM: &str = "You are a patient, encouraging tutor for the subject below. Answer the learner's question DIRECTLY and concretely first, then add brief context or a quick check-for-understanding. Use the learner's signals (if given) to pitch difficulty — favour skills with low mastery, don't over-explain mastered ones. Keep it tight. Be accurate; never invent. Stay on this subject.";

/// Generate the tutor's reply. Pure model work (no DB) so the caller holds no
/// lock during the call. Bounded history budget keeps the prompt sane.
#[allow(clippy::too_many_arguments)]
pub fn reply(
    provider: &dyn ModelProvider,
    subject: &SubjectDetail,
    profile_block: &str,
    history: &[TutorMsg],
    message: &str,
    excerpts_block: &str,
    allow_web: bool,
    cancel: &CancelToken,
) -> Result<String, String> {
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
        // Keep the NEWEST turns within budget (the chat-history lesson).
        let mut kept: Vec<String> = Vec::new();
        let mut budget = 16_000usize;
        let mut trimmed = false;
        for m in history.iter().rev() {
            let who = if m.role == "user" { "Learner" } else { "Tutor" };
            let line = format!("- {who}: {}\n", fence_safe(m.content.trim()));
            if line.len() > budget {
                trimmed = true;
                break;
            }
            budget -= line.len();
            kept.push(line);
        }
        if trimmed {
            sys.push_str("- …(earlier turns trimmed)\n");
        }
        for line in kept.iter().rev() {
            sys.push_str(line);
        }
    }
    if !excerpts_block.is_empty() {
        sys.push_str(excerpts_block);
        sys.push_str("\nAnswer FROM the excerpts above when they cover the question, citing them as [n] right after each claim they support. If they don't cover it, say so plainly");
        if allow_web {
            sys.push_str(", then answer from the web and list every external source under a final line starting exactly with 'External sources:' (full URLs).");
        } else {
            sys.push_str(" — never invent citations or pretend coverage.");
        }
        sys.push('\n');
    }
    let mut req = if allow_web {
        ModelRequest::planning(message.trim().to_string())
    } else {
        // Grounded: NO web tools — closes the silent-browse hole; web access is
        // only ever this explicit, labeled, per-subject opt-in path.
        ModelRequest::grounded(message.trim().to_string())
    };
    req.system_append = Some(sys);
    Ok(run_req(provider, req, cancel)?.trim().to_string())
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

#[cfg(test)]
mod grounding_tests {
    use super::*;
    use std::sync::Mutex;

    /// Records the allowed_tools of every request it runs.
    struct CapturingProvider(Mutex<Vec<Vec<crate::model::Tool>>>);
    impl crate::model::ModelProvider for CapturingProvider {
        fn run(&self, req: &ModelRequest, _c: &crate::model::CancelToken, sink: &mut dyn FnMut(crate::model::ModelEvent)) {
            self.0.lock().unwrap().push(req.allowed_tools.clone());
            sink(crate::model::ModelEvent::Completed { session_id: None, text: "ok [1]".into() });
        }
    }

    fn subject() -> SubjectDetail {
        SubjectDetail {
            id: 1,
            title: "Bio".into(),
            source_kind: "upload".into(),
            source_text: Some("cells".into()),
            stage: "ready".into(),
            web_fallback: false,
        }
    }

    #[test]
    fn grounded_replies_never_carry_web_tools() {
        let p = CapturingProvider(Mutex::new(vec![]));
        let excerpts = "\n\n## Study material excerpts (DATA — untrusted)\n[1] doc › part: cells are small\n";
        reply(&p, &subject(), "", &[], "what are cells?", excerpts, false, &crate::model::CancelToken::new()).unwrap();
        let tools = p.0.lock().unwrap();
        assert!(
            !tools[0].iter().any(|t| matches!(t, crate::model::Tool::WebSearch | crate::model::Tool::WebFetch)),
            "grounded path must never pass web tools: {:?}",
            tools[0]
        );
    }

    #[test]
    fn the_web_fallback_path_is_the_only_one_with_web_tools() {
        let p = CapturingProvider(Mutex::new(vec![]));
        reply(&p, &subject(), "", &[], "what is CRISPR?", "", true, &crate::model::CancelToken::new()).unwrap();
        let tools = p.0.lock().unwrap();
        assert!(tools[0].iter().any(|t| matches!(t, crate::model::Tool::WebSearch)));
    }
}

