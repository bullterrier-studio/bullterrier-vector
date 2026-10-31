//! `bullterrier_vector._native._core`: the Rust core for Python. Each function passes its arguments to the core
//! unchanged, so the core's range checks are the only ones and every language gets the same errors.
//!
//! The module is empty until the `Path` bindings land.

use pyo3::prelude::*;

pyo3::import_exception!(bullterrier_vector.errors, BullterrierVectorError);
pyo3::import_exception!(bullterrier_vector.errors, ParameterError);
pyo3::import_exception!(bullterrier_vector.errors, GeometryError);

/// The Rust core of bullterrier-vector, for Python.
#[pymodule]
mod _core {}

/// The Python exception for a core error: one class per kind, `ParameterError` and `GeometryError`.
#[expect(dead_code, reason = "no binding returns a core error until the Path bindings land")]
fn exception(error: &bullterrier_vector::Error) -> PyErr {
    match error {
        bullterrier_vector::Error::Parameter { .. } => ParameterError::new_err(error.to_string()),
        bullterrier_vector::Error::Geometry { .. } => GeometryError::new_err(error.to_string()),
        // `Error` is non-exhaustive: a variant this binding does not map yet is still a package error, never a panic
        _ => BullterrierVectorError::new_err(error.to_string()),
    }
}
