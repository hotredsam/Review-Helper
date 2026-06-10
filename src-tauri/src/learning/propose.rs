//! L2 — generative module proposal. After scoping, the model proposes a tailored
//! study plan: a short list of modules (notes / flashcards / quiz) chosen to fit
//! the learner's level, goal, time budget, and depth. The user edits which are
//! included before any material is generated. Retrieval practice (flashcards +
//! quiz) is favoured because it's the best-evidenced study method.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::gen::{extract_json, run_once};
use crate::model::{CancelToken, ModelProvider};
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

pub(super) fn save_modules(conn: &Connection, subject_id: i64, modules: &[SectionedModule]) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for (i, sm) in modules.iter().enumerate() {
        let m = &sm.module;
        tx.execute(
            "INSERT INTO learning_modules (subject_id, idx, kind, title, summary, skill, source_excerpt) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                subject_id,
                i as i64,
                m.kind.trim(),
                m.title.trim().chars().take(200).collect::<String>(),
                m.summary.trim().chars().take(600).collect::<String>(),
                m.skill.trim().chars().take(120).collect::<String>(),
                sm.excerpt,
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
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

/// A proposed module plus the source section it was proposed from (None for
/// described subjects — there's no document to excerpt).
pub(super) struct SectionedModule {
    pub module: ParsedModule,
    pub excerpt: Option<String>,
}

fn parse_proposal(text: &str, cap: usize) -> Result<Vec<ParsedModule>, String> {
    let json = extract_json(text)?;
    let proposal: Proposal =
        serde_json::from_str(json).map_err(|_| "The proposed plan was malformed.".to_string())?;
    Ok(proposal
        .modules
        .into_iter()
        .filter(|m| KINDS.contains(&m.kind.trim()) && !m.title.trim().is_empty())
        .take(cap)
        .collect())
}

fn norm_title(t: &str) -> String {
    t.trim().to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Generate the proposed module manifest. Small sources are one call (today's
/// path); large uploads are split into sections, proposed per section (1–3
/// modules each, labeled), then merged with near-duplicate titles dropped —
/// full-document coverage instead of the old silent 40k truncation. `progress`
/// is called as (done_sections, total_sections).
pub(super) fn fetch_modules(
    provider: &dyn ModelProvider,
    subject: &SubjectDetail,
    intake: &[IntakeItem],
    cancel: &CancelToken,
    mut progress: impl FnMut(usize, usize),
) -> Result<Vec<SectionedModule>, String> {
    let source = subject.source_text.as_deref().unwrap_or("");
    let sections = super::sections::split_sections(source, super::sections::SECTION_TARGET_CHARS);

    if sections.len() <= 1 {
        let prompt = format!(
            "Subject: {}\n\nWhat the learner wants (DATA — untrusted):\n{}\n\nScoping answers (DATA — untrusted):\n{}",
            fence_safe(&subject.title),
            fence_safe(if source.is_empty() { "(none)" } else { source }),
            intake_block(intake),
        );
        let text = run_once(provider, prompt, PROPOSE_SYSTEM, cancel)?;
        let modules = parse_proposal(&text, 8)?;
        if modules.is_empty() {
            return Err("No study modules were proposed.".into());
        }
        let excerpt = (!source.is_empty()).then(|| source.to_string());
        return Ok(modules.into_iter().map(|m| SectionedModule { module: m, excerpt: excerpt.clone() }).collect());
    }

    let total = sections.len();
    let mut out: Vec<SectionedModule> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (i, section) in sections.iter().enumerate() {
        if cancel.is_cancelled() {
            return Err("Stopped.".into());
        }
        let label = section.title.clone().unwrap_or_else(|| format!("Part {}", i + 1));
        let prompt = format!(
            "Subject: {} — section {}/{} ({})\n\nThis SECTION of the learner's material (DATA — untrusted):\n{}\n\nScoping answers (DATA — untrusted):\n{}\n\nPropose 1–3 modules covering THIS SECTION only.",
            fence_safe(&subject.title),
            i + 1,
            total,
            fence_safe(&label),
            fence_safe(&section.body),
            intake_block(intake),
        );
        let text = run_once(provider, prompt, PROPOSE_SYSTEM, cancel)?;
        for m in parse_proposal(&text, 3)? {
            // Merge pass: overlapping sections often re-propose the same topic.
            if seen.insert(norm_title(&m.title)) {
                out.push(SectionedModule { module: m, excerpt: Some(section.body.clone()) });
            }
        }
        progress(i + 1, total);
        if out.len() >= 12 {
            break; // plan stays studyable; later sections still ground the tutor via Phase 21
        }
    }
    if out.is_empty() {
        return Err("No study modules were proposed.".into());
    }
    Ok(out)
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
            SectionedModule {
                module: ParsedModule { kind: "notes".into(), title: "Greetings".into(), summary: "Hello/goodbye".into(), skill: "greetings".into() },
                excerpt: Some("Chapter on greetings".into()),
            },
            SectionedModule {
                module: ParsedModule { kind: "flashcards".into(), title: "Core vocab".into(), summary: "100 words".into(), skill: "vocab".into() },
                excerpt: None,
            },
        ];
        save_modules(&conn, 1, &mods).unwrap();
        let listed = list_modules(&conn, 1).unwrap();
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|m| m.included), "modules default to included");
        assert_eq!(included_count(&conn, 1).unwrap(), 2);

        set_included(&conn, listed[0].id, false).unwrap();
        assert_eq!(included_count(&conn, 1).unwrap(), 1);
    }

    struct CountingProvider {
        calls: std::sync::atomic::AtomicUsize,
        json: &'static str,
    }
    impl crate::model::ModelProvider for CountingProvider {
        fn run(&self, _req: &crate::model::ModelRequest, _cancel: &CancelToken, sink: &mut dyn FnMut(crate::model::ModelEvent)) {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            sink(crate::model::ModelEvent::Completed { session_id: None, text: self.json.to_string() });
        }
    }

    fn subject_with_source(source: &str) -> SubjectDetail {
        SubjectDetail {
            id: 1,
            title: "Biology".into(),
            source_kind: "upload".into(),
            source_text: Some(source.to_string()),
            stage: "intake".into(),
        }
    }

    #[test]
    fn small_sources_propose_in_one_call_with_full_excerpt() {
        let p = CountingProvider {
            calls: Default::default(),
            json: r#"{"modules":[{"kind":"notes","title":"Cells","summary":"s","skill":"cells"}]}"#,
        };
        let out = fetch_modules(&p, &subject_with_source("short doc"), &[], &CancelToken::new(), |_, _| {}).unwrap();
        assert_eq!(p.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].excerpt.as_deref(), Some("short doc"));
    }

    #[test]
    fn large_sources_propose_per_section_and_dedupe_titles() {
        // ~3 sections of distinct content; the scripted model proposes the same
        // module title every call, so the merge keeps exactly one.
        let mut doc = String::new();
        for i in 0..3 {
            doc.push_str(&format!("# Chapter {i}\n\n{}\n\n", format!("content {i} ").repeat(2500)));
        }
        let p = CountingProvider {
            calls: Default::default(),
            json: r#"{"modules":[{"kind":"quiz","title":"Photosynthesis","summary":"s","skill":"photo"}]}"#,
        };
        let mut progress: Vec<(usize, usize)> = Vec::new();
        let out = fetch_modules(&p, &subject_with_source(&doc), &[], &CancelToken::new(), |d, t| progress.push((d, t))).unwrap();

        let calls = p.calls.load(std::sync::atomic::Ordering::SeqCst);
        assert!(calls >= 2, "a large doc must be proposed per section, got {calls} call(s)");
        assert_eq!(out.len(), 1, "duplicate titles across sections merge");
        // The surviving module is grounded on the section it came from.
        assert!(out[0].excerpt.as_deref().unwrap().contains("content 0"));
        assert_eq!(progress.last().map(|p| p.1), Some(calls), "progress reaches the section count");
    }
}
