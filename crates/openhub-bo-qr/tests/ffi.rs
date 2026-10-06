//! The QR operations over the FFI, combined with the core registry the way
//! bindings do it.

use std::collections::BTreeSet;

use openhub_bo_core::Chain;
use openhub_bo_core::CoreOps;
use openhub_bo_core::ffi::{JsonValue, call};
use openhub_bo_qr::QrOps;
use serde_json::json;

const OPS: Chain<CoreOps, QrOps> = Chain(CoreOps, QrOps);

fn run_op(op: &str, payload: JsonValue) -> JsonValue {
    serde_json::from_str(&call(&OPS, op, &payload.to_string())).unwrap()
}

#[test]
fn amounts_cross_as_strings() {
    let body = include_str!("../../../fixtures/openhub/generate_response.json");
    let input = json!({"description": "x", "amount": "10.5", "reference": "1", "expires_in": 1,
        "establishment_id": 1, "establishment_name": "x"});
    let qr = run_op(
        "qr.generate.parse",
        json!({ "input": input, "response": {"status": 200, "body": body} }),
    );
    assert_eq!(qr["value"]["amount"], "10.5");
    assert_eq!(qr["value"]["status"], "pending");
}

#[test]
fn errors_are_tagged_and_classified() {
    let result = run_op(
        "qr.verify.build",
        json!({
            "config": {"client_id": "id", "client_secret": "s"},
            "token": {"access_token": "t", "token_type": "Bearer", "expires_at": 0, "scope": null},
            "input": {"reference": "a/b"}
        }),
    );
    assert_eq!(result["ok"], false);
    assert_eq!(result["error"]["kind"], "validation");
    assert_eq!(result["error"]["field"], "reference");
    assert_eq!(result["error"]["retryable"], false);

    let gateway = run_op(
        "qr.verify.parse",
        json!({ "input": {"reference": "1"}, "response": {"status": 502, "body": "Error forwarding call"} }),
    );
    assert_eq!(gateway["error"]["kind"], "api");
    assert_eq!(gateway["error"]["retryable"], true);
}

#[test]
fn chain_dispatches_both_registries_and_describes_unique_names() {
    assert_eq!(
        run_op(
            "token.build",
            json!({"config": {"client_id": "i", "client_secret": "s"}, "input": {"now": 0}})
        )["ok"],
        true
    );

    let described = run_op("describe", json!({}));
    let entries = described["value"].as_array().unwrap();
    let names: BTreeSet<_> = entries
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), entries.len(), "duplicate operation names");
    assert_eq!(
        names,
        BTreeSet::from([
            "token",
            "qr.generate",
            "qr.verify",
            "qr.cancel",
            "qr.webhook.parse"
        ])
    );
    let generate = entries.iter().find(|e| e["name"] == "qr.generate").unwrap();
    assert_eq!(generate["idempotent"], false);
}
