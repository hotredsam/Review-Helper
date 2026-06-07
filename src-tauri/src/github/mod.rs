//! GitHub integration: device-flow auth (built + tested, optional), a gh-token
//! import path, the OS-keychain token store, the REST client, and the
//! shallow-clone cache.
//!
//! Security: the token lives only in the macOS Keychain — never logged, never
//! written to disk in cleartext, never embedded in a clone URL or git config.
//! The model only ever reads the clone directory, never the token.

pub mod api;
pub mod clone;
pub mod commands;
pub mod device;
pub mod keychain;

/// Shared blocking HTTP client. GitHub requires a User-Agent header.
///
/// Timeouts are mandatory: without them a hung GitHub call would block its
/// thread forever, and any call made under the DB lock would freeze the whole
/// app. 10s to connect, 30s overall caps the worst case.
pub(crate) fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent("review-helper")
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())
}
