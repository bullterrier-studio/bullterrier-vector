//! `Drawing`: a tree of steps that draws shapes.

use crate::expr::{Num, PyExpr, PyParam};
use crate::shape::PyShape;
use crate::{PyPath, PyPoint, exception, raised};
use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};

/// An immutable tree of steps that draws shapes. Every step returns a new drawing and leaves this one alone; a drawing
/// holds no callbacks, so it draws again with new values.
#[pyclass(name = "Drawing", module = "bullterrier_vector", frozen)]
#[derive(Clone)]
pub(crate) struct PyDrawing(vector::Drawing);

/// The drawing a step made, or the error it left: a step that cannot be taken is raised at the call that made it.
fn checked(drawing: vector::Drawing) -> PyResult<PyDrawing> {
    match drawing.error() {
        Some(error) => Err(exception(error)),
        None => Ok(PyDrawing(drawing)),
    }
}

/// What a drawing is made of, as the caller gave it: a `Path`, a `Shape` or a `Drawing`.
struct Item(vector::Drawing);

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
struct Pair(vector::Expr, vector::Expr);

impl<'a, 'py> FromPyObject<'a, 'py> for Pair {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(point) = object.cast::<PyPoint>() {
            let PyPoint { x, y } = *point.get();
            return Ok(Self(x.into(), y.into()));
        }
        match object.extract::<(Num, Num)>() {
            Ok((x, y)) => Ok(Self(x.0, y.0)),
            Err(_) => Err(PyTypeError::new_err(format!(
                "expected a Point or an (x, y) tuple of numbers, got {}",
                object.repr()?
            ))),
        }
    }
}

/// How much to scale by: a number, or a pair.
enum By {
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
                "expected a number or an (x, y) pair of numbers, got {}",
                object.repr()?
            ))),
        }
    }
}

/// The stroke a call describes; the core holds the defaults of what is left out.
fn stroke_of(
    color: &str,
    width: Option<Num>,
    join: Option<&str>,
    cap: Option<&str>,
    miter_limit: Option<Num>,
) -> PyResult<vector::Stroke> {
    let mut stroke = vector::Stroke::new(raised(vector::Color::parse(color))?);
    if let Some(width) = width {
        stroke = stroke.width(width.0);
    }
    if let Some(join) = join {
        stroke = stroke.join(raised(vector::Join::parse(join))?);
    }
    if let Some(cap) = cap {
        stroke = stroke.cap(raised(vector::Cap::parse(cap))?);
    }
    if let Some(limit) = miter_limit {
        stroke = stroke.miter_limit(limit.0);
    }
    Ok(stroke)
}

/// What a selection asks of one attribute: a number, a character, a `Point`, or a list of them for any of.
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
        return match (characters.next(), characters.next()) {
            (Some(character), None) => Ok(character.into()),
            _ => Err(crate::ParameterError::new_err(format!(
                "{name}: a character is one character, got {text:?}"
            ))),
        };
    }
    if let Ok(point) = value.cast::<PyPoint>() {
        let PyPoint { x, y } = *point.get();
        return Ok(vector::Point::new(x, y).into());
    }
    match value.extract::<f64>() {
        Ok(number) => Ok(number.into()),
        Err(_) => Err(PyTypeError::new_err(format!(
            "{name}: expected a number, a character, a Point or a list of them, got {}",
            value.repr()?
        ))),
    }
}

/// The selection the keyword arguments of `only` and `without` describe.
fn selection(attributes: Option<&Bound<'_, PyDict>>) -> PyResult<vector::Select> {
    let mut select = vector::Select::new();
    if let Some(attributes) = attributes {
        for (name, value) in attributes {
            let name = name.extract::<String>()?;
            select = select.attribute(&name, wanted(&name, &value)?);
        }
    }
    Ok(select)
}

/// The values the keyword arguments of `shapes` and `with_values` give: a number, or a list with a number per shape.
fn values(given: Option<&Bound<'_, PyDict>>) -> PyResult<vector::Values> {
    let mut values = vector::Values::new();
    if let Some(given) = given {
        for (name, value) in given {
            let name = name.extract::<String>()?;
            let value = match value.extract::<f64>() {
                Ok(number) => vector::Value::from(number),
                Err(_) => match value.extract::<Vec<f64>>() {
                    Ok(numbers) => vector::Value::from(numbers),
                    Err(_) => {
                        return Err(PyTypeError::new_err(format!(
                            "{name}: expected a number or a list of numbers, got {}",
                            value.repr()?
                        )));
                    }
                },
            };
            values = values.set(name, value);
        }
    }
    Ok(values)
}

#[pymethods]
impl PyDrawing {
    /// The items stacked in order, the first at the bottom, as a drawing of `size` (width, height in design pixels)
    /// if one is given.
    #[new]
    #[pyo3(signature = (*items, size=None))]
    fn new(items: &Bound<'_, PyTuple>, size: Option<(f64, f64)>) -> PyResult<Self> {
        let items = items
            .iter()
            .map(|item| item.extract::<Item>().map(|item| item.0))
            .collect::<PyResult<Vec<_>>>()?;
        let drawing = vector::Drawing::new(items);
        checked(match size {
            Some((width, height)) => drawing.with_size(width, height),
            None => drawing,
        })
    }

