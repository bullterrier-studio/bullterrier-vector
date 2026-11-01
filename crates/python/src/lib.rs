//! `bullterrier_vector._native._core`: the Rust core for Python. Each method passes its arguments to the core
//! unchanged, so the core's range checks are the only ones and every language gets the same errors; this file only
//! converts between Python's values and the core's, and maps each core error to its exception.

mod drawing;
mod expr;
mod mesh;
mod shape;
mod text;

use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};

use crate::drawing::{PyDrawing, refuse_expr, refuse_expr_in_pair};
use crate::expr::Num;

pyo3::import_exception!(bullterrier_vector.errors, BullterrierVectorError);
pyo3::import_exception!(bullterrier_vector.errors, ParameterError);
pyo3::import_exception!(bullterrier_vector.errors, GeometryError);
pyo3::import_exception!(bullterrier_vector.errors, FormatError);

/// The Rust core of bullterrier-vector, for Python.
#[pymodule]
mod _core {
    #[pymodule_export]
    use super::drawing::PyDrawing;
    #[pymodule_export]
    use super::expr::{PyExpr, PyParam};
    #[pymodule_export]
    use super::mesh::PyMesh;
    #[pymodule_export]
    use super::shape::{PyShape, PyStyle};
    #[pymodule_export]
    use super::text::{PyFont, text};
    #[pymodule_export]
    use super::{PyPath, PyPoint, PyRect, drawing_schema};
}

/// The JSON Schema of the drawing format as text, generated from the library's own declarations of its calls.
#[pyfunction]
fn drawing_schema() -> String {
    vector::drawing_schema()
}

/// The Python exception for a core error: one class per kind: `ParameterError`, `GeometryError` and `FormatError`.
pub(crate) fn exception(error: &vector::Error) -> PyErr {
    match error {
        vector::Error::Parameter { .. } => ParameterError::new_err(error.to_string()),
        vector::Error::Geometry { .. } => GeometryError::new_err(error.to_string()),
        vector::Error::Format { .. } => FormatError::new_err(error.to_string()),
        // `Error` is non-exhaustive: a variant this binding does not map yet is still a package error, never a panic
        _ => BullterrierVectorError::new_err(error.to_string()),
    }
}

/// Run a core call and raise its error as the matching exception.
pub(crate) fn raised<T>(result: Result<T, vector::Error>) -> PyResult<T> {
    result.map_err(|error| exception(&error))
}

/// A position, in design pixels with `y` pointing down. A `(x, y)` tuple goes anywhere a point does.
#[pyclass(name = "Point", module = "bullterrier_vector", frozen)]
#[derive(Clone, Copy)]
pub(crate) struct PyPoint {
    /// Distance to the right.
    #[pyo3(get)]
    pub(crate) x: f64,
    /// Distance down.
    #[pyo3(get)]
    pub(crate) y: f64,
}

/// A point as the caller gave it: a `Point` or a tuple of two numbers.
#[derive(Clone, Copy)]
struct Pt(vector::Point);

impl<'a, 'py> FromPyObject<'a, 'py> for Pt {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(point) = object.cast::<PyPoint>() {
            let PyPoint { x, y } = *point.get();
            return Ok(Self(vector::Point::new(x, y)));
        }
        if let Ok(pair) = object.extract::<(f64, f64)>() {
            return Ok(Self(pair.into()));
        }
        refuse_expr_in_pair(&object)?;
        Err(PyTypeError::new_err(format!(
            "expected a Point or an (x, y) tuple of numbers, got {}",
            object.repr()?
        )))
    }
}

impl From<vector::Point> for PyPoint {
    fn from(point: vector::Point) -> Self {
        Self { x: point.x, y: point.y }
    }
}

#[pymethods]
impl PyPoint {
    #[new]
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn __add__(&self, other: Pt) -> Self {
        Self {
            x: self.x + other.0.x,
            y: self.y + other.0.y,
        }
    }

    fn __radd__(&self, other: Pt) -> Self {
        self.__add__(other)
    }

    fn __sub__(&self, other: Pt) -> Self {
        Self {
            x: self.x - other.0.x,
            y: self.y - other.0.y,
        }
    }

