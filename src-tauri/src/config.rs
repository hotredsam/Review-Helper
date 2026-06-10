//! Centralized tunables. Every timeout lives here (externalize-what-changes):
//! call sites never hardcode their own numbers.

/// Hard ceiling on one model call. A hung `claude` child is killed at this
/// deadline and surfaces as a clean failure instead of bricking the feature.
pub const MODEL_TIMEOUT_SECS: u64 = 300;

/// Hard ceiling on one git network operation (clone/fetch). Stalled networks
/// error out instead of freezing the app.
pub const NETWORK_TIMEOUT_SECS: u64 = 120;
