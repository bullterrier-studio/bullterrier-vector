//! `Shape` and `Style`: what a drawing draws, to read.

use crate::{PyPath, PyPoint};
use bullterrier_vector as vector;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// How a shape is painted, as it is drawn. Colours are `"#rrggbbaa"`, or `None` where nothing is painted.
#[pyclass(name = "Style", module = "bullterrier_vector", frozen, eq)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyStyle(vector::Style);

/// The text of a colour, or `None` where it paints nothing.
fn color_text(color: vector::Color) -> Option<String> {
    color.rgba().map(|_| color.to_string())
}

impl PyStyle {
    fn stroke_part<T>(&self, part: impl FnOnce(&vector::StrokeStyle) -> T) -> Option<T> {
        self.0.stroke().map(part)
    }
}

#[pymethods]
impl PyStyle {
    /// The fill colour, or `None`.
    #[getter]
    fn fill(&self) -> Option<String> {
        color_text(self.0.fill())
    }

    /// The stroke colour, or `None`.
    #[getter]
    fn stroke(&self) -> Option<String> {
        self.stroke_part(|stroke| color_text(stroke.color())).flatten()
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

    /// How it is painted over what is below (`"normal"` or `"multiply"`).
    #[getter]
    fn blend(&self) -> &'static str {
        self.0.blend().name()
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        (
            self.fill(),
            self.stroke(),
            self.stroke_width(),
            self.join(),
            self.cap(),
            self.miter_limit(),
            self.opacity(),
            self.blend(),
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

    /// A read-only mapping: `index` always, then what generators wrote (`glyph`, `char`, `word`, `line`, `part`,
    /// `copy`, `level`, `order`, `origin`).
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
        py.import("types")?.getattr("MappingProxyType")?.call1((found,))
    }
}
