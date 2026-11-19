//! `bullterrier_vector._native._core`: the Rust core for Python. Each method passes its arguments to the core
//! unchanged, so the core's range checks are the only ones and every language gets the same errors; this file only
//! converts between Python's values and the core's, and maps each core error to its exception.

mod branch;
mod drawing;
mod expr;
mod field;
mod halftone;
mod inputs;
mod mesh;
mod overlay;
mod select;
mod shape;
mod text;
mod trace;

use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};

use crate::drawing::{PyDrawing, refuse_expr, refuse_expr_in_pair};
use crate::expr::Num;
use crate::field::{PyFlow, Spacing, Tone, bands_of, hatch_of, rows_of, stipple_of};
use crate::halftone::{ShapeArg, ToneArg};
use crate::inputs::{Item, Pair};
use crate::shape::PaintArg;

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
    use super::expr::{PyExpr, PyParam, by_index, by_order, random, stagger};
    #[pymodule_export]
    use super::field::{PyField, PyFlow};
    #[pymodule_export]
    use super::halftone::PyHalftone;
    #[pymodule_export]
    use super::mesh::PyMesh;
    #[pymodule_export]
    use super::overlay::{characters, marks};
    #[pymodule_export]
    use super::shape::{PyGradient, PyShadow, PyShape, PyStyle, linear_gradient, radial_gradient};
    #[pymodule_export]
    use super::text::{PyFont, PyRun, run, text};
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

/// The two items of anything that iterates (a tuple, a list, a row of a numpy array), `None` unless there are exactly two.
pub(crate) fn two_items<'py>(object: &Bound<'py, PyAny>) -> Option<(Bound<'py, PyAny>, Bound<'py, PyAny>)> {
    let mut items = object.try_iter().ok()?;
    let pair = (items.next()?.ok()?, items.next()?.ok()?);
    items.next().is_none().then_some(pair)
}

/// A point as the caller gave it: a `Point` or a sequence of two numbers.
#[derive(Clone, Copy)]
struct Pt(vector::Point);

impl<'a, 'py> FromPyObject<'a, 'py> for Pt {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(point) = object.cast::<PyPoint>() {
            let PyPoint { x, y } = *point.get();
            return Ok(Self(vector::Point::new(x, y)));
        }
        if let Some((x, y)) = two_items(&object)
            && let (Ok(x), Ok(y)) = (x.extract::<f64>(), y.extract::<f64>())
        {
            return Ok(Self(vector::Point::new(x, y)));
        }
        refuse_expr_in_pair(&object)?;
        Err(PyTypeError::new_err(format!(
            "expected a Point or an (x, y) tuple of numbers, got {}",
            object.repr()?
        )))
    }
}

/// A frame as the caller gave it: a `Rect`, or a sequence of four numbers `x, y, width, height`.
#[derive(Clone, Copy)]
pub(crate) struct Frame(pub(crate) vector::Rect);

