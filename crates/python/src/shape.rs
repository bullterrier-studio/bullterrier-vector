//! `Shape` and `Style`: what a drawing draws, to read.

use crate::{Pt, PyPath, PyPoint, raised};
use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// How a shape is painted, as it is drawn. Colours are `"#rrggbbaa"`, or `None` where nothing is painted; a fill or
/// stroke that is a gradient is a `Gradient`.
#[pyclass(name = "Style", module = "bullterrier_vector", frozen, eq)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyStyle(vector::Style);

/// A gradient paint for a fill or a stroke: `linear_gradient` or `radial_gradient` make one. Its geometry is in the
/// design frame, not in the shape it paints; beyond its end stops it holds their colours.
#[pyclass(name = "Gradient", module = "bullterrier_vector", frozen, eq)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyGradient(vector::Gradient);

#[pymethods]
impl PyGradient {
    /// `"linear"` or `"radial"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.0.kind() {
            vector::GradientKind::Linear { .. } => "linear",
            vector::GradientKind::Radial { .. } => "radial",
        }
    }

    /// The stops as `(offset, "#rrggbbaa")`, in order of offset.
    #[getter]
    fn stops(&self) -> Vec<(f64, String)> {
        self.0
            .stops()
            .iter()
            .map(|(offset, [red, green, blue, alpha])| (*offset, format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}")))
            .collect()
    }

    /// Where offset 0 lies on a linear gradient, else `None`.
    #[getter]
    fn start(&self) -> Option<PyPoint> {
        match self.0.kind() {
            vector::GradientKind::Linear { start, .. } => Some(start.into()),
            vector::GradientKind::Radial { .. } => None,
        }
    }

    /// Where offset 1 lies on a linear gradient, else `None`.
    #[getter]
    fn end(&self) -> Option<PyPoint> {
        match self.0.kind() {
            vector::GradientKind::Linear { end, .. } => Some(end.into()),
            vector::GradientKind::Radial { .. } => None,
        }
    }

    /// The centre of a radial gradient, else `None`.
    #[getter]
    fn center(&self) -> Option<PyPoint> {
        match self.0.kind() {
            vector::GradientKind::Radial { center, .. } => Some(center.into()),
            vector::GradientKind::Linear { .. } => None,
        }
    }

    /// The distance of offset 1 on a radial gradient, else `None`.
    #[getter]
    fn radius(&self) -> Option<f64> {
        match self.0.kind() {
            vector::GradientKind::Radial { radius, .. } => Some(radius),
            vector::GradientKind::Linear { .. } => None,
        }
    }

    fn __repr__(&self) -> String {
        let point = |point: vector::Point| format!("({:?}, {:?})", point.x, point.y);
        let stops = self
            .stops()
            .iter()
            .map(|(offset, color)| format!("({offset:?}, '{color}')"))
            .collect::<Vec<_>>()
            .join(", ");
        match self.0.kind() {
            vector::GradientKind::Linear { start, end } => {
                format!("linear_gradient([{stops}], start={}, end={})", point(start), point(end))
            }
            vector::GradientKind::Radial { center, radius } => {
                format!(
                    "radial_gradient([{stops}], center={}, radius={radius:?})",
                    point(center)
                )
            }
        }
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        self.__repr__().into_pyobject(py)?.hash()
    }
}

/// A linear gradient along the line from `start` (offset 0) to `end` (offset 1), constant across it, with `stops` as
/// `(offset, color)`: at least 2, offsets 0 to 1 that do not decrease, colours `"#rrggbb"` or `"#rrggbbaa"`. Beyond
/// the line it holds the end colours. Its geometry is in the design frame, so set the paint after the transforms.
#[pyfunction]
#[pyo3(signature = (stops, *, start, end))]
#[expect(clippy::needless_pass_by_value, reason = "pyo3 extracts the list as an owned Vec")]
pub(crate) fn linear_gradient(stops: Vec<(f64, String)>, start: Pt, end: Pt) -> PyResult<PyGradient> {
    raised(vector::Gradient::linear(start.0, end.0, colored(&stops)?)).map(PyGradient)
}

