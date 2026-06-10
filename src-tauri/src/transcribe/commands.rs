//! Mic commands. The frontend listens for `transcribe-event` (state changes,
//! download progress, streaming partials, the final text) and drives the
//! session with start/stop/cancel. Real local Whisper — the Phase 10 stub is
//! gone.

use tauri::AppHandle;

/// Begin recording (downloading/loading the model first if needed). Async so
/// the model load never sits on the main thread.
#[tauri::command]
pub async fn transcribe_start(app: AppHandle) -> Result<(), String> {
    super::start(app)
}

/// Stop recording and return the final, clean transcript.
#[tauri::command]
pub async fn transcribe_stop() -> Result<String, String> {
    super::stop()
}

/// Discard the recording without transcribing.
#[tauri::command]
pub async fn transcribe_cancel() -> Result<(), String> {
    super::cancel()
}