    fn __rsub__(&self, other: Pt) -> Self {
        Self {
            x: other.0.x - self.x,
            y: other.0.y - self.y,
        }
    }

    fn __mul__(&self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
        }
    }

    fn __rmul__(&self, factor: f64) -> Self {
        self.__mul__(factor)
    }

    #[expect(
        clippy::unused_self,
        reason = "Python calls len() on the instance; a point always has two numbers"
    )]
    fn __len__(&self) -> usize {
        2
    }

    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyIterator>> {
        PyList::new(py, [self.x, self.y])?.try_iter()
    }

    fn __eq__(&self, other: Pt) -> bool {
        vector::Point::new(self.x, self.y) == other.0
    }

    /// Hashes as the tuple `(x, y)` does, which a point equals.
    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        PyTuple::new(py, [self.x, self.y])?.hash()
    }

    fn __repr__(&self) -> String {
        format!("Point({:?}, {:?})", self.x, self.y)
    }
}

/// An axis-aligned rectangle: its top-left corner and its size, in design pixels.
#[pyclass(name = "Rect", module = "bullterrier_vector", frozen, eq)]
#[derive(Clone, Copy, PartialEq)]
struct PyRect {
    /// Left edge.
    #[pyo3(get)]
    x: f64,
    /// Top edge.
    #[pyo3(get)]
    y: f64,
    /// Size to the right.
    #[pyo3(get)]
    width: f64,
    /// Size downwards.
    #[pyo3(get)]
    height: f64,
}

#[pymethods]
impl PyRect {
    #[new]
    fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        PyTuple::new(py, [self.x, self.y, self.width, self.height])?.hash()
    }

    fn __repr__(&self) -> String {
        format!(
            "Rect(x={:?}, y={:?}, width={:?}, height={:?})",
            self.x, self.y, self.width, self.height
        )
    }
}

/// How much to scale by, as the caller gave it: a number, or an `(x, y)` pair.
#[derive(Clone, Copy)]
struct By(vector::Scale);

impl<'a, 'py> FromPyObject<'a, 'py> for By {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        refuse_expr(&object)?;
        if let Ok(factor) = object.extract::<f64>() {
            return Ok(Self(factor.into()));
        }
        refuse_expr_in_pair(&object)?;
        match object.extract::<Pt>() {
            Ok(Pt(pair)) => Ok(Self((pair.x, pair.y).into())),
            Err(_) => Err(PyTypeError::new_err(format!(
                "expected a number or an (x, y) pair of numbers, got {}",
                object.repr()?
            ))),
        }
    }
}

/// A number a `Path` verb takes: a plain number, never a Param or an expression.
#[derive(Clone, Copy)]
struct Plain(f64);

impl<'a, 'py> FromPyObject<'a, 'py> for Plain {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        refuse_expr(&object)?;
        object.extract::<f64>().map(Self).map_err(|_| {
            PyTypeError::new_err(format!(
                "expected a number, got {}",
                object.repr().map_or_else(|_| String::new(), |text| text.to_string())
            ))
        })
    }
}

/// A Bezier outline made of sub-paths, each open or closed; a plain value, so every verb returns a new path.
#[pyclass(name = "Path", module = "bullterrier_vector", frozen, eq)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyPath(pub(crate) vector::Path);

impl PyPath {
    fn of(result: Result<vector::Path, vector::Error>) -> PyResult<Self> {
        raised(result).map(Self)
    }
}

#[pymethods]
impl PyPath {
    /// An empty path.
    #[new]
    fn new() -> Self {
        Self(vector::Path::new())
    }

    /// Start a new sub-path at `point`.
    fn move_to(&self, point: Pt) -> PyResult<Self> {
        Self::of(self.0.move_to(point.0))
    }

    /// A straight segment to `point`.
    fn line_to(&self, point: Pt) -> PyResult<Self> {
        Self::of(self.0.line_to(point.0))
    }

    /// A cubic Bezier segment to `point`, leaving towards `control1` and arriving from `control2`.
    fn curve_to(&self, control1: Pt, control2: Pt, point: Pt) -> PyResult<Self> {
        Self::of(self.0.curve_to(control1.0, control2.0, point.0))
    }

