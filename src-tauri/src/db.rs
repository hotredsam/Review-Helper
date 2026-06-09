use rusqlite::Connection;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

/// Managed Tauri state: the one app-wide SQLite connection behind a mutex.
/// Every command locks this to talk to the database; the frontend never does.
pub struct Db(pub Mutex<Connection>);

/// The full schema (13 tables), embedded at compile time so the binary carries
/// no runtime dependency on the planning file. The schema is fixed and tested —
/// it is used as-is, never hand-edited here.
const SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../.planning/schema.sql"
));

/// Open the app database in the OS app-data dir, enable foreign keys, and run
/// migrations. Called once at startup.
pub fn connect_app_db(app: &AppHandle) -> Result<Connection, Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let conn = Connection::open(dir.join("review-helper.db"))?;
    init_connection(&conn)?;
    Ok(conn)
}

/// Per-connection setup: enable foreign-key enforcement (a per-connection
/// pragma, not persisted in the file) and apply migrations. Shared by the app
/// and by tests that open their own connections.
pub fn init_connection(conn: &Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "foreign_keys", true)?;
    // Crash-safe persistence: WAL gives atomic commits and recovery if the app
    // is killed mid-write (the plan/decision record must never be left torn);
    // synchronous=NORMAL is the safe, fast pairing for WAL. journal_mode returns
    // a row, so set both via execute_batch (which ignores result rows). WAL is a
    // no-op on in-memory test DBs, which is fine. synchronous is per-connection,
    // so it lives here in the shared init path.
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    // Background model threads (plan/grill) hold the gates concurrently with
    // foreground commands; without a busy timeout a transient lock returns
    // SQLITE_BUSY immediately instead of waiting briefly and retrying.
    conn.busy_timeout(std::time::Duration::from_millis(5000))?;
    run_migrations(conn)
}

/// Idempotent migration. The fixed schema is applied once, guarded by SQLite's
/// `user_version`; on an already-migrated database this is a no-op, so reopening
/// (e.g. on every app restart) never errors on "table already exists".
pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(SCHEMA)?;
        conn.pragma_update(None, "user_version", 1)?;
    }
    if version < 2 {
        migrate_v2(conn)?;
        conn.pragma_update(None, "user_version", 2)?;
    }
    if version < 3 {
        migrate_v3(conn)?;
        conn.pragma_update(None, "user_version", 3)?;
    }
    Ok(())
}

/// v3: persisted chat transcripts + messages (past chats survive restarts; the
/// model gets the full text of all prior chats). No inbound FKs from old tables,
/// so this is pure `CREATE … IF NOT EXISTS` — fully idempotent. Fresh databases
/// already carry these from schema.sql, so the migration is a no-op there.
fn migrate_v3(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS chat_transcripts (
           id INTEGER PRIMARY KEY,
           project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
           title TEXT,
           created_at TEXT NOT NULL DEFAULT (datetime('now')),
           updated_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS chat_messages (
           id INTEGER PRIMARY KEY,
           transcript_id INTEGER NOT NULL REFERENCES chat_transcripts(id) ON DELETE CASCADE,
           role TEXT NOT NULL CHECK (role IN ('user','assistant')),
           content TEXT NOT NULL,
           created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE INDEX IF NOT EXISTS idx_chat_transcripts_project ON chat_transcripts(project_id, updated_at);
         CREATE INDEX IF NOT EXISTS idx_chat_messages_transcript ON chat_messages(transcript_id, id);",
    )
}

/// v2: make `learning_cards.term` uniqueness case-insensitive, and add a
/// project-leading answers index. SQLite cannot alter a UNIQUE constraint in
/// place, so the term change is a table rebuild (cards have no inbound foreign
/// keys, so this is safe). `INSERT OR IGNORE` collapses any pre-existing
/// case-variant duplicates onto the oldest row. Fresh databases already carry
/// the NOCASE constraint from `schema.sql`, so the rebuild is a harmless no-op
/// there; everything here is idempotent.
fn migrate_v2(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE learning_cards_new (
           id INTEGER PRIMARY KEY, term TEXT NOT NULL,
           domain TEXT CHECK (domain IN ('architecture','frontend','backend','pipes','deployment','business','design','ux','other')),
           what_md TEXT, when_md TEXT, why_md TEXT,
           source TEXT CHECK (source IN ('seed','detected','generated')),
           created_at TEXT NOT NULL DEFAULT (datetime('now')),
           UNIQUE (term COLLATE NOCASE)
         );
         INSERT OR IGNORE INTO learning_cards_new (id, term, domain, what_md, when_md, why_md, source, created_at)
           SELECT id, term, domain, what_md, when_md, why_md, source, created_at
           FROM learning_cards ORDER BY id;
         DROP TABLE learning_cards;
         ALTER TABLE learning_cards_new RENAME TO learning_cards;
         CREATE INDEX IF NOT EXISTS idx_answers_project_question ON answers(project_id, question_id);",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrated_memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn
    }

    #[test]
    fn migrations_create_all_tables() {
        let conn = migrated_memory_db();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // schema.sql defines 15 tables (13 + chat_transcripts + chat_messages).
        assert_eq!(count, 15);
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = migrated_memory_db();
        // Re-running must not error even though every table already exists.
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
        let projects: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'projects'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(projects, 1);
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 3);
    }

    #[test]
    fn learning_card_term_uniqueness_is_case_insensitive() {
        let conn = migrated_memory_db();
        conn.execute("INSERT INTO learning_cards (term, source) VALUES ('Foo', 'seed')", [])
            .unwrap();
        // A case-variant of an existing term collides — no duplicate card.
        let dup = conn.execute("INSERT INTO learning_cards (term, source) VALUES ('foo', 'seed')", []);
        assert!(dup.is_err(), "'foo' must collide with 'Foo' under NOCASE uniqueness");
    }

    #[test]
    fn busy_timeout_is_set() {
        let conn = migrated_memory_db();
        let ms: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ms, 5000, "busy_timeout is set so transient locks retry, not fail");
    }

    #[test]
    fn foreign_keys_are_enabled() {
        let conn = migrated_memory_db();
        let on: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(on, 1);
    }
}
