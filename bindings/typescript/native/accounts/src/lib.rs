//! WASM module of `@openhub-bo/accounts`. All logic lives in the Rust crates;
//! this only exposes the JSON entry point (see `openhub_bo_core::ffi`).

use openhub_bo_accounts::AccountsOps;
use wasm_bindgen::prelude::*;

/// Runs an operation; `payload` and the result are JSON strings.
#[wasm_bindgen]
pub fn call(op: &str, payload: &str) -> String {
    openhub_bo_core::ffi::call(&AccountsOps, op, payload)
}

#[wasm_bindgen(js_name = protocolVersion)]
pub fn protocol_version() -> u32 {
    openhub_bo_core::ffi::PROTOCOL_VERSION
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}
