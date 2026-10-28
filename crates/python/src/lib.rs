//! `bullterrier_vector._native._core`: the Rust core's functions for Python. Each passes its arguments to the core
//! unchanged, so the core's range checks are the only ones and every language gets the same errors.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

pyo3::import_exception!(bullterrier_vector.errors, ParameterError);

/// The Rust core of bullterrier-vector, for Python.
#[pymodule]
mod _core {
    #[pymodule_export]
    use super::polygon_svg;
}

/// A regular polygon with `sides` corners (3 to 1000) on a circle of `radius` (more than 0, at most 1 000 000), as an
/// SVG document; the first corner points up and the rest follow clockwise.
///
/// `ParameterError` if either value is out of range, naming it and the value given. An int beyond 64 bits is an
/// `OverflowError` and a non-int `sides` a `TypeError`: `PyO3`'s extraction, before the core is called.
#[pyfunction]
fn polygon_svg(sides: i64, radius: f64) -> PyResult<String> {
    bullterrier_vector::polygon_svg(sides, radius).map_err(|error| match error {
        bullterrier_vector::Error::Parameter { .. } => ParameterError::new_err(error.to_string()),
        // `Error` is non-exhaustive: a variant this binding does not know yet is still a ValueError, never a panic
        _ => PyValueError::new_err(error.to_string()),
    })
}