/// A radial gradient around `center`: offset 0 at the centre, offset 1 at `radius`, then the end colour. `stops` as for
/// `linear_gradient`.
#[pyfunction]
#[pyo3(signature = (stops, *, center, radius))]
#[expect(clippy::needless_pass_by_value, reason = "pyo3 extracts the list as an owned Vec")]
pub(crate) fn radial_gradient(stops: Vec<(f64, String)>, center: Pt, radius: f64) -> PyResult<PyGradient> {
    raised(vector::Gradient::radial(center.0, radius, colored(&stops)?)).map(PyGradient)
}

/// The stops with their colours read.
fn colored(stops: &[(f64, String)]) -> PyResult<Vec<(f64, vector::Color)>> {
    stops
        .iter()
        .map(|(offset, text)| Ok((*offset, raised(vector::Color::parse(text))?)))
        .collect()
}

/// A fill or stroke as the caller gave it: a colour text or a `Gradient`.
pub(crate) struct PaintArg(pub(crate) vector::Paint);

impl<'a, 'py> FromPyObject<'a, 'py> for PaintArg {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(gradient) = object.cast::<PyGradient>() {
            return Ok(Self(gradient.get().0.clone().into()));
        }
        match object.extract::<String>() {
            Ok(text) => Ok(Self(raised(vector::Color::parse(&text))?.into())),
            Err(_) => Err(PyTypeError::new_err(format!(
                "expected a colour text (\"#rrggbb\", \"#rrggbbaa\" or \"none\") or a Gradient, got {}",
                object.repr()?
            ))),
        }
    }
}

/// A drop shadow, as it is drawn: the offset and blur in design pixels, and the colour as `"#rrggbbaa"`.
#[pyclass(name = "Shadow", module = "bullterrier_vector", frozen, eq)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyShadow(vector::ShadowStyle);

#[pymethods]
impl PyShadow {
    /// The offset to the right in design pixels.
    #[getter]
    fn dx(&self) -> f64 {
        self.0.dx()
    }

    /// The offset down in design pixels.
    #[getter]
    fn dy(&self) -> f64 {
        self.0.dy()
    }

    /// The blur radius in design pixels (the Gaussian's deviation is half of it); 0 is a hard copy.
    #[getter]
    fn blur(&self) -> f64 {
        self.0.blur()
    }

    /// The colour the silhouette is tinted, `"#rrggbbaa"`.
    #[getter]
    fn color(&self) -> String {
        self.0.color().to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "Shadow(dx={:?}, dy={:?}, blur={:?}, color='{}')",
            self.dx(),
            self.dy(),
            self.blur(),
            self.color()
        )
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        (self.dx(), self.dy(), self.blur(), self.color())
            .into_pyobject(py)?
            .hash()
    }
}

/// A halftone fill, as it is drawn: a screen of dots in a colour (`"#rrggbbaa"`), the tone (0 solid, 1 no dots), the pitch
/// in design pixels and the angle in degrees of the lattice.
#[pyclass(name = "Halftone", module = "bullterrier_vector", frozen, eq)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyHalftone(vector::HalftoneStyle);

#[pymethods]
impl PyHalftone {
    /// The colour of the dots, `"#rrggbbaa"`.
    #[getter]
    fn color(&self) -> String {
        self.0.color().to_string()
    }

    /// The share of each lattice cell the dot leaves bare, 0 (solid) to 1 (no dots).
    #[getter]
    fn tone(&self) -> f64 {
        self.0.tone()
    }

    /// The distance between neighbouring dots in design pixels.
    #[getter]
    fn pitch(&self) -> f64 {
        self.0.pitch()
    }

    /// The turn of the lattice in degrees, clockwise on the screen about the origin of the design frame.
    #[getter]
    fn angle(&self) -> f64 {
        self.0.angle()
    }

    fn __repr__(&self) -> String {
        format!(
            "Halftone(color='{}', tone={:?}, pitch={:?}, angle={:?})",
            self.color(),
            self.tone(),
            self.pitch(),
            self.angle()
        )
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        (self.color(), self.tone(), self.pitch(), self.angle())
            .into_pyobject(py)?
            .hash()
    }
}

