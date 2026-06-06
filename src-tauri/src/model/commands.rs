//! Tauri commands for the model layer. The frontend calls `model_run` and listens
//! for `model-event`s; it never spawns `claude` itself.

use tauri::{AppHandle, Emitter};

use super::claude::ClaudeCodeProvider;
use super::{ModelEvent, ModelProvider, ModelRequest};

/// The single place the active provider is chosen. Phase 2 / T3 will route this
/// from settings (Claude vs. the local stub); for now everything uses Claude Code.
fn current_provider() -> Box<dyn ModelProvider> {
    Box::new(ClaudeCodeProvider::new())
}

/// Run a read-only planning request on a background thread, emitting each
/// `ModelEvent` to the frontend as `model-event`. Returns immediately.
#[tauri::command]
pub fn model_run(app: AppHandle, prompt: String, session_id: Option<String>) {
    std::thread::spawn(move || {
        let provider = current_provider();
        let mut req = ModelRequest::planning(prompt);
        req.session_id = session_id;
        provider.run(&req, &mut |event: ModelEvent| {
            let _ = app.emit("model-event", &event);
        });
    });
}