    /// A quadratic Bezier segment to `point`, bent towards `control`.
    fn quad_to(&self, control: Pt, point: Pt) -> PyResult<Self> {
        Self::of(self.0.quad_to(control.0, point.0))
    }

    /// Close the open sub-path: the line back to its start is implied, not a segment of its own.
    fn close(&self) -> PyResult<Self> {
        Self::of(self.0.close())
    }

    /// One open segment from `start` to `end`.
    #[staticmethod]
    fn line(start: Pt, end: Pt) -> PyResult<Self> {
        Self::of(vector::Path::line(start.0, end.0))
    }

    /// Straight segments through `points`, closed if `closed`.
    #[staticmethod]
    #[pyo3(signature = (points, *, closed=false))]
    fn polyline(points: &Bound<'_, PyAny>, closed: bool) -> PyResult<Self> {
        let points = points
            .try_iter()?
            .map(|item| item?.extract::<Pt>().map(|point| point.0))
            .collect::<PyResult<Vec<_>>>()?;
        Self::of(vector::Path::polyline(points, closed))
    }

    /// A closed rectangle, its corners rounded by `radius`, starting at the left end of its top edge.
    #[staticmethod]
    #[pyo3(signature = (x, y, width, height, *, radius=0.0))]
    fn rect(x: f64, y: f64, width: f64, height: f64, radius: f64) -> PyResult<Self> {
        Self::of(vector::Path::rect(x, y, width, height, radius))
    }

    /// A closed circle, starting at the top and running clockwise.
    #[staticmethod]
    fn circle(center: Pt, radius: f64) -> PyResult<Self> {
        Self::of(vector::Path::circle(center.0, radius))
    }

    /// A closed regular polygon with a corner at the top, the rest clockwise.
    #[staticmethod]
    fn polygon(center: Pt, radius: f64, sides: i64) -> PyResult<Self> {
        Self::of(vector::Path::polygon(center.0, radius, sides))
    }

    /// A closed star with a tip at the top, the rest clockwise.
    #[staticmethod]
    #[pyo3(signature = (center, outer, inner, *, points=5))]
    fn star(center: Pt, outer: f64, inner: f64, points: i64) -> PyResult<Self> {
        Self::of(vector::Path::star(center.0, outer, inner, points))
    }

    /// The path an SVG path `d` attribute describes.
    #[staticmethod]
    fn from_svg_path(data: &str) -> PyResult<Self> {
        Self::of(vector::Path::from_svg_path(data))
    }

    /// The length in design pixels.
    #[getter]
    fn length(&self) -> f64 {
        self.0.length()
    }

