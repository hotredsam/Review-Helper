//! Question generation: the bank supplies (dimension, topic, hint); the model
//! writes a repo-specific question + a recommended answer for each. Pure logic
//! (bank, prompt, parse, topic selection) — the model call lives in commands.

use std::collections::HashSet;
use std::sync::OnceLock;

use rusqlite::Connection;
use serde::Deserialize;

use crate::plan::parse::{extract_json, flexible_string};

const BANK_JSON: &str = include_str!("bank.json");

pub const GRILL_SYSTEM: &str = r#"You are Review Helper's interviewer. You ask the builder sharp, repo-specific questions that pin down what they're building — covering product AND build concerns. Explore the repository read-only in your working directory and use the PROJECT CONTEXT so every question is specific to THIS project (reference real files, the plan, the chosen stack). Never edit, write, or delete files, and never run shell commands.

You are given a list of TOPICS, each with a dimension and a focus hint. For EACH topic, write:
- "question": ONE specific question about THIS project — concrete, answerable, grounded in what you actually see. Never generic ("What is your architecture?"); name the real thing.
- "recommended_answer": your best-guess answer given the repo + plan + context — the answer you'd suggest if the builder is unsure. Honest and specific. Only say it's unknowable if it truly is.

Echo the given dimension and bank_topic verbatim.

