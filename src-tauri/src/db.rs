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
        // A crash during a previous first launch can leave some tables created
        // at user_version 0, which would brick every later launch on "table
        // already exists". Nothing real exists before v1 completes, so clear
        // any partial base schema and apply it atomically.
        reset_partial_base(conn)?;
        migration_step(conn, 1, |c| c.execute_batch(SCHEMA))?;
    }
    if version < 2 {
        migration_step(conn, 2, migrate_v2)?;
    }
    if version < 3 {
        migration_step(conn, 3, migrate_v3)?;
    }
    if version < 4 {
        migration_step(conn, 4, migrate_v4)?;
    }
    if version < 5 {
        migration_step(conn, 5, migrate_v5)?;
    }
    if version < 6 {
        migration_step(conn, 6, migrate_v6)?;
    }
    if version < 7 {
        migration_step(conn, 7, migrate_v7)?;
    }
    Ok(())
}

/// v6: Learning mode — subjects, intake-grill, the proposed module manifest, and
/// the generated materials (notes/flashcards/quiz/tutor) plus the adaptive
/// learner model (per-skill BKT mastery + a pace/engagement profile). All
/// `CREATE … IF NOT EXISTS` with no inbound FKs from old tables — idempotent; a
/// no-op on fresh DBs that already carry these from schema.sql.
fn migrate_v6(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS learning_subjects (
           id INTEGER PRIMARY KEY,
           title TEXT NOT NULL,
           source_kind TEXT NOT NULL CHECK (source_kind IN ('describe','upload')),
           source_text TEXT,
           stage TEXT NOT NULL DEFAULT 'intake' CHECK (stage IN ('intake','proposed','ready')),
           created_at TEXT NOT NULL DEFAULT (datetime('now')),
           updated_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS learning_intake (
           id INTEGER PRIMARY KEY,
           subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
           idx INTEGER NOT NULL, question TEXT NOT NULL, answer TEXT,
           created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS learning_modules (
           id INTEGER PRIMARY KEY,
           subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
           idx INTEGER NOT NULL,
           kind TEXT NOT NULL CHECK (kind IN ('notes','flashcards','quiz','tutor')),
           title TEXT NOT NULL, summary TEXT, skill TEXT,
           included INTEGER NOT NULL DEFAULT 1,
           status TEXT NOT NULL DEFAULT 'proposed' CHECK (status IN ('proposed','generating','ready','failed')),
           created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS learning_notes (
           id INTEGER PRIMARY KEY,
           module_id INTEGER NOT NULL REFERENCES learning_modules(id) ON DELETE CASCADE,
           body_md TEXT NOT NULL,
           created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS learning_flashcards (
           id INTEGER PRIMARY KEY,
           module_id INTEGER NOT NULL REFERENCES learning_modules(id) ON DELETE CASCADE,
           subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
           skill TEXT, front TEXT NOT NULL, back TEXT NOT NULL,
           fsrs_json TEXT, due TEXT, reps INTEGER NOT NULL DEFAULT 0,
           created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS learning_quiz_questions (
           id INTEGER PRIMARY KEY,
           module_id INTEGER NOT NULL REFERENCES learning_modules(id) ON DELETE CASCADE,
           subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
           skill TEXT, question TEXT NOT NULL, options TEXT NOT NULL,
           answer_idx INTEGER NOT NULL, explanation TEXT,
           created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS learning_quiz_attempts (
           id INTEGER PRIMARY KEY,
           question_id INTEGER NOT NULL REFERENCES learning_quiz_questions(id) ON DELETE CASCADE,
           subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
           correct INTEGER NOT NULL, latency_ms INTEGER,
           created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS learning_skill_mastery (
           subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
           skill TEXT NOT NULL,
           p_known REAL NOT NULL DEFAULT 0.3, n_obs INTEGER NOT NULL DEFAULT 0,
           updated_at TEXT NOT NULL DEFAULT (datetime('now')),
           PRIMARY KEY (subject_id, skill)
         );
         CREATE TABLE IF NOT EXISTS learning_tutor_messages (
           id INTEGER PRIMARY KEY,
           subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
           role TEXT NOT NULL CHECK (role IN ('user','assistant')),
           content TEXT NOT NULL,
           created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS learning_profile (
           subject_id INTEGER PRIMARY KEY REFERENCES learning_subjects(id) ON DELETE CASCADE,
           sessions INTEGER NOT NULL DEFAULT 0,
           total_attempts INTEGER NOT NULL DEFAULT 0,
           total_correct INTEGER NOT NULL DEFAULT 0,
           total_latency_ms INTEGER NOT NULL DEFAULT 0,
           flashcard_reviews INTEGER NOT NULL DEFAULT 0,
           updated_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE INDEX IF NOT EXISTS idx_learning_intake ON learning_intake(subject_id, idx);
         CREATE INDEX IF NOT EXISTS idx_learning_modules ON learning_modules(subject_id, idx);
         CREATE INDEX IF NOT EXISTS idx_learning_flashcards_due ON learning_flashcards(subject_id, due);
         CREATE INDEX IF NOT EXISTS idx_learning_quiz ON learning_quiz_questions(subject_id);
         CREATE INDEX IF NOT EXISTS idx_learning_tutor ON learning_tutor_messages(subject_id, id);",
    )
}

/// v5: Understand-hub additions — per-project card membership, cached premade
/// questions per card, and per-card inline chat. Pure `CREATE … IF NOT EXISTS`
/// (no inbound FKs from old tables) — idempotent; a no-op on fresh DBs.
fn migrate_v5(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS project_cards (
           project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
           term TEXT NOT NULL,
           created_at TEXT NOT NULL DEFAULT (datetime('now')),
           PRIMARY KEY (project_id, term)
         );
         CREATE TABLE IF NOT EXISTS card_questions (
           id INTEGER PRIMARY KEY,
           term TEXT NOT NULL,
           question TEXT NOT NULL,
           created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS card_chat_messages (
           id INTEGER PRIMARY KEY,
           project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
           term TEXT NOT NULL,
           role TEXT NOT NULL CHECK (role IN ('user','assistant')),
           content TEXT NOT NULL,
           created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE INDEX IF NOT EXISTS idx_card_questions_term ON card_questions(term);
         CREATE INDEX IF NOT EXISTS idx_card_chat ON card_chat_messages(project_id, term, id);",
    )
}

/// v4: add `questions.ui_spec` (model-emitted input UI per grill question).
/// Guarded by a column-existence check so it's idempotent — fresh databases
/// already carry the column from schema.sql, so the ALTER is skipped there.
fn migrate_v4(conn: &Connection) -> rusqlite::Result<()> {
    let has_col: bool = conn
        .prepare("PRAGMA table_info(questions)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .filter_map(Result::ok)
        .any(|name| name == "ui_spec");
    if !has_col {
        conn.execute_batch("ALTER TABLE questions ADD COLUMN ui_spec TEXT;")?;
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
/// v7 (Phase 19): chunked ingest. Each module remembers the section of the
/// source document it was proposed from, so material generation grounds on the
/// RIGHT part of a big upload instead of one truncated blob.
fn migrate_v7(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("ALTER TABLE learning_modules ADD COLUMN source_excerpt TEXT;")
}

/// Run one migration step atomically: the schema change and the version bump
/// commit together, so a crash mid-step rolls back to a cleanly re-runnable
/// state instead of stranding a half-applied version.
fn migration_step(
    conn: &Connection,
    to_version: i64,
    body: impl FnOnce(&Connection) -> rusqlite::Result<()>,
) -> rusqlite::Result<()> {
    conn.execute_batch("BEGIN IMMEDIATE")?;
    match body(conn).and_then(|_| conn.pragma_update(None, "user_version", to_version)) {
        Ok(()) => conn.execute_batch("COMMIT"),
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

/// Drop any tables a pre-transactional first launch left behind at version 0.
/// Table names are read from the schema itself so the list can't drift.
fn reset_partial_base(conn: &Connection) -> rusqlite::Result<()> {
    for line in SCHEMA.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("CREATE TABLE ") {
            if let Some(name) = rest.split([' ', '(']).next() {
                conn.execute_batch(&format!("DROP TABLE IF EXISTS {name};"))?;
            }
        }
    }
    Ok(())
}

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
        // schema.sql defines 28 tables (18 + the 10 learning_* tables from v6).
        assert_eq!(count, 28);
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
        assert_eq!(version, 7);
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

    #[test]
    fn partial_base_schema_from_a_crashed_first_launch_recovers() {
        // Simulate the pre-fix failure mode: some tables exist, user_version 0.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE projects (id INTEGER PRIMARY KEY, name TEXT);").unwrap();
        run_migrations(&conn).expect("a partial base schema must not brick the app");
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert!(v >= 7);
        // The real schema replaced the partial table (kind column exists).
        conn.execute("INSERT INTO projects (name, kind) VALUES ('ok','new')", []).unwrap();
    }
}
