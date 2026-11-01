//! `Font` and `text`: type as geometry.

use crate::drawing::{PyDrawing, checked};
use crate::expr::{Num, PyParam};
use crate::{ParameterError, Pt, raised};
use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedBytes;
use pyo3::types::{PyDict, PyMapping, PyMappingMethods, PyString};

/// A TrueType or OpenType font, read from its bytes once. A font is its bytes: two fonts are the same when their bytes
/// are, whatever name they carry.
#[pyclass(name = "Font", module = "bullterrier_vector", frozen)]
#[derive(Clone)]
pub(crate) struct PyFont(pub(crate) vector::Font);

#[pymethods]
impl PyFont {
    /// The font in `data`, the bytes of a `.ttf` or `.otf` file. `name` is what a saved drawing calls it.
    #[new]
    #[pyo3(signature = (data, *, name=None))]
    fn new(data: &Bound<'_, PyAny>, name: Option<&str>) -> PyResult<Self> {
        let font = raised(vector::Font::new(bytes_of("data", data)?.to_vec()))?;
        match name {
            Some(name) => Ok(Self(raised(font.named(name))?)),
            None => Ok(Self(font)),
        }
    }

    /// The name a saved drawing calls the font, or `None`.
    #[getter]
    fn name(&self) -> Option<&str> {
        self.0.name()
    }

    /// The variation axes by tag, in the font's order: a new dict of `Param`s with the font's own range and default.
    #[getter]
    fn axes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let axes = PyDict::new(py);
        for param in self.0.axes() {
            axes.set_item(param.name().to_owned(), PyParam::object(py, param)?)?;
        }
        Ok(axes)
    }

    /// The tags of the OpenType features the font lists, sorted.
    #[getter]
    fn features(&self) -> Vec<String> {
        self.0.features()
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let name = self.0.name().into_pyobject(py)?.repr()?;
        Ok(format!("Font(name={name}, axes={:?})", self.axes(py)?.keys()))
    }
}

/// The bytes of a `bytes` or `bytearray` argument called `what`.
pub(crate) fn bytes_of(what: &str, value: &Bound<'_, PyAny>) -> PyResult<PyBackedBytes> {
    match value.extract::<PyBackedBytes>() {
        Ok(bytes) => Ok(bytes),
        Err(_) => Err(PyTypeError::new_err(format!(
            "{what}: expected bytes, got a {}",
            value.get_type().name()?
        ))),
    }
}

/// The entries of a mapping of any kind (a `dict` or not), as the stub promises.
pub(crate) fn entries<'py>(mapping: &Bound<'py, PyMapping>) -> PyResult<Vec<(Bound<'py, PyAny>, Bound<'py, PyAny>)>> {
    mapping.items()?.iter().map(|entry| entry.extract()).collect()
}

/// The whole number of a feature: any Python int in range.
fn feature(tag: &str, value: &Bound<'_, PyAny>) -> PyResult<u32> {
    let Ok(number) = value.extract::<i128>() else {
        return Err(PyTypeError::new_err(format!(
            "features: {tag:?} expected a whole number, got {}",
            value.repr()?
        )));
    };
    u32::try_from(number).map_err(|_| {
        ParameterError::new_err(format!(
            "features: {tag:?} must be a whole number from 0 to {}, got {number}",
            u32::MAX
        ))
    })
}

/// A drawing of `content` in `font`: one shape for every glyph that has an outline (a space or a line break has none),
/// shaped with kerning, ligatures and the rules of the script. `"\n"` breaks a line. What is left out takes the
/// defaults of the core: `size` 100, `tracking` 0, `leading` 1.2 times the size, `align` `"left"`, `anchor` `(0, 0)`.
#[pyfunction]
#[pyo3(signature = (content, *, font, size=None, axes=None, features=None, tracking=None, leading=None, align=None, anchor=None))]
#[expect(clippy::too_many_arguments, reason = "the arguments of the Python call, one for one")]
pub(crate) fn text(
    content: &Bound<'_, PyString>,
    font: &Bound<'_, PyFont>,
    size: Option<Num>,
    axes: Option<&Bound<'_, PyMapping>>,
    features: Option<&Bound<'_, PyMapping>>,
    tracking: Option<Num>,
    leading: Option<Num>,
    align: Option<&str>,
    anchor: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyDrawing> {
    // a `str` holding a lone surrogate has no UTF-8 form: that is a problem in the argument, so a `ParameterError`
    let content = content
        .to_str()
        .map_err(|_| ParameterError::new_err("content: the text is not valid UTF-8 (it holds a lone surrogate)"))?;
    let mut options = vector::Text::new(&font.get().0);
    if let Some(size) = size {
        options = options.size(size.0);
    }
    if let Some(tracking) = tracking {
        options = options.tracking(tracking.0);
    }
    if let Some(leading) = leading {
        options = options.leading(leading.0);
    }
    if let Some(align) = align {
        options = options.align(raised(vector::Align::parse(align))?);
    }
    if let Some(anchor) = anchor {
        // the anchor is read once and as plain numbers: a Param or an expression is refused here, not mis-described
        let Ok(Pt(anchor)) = anchor.extract::<Pt>() else {
            return Err(PyTypeError::new_err(format!(
                "anchor: expected a Point or an (x, y) tuple of plain numbers, got {}",
                anchor.repr()?
            )));
        };
        options = options.anchor(anchor);
    }
    if let Some(axes) = axes {
        let axes = entries(axes)?
            .into_iter()
            .map(|(tag, value)| Ok((tag.extract::<String>()?, value.extract::<Num>()?.0)))
            .collect::<PyResult<Vec<_>>>()?;
        options = options.axes(axes);
    }
    if let Some(features) = features {
        let features = entries(features)?
            .into_iter()
            .map(|(tag, value)| {
                let tag = tag.extract::<String>()?;
                let value = feature(&tag, &value)?;
                Ok((tag, value))
            })
            .collect::<PyResult<Vec<_>>>()?;
        options = options.features(features);
    }
    checked(vector::text(content, options))
}
