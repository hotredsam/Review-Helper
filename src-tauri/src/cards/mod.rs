//! Learning cards — the Understand hub's content. Seeded with a curated set
//! (seed_cards.json), extended by tech detected in attached repos, and by
//! on-demand generation. Cards span build AND product domains.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub mod commands;

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

/// Upsert a card with full content (generation + chat capture). Wired in T2.
#[allow(dead_code)]
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

/// Known technologies: (match key, display term, domain).
const KNOWN_TECH: &[(&str, &str, &str)] = &[
    ("react", "React", "frontend"),
    ("vue", "Vue", "frontend"),
    ("svelte", "Svelte", "frontend"),
    ("angular", "Angular", "frontend"),
    ("solid-js", "SolidJS", "frontend"),
    ("next", "Next.js", "frontend"),
    ("nuxt", "Nuxt", "frontend"),
    ("vite", "Vite", "frontend"),
    ("tailwindcss", "Tailwind CSS", "frontend"),
    ("typescript", "TypeScript", "frontend"),
    ("express", "Express", "backend"),
    ("fastify", "Fastify", "backend"),
    ("nestjs", "NestJS", "backend"),
    ("django", "Django", "backend"),
    ("flask", "Flask", "backend"),
    ("fastapi", "FastAPI", "backend"),
    ("rails", "Ruby on Rails", "backend"),
    ("laravel", "Laravel", "backend"),
    ("axum", "Axum", "backend"),
    ("actix-web", "Actix Web", "backend"),
    ("postgresql", "PostgreSQL", "backend"),
    ("postgres", "PostgreSQL", "backend"),
    ("mysql", "MySQL", "backend"),
    ("sqlite", "SQLite", "backend"),
    ("rusqlite", "SQLite", "backend"),
    ("mongodb", "MongoDB", "backend"),
    ("redis", "Redis", "backend"),
    ("prisma", "Prisma", "backend"),
    ("graphql", "GraphQL", "backend"),
    ("tauri", "Tauri", "architecture"),
    ("electron", "Electron", "architecture"),
    ("docker", "Docker", "deployment"),
    ("kubernetes", "Kubernetes", "deployment"),
    ("terraform", "Terraform", "deployment"),
    ("kafka", "Kafka", "pipes"),
    ("celery", "Celery", "pipes"),
    ("stripe", "Stripe", "pipes"),
];

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric()
}

/// Whole-word (boundary-aware) presence check, to avoid matching e.g. "react"
/// inside "preact". `haystack` should already be lowercased.
fn mentions(haystack: &str, word: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut from = 0;
    while let Some(pos) = haystack[from..].find(word) {
        let start = from + pos;
        let end = start + word.len();
        let before_ok = start == 0 || !is_word_char(bytes[start - 1]);
        let after_ok = end >= bytes.len() || !is_word_char(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

/// Scan a clone's manifests for known tech and add detected-tech cards (content
/// generated on demand). Returns the number added.
pub fn detect_tech_in_clone(conn: &Connection, clone_path: &str) -> Result<usize, String> {
    let root = std::path::Path::new(clone_path);
    let manifests = [
        "package.json", "Cargo.toml", "requirements.txt", "pyproject.toml", "go.mod", "Gemfile",
        "composer.json", "pom.xml",
    ];
    let mut haystack = String::new();
    for m in manifests {
        if let Ok(c) = std::fs::read_to_string(root.join(m)) {
            haystack.push_str(&c.to_lowercase());
            haystack.push('\n');
        }
    }
    let mut added = 0;
    for (key, term, domain) in KNOWN_TECH {
        if mentions(&haystack, key) && get(conn, term)?.is_none() {
            conn.execute(
                "INSERT INTO learning_cards (term, domain, source) VALUES (?1, ?2, 'detected') \
                 ON CONFLICT(term) DO NOTHING",
                params![term, domain],
            )
            .map_err(|e| e.to_string())?;
            added += 1;
        }
    }
    Ok(added)
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
    fn detects_tech_from_manifests_with_word_boundaries() {
        let conn = db();
        let dir = std::env::temp_dir().join(format!("rh-cards-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("package.json"),
            r#"{"dependencies":{"react":"19","express":"4","preact-compat":"1"}}"#,
        )
        .unwrap();

        let added = detect_tech_in_clone(&conn, dir.to_str().unwrap()).unwrap();
        assert!(added >= 2);
        assert!(get(&conn, "React").unwrap().is_some());
        assert!(get(&conn, "Express").unwrap().is_some());
        assert_eq!(get(&conn, "React").unwrap().unwrap().source.as_deref(), Some("detected"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn mentions_respects_word_boundaries() {
        assert!(mentions("\"react\": \"19\"", "react"));
        assert!(mentions("react-dom", "react"));
        assert!(!mentions("preact", "react"));
        assert!(!mentions("contextual", "next"));
    }
}
