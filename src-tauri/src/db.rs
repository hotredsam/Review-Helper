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
    Ok(())
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
        // schema.sql defines 13 tables.
        assert_eq!(count, 13);
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
        assert_eq!(version, 1);
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
