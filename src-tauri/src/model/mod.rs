//! The model-provider layer. ALL model use in the app goes through the
//! `ModelProvider` trait — there are no ad-hoc `claude` calls elsewhere.
//!
//! Planning/analysis calls are read-only against source: a request carries only
//! the read/search tools in `READ_ONLY_TOOLS` (never Bash/Edit/Write), so the
//! model can never mutate the user's repo. The app performs every write itself.

use serde::{Deserialize, Serialize};

pub mod claude;
pub mod commands;
pub mod fake;
pub mod local;

/// Read/search tools a planning call may use. Deliberately excludes
/// Bash/Edit/Write/NotebookEdit — the model never mutates source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tool {
    Read,
    Grep,
    Glob,
    WebSearch,
    WebFetch,
}

impl Tool {
    /// The exact tool name the Claude Code CLI expects in `--allowedTools`.
    pub fn cli_name(self) -> &'static str {
        match self {
            Tool::Read => "Read",
            Tool::Grep => "Grep",
            Tool::Glob => "Glob",
            Tool::WebSearch => "WebSearch",
            Tool::WebFetch => "WebFetch",
        }
    }
}

/// The read-only allow-list every planning/analysis call uses.
pub const READ_ONLY_TOOLS: [Tool; 5] = [
    Tool::Read,
    Tool::Grep,
    Tool::Glob,
    Tool::WebSearch,
    Tool::WebFetch,
];

/// One model invocation. `session_id` resumes a prior session (multi-turn chat);
/// `cwd` is the only directory the model may read (e.g. the shallow clone cache).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequest {
    pub prompt: String,
    pub system_append: Option<String>,
    pub allowed_tools: Vec<Tool>,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    /// Optional model alias (e.g. "haiku", "sonnet", "opus"). `None` = CLI default.
    pub model: Option<String>,
}

impl ModelRequest {
    /// A read-only planning request with the standard allow-list.
    pub fn planning(prompt: impl Into<String>) -> Self {
        ModelRequest {
            prompt: prompt.into(),
            system_append: None,
            allowed_tools: READ_ONLY_TOOLS.to_vec(),
            session_id: None,
            cwd: None,
            model: None,
        }
    }
}

/// Why the provider couldn't run — drives the "Claude not available" UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableReason {
    NotInstalled,
    NotAuthenticated,
    CreditExhausted,
    Unknown,
}

/// Streamed events from a model run, in order, ending in exactly one terminal
/// event (`Completed`, `Unavailable`, or `Failed`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModelEvent {
    /// The run started; `session_id` can be used to resume it later.
    Started {
        session_id: Option<String>,
        model: Option<String>,
    },
    /// A chunk of assistant text to append to the panel.
    AssistantText { text: String },
    /// The model invoked a (read-only) tool.
    ToolUse { name: String },
    /// Non-fatal information (e.g. a tool was denied, or a stream line was skipped).
    Notice { message: String },
    /// Terminal: the run finished successfully. `text` is the full answer.
    Completed {
        session_id: Option<String>,
        text: String,
    },
    /// Terminal: the provider couldn't run at all. The app stays read-only and
    /// the UI shows the "Claude not available" banner.
    Unavailable {
        reason: UnavailableReason,
        detail: String,
    },
    /// Terminal: the run started but errored mid-stream.
    Failed { detail: String },
}

impl ModelEvent {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ModelEvent::Completed { .. }
                | ModelEvent::Unavailable { .. }
                | ModelEvent::Failed { .. }
        )
    }
}

/// The single entry point for all model use. Implementations stream
/// `ModelEvent`s to `sink` in order and return once a terminal event has been
/// emitted. `run` blocks; callers spawn it on a background thread and forward
/// each event to the frontend.
pub trait ModelProvider: Send + Sync {
    fn run(&self, req: &ModelRequest, sink: &mut dyn FnMut(ModelEvent));
}
