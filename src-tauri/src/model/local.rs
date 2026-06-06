//! `LocalStubProvider` — the off-by-default local-model slot. The interface is
//! defined so a future router can send routine work to a local endpoint, but v1
//! ships only a "configure me" stub: it emits a single notice and stays inert.

use super::{ModelEvent, ModelProvider, ModelRequest, UnavailableReason};

pub struct LocalStubProvider;

impl ModelProvider for LocalStubProvider {
    fn run(&self, _req: &ModelRequest, sink: &mut dyn FnMut(ModelEvent)) {
        sink(ModelEvent::Unavailable {
            reason: UnavailableReason::Unknown,
            detail: "The local model provider is a configure-me stub in v1. \
                     Switch to Claude in Settings to run real calls."
                .into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_emits_one_terminal_notice() {
        let provider = LocalStubProvider;
        let mut events = Vec::new();
        provider.run(&ModelRequest::planning("hi"), &mut |e| events.push(e));
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ModelEvent::Unavailable { .. }));
        assert!(events[0].is_terminal());
    }
}
