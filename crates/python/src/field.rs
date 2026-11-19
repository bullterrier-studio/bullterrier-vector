//! `Field` and `Flow`: a number and a direction at every point, the inputs of `hatch`.

use crate::expr::Num;
use crate::inputs::{By, Item};
use crate::trace::whole;
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
pub(crate) struct Levels(pub(crate) Vec<f64>);

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
    fn distance(py: Python<'_>, drawing: Item, normalized: bool) -> PyResult<Self> {
        raised(py.detach(|| vector::Field::distance(drawing.0, normalized))).map(Self)
    }

    /// An 8-bit grayscale map as a field, 0 (black) to 1 (white): `data` holds `height` rows of `width` bytes, top row
    /// first (`bytes`, a `bytearray`, a `memoryview`, a `numpy` array of `uint8`, or a list of numbers). It lies over
    /// `box` (`(x, y, width, height)`, default one design pixel a map pixel from the origin); the field is bilinear
    /// between pixel centers and holds the nearest edge value beyond the box. `name` is what a saved drawing calls it.
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
    /// pixel a map pixel from the origin), top row first, as `bytes`: each pixel is the field at its center with `low`
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
        let bytes = raised(py.detach(|| self.0.to_map(width, height, r#box.map(|frame| frame.0), low, high)))?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Each map of `maps` (the frames of a sequence: `Field.map`s of one size and box) as the Gaussian weighted mean of
    /// itself and its neighbours, `sigma` in maps (frames up to `ceil(3 * sigma)` away; the ends are not padded), as a list
    /// of unnamed maps. A static sequence is unchanged, `sigma` 0 gives the maps back.
    #[staticmethod]
    #[expect(clippy::needless_pass_by_value, reason = "pyo3 extracts the list as an owned Vec")]
    #[pyo3(signature = (maps, *, sigma))]
    fn smooth_maps(py: Python<'_>, maps: Vec<PyRef<'_, PyField>>, sigma: f64) -> PyResult<Vec<Self>> {
        let maps: Vec<vector::Field> = maps.iter().map(|map| map.0.clone()).collect();
        let smooth = raised(py.detach(|| vector::Field::smooth_maps(&maps, sigma)))?;
        Ok(smooth.into_iter().map(Self).collect())
    }

    /// The lines where the field equals each of `levels` (a number or a list), as one `Path`: over `box` cut into cells no
    /// wider than `cell` pixels, a line crossing a cell edge at the linear interpolation of the readings at its corners.
    /// A line that reaches the box is open there, any other is closed.
    ///
    /// `smooth` blurs the readings with a Gaussian of that many pixels before tracing, which removes the tiny loops and the jag
    /// of a photograph's tone; `min_length` drops every line shorter than that many pixels. Every line runs with the high side
    /// (a reading at or above the level) on its left as drawn, so the loops of a map that is high inside a region fill as it.
    #[pyo3(signature = (levels, *, r#box, cell, smooth=0.0, min_length=0.0))]
    fn contours(
        &self,
        py: Python<'_>,
        levels: Levels,
        r#box: Frame,
        cell: f64,
        smooth: f64,
        min_length: f64,
    ) -> PyResult<PyPath> {
        let Levels(levels) = levels;
        let tidy = vector::Tidy::default().smooth(smooth).min_length(min_length);
        raised(py.detach(|| self.0.contours_with(&levels, r#box.0, cell, &tidy))).map(PyPath)
    }

    /// Smooth seeded noise from -1 to 1, one lattice cell `wavelength` design pixels wide; `phase` in cycles moves it (one
    /// cycle on is the same field, up to the rounding of the sum), each of `octaves` (1 to 8) adds the noise at half the
    /// wavelength and half the amplitude. `wavelength` and `phase` may be `Param`s.
    #[staticmethod]
    #[pyo3(
        signature = (*, wavelength=None, phase=None, seed=0.0, octaves=None),
        text_signature = "(*, wavelength=400, phase=0, seed=0, octaves=1)"
    )]
    fn noise(
        wavelength: Option<Num>,
        phase: Option<Num>,
        seed: f64,
        octaves: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let wavelength = wavelength.map_or_else(|| vector::Expr::from(vector::NOISE_WAVELENGTH), |number| number.0);
        let phase = phase.map_or_else(|| vector::Expr::from(0.0), |number| number.0);
        let octaves = octaves.map_or(Ok(1), |count| whole("octaves", count))?;
        raised(vector::Field::noise(wavelength, phase, seed, octaves)).map(Self)
    }

    /// The smaller of this field and `other` (a `Field` or a number) at each point.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "pyo3 extracts the operand as an owned value"
    )]
    fn min(&self, other: Tone) -> PyResult<Self> {
        raised(self.0.min(&other.0)).map(Self)
    }

    /// The larger of this field and `other` (a `Field` or a number) at each point.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "pyo3 extracts the operand as an owned value"
    )]
    fn max(&self, other: Tone) -> PyResult<Self> {
        raised(self.0.max(&other.0)).map(Self)
    }

    fn __add__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        combined(py, other, |other| self.0.add(&other))
    }

    fn __radd__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        combined(py, other, |other| other.add(&self.0))
    }

    fn __sub__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        combined(py, other, |other| self.0.add(&negated(&other)?))
    }

    fn __rsub__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        combined(py, other, |other| other.add(&negated(&self.0)?))
    }

    fn __mul__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        combined(py, other, |other| self.0.mul(&other))
    }

    fn __rmul__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        combined(py, other, |other| other.mul(&self.0))
    }

    fn __neg__(&self) -> PyResult<Self> {
        raised(negated(&self.0)).map(Self)
    }

    fn __abs__(&self) -> PyResult<Self> {
        raised(self.0.abs()).map(Self)
    }

    /// The field's number at `(x, y)`.
    fn at(&self, x: f64, y: f64) -> f64 {
        self.0.at(vector::Point::new(x, y))
    }

    fn __repr__(&self) -> String {
        self.0.to_string()
    }
}

