//! Assessment engine: a deterministic scan (skills/big-picture/scan.py, embedded)
//! plus model scoring of the six vibecoding dimensions, production readiness, top
//! fixes, and a hygiene cleanup list — grounded in the scan's numbers.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;

use crate::plan::parse::extract_json;

pub mod commands;

const SCAN_PY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../skills/big-picture/scan.py"
));

pub const ASSESS_SYSTEM: &str = r#"You are Review Helper's assessor. You judge how well an existing project is set up — grounded in the SCAN FACTS provided (deterministic measurements) plus your own READ-ONLY exploration of the repository in your working directory. Never edit, write, move, or delete files, and never run shell commands.

Score honestly on a 0–100 scale (100 = best). Every score needs a one-line reason tied to a scan number or to something you actually observed — never a vibe. A low score with a clear reason beats a generous one. Don't invent problems or strengths the repo doesn't show.

SIX VIBECODING DIMENSIONS:
- architecture: a real, intended structure (core vs shared vs disposable), concerns separated, not accreted.
- modularity: small, single-responsibility files. The scan's files_over_300_lines flags files over 300 (hard over 500); duplicate utilities lower this.
- context_hygiene: code scoped so an agent can work one task; presence of CLAUDE.md / planning docs (scan: has_claude_md, has_planning_dir).
- security: hardcoded keys/credentials, client-side secret exposure, missing auth on data access. ANY secret_pattern_hits caps this dimension low. Weight heavily; never soften.
- git_discipline: meaningful history, no giant untracked state (scan: git_commits).
- workflow: a written plan the code follows; does current state match it.

PRODUCTION READINESS (0–100 each): tests (scan: has_tests), error_handling, secrets (scan: secret_pattern_hits), build_ci (scan: has_ci), dependencies, docs (scan: has_readme).

Then give top_fixes (the 3 highest-leverage fixes, each naming a file or pattern, not "improve architecture") and hygiene (cleanup items: oversized files from the scan, dead code, unused deps — empty array if clean).

