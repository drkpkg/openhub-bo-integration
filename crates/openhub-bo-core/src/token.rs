use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::envelope::message_from_body;
use crate::error::{Error, Result};
use crate::http::{HttpRequest, HttpResponse, Method};
use crate::operation::{Ctx, Operation};

/// Tokens are treated as expired this many seconds early to absorb clock
/// skew and request latency.
pub const EXPIRY_SKEW_SECS: i64 = 60;

const TOKEN_PATH: &str = "oauth-client-credentials/access-token?grant_type=client_credentials";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessToken {
    pub access_token: String,
    pub token_type: String,
    /// Unix timestamp (seconds) after which the token must be refreshed.
    pub expires_at: i64,
    pub scope: Option<String>,
}

impl AccessToken {
    pub fn is_valid(&self, now: i64) -> bool {
        now < self.expires_at
    }

    pub(crate) fn authorization(&self) -> String {
        format!("Bearer {}", self.access_token)
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessToken")
            .field("access_token", &"***")
            .field("token_type", &self.token_type)
            .field("expires_at", &self.expires_at)
            .field("scope", &self.scope)
            .finish()
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default = "default_token_type")]
    token_type: String,
    #[serde(default = "default_expires_in")]
    expires_in: i64,
    #[serde(default)]
    scope: Option<String>,
}

fn default_token_type() -> String {
    "Bearer".to_owned()
}

fn default_expires_in() -> i64 {
    3600
}

/// OAuth 2.0 client-credentials request.
fn build_token_request(config: &Config) -> Result<HttpRequest> {
    let headers = BTreeMap::from([
        ("Authorization".to_owned(), config.basic_authorization()?),
        (
            "Content-Type".to_owned(),
            "application/x-www-form-urlencoded".to_owned(),
        ),
        ("Accept".to_owned(), "application/json".to_owned()),
    ]);
    Ok(HttpRequest {
        method: Method::Post,
        url: config.url(TOKEN_PATH),
        headers,
        body: None,
    })
}

fn parse_token_response(response: &HttpResponse, now: i64) -> Result<AccessToken> {
    if !response.is_success() {
        return Err(Error::Authentication {
            status: response.status,
            message: message_from_body(&response.body),
        });
    }
    let parsed: TokenResponse = serde_json::from_str(&response.body)
        .map_err(|e| Error::decode(format!("invalid token response: {e}")))?;
    if parsed.access_token.is_empty() {
        return Err(Error::decode("token response has an empty access_token"));
    }
    let lifetime = (parsed.expires_in - EXPIRY_SKEW_SECS).max(0);
    Ok(AccessToken {
        access_token: parsed.access_token,
        token_type: parsed.token_type,
        expires_at: now + lifetime,
        scope: parsed.scope,
    })
}

/// OAuth 2.0 client credentials. Sandbox answers HTTP 201 with
/// `token_type: "access_token"` and no `scope`.
pub struct Token;

#[derive(Debug, Clone, Deserialize)]
pub struct TokenInput {
    /// Current Unix time in seconds, supplied by the host (the core has no clock).
    pub now: i64,
}

impl Operation for Token {
    const NAME: &'static str = "token";
    const IDEMPOTENT: bool = true;

    type Input = TokenInput;
    type Output = AccessToken;

    fn request(ctx: &Ctx<'_>, _input: &TokenInput) -> Result<HttpRequest> {
        build_token_request(ctx.config)
    }

    fn response(response: &HttpResponse, input: &TokenInput) -> Result<AccessToken> {
        parse_token_response(response, input.now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../fixtures/openhub/token_response.json");

    #[test]
    fn builds_client_credentials_request() {
        let request = build_token_request(&Config::new("id", "secret")).unwrap();
        assert_eq!(request.method, Method::Post);
        assert_eq!(
            request.url,
            "https://atcgwapitest.redenlace.com.bo/sandbox/oauth-client-credentials/access-token?grant_type=client_credentials"
        );
        assert_eq!(request.headers["Authorization"], "Basic aWQ6c2VjcmV0");
    }

    #[test]
    fn parses_token_with_skew() {
        let token = parse_token_response(&HttpResponse::new(200, FIXTURE), 1_000).unwrap();
        assert_eq!(token.expires_at, 1_000 + 3600 - EXPIRY_SKEW_SECS);
        assert_eq!(token.scope.as_deref(), Some("api.qr"));
        assert!(token.is_valid(1_000));
        assert!(!token.is_valid(token.expires_at));
    }

    #[test]
    fn rejected_credentials_are_authentication_errors() {
        let body = r#"{"error":"invalid_client","error_description":"Bad credentials"}"#;
        let err = parse_token_response(&HttpResponse::new(401, body), 0).unwrap_err();
        assert_eq!(
            err,
            Error::Authentication {
                status: 401,
                message: "Bad credentials".into()
            }
        );
    }
}
