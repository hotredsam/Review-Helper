//! Learning cards — the Understand hub's content. Seeded with a curated set
//! (seed_cards.json), extended by tech detected in attached repos, and by
//! on-demand generation. Cards span build AND product domains.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub mod commands;
pub mod study;
mod detect;
pub use detect::detect_tech_in_clone;

const SEED_JSON: &str = include_str!("seed_cards.json");

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Card {
    pub id: i64,
    pub term: String,
    pub domain: Option<String>,
    pub what_md: Option<String>,
    pub when_md: Option<String>,
    pub why_md: Option<String>,
    pub source: Option<String>,
}

#[derive(Deserialize)]
struct SeedCard {
    term: String,
    domain: String,
    what: String,
    when: String,
    why: String,
}

const COLS: &str = "id, term, domain, what_md, when_md, why_md, source";

fn row_to_card(r: &rusqlite::Row) -> rusqlite::Result<Card> {
    Ok(Card {
        id: r.get(0)?,
        term: r.get(1)?,
        domain: r.get(2)?,
        what_md: r.get(3)?,
        when_md: r.get(4)?,
        why_md: r.get(5)?,
        source: r.get(6)?,
    })
}

/// Seed the curated cards once (idempotent via the term UNIQUE constraint).
pub fn seed(conn: &Connection) -> Result<usize, String> {
    let cards: Vec<SeedCard> = serde_json::from_str(SEED_JSON).map_err(|e| e.to_string())?;
    let mut added = 0;
    for c in cards {
        added += conn
            .execute(
                "INSERT INTO learning_cards (term, domain, what_md, when_md, why_md, source) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 'seed') ON CONFLICT(term) DO NOTHING",
                params![c.term, c.domain, c.what, c.when, c.why],
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(added)
}

pub fn get(conn: &Connection, term: &str) -> Result<Option<Card>, String> {
    conn.query_row(
        &format!("SELECT {COLS} FROM learning_cards WHERE term = ?1 COLLATE NOCASE"),
        params![term.trim()],
        row_to_card,
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn list(conn: &Connection) -> Result<Vec<Card>, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT {COLS} FROM learning_cards ORDER BY domain, term"))
        .map_err(|e| e.to_string())?;
    stmt.query_map([], row_to_card)
        .and_then(Iterator::collect)
        .map_err(|e| e.to_string())
}

/// Upsert a card with full content (generation + chat capture).
pub fn upsert(
    conn: &Connection,
    term: &str,
    domain: &str,
    what: &str,
    when: &str,
    why: &str,
    source: &str,
) -> Result<Card, String> {
    let term = term.trim();
    if term.is_empty() {
        return Err("A card needs a term.".into());
    }
    // NOTE: schema's UNIQUE(term) is case-sensitive while get() looks up COLLATE
    // NOCASE. The get-before-upsert pattern in callers + ON CONFLICT(term) keep
    // duplicate case-variants unreachable from app code today. Making the
    // constraint itself NOCASE is a fixed-schema change that needs sign-off.
    conn.execute(
        "INSERT INTO learning_cards (term, domain, what_md, when_md, why_md, source) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
         ON CONFLICT(term) DO UPDATE SET domain = excluded.domain, what_md = excluded.what_md, \
         when_md = excluded.when_md, why_md = excluded.why_md, source = excluded.source",
        params![term, domain, what, when, why, source],
    )
    .map_err(|e| e.to_string())?;
    get(conn, term)?.ok_or_else(|| "card vanished after upsert".into())
}

/// Capture an explanation (e.g. a chat answer) as a card, preserving any
/// existing `when`/`why` so a capture never blanks a richer card.
pub fn capture(conn: &Connection, term: &str, explanation: &str, domain: &str) -> Result<Card, String> {
    let explanation = explanation.trim();
    if explanation.is_empty() {
        return Err("Nothing to capture.".into());
    }
    let (when_md, why_md) = match get(conn, term)? {
        Some(c) => (c.when_md.unwrap_or_default(), c.why_md.unwrap_or_default()),
        None => (String::new(), String::new()),
    };
    // `source` is constrained by the fixed schema to seed/detected/generated;
    // a captured explanation is generated content.
    upsert(conn, term, normalize_domain(domain), explanation, &when_md, &why_md, "generated")
}

// ---- On-demand generation (T2) ----

const CARD_SYSTEM: &str = r#"You explain one concept as a learning card for someone vibecoding the right way (covering build AND product topics, not just tech). Given a TERM, produce a concise, honest card. Be accurate; if the term is ambiguous, take its most common software/product meaning. No fluff, no hype.

Output ONLY this JSON object (first character {, last }):
{"domain": one of "architecture"|"frontend"|"backend"|"pipes"|"deployment"|"business"|"design"|"ux"|"other",
 "what": "1-2 sentences: what it is",
 "when": "1 sentence: when to use it / reach for it",
 "why": "1 sentence: why it matters or the key trade-off"}"#;

#[derive(Debug, Deserialize)]
pub(crate) struct GenCard {
    #[serde(deserialize_with = "crate::plan::parse::flexible_string")]
    pub domain: String,
    #[serde(deserialize_with = "crate::plan::parse::flexible_string")]
    pub what: String,
    #[serde(deserialize_with = "crate::plan::parse::flexible_string")]
    pub when: String,
    #[serde(deserialize_with = "crate::plan::parse::flexible_string")]
    pub why: String,
}

/// Normalize a model-supplied domain to a schema-valid value (CHECK constraint).
pub(crate) fn normalize_domain(d: &str) -> &'static str {
    match d.trim().to_lowercase().as_str() {
        "architecture" => "architecture",
        "frontend" => "frontend",
        "backend" => "backend",
        "pipes" => "pipes",
        "deployment" => "deployment",
        "business" => "business",
        "design" => "design",
        "ux" => "ux",
        _ => "other",
    }
}

/// Parse + validate model output into a GenCard. Rejects malformed JSON and
/// incomplete content (empty what/when/why) so an empty card is never stored —
/// the "never dead-end" invariant is enforced here, not just in tests.
fn parse_gen_card(text: &str) -> Result<GenCard, String> {
    let json = crate::plan::parse::extract_json(text).ok_or("No card JSON found in the output.")?;
    let card: GenCard = serde_json::from_str(json).map_err(|_| {
        "Could not generate a card for that term — the model's response was malformed. Please try again."
            .to_string()
    })?;
    if card.what.trim().is_empty() || card.when.trim().is_empty() || card.why.trim().is_empty() {
        return Err("The model returned incomplete card content. Please try again.".into());
    }
    Ok(card)
}

/// Generate a card's content for a term via the model (no DB access). Surfaces
/// the real failure detail on the offline / unavailable / errored paths.
pub(crate) fn generate_card(provider: &dyn crate::model::ModelProvider, term: &str, cancel: &crate::model::CancelToken) -> Result<GenCard, String> {
    use crate::model::{ModelEvent, ModelRequest};
    let mut req = ModelRequest::planning(format!("Explain this term as a card: {}", term.trim()));
    req.system_append = Some(CARD_SYSTEM.to_string());
    let mut text = None;
    let mut failure: Option<String> = None;
    provider.run(&req, cancel, &mut |e: ModelEvent| match e {
        ModelEvent::Completed { text: t, .. } => text = Some(t),
        ModelEvent::Unavailable { detail, .. } | ModelEvent::Failed { detail } => {
            failure = Some(detail)
        }
        ModelEvent::Stopped => failure = Some("Stopped.".into()),
        _ => {}
    });
    if let Some(detail) = failure {
        return Err(detail);
    }
    let text = text.ok_or("The model produced no result.")?;
    parse_gen_card(&text)
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

    #[test]
    fn seeds_curated_cards_idempotently() {
        let conn = db();
        let n = seed(&conn).unwrap();
        assert!(n >= 40, "expected ~40+ seed cards, got {n}");
        assert_eq!(seed(&conn).unwrap(), 0, "re-seeding adds nothing");
        let mvp = get(&conn, "mvp").unwrap().unwrap(); // case-insensitive
        assert_eq!(mvp.domain.as_deref(), Some("business"));
        assert_eq!(mvp.source.as_deref(), Some("seed"));
        assert!(list(&conn).unwrap().len() >= 40);
    }

    #[test]
    fn upsert_creates_then_updates() {
        let conn = db();
        let c = upsert(&conn, "Foo", "other", "w", "wh", "y", "generated").unwrap();
        assert_eq!(c.what_md.as_deref(), Some("w"));
        let c2 = upsert(&conn, "Foo", "other", "w2", "wh", "y", "generated").unwrap();
        assert_eq!(c2.what_md.as_deref(), Some("w2"));
        assert_eq!(c2.id, c.id);
    }

    #[test]
    fn capture_yields_retrievable_card_and_preserves_when_why() {
        let conn = db();
        // A chat explanation becomes a retrievable card.
        let c = capture(&conn, "Idempotency", "Same call, same effect.", "backend").unwrap();
        assert_eq!(c.what_md.as_deref(), Some("Same call, same effect."));
        assert_eq!(c.source.as_deref(), Some("generated"));
        assert!(get(&conn, "idempotency").unwrap().is_some()); // retrievable, case-insensitive

        // Enrich it with when/why, then re-capture: when/why are preserved.
        upsert(&conn, "Idempotency", "backend", "x", "on retries", "avoids dup writes", "generated").unwrap();
        let re = capture(&conn, "Idempotency", "Updated explanation.", "backend").unwrap();
        assert_eq!(re.what_md.as_deref(), Some("Updated explanation."));
        assert_eq!(re.when_md.as_deref(), Some("on retries"));
        assert_eq!(re.why_md.as_deref(), Some("avoids dup writes"));

        // Empty explanation is rejected.
        assert!(capture(&conn, "Idempotency", "   ", "backend").is_err());
    }

    #[test]
    fn parse_gen_card_rejects_incomplete_or_malformed_content() {
        // null coerces to "" via flexible_string -> incomplete -> Err (never dead-end).
        assert!(parse_gen_card(r#"{"domain":"backend","what":null,"when":"x","why":"y"}"#).is_err());
        // whitespace-only field -> incomplete -> Err.
        assert!(parse_gen_card(r#"{"domain":"backend","what":"  ","when":"x","why":"y"}"#).is_err());
        // not JSON at all -> malformed -> Err (user-actionable message, no serde internals).
        let e = parse_gen_card("the model refused").unwrap_err();
        assert!(!e.contains("expected"), "error should not leak serde internals: {e}");
        // complete content -> Ok.
        let ok = parse_gen_card(r#"{"domain":"backend","what":"It is X.","when":"When Y.","why":"Because Z."}"#)
            .unwrap();
        assert_eq!(ok.what, "It is X.");
        assert_eq!(normalize_domain(&ok.domain), "backend");
    }

    #[test]
    #[ignore = "real model card generation; needs auth + uses credits. Run: cargo test -- --ignored"]
    fn real_card_generation() {
        let card = generate_card(&crate::model::claude::ClaudeCodeProvider::new(), "Bloom filter", &crate::model::CancelToken::new()).unwrap();
        assert!(!card.what.trim().is_empty());
        let domains = [
            "architecture", "frontend", "backend", "pipes", "deployment", "business", "design",
            "ux", "other",
        ];
        assert!(domains.contains(&normalize_domain(&card.domain)));
    }
}
