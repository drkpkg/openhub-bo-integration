//! Webhook pieces shared by every product family. ATC does not sign payloads:
//! it echoes the static header configured in [`WebhookTarget`]. Payload
//! parsing is family-specific and lives with each family.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

use crate::error::{Error, Result};
use crate::validate::require_text;

/// Where ATC must POST notifications, and the static header it will send so
/// the integrator can authenticate the call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookTarget {
    pub url: String,
    /// Header name, e.g. `x-api-key`.
    pub key: String,
    /// Header value; treat it as a secret.
    pub value: String,
}

impl WebhookTarget {
    pub fn validate(&self) -> Result<()> {
        let url = self.url.trim();
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Err(Error::validation("webhook.url", "must be an http(s) URL"));
        }
        require_text("webhook.key", &self.key)?;
        if !self
            .key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::validation(
                "webhook.key",
                "must be a valid HTTP header name",
            ));
        }
        require_text("webhook.value", &self.value)
    }
}

/// An incoming webhook call as received by the integrator, plus the header
/// name/value configured when the payment was created. Input of every
/// family's webhook handler.
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookCall {
    pub headers: BTreeMap<String, String>,
    pub body: String,
    /// Header name and value sent as `webhook.key` / `webhook.value`.
    pub key: String,
    pub value: String,
}

impl WebhookCall {
    pub fn authenticate(&self) -> Result<()> {
        authenticate(&self.headers, &self.key, &self.value)
    }
}

/// Checks the auth header of an incoming webhook: name case-insensitive,
/// value in constant time.
pub fn authenticate(headers: &BTreeMap<String, String>, key: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(Error::validation(
            "value",
            "expected webhook header value must not be empty",
        ));
    }
    let received = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
        .ok_or_else(|| Error::WebhookAuth {
            message: format!("missing `{key}` header"),
        })?;
    verify_secret(&format!("`{key}` header"), received, value)
}

/// Constant-time comparison of a received secret (header, query token...).
pub fn verify_secret(what: &str, received: &str, expected: &str) -> Result<()> {
    if expected.is_empty() {
        return Err(Error::validation(
            "value",
            "expected secret must not be empty",
        ));
    }
    if !bool::from(received.as_bytes().ct_eq(expected.as_bytes())) {
        return Err(Error::WebhookAuth {
            message: format!("{what} does not match"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(name: &str, value: &str) -> BTreeMap<String, String> {
        BTreeMap::from([(name.to_owned(), value.to_owned())])
    }

    #[test]
    fn authenticates_case_insensitively() {
        assert!(authenticate(&headers("X-API-KEY", "s3cret"), "x-api-key", "s3cret").is_ok());
    }

    #[test]
    fn rejects_wrong_or_missing_header() {
        for h in [headers("x-api-key", "nope"), headers("other", "s3cret")] {
            assert!(matches!(
                authenticate(&h, "x-api-key", "s3cret"),
                Err(Error::WebhookAuth { .. })
            ));
        }
    }

    #[test]
    fn validates_target() {
        let target = |url: &str, key: &str| WebhookTarget {
            url: url.into(),
            key: key.into(),
            value: "v".into(),
        };
        assert!(target("https://a.bo/h", "x-api-key").validate().is_ok());
        assert!(target("ftp://a.bo", "x-api-key").validate().is_err());
        assert!(target("https://a.bo", "bad key").validate().is_err());
    }
}
