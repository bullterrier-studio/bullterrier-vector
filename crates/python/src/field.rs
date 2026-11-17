//! `Field` and `Flow`: a number and a direction at every point, the inputs of `hatch`.

use crate::{Pt, PyPath, raised};
use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

/// A number at every point: a ramp along a line or the distance to a path's outline. `hatch` reads one as its `tone`.
#[pyclass(name = "Field", module = "bullterrier_vector", frozen, eq)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyField(pub(crate) vector::Field);

#[pymethods]
impl PyField {
    /// 0 at `start`, 1 at `end`, linear along the line between and held beyond it.
    #[staticmethod]
    fn ramp(start: Pt, end: Pt) -> PyResult<Self> {
        raised(vector::Field::ramp(start.0, end.0)).map(Self)
    }

    /// The distance to the outline of `path` in pixels, negative inside. With `normalized`, inside each part 0 on its
    /// outer edge rising to 1 on its holes' edges (or at its deepest point where it has none); 0 outside.
    #[staticmethod]
    #[pyo3(signature = (path, *, normalized=false))]
    fn distance(path: &PyPath, normalized: bool) -> PyResult<Self> {
        raised(vector::Field::distance(&path.0, normalized)).map(Self)
    }

    /// The field's number at `(x, y)`.
    fn at(&self, x: f64, y: f64) -> f64 {
        self.0.at(vector::Point::new(x, y))
    }

    fn __repr__(&self) -> String {
        self.0.to_string()
    }
}

/// A direction without sign at every point: a line through the point, not an arrow. `hatch` draws along one.
#[pyclass(name = "Flow", module = "bullterrier_vector", frozen, eq)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyFlow(pub(crate) vector::Flow);

#[pymethods]
impl PyFlow {
    /// The same direction everywhere: `degrees` clockwise from the right on screen.
    #[staticmethod]
    fn angle(angle: f64) -> PyResult<Self> {
        raised(vector::Flow::angle(angle)).map(Self)
    }

    /// Smooth seeded noise; `phase` in cycles moves it, one cycle on is the same flow.
    #[staticmethod]
    #[pyo3(signature = (*, wavelength=400.0, phase=0.0, seed=0.0))]
    fn noise(wavelength: f64, phase: f64, seed: f64) -> PyResult<Self> {
        raised(vector::Flow::noise(wavelength, phase, seed)).map(Self)
    }

    /// The direction of the outline of `path` nearest each point.
    #[staticmethod]
    fn contour(path: &PyPath) -> PyResult<Self> {
        raised(vector::Flow::contour(&path.0)).map(Self)
    }

    /// `fraction` (0 to 1) of the way to `other`; a direction and its opposite count as the same, so they never cancel.
    fn mix(&self, other: &PyFlow, fraction: f64) -> PyResult<Self> {
        raised(self.0.mix(&other.0, fraction)).map(Self)
    }

    /// The direction at `(x, y)` in degrees from 0 up to 180, clockwise from the right on screen.
    fn at(&self, x: f64, y: f64) -> f64 {
        self.0.at(vector::Point::new(x, y))
    }

    fn __repr__(&self) -> String {
        self.0.to_string()
    }
}

/// A tone as the caller gave it: a `Field`, or a number that is the same everywhere.
pub(crate) struct Tone(pub(crate) vector::Field);

impl<'a, 'py> FromPyObject<'a, 'py> for Tone {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(field) = object.cast::<PyField>() {
            return Ok(Self(field.get().0.clone()));
        }
        match object.extract::<f64>() {
            Ok(number) => raised(vector::Field::constant(number)).map(Self),
            Err(_) => Err(PyTypeError::new_err(format!(
                "expected a Field or a number, got {}",
                object.repr()?
            ))),
        }
    }
}

/// A spacing as the caller gave it: one number, or `(at tone 0, at tone 1)`.
#[derive(Clone, Copy)]
pub(crate) struct Spacing(pub(crate) (f64, f64));

impl<'a, 'py> FromPyObject<'a, 'py> for Spacing {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(one) = object.extract::<f64>() {
            return Ok(Self((one, one)));
        }
        match object.extract::<(f64, f64)>() {
            Ok(pair) => Ok(Self(pair)),
            Err(_) => Err(PyTypeError::new_err(format!(
                "expected a number or a (at tone 0, at tone 1) pair of numbers, got {}",
                object.repr()?
            ))),
        }
    }
}

/// The hatch a call's `spacing`, `tone` and `flow` describe: a pair of spacings that differ needs a tone to read.
pub(crate) fn hatch_of(spacing: Spacing, tone: Option<Tone>, flow: Option<&PyFlow>) -> PyResult<vector::Hatch> {
    let ((near, far), tone) = (spacing.0, tone.map(|tone| tone.0));
    let hatch = match tone {
        Some(tone) => vector::Hatch::between(near, far, tone),
        None if near.to_bits() == far.to_bits() => vector::Hatch::new(near),
        None => {
            return Err(crate::ParameterError::new_err(
                "tone: is needed when `spacing` is a pair",
            ));
        }
    };
    Ok(match flow {
        Some(flow) => hatch.flow(flow.0.clone()),
        None => hatch,
    })
}
