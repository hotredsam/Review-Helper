//! GitHub OAuth device flow. Built and unit-tested; it becomes the active sign-in
//! path once a `github.client_id` is configured. The token is captured here and
//! handed to the keychain by the command layer — it is never returned to the UI.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::http_client;

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// One poll result. `Authorized` carries the token (kept internal — never sent
/// to the frontend).
#[derive(Debug, Clone, PartialEq)]
pub enum PollOutcome {
    Pending,
    SlowDown,
    Authorized(String),
    Denied,
    Expired,
    Error(String),
}

pub fn parse_device_code(v: &Value) -> Result<DeviceCode, String> {
    Ok(DeviceCode {
        device_code: v
            .get("device_code")
            .and_then(Value::as_str)
            .ok_or("device-code response missing `device_code`")?
            .to_string(),
        user_code: v
            .get("user_code")
            .and_then(Value::as_str)
            .ok_or("device-code response missing `user_code`")?
            .to_string(),
        verification_uri: v
            .get("verification_uri")
            .and_then(Value::as_str)
            .unwrap_or("https://github.com/login/device")
            .to_string(),
        expires_in: v.get("expires_in").and_then(Value::as_u64).unwrap_or(900),
        interval: v.get("interval").and_then(Value::as_u64).unwrap_or(5),
    })
}

/// Classify a token-endpoint poll response. GitHub returns HTTP 200 with an
/// `error` field for the pending/slow-down/denied/expired cases, so we parse the
/// body, not the status.
pub fn classify_poll(v: &Value) -> PollOutcome {
    if let Some(token) = v.get("access_token").and_then(Value::as_str) {
        return PollOutcome::Authorized(token.to_string());
    }
    match v.get("error").and_then(Value::as_str) {
        Some("authorization_pending") => PollOutcome::Pending,
        Some("slow_down") => PollOutcome::SlowDown,
        Some("access_denied") => PollOutcome::Denied,
        Some("expired_token") | Some("incorrect_device_code") => PollOutcome::Expired,
        Some(other) => PollOutcome::Error(other.to_string()),
        None => PollOutcome::Error("unexpected token response".into()),
    }
}

/// Step 1: request the device + user codes.
pub fn request_device_code(client_id: &str) -> Result<DeviceCode, String> {
    let resp = http_client()?
        .post(DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&[("client_id", client_id), ("scope", "repo")])
        .send()
        .map_err(|e| e.to_string())?;
    let v: Value = resp.json().map_err(|e| e.to_string())?;
    parse_device_code(&v)
}

/// Step 2: poll once for the access token.
pub fn poll_token(client_id: &str, device_code: &str) -> PollOutcome {
    let send = http_client().and_then(|c| {
        c.post(TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", client_id),
                ("device_code", device_code),
                ("grant_type", GRANT_TYPE),
            ])
            .send()
            .map_err(|e| e.to_string())
    });
    match send.and_then(|r| r.json::<Value>().map_err(|e| e.to_string())) {
        Ok(v) => classify_poll(&v),
        Err(e) => PollOutcome::Error(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_a_device_code_response() {
        let v = json!({
            "device_code": "abc123",
            "user_code": "WDJB-MJHT",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 5
        });
        let code = parse_device_code(&v).unwrap();
        assert_eq!(code.user_code, "WDJB-MJHT");
        assert_eq!(code.interval, 5);
    }

    #[test]
    fn missing_fields_error() {
        assert!(parse_device_code(&json!({"user_code": "X"})).is_err());
    }

    #[test]
    fn classifies_all_poll_outcomes() {
        assert_eq!(
            classify_poll(&json!({"access_token": "gho_x", "token_type": "bearer"})),
            PollOutcome::Authorized("gho_x".into())
        );
        assert_eq!(classify_poll(&json!({"error": "authorization_pending"})), PollOutcome::Pending);
        assert_eq!(classify_poll(&json!({"error": "slow_down"})), PollOutcome::SlowDown);
        assert_eq!(classify_poll(&json!({"error": "access_denied"})), PollOutcome::Denied);
        assert_eq!(classify_poll(&json!({"error": "expired_token"})), PollOutcome::Expired);
        assert!(matches!(
            classify_poll(&json!({"error": "device_flow_disabled"})),
            PollOutcome::Error(_)
        ));
    }
}
