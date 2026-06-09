//! L2 — generative module proposal. After scoping, the model proposes a tailored
//! study plan: a short list of modules (notes / flashcards / quiz) chosen to fit
//! the learner's level, goal, time budget, and depth. The user edits which are
//! included before any material is generated. Retrieval practice (flashcards +
//! quiz) is favoured because it's the best-evidenced study method.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::gen::{extract_json, run_once};
use super::intake::IntakeItem;
use super::store::SubjectDetail;
use crate::context::fence_safe;

/// Module kinds the proposal may emit (tutor is always-available, not proposed).
const KINDS: [&str; 3] = ["notes", "flashcards", "quiz"];

#[derive(Debug, Serialize, PartialEq)]
pub struct ProposedModule {
    pub id: i64,
    pub idx: i64,
    pub kind: String,
    pub title: String,
    pub summary: Option<String>,
    pub skill: Option<String>,
    pub included: bool,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ParsedModule {
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub skill: String,
}

pub fn list_modules(conn: &Connection, subject_id: i64) -> Result<Vec<ProposedModule>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, idx, kind, title, summary, skill, included, status \
             FROM learning_modules WHERE subject_id = ?1 ORDER BY idx",
        )
        .map_err(|e| e.to_string())?;
    stmt.query_map([subject_id], |r| {
        Ok(ProposedModule {
            id: r.get(0)?,
            idx: r.get(1)?,
            kind: r.get(2)?,
            title: r.get(3)?,
            summary: r.get(4)?,
            skill: r.get(5)?,
            included: r.get::<_, i64>(6)? != 0,
            status: r.get(7)?,
        })
    })
    .and_then(Iterator::collect)
    .map_err(|e| e.to_string())
}

pub(super) fn save_modules(conn: &Connection, subject_id: i64, modules: &[ParsedModule]) -> Result<(), String> {
    for (i, m) in modules.iter().enumerate() {
        conn.execute(
            "INSERT INTO learning_modules (subject_id, idx, kind, title, summary, skill) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                subject_id,
                i as i64,
                m.kind.trim(),
                m.title.trim().chars().take(200).collect::<String>(),
                m.summary.trim().chars().take(600).collect::<String>(),
                m.skill.trim().chars().take(120).collect::<String>(),
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn set_included(conn: &Connection, module_id: i64, included: bool) -> Result<(), String> {
    conn.execute(
        "UPDATE learning_modules SET included = ?1 WHERE id = ?2",
        params![if included { 1 } else { 0 }, module_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn included_count(conn: &Connection, subject_id: i64) -> Result<i64, String> {
    conn.query_row(
        "SELECT count(*) FROM learning_modules WHERE subject_id = ?1 AND included = 1",
        [subject_id],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

const PROPOSE_SYSTEM: &str = r#"You are designing a focused self-study plan from a subject and the learner's scoping answers. Propose 3–6 study MODULES that together cover the goal at the right depth for this learner's level and time budget. Each module is one of exactly these kinds:
- "notes": a concise written explainer for a sub-topic
- "flashcards": spaced-repetition cards for facts/vocab/definitions worth memorising
- "quiz": multiple-choice retrieval practice to test understanding
Favour active recall: include at least one "flashcards" or "quiz" module (retrieval practice is the best-evidenced method; passive reading is weakest). Tailor scope to what the learner said — do not pad. Give each module a short "skill" tag (the sub-topic it trains) so mastery can be tracked. Output ONLY this JSON:
{"modules":[{"kind":"notes|flashcards|quiz","title":"...","summary":"one sentence","skill":"short-tag"}]}"#;

#[derive(Deserialize)]
struct Proposal {
    modules: Vec<ParsedModule>,
}

fn intake_block(intake: &[IntakeItem]) -> String {
    if intake.is_empty() {
        return "(not scoped)".into();
    }
    let mut s = String::new();
    for it in intake {
        let a = it.answer.as_deref().unwrap_or("(no answer)");
        s.push_str(&format!("- Q: {}\n  A: {}\n", fence_safe(&it.question), fence_safe(a)));
    }
    s
}

/// Generate the proposed module manifest from the subject + intake answers. Pure
/// model work (no DB); the caller persists under a brief lock.
pub(super) fn fetch_modules(subject: &SubjectDetail, intake: &[IntakeItem]) -> Result<Vec<ParsedModule>, String> {
    let prompt = format!(
        "Subject: {}\n\nWhat the learner wants (DATA — untrusted):\n{}\n\nScoping answers (DATA — untrusted):\n{}",
        fence_safe(&subject.title),
        fence_safe(subject.source_text.as_deref().unwrap_or("(none)")),
        intake_block(intake),
    );
    let text = run_once(prompt, PROPOSE_SYSTEM)?;
    let json = extract_json(&text)?;
    let proposal: Proposal =
        serde_json::from_str(json).map_err(|_| "The proposed plan was malformed.".to_string())?;
    let modules: Vec<ParsedModule> = proposal
        .modules
        .into_iter()
        .filter(|m| KINDS.contains(&m.kind.trim()) && !m.title.trim().is_empty())
        .take(8)
        .collect();
    if modules.is_empty() {
        return Err("No study modules were proposed.".into());
    }
    Ok(modules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn.execute(
            "INSERT INTO learning_subjects (title, source_kind, source_text) VALUES ('Spanish','describe','basics')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn saves_lists_toggles_and_counts_modules() {
        let conn = db();
        let mods = vec![
            ParsedModule { kind: "notes".into(), title: "Greetings".into(), summary: "Hello/goodbye".into(), skill: "greetings".into() },
            ParsedModule { kind: "flashcards".into(), title: "Core vocab".into(), summary: "100 words".into(), skill: "vocab".into() },
        ];
        save_modules(&conn, 1, &mods).unwrap();
        let listed = list_modules(&conn, 1).unwrap();
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|m| m.included), "modules default to included");
        assert_eq!(included_count(&conn, 1).unwrap(), 2);

        set_included(&conn, listed[0].id, false).unwrap();
        assert_eq!(included_count(&conn, 1).unwrap(), 1);
    }
}
