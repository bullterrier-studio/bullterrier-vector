//! `Field` and `Flow`: a number and a direction at every point, the inputs of `hatch`.

use crate::drawing::Item;
use crate::expr::Num;
use crate::{Frame, Pt, PyPath, raised};
use bullterrier_vector as vector;
use pyo3::buffer::PyBuffer;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedBytes;
use pyo3::types::PyBytes;

/// The bytes of a map the caller gave: `bytes`, a `bytearray`, anything with the buffer protocol of bytes (a
/// `memoryview`, a contiguous `numpy` array of `uint8`) or a list of whole numbers from 0 to 255.
fn map_bytes(data: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    if let Ok(bytes) = data.extract::<PyBackedBytes>() {
        return Ok(bytes.to_vec());
    }
    if let Ok(buffer) = PyBuffer::<u8>::get(data) {
        return buffer
            .to_vec(data.py())
            .map_err(|_| PyTypeError::new_err("data: the buffer must be contiguous 8-bit values"));
    }
    data.extract::<Vec<u8>>().map_err(|_| {
        PyTypeError::new_err(format!(
            "data: expected bytes, a buffer of 8-bit values or a list of numbers from 0 to 255, got a {}",
            data.get_type()
                .name()
                .map_or_else(|_| String::new(), |name| name.to_string())
        ))
    })
}

/// Levels as the caller gave them: one number or a sequence of numbers.
struct Levels(Vec<f64>);

impl<'a, 'py> FromPyObject<'a, 'py> for Levels {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(one) = object.extract::<f64>() {
            return Ok(Self(vec![one]));
        }
        object.extract::<Vec<f64>>().map(Self).map_err(|_| {
            PyTypeError::new_err(format!(
                "expected a number or a list of numbers, got {}",
                object.repr().map_or_else(|_| String::new(), |text| text.to_string())
            ))
        })
    }
}

/// A number at every point: a ramp along a line or the distance to a path's outline. `hatch` reads one as its `tone`.
#[pyclass(name = "Field", module = "bullterrier_vector", frozen, eq)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyField(pub(crate) vector::Field);

#[pymethods]
impl PyField {
    /// 0 at `start`, 1 at `end`, linear along the line between and held beyond it.
    #[staticmethod]
    fn ramp(start: Pt, end: Pt) -> PyResult<Self> {
        raised(vector::Field::ramp(start.0, end.0)).map(Self)
    }

    /// The distance to the outline of `drawing` (a Path, a Shape or a Drawing: all its shapes together, drawn with the
    /// values of the step that reads the field) in pixels, negative inside. With `normalized`, inside each part 0 on its
    /// outer edge rising to 1 on its holes' edges (or at its deepest point where it has none); 0 outside.
    #[staticmethod]
    #[pyo3(signature = (drawing, *, normalized=false))]
    fn distance(drawing: Item, normalized: bool) -> PyResult<Self> {
        raised(vector::Field::distance(drawing.0, normalized)).map(Self)
    }

    /// An 8-bit grayscale map as a field, 0 (black) to 1 (white): `data` holds `height` rows of `width` bytes, top row
    /// first (`bytes`, a `bytearray`, a `memoryview`, a `numpy` array of `uint8`, or a list of numbers). It lies over
    /// `box` (`(x, y, width, height)`, default one design pixel a map pixel from the origin); the field is bilinear
    /// between pixel centres and holds the nearest edge value beyond the box. `name` is what a saved drawing calls it.
    #[staticmethod]
    #[pyo3(signature = (width, height, data, *, r#box=None, name=None))]
    fn map(
        width: usize,
        height: usize,
        data: &Bound<'_, PyAny>,
        r#box: Option<Frame>,
        name: Option<&str>,
    ) -> PyResult<Self> {
        let bytes = map_bytes(data)?;
        raised(vector::Field::map(
            width,
            height,
            &bytes,
            r#box.map(|frame| frame.0),
            name,
        ))
        .map(Self)
    }

