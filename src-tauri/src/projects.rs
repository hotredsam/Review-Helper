use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::Db;

/// A row of the `projects` table.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub app_type: Option<String>,
    pub github_repo_url: Option<String>,
    pub clone_path: Option<String>,
    pub default_branch: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

const COLUMNS: &str =
    "id, name, kind, app_type, github_repo_url, clone_path, default_branch, created_at, updated_at";

const VALID_KINDS: [&str; 2] = ["imported", "new"];

fn row_to_project(row: &rusqlite::Row) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        app_type: row.get(3)?,
        github_repo_url: row.get(4)?,
        clone_path: row.get(5)?,
        default_branch: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

// ---- Data-layer free functions (take a &Connection so they are unit-testable) ----

pub fn insert(
    conn: &Connection,
    name: &str,
    kind: &str,
    app_type: Option<&str>,
) -> Result<Project, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Project name cannot be empty.".into());
    }
    if !VALID_KINDS.contains(&kind) {
        return Err(format!(
            "Invalid project kind '{kind}'. Expected 'imported' or 'new'."
        ));
    }
    conn.execute(
        "INSERT INTO projects (name, kind, app_type) VALUES (?1, ?2, ?3)",
        params![name, kind, app_type],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    get(conn, id)?.ok_or_else(|| "Failed to load project after insert.".into())
}

/// Insert a project attached to a GitHub repo (the import / link / create-from-app
/// paths). `default_branch` falls back to `main`.
pub fn insert_attached(
    conn: &Connection,
    name: &str,
    kind: &str,
    github_repo_url: Option<&str>,
    default_branch: Option<&str>,
) -> Result<Project, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Project name cannot be empty.".into());
    }
    if !VALID_KINDS.contains(&kind) {
        return Err(format!(
            "Invalid project kind '{kind}'. Expected 'imported' or 'new'."
        ));
    }
    conn.execute(
        "INSERT INTO projects (name, kind, github_repo_url, default_branch) VALUES (?1, ?2, ?3, ?4)",
        params![name, kind, github_repo_url, default_branch.unwrap_or("main")],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    get(conn, id)?.ok_or_else(|| "Failed to load project after insert.".into())
}

/// Record where a project's repo was cloned (the local cache dir).
pub fn set_clone_path(conn: &Connection, id: i64, path: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE projects SET clone_path = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![path, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<Project>, String> {
    let sql = format!("SELECT {COLUMNS} FROM projects ORDER BY created_at ASC, id ASC");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_project)
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Project>, String> {
    let sql = format!("SELECT {COLUMNS} FROM projects WHERE id = ?1");
    conn.query_row(&sql, params![id], row_to_project)
        .optional()
        .map_err(|e| e.to_string())
}

pub fn rename(conn: &Connection, id: i64, name: &str) -> Result<Project, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Project name cannot be empty.".into());
    }
    let changed = conn
        .execute(
            "UPDATE projects SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![name, id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("No project with id {id}."));
    }
    get(conn, id)?.ok_or_else(|| "Failed to load project after update.".into())
}

pub fn delete(conn: &Connection, id: i64) -> Result<bool, String> {
    let changed = conn
        .execute("DELETE FROM projects WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(changed > 0)
}

// ---- Tauri commands: lock the shared connection, then delegate to the data layer ----

fn with_conn<T>(
    db: &State<Db>,
    f: impl FnOnce(&Connection) -> Result<T, String>,
) -> Result<T, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    f(&conn)
}

#[tauri::command]
pub fn create_project(
    db: State<Db>,
    name: String,
    kind: String,
    app_type: Option<String>,
) -> Result<Project, String> {
    with_conn(&db, |c| insert(c, &name, &kind, app_type.as_deref()))
}

#[tauri::command]
pub fn list_projects(db: State<Db>) -> Result<Vec<Project>, String> {
    with_conn(&db, list)
}

