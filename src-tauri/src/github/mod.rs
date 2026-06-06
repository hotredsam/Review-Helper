//! GitHub integration: device-flow auth (built + tested, optional), a gh-token
//! import path, the OS-keychain token store, the REST client, and the
//! shallow-clone cache.
//!
//! Security: the token lives only in the macOS Keychain — never logged, never
//! written to disk in cleartext, never embedded in a clone URL or git config.
//! The model only ever reads the clone directory, never the token.

pub mod api;
pub mod commands;
pub mod device;
pub mod keychain;

/// Shared blocking HTTP client. GitHub requires a User-Agent header.
pub(crate) fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent("review-helper")
        .build()
        .map_err(|e| e.to_string())
}
