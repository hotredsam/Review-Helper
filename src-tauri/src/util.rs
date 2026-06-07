//! Small shared helpers.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// Run `work` on a background thread; if it panics, call `on_panic` (typically
/// to emit a `Failed` event) so the UI never hangs forever on a silently-dead
/// thread. Enforces CLAUDE.md's "nothing happens silently" for spawned work.
pub fn spawn_guarded<F, P>(work: F, on_panic: P)
where
    F: FnOnce() + Send + 'static,
    P: FnOnce() + Send + 'static,
{
    std::thread::spawn(move || {
        if catch_unwind(AssertUnwindSafe(work)).is_err() {
            on_panic();
        }
    });
}
