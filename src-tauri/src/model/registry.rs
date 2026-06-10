//! Registry of in-flight cancellable model runs, keyed by a stable string
//! ("chat:42", "tutor:7", "learning:13", "plan:3", …). The frontend's Stop and
//! Cancel buttons resolve to one generic `model_stop(run_key)` command; every
//! long-running call site registers here so it can be killed mid-flight.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::CancelToken;

static RUNS: OnceLock<Mutex<HashMap<String, CancelToken>>> = OnceLock::new();

fn runs() -> &'static Mutex<HashMap<String, CancelToken>> {
    RUNS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a run under `key`, returning its token. A prior run under the same
/// key is cancelled — the newest request wins (e.g. re-sending a chat).
pub fn register(key: &str) -> CancelToken {
    let token = CancelToken::new();
    let mut map = runs().lock().unwrap_or_else(|p| p.into_inner());
    if let Some(old) = map.insert(key.to_string(), token.clone()) {
        old.cancel();
    }
    token
}

/// Drop a finished run (no-op if it was already replaced or stopped).
pub fn finish(key: &str) {
    let mut map = runs().lock().unwrap_or_else(|p| p.into_inner());
    map.remove(key);
}

/// Cancel the run under `key`. Returns whether one was found.
pub fn stop(key: &str) -> bool {
    let map = runs().lock().unwrap_or_else(|p| p.into_inner());
    match map.get(key) {
        Some(t) => {
            t.cancel();
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_cancels_a_registered_run() {
        let token = register("test:stop");
        assert!(!token.is_cancelled());
        assert!(stop("test:stop"));
        assert!(token.is_cancelled());
        finish("test:stop");
    }

    #[test]
    fn stop_of_unknown_key_reports_false() {
        assert!(!stop("test:nothing-here"));
    }

    #[test]
    fn re_register_cancels_the_previous_run() {
        let first = register("test:dup");
        let second = register("test:dup");
        assert!(first.is_cancelled(), "old run must be cancelled by the new one");
        assert!(!second.is_cancelled());
        assert!(stop("test:dup"));
        assert!(second.is_cancelled());
        finish("test:dup");
    }

    #[test]
    fn finish_removes_the_key() {
        let _ = register("test:fin");
        finish("test:fin");
        assert!(!stop("test:fin"));
    }
}