    /// Every shape moved `dx` to the right and `dy` down.
    fn translate(&self, dx: Num, dy: Num) -> PyResult<Self> {
        checked(self.0.translate(dx.0, dy.0))
    }

    /// Every shape turned `angle` degrees clockwise about `about` (the middle of its bounds by default).
    #[pyo3(signature = (angle, *, about=None))]
    fn rotate(&self, angle: Num, about: Option<Pair>) -> PyResult<Self> {
        let mut turn = vector::Rotate::new(vector::Deg(angle.0));
        if let Some(Pair(x, y)) = about {
            turn = turn.about((x, y));
        }
        checked(self.0.rotate(turn))
    }

    /// Every shape scaled by a number, or by an `(x, y)` pair, about `about` (the middle of its bounds by default).
    #[pyo3(signature = (by, *, about=None))]
    fn scale(&self, by: By, about: Option<Pair>) -> PyResult<Self> {
        let mut scaling = match by {
            By::One(factor) => vector::ScaleBy::from(factor.0),
            By::Two(Pair(x, y)) => vector::ScaleBy::from((x, y)),
        };
        if let Some(Pair(x, y)) = about {
            scaling = scaling.about((x, y));
        }
        checked(self.0.scale(scaling))
    }

    /// Every shape filled with `color` (`"#rrggbb"`, `"#rrggbbaa"` or `"none"`).
    pub(crate) fn fill(&self, color: &str) -> PyResult<Self> {
        checked(self.0.fill(raised(vector::Color::parse(color))?))
    }

    /// Every outline stroked with `color`; what is left out takes the defaults (2 pixels, round, round, 4).
    #[pyo3(signature = (color, *, width=None, join=None, cap=None, miter_limit=None))]
    pub(crate) fn stroke(
        &self,
        color: &str,
        width: Option<Num>,
        join: Option<&str>,
        cap: Option<&str>,
        miter_limit: Option<Num>,
    ) -> PyResult<Self> {
        checked(self.0.stroke(stroke_of(color, width, join, cap, miter_limit)?))
    }

    /// The opacity of every shape multiplied by `alpha`, from 0 to 1.
    pub(crate) fn opacity(&self, alpha: Num) -> PyResult<Self> {
        checked(self.0.opacity(alpha.0))
    }

    /// Every shape painted over what is below it as `mode` (`"normal"` or `"multiply"`).
    pub(crate) fn blend(&self, mode: &str) -> PyResult<Self> {
        checked(self.0.blend(raised(vector::Blend::parse(mode))?))
    }

    /// Only the shapes whose attributes equal these (a list means any of). The list is renumbered.
    #[pyo3(signature = (**attributes))]
    fn only(&self, attributes: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        checked(self.0.only(selection(attributes)?))
    }

    /// The shapes whose attributes do not equal these. The list is renumbered.
    #[pyo3(signature = (**attributes))]
    fn without(&self, attributes: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        checked(self.0.without(selection(attributes)?))
    }

    /// The same drawing with these Params fixed to the numbers (or lists with a number per shape) given.
    #[pyo3(signature = (**given))]
    fn with_values(&self, given: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        checked(self.0.with_values(&values(given)?))
    }

    /// Every Param the drawing reads, in the order first used.
    fn parameters(&self, py: Python<'_>) -> PyResult<Vec<Py<PyParam>>> {
        raised(self.0.parameters())?
            .into_iter()
            .map(|param| PyParam::object(py, param))
            .collect()
    }

    /// The shapes the drawing evaluates to, bottom first, in design pixels, with each Param at the value given by
    /// name (a number, or a list with a number per shape) and at its default where none is.
    #[pyo3(signature = (**given))]
    fn shapes(&self, given: Option<&Bound<'_, PyDict>>) -> PyResult<Vec<PyShape>> {
        let shapes = raised(self.0.shapes(&values(given)?))?;
        Ok(shapes.into_iter().map(PyShape).collect())
    }

    /// The design frame `(width, height)` in design pixels, or `None`.
    #[getter]
    fn size(&self) -> Option<(f64, f64)> {
        self.0.size()
    }

    fn __repr__(&self) -> String {
        self.0.size().map_or_else(
            || "Drawing(size=None)".to_owned(),
            |(width, height)| format!("Drawing(size=({width:?}, {height:?}))"),
        )
    }
}

impl PyDrawing {
    /// The drawing of a path, to give it a style step.
    pub(crate) fn of(path: &PyPath) -> Self {
        Self(path.0.clone().into())
    }
}

/// The error for a Param or expression where only a plain number goes (a `Path` verb).
pub(crate) fn refuse_expr(object: &Bound<'_, PyAny>) -> PyResult<()> {
    if object.is_instance_of::<PyExpr>() {
        return Err(crate::ParameterError::new_err(format!(
            "{object} is a Param or an expression, which only a Drawing step reads: wrap the path in Drawing(path) first"
        )));
    }
    Ok(())
}

/// The same for either number of an `(x, y)` tuple.
pub(crate) fn refuse_expr_in_pair(object: &Bound<'_, PyAny>) -> PyResult<()> {
    if let Ok((x, y)) = object.extract::<(Bound<'_, PyAny>, Bound<'_, PyAny>)>() {
        refuse_expr(&x)?;
        refuse_expr(&y)?;
    }
    Ok(())
}
