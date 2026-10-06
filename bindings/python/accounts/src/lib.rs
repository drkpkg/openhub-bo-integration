//! Native module `openhub_bo.accounts._native`: merchant accounts, balances
//! and movements. The token comes from `openhub_bo.core`. Logic lives in
//! `openhub-accounts`.

use openhub_accounts::AccountsOps;
use pyo3::prelude::*;

/// Runs a merchant-accounts operation; `payload` and the result are JSON strings
/// (see `openhub_core::ffi`).
#[pyfunction]
fn call(op: &str, payload: &str) -> String {
    openhub_core::ffi::call(&AccountsOps, op, payload)
}

#[pymodule]
#[pyo3(name = "_native")]
fn openhub_bo_accounts_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(call, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("PROTOCOL_VERSION", openhub_core::ffi::PROTOCOL_VERSION)?;
    Ok(())
}
