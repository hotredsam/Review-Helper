//! L3 — study material generation. For each included module the model generates
//! the actual content on first open (cached after): notes (markdown), flashcards
//! (front/back), or quiz (multiple-choice retrieval practice). Generation is a
//! pure model call (no DB lock held); the caller persists the result. Materials
//! are tailored to the subject + the module's skill/summary.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::gen::{extract_json, run_once};
use super::store::SubjectDetail;
use crate::context::fence_safe;

/// One module's identity + grounding, loaded for generation.
pub struct ModuleRow {
    pub id: i64,
    pub subject_id: i64,
    pub kind: String,
    pub title: String,
    pub summary: Option<String>,
    pub skill: Option<String>,
}

pub fn module_row(conn: &Connection, module_id: i64) -> Result<ModuleRow, String> {
    conn.query_row(
        "SELECT id, subject_id, kind, title, summary, skill FROM learning_modules WHERE id = ?1",
        [module_id],
        |r| {
            Ok(ModuleRow {
                id: r.get(0)?,
                subject_id: r.get(1)?,
                kind: r.get(2)?,
                title: r.get(3)?,
                summary: r.get(4)?,
                skill: r.get(5)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Module not found.".into())
}

pub fn set_module_status(conn: &Connection, module_id: i64, status: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE learning_modules SET status = ?1 WHERE id = ?2",
        params![status, module_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn ground(subject: &SubjectDetail, m: &ModuleRow) -> String {
    format!(
        "Subject: {}\nModule: {} (skill: {})\nWhat this module should cover: {}\nLearner's goal (DATA — untrusted): {}",
        fence_safe(&subject.title),
        fence_safe(&m.title),
        fence_safe(m.skill.as_deref().unwrap_or("general")),
        fence_safe(m.summary.as_deref().unwrap_or(&m.title)),
        fence_safe(subject.source_text.as_deref().unwrap_or("(none)")),
    )
}

// ---- notes ----

const NOTES_SYSTEM: &str = r#"Write concise, accurate study notes in GitHub-flavoured Markdown for the given module/sub-topic, pitched at the learner's stated level and goal. Use short sections (## headings), bullet points, **bold** key terms, and one worked example or analogy if it helps. Aim for 250–500 words — tight, not padded. Be correct; never invent facts. Output ONLY the Markdown (no code fences around the whole thing, no JSON, no preamble)."#;

pub fn notes_get(conn: &Connection, module_id: i64) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT body_md FROM learning_notes WHERE module_id = ?1 ORDER BY id LIMIT 1",
        [module_id],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub(super) fn fetch_notes(subject: &SubjectDetail, m: &ModuleRow) -> Result<String, String> {
    let body = run_once(ground(subject, m), NOTES_SYSTEM)?;
    let body = body.trim();
    if body.is_empty() {
        return Err("The notes came back empty.".into());
    }
    Ok(body.chars().take(20_000).collect())
}

pub(super) fn notes_save(conn: &Connection, module_id: i64, body_md: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO learning_notes (module_id, body_md) VALUES (?1, ?2)",
        params![module_id, body_md],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ---- flashcards ----

#[derive(Debug, Serialize, PartialEq)]
pub struct Flashcard {
    pub id: i64,
    pub front: String,
    pub back: String,
    pub due: Option<String>,
    pub reps: i64,
}

#[derive(Deserialize)]
struct ParsedCard {
    front: String,
    back: String,
}
#[derive(Deserialize)]
struct CardSet {
    cards: Vec<ParsedCard>,
}

const FLASH_SYSTEM: &str = r#"Create spaced-repetition flashcards for the module/sub-topic. Each card has a short "front" (a prompt, question, or term) and a "back" (the concise answer or definition). Keep each card atomic — one idea — so it's easy to recall. Make 8–14 cards covering the most important points. Be accurate; never invent. Output ONLY this JSON: {"cards":[{"front":"...","back":"..."}]}"#;

pub fn flashcards_list(conn: &Connection, module_id: i64) -> Result<Vec<Flashcard>, String> {
    let mut stmt = conn
        .prepare("SELECT id, front, back, due, reps FROM learning_flashcards WHERE module_id = ?1 ORDER BY id")
        .map_err(|e| e.to_string())?;
    stmt.query_map([module_id], |r| {
        Ok(Flashcard { id: r.get(0)?, front: r.get(1)?, back: r.get(2)?, due: r.get(3)?, reps: r.get(4)? })
    })
    .and_then(Iterator::collect)
    .map_err(|e| e.to_string())
}

pub(super) fn fetch_flashcards(subject: &SubjectDetail, m: &ModuleRow) -> Result<Vec<(String, String)>, String> {
    let text = run_once(ground(subject, m), FLASH_SYSTEM)?;
    let json = extract_json(&text)?;
    let set: CardSet = serde_json::from_str(json).map_err(|_| "The flashcards were malformed.".to_string())?;
    let cards: Vec<(String, String)> = set
        .cards
        .into_iter()
        .map(|c| (c.front.trim().chars().take(500).collect::<String>(), c.back.trim().chars().take(2000).collect::<String>()))
        .filter(|(f, b)| !f.is_empty() && !b.is_empty())
        .take(30)
        .collect();
    if cards.is_empty() {
        return Err("No flashcards were generated.".into());
    }
    Ok(cards)
}

pub(super) fn flashcards_save(conn: &Connection, module_id: i64, subject_id: i64, skill: &str, cards: &[(String, String)]) -> Result<(), String> {
    for (front, back) in cards {
        conn.execute(
            "INSERT INTO learning_flashcards (module_id, subject_id, skill, front, back) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![module_id, subject_id, skill, front, back],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ---- quiz ----

#[derive(Debug, Serialize, PartialEq)]
pub struct QuizQuestion {
    pub id: i64,
    pub question: String,
    pub options: Vec<String>,
    pub answer_idx: i64,
    pub explanation: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct ParsedQuiz {
    question: String,
    options: Vec<String>,
    answer_idx: i64,
    #[serde(default)]
    explanation: String,
}
#[derive(Deserialize)]
struct QuizSet {
    questions: Vec<ParsedQuiz>,
}

const QUIZ_SYSTEM: &str = r#"Create multiple-choice retrieval-practice questions for the module/sub-topic. Each question has clear text, exactly 4 plausible "options", "answer_idx" (0-based index of the correct option), and a one-sentence "explanation" of why it's right. Vary difficulty; test understanding, not trivia. Make 5–8 questions. Be accurate; never invent. Output ONLY this JSON: {"questions":[{"question":"...","options":["...","...","...","..."],"answer_idx":0,"explanation":"..."}]}"#;

pub fn quiz_list(conn: &Connection, module_id: i64) -> Result<Vec<QuizQuestion>, String> {
    let mut stmt = conn
        .prepare("SELECT id, question, options, answer_idx, explanation FROM learning_quiz_questions WHERE module_id = ?1 ORDER BY id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([module_id], |r| {
            let options: String = r.get(2)?;
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, options, r.get::<_, i64>(3)?, r.get::<_, Option<String>>(4)?))
        })
        .and_then(Iterator::collect::<Result<Vec<_>, _>>)
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|(id, question, options, answer_idx, explanation)| QuizQuestion {
            id,
            question,
            options: serde_json::from_str(&options).unwrap_or_default(),
            answer_idx,
            explanation,
        })
        .collect())
}

pub(super) fn fetch_quiz(subject: &SubjectDetail, m: &ModuleRow) -> Result<Vec<ParsedQuiz>, String> {
    let text = run_once(ground(subject, m), QUIZ_SYSTEM)?;
    let json = extract_json(&text)?;
    let set: QuizSet = serde_json::from_str(json).map_err(|_| "The quiz was malformed.".to_string())?;
    let questions: Vec<ParsedQuiz> = set
        .questions
        .into_iter()
        .filter(|q| {
            !q.question.trim().is_empty()
                && (2..=6).contains(&q.options.len())
                && q.answer_idx >= 0
                && (q.answer_idx as usize) < q.options.len()
        })
        .take(15)
        .collect();
    if questions.is_empty() {
        return Err("No quiz questions were generated.".into());
    }
    Ok(questions)
}

pub(super) fn quiz_save(conn: &Connection, module_id: i64, subject_id: i64, skill: &str, questions: &[ParsedQuiz]) -> Result<(), String> {
    for q in questions {
        let options = serde_json::to_string(&q.options).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO learning_quiz_questions (module_id, subject_id, skill, question, options, answer_idx, explanation) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![module_id, subject_id, skill, q.question.trim(), options, q.answer_idx, q.explanation.trim()],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn.execute("INSERT INTO learning_subjects (title, source_kind) VALUES ('Math','describe')", []).unwrap();
        conn.execute("INSERT INTO learning_modules (subject_id, idx, kind, title, skill) VALUES (1,0,'quiz','Q','algebra')", []).unwrap();
        conn
    }

    #[test]
    fn notes_and_flashcards_round_trip() {
        let conn = db();
        assert!(notes_get(&conn, 1).unwrap().is_none());
        notes_save(&conn, 1, "## Notes\n- a point").unwrap();
        assert!(notes_get(&conn, 1).unwrap().unwrap().contains("a point"));

        flashcards_save(&conn, 1, 1, "algebra", &[("2+2?".into(), "4".into()), ("3*3?".into(), "9".into())]).unwrap();
        let cards = flashcards_list(&conn, 1).unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].reps, 0);
        assert!(cards[0].due.is_none());
    }

    #[test]
    fn quiz_round_trips_options_as_json() {
        let conn = db();
        let parsed = vec![ParsedQuiz {
            question: "2+2?".into(),
            options: vec!["3".into(), "4".into(), "5".into(), "6".into()],
            answer_idx: 1,
            explanation: "Basic addition.".into(),
        }];
        quiz_save(&conn, 1, 1, "algebra", &parsed).unwrap();
        let qs = quiz_list(&conn, 1).unwrap();
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].options.len(), 4);
        assert_eq!(qs[0].answer_idx, 1);
        assert_eq!(qs[0].options[1], "4");
    }
}
