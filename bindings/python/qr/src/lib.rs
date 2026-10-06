//! Native module `openhub_bo.qr._native`: QR Simple / MLD-BCB operations.
//! The token comes from `openhub_bo.core`. Logic lives in `openhub-qr`.

use openhub_qr::QrOps;
use pyo3::prelude::*;

/// Runs a QR operation; `payload` and the result are JSON strings
/// (see `openhub_core::ffi`).
#[pyfunction]
fn call(op: &str, payload: &str) -> String {
    openhub_core::ffi::call(&QrOps, op, payload)
}

#[pymodule]
#[pyo3(name = "_native")]
fn openhub_bo_qr_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(call, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("PROTOCOL_VERSION", openhub_core::ffi::PROTOCOL_VERSION)?;
    Ok(())
}
