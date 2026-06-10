//! The ONE model call this feature is allowed (per session, gated): a haiku
//! reconcile pass that rewrites the Observations sections of both profile
//! files from the deterministic digest + recent evidence. No tools, fail-closed
//! validation, silent skip when the model is unavailable — Learning mode never
//! blocks on the profile.

use crate::model::commands::provider_for;
use crate::model::{CancelToken, ModelEvent, ModelProvider, ModelRequest};
use crate::settings::load_model_config;

const REFLECT_SYSTEM: &str = r#"You maintain two short "how this user works" profiles from MEASURED evidence. Rewrite both Observations sections (reconcile: keep what's still supported, update what changed, drop what the evidence no longer supports — do not append-only). Rules: every line is one bounded behavioral fact citing its evidence counts, e.g. "- Prefers worked examples first (evidence: 9/11 re-asks followed definition-first explanations)". NEVER personality-type the user (no "visual learner" labels). Each section under 1800 characters. Output ONLY this JSON:
{"learner_observations":"- ...\n- ...","review_observations":"- ...\n- ..."}"#;

#[derive(serde::Deserialize)]
struct Reflection {
    learner_observations: String,
    review_observations: String,
}

/// Run the gated reflection. Returns what happened for the caller's log line.
/// Takes the Db (not a held lock): the model call must never run under the
/// global mutex — the same discipline as every other model path.
pub fn run(db: &crate::db::Db) -> Result<String, String> {
    let (facts, learner_now, review_now, evidence, config) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        if !super::enabled(&conn) {
            return Ok("disabled".into());
        }
        let fresh = super::unreflected_count(&conn);
        if fresh < super::REFLECT_MIN_EVENTS {
            return Ok(format!("skipped ({fresh} new events, need {})", super::REFLECT_MIN_EVENTS));
        }
        // Keep the Facts current regardless of whether the model is reachable.
        let _ = super::write_facts(&conn, super::LEARNER_FILE);
        let _ = super::write_facts(&conn, super::REVIEW_FILE);
        (
            super::facts(&conn),
            super::read_or_create(super::LEARNER_FILE).unwrap_or_default(),
            super::read_or_create(super::REVIEW_FILE).unwrap_or_default(),
            super::evidence_lines(&conn, 10).join("\n"),
            load_model_config(&conn),
        )
    };

    let prompt = format!(
        "Measured digest (deterministic):\n{}\n\nMost recent events (numbers/labels only):\n{}\n\nCurrent learner Observations:\n{}\n\nCurrent review Observations:\n{}",
        serde_json::to_string(&facts).unwrap_or_default(),
        evidence,
        learner_now,
        review_now,
    );

    let mut req = ModelRequest::planning(prompt);
    req.allowed_tools = vec![]; // no tools — pure reconcile
    req.system_append = Some(REFLECT_SYSTEM.to_string());
    req.model = Some("haiku".into());

    let mut text: Option<String> = None;
    let mut failure: Option<String> = None;
    provider_for(&config).run(&req, &CancelToken::new(), &mut |e: ModelEvent| match e {
        ModelEvent::Completed { text: t, .. } => text = Some(t),
        ModelEvent::Unavailable { detail, .. } | ModelEvent::Failed { detail } => failure = Some(detail),
        _ => {}
    });
    let Some(text) = text else {
        // Offline / unavailable: events fold into the next session, Facts stayed fresh.
        return Ok(format!("model unavailable — facts updated only ({})", failure.unwrap_or_default()));
    };

    let json = crate::plan::parse::extract_json(&text).ok_or("Reflection returned no JSON.")?;
    let parsed: Reflection = serde_json::from_str(json).map_err(|_| "Reflection JSON malformed — keeping previous observations.")?;
    super::write_observations(super::LEARNER_FILE, &parsed.learner_observations)?;
    super::write_observations(super::REVIEW_FILE, &parsed.review_observations)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    super::mark_reflected(&conn);
    Ok("reflected".into())
}

#[cfg(test)]
mod tests {
    use crate::db::{init_connection, Db};
    use rusqlite::Connection;
    use std::sync::Mutex;

    fn setup_db() -> Db {
        let dir = std::env::temp_dir().join(format!("rh-reflect-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        crate::profile::init(dir);
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        // Route to the Local stub: a reflection must NEVER spend real credits in tests.
        crate::settings::save_model_config(
            &conn,
            &crate::settings::ModelConfig {
                provider: crate::settings::ProviderKind::Local,
                local_endpoint: None,
                api_credit_overflow: false,
            },
        )
        .unwrap();
        Db(Mutex::new(conn))
    }

    #[test]
    fn reflection_is_gated_below_the_event_threshold() {
        let db = setup_db();
        {
            let conn = db.0.lock().unwrap();
            for _ in 0..(crate::profile::REFLECT_MIN_EVENTS - 1) {
                crate::profile::record(&conn, "quiz_answer", Some(1), None, &serde_json::json!({"correct": true}));
            }
        }
        let out = super::run(&db).unwrap();
        assert!(out.starts_with("skipped"), "got: {out}");
    }

    #[test]
    fn reflection_degrades_to_facts_only_when_the_model_is_unavailable() {
        let db = setup_db();
        {
            let conn = db.0.lock().unwrap();
            for _ in 0..crate::profile::REFLECT_MIN_EVENTS {
                crate::profile::record(&conn, "quiz_answer", Some(1), None, &serde_json::json!({"correct": true}));
            }
        }
        // Local stub answers "unavailable" — exactly the offline path: no
        // observation rewrite, facts still refreshed, events fold forward.
        let out = super::run(&db).unwrap();
        assert!(out.starts_with("model unavailable"), "got: {out}");
        let conn = db.0.lock().unwrap();
        assert!(crate::profile::unreflected_count(&conn) >= crate::profile::REFLECT_MIN_EVENTS, "events not consumed on a failed reflection");
    }
}
