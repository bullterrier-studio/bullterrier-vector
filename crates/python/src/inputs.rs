//! What a drawing step takes, read from what the Python caller gave: an item (a `Path`, a `Shape` or a `Drawing`), a pair of
//! numbers and a scale factor.

use crate::drawing::PyDrawing;
use crate::expr::Num;
use crate::shape::PyShape;
use crate::{PyPath, PyPoint, two_items};
use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

/// What a drawing is made of, as the caller gave it: a `Path`, a `Shape` or a `Drawing`.
pub(crate) struct Item(pub(crate) vector::Drawing);

impl<'a, 'py> FromPyObject<'a, 'py> for Item {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(drawing) = object.cast::<PyDrawing>() {
            return Ok(Self(drawing.get().0.clone()));
        }
        if let Ok(path) = object.cast::<PyPath>() {
            return Ok(Self(path.get().0.clone().into()));
        }
        if let Ok(shape) = object.cast::<PyShape>() {
            return Ok(Self(shape.get().0.clone().into()));
        }
        Err(PyTypeError::new_err(format!(
            "expected a Path, a Shape or a Drawing, got {}",
            object.repr()?
        )))
    }
}

/// A pair of numbers: a `Point`, or an `(x, y)` tuple of numbers, Params or expressions.
pub(crate) struct Pair(pub(crate) vector::Expr, pub(crate) vector::Expr);

impl<'a, 'py> FromPyObject<'a, 'py> for Pair {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(point) = object.cast::<PyPoint>() {
            let PyPoint { x, y } = *point.get();
            return Ok(Self(x.into(), y.into()));
        }
        if let Some((x, y)) = two_items(&object)
            && let (Ok(x), Ok(y)) = (x.extract::<Num>(), y.extract::<Num>())
        {
            return Ok(Self(x.0, y.0));
        }
        Err(PyTypeError::new_err(format!(
            "expected a Point or an (x, y) tuple of numbers, got {}",
            object.repr()?
        )))
    }
}

/// One number for both of a pair, or the pair: a scale's `by`.
pub(crate) enum By {
    One(Num),
    Two(Pair),
}

impl<'a, 'py> FromPyObject<'a, 'py> for By {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(one) = object.extract::<Num>() {
            return Ok(Self::One(one));
        }
        match object.extract::<Pair>() {
            Ok(two) => Ok(Self::Two(two)),
            Err(_) => Err(PyTypeError::new_err(format!(
                "expected a number or a pair of numbers, got {}",
                object.repr()?
            ))),
        }
    }
}
