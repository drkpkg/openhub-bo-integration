//! One type per endpoint ([`Operation`]) or pure function ([`Handler`]), and
//! a [`registry!`](crate::registry) macro that exposes them over the FFI.
//!
//! Adding an endpoint means writing one `Operation` impl and listing it in a
//! registry; bindings pick it up through `call(op, payload)` without changes.

use std::collections::BTreeMap;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::ffi::JsonValue;
use crate::http::{HttpRequest, HttpResponse};
use crate::token::AccessToken;

/// What an operation needs to build its request.
#[derive(Debug, Clone, Copy)]
pub struct Ctx<'a> {
    pub config: &'a Config,
    pub token: Option<&'a AccessToken>,
}

impl<'a> Ctx<'a> {
    pub fn new(config: &'a Config, token: Option<&'a AccessToken>) -> Self {
        Self { config, token }
    }

    pub fn token(&self) -> Result<&'a AccessToken> {
        self.token.ok_or_else(|| {
            Error::validation("token", "an access token is required for this operation")
        })
    }

    /// Headers for business calls. The gateway (Sensedia) reads the token from
    /// `access_token` (verified in the sandbox: `Authorization: Bearer` alone
    /// gets a 401); the Bearer header is what the docs' tables describe, so
    /// both are sent.
    /// [`Ctx::api_headers`] plus product-specific headers (e.g. `branchCode`).
    pub fn api_headers_with(&self, extra: &[(&str, &str)]) -> Result<BTreeMap<String, String>> {
        let mut headers = self.api_headers()?;
        headers.extend(
            extra
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned())),
        );
        Ok(headers)
    }

    pub fn api_headers(&self) -> Result<BTreeMap<String, String>> {
        let token = self.token()?;
        Ok(BTreeMap::from([
            ("Authorization".to_owned(), token.authorization()),
            ("access_token".to_owned(), token.access_token.clone()),
            ("client_id".to_owned(), self.config.client_id.clone()),
            ("Content-Type".to_owned(), "application/json".to_owned()),
            ("Accept".to_owned(), "application/json".to_owned()),
        ]))
    }
}

/// An HTTP endpoint: builds the request and parses the response.
pub trait Operation {
    /// FFI name, e.g. `qr.generate`. Stages are `<NAME>.build` / `<NAME>.parse`.
    const NAME: &'static str;
    /// Safe to repeat. When false, a lost response leaves the outcome unknown
    /// (e.g. a QR may have been created).
    const IDEMPOTENT: bool;
    /// Client timeout recommended by ATC for this endpoint, if any.
    const TIMEOUT_SECS: Option<u32> = None;

    type Input: DeserializeOwned;
    type Output: Serialize;

    fn request(ctx: &Ctx<'_>, input: &Self::Input) -> Result<HttpRequest>;
    fn response(response: &HttpResponse, input: &Self::Input) -> Result<Self::Output>;
}

/// A pure function without HTTP, e.g. webhook parsing.
pub trait Handler {
    const NAME: &'static str;

    type Input: DeserializeOwned;
    type Output: Serialize;

    fn handle(input: Self::Input) -> Result<Self::Output>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpInfo {
    Operation {
        name: &'static str,
        idempotent: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout_secs: Option<u32>,
    },
    Handler {
        name: &'static str,
    },
}

impl OpInfo {
    pub fn operation<O: Operation>() -> Self {
        OpInfo::Operation {
            name: O::NAME,
            idempotent: O::IDEMPOTENT,
            timeout_secs: O::TIMEOUT_SECS,
        }
    }

    pub fn handler<H: Handler>() -> Self {
        OpInfo::Handler { name: H::NAME }
    }
}

/// A set of operations exposed over the FFI. Implement it with
/// [`registry!`](crate::registry).
pub trait Registry {
    fn dispatch(&self, op: &str, payload: &str) -> Option<Result<JsonValue>>;
    fn operations(&self) -> Vec<OpInfo>;
}

/// Two registries exposed as one; `a` wins on name clashes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Chain<A, B>(pub A, pub B);

impl<A: Registry, B: Registry> Registry for Chain<A, B> {
    fn dispatch(&self, op: &str, payload: &str) -> Option<Result<JsonValue>> {
        self.0
            .dispatch(op, payload)
            .or_else(|| self.1.dispatch(op, payload))
    }

    fn operations(&self) -> Vec<OpInfo> {
        let mut ops = self.0.operations();
        ops.extend(self.1.operations());
        ops
    }
}

/// Declares a unit struct implementing [`Registry`]:
///
/// ```ignore
/// openhub_bo_core::registry!(pub OpenHub {
///     operations: [Token, qr::Generate, qr::Verify],
///     handlers: [qr::ParseWebhook],
/// });
/// ```
#[macro_export]
macro_rules! registry {
    ($vis:vis $name:ident {
        operations: [$($op:ty),* $(,)?],
        handlers: [$($handler:ty),* $(,)?] $(,)?
    }) => {
        #[derive(Debug, Clone, Copy, Default)]
        $vis struct $name;

        impl $crate::Registry for $name {
            fn dispatch(
                &self,
                op: &str,
                payload: &str,
            ) -> ::core::option::Option<$crate::Result<$crate::ffi::JsonValue>> {
                $(
                    if let ::core::option::Option::Some(stage) = op
                        .strip_prefix(<$op as $crate::Operation>::NAME)
                        .and_then(|rest| rest.strip_prefix('.'))
                    {
                        return ::core::option::Option::Some($crate::ffi::run::<$op>(stage, payload));
                    }
                )*
                $(
                    if op == <$handler as $crate::Handler>::NAME {
                        return ::core::option::Option::Some($crate::ffi::run_handler::<$handler>(payload));
                    }
                )*
                ::core::option::Option::None
            }

            fn operations(&self) -> ::std::vec::Vec<$crate::OpInfo> {
                ::std::vec![
                    $($crate::OpInfo::operation::<$op>(),)*
                    $($crate::OpInfo::handler::<$handler>(),)*
                ]
            }
        }
    };
}
