//! A scripted provider for tests and offline UI work: it emits a fixed sequence
//! of events without touching any subprocess.

use super::{ModelEvent, ModelProvider, ModelRequest};

pub struct FakeProvider {
    script: Vec<ModelEvent>,
}

impl FakeProvider {
    pub fn new(script: Vec<ModelEvent>) -> Self {
        FakeProvider { script }
    }

    /// A canned successful streaming reply.
    pub fn echo() -> Self {
        FakeProvider::new(vec![
            ModelEvent::Started {
                session_id: Some("fake-session".into()),
                model: Some("fake".into()),
            },
            ModelEvent::AssistantText {
                text: "Thinking… ".into(),
            },
            ModelEvent::AssistantText {
                text: "done.".into(),
            },
            ModelEvent::Completed {
                session_id: Some("fake-session".into()),
                text: "Thinking… done.".into(),
            },
        ])
    }
}

impl ModelProvider for FakeProvider {
    fn run(&self, _req: &ModelRequest, sink: &mut dyn FnMut(ModelEvent)) {
        for event in &self.script {
            sink(event.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{UnavailableReason, READ_ONLY_TOOLS};

    fn collect(provider: &dyn ModelProvider, req: &ModelRequest) -> Vec<ModelEvent> {
        let mut events = Vec::new();
        provider.run(req, &mut |e| events.push(e));
        events
    }

    #[test]
    fn streams_ordered_events_ending_in_one_terminal() {
        let provider = FakeProvider::echo();
        let events = collect(&provider, &ModelRequest::planning("hello"));

        assert!(matches!(events.first().unwrap(), ModelEvent::Started { .. }));
        assert!(events.last().unwrap().is_terminal());
        // exactly one terminal event, and it is last
        assert_eq!(events.iter().filter(|e| e.is_terminal()).count(), 1);

        // assistant text arrives in order and reassembles the full answer
        let text: String = events
            .iter()
            .filter_map(|e| match e {
                ModelEvent::AssistantText { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, "Thinking… done.");
    }

    #[test]
    fn planning_request_uses_the_read_only_allow_list() {
        let req = ModelRequest::planning("x");
        assert_eq!(req.allowed_tools, READ_ONLY_TOOLS.to_vec());
    }

    #[test]
    fn a_scripted_unavailable_is_terminal() {
        let provider = FakeProvider::new(vec![ModelEvent::Unavailable {
            reason: UnavailableReason::NotInstalled,
            detail: "claude not found".into(),
        }]);
        let events = collect(&provider, &ModelRequest::planning("x"));
        assert_eq!(events.len(), 1);
        assert!(events[0].is_terminal());
    }
}