impl<'a, 'py> FromPyObject<'a, 'py> for Frame {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(rect) = object.cast::<PyRect>() {
            let PyRect { x, y, width, height } = *rect.get();
            return Ok(Self(vector::Rect { x, y, width, height }));
        }
        if let Ok(numbers) = object.extract::<Vec<f64>>()
            && let [x, y, width, height] = numbers[..]
        {
            return Ok(Self(vector::Rect { x, y, width, height }));
        }
        Err(PyTypeError::new_err(format!(
            "expected a Rect or four numbers (x, y, width, height), got {}",
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
pub(crate) struct PyRect {
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

impl From<vector::Rect> for PyRect {
    fn from(rect: vector::Rect) -> Self {
        Self {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        }
    }
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

/// The `tolerance` default of the boolean verbs: the core's `FLATNESS`, the flatness in design pixels that the pixels use
/// (the core keeps it private). The stub `_core.pyi` and the `text_signature`s write the literal.
// constant: the core's FLATNESS, a tolerance below any visible effect
pub(crate) const DEFAULT_TOLERANCE: f64 = 0.01;

/// The `miter_limit` default of `offset` and `outline`: the core's default stroke miter limit (the core keeps it private).
// constant: SVG's own default miter limit
pub(crate) const DEFAULT_MITER_LIMIT: f64 = 4.0;

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

    /// An open circular arc about `center` from the angle `start` to `end`, in degrees clockwise on the screen from the
    /// +x axis (`y` down, as for `rotate`); `end` below `start` runs counter-clockwise. The sweep is at most 360 degrees
    /// and not 0; 360 is a full circle that is still open (it ends where it starts).
    #[staticmethod]
    fn arc(center: Pt, radius: f64, start: f64, end: f64) -> PyResult<Self> {
        Self::of(vector::Path::arc(center.0, radius, start, end))
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
        raised(self.0.bounds()).map(PyRect::from)
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

    /// The sub-paths, each as a path of its own: an outline and its counters come apart.
    fn subpaths(&self) -> Vec<Self> {
        self.0.subpaths().into_iter().map(Self).collect()
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

    /// The pieces a dash `pattern` (`[on, off, ...]`, an odd one repeated) leaves; `phase` in cycles moves the dashes forward.
    #[pyo3(signature = (pattern, *, phase=0.0, near=None))]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a Python list is extracted into an owned Vec"
    )]
    fn dash(&self, pattern: Vec<f64>, phase: f64, near: Option<Pt>) -> PyResult<Self> {
        Self::of(self.0.dash(&pattern, phase, near.map(|near| near.0)))
    }

    /// Straight segments with corners `spacing` apart along the path, its ends kept.
    fn resample(&self, spacing: f64) -> PyResult<Self> {
        Self::of(self.0.resample(spacing))
    }

    /// The path with its points displaced sideways by a sine of `amplitude` pixels and one `wavelength` long travelling
    /// towards `angle` degrees (0: towards +x, displacing down the screen); `phase` in cycles. Edges are cut into pieces
    /// of a 24th of a wavelength at most, every vertex kept, so the result is made of straight lines.
    #[pyo3(signature = (*, amplitude=8.0, wavelength=120.0, angle=0.0, phase=0.0))]
    fn wave(&self, amplitude: f64, wavelength: f64, angle: f64, phase: f64) -> PyResult<Self> {
        Self::of(self.0.wave(amplitude, wavelength, angle, phase))
    }

    /// The path with its points displaced by smooth seeded noise: `x` and `y` each by at most `amplitude` pixels, the
    /// noise one `wavelength` per lattice cell; `phase` in cycles moves it, `seed` (a whole number) chooses it. Edges
    /// are cut into pieces of a 24th of a wavelength at most, every vertex kept, so the result is made of straight lines.
    #[pyo3(signature = (*, amplitude=4.0, wavelength=80.0, phase=0.0, seed=0.0))]
    fn wobble(&self, amplitude: f64, wavelength: f64, phase: f64, seed: f64) -> PyResult<Self> {
        Self::of(self.0.wobble(amplitude, wavelength, phase, seed))
    }

    /// The path with every point turned about `about` (the middle of the bounds by default) by `angle` degrees clockwise
    /// at the centre, less and less further out and not at all from `radius` on (the farthest corner of the bounds by
    /// default): the turn at distance `r` is `angle * (1 - s^2 (3 - 2s))`, `s = r / radius`. Points keep their distance
    /// to `about`. Edges are cut into pieces so that the result stays within a hundredth of the radius of the true
    /// twist (and within 0.05 design pixels of it), every vertex kept, so the result is made of straight lines.
    #[pyo3(signature = (angle=90.0, *, radius=None, about=None))]
    fn twist(&self, angle: f64, radius: Option<f64>, about: Option<Pt>) -> PyResult<Self> {
        Self::of(self.0.twist(angle, radius, about.map(|about| about.0)))
    }

    /// The path with every point moved along its ray from `about` (the middle of the bounds by default) from distance
    /// `r` to `r * (1 + amount * (1 - s^2 (3 - 2s)))`, `s = r / radius` (`radius`: the farthest corner of the bounds by
    /// default): a positive `amount` bulges the middle outward, a negative one pinches it in, and nothing at `radius` or
    /// beyond moves. `amount` must be above -1 and below 16/11, so that the shape does not fold over. The result is made
    /// of straight lines, every vertex kept.
    #[pyo3(signature = (amount=0.5, *, radius=None, about=None))]
    fn bulge(&self, amount: f64, radius: Option<f64>, about: Option<Pt>) -> PyResult<Self> {
        Self::of(self.0.bulge(amount, radius, about.map(|about| about.0)))
    }

    /// The frame (a `Rect` or `(x, y, width, height)`; the bounds of the path by default) mapped onto the region between
    /// the curves `top` and `bottom` (the frame's straight bottom edge by default): a point goes to the blend, by its share
    /// of the frame's height, of the points of the curves at its share of the frame's width, read as fractions of their
    /// lengths. `amount` blends each edge between the frame's own (0) and the curve (1). Edges are cut into pieces of at
    /// most a 256th of the frame's width, every vertex kept, so the result is made of straight lines.
    #[pyo3(signature = (top, bottom=None, *, frame=None, amount=1.0))]
    fn envelope(
        &self,
        top: &Bound<'_, Self>,
        bottom: Option<&Bound<'_, Self>>,
        frame: Option<Frame>,
        amount: f64,
    ) -> PyResult<Self> {
        Self::of(self.0.envelope(
            &top.get().0,
            bottom.map(|bottom| &bottom.get().0),
            frame.map(|frame| frame.0),
            amount,
        ))
    }

    /// The path arched: the whole band of the frame (a `Rect` or `(x, y, width, height)`; the bounds of the path by
    /// default) lifted by a parabola, its middle `bend` times the frame's width above the corners (below if negative),
    /// top and bottom edge alike, so the height is kept; columns stay vertical and `x` is kept.
    #[pyo3(signature = (bend, *, frame=None))]
    fn arch(&self, bend: f64, frame: Option<Frame>) -> PyResult<Self> {
        Self::of(self.0.arch(bend, frame.map(|frame| frame.0)))
    }

    /// The outline `fraction` of the way from this path to `target`: 0 is this path, 1 is `target`, any other finite
    /// number is the straight lerp (outside 0 to 1 it extrapolates). Closed contours pair by role (outer or hole) and
    /// place, each pair aligned at its start; a contour with no partner shrinks to, or grows from, a point, and a hole
    /// with no partner only takes away what the nonzero rule says and never paints outside the body. The result is made
    /// of straight lines; open sub-paths stay open.
    fn morph(&self, target: &Bound<'_, Self>, fraction: f64) -> PyResult<Self> {
        Self::of(self.0.morph(&target.get().0, fraction))
    }

    /// Evenly spaced lines along `flow` (default `Flow.angle(45)`) inside the region this path fills: `spacing` pixels apart,
    /// or a pair (spacing where `tone` is 0, where it is 1) read from `tone`, a `Field` or a number; the lines keep out of
    /// the region the path `avoid` fills and close where they come round to their own start. No two lines come within half
    /// a spacing. With `below`, lines only where the tone is at most that level (needs a `tone`; 0 is dark: passes at other
    /// angles and levels over each other crosshatch). A `Param` in `tone` or `below` is read at its default here; a
    /// `Drawing` step reads it with the values of the draw. The result is open lines, so stroke it.
    #[pyo3(signature = (spacing, *, tone=None, flow=None, avoid=None, below=None))]
    fn hatch(
        &self,
        spacing: Spacing,
        tone: Option<Tone>,
        flow: Option<&PyFlow>,
        avoid: Option<&Bound<'_, Self>>,
        below: Option<Num>,
    ) -> PyResult<Self> {
        let hatch = hatch_of(spacing, tone, flow, below)?;
        Self::of(self.0.hatch(&hatch, avoid.map(|path| &path.get().0)))
    }

    /// The parts of this path's lines that lie inside the region `region` fills: open lines cut where they cross its
    /// outline, closed ones kept whole when they are inside.
    fn within(&self, py: Python<'_>, region: &Bound<'_, Self>) -> PyResult<Self> {
        let region = region.get().0.clone();
        Self::of(py.detach(|| self.0.within(&region)))
    }

    /// Ridgelines inside the region this path fills: rows `spacing` pixels apart, each lifted towards the top of the picture by
    /// `amplitude` times `tone` (a `Field` or a number) at the point of the row, and cut where a row in front (lower in the
    /// picture) passes over it. The result is open lines, so stroke it. A `Param` is read at its default here.
    #[pyo3(signature = (spacing, *, tone, amplitude))]
    fn rows(&self, py: Python<'_>, spacing: Num, tone: Tone, amplitude: Num) -> PyResult<Self> {
        let rows = rows_of(spacing, tone, amplitude)?;
        Self::of(py.detach(|| self.0.rows(&rows)))
    }

    /// Lines whose width follows a tone, laid over the region this path fills and outlined as wide at each point as `tone`
    /// asks: `width` is `(width where the tone is 0, where it is 1)`. `family` is `"lines"` (along `angle` degrees), `"waves"` (`wavelength`
    /// long, `amplitude` high), `"rings"` or `"spiral"` round `centre` (default the middle of the bounds) or `"rays"` out of
    /// it. The result is closed outlines, so fill it. A `Param` is read at its default here.
    #[expect(clippy::too_many_arguments, reason = "the arguments of one Python call")]
    #[pyo3(signature = (spacing, *, tone, width, family="lines", angle=None, centre=None, wavelength=None, amplitude=None))]
    fn bands(
        &self,
        py: Python<'_>,
        spacing: Num,
        tone: Tone,
        width: (Num, Num),
        family: &str,
        angle: Option<Num>,
        centre: Option<Pt>,
        wavelength: Option<Num>,
        amplitude: Option<Num>,
    ) -> PyResult<Self> {
        let bands = bands_of(spacing, tone, width, family, angle, centre, wavelength, amplitude)?;
        Self::of(py.detach(|| self.0.bands(&bands)))
    }

    /// Round dots on a square lattice `spacing` apart inside the region this path fills, one at most per cell; `radius` is
    /// pixels, one number or `(radius where the tone is 0, where it is 1)` read at each dot's centre; `density` is `(share of cells with a dot where the tone is 0, where it is 1)`, default `(1, 0)`: dark is
    /// dense. `seed` moves them. The result is closed dots, so fill it.
    #[pyo3(signature = (spacing, *, tone, radius, density=None, seed=None))]
    fn stipple(
        &self,
        py: Python<'_>,
        spacing: Num,
        tone: Tone,
        radius: crate::inputs::By,
        density: Option<(Num, Num)>,
        seed: Option<Num>,
    ) -> PyResult<Self> {
        let stipple = stipple_of(spacing, tone, radius, density, seed)?;
        Self::of(py.detach(|| self.0.stipple(&stipple)))
    }

    /// Few smooth curves within `tolerance` pixels of the path, sharp corners (turns past 45 degrees) kept.
    #[pyo3(signature = (*, tolerance=0.5))]
    fn simplify(&self, tolerance: f64) -> PyResult<Self> {
        Self::of(self.0.simplify(tolerance))
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

    /// Carried along `path`: a point `(x, y)` goes `start * length + x` along it and `y` to its right.
    #[pyo3(signature = (path, *, start=Plain(0.0), near=None), text_signature = "($self, path, *, start=0.0, near=None)")]
    fn along(&self, path: &Bound<'_, Self>, start: Plain, near: Option<Pt>) -> PyResult<Self> {
        Self::of(self.0.along(&path.get().0, start.0, near.map(|near| near.0)))
    }

    /// This path with its overlaps removed: the closed polygons the nonzero rule fills, in canonical form (outers
    /// clockwise, holes counter-clockwise, y down). Curves become straight edges within `tolerance` design pixels.
    #[pyo3(signature = (*, tolerance=Plain(DEFAULT_TOLERANCE)), text_signature = "($self, *, tolerance=0.01)")]
    fn resolve(&self, tolerance: Plain) -> PyResult<Self> {
        Self::of(self.0.resolve(tolerance.0))
    }

    /// The region either path fills, as canonical straight-edged polygons within `tolerance` design pixels.
    #[pyo3(signature = (other, *, tolerance=Plain(DEFAULT_TOLERANCE)), text_signature = "($self, other, *, tolerance=0.01)")]
    fn union(&self, other: &Bound<'_, Self>, tolerance: Plain) -> PyResult<Self> {
        Self::of(self.0.union(&other.get().0, tolerance.0))
    }

    /// The region this path fills and `other` does not; as `union` otherwise.
    #[pyo3(signature = (other, *, tolerance=Plain(DEFAULT_TOLERANCE)), text_signature = "($self, other, *, tolerance=0.01)")]
    fn subtract(&self, other: &Bound<'_, Self>, tolerance: Plain) -> PyResult<Self> {
        Self::of(self.0.subtract(&other.get().0, tolerance.0))
    }

    /// The region both paths fill (an empty path if none); as `union` otherwise.
    #[pyo3(signature = (other, *, tolerance=Plain(DEFAULT_TOLERANCE)), text_signature = "($self, other, *, tolerance=0.01)")]
    fn intersect(&self, other: &Bound<'_, Self>, tolerance: Plain) -> PyResult<Self> {
        Self::of(self.0.intersect(&other.get().0, tolerance.0))
    }

    /// The region only one of the paths fills; as `union` otherwise.
    #[pyo3(signature = (other, *, tolerance=Plain(DEFAULT_TOLERANCE)), text_signature = "($self, other, *, tolerance=0.01)")]
    fn exclude(&self, other: &Bound<'_, Self>, tolerance: Plain) -> PyResult<Self> {
        Self::of(self.0.exclude(&other.get().0, tolerance.0))
    }

    /// The region this path fills moved outward by `distance`, inward when negative, as canonical straight-edged
    /// polygons; `join` is `"round"`, `"miter"` or `"bevel"`, a miter sharper than `miter_limit` is clipped flat.
    #[pyo3(
        signature = (distance, *, join="round", miter_limit=Plain(DEFAULT_MITER_LIMIT), tolerance=Plain(DEFAULT_TOLERANCE)),
        text_signature = "($self, distance, *, join='round', miter_limit=4, tolerance=0.01)"
    )]
    fn offset(&self, distance: Plain, join: &str, miter_limit: Plain, tolerance: Plain) -> PyResult<Self> {
        let join = raised(vector::Join::parse(join))?;
        Self::of(self.0.offset(distance.0, join, miter_limit.0, tolerance.0))
    }

    /// The region a stroke of `width` along this path paints, as a closed outline; `cap` is `"butt"`, `"round"` or
    /// `"square"`; `join` and `miter_limit` as for `offset`. `profile`, a list of `(position, factor)` pairs, varies
    /// the width along each sub-path (position 0 to 1 by arc length, factors multiply `width`, linear between stops,
    /// the end values held): a taper is `[(0, 0), (0.15, 1), (0.85, 1), (1, 0)]`. Joins and caps are then round.
    #[pyo3(
        signature = (width, *, profile=None, join="round", cap="round", miter_limit=Plain(DEFAULT_MITER_LIMIT), tolerance=Plain(DEFAULT_TOLERANCE)),
        text_signature = "($self, width, *, profile=None, join='round', cap='round', miter_limit=4, tolerance=0.01)"
    )]
    fn outline(
        &self,
        width: Plain,
        profile: Option<Vec<Pt>>,
        join: &str,
        cap: &str,
        miter_limit: Plain,
        tolerance: Plain,
    ) -> PyResult<Self> {
        let join = raised(vector::Join::parse(join))?;
        let cap = raised(vector::Cap::parse(cap))?;
        if let Some(profile) = profile {
            raised(vector::require_round_stroke(join, cap))?;
            let stops: Vec<(f64, f64)> = profile.iter().map(|stop| (stop.0.x, stop.0.y)).collect();
            return Self::of(self.0.tapered_outline(width.0, &stops, tolerance.0));
        }
        Self::of(self.0.outline(width.0, join, cap, miter_limit.0, tolerance.0))
    }

    /// The region the region this path fills covers as it moves `dx` to the right and `dy` down in a straight line, as
    /// canonical straight-edged polygons within `tolerance` design pixels; a hole it crosses fills in.
    #[pyo3(signature = (dx, dy, *, tolerance=Plain(DEFAULT_TOLERANCE)), text_signature = "($self, dx, dy, *, tolerance=0.01)")]
    fn sweep(&self, dx: Plain, dy: Plain, tolerance: Plain) -> PyResult<Self> {
        Self::of(self.0.sweep(dx.0, dy.0, tolerance.0))
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

    /// A drawing of this path filled with `color` (`"#rgb"`, `"#rrggbb"`, `"#rrggbbaa"` or `"none"`) or a `Gradient`.
    fn fill(&self, color: PaintArg) -> PyResult<PyDrawing> {
        PyDrawing::of(self).fill(color)
    }

    /// A drawing of this path stroked with `color` or a `Gradient`; what is left out takes the defaults (2 pixels, round,
    /// round, 4). `profile` is a list of `(position, factor)` pairs, as for `Drawing.stroke`.
    #[pyo3(signature = (color, *, width=None, join=None, cap=None, miter_limit=None, profile=None))]
    fn stroke(
        &self,
        color: PaintArg,
        width: Option<Num>,
        join: Option<&str>,
        cap: Option<&str>,
        miter_limit: Option<Num>,
        profile: Option<Vec<Pair>>,
    ) -> PyResult<PyDrawing> {
        PyDrawing::of(self).stroke(color, width, join, cap, miter_limit, profile)
    }

    /// A drawing of this path with its opacity multiplied by `alpha`, from 0 to 1.
    fn opacity(&self, alpha: Num) -> PyResult<PyDrawing> {
        PyDrawing::of(self).opacity(alpha)
    }

    /// A drawing of this path painted over what is below it as `mode` (a CSS blend name: `"normal"`, `"multiply"`, `"screen"`, `"overlay"`, `"darken"`, `"lighten"`, `"color-dodge"`, `"color-burn"`, `"hard-light"`, `"soft-light"`, `"difference"` or `"exclusion"`).
    fn blend(&self, mode: &str) -> PyResult<PyDrawing> {
        PyDrawing::of(self).blend(mode)
    }

    /// A drawing of this path filled with a screen of dots of `color`; see `Drawing.halftone`.
    #[expect(clippy::too_many_arguments, reason = "the arguments of one Python call")]
    #[pyo3(signature = (color, *, tone, pitch=None, angle=None, shape=None, aspect=None, screen=None, centre=None, amplitude=None, wavelength=None))]
    fn halftone(
        &self,
        color: &str,
        tone: ToneArg,
        pitch: Option<Num>,
        angle: Option<Num>,
        shape: Option<ShapeArg>,
        aspect: Option<Num>,
        screen: Option<&str>,
        centre: Option<Pt>,
        amplitude: Option<Num>,
        wavelength: Option<Num>,
    ) -> PyResult<PyDrawing> {
        PyDrawing::of(self).halftone(
            color, tone, pitch, angle, shape, aspect, screen, centre, amplitude, wavelength,
        )
    }

    /// A drawing of this path as a knockout: it removes what is below it in a drawing instead of painting.
    fn knockout(&self) -> PyResult<PyDrawing> {
        PyDrawing::of(self).knockout()
    }

    /// A drawing of this path cut to the region `region` (a Path, a Shape or a Drawing) fills.
    fn clip(&self, region: Item) -> PyResult<PyDrawing> {
        PyDrawing::of(self).clip(region)
    }

    /// A drawing of this path faded by the luminance of `source` (a Path, a Shape or a Drawing) as painted.
    fn mask(&self, source: Item) -> PyResult<PyDrawing> {
        PyDrawing::of(self).mask(source)
    }

    /// A drawing of this path with a drop shadow behind it: `dx` and `dy` design pixels from it (0 and 8 where left
    /// out), blurred by `blur` design pixels (16; 0 is a hard copy) and tinted `color` (`"#00000059"`).
    #[pyo3(signature = (*, dx=None, dy=None, blur=None, color=None))]
    fn shadow(&self, dx: Option<Num>, dy: Option<Num>, blur: Option<Num>, color: Option<&str>) -> PyResult<PyDrawing> {
        PyDrawing::of(self).shadow(dx, dy, blur, color)
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