    /// The smallest rectangle around the outline.
    #[getter]
    fn bounds(&self) -> PyResult<PyRect> {
        let rect = raised(self.0.bounds())?;
        Ok(PyRect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        })
    }

    /// The point a fraction `at` (0 to 1) of the way along.
    #[pyo3(signature = (at, *, near=None))]
    fn point_at(&self, at: f64, near: Option<Pt>) -> PyResult<PyPoint> {
        raised(self.0.point_at(at, near.map(|near| near.0))).map(PyPoint::from)
    }

    /// The unit direction of travel a fraction `at` of the way along.
    #[pyo3(signature = (at, *, near=None))]
    fn tangent_at(&self, at: f64, near: Option<Pt>) -> PyResult<PyPoint> {
        raised(self.0.tangent_at(at, near.map(|near| near.0))).map(PyPoint::from)
    }

    /// The position (0 to 1) of the point on the path nearest `point`.
    #[pyo3(signature = (point, *, near=None))]
    fn locate(&self, point: Pt, near: Option<Pt>) -> PyResult<f64> {
        raised(self.0.locate(point.0, near.map(|near| near.0)))
    }

    /// Whether `point` is inside the filled region (nonzero rule).
    fn contains(&self, point: Pt) -> PyResult<bool> {
        raised(self.0.contains(point.0))
    }

    /// The separate parts: each closed outline with the holes inside it, and each open line.
    fn split(&self) -> Vec<Self> {
        self.0.split().into_iter().map(Self).collect()
    }

    /// The piece between the positions `start` and `end` (0 to 1, `start` not after `end`).
    #[pyo3(signature = (start, end, *, near=None))]
    fn slice(&self, start: f64, end: f64, near: Option<Pt>) -> PyResult<Self> {
        Self::of(self.0.slice(start, end, near.map(|near| near.0)))
    }

    /// The same outline travelled the other way.
    fn reverse(&self) -> Self {
        Self(self.0.reverse())
    }

    /// Moved by `dx` to the right and `dy` down.
    fn translate(&self, dx: Plain, dy: Plain) -> PyResult<Self> {
        Self::of(self.0.translate(dx.0, dy.0))
    }

    /// Turned `angle` degrees clockwise about `about` (the middle of the bounds by default).
    #[pyo3(signature = (angle, *, about=None))]
    fn rotate(&self, angle: Plain, about: Option<Pt>) -> PyResult<Self> {
        Self::of(self.0.rotate(angle.0, about.map(|about| about.0)))
    }

    /// Scaled by a number, or by an `(x, y)` pair, about `about` (the middle of the bounds by default).
    #[pyo3(signature = (by, *, about=None))]
    fn scale(&self, by: By, about: Option<Pt>) -> PyResult<Self> {
        Self::of(self.0.scale(by.0, about.map(|about| about.0)))
    }

    /// The path with every point, on and off the curve, replaced by what `fn(point)` returns.
    fn map_points(&self, r#fn: &Bound<'_, PyAny>) -> PyResult<Self> {
        // the core's callback can only fail with a core error, so the function's own exception is kept here and the
        // placeholder error that stops the core is dropped for it
        let mut raised_by_fn: Option<PyErr> = None;
        let mapped = self.0.map_points(|point| {
            let called = r#fn.call1((PyPoint::from(point),));
            match called.and_then(|result| result.extract::<Pt>()) {
                Ok(next) => Ok(next.0),
                Err(error) => {
                    raised_by_fn = Some(error);
                    Err(vector::Error::Geometry {
                        verb: "map_points".to_owned(),
                        message: "the function raised".to_owned(),
                    })
                }
            }
        });
        match (mapped, raised_by_fn) {
            (_, Some(error)) => Err(error),
            (result, None) => Self::of(result),
        }
    }

    /// A drawing of this path filled with `color` (`"#rrggbb"`, `"#rrggbbaa"` or `"none"`).
    fn fill(&self, color: &str) -> PyResult<PyDrawing> {
        PyDrawing::of(self).fill(color)
    }

    /// A drawing of this path stroked with `color`; what is left out takes the defaults (2 pixels, round, round, 4).
    #[pyo3(signature = (color, *, width=None, join=None, cap=None, miter_limit=None))]
    fn stroke(
        &self,
        color: &str,
        width: Option<Num>,
        join: Option<&str>,
        cap: Option<&str>,
        miter_limit: Option<Num>,
    ) -> PyResult<PyDrawing> {
        PyDrawing::of(self).stroke(color, width, join, cap, miter_limit)
    }

    /// A drawing of this path with its opacity multiplied by `alpha`, from 0 to 1.
    fn opacity(&self, alpha: Num) -> PyResult<PyDrawing> {
        PyDrawing::of(self).opacity(alpha)
    }

    /// A drawing of this path painted over what is below it as `mode` (`"normal"` or `"multiply"`).
    fn blend(&self, mode: &str) -> PyResult<PyDrawing> {
        PyDrawing::of(self).blend(mode)
    }

    /// The path as SVG path data.
    fn to_svg_path(&self) -> String {
        self.0.to_svg_path()
    }

    /// Hashes by its path data, which writes equal paths alike.
    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        self.0.to_svg_path().into_pyobject(py)?.hash()
    }

    fn __repr__(&self) -> String {
        let data = self.0.to_svg_path();
        if data.is_empty() {
            "Path()".to_owned()
        } else {
            format!("Path.from_svg_path('{data}')")
        }
    }
}
