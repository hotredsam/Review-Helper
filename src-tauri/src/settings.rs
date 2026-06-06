//! App settings persisted in the SQLite `settings` table (key/value). The model
//! provider config lives here (not in the frontend) because the backend must
//! read it to route every `model_run`.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::Db;

pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn set(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Which provider model calls route to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Claude,
    Local,
}

/// The persisted model-provider configuration. `local_endpoint` and
/// `api_credit_overflow` are stubs in v1 — stored and surfaced, no behavior yet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelConfig {
    pub provider: ProviderKind,
    pub local_endpoint: Option<String>,
    pub api_credit_overflow: bool,
}

impl Default for ModelConfig {
    fn default() -> Self {
        ModelConfig {
            provider: ProviderKind::Claude,
            local_endpoint: None,
            api_credit_overflow: false,
        }
    }
}

const MODEL_CONFIG_KEY: &str = "model.config";

/// Load the config, falling back to the default (Claude) when absent or malformed.
pub fn load_model_config(conn: &Connection) -> ModelConfig {
    match get(conn, MODEL_CONFIG_KEY) {
        Ok(Some(json)) => serde_json::from_str(&json).unwrap_or_default(),
        _ => ModelConfig::default(),
    }
}

pub fn save_model_config(conn: &Connection, config: &ModelConfig) -> Result<(), String> {
    let json = serde_json::to_string(config).map_err(|e| e.to_string())?;
    set(conn, MODEL_CONFIG_KEY, &json)
}

#[tauri::command]
pub fn get_model_config(db: State<Db>) -> Result<ModelConfig, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    Ok(load_model_config(&conn))
}

#[tauri::command]
pub fn set_model_config(db: State<Db>, config: ModelConfig) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    save_model_config(&conn, &config)
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
    fn set_get_roundtrip_and_upsert() {
        let conn = memory_db();
        assert_eq!(get(&conn, "k").unwrap(), None);
        set(&conn, "k", "v1").unwrap();
        assert_eq!(get(&conn, "k").unwrap().as_deref(), Some("v1"));
        set(&conn, "k", "v2").unwrap(); // upsert, not a duplicate-key error
        assert_eq!(get(&conn, "k").unwrap().as_deref(), Some("v2"));
    }

    #[test]
    fn defaults_to_claude_when_empty() {
        let conn = memory_db();
        assert_eq!(load_model_config(&conn), ModelConfig::default());
        assert_eq!(load_model_config(&conn).provider, ProviderKind::Claude);
    }

    #[test]
    fn model_config_roundtrips() {
        let conn = memory_db();
        let config = ModelConfig {
            provider: ProviderKind::Local,
            local_endpoint: Some("http://localhost:11434/v1".into()),
            api_credit_overflow: true,
        };
        save_model_config(&conn, &config).unwrap();
        assert_eq!(load_model_config(&conn), config);
    }

    #[test]
    fn malformed_config_falls_back_to_default() {
        let conn = memory_db();
        set(&conn, MODEL_CONFIG_KEY, "{not valid json").unwrap();
        assert_eq!(load_model_config(&conn), ModelConfig::default());
    }
}
