//! Parse the model's JSON plan output into a typed `GeneratedPlan`. Robust to
//! stray prose / code fences via a string-aware brace-balancing extractor; a
//! missing required field is a hard error (we never fabricate or default it).

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct GeneratedPlan {
    pub current_state: String,
    pub body_md: String,
    pub confidence: String,
    pub notes: String,
    pub phases: Vec<GenPhase>,
    pub decisions: Vec<GenDecision>,
    pub stack: GenStack,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct GenPhase {
    pub title: String,
    pub goal: String,
    pub tasks: Vec<GenTask>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct GenTask {
    pub title: String,
    pub body: String,
    pub verification: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct GenDecision {
    pub topic: String,
    pub choice: String,
    pub rationale: String,
    pub alternatives: String,
    pub consequences: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct GenStack {
    pub frontend: Option<String>,
    pub backend: Option<String>,
    pub database: Option<String>,
    pub deployment: Option<String>,
    pub pipes: Option<String>,
}

/// Parse the model output into a plan, or a descriptive error.
pub fn parse_plan(raw: &str) -> Result<GeneratedPlan, String> {
    let json = extract_json(raw).ok_or("No JSON object found in the model output.")?;
    serde_json::from_str::<GeneratedPlan>(json)
        .map_err(|e| format!("The model's plan JSON did not match the expected shape: {e}"))
}

/// Find the first balanced `{ … }` object, ignoring braces inside string
/// literals (so `{`/`}` in body_md or rationale don't confuse it). Returns the
/// substring, or None if no balanced object exists.
fn extract_json(raw: &str) -> Option<&str> {
    let s = raw.trim();
    let bytes = s.as_bytes();
    let start = s.find('{')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escaped = false;
    for i in start..bytes.len() {
        let c = bytes[i];
        if in_str {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_str = false;
            }
        } else {
            match c {
                b'"' => in_str = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&s[start..=i]);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{
      "current_state": "An early scaffold.",
      "body_md": "Plan body with braces { like this } in prose.",
      "confidence": "low",
      "notes": "Mostly from the README.",
      "phases": [
        {"title": "Setup", "goal": "Runnable", "tasks": [
          {"title": "Init", "body": "do it", "verification": "it runs"}
        ]}
      ],
      "decisions": [
        {"topic": "DB", "choice": "SQLite", "rationale": "simple", "alternatives": "pg", "consequences": "local only"}
      ],
      "stack": {"frontend": null, "backend": "Rust", "database": "SQLite", "deployment": null, "pipes": null}
    }"#;

    #[test]
    fn parses_a_valid_plan_with_braces_in_strings() {
        let plan = parse_plan(VALID).unwrap();
        assert_eq!(plan.confidence, "low");
        assert_eq!(plan.phases.len(), 1);
        assert_eq!(plan.phases[0].tasks[0].verification, "it runs");
        assert_eq!(plan.decisions[0].choice, "SQLite");
        assert_eq!(plan.stack.backend.as_deref(), Some("Rust"));
        assert!(plan.stack.frontend.is_none());
        assert!(plan.body_md.contains("{ like this }"));
    }

    #[test]
    fn extracts_from_fenced_and_prefixed_output() {
        let wrapped = format!("Here is the plan:\n```json\n{VALID}\n```\nDone.");
        let plan = parse_plan(&wrapped).unwrap();
        assert_eq!(plan.phases.len(), 1);
    }

    #[test]
    fn rejects_missing_required_fields() {
        let bad = r#"{"current_state": "x", "phases": []}"#;
        assert!(parse_plan(bad).is_err());
    }

    #[test]
    fn rejects_non_json() {
        assert!(parse_plan("the model refused to answer").is_err());
        assert!(parse_plan("{ unbalanced").is_err());
    }
}
