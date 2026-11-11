//! `Drawing`: a tree of steps that draws shapes.

use crate::expr::{Num, PyExpr, PyParam};
use crate::mesh::PyMesh;
use crate::shape::{PaintArg, PyShape};
use crate::text::{bytes_of, entries};
use crate::{DEFAULT_MITER_LIMIT, DEFAULT_TOLERANCE, PyPath, PyPoint, PyRect, exception, raised};
use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyMapping, PyString, PyTuple};

/// An immutable tree of steps that draws shapes. Every step returns a new drawing and leaves this one alone; a drawing
/// holds no callbacks, so it draws again with new values.
#[pyclass(name = "Drawing", module = "bullterrier_vector", frozen)]
#[derive(Clone)]
pub(crate) struct PyDrawing(vector::Drawing);

/// The drawing a step made, or the error it left: a step that cannot be taken is raised at the call that made it.
pub(crate) fn checked(drawing: vector::Drawing) -> PyResult<PyDrawing> {
    match drawing.error() {
        Some(error) => Err(exception(error)),
        None => Ok(PyDrawing(drawing)),
    }
}

/// What a drawing is made of, as the caller gave it: a `Path`, a `Shape` or a `Drawing`.
pub(crate) struct Item(vector::Drawing);

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
    paint: PaintArg,
    width: Option<Num>,
    join: Option<&str>,
    cap: Option<&str>,
    miter_limit: Option<Num>,
) -> PyResult<vector::Stroke> {
    let mut stroke = vector::Stroke::new(paint.0);
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

    /// Every shape turned `angle` degrees clockwise about `about` (each shape's origin by default, else the middle of its bounds); the origin moves with the shape.
    #[pyo3(signature = (angle, *, about=None))]
    fn rotate(&self, angle: Num, about: Option<Pair>) -> PyResult<Self> {
        let mut turn = vector::Rotate::new(vector::Deg(angle.0));
        if let Some(Pair(x, y)) = about {
            turn = turn.about((x, y));
        }
        checked(self.0.rotate(turn))
    }

    /// Every shape scaled by a number, or by an `(x, y)` pair, about `about` (each shape's origin by default, else the middle of its bounds); the origin moves with the shape.
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

    /// Every shape carried along `path`: a point `(x, y)` goes `start` lengths of the path plus `x` along it and `y` to
    /// its right. `start` may be a number, a Param or an expression; a closed path is walked from the point nearest `near`.
    #[pyo3(signature = (path, *, start=None, near=None))]
    fn along(&self, path: &Bound<'_, PyPath>, start: Option<Num>, near: Option<Pair>) -> PyResult<Self> {
        let mut bend = vector::Along::new();
        if let Some(Num(start)) = start {
            bend = bend.start(start);
        }
        if let Some(Pair(x, y)) = near {
            bend = bend.near((x, y));
        }
        checked(self.0.along(&path.get().0, bend))
    }

    /// Every shape with its overlaps removed: the closed polygons it fills by the nonzero rule, in canonical form.
    /// Curves become straight edges within `tolerance` design pixels (a number, not a Param).
    #[pyo3(signature = (*, tolerance=DEFAULT_TOLERANCE), text_signature = "($self, *, tolerance=0.01)")]
    fn resolve(&self, tolerance: f64) -> PyResult<Self> {
        checked(self.0.resolve(tolerance))
    }

    /// Every shape filling the region it and `other` (a Drawing, a Path or a Shape; all its shapes at once) fill
    /// together, keeping its style; as `Path.union`. `other` may hold Params.
    #[pyo3(signature = (other, *, tolerance=DEFAULT_TOLERANCE), text_signature = "($self, other, *, tolerance=0.01)")]
    fn union(&self, other: Item, tolerance: f64) -> PyResult<Self> {
        checked(self.0.union(other.0, tolerance))
    }

    /// Every shape filling the region it fills and `other` does not; as `Drawing.union`.
    #[pyo3(signature = (other, *, tolerance=DEFAULT_TOLERANCE), text_signature = "($self, other, *, tolerance=0.01)")]
    fn subtract(&self, other: Item, tolerance: f64) -> PyResult<Self> {
        checked(self.0.subtract(other.0, tolerance))
    }

    /// Every shape filling the region it and `other` both fill (empty if none); as `Drawing.union`.
    #[pyo3(signature = (other, *, tolerance=DEFAULT_TOLERANCE), text_signature = "($self, other, *, tolerance=0.01)")]
    fn intersect(&self, other: Item, tolerance: f64) -> PyResult<Self> {
        checked(self.0.intersect(other.0, tolerance))
    }

    /// Every shape filling the region only one of it and `other` fills; as `Drawing.union`.
    #[pyo3(signature = (other, *, tolerance=DEFAULT_TOLERANCE), text_signature = "($self, other, *, tolerance=0.01)")]
    fn exclude(&self, other: Item, tolerance: f64) -> PyResult<Self> {
        checked(self.0.exclude(other.0, tolerance))
    }

    /// The edge of the region every shape fills moved outward by `distance` (a number, a Param or an expression, for
    /// each shape), inward when negative; as `Path.offset`. `join`, `miter_limit` and `tolerance` are plain values.
    #[pyo3(
        signature = (distance, *, join="round", miter_limit=DEFAULT_MITER_LIMIT, tolerance=DEFAULT_TOLERANCE),
        text_signature = "($self, distance, *, join='round', miter_limit=4, tolerance=0.01)"
    )]
    fn offset(&self, distance: Num, join: &str, miter_limit: f64, tolerance: f64) -> PyResult<Self> {
        let join = raised(vector::Join::parse(join))?;
        checked(self.0.offset(distance.0, join, miter_limit, tolerance))
    }

    /// Every shape turned into the region a stroke of `width` (a number, a Param or an expression, for each shape)
    /// along it paints; as `Path.outline`.
    #[pyo3(
        signature = (width, *, join="round", cap="round", miter_limit=DEFAULT_MITER_LIMIT, tolerance=DEFAULT_TOLERANCE),
        text_signature = "($self, width, *, join='round', cap='round', miter_limit=4, tolerance=0.01)"
    )]
    fn outline(&self, width: Num, join: &str, cap: &str, miter_limit: f64, tolerance: f64) -> PyResult<Self> {
        let join = raised(vector::Join::parse(join))?;
        let cap = raised(vector::Cap::parse(cap))?;
        checked(self.0.outline(width.0, join, cap, miter_limit, tolerance))
    }

    /// Every shape cut to the piece of its outline between `start` and `end` (fractions 0 to 1 of its length, each a
    /// number, a Param or an expression, read per shape): `slice(0, progress)` draws it on. A closed outline is measured
    /// from its top, or from the point nearest `near`.
    #[pyo3(signature = (start, end, *, near=None))]
    fn slice(&self, start: Num, end: Num, near: Option<Pair>) -> PyResult<Self> {
        checked(match near {
            Some(Pair(x, y)) => self.0.slice_near(start.0, end.0, (x, y)),
            None => self.0.slice(start.0, end.0),
        })
    }

    /// Every shape filled with `color` (`"#rrggbb"`, `"#rrggbbaa"` or `"none"`) or a `Gradient`.
    pub(crate) fn fill(&self, color: PaintArg) -> PyResult<Self> {
        checked(self.0.fill(color.0))
    }

    /// Every outline stroked with `color` (`"#rrggbb"`, `"#rrggbbaa"`) or a `Gradient`; what is left out takes the
    /// defaults (2 pixels, round, round, 4).
    #[pyo3(signature = (color, *, width=None, join=None, cap=None, miter_limit=None))]
    pub(crate) fn stroke(
        &self,
        color: PaintArg,
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

    /// Every shape painted over what is below it as `mode` (a CSS blend name, as on `Path.blend`).
    pub(crate) fn blend(&self, mode: &str) -> PyResult<Self> {
        checked(self.0.blend(raised(vector::Blend::parse(mode))?))
    }

    /// A drop shadow behind every shape, replacing any it has: `dx` and `dy` design pixels from it (0 and 8 where left
    /// out), blurred by `blur` design pixels (16; 0 is a hard copy) and tinted `color` (`"#00000059"`). It is the
    /// silhouette of the shape's fill and stroke, inside the group its opacity and blend apply to.
    #[pyo3(signature = (*, dx=None, dy=None, blur=None, color=None))]
    pub(crate) fn shadow(
        &self,
        dx: Option<Num>,
        dy: Option<Num>,
        blur: Option<Num>,
        color: Option<&str>,
    ) -> PyResult<Self> {
        let mut shadow = vector::Shadow::new();
        if let Some(dx) = dx {
            shadow = shadow.dx(dx.0);
        }
        if let Some(dy) = dy {
            shadow = shadow.dy(dy.0);
        }
        if let Some(blur) = blur {
            shadow = shadow.blur(blur.0);
        }
        if let Some(color) = color {
            shadow = shadow.color(raised(vector::Color::parse(color))?);
        }
        checked(self.0.shadow(shadow))
    }

    /// Every shape filled with a screen of dots of `color` (replacing any fill, as `fill` replaces it): `tone` from 0
    /// (solid) to 1 (no dots) is the share of each cell left bare, `pitch` (8) the distance between dots in design
    /// pixels and `angle` (45) the turn of the lattice in degrees, which is anchored to the origin of the design frame.
    #[pyo3(signature = (color, *, tone, pitch=None, angle=None))]
    pub(crate) fn halftone(&self, color: &str, tone: Num, pitch: Option<Num>, angle: Option<Num>) -> PyResult<Self> {
        let mut halftone = vector::Halftone::new(raised(vector::Color::parse(color))?, tone.0);
        if let Some(pitch) = pitch {
            halftone = halftone.pitch(pitch.0);
        }
        if let Some(angle) = angle {
            halftone = halftone.angle(vector::Deg(angle.0));
        }
        checked(self.0.halftone(halftone))
    }

    /// Every shape removing what is below it in the drawing instead of painting: the area its fill and stroke cover is
    /// cut out of the shapes before it, their shadows included. Its colours, opacity, blend and shadow are not drawn.
    pub(crate) fn knockout(&self) -> PyResult<Self> {
        checked(self.0.knockout())
    }

    /// Every shape, and its shadow, cut to the region `region` (a Drawing, a Path or a Shape; all its shapes united
    /// once) fills. Each shape is cut on its own; a region that fills nothing leaves nothing drawn. `region` may hold
    /// Params.
    pub(crate) fn clip(&self, region: Item) -> PyResult<Self> {
        checked(self.0.clip(region.0))
    }

    /// Every shape, and its shadow, faded by the luminance of `source` (a Drawing, a Path or a Shape) as painted.
    /// Each shape is masked on its own; a shape takes one mask and the shapes of `source` take none.
    pub(crate) fn mask(&self, source: Item) -> PyResult<Self> {
        checked(self.0.mask(source.0))
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

    /// Every shape split into its parts: each closed outline with its holes, and each open line. A part keeps the
    /// attributes and style of its shape and has `part` (from 0, within the shape) and the middle of its own bounds as
    /// `origin`. The list is renumbered.
    fn split(&self) -> PyResult<Self> {
        checked(self.0.split())
    }

    /// The shapes whose attribute `by` is equal (all shapes, if `None`) merged into one, holding their sub-paths in
    /// list order, with the attributes and style of the first; it sits where the first was. Sub-paths are joined, not
    /// united.
    #[pyo3(signature = (*, by=None))]
    fn merge(&self, by: Option<&str>) -> PyResult<Self> {
        checked(by.map_or_else(|| self.0.merge(), |by| self.0.merge_by(by)))
    }

    /// The same drawing with these Params fixed to the numbers (or lists with a number per shape) given.
    #[pyo3(signature = (**given))]
    fn with_values(&self, given: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        checked(self.0.with_values(&values(given)?))
    }

    /// The drawing as JSON in the canonical form of formats.md.
    fn to_json(&self) -> PyResult<String> {
        raised(self.0.to_json())
    }

    /// The drawing a JSON document describes, given the bytes of the inputs it names (its fonts) by name.
    #[staticmethod]
    #[pyo3(signature = (text, *, inputs=None))]
    fn from_json(py: Python<'_>, text: &Bound<'_, PyString>, inputs: Option<&Bound<'_, PyMapping>>) -> PyResult<Self> {
        // a `str` holding a lone surrogate has no UTF-8 form: that is a problem in the text, so a `FormatError`
        let text = text.to_str().map_err(|_| {
            exception(&vector::Error::Format {
                pointer: String::new(),
                message: "the text is not valid UTF-8 (it holds a lone surrogate)".to_owned(),
            })
        })?;
        let mut supplied = vector::Inputs::new();
        if let Some(inputs) = inputs {
            for (name, bytes) in entries(inputs)? {
                let bytes = bytes_of(&format!("inputs[{name}]"), &bytes)?;
                supplied = supplied.set(name.extract::<String>()?, bytes.to_vec());
            }
        }
        checked(raised(py.detach(|| vector::Drawing::from_json(text, &supplied)))?)
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
    fn shapes(&self, py: Python<'_>, given: Option<&Bound<'_, PyDict>>) -> PyResult<Vec<PyShape>> {
        let values = values(given)?;
        let shapes = raised(py.detach(|| self.0.shapes(&values)))?;
        Ok(shapes.into_iter().map(PyShape).collect())
    }

    /// The smallest rectangle that holds every shape the drawing draws, with each Param at the value given by name.
    #[pyo3(signature = (**given))]
    fn bounds(&self, py: Python<'_>, given: Option<&Bound<'_, PyDict>>) -> PyResult<PyRect> {
        let values = values(given)?;
        raised(py.detach(|| self.0.bounds(&values))).map(PyRect::from)
    }

    /// The drawing as an SVG document of `width` by `height` pixels, with each Param at the value given by name.
    #[pyo3(signature = (width, height, /, **given))]
    fn to_svg(&self, py: Python<'_>, width: f64, height: f64, given: Option<&Bound<'_, PyDict>>) -> PyResult<String> {
        let values = values(given)?;
        raised(py.detach(|| self.0.to_svg(vector::Size::px(width, height), &values)))
    }

    /// The drawing as straight-alpha RGBA bytes of `width` by `height` pixels, rows from the top, with each Param at the
    /// value given by name.
    #[pyo3(signature = (width, height, /, **given))]
    fn to_rgba<'py>(
        &self,
        py: Python<'py>,
        width: f64,
        height: f64,
        given: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let values = values(given)?;
        let bytes = raised(py.detach(|| self.0.to_rgba(vector::Size::px(width, height), &values)))?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// The drawing as a PNG file of `width` by `height` pixels (8-bit RGBA: the pixels of `to_rgba`), with each Param at
    /// the value given by name.
    #[pyo3(signature = (width, height, /, **given))]
    fn to_png<'py>(
        &self,
        py: Python<'py>,
        width: f64,
        height: f64,
        given: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let values = values(given)?;
        let bytes = raised(py.detach(|| self.0.to_png(vector::Size::px(width, height), &values)))?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// The triangles of the drawing at `width` by `height` pixels, with each Param at the value given by name: a new
    /// `Mesh`, or `into` refilled and returned.
    #[pyo3(signature = (width, height, /, *, into=None, **given))]
    fn to_mesh(
        &self,
        py: Python<'_>,
        width: f64,
        height: f64,
        into: Option<Py<PyMesh>>,
        given: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyMesh>> {
        let size = vector::Size::px(width, height);
        let given = values(given)?;
        if let Some(mesh) = into {
            let mut filled = mesh.bind(py).try_borrow_mut()?;
            let into = &mut filled.0;
            raised(py.detach(|| self.0.to_mesh(size, &given, into)))?;
            drop(filled);
            return Ok(mesh);
        }
        let mut mesh = vector::Mesh::new();
        raised(py.detach(|| self.0.to_mesh(size, &given, &mut mesh)))?;
        Py::new(py, PyMesh(mesh))
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
