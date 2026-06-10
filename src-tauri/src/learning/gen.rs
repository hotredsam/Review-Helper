//! Shared model plumbing for Learning mode. One synchronous round-trip through
//! the read-only planning provider (same as the Understand-hub generators), plus
//! JSON extraction. Generation commands are serialized per subject by
//! `LearningGate`, so a blocking call here is safe and shows a spinner in the UI.

use crate::model::{CancelToken, ModelEvent, ModelProvider, ModelRequest};

/// Run one model turn (read-only tools) and return its final text, or a clear
/// error on the offline / unavailable / empty paths.
pub(super) fn run_once(provider: &dyn ModelProvider, prompt: String, system: &str, cancel: &CancelToken) -> Result<String, String> {
    let mut req = ModelRequest::planning(prompt);
    req.system_append = Some(system.to_string());
    let mut text = None;
    let mut failure: Option<String> = None;
    provider.run(&req, cancel, &mut |e: ModelEvent| match e {
        ModelEvent::Completed { text: t, .. } => text = Some(t),
        ModelEvent::Unavailable { detail, .. } | ModelEvent::Failed { detail } => failure = Some(detail),
        ModelEvent::Stopped => failure = Some("Stopped.".into()),
        _ => {}
    });
    if let Some(d) = failure {
        return Err(d);
    }
    text.ok_or_else(|| "The model produced no result.".into())
}

/// Pull the first JSON object/array out of a model reply (handles ```json fences
/// and surrounding prose). Thin wrapper over the plan parser used everywhere.
pub(super) fn extract_json(text: &str) -> Result<&str, String> {
    crate::plan::parse::extract_json(text).ok_or_else(|| "The model returned no usable JSON.".into())
}
