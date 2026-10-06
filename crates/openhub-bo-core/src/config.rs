use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// OpenHub environments as published in the docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    Development,
    /// Called "certificación" in the docs.
    #[default]
    Sandbox,
    Production,
}

impl Environment {
    pub fn base_url(self) -> &'static str {
        match self {
            Environment::Development => "https://atcgwapitest.redenlace.com.bo/desarrollo",
            Environment::Sandbox => "https://atcgwapitest.redenlace.com.bo/sandbox",
            Environment::Production => "https://api.redenlace.com.bo",
        }
    }
}

/// Integrator credentials, provisioned by ATC.
///
/// ATC may hand out either `client_id` + `client_secret` or a ready-made
/// "Token Basic"; both are supported. `client_id` is always required because
/// it travels as a header on every API call.
#[derive(Clone, Deserialize)]
pub struct Config {
    pub client_id: String,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub basic_token: Option<String>,
    #[serde(default)]
    pub environment: Environment,
    /// Overrides the environment URL (mock servers, proxies).
    #[serde(default)]
    pub base_url: Option<String>,
}

impl Config {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: Some(client_secret.into()),
            basic_token: None,
            environment: Environment::default(),
            base_url: None,
        }
    }

    pub fn with_environment(mut self, environment: Environment) -> Self {
        self.environment = environment;
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.client_id.trim().is_empty() {
            return Err(Error::validation("client_id", "must not be empty"));
        }
        if self.client_secret.is_none() && self.basic_token.is_none() {
            return Err(Error::validation(
                "client_secret",
                "either client_secret or basic_token is required",
            ));
        }
        Ok(())
    }

    pub fn url(&self, path: &str) -> String {
        let base = self
            .base_url
            .as_deref()
            .unwrap_or_else(|| self.environment.base_url());
        format!(
            "{}/{}",
            base.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    pub(crate) fn basic_authorization(&self) -> Result<String> {
        self.validate()?;
        if let Some(token) = &self.basic_token {
            let token = token.trim();
            let token = token.strip_prefix("Basic ").unwrap_or(token);
            return Ok(format!("Basic {token}"));
        }
        let secret = self.client_secret.as_deref().unwrap_or_default();
        let encoded = STANDARD.encode(format!("{}:{}", self.client_id, secret));
        Ok(format!("Basic {encoded}"))
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("client_id", &self.client_id)
            .field("client_secret", &self.client_secret.as_ref().map(|_| "***"))
            .field("basic_token", &self.basic_token.as_ref().map(|_| "***"))
            .field("environment", &self.environment)
            .field("base_url", &self.base_url)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_basic_header_from_secret() {
        let config = Config::new("id", "secret");
        assert_eq!(config.basic_authorization().unwrap(), "Basic aWQ6c2VjcmV0");
    }

    #[test]
    fn accepts_ready_made_basic_token() {
        let mut config = Config::new("id", "unused");
        config.client_secret = None;
        config.basic_token = Some("Basic abc==".into());
        assert_eq!(config.basic_authorization().unwrap(), "Basic abc==");
    }

    #[test]
    fn joins_urls_without_double_slashes() {
        let config = Config::new("id", "s").with_base_url("http://localhost:8080/");
        assert_eq!(config.url("/qr/x"), "http://localhost:8080/qr/x");
    }

    #[test]
    fn debug_redacts_secrets() {
        let rendered = format!("{:?}", Config::new("id", "top-secret"));
        assert!(!rendered.contains("top-secret"));
    }
}
