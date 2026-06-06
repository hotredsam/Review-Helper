//! `ClaudeCodeProvider` — the v1 adapter. Spawns the Claude Code CLI in headless
//! mode (`claude -p`) with stream-json output, parses the JSONL event stream, and
//! forwards each line as a `ModelEvent`. The allow-list + disallow-list keep every
//! call strictly read-only against the user's source.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

use serde_json::Value;

use super::{ModelEvent, ModelProvider, ModelRequest, UnavailableReason};

/// Tools that must never be available to the model — defense in depth alongside
/// the read-only allow-list (which already omits them).
const DISALLOWED: &str = "Bash,Edit,Write,MultiEdit,NotebookEdit,Task";

pub struct ClaudeCodeProvider {
    binary: String,
}

impl Default for ClaudeCodeProvider {
    fn default() -> Self {
        ClaudeCodeProvider {
            binary: "claude".into(),
        }
    }
}

impl ClaudeCodeProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the binary path (used by tests to point at a stub script).
    pub fn with_binary(binary: impl Into<String>) -> Self {
        ClaudeCodeProvider {
            binary: binary.into(),
        }
    }

    fn command(&self, req: &ModelRequest) -> Command {
        let mut cmd = Command::new(&self.binary);
        cmd.arg("-p")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose") // required for stream-json
            .arg("--include-partial-messages") // token-level deltas
            .arg("--disallowedTools")
            .arg(DISALLOWED);

        let allow = req
            .allowed_tools
            .iter()
            .map(|t| t.cli_name())
            .collect::<Vec<_>>()
            .join(",");
        if !allow.is_empty() {
            cmd.arg("--allowedTools").arg(allow);
        }
        if let Some(system) = &req.system_append {
            cmd.arg("--append-system-prompt").arg(system);
        }
        if let Some(session) = &req.session_id {
            cmd.arg("--resume").arg(session);
        }
        if let Some(model) = &req.model {
            cmd.arg("--model").arg(model);
        }
        if let Some(dir) = &req.cwd {
            cmd.arg("--add-dir").arg(dir).current_dir(dir);
        }
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd
    }
}

impl ModelProvider for ClaudeCodeProvider {
    fn run(&self, req: &ModelRequest, sink: &mut dyn FnMut(ModelEvent)) {
        let mut child = match self.command(req).spawn() {
            Ok(child) => child,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                sink(ModelEvent::Unavailable {
                    reason: UnavailableReason::NotInstalled,
                    detail: format!("`{}` was not found on PATH.", self.binary),
                });
                return;
            }
            Err(e) => {
                sink(ModelEvent::Unavailable {
                    reason: UnavailableReason::Unknown,
                    detail: e.to_string(),
                });
                return;
            }
        };

        // Send the prompt over stdin and close it so the model starts.
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(req.prompt.as_bytes());
        }

        let mut saw_terminal = false;
        if let Some(stdout) = child.stdout.take() {
            for line in BufReader::new(stdout).lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => continue,
                };
                if line.trim().is_empty() {
                    continue;
                }
                if let Some(event) = parse_line(&line) {
                    saw_terminal |= event.is_terminal();
                    sink(event);
                }
            }
        }

        // No terminal event means the process failed before producing a result
        // (bad auth, bad flag, crash). Surface stderr as an Unavailable event so
        // the app stays read-only and the UI can explain why.
        if !saw_terminal {
            let mut stderr = String::new();
            if let Some(mut handle) = child.stderr.take() {
                let _ = handle.read_to_string(&mut stderr);
            }
            let status = child.wait().ok();
            let detail = if stderr.trim().is_empty() {
                format!(
                    "claude exited without a result (status {:?}).",
                    status.and_then(|s| s.code())
                )
            } else {
                stderr.trim().to_string()
            };
            sink(ModelEvent::Unavailable {
                reason: classify_stderr(&detail),
                detail,
            });
        } else {
            let _ = child.wait();
        }
    }
}

