//! `halftone`: what its arguments make, and `Halftone`, the screen a drawn shape carries.

use crate::expr::Num;
use crate::field::PyField;
use crate::{Pt, PyPath, PyPoint, raised};
use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

/// A tone as the caller gave it: a number (a `Param`, an expression, a number for each shape), or a `Field`.
pub(crate) enum ToneArg {
    Number(Num),
    Field(vector::Field),
}

impl<'a, 'py> FromPyObject<'a, 'py> for ToneArg {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(field) = object.cast::<PyField>() {
            return Ok(Self::Field(field.get().0.clone()));
        }
        object.extract::<Num>().map(Self::Number).map_err(|_| {
            PyTypeError::new_err(format!(
                "expected a Field or a number, got {}",
                object.repr().map_or_else(|_| String::new(), |text| text.to_string())
            ))
        })
    }
}

/// A dot as the caller gave it: the word of a shape, or a `Path`, a closed outline.
pub(crate) struct ShapeArg(vector::Mark);

impl<'a, 'py> FromPyObject<'a, 'py> for ShapeArg {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(path) = object.cast::<PyPath>() {
            return Ok(Self(vector::Mark::path(path.get().0.clone())));
        }
        if let Ok(word) = object.extract::<&str>() {
            return raised(vector::Mark::parse(word)).map(Self);
        }
        Err(PyTypeError::new_err(format!(
            "expected the word of a shape or a Path, got {}",
            object.repr().map_or_else(|_| String::new(), |text| text.to_string())
        )))
    }
}

/// The halftone a call describes.
#[expect(
    clippy::too_many_arguments,
    reason = "the arguments of the one Python call, passed as they came"
)]
pub(crate) fn halftone_of(
    color: &str,
    tone: ToneArg,
    pitch: Option<Num>,
    angle: Option<Num>,
    shape: Option<ShapeArg>,
    aspect: Option<Num>,
    screen: Option<&str>,
    center: Option<Pt>,
    amplitude: Option<Num>,
    wavelength: Option<Num>,
) -> PyResult<vector::Halftone> {
    let color = raised(vector::Color::parse(color))?;
    let mut halftone = match tone {
        ToneArg::Number(number) => vector::Halftone::new(color, number.0),
        ToneArg::Field(field) => vector::Halftone::new(color, field),
    };
    if let Some(pitch) = pitch {
        halftone = halftone.pitch(pitch.0);
    }
    if let Some(angle) = angle {
        halftone = halftone.angle(vector::Deg(angle.0));
    }
    if let Some(ShapeArg(mark)) = shape {
        halftone = halftone.shape(mark);
    }
    if let Some(aspect) = aspect {
        halftone = halftone.aspect(aspect.0);
    }
    if let Some(screen) = screen {
        halftone = halftone.screen(raised(vector::Screen::parse(screen))?);
    }
    if let Some(Pt(center)) = center {
        halftone = halftone.center(center);
    }
    if let Some(amplitude) = amplitude {
        halftone = halftone.amplitude(amplitude.0);
    }
    if let Some(wavelength) = wavelength {
        halftone = halftone.wavelength(wavelength.0);
    }
    Ok(halftone)
}

/// A halftone fill, as it is drawn: a screen of dots in a colour (`"#rrggbbaa"`) on the layout `screen`, the `tone` (0
/// solid, 1 no dots; a number, or the `Field` it was given), the `pitch` in design pixels, the `angle` in degrees, the
/// `shape` of a dot (a word, or the `Path` it was given), its `aspect`, the `center` of rings, a spiral and rays (`None`:
/// the middle of the shape's bounds) and a wave's `amplitude` and `wavelength` in pitches.
#[pyclass(name = "Halftone", module = "bullterrier_vector", frozen, eq)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyHalftone(pub(crate) vector::HalftoneStyle);

#[pymethods]
impl PyHalftone {
    /// The colour of the dots, `"#rrggbbaa"`.
    #[getter]
    fn color(&self) -> String {
        self.0.color().to_string()
    }

    /// The share of each cell the dot leaves bare, 0 (solid) to 1 (no dots): a number, or a `Field` read at the center
    /// of each dot.
    #[getter]
    fn tone(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(match self.0.tone() {
            vector::Tone::Level(level) => level.into_pyobject(py)?.into_any().unbind(),
            vector::Tone::Field(field) => Bound::new(py, PyField((**field).clone()))?.into_any().unbind(),
        })
    }

    /// The distance between neighbouring rows and dots in design pixels.
    #[getter]
    fn pitch(&self) -> f64 {
        self.0.pitch()
    }

    /// The turn of the layout in degrees, clockwise on the screen about the origin of the design frame.
    #[getter]
    fn angle(&self) -> f64 {
        self.0.angle()
    }

    /// What a dot is: `"dot"`, `"square"`, `"ellipse"`, `"diamond"` or `"euclidean"`, or the `Path` of the caller's outline.
    #[getter]
    fn shape(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(match self.0.shape() {
            vector::Mark::Path(path) => Bound::new(py, PyPath((**path).clone()))?.into_any().unbind(),
            other => other.name().into_pyobject(py)?.into_any().unbind(),
        })
    }

    /// How much longer an ellipse is than wide, at least 1.
    #[getter]
    fn aspect(&self) -> f64 {
        self.0.aspect()
    }

    /// Where the dots lie: `"grid"`, `"wave"`, `"rings"`, `"spiral"` or `"rays"`.
    #[getter]
    fn screen(&self) -> &'static str {
        self.0.screen().name()
    }

    /// The center of rings, a spiral and rays, or `None` for the middle of the shape's bounding box.
    #[getter]
    fn center(&self) -> Option<PyPoint> {
        self.0.center().map(|center| PyPoint {
            x: center.x,
            y: center.y,
        })
    }

    /// How high a wave swings its rows, in pitches.
    #[getter]
    fn amplitude(&self) -> f64 {
        self.0.amplitude()
    }

    /// How long a wave is, in pitches.
    #[getter]
    fn wavelength(&self) -> f64 {
        self.0.wavelength()
    }

    fn __repr__(&self) -> String {
        self.describe()
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        // a field tone hashes as no number: two screens that differ only in their field collide, which is allowed
        (
            self.color(),
            self.0.tone().level(),
            self.pitch(),
            self.angle(),
            self.0.shape().name(),
            self.aspect(),
            self.screen(),
            self.0.center().map(|center| (center.x, center.y)),
            self.amplitude(),
            self.wavelength(),
        )
            .into_pyobject(py)?
            .hash()
    }
}

impl PyHalftone {
    /// The repr: a field tone is `Field(...)`, a caller's outline `Path(...)` with its path data.
    pub(crate) fn describe(&self) -> String {
        let tone = self
            .0
            .tone()
            .level()
            .map_or_else(|| "Field(...)".to_owned(), |level| format!("{level:?}"));
        let shape = match self.0.shape() {
            vector::Mark::Path(path) => format!("Path({:?})", path.to_svg_path()),
            other => format!("'{}'", other.name()),
        };
        let center = self.0.center().map_or_else(
            || "None".to_owned(),
            |center| format!("({:?}, {:?})", center.x, center.y),
        );
        format!(
            "Halftone(color='{}', tone={tone}, pitch={:?}, angle={:?}, shape={shape}, aspect={:?}, screen='{}', center={center}, amplitude={:?}, wavelength={:?})",
            self.color(),
            self.pitch(),
            self.angle(),
            self.aspect(),
            self.screen(),
            self.amplitude(),
            self.wavelength()
        )
    }
}