OUTPUT: Emit ONLY this JSON object — nothing before or after, no ``` fences. First character `{`, last `}`:
{
  "dimensions": {
    "architecture": {"score": int, "reason": string},
    "modularity": {"score": int, "reason": string},
    "context_hygiene": {"score": int, "reason": string},
    "security": {"score": int, "reason": string},
    "git_discipline": {"score": int, "reason": string},
    "workflow": {"score": int, "reason": string}
  },
  "dimensions_overall": int,
  "production": {
    "tests": {"score": int, "reason": string},
    "error_handling": {"score": int, "reason": string},
    "secrets": {"score": int, "reason": string},
    "build_ci": {"score": int, "reason": string},
    "dependencies": {"score": int, "reason": string},
    "docs": {"score": int, "reason": string}
  },
  "production_overall": int,
  "top_fixes": [string, string, string],
  "hygiene": [string]
}
Every key must be present. This is parsed deterministically; stray text breaks it."#;

pub fn assess_user(scan_facts: &str) -> String {
    format!(
        "SCAN FACTS (deterministic measurements of the repo):\n\n{}\n\nExplore the repo read-only to judge what the scanner can't, then emit the assessment JSON per your instructions.",
        scan_facts.trim()
    )
}

/// Run the embedded deterministic scanner against a repo dir, returning its JSON.
pub fn run_scan(repo_dir: &str) -> Result<String, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new("python3")
        .arg("-")
        .arg(repo_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "python3 was not found — it's needed for the repo scan.".to_string()
            } else {
                e.to_string()
            }
        })?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(SCAN_PY.as_bytes());
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "scan failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Parse + lightly validate the model's assessment JSON.
pub fn parse_assessment(raw: &str) -> Result<Value, String> {
    let json = extract_json(raw).ok_or("No JSON object found in the assessment output.")?;
    let v: Value = serde_json::from_str(json).map_err(|e| format!("Assessment JSON invalid: {e}"))?;
    let ok = v.get("dimensions").map(Value::is_object).unwrap_or(false)
        && v.get("production").map(Value::is_object).unwrap_or(false)
        && v.get("top_fixes").map(Value::is_array).unwrap_or(false)
        && v.get("hygiene").map(Value::is_array).unwrap_or(false);
    if !ok {
        return Err("Assessment JSON missing dimensions/production/top_fixes/hygiene.".into());
    }
    Ok(v)
}

fn clamp_overall(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(Value::as_i64).unwrap_or(0).clamp(0, 100)
}

/// Persist an assessment, tied to the project's latest plan version.
pub fn save_assessment(conn: &Connection, project_id: i64, a: &Value) -> Result<(), String> {
    let plan_version: Option<i64> = conn
        .query_row(
            "SELECT MAX(version) FROM plans WHERE project_id = ?1",
            [project_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();

    let dimensions = a.get("dimensions").cloned().unwrap_or(Value::Null).to_string();
    let production = serde_json::json!({
        "scores": a.get("production").cloned().unwrap_or(Value::Null),
        "overall": clamp_overall(a, "production_overall"),
    })
    .to_string();
    let top_fixes = a
        .get("top_fixes")
        .cloned()
        .unwrap_or_else(|| Value::Array(vec![]))
        .to_string();
    let hygiene = a
        .get("hygiene")
        .cloned()
        .unwrap_or_else(|| Value::Array(vec![]))
        .to_string();

    conn.execute(
        "INSERT INTO assessments (project_id, plan_version, dimension_scores, overall, production_readiness, hygiene, top_fixes) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![project_id, plan_version, dimensions, clamp_overall(a, "dimensions_overall"), production, hygiene, top_fixes],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct AssessmentView {
    pub overall: i64,
    pub dimensions: Value,
    pub production: Value,
    pub top_fixes: Value,
    pub hygiene: Value,
    pub created_at: String,
}

/// The latest assessment for a project, or None.
pub fn get_assessment(conn: &Connection, project_id: i64) -> Result<Option<AssessmentView>, String> {
    let row = conn
        .query_row(
            "SELECT overall, dimension_scores, production_readiness, top_fixes, hygiene, created_at \
             FROM assessments WHERE project_id = ?1 ORDER BY id DESC LIMIT 1",
            [project_id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let (overall, dims, prod, fixes, hyg, created_at) = match row {
        Some(r) => r,
        None => return Ok(None),
    };
    let p = |s: Option<String>| s.and_then(|x| serde_json::from_str(&x).ok()).unwrap_or(Value::Null);
    Ok(Some(AssessmentView {
        overall,
        dimensions: p(dims),
        production: p(prod),
        top_fixes: p(fixes),
        hygiene: p(hyg),
        created_at,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_connection;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn
    }

    const SAMPLE: &str = r#"{
      "dimensions": {
        "architecture": {"score": 70, "reason": "clear module split"},
        "modularity": {"score": 80, "reason": "no files over 300 lines"},
        "context_hygiene": {"score": 90, "reason": "CLAUDE.md present"},
        "security": {"score": 60, "reason": "no secret hits but no auth yet"},
        "git_discipline": {"score": 75, "reason": "20 commits"},
        "workflow": {"score": 85, "reason": "plan matches code"}
      },
      "dimensions_overall": 110,
      "production": {
        "tests": {"score": 80, "reason": "has tests"},
        "error_handling": {"score": 65, "reason": "ok"},
        "secrets": {"score": 90, "reason": "keychain"},
        "build_ci": {"score": 40, "reason": "no CI"},
        "dependencies": {"score": 70, "reason": "few deps"},
        "docs": {"score": 85, "reason": "README"}
      },
      "production_overall": 71,
      "top_fixes": ["Add CI", "Cover error paths", "Document setup"],
      "hygiene": ["No obvious cruft"]
    }"#;

    #[test]
    fn parses_and_persists_an_assessment_clamping_overall() {
        let conn = db();
        conn.execute("INSERT INTO projects (name, kind) VALUES ('D','new')", []).unwrap();
        let pid = conn.last_insert_rowid();

        let a = parse_assessment(SAMPLE).unwrap();
        save_assessment(&conn, pid, &a).unwrap();

        let view = get_assessment(&conn, pid).unwrap().unwrap();
        assert_eq!(view.overall, 100, "dimensions_overall 110 clamps to 100");
        assert_eq!(view.dimensions["security"]["score"], 60);
        assert_eq!(view.production["overall"], 71);
        assert_eq!(view.top_fixes.as_array().unwrap().len(), 3);
    }

    #[test]
    fn rejects_malformed_assessment() {
        assert!(parse_assessment("not json").is_err());
        assert!(parse_assessment(r#"{"dimensions": {}}"#).is_err());
    }

    #[test]
    fn run_scan_emits_expected_keys() {
        // scan a tiny temp repo (no model).
        let dir = std::env::temp_dir().join(format!("rh-scan-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "# demo").unwrap();
        std::fs::write(dir.join("main.rs"), "fn main() {}\n").unwrap();

        let facts = run_scan(dir.to_str().unwrap()).unwrap();
        let v: Value = serde_json::from_str(&facts).unwrap();
        assert_eq!(v["has_readme"], true);
        assert_eq!(v["source_files"], 1);
        assert!(v.get("files_over_300_lines").unwrap().is_array());

        std::fs::remove_dir_all(&dir).ok();
    }
}