/// Probe whether the CLI is installed/runnable. Returns the version string, or an
/// `Unavailable` event describing why not.
pub fn check_available(binary: &str) -> Result<String, ModelEvent> {
    match Command::new(binary).arg("--version").output() {
        Ok(out) if out.status.success() => {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        }
        Ok(out) => Err(ModelEvent::Unavailable {
            reason: UnavailableReason::Unknown,
            detail: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(ModelEvent::Unavailable {
            reason: UnavailableReason::NotInstalled,
            detail: format!("`{binary}` was not found on PATH."),
        }),
        Err(e) => Err(ModelEvent::Unavailable {
            reason: UnavailableReason::Unknown,
            detail: e.to_string(),
        }),
    }
}

/// Parse one JSONL line of `stream-json` output into at most one `ModelEvent`.
fn parse_line(line: &str) -> Option<ModelEvent> {
    let v: Value = serde_json::from_str(line).ok()?;
    match v.get("type")?.as_str()? {
        "system" if v.get("subtype").and_then(Value::as_str) == Some("init") => {
            Some(ModelEvent::Started {
                session_id: v.get("session_id").and_then(Value::as_str).map(String::from),
                model: v.get("model").and_then(Value::as_str).map(String::from),
            })
        }
        "stream_event" => {
            let event = v.get("event")?;
            if event.get("type").and_then(Value::as_str) == Some("content_block_delta") {
                let delta = event.get("delta")?;
                if delta.get("type").and_then(Value::as_str) == Some("text_delta") {
                    return Some(ModelEvent::AssistantText {
                        text: delta.get("text")?.as_str()?.to_string(),
                    });
                }
            }
            None
        }
        "assistant" => {
            // Text already streamed via stream_event deltas; surface tool use only.
            let content = v.get("message")?.get("content")?.as_array()?;
            for block in content {
                if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                    let name = block.get("name").and_then(Value::as_str).unwrap_or("tool");
                    return Some(ModelEvent::ToolUse {
                        name: name.to_string(),
                    });
                }
            }
            None
        }
        "result" => Some(classify_result(&v)),
        _ => None,
    }
}

fn classify_result(v: &Value) -> ModelEvent {
    let session_id = v.get("session_id").and_then(Value::as_str).map(String::from);
    let is_error = v.get("is_error").and_then(Value::as_bool).unwrap_or(false);
    let success = v.get("subtype").and_then(Value::as_str) == Some("success");
    if success && !is_error {
        return ModelEvent::Completed {
            session_id,
            text: v
                .get("result")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        };
    }
    let detail = v
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or("the model returned an error")
        .to_string();
    match v.get("api_error_status").and_then(Value::as_i64) {
        Some(402) | Some(429) => ModelEvent::Unavailable {
            reason: UnavailableReason::CreditExhausted,
            detail,
        },
        _ => ModelEvent::Failed { detail },
    }
}

pub(crate) fn classify_stderr(detail: &str) -> UnavailableReason {
    let d = detail.to_lowercase();
    if d.contains("not found") || d.contains("no such file") {
        UnavailableReason::NotInstalled
    } else if d.contains("auth") || d.contains("log in") || d.contains("login") || d.contains("unauthorized") {
        UnavailableReason::NotAuthenticated
    } else if d.contains("credit") || d.contains("quota") || d.contains("rate limit") {
        UnavailableReason::CreditExhausted
    } else {
        UnavailableReason::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_init_into_started() {
        let line = r#"{"type":"system","subtype":"init","session_id":"abc","model":"claude-opus-4-8"}"#;
        match parse_line(line).unwrap() {
            ModelEvent::Started { session_id, model } => {
                assert_eq!(session_id.as_deref(), Some("abc"));
                assert_eq!(model.as_deref(), Some("claude-opus-4-8"));
            }
            other => panic!("expected Started, got {other:?}"),
        }
    }

    #[test]
    fn parses_text_delta_into_assistant_text() {
        let line = r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hel"}}}"#;
        assert_eq!(
            parse_line(line).unwrap(),
            ModelEvent::AssistantText { text: "Hel".into() }
        );
    }

    #[test]
    fn parses_tool_use_from_assistant() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read"}]}}"#;
        assert_eq!(
            parse_line(line).unwrap(),
            ModelEvent::ToolUse { name: "Read".into() }
        );
    }

    #[test]
    fn ignores_plain_assistant_text_block() {
        // text already streamed via deltas; an assistant text block yields nothing
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hi"}]}}"#;
        assert!(parse_line(line).is_none());
    }

    #[test]
    fn parses_success_result_into_completed() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"result":"ping","session_id":"s1"}"#;
        assert_eq!(
            parse_line(line).unwrap(),
            ModelEvent::Completed {
                session_id: Some("s1".into()),
                text: "ping".into()
            }
        );
    }

    #[test]
    fn classifies_credit_exhaustion() {
        let line = r#"{"type":"result","subtype":"error","is_error":true,"api_error_status":402,"result":"payment required"}"#;
        match parse_line(line).unwrap() {
            ModelEvent::Unavailable { reason, .. } => {
                assert_eq!(reason, UnavailableReason::CreditExhausted)
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn classifies_generic_error_as_failed() {
        let line = r#"{"type":"result","subtype":"error","is_error":true,"result":"boom"}"#;
        assert_eq!(
            parse_line(line).unwrap(),
            ModelEvent::Failed { detail: "boom".into() }
        );
    }

    #[test]
    fn skips_unknown_and_malformed_lines() {
        assert!(parse_line("not json").is_none());
        assert!(parse_line(r#"{"type":"rate_limit_event"}"#).is_none());
    }

    #[test]
    fn classify_stderr_detects_reasons() {
        assert_eq!(classify_stderr("command not found"), UnavailableReason::NotInstalled);
        assert_eq!(
            classify_stderr("Authentication failed, please log in"),
            UnavailableReason::NotAuthenticated
        );
        assert_eq!(classify_stderr("you are rate limited"), UnavailableReason::CreditExhausted);
        assert_eq!(classify_stderr("weird thing"), UnavailableReason::Unknown);
    }

    #[test]
    fn not_installed_binary_yields_unavailable() {
        let provider = ClaudeCodeProvider::with_binary("definitely-not-a-real-binary-xyz");
        let mut events = Vec::new();
        provider.run(&ModelRequest::planning("hi"), &mut |e| events.push(e));
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ModelEvent::Unavailable {
                reason: UnavailableReason::NotInstalled,
                ..
            }
        ));
    }

    #[test]
    #[ignore = "spawns the real `claude` CLI; needs auth + network and uses credits. Run with: cargo test -- --ignored"]
    fn real_claude_streams_and_resumes() {
        let provider = ClaudeCodeProvider::new();
        let mut req = ModelRequest::planning("Reply with exactly one word: ping");
        req.model = Some("haiku".into());

        let mut events = Vec::new();
        provider.run(&req, &mut |e| events.push(e));

        assert!(
            matches!(events.first(), Some(ModelEvent::Started { .. })),
            "first event should be Started: {events:?}"
        );
        assert!(
            events.iter().any(|e| matches!(e, ModelEvent::AssistantText { .. })),
            "should stream some text: {events:?}"
        );
        let session = match events.last() {
            Some(ModelEvent::Completed { session_id, .. }) => session_id.clone(),
            other => panic!("expected Completed, got {other:?}"),
        };
        assert!(session.is_some(), "completed run should carry a session id");

        // Second turn resumes the same session.
        let mut req2 = ModelRequest::planning("Now reply with exactly one word: pong");
        req2.model = Some("haiku".into());
        req2.session_id = session;
        let mut events2 = Vec::new();
        provider.run(&req2, &mut |e| events2.push(e));
        assert!(
            matches!(events2.last(), Some(ModelEvent::Completed { .. })),
            "resumed turn should complete: {events2:?}"
        );
    }
}