    /// This field written out as an 8-bit grayscale map of `width` by `height` pixels over `box` (default one design
    /// pixel a map pixel from the origin), top row first, as `bytes`: each pixel is the field at its centre with `low`
    /// written as 0 and `high` as 255 (linear between, held beyond), the input form of `Field.map` and of an image map.
    #[pyo3(signature = (width, height, *, r#box=None, low=0.0, high=1.0))]
    fn to_map<'py>(
        &self,
        py: Python<'py>,
        width: usize,
        height: usize,
        r#box: Option<Frame>,
        low: f64,
        high: f64,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = raised(self.0.to_map(width, height, r#box.map(|frame| frame.0), low, high))?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// The lines where the field equals each of `levels` (a number or a list), as one `Path`: over `box` cut into cells no
    /// wider than `step` pixels, a line crossing a cell edge at the linear interpolation of the readings at its corners.
    /// A line that reaches the box is open there, any other is closed.
    #[pyo3(signature = (levels, *, r#box, step))]
    fn contours(&self, levels: Levels, r#box: Frame, step: f64) -> PyResult<PyPath> {
        raised(self.0.contours(&levels.0, r#box.0, step)).map(PyPath)
    }

    /// The field's number at `(x, y)`.
    fn at(&self, x: f64, y: f64) -> f64 {
        self.0.at(vector::Point::new(x, y))
    }

    fn __repr__(&self) -> String {
        self.0.to_string()
    }
}

/// A direction without sign at every point: a line through the point, not an arrow. `hatch` draws along one.
#[pyclass(name = "Flow", module = "bullterrier_vector", frozen, eq)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyFlow(pub(crate) vector::Flow);

#[pymethods]
impl PyFlow {
    /// The same direction everywhere: `angle` degrees clockwise from the right on screen (a number, a `Param` or an
    /// expression, read with the values of the step that reads the flow).
    #[staticmethod]
    fn angle(angle: Num) -> PyResult<Self> {
        raised(vector::Flow::angle(angle.0)).map(Self)
    }

    /// Smooth seeded noise; `phase` in cycles moves it, one cycle on is the same flow. `wavelength` and `phase` may be
    /// `Param`s.
    #[staticmethod]
    #[pyo3(signature = (*, wavelength=Num(400.0.into()), phase=Num(0.0.into()), seed=0.0))]
    fn noise(wavelength: Num, phase: Num, seed: f64) -> PyResult<Self> {
        raised(vector::Flow::noise(wavelength.0, phase.0, seed)).map(Self)
    }

    /// The direction of the outline of `drawing` (a Path, a Shape or a Drawing: all its shapes together, drawn with the
    /// values of the step that reads the flow) nearest each point; where two edges are about as near, as at the middle
    /// of a stroke, their directions blend.
    #[staticmethod]
    fn contour(drawing: Item) -> PyResult<Self> {
        raised(vector::Flow::contour(drawing.0)).map(Self)
    }

    /// `fraction` (0 to 1; a number, a `Param` or an expression) of the way to `other`; a direction and its opposite
    /// count as the same, so they never cancel.
    fn mix(&self, other: &PyFlow, fraction: Num) -> PyResult<Self> {
        raised(self.0.mix(&other.0, fraction.0)).map(Self)
    }

    /// The direction at `(x, y)` in degrees from 0 up to 180, clockwise from the right on screen.
    fn at(&self, x: f64, y: f64) -> f64 {
        self.0.at(vector::Point::new(x, y))
    }

    fn __repr__(&self) -> String {
        self.0.to_string()
    }
}

/// A tone as the caller gave it: a `Field`, or a number (a `Param`, an expression) that is the same everywhere.
pub(crate) struct Tone(pub(crate) vector::Field);

impl<'a, 'py> FromPyObject<'a, 'py> for Tone {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(field) = object.cast::<PyField>() {
            return Ok(Self(field.get().0.clone()));
        }
        match object.extract::<Num>() {
            Ok(number) => raised(vector::Field::constant(number.0)).map(Self),
            Err(_) => Err(PyTypeError::new_err(format!(
                "expected a Field or a number, got {}",
                object.repr()?
            ))),
        }
    }
}

/// A spacing as the caller gave it: one number, or `(at tone 0, at tone 1)`.
#[derive(Clone, Copy)]
pub(crate) struct Spacing(pub(crate) (f64, f64));

impl<'a, 'py> FromPyObject<'a, 'py> for Spacing {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(one) = object.extract::<f64>() {
            return Ok(Self((one, one)));
        }
        match object.extract::<(f64, f64)>() {
            Ok(pair) => Ok(Self(pair)),
            Err(_) => Err(PyTypeError::new_err(format!(
                "expected a number or a (at tone 0, at tone 1) pair of numbers, got {}",
                object.repr()?
            ))),
        }
    }
}

/// The hatch a call's `spacing`, `tone`, `flow` and `below` describe: a pair of spacings that differ needs a tone to read.
pub(crate) fn hatch_of(
    spacing: Spacing,
    tone: Option<Tone>,
    flow: Option<&PyFlow>,
    below: Option<Num>,
) -> PyResult<vector::Hatch> {
    let ((near, far), tone) = (spacing.0, tone.map(|tone| tone.0));
    let hatch = match tone {
        Some(tone) => vector::Hatch::between(near, far, tone),
        None if near.to_bits() == far.to_bits() => vector::Hatch::new(near),
        None => {
            return Err(crate::ParameterError::new_err(
                "tone: is needed when `spacing` is a pair",
            ));
        }
    };
    let hatch = match flow {
        Some(flow) => hatch.flow(flow.0.clone()),
        None => hatch,
    };
    Ok(match below {
        Some(level) => hatch.below(level.0),
        None => hatch,
    })
}