/// `field` times -1: the one spelling of a negation and of the right side of a subtraction.
fn negated(field: &vector::Field) -> Result<vector::Field, vector::Error> {
    field.mul(&vector::Field::constant(-1.0)?)
}

/// `operation` on `other` as a field (a number is a constant field), or `NotImplemented` when `other` is neither, so that
/// Python tries the other operand's method and then raises `TypeError`.
fn combined(
    py: Python<'_>,
    other: &Bound<'_, PyAny>,
    operation: impl FnOnce(vector::Field) -> Result<vector::Field, vector::Error>,
) -> PyResult<Py<PyAny>> {
    let field = if let Ok(field) = other.cast::<PyField>() {
        field.get().0.clone()
    } else if let Ok(number) = other.extract::<Num>() {
        raised(vector::Field::constant(number.0))?
    } else {
        return Ok(py.NotImplemented());
    };
    Ok(PyField(raised(operation(field))?)
        .into_pyobject(py)?
        .into_any()
        .unbind())
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

    /// Smooth seeded noise; `phase` in cycles moves it, one cycle on is the same flow up to the rounding of the sum. `wavelength` and `phase` may be
    /// `Param`s.
    #[staticmethod]
    #[pyo3(
        signature = (*, wavelength=None, phase=None, seed=0.0),
        text_signature = "(*, wavelength=400, phase=0, seed=0)"
    )]
    fn noise(wavelength: Option<Num>, phase: Option<Num>, seed: f64) -> PyResult<Self> {
        let wavelength = wavelength.map_or_else(|| vector::Expr::from(vector::NOISE_WAVELENGTH), |number| number.0);
        let phase = phase.map_or_else(|| vector::Expr::from(0.0), |number| number.0);
        raised(vector::Flow::noise(wavelength, phase, seed)).map(Self)
    }

    /// The direction of the outline of `drawing` (a Path, a Shape or a Drawing: all its shapes together, drawn with the
    /// values of the step that reads the flow) nearest each point; where two edges are about as near, as at the middle
    /// of a stroke, their directions blend.
    #[staticmethod]
    fn contour(py: Python<'_>, drawing: Item) -> PyResult<Self> {
        raised(py.detach(|| vector::Flow::contour(drawing.0))).map(Self)
    }

    /// `fraction` (0 to 1; a number, a `Param` or an expression) of the way to `other`; a direction and its opposite
    /// count as the same, so they never cancel.
    fn mix(&self, other: &PyFlow, fraction: Num) -> PyResult<Self> {
        raised(self.0.mix(&other.0, fraction.0)).map(Self)
    }

    /// This flow turned `angle` degrees clockwise on screen (a number, a `Param` or an expression, read with the values
    /// of the step that reads the flow): `Flow.contour(d).rotate(90)` runs across the outlines of `d`.
    fn rotate(&self, angle: Num) -> PyResult<Self> {
        raised(self.0.rotate(angle.0)).map(Self)
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

/// The rows a call's `spacing`, `tone` and `amplitude` describe.
pub(crate) fn rows_of(spacing: Num, tone: Tone, amplitude: Num) -> PyResult<vector::Rows> {
    raised(vector::Rows::new(spacing.0, tone.0, amplitude.0))
}

/// The bands a call describes: `family` is one of `lines`, `waves`, `rings`, `spiral` and `rays`.
#[expect(
    clippy::too_many_arguments,
    reason = "the arguments of the one Python call, passed as they came"
)]
pub(crate) fn bands_of(
    spacing: Num,
    tone: Tone,
    width: (Num, Num),
    family: &str,
    angle: Option<Num>,
    center: Option<Pt>,
    wavelength: Option<Num>,
    amplitude: Option<Num>,
) -> PyResult<vector::Bands> {
    for (name, given) in [
        ("angle", angle.is_some()),
        ("center", center.is_some()),
        ("wavelength", wavelength.is_some()),
        ("amplitude", amplitude.is_some()),
    ] {
        if given {
            raised(vector::Bands::refuse_unread(family, name))?;
        }
    }
    let (spacing, tone, width) = (spacing.0, tone.0, (width.0.0, width.1.0));
    let needed = |value: Option<Num>, name: &str| {
        value
            .map(|number| number.0)
            .ok_or_else(|| crate::ParameterError::new_err(format!("{name}: is needed when `family` is \"waves\"")))
    };
    let bands = match family {
        "lines" => raised(vector::Bands::lines(spacing, tone, width))?,
        "waves" => raised(vector::Bands::waves(
            spacing,
            tone,
            width,
            needed(wavelength, "wavelength")?,
            needed(amplitude, "amplitude")?,
        ))?,
        "rings" => raised(vector::Bands::rings(spacing, tone, width))?,
        "spiral" => raised(vector::Bands::spiral(spacing, tone, width))?,
        "rays" => raised(vector::Bands::rays(spacing, tone, width))?,
        other => {
            return Err(crate::ParameterError::new_err(format!(
                "family: must be one of lines, waves, rings, spiral, rays, got {other:?}"
            )));
        }
    };
    let bands = match angle {
        Some(angle) => raised(bands.angle(angle.0))?,
        None => bands,
    };
    match center {
        Some(center) => raised(bands.center(center.0)),
        None => Ok(bands),
    }
}

/// The stipple a call's numbers describe: one `radius` is the pair of it.
pub(crate) fn stipple_of(
    spacing: Num,
    tone: Tone,
    radius: By,
    density: Option<(Num, Num)>,
    seed: Option<Num>,
) -> PyResult<vector::Stipple> {
    let radius = match radius {
        By::One(one) => (one.0.clone(), one.0),
        By::Two(two) => (two.0, two.1),
    };
    let stipple = raised(vector::Stipple::new(spacing.0, tone.0, radius))?
        .seed(seed.map_or_else(|| vector::Expr::from(0.0), |seed| seed.0));
    Ok(match density {
        Some((dense, light)) => stipple.density(dense.0, light.0),
        None => stipple,
    })
}
