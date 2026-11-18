//! The tidying numbers of `Drawing.trace_levels` and `trace_labels`.

use crate::exception;
use crate::expr::Num;
use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyAnyMethods;

/// The levels of `trace_levels` as the caller gave them: one number, Param or expression, or a sequence of them.
pub(crate) struct Levels(pub(crate) Vec<vector::Expr>);

impl<'a, 'py> FromPyObject<'a, 'py> for Levels {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(Num(one)) = object.extract::<Num>() {
            return Ok(Self(vec![one]));
        }
        object
            .extract::<Vec<Num>>()
            .map(|list| Self(list.into_iter().map(|Num(level)| level).collect()))
            .map_err(|_| {
                PyTypeError::new_err(format!(
                    "expected a number or a list of numbers, Params or expressions, got {}",
                    object.repr().map_or_else(|_| String::new(), |text| text.to_string())
                ))
            })
    }
}

/// The whole number argument `name` as a count: a negative number is 0 and one past the range the largest, which the
/// call refuses naming `name`; a number with a fraction, or that is not finite, is refused here naming it.
pub(crate) fn whole(name: &str, value: &Bound<'_, PyAny>) -> PyResult<usize> {
    if let Ok(number) = value.extract::<i128>() {
        if number < 0 {
            return Err(exception(&vector::Error::Parameter {
                parameter: name.to_owned(),
                message: format!("must be a whole number from 1, got {number}"),
            }));
        }
        return Ok(usize::try_from(number).unwrap_or(usize::MAX));
    }
    match value.extract::<f64>() {
        Ok(number) if number.fract() == 0.0 && number.is_finite() => {
            // a whole float is the integer it reads as
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "whole, and a cast saturates"
            )]
            Ok(number as usize)
        }
        Ok(number) => Err(exception(&vector::Error::Parameter {
            parameter: name.to_owned(),
            message: format!("must be a whole number from 1, got {number}"),
        })),
        Err(_) => Err(PyTypeError::new_err(format!(
            "{name}: expected a whole number, got {}",
            value.repr()?
        ))),
    }
}

/// The tidying numbers of a trace as the caller gave them; what is left out keeps the default.
pub(crate) fn trace_of(
    smooth: Option<Num>,
    tolerance: Option<Num>,
    open: Option<Num>,
    close: Option<Num>,
    min_area: Option<Num>,
    min_hole: Option<Num>,
) -> vector::Trace {
    let mut trace = vector::Trace::new();
    if let Some(Num(value)) = smooth {
        trace = trace.smooth(value);
    }
    if let Some(Num(value)) = tolerance {
        trace = trace.tolerance(value);
    }
    if let Some(Num(value)) = open {
        trace = trace.open(value);
    }
    if let Some(Num(value)) = close {
        trace = trace.close(value);
    }
    if let Some(Num(value)) = min_area {
        trace = trace.min_area(value);
    }
    if let Some(Num(value)) = min_hole {
        trace = trace.min_hole(value);
    }
    trace
}

/// The colours a trace keeps apart unless told otherwise.
const DEFAULT_COLORS: usize = 32;

/// `max_colors` as the caller gave it, or the default.
pub(crate) fn colors(max_colors: Option<&Bound<'_, PyAny>>) -> PyResult<usize> {
    max_colors.map_or(Ok(DEFAULT_COLORS), |count| whole("max_colors", count))
}
