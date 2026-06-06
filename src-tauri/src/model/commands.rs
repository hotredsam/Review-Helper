//! Tauri commands for the model layer. The frontend calls `model_run` and listens
//! for `model-event`s; it never spawns `claude` itself. Routing (Claude vs. the
//! local stub) is read from the persisted settings on every call.

use tauri::{AppHandle, Emitter, State};

use super::claude::ClaudeCodeProvider;
use super::local::LocalStubProvider;
use super::{ModelEvent, ModelProvider, ModelRequest};
use crate::db::Db;
use crate::settings::{load_model_config, ModelConfig, ProviderKind};

/// The single place the active provider is chosen, from the persisted config.
fn provider_for(config: &ModelConfig) -> Box<dyn ModelProvider> {
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
        // The stub yields exactly one terminal "unavailable" notice.
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ModelEvent::Unavailable { .. }));
    }
}
