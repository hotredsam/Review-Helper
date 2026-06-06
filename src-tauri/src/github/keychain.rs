//! GitHub token storage in the macOS Keychain (generic password item). The token
//! never leaves the keychain except into in-memory use for an HTTP call or a git
//! askpass helper.

use keyring::{Entry, Error as KeyringError};

const SERVICE: &str = "com.reviewhelper.app";
const ACCOUNT: &str = "github-token";

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE, ACCOUNT).map_err(|e| e.to_string())
}

pub fn save_token(token: &str) -> Result<(), String> {
    entry()?.set_password(token).map_err(|e| e.to_string())
}

/// Returns the token, or `None` if not signed in (the keychain has no entry).
pub fn get_token() -> Result<Option<String>, String> {
    match entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Sign out. Idempotent: deleting a missing entry is not an error.
pub fn delete_token() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "writes to the real macOS Keychain (may prompt); run: cargo test -- --ignored"]
    fn save_get_delete_roundtrip() {
        // Use a throwaway entry so we never touch the real github-token item.
        let e = Entry::new("com.reviewhelper.app.test", "roundtrip").unwrap();
        e.set_password("test-token-xyz").unwrap();
        assert_eq!(e.get_password().unwrap(), "test-token-xyz"); // survives a fresh read
        e.delete_credential().unwrap();
        assert!(matches!(e.get_password(), Err(KeyringError::NoEntry))); // sign-out clears it
    }
}