/// What a fill or stroke is in Python: the colour text, a `Gradient`, or `None` where nothing is painted.
fn paint_object(py: Python<'_>, paint: &vector::Paint) -> PyResult<Option<Py<PyAny>>> {
    Ok(match paint {
        vector::Paint::Color(color) => match color.rgba() {
            Some(_) => Some(color.to_string().into_pyobject(py)?.into_any().unbind()),
            None => None,
        },
        vector::Paint::Gradient(gradient) => Some(Bound::new(py, PyGradient(gradient.clone()))?.into_any().unbind()),
    })
}

/// The repr of a fill or stroke: the colour text quoted, a gradient's call, or `None`.
fn paint_repr(paint: &vector::Paint) -> String {
    match paint {
        vector::Paint::Color(color) => color.rgba().map_or_else(|| "None".to_owned(), |_| format!("'{color}'")),
        vector::Paint::Gradient(gradient) => PyGradient(gradient.clone()).__repr__(),
    }
}

impl PyStyle {
    fn stroke_part<T>(&self, part: impl FnOnce(&vector::StrokeStyle) -> T) -> Option<T> {
        self.0.stroke().map(part)
    }
}

#[pymethods]
impl PyStyle {
    /// The fill: a colour `"#rrggbbaa"`, a `Gradient`, or `None`.
    #[getter]
    fn fill(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        paint_object(py, self.0.fill())
    }

    /// The stroke: a colour `"#rrggbbaa"`, a `Gradient`, or `None`.
    #[getter]
    fn stroke(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.0
            .stroke()
            .map_or(Ok(None), |stroke| paint_object(py, stroke.paint()))
    }

    /// The stroke width in design pixels, or `None` where there is no stroke.
    #[getter]
    fn stroke_width(&self) -> Option<f64> {
        self.stroke_part(vector::StrokeStyle::width)
    }

    /// How the corners of the stroke are joined (`"round"`, `"miter"`, `"bevel"`), or `None`.
    #[getter]
    fn join(&self) -> Option<&'static str> {
        self.stroke_part(|stroke| stroke.join().name())
    }

    /// How the ends of an open stroke are finished (`"butt"`, `"round"`, `"square"`), or `None`.
    #[getter]
    fn cap(&self) -> Option<&'static str> {
        self.stroke_part(|stroke| stroke.cap().name())
    }

    /// The ratio at which a mitred join is cut to a bevel, or `None`.
    #[getter]
    fn miter_limit(&self) -> Option<f64> {
        self.stroke_part(vector::StrokeStyle::miter_limit)
    }

    /// The opacity, 0 to 1.
    #[getter]
    fn opacity(&self) -> f64 {
        self.0.opacity()
    }

    /// How it is painted over what is below (a CSS blend name such as `"multiply"`, `"screen"` or `"color-dodge"`).
    #[getter]
    fn blend(&self) -> &'static str {
        self.0.blend().name()
    }

    /// The drop shadow behind the shape, or `None`.
    #[getter]
    fn shadow(&self) -> Option<PyShadow> {
        self.0.shadow().copied().map(PyShadow)
    }

    /// The screen of dots the shape is filled with, or `None` (`fill` is then `None` too).
    #[getter]
    fn halftone(&self) -> Option<PyHalftone> {
        self.0.halftone().copied().map(PyHalftone)
    }

    /// Whether the shape removes what is below it in its drawing instead of painting.
    #[getter]
    fn knockout(&self) -> bool {
        self.0.knockout()
    }

    /// The region the shape and its shadow are cut to, or `None`.
    #[getter]
    fn clip(&self) -> Option<PyPath> {
        self.0.clip().cloned().map(PyPath)
    }

    /// The shapes whose luminance sets the opacity of the shape and its shadow, or `None`.
    #[getter]
    fn mask(&self) -> Option<Vec<PyShape>> {
        self.0
            .mask()
            .map(|shapes| shapes.iter().cloned().map(PyShape).collect())
    }

    fn __repr__(&self) -> String {
        let number = |value: Option<f64>| value.map_or_else(|| "None".to_owned(), |value| format!("{value:?}"));
        format!(
            "Style(fill={}, halftone={}, stroke={}, stroke_width={}, opacity={:?}, blend='{}', shadow={}, knockout={}, clip={}, mask={})",
            paint_repr(self.0.fill()),
            self.halftone()
                .map_or_else(|| "None".to_owned(), |halftone| halftone.__repr__()),
            self.0
                .stroke()
                .map_or_else(|| "None".to_owned(), |stroke| paint_repr(stroke.paint())),
            number(self.stroke_width()),
            self.opacity(),
            self.blend(),
            self.shadow()
                .map_or_else(|| "None".to_owned(), |shadow| shadow.__repr__()),
            if self.knockout() { "True" } else { "False" },
            self.clip().map_or_else(|| "None".to_owned(), |clip| clip.__repr__()),
            self.0
                .mask()
                .map_or_else(|| "None".to_owned(), |shapes| format!("<{} shapes>", shapes.len())),
        )
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        (
            paint_repr(self.0.fill()),
            self.halftone(),
            self.0.stroke().map(|stroke| paint_repr(stroke.paint())),
            self.stroke_width(),
            self.join(),
            self.cap(),
            self.miter_limit(),
            self.opacity(),
            self.blend(),
            self.shadow(),
            self.knockout(),
            (
                self.clip(),
                // the paths of the shapes tell masks apart well enough: equal masks have equal paths
                self.0.mask().map(|shapes| {
                    shapes
                        .iter()
                        .map(|shape| PyPath(shape.path().clone()))
                        .collect::<Vec<_>>()
                }),
            ),
        )
            .into_pyobject(py)?
            .hash()
    }
}

