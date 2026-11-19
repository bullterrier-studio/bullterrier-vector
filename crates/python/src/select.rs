//! The selections of `only` and `without`: attributes and the values they ask for.

use crate::PyPoint;
use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};

/// What a selection asks of one attribute: a number, a character, text, a `Point`, or a list of them for any of.
fn wanted(name: &str, value: &Bound<'_, PyAny>) -> PyResult<vector::Wanted> {
    if value.is_instance_of::<PyList>() || value.is_instance_of::<PyTuple>() {
        let listed = value
            .try_iter()?
            .map(|one| wanted(name, &one?))
            .collect::<PyResult<Vec<_>>>()?;
        return Ok(listed.into());
    }
    if let Ok(text) = value.cast::<PyString>() {
        let text = text.to_str()?;
        let mut characters = text.chars();
        return Ok(match (characters.next(), characters.next()) {
            (Some(character), None) => character.into(),
            _ => text.into(),
        });
    }
    if let Ok(point) = value.cast::<PyPoint>() {
        let PyPoint { x, y } = *point.get();
        return Ok(vector::Point::new(x, y).into());
    }
    match value.extract::<f64>() {
        Ok(number) => Ok(number.into()),
        Err(_) => Err(PyTypeError::new_err(format!(
            "{name}: expected a number, text, a Point or a list of them, got {}",
            value.repr()?
        ))),
    }
}

/// The selection the keyword arguments of `only` and `without` describe.
pub(crate) fn selection(attributes: Option<&Bound<'_, PyDict>>) -> PyResult<vector::Select> {
    let mut select = vector::Select::new();
    if let Some(attributes) = attributes {
        for (name, value) in attributes {
            let name = name.extract::<String>()?;
            select = select.attribute(&name, wanted(&name, &value)?);
        }
    }
    Ok(select)
}
