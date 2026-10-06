//! Native module `openhub_bo.core._native`: operations shared by every
//! product package (OAuth token). Logic lives in `openhub-core`.

use openhub_core::CoreOps;
use pyo3::prelude::*;

/// Runs a core operation; `payload` and the result are JSON strings
/// (see `openhub_core::ffi`).
#[pyfunction]
fn call(op: &str, payload: &str) -> String {
    openhub_core::ffi::call(&CoreOps, op, payload)
}

#[pymodule]
#[pyo3(name = "_native")]
fn openhub_bo_core_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(call, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("PROTOCOL_VERSION", openhub_core::ffi::PROTOCOL_VERSION)?;
    Ok(())
}
