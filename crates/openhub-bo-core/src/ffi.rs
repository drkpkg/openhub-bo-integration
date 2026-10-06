//! Language-neutral JSON boundary used by every binding.
//!
//! Each binding exposes one function, `call(op, payload) -> String`, backed by
//! a [`Registry`]. The result is always an envelope:
//! `{"ok": true, "value": ...}` or `{"ok": false, "error": {"kind", ..., "retryable"}}`.
//! It never panics across the FFI.
//!
//! Op names:
//! - `<operation>.build` with `{config, token?, input}` → `HttpRequest`
//! - `<operation>.parse` with `{input, response}` → the operation's output
//! - `<handler>` with the handler's input
//! - `version`, `describe` (lists registered operations)

use std::panic::{AssertUnwindSafe, catch_unwind};

use serde::Deserialize;
use serde::de::DeserializeOwned;
pub use serde_json::Value as JsonValue;
use serde_json::json;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::HttpResponse;
use crate::operation::{Ctx, Handler, Operation, Registry};
use crate::token::AccessToken;

/// Bump when the payload/result contract changes incompatibly.
pub const PROTOCOL_VERSION: u32 = 2;

pub fn call(registry: &dyn Registry, op: &str, payload: &str) -> String {
    let outcome = catch_unwind(AssertUnwindSafe(|| dispatch(registry, op, payload)))
        .unwrap_or_else(|_| Err(ffi_error(format!("core panicked while handling `{op}`"))));
    let envelope = match outcome {
        Ok(value) => json!({ "ok": true, "value": value }),
        Err(error) => {
            let mut detail =
                serde_json::to_value(&error).unwrap_or_else(|_| json!({ "kind": "ffi" }));
            detail["retryable"] = error.is_retryable().into();
            json!({ "ok": false, "error": detail })
        }
    };
    envelope.to_string()
}

fn dispatch(registry: &dyn Registry, op: &str, payload: &str) -> Result<JsonValue> {
    match op {
        "version" => Ok(json!({
            "core": env!("CARGO_PKG_VERSION"),
            "protocol": PROTOCOL_VERSION,
        })),
        "describe" => to_value(registry.operations()),
        _ => registry
            .dispatch(op, payload)
            .unwrap_or_else(|| Err(ffi_error(format!("unknown operation `{op}`")))),
    }
}

#[derive(Deserialize)]
struct Build<I> {
    config: Config,
    #[serde(default)]
    token: Option<AccessToken>,
    input: I,
}

#[derive(Deserialize)]
struct Parse<I> {
    input: I,
    response: HttpResponse,
}

/// Runs one stage of an operation. Used by [`registry!`](crate::registry).
pub fn run<O: Operation>(stage: &str, payload: &str) -> Result<JsonValue> {
    match stage {
        "build" => {
            let p: Build<O::Input> = parse(payload)?;
            to_value(O::request(
                &Ctx::new(&p.config, p.token.as_ref()),
                &p.input,
            )?)
        }
        "parse" => {
            let p: Parse<O::Input> = parse(payload)?;
            to_value(O::response(&p.response, &p.input)?)
        }
        other => Err(ffi_error(format!(
            "unknown stage `{other}` for `{}`",
            O::NAME
        ))),
    }
}

/// Runs a handler. Used by [`registry!`](crate::registry).
pub fn run_handler<H: Handler>(payload: &str) -> Result<JsonValue> {
    to_value(H::handle(parse(payload)?)?)
}

fn parse<T: DeserializeOwned>(payload: &str) -> Result<T> {
    serde_json::from_str(payload).map_err(|e| ffi_error(format!("invalid payload: {e}")))
}

fn to_value<T: serde::Serialize>(value: T) -> Result<JsonValue> {
    serde_json::to_value(value).map_err(|e| ffi_error(e.to_string()))
}

fn ffi_error(message: String) -> Error {
    Error::Ffi { message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CoreOps;

    fn run_op(op: &str, payload: JsonValue) -> JsonValue {
        serde_json::from_str(&call(&CoreOps, op, &payload.to_string())).unwrap()
    }

    #[test]
    fn round_trips_token_flow() {
        let config = json!({"client_id": "id", "client_secret": "s", "environment": "production"});
        let request = run_op(
            "token.build",
            json!({ "config": config, "input": {"now": 0} }),
        );
        assert_eq!(request["ok"], true);
        assert!(
            request["value"]["url"]
                .as_str()
                .unwrap()
                .starts_with("https://api.redenlace.com.bo/")
        );

        let body = include_str!("../../../fixtures/openhub/token_response.json");
        let token = run_op(
            "token.parse",
            json!({ "input": {"now": 0}, "response": {"status": 201, "body": body} }),
        );
        assert_eq!(token["value"]["expires_at"], 3540);
    }

    #[test]
    fn errors_are_tagged_and_classified() {
        let missing = run_op(
            "token.build",
            json!({ "config": {"client_id": "id"}, "input": {"now": 0} }),
        );
        assert_eq!(missing["error"]["kind"], "validation");
        assert_eq!(missing["error"]["retryable"], false);

        let gateway = run_op(
            "token.parse",
            json!({ "input": {"now": 0}, "response": {"status": 503, "body": "down"} }),
        );
        assert_eq!(gateway["error"]["kind"], "authentication");
    }

    #[test]
    fn unknown_op_stage_and_bad_json_do_not_panic() {
        for (op, payload) in [
            ("nope", "{}"),
            ("token.nope", "{}"),
            ("token.parse", "not json"),
        ] {
            let result: JsonValue = serde_json::from_str(&call(&CoreOps, op, payload)).unwrap();
            assert_eq!(result["error"]["kind"], "ffi", "{op}");
        }
    }

    #[test]
    fn describe_lists_operations() {
        let described = run_op("describe", json!({}));
        assert_eq!(
            described["value"],
            json!([{"kind": "operation", "name": "token", "idempotent": true}])
        );
    }
}
