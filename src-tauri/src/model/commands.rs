//! Tauri commands for the model layer. The frontend calls `model_run`/`model_status`
//! and listens for `model-event`s; it never spawns `claude` itself. Routing
//! (Claude vs. the local stub) is read from the persisted settings on every call.

use std::process::Command;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use super::claude::{classify_stderr, ClaudeCodeProvider};
use super::local::LocalStubProvider;
use super::{ModelEvent, ModelProvider, ModelRequest, UnavailableReason};
use crate::db::Db;
use crate::settings::{load_model_config, ModelConfig, ProviderKind};

/// The single place the active provider is chosen, from the persisted config.
pub(crate) fn provider_for(config: &ModelConfig) -> Box<dyn ModelProvider> {
    match config.provider {
        ProviderKind::Claude => Box::new(ClaudeCodeProvider::new()),
        ProviderKind::Local => Box::new(LocalStubProvider),
    }
}

/// Run a read-only planning request on a background thread, emitting each
/// `ModelEvent` to the frontend as `model-event`. Returns immediately.
#[tauri::command]
pub fn model_run(app: AppHandle, db: State<'_, Db>, prompt: String, session_id: Option<String>) {
    // Read the routing config before spawning (State isn't 'static).
    let config = match db.0.lock() {
        Ok(conn) => load_model_config(&conn),
        Err(_) => ModelConfig::default(),
    };
    std::thread::spawn(move || {
        let provider = provider_for(&config);
        let mut req = ModelRequest::planning(prompt);
        req.session_id = session_id;
        provider.run(&req, &mut |event: ModelEvent| {
            let _ = app.emit("model-event", &event);
        });
    });
}

/// Whether the active provider is usable, with enough debug detail (the probe
/// command, exit code, stderr) to explain why not. Drives the "Claude not
/// available" banner and the debug panel.
#[derive(Debug, Serialize)]
pub struct ModelStatus {
    pub provider: ProviderKind,
    pub available: bool,
    pub version: Option<String>,
    pub reason: Option<UnavailableReason>,
    pub command: String,
    pub exit_code: Option<i32>,
    pub stderr: String,
}

#[tauri::command]
pub fn model_status(db: State<'_, Db>) -> Result<ModelStatus, String> {
    let config = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        load_model_config(&conn)
    };
    Ok(match config.provider {
        ProviderKind::Local => ModelStatus {
            provider: ProviderKind::Local,
            available: true,
            version: None,
            reason: None,
            command: "(local stub)".into(),
            exit_code: Some(0),
            stderr: String::new(),
        },
        ProviderKind::Claude => probe_claude("claude"),
    })
}

/// Probe the Claude CLI with `--version` (free; no model call) and report the
/// command, exit code and stderr so the UI can explain availability.
fn probe_claude(binary: &str) -> ModelStatus {
    let command = format!("{binary} --version");
    match Command::new(binary).arg("--version").output() {
        Ok(out) => {
            let available = out.status.success();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            ModelStatus {
                provider: ProviderKind::Claude,
                available,
                version: available
                    .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string()),
                reason: (!available).then(|| classify_stderr(&stderr)),
                command,
                exit_code: out.status.code(),
                stderr,
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ModelStatus {
            provider: ProviderKind::Claude,
            available: false,
            version: None,
            reason: Some(UnavailableReason::NotInstalled),
            command,
            exit_code: None,
            stderr: format!("`{binary}` was not found on PATH."),
        },
        Err(e) => ModelStatus {
            provider: ProviderKind::Claude,
            available: false,
            version: None,
            reason: Some(UnavailableReason::Unknown),
            command,
            exit_code: None,
            stderr: e.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_local_config_to_the_stub() {
        let config = ModelConfig {
            provider: ProviderKind::Local,
            local_endpoint: None,
            api_credit_overflow: false,
        };
        let provider = provider_for(&config);
        let mut events = Vec::new();
        provider.run(&ModelRequest::planning("hi"), &mut |e| events.push(e));
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ModelEvent::Unavailable { .. }));
    }

    #[test]
    fn probe_of_missing_binary_is_unavailable() {
        let status = probe_claude("definitely-not-a-real-binary-xyz");
        assert!(!status.available);
        assert_eq!(status.reason, Some(UnavailableReason::NotInstalled));
        assert!(status.version.is_none());
        assert!(status.command.contains("--version"));
    }
}
