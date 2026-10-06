//! WASM module of `@openhub-bo/core`. All logic lives in the Rust crates;
//! this only exposes the JSON entry point (see `openhub_bo_core::ffi`).

use openhub_bo_core::CoreOps;
use wasm_bindgen::prelude::*;

/// Runs an operation; `payload` and the result are JSON strings.
#[wasm_bindgen]
pub fn call(op: &str, payload: &str) -> String {
    openhub_bo_core::ffi::call(&CoreOps, op, payload)
}

#[wasm_bindgen(js_name = protocolVersion)]
pub fn protocol_version() -> u32 {
    openhub_bo_core::ffi::PROTOCOL_VERSION
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}
