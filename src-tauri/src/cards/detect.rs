//! Tech detection: scan an attached clone's manifests for known technologies
//! and add detected-tech cards. Treated as untrusted input — symlinks that
//! escape the clone are refused so a hostile repo can't make us read (and leak
//! into the model haystack) files outside it.

use rusqlite::{params, Connection};

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
/// generated on demand). Returns the number added. Refuses to follow symlinks
/// out of the clone (treats the clone as untrusted).
pub fn detect_tech_in_clone(conn: &Connection, clone_path: &str) -> Result<usize, String> {
    let root = std::path::Path::new(clone_path);
    let canon_root = match std::fs::canonicalize(root) {
        Ok(r) => r,
        Err(_) => return Ok(0), // clone path missing/unreadable — nothing to detect
    };
    let manifests = [
        "package.json", "Cargo.toml", "requirements.txt", "pyproject.toml", "go.mod", "Gemfile",
        "composer.json", "pom.xml",
    ];
    let mut haystack = String::new();
    for m in manifests {
        let candidate = root.join(m);
        // Resolve symlinks; require the real target to stay inside the clone so a
        // malicious clone can't point package.json at e.g. /etc/passwd or ~/.ssh.
        let Ok(real) = std::fs::canonicalize(&candidate) else {
            continue;
        };
        if !real.starts_with(&canon_root) {
            continue;
        }
        if let Ok(c) = std::fs::read_to_string(&real) {
            haystack.push_str(&c.to_lowercase());
            haystack.push('\n');
        }
    }
    let mut added = 0;
    for (key, term, domain) in KNOWN_TECH {
        if mentions(&haystack, key) && super::get(conn, term)?.is_none() {
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
    use crate::cards::get;
    use crate::db::init_connection;
    use rusqlite::Connection;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn
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

    #[cfg(unix)]
    #[test]
    fn refuses_symlinked_manifest_escaping_the_clone() {
        let conn = db();
        let base = std::env::temp_dir().join(format!("rh-detect-sym-{}", std::process::id()));
        let clone = base.join("clone");
        std::fs::create_dir_all(&clone).unwrap();
        // A file OUTSIDE the clone that mentions a known tech.
        let outside = base.join("outside.json");
        std::fs::write(&outside, r#"{"dependencies":{"react":"19"}}"#).unwrap();
        // package.json inside the clone is a symlink pointing outside it.
        std::os::unix::fs::symlink(&outside, clone.join("package.json")).unwrap();

        let added = detect_tech_in_clone(&conn, clone.to_str().unwrap()).unwrap();
        assert_eq!(added, 0, "a symlinked manifest escaping the clone must not be read");
        assert!(get(&conn, "React").unwrap().is_none());

        std::fs::remove_dir_all(&base).ok();
    }
}
