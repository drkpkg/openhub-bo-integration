//! Native module `openhub_bo.payouts._native`: QR payouts and ACH
//! transfer batches. The token comes from `openhub_bo.core`. Logic lives in
//! `openhub-bo-payouts`.

use openhub_bo_payouts::PayoutsOps;
use pyo3::prelude::*;

/// Runs a payout operation; `payload` and the result are JSON strings
/// (see `openhub_bo_core::ffi`).
#[pyfunction]
fn call(op: &str, payload: &str) -> String {
    openhub_bo_core::ffi::call(&PayoutsOps, op, payload)
}

#[pymodule]
#[pyo3(name = "_native")]
fn openhub_bo_payouts_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(call, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("PROTOCOL_VERSION", openhub_bo_core::ffi::PROTOCOL_VERSION)?;
    Ok(())
}
