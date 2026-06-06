//! Two-way chat — a grounded conversation on the model adapter. Each turn
//! injects the ProjectContext bundle (so the model references current project
//! state) and resumes the prior session (multi-turn). Inferred updates are
//! emitted as tagged blocks and parsed into pending suggestions (T2).

pub mod commands;

use crate::suggestions::{is_valid_kind, ParsedSuggestion};

pub const CHAT_SYSTEM: &str = r#"You are Review Helper's project companion. You help the builder think through what they're building. Be concrete, honest, and grounded in the PROJECT CONTEXT below and the repository in your working directory (which you may read, READ-ONLY). Never edit, write, or delete files, and never run shell commands. Answer conversationally and concisely; reference the real plan, decisions, stack, and answered questions when relevant.

If — and ONLY if — the conversation clearly implies a concrete update to the project record, emit it as a tagged block at the VERY END of your reply, after your prose. Never invent updates the user didn't imply; if none apply, emit no blocks. One block per update, exact delimiters, a valid JSON object inside, no nesting:

<<<RH:SUGGESTION kind=decision>>>
{"topic":"...","choice":"...","rationale":"..."}
<<<RH:END>>>

Valid kinds and payloads:
- decision: {"topic","choice","rationale"}
- feature:  {"title","detail"}
- stack:    {"pane","choice"}   (pane: frontend|backend|database|deployment|pipes)
- answer:   {"question","answer"}

These become PENDING suggestions the user approves — nothing changes the record on its own. Your prose must read naturally without the blocks."#;

const OPEN: &str = "<<<RH:SUGGESTION";
const END: &str = "<<<RH:END>>>";

/// Split a model reply into the visible prose and any tagged suggestion blocks.
/// Robust by design: unterminated blocks are dropped, and blocks with an
/// unknown kind or a non-object JSON body are skipped — never crash, never
/// invent. Returns (visible_reply, parsed_suggestions).
pub fn parse_suggestions(text: &str) -> (String, Vec<ParsedSuggestion>) {
    let mut visible = String::new();
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(OPEN) {
        visible.push_str(&rest[..start]);
        let after = &rest[start..];
        let header_end = match after.find(">>>") {
            Some(i) => i + 3,
            None => {
                // malformed open tag — keep it as visible text and stop.
                visible.push_str(after);
                rest = "";
                break;
            }
        };
        let header = &after[..header_end];
        let tail = &after[header_end..];
        let end_idx = match tail.find(END) {
            Some(i) => i,
            None => {
                rest = ""; // unterminated block — drop the remainder.
                break;
            }
        };
        let body = tail[..end_idx].trim();
        if let Some(kind) = parse_kind(header) {
            let body_is_object = serde_json::from_str::<serde_json::Value>(body)
                .map(|v| v.is_object())
                .unwrap_or(false);
            if is_valid_kind(&kind) && body_is_object {
                out.push(ParsedSuggestion { kind, payload: body.to_string() });
            }
        }
        rest = &tail[end_idx + END.len()..];
    }
    visible.push_str(rest);
    (visible.trim().to_string(), out)
}

fn parse_kind(header: &str) -> Option<String> {
    let i = header.find("kind=")?;
    let after = &header[i + 5..];
    let end = after.find(|c: char| c == '>' || c.is_whitespace()).unwrap_or(after.len());
    let kind = after[..end].trim().trim_matches('"');
    if kind.is_empty() {
        None
    } else {
        Some(kind.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_decision_and_a_feature_and_strips_the_blocks() {
        let text = "Sounds like SQLite fits, and a CSV export would help.\n\n\
<<<RH:SUGGESTION kind=decision>>>\n{\"topic\":\"Database\",\"choice\":\"SQLite\",\"rationale\":\"local, simple\"}\n<<<RH:END>>>\n\
<<<RH:SUGGESTION kind=feature>>>\n{\"title\":\"Export to CSV\",\"detail\":\"download the table\"}\n<<<RH:END>>>";
        let (visible, parsed) = parse_suggestions(text);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].kind, "decision");
        assert_eq!(parsed[1].kind, "feature");
        assert!(visible.starts_with("Sounds like SQLite fits"));
        assert!(!visible.contains("RH:SUGGESTION"), "blocks stripped from visible reply");
    }

    #[test]
    fn no_blocks_yields_no_suggestions() {
        let (visible, parsed) = parse_suggestions("Just a normal answer, nothing to record.");
        assert!(parsed.is_empty());
        assert_eq!(visible, "Just a normal answer, nothing to record.");
    }

    #[test]
    fn malformed_or_unknown_blocks_are_skipped_not_invented() {
        // unknown kind, invalid-JSON body, and an unterminated block: all dropped.
        let text = "Reply.\n\
<<<RH:SUGGESTION kind=bogus>>>\n{\"x\":1}\n<<<RH:END>>>\n\
<<<RH:SUGGESTION kind=decision>>>\nnot json\n<<<RH:END>>>\n\
<<<RH:SUGGESTION kind=feature>>>\n{\"title\":\"unterminated\"}";
        let (_visible, parsed) = parse_suggestions(text);
        assert!(parsed.is_empty(), "no valid blocks => no suggestions");
    }
}
