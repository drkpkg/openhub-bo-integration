use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

/// Field-level error as reported by OpenHub in `errors[]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiFieldError {
    #[serde(default)]
    pub field: Option<String>,
    pub message: String,
    #[serde(default)]
    pub code: Option<String>,
}

/// Every failure the core can report. Serialized with a `kind` tag so bindings
/// can map it to idiomatic exception types.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Error {
    /// Input rejected locally, before reaching OpenHub.
    #[error("invalid `{field}`: {message}")]
    Validation { field: String, message: String },

    /// Credentials or access token rejected (HTTP 401/403). Hosts should drop
    /// the cached token and retry once.
    #[error("authentication failed (HTTP {status}): {message}")]
    Authentication { status: u16, message: String },

    /// OpenHub answered with an error. Build it with [`Error::api`] or
    /// [`Error::business`] so `retryable` is set consistently.
    #[error("OpenHub error (HTTP {status}): {message}")]
    Api {
        status: u16,
        message: String,
        errors: Vec<ApiFieldError>,
        /// Whether repeating the call later may succeed.
        retryable: bool,
    },

    /// OpenHub could not confirm whether a non-idempotent operation (e.g. a
    /// payout) was applied. Query its status before trying again.
    #[error("outcome unknown: {message}")]
    Ambiguous {
        message: String,
        errors: Vec<ApiFieldError>,
    },

    /// The response could not be understood.
    #[error("unexpected response: {message}")]
    Decode { message: String },

    /// Incoming webhook did not carry the expected auth header.
    #[error("webhook rejected: {message}")]
    WebhookAuth { message: String },

    /// Unknown operation or malformed payload on the FFI boundary.
    #[error("invalid FFI call: {message}")]
    Ffi { message: String },
}

impl Error {
    /// Whether repeating the same call later may succeed.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Api {
                retryable: true,
                ..
            }
        )
    }

    /// An error classified by HTTP status alone: gateway/backend failures
    /// (5xx), timeouts (408) and throttling (429) are retryable.
    pub fn api(status: u16, message: impl Into<String>, errors: Vec<ApiFieldError>) -> Self {
        Error::Api {
            status,
            message: message.into(),
            errors,
            retryable: status >= 500 || matches!(status, 408 | 429),
        }
    }

    /// A structured rejection from OpenHub's backend. Never retryable, even
    /// when sent with HTTP 5xx (some FX endpoints answer "not found" with 500).
    pub fn business(status: u16, message: impl Into<String>, errors: Vec<ApiFieldError>) -> Self {
        Error::Api {
            status,
            message: message.into(),
            errors,
            retryable: false,
        }
    }

    pub fn validation(field: &str, message: impl Into<String>) -> Self {
        Error::Validation {
            field: field.to_owned(),
            message: message.into(),
        }
    }

    pub fn decode(message: impl Into<String>) -> Self {
        Error::Decode {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_classification() {
        let api = |status| Error::api(status, "", Vec::new());
        assert!(api(502).is_retryable());
        assert!(api(429).is_retryable());
        assert!(!api(400).is_retryable());
        assert!(!api(409).is_retryable());
        assert!(!Error::business(500, "not found", Vec::new()).is_retryable());
        assert!(!Error::validation("x", "y").is_retryable());
    }
}