OUTPUT: Emit ONLY this JSON object — nothing before or after, no ``` fences. First character {, last }:
{"questions": [
  {"dimension": "...", "bank_topic": "...", "question": "...", "recommended_answer": "..."}
]}
One object per provided topic, same order. This is parsed deterministically; stray text breaks it."#;

#[derive(Debug, Deserialize)]
pub struct GenQuestion {
    #[serde(deserialize_with = "flexible_string")]
    pub dimension: String,
    #[serde(deserialize_with = "flexible_string")]
    pub bank_topic: String,
    #[serde(deserialize_with = "flexible_string")]
    pub question: String,
    #[serde(deserialize_with = "flexible_string")]
    pub recommended_answer: String,
}

#[derive(Debug, Deserialize)]
struct GenBatch {
    questions: Vec<GenQuestion>,
}

#[derive(Debug, Deserialize)]
pub struct BankTopic {
    pub dimension: String,
    pub topic: String,
    pub hint: String,
}

/// The parsed topic bank (loaded once). A corrupt bank degrades grilling to
/// "no bank topics to add" (select_topics handles an empty bank) rather than
/// crashing the app on the first grill.
pub fn bank() -> &'static [BankTopic] {
    static BANK: OnceLock<Vec<BankTopic>> = OnceLock::new();
    BANK.get_or_init(|| {
        serde_json::from_str(BANK_JSON).unwrap_or_else(|e| {
            eprintln!("grill: bank.json failed to parse, continuing with no bank topics: {e}");
            Vec::new()
        })
    })
}

/// Map a depth slider value (1–5, ~1–5h) to a target total question count.
pub fn target_for_depth(depth: i64) -> i64 {
    depth.clamp(1, 5) * 5
}

/// Pick uncovered bank topics to fill up to the depth target, skipping topics
/// already present (non-deleted) for this project so re-grilling doesn't dupe.
pub fn select_topics(conn: &Connection, project_id: i64, depth: i64) -> Result<Vec<&'static BankTopic>, String> {
    let total: i64 = conn
        .query_row(
            "SELECT count(*) FROM questions WHERE project_id = ?1 AND status != 'deleted'",
            [project_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let need = (target_for_depth(depth) - total).max(0) as usize;
    if need == 0 {
        return Ok(vec![]);
    }

    let mut stmt = conn
        .prepare(
            "SELECT bank_topic FROM questions \
             WHERE project_id = ?1 AND status != 'deleted' AND bank_topic IS NOT NULL",
        )
        .map_err(|e| e.to_string())?;
    let covered: HashSet<String> = stmt
        .query_map([project_id], |r| r.get::<_, String>(0))
        .and_then(Iterator::collect::<rusqlite::Result<HashSet<_>>>)
        .map_err(|e| e.to_string())?;

    Ok(bank()
        .iter()
        .filter(|t| !covered.contains(&t.topic))
        .take(need)
        .collect())
}

/// Build the user prompt listing the topics to generate questions for.
pub fn grill_user(topics: &[&BankTopic]) -> String {
    let mut s = String::from(
        "Write one question + recommended answer for EACH of these topics, grounded in this specific project:\n\n",
    );
    for t in topics {
        s.push_str(&format!(
            "- dimension: {} | bank_topic: {} | focus: {}\n",
            t.dimension, t.topic, t.hint
        ));
    }
    s.push_str("\nExplore the repo read-only, then emit the questions JSON per your instructions.");
    s
}

/// Max length (chars) for a generated question or recommended answer. Bounds
/// DB growth on untrusted/oversized model output (mirrors the typed-answer cap).
const MAX_FIELD: usize = 5_000;

/// Parse + validate the model's question batch. Keeps only complete, bounded
/// entries: every field non-empty and question/recommended_answer within
/// MAX_FIELD. Empty/partial/oversized entries are dropped, not stored.
pub fn parse_questions(raw: &str) -> Result<Vec<GenQuestion>, String> {
    let json = extract_json(raw).ok_or("No question JSON found in the output.")?;
    let batch: GenBatch = serde_json::from_str(json)
        .map_err(|_| "The question response was malformed. Please try again.".to_string())?;
    let qs: Vec<GenQuestion> = batch
        .questions
        .into_iter()
        .filter(|q| {
            !q.question.trim().is_empty()
                && !q.recommended_answer.trim().is_empty()
                && !q.dimension.trim().is_empty()
                && !q.bank_topic.trim().is_empty()
                && q.question.len() <= MAX_FIELD
                && q.recommended_answer.len() <= MAX_FIELD
        })
        .collect();
    if qs.is_empty() {
        return Err("The model returned no usable questions.".into());
    }
    Ok(qs)
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
        conn.execute("INSERT INTO projects (name, kind) VALUES ('G','new')", []).unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn bank_loads_and_is_nonempty() {
        assert!(bank().len() >= 25, "expected a substantial topic bank");
        assert!(bank().iter().all(|t| !t.topic.is_empty() && !t.dimension.is_empty()));
    }

    #[test]
    fn depth_scales_the_target() {
        assert_eq!(target_for_depth(1), 5);
        assert_eq!(target_for_depth(5), 25);
        assert_eq!(target_for_depth(99), 25, "clamped");
        assert_eq!(target_for_depth(0), 5, "clamped up");
    }

    #[test]
    fn select_topics_fills_to_target_then_skips_covered() {
        let conn = db();
        let pid = project(&conn);
        // depth 1 -> 5 topics, none covered yet.
        let first = select_topics(&conn, pid, 1).unwrap();
        assert_eq!(first.len(), 5);

        // Insert those 5 as questions; re-selecting at depth 1 yields none (target met).
        for t in &first {
            conn.execute(
                "INSERT INTO questions (project_id, dimension, bank_topic, text, status) VALUES (?1,?2,?3,'q','open')",
                rusqlite::params![pid, t.dimension, t.topic],
            )
            .unwrap();
        }
        assert!(select_topics(&conn, pid, 1).unwrap().is_empty(), "target already met");

        // Raising depth to 2 (target 10) yields 5 more, none repeating a covered topic.
        let more = select_topics(&conn, pid, 2).unwrap();
        assert_eq!(more.len(), 5);
        let covered: std::collections::HashSet<_> = first.iter().map(|t| &t.topic).collect();
        assert!(more.iter().all(|t| !covered.contains(&t.topic)), "no repeats");
    }

    #[test]
    fn parse_questions_keeps_only_complete_bounded_entries() {
        let huge = "x".repeat(MAX_FIELD + 1);
        let raw = format!(
            r#"Here you go:
        {{"questions":[
          {{"dimension":"vision","bank_topic":"Core problem","question":"What problem does X solve?","recommended_answer":"It solves Y."}},
          {{"dimension":"users","bank_topic":"Primary user","question":"  ","recommended_answer":"skip: empty question"}},
          {{"dimension":"scope","bank_topic":"MVP","question":"Real question?","recommended_answer":""}},
          {{"dimension":"data","bank_topic":"Entities","question":"{huge}","recommended_answer":"too long"}}
        ]}}"#
        );
        let qs = parse_questions(&raw).unwrap();
        assert_eq!(qs.len(), 1, "empty-question, empty-answer, and oversized entries dropped");
        assert_eq!(qs[0].bank_topic, "Core problem");

        assert!(parse_questions("not json").is_err());
        assert!(parse_questions(r#"{"questions":[]}"#).is_err());
    }
}