/// One outline with the style it is painted in and the attributes its generators wrote: what `Drawing.shapes()`
/// returns, in design pixels. A shape is read, never built by hand; `Drawing(*shapes)` puts shapes back.
#[pyclass(name = "Shape", module = "bullterrier_vector", frozen, eq)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyShape(pub(crate) vector::Shape);

#[pymethods]
impl PyShape {
    /// The outline.
    #[getter]
    fn path(&self) -> PyPath {
        PyPath(self.0.path().clone())
    }

    /// The style as drawn.
    #[getter]
    fn style(&self) -> PyStyle {
        PyStyle(self.0.style().clone())
    }

    fn __repr__(&self) -> String {
        format!(
            "Shape(index={}, path={}, style={})",
            self.0.index(),
            PyPath(self.0.path().clone()).__repr__(),
            PyStyle(self.0.style().clone()).__repr__(),
        )
    }

    /// A read-only mapping: `index` always, then what generators wrote (`glyph`, `char`, `word`, `line`, `part`,
    /// `copy`, `level`, `order`, `origin`, and `id`, `class`, `group` from an SVG import).
    #[getter]
    fn attributes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let found = PyDict::new(py);
        let attributes = self.0.attributes();
        found.set_item("index", self.0.index())?;
        if let Some(glyph) = attributes.glyph() {
            found.set_item("glyph", glyph)?;
        }
        if let Some(character) = attributes.char() {
            found.set_item("char", character)?;
        }
        if let Some(word) = attributes.word() {
            found.set_item("word", word)?;
        }
        if let Some(line) = attributes.line() {
            found.set_item("line", line)?;
        }
        if let Some(part) = attributes.part() {
            found.set_item("part", part)?;
        }
        if let Some(copy) = attributes.copy() {
            found.set_item("copy", copy)?;
        }
        if let Some(level) = attributes.level() {
            found.set_item("level", level)?;
        }
        if let Some(order) = attributes.order() {
            found.set_item("order", order)?;
        }
        if let Some(origin) = attributes.origin() {
            found.set_item("origin", PyPoint::from(origin))?;
        }
        for (name, text) in [
            ("id", attributes.id()),
            ("class", attributes.class()),
            ("group", attributes.group()),
        ] {
            if let Some(text) = text {
                found.set_item(name, text)?;
            }
        }
        py.import("types")?.getattr("MappingProxyType")?.call1((found,))
    }
}
