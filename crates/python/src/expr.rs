//! `Expr` and `Param`: numbers a drawing reads when it is drawn.

use crate::raised;
use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyFloat;

/// An expression on Params and numbers, built with `+ - * /` and unary `-`. A drawing step takes one wherever it takes a
/// number, and works its value out when the drawing is drawn. Printing one gives the arithmetic that made it.
#[pyclass(name = "Expr", module = "bullterrier_vector", subclass, frozen)]
#[derive(Clone)]
pub(crate) struct PyExpr(pub(crate) vector::Expr);

#[pymethods]
impl PyExpr {
    fn __add__(&self, other: Num) -> Self {
        Self(self.0.clone() + other.0)
    }

    fn __radd__(&self, other: Num) -> Self {
        Self(other.0 + self.0.clone())
    }

    fn __sub__(&self, other: Num) -> Self {
        Self(self.0.clone() - other.0)
    }

    fn __rsub__(&self, other: Num) -> Self {
        Self(other.0 - self.0.clone())
    }

    fn __mul__(&self, other: Num) -> Self {
        Self(self.0.clone() * other.0)
    }

    fn __rmul__(&self, other: Num) -> Self {
        Self(other.0 * self.0.clone())
    }

    fn __truediv__(&self, other: Num) -> Self {
        Self(self.0.clone() / other.0)
    }

    fn __rtruediv__(&self, other: Num) -> Self {
        Self(other.0 / self.0.clone())
    }

    fn __neg__(&self) -> Self {
        Self(-self.0.clone())
    }

    /// Equal when built the same way from the same Params.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        other.cast::<Self>().is_ok_and(|other| other.get().0 == self.0)
    }

    /// Hashes by the printed form, which equal expressions share.
    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        self.0.to_string().into_pyobject(py)?.hash()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("Expr('{}')", self.0)
    }
}

/// A number as the caller gave it to a drawing step: a plain number, a `Param` or an expression.
pub(crate) struct Num(pub(crate) vector::Expr);

impl<'a, 'py> FromPyObject<'a, 'py> for Num {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(expr) = object.cast::<PyExpr>() {
            return Ok(Self(expr.get().0.clone()));
        }
        match object.extract::<f64>() {
            Ok(number) => Ok(Self(number.into())),
            Err(_) => Err(PyTypeError::new_err(format!(
                "expected a number, a Param or an expression, got {}",
                object.repr()?
            ))),
        }
    }
}

/// A number from `first` on the first shape of the list a step works on to `last` on the last, linear between, by the
/// shape's `index`; `first` on a single shape.
#[pyfunction]
pub(crate) fn by_index(first: Num, last: Num) -> PyExpr {
    PyExpr(vector::by_index(first.0, last.0))
}

/// As `by_index`, by the shape's `order`: `first` at order 0, `last` at order 1.
#[pyfunction]
pub(crate) fn by_order(first: Num, last: Num) -> PyExpr {
    PyExpr(vector::by_order(first.0, last.0))
}

/// A number from `low` up to `high` for each shape, from `seed` and the shape's index; the same seed gives the same
/// numbers everywhere (`None`: 0). `low` above `high`, or a `seed` that is not a whole number, is a `ParameterError`.
#[pyfunction]
#[pyo3(signature = (low, high, *, seed=None))]
pub(crate) fn random(low: Num, high: Num, seed: Option<Num>) -> PyExpr {
    let mut random = vector::Random::new(low.0, high.0);
    if let Some(seed) = seed {
        random = random.seed(seed.0);
    }
    PyExpr(random.into())
}

/// Each shape's own progress from 0 to 1 out of `progress`: the shape at order `p` runs while `progress` goes from
/// `p * spread` to `p * spread + 1 - spread` (`spread` `None`: 0.5). `spread` outside 0 up to but not including 1 is a `ParameterError`.
#[pyfunction]
#[pyo3(signature = (progress, *, spread=None))]
pub(crate) fn stagger(progress: Num, spread: Option<Num>) -> PyExpr {
    let mut stagger = vector::Stagger::new(progress.0);
    if let Some(spread) = spread {
        stagger = stagger.spread(spread.0);
    }
    PyExpr(stagger.into())
}

/// A named number with a unit, a range, a default and a meaning. It is an `Expr`: a step takes it wherever it takes a
/// number, and its value is given when the drawing is drawn.
#[pyclass(name = "Param", module = "bullterrier_vector", extends = PyExpr, frozen)]
pub(crate) struct PyParam(vector::Param);

impl PyParam {
    /// The Python object for a core Param.
    pub(crate) fn object(py: Python<'_>, param: vector::Param) -> PyResult<Py<Self>> {
        Py::new(py, Self::initializer(param))
    }

    fn initializer(param: vector::Param) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyExpr(vector::Expr::from(&param))).add_subclass(Self(param))
    }
}

#[pymethods]
impl PyParam {
    #[new]
    #[pyo3(signature = (name, *, default, min, max, unit="", meaning=""))]
    fn new(
        name: &str,
        default: f64,
        min: f64,
        max: f64,
        unit: &str,
        meaning: &str,
    ) -> PyResult<PyClassInitializer<Self>> {
        let unit = raised(vector::Unit::parse(unit))?;
        let param = raised(raised(vector::Param::new(name, default, min, max))?.unit(unit))?;
        Ok(Self::initializer(param.meaning(meaning)))
    }

    /// The name a value is given by when drawing.
    #[getter]
    fn name(&self) -> &str {
        self.0.name()
    }

    /// The value taken when none is given.
    #[getter]
    fn default(&self) -> f64 {
        self.0.default()
    }

    /// The lowest value.
    #[getter]
    fn min(&self) -> f64 {
        self.0.min()
    }

    /// The highest value.
    #[getter]
    fn max(&self) -> f64 {
        self.0.max()
    }

    /// `"px"`, `"deg"`, `"cycle"`, `"fraction"`, or `""` for a plain number.
    #[getter]
    fn unit(&self) -> &'static str {
        self.0.unit_of().name()
    }

    /// The line a person reads beside a slider.
    #[getter]
    fn meaning(&self) -> &str {
        self.0.meaning_of()
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let number = |value: f64| PyFloat::new(py, value).repr().map(|text| text.to_string());
        let text = |value: &str| value.into_pyobject(py)?.repr().map(|text| text.to_string());
        Ok(format!(
            "Param({}, default={}, min={}, max={}, unit={}, meaning={})",
            text(self.0.name())?,
            number(self.0.default())?,
            number(self.0.min())?,
            number(self.0.max())?,
            text(self.0.unit_of().name())?,
            text(self.0.meaning_of())?,
        ))
    }
}
