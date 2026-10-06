//! Native module `OpenhubBo::Accounts::Native` of the openhub-bo-accounts gem. All logic
//! lives in the Rust crates; Ruby talks to them through one JSON entry point
//! (see `openhub_core::ffi`).

use magnus::{Error, Ruby, function, prelude::*};

fn call(op: String, payload: String) -> String {
    openhub_core::ffi::call(&openhub_accounts::AccountsOps, &op, &payload)
}

fn protocol_version() -> u32 {
    openhub_core::ffi::PROTOCOL_VERSION
}

fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[magnus::init(name = "openhub_bo_accounts")]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby
        .define_module("OpenhubBo")?
        .define_module("Accounts")?
        .define_module("Native")?;
    module.define_module_function("call", function!(call, 2))?;
    module.define_module_function("protocol_version", function!(protocol_version, 0))?;
    module.define_module_function("version", function!(version, 0))?;
    Ok(())
}