#[tauri::command]
pub fn get_project(db: State<Db>, id: i64) -> Result<Option<Project>, String> {
    with_conn(&db, |c| get(c, id))
}

#[tauri::command]
pub fn rename_project(db: State<Db>, id: i64, name: String) -> Result<Project, String> {
    with_conn(&db, |c| rename(c, id, &name))
}

#[tauri::command]
pub fn delete_project(app: tauri::AppHandle, db: State<Db>, id: i64) -> Result<bool, String> {
    let removed = with_conn(&db, |c| delete(c, id))?;
    if removed {
        // The shallow-clone cache is per-project state — deleting the project
        // without it would leak a multi-MB directory on disk forever.
        use tauri::Manager;
        if let Ok(dir) = app.path().app_data_dir() {
            let _ = std::fs::remove_dir_all(dir.join("clones").join(id.to_string()));
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn
    }

    #[test]
    fn insert_then_list_and_get() {
        let conn = memory_db();
        let a = insert(&conn, "Alpha", "new", None).unwrap();
        let b = insert(&conn, "Beta", "imported", Some("web")).unwrap();

        assert_eq!(a.name, "Alpha");
        assert_eq!(a.kind, "new");
        // DB defaults are applied and returned.
        assert_eq!(a.default_branch.as_deref(), Some("main"));
        assert!(!a.created_at.is_empty());
        assert_eq!(b.app_type.as_deref(), Some("web"));

        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "Alpha");
        assert_eq!(all[1].name, "Beta");

        let fetched = get(&conn, a.id).unwrap().unwrap();
        assert_eq!(fetched, a);
        assert!(get(&conn, 9999).unwrap().is_none());
    }

    #[test]
    fn rename_and_delete() {
        let conn = memory_db();
        let p = insert(&conn, "Old", "new", None).unwrap();
        let renamed = rename(&conn, p.id, "  New Name  ").unwrap();
        assert_eq!(renamed.name, "New Name"); // trimmed

        assert!(rename(&conn, 9999, "X").is_err()); // missing id

        assert!(delete(&conn, p.id).unwrap());
        assert!(!delete(&conn, p.id).unwrap()); // already gone
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn rejects_bad_input() {
        let conn = memory_db();
        assert!(insert(&conn, "   ", "new", None).is_err()); // empty name
        assert!(insert(&conn, "Ok", "bogus", None).is_err()); // invalid kind
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn insert_attached_sets_github_fields() {
        let conn = memory_db();
        let p = insert_attached(
            &conn,
            "Repo",
            "imported",
            Some("https://github.com/o/Repo.git"),
            Some("develop"),
        )
        .unwrap();
        assert_eq!(p.kind, "imported");
        assert_eq!(p.github_repo_url.as_deref(), Some("https://github.com/o/Repo.git"));
        assert_eq!(p.default_branch.as_deref(), Some("develop"));

        // default branch falls back to main
        let q = insert_attached(&conn, "Repo2", "new", Some("u"), None).unwrap();
        assert_eq!(q.default_branch.as_deref(), Some("main"));
    }

    #[test]
    fn projects_persist_across_reopen() {
        // Simulate an app restart: write with one connection, drop it, reopen
        // the same file, and confirm the rows are still there.
        let dir = std::env::temp_dir().join(format!("rh-db-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("persist.db");

        {
            let conn = Connection::open(&path).unwrap();
            init_connection(&conn).unwrap();
            insert(&conn, "Alpha", "new", None).unwrap();
            insert(&conn, "Beta", "imported", None).unwrap();
        } // connection dropped == app closed

        {
            let conn = Connection::open(&path).unwrap();
            init_connection(&conn).unwrap(); // migrations must be a no-op here
            let all = list(&conn).unwrap();
            assert_eq!(all.len(), 2);
            assert_eq!(all[0].name, "Alpha");
            assert_eq!(all[1].name, "Beta");
        }

        std::fs::remove_dir_all(&dir).ok();
    }
}
