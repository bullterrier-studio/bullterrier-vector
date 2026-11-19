//! `Drawing`: a tree of steps that draws shapes.

use crate::expr::{Num, PyExpr, PyParam};
use crate::field::{PyField, PyFlow, Spacing, Tone, bands_of, hatch_of, rows_of, stipple_of};
use crate::halftone::{ShapeArg, ToneArg, halftone_of};
use crate::inputs::{By, Item, Pair};
use crate::mesh::PyMesh;
use crate::shape::{PaintArg, PyShape};
use crate::text::{bytes_of, entries};
use crate::trace::{Levels, colors, trace_of, whole};
use crate::{DEFAULT_MITER_LIMIT, DEFAULT_TOLERANCE, Frame, Pt, PyPath, PyPoint, PyRect, exception, raised};
use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyMapping, PyString, PyTuple};

/// An immutable tree of steps that draws shapes. Every step returns a new drawing and leaves this one alone; a drawing
/// holds no callbacks, so it draws again with new values.
#[pyclass(name = "Drawing", module = "bullterrier_vector", frozen)]
#[derive(Clone)]
pub(crate) struct PyDrawing(pub(crate) vector::Drawing);

/// The drawing a step made, or the error it left: a step that cannot be taken is raised at the call that made it.
pub(crate) fn checked(drawing: vector::Drawing) -> PyResult<PyDrawing> {
    match drawing.error() {
        Some(error) => Err(exception(error)),
        None => Ok(PyDrawing(drawing)),
    }
}

/// The stroke a call describes; the core holds the defaults of what is left out.
fn stroke_of(
    paint: PaintArg,
    width: Option<Num>,
    join: Option<&str>,
    cap: Option<&str>,
    miter_limit: Option<Num>,
    profile: Option<Vec<Pair>>,
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
    if let Some(profile) = profile {
        stroke = stroke.profile(profile.into_iter().map(|Pair(at, factor)| (at, factor)));
    }
    Ok(stroke)
}

/// What a selection asks of one attribute: a number, a character, text, a `Point`, or a list of them for any of.
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
        return Ok(match (characters.next(), characters.next()) {
            (Some(character), None) => character.into(),
            _ => text.into(),
        });
    }
    if let Ok(point) = value.cast::<PyPoint>() {
        let PyPoint { x, y } = *point.get();
        return Ok(vector::Point::new(x, y).into());
    }
    match value.extract::<f64>() {
        Ok(number) => Ok(number.into()),
        Err(_) => Err(PyTypeError::new_err(format!(
            "{name}: expected a number, text, a Point or a list of them, got {}",
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

    /// A copy of all the shapes at each stop along `path`: `count` stops (both ends of an open path) or one every
    /// `spacing` pixels (exactly one of the two, whole and plain numbers); `start` (a number, a Param or an expression)
    /// moves them on by that share of the path; each copy is carried by the middle of its bounds (or the origin its
    /// shapes share) and, with `turn`, turned so its x axis runs along the path. Not bent: `along` bends.
    #[pyo3(signature = (path, *, count=None, spacing=None, start=None, turn=true, near=None))]
    fn place_along(
        &self,
        path: &Bound<'_, PyPath>,
        count: Option<i64>,
        spacing: Option<f64>,
        start: Option<Num>,
        turn: bool,
        near: Option<Pair>,
    ) -> PyResult<Self> {
        let spacing = match (count, spacing) {
            // a negative count is 0, which the step refuses naming `count`
            (Some(count), None) => vector::Spacing::Count(usize::try_from(count).unwrap_or(0)),
            (None, Some(every)) => vector::Spacing::Every(every),
            _ => {
                return Err(exception(&vector::Error::Parameter {
                    parameter: String::from("count"),
                    message: String::from("exactly one of count and spacing must be given"),
                }));
            }
        };
        let mut options = vector::PlaceAlong::new().turn(turn);
        if let Some(Num(start)) = start {
            options = options.start(start);
        }
        if let Some(Pair(x, y)) = near {
            options = options.near((x, y));
        }
        checked(self.0.place_along(&path.get().0, spacing, options))
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
    /// along it paints; as `Path.outline`. `profile` is a list of `(position, factor)` pairs (numbers, Params or
    /// expressions) that varies the width along each sub-path: a taper is `[(0, 0), (0.15, 1), (0.85, 1), (1, 0)]`.
    #[pyo3(
        signature = (width, *, profile=None, join="round", cap="round", miter_limit=DEFAULT_MITER_LIMIT, tolerance=DEFAULT_TOLERANCE),
        text_signature = "($self, width, *, profile=None, join='round', cap='round', miter_limit=4, tolerance=0.01)"
    )]
    fn outline(
        &self,
        width: Num,
        profile: Option<Vec<Pair>>,
        join: &str,
        cap: &str,
        miter_limit: f64,
        tolerance: f64,
    ) -> PyResult<Self> {
        let join = raised(vector::Join::parse(join))?;
        let cap = raised(vector::Cap::parse(cap))?;
        if let Some(profile) = profile {
            raised(vector::require_round_stroke(join, cap))?;
            let stops = profile.into_iter().map(|Pair(at, factor)| (at, factor));
            return checked(self.0.tapered_outline(width.0, stops, tolerance));
        }
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

    /// Every shape cut into the pieces a dash `pattern` leaves (a list of numbers, Params or expressions, read per shape);
    /// `phase` in cycles of the pattern moves the dashes forward: marching ants are `dash([8, 4], phase=march)`. As
    /// `Path.dash`; a closed outline is measured from its top, or from the point nearest `near`.
    #[pyo3(signature = (pattern, *, phase=None, near=None), text_signature = "($self, pattern, *, phase=0, near=None)")]
    fn dash(&self, pattern: Vec<Num>, phase: Option<Num>, near: Option<Pair>) -> PyResult<Self> {
        let pattern = pattern.into_iter().map(|Num(length)| length);
        let phase = phase.map_or_else(|| vector::Expr::from(0.0), |Num(phase)| phase);
        checked(match near {
            Some(Pair(x, y)) => self.0.dash_near(pattern, phase, (x, y)),
            None => self.0.dash(pattern, phase),
        })
    }

    /// Every shape as straight segments with corners `spacing` apart along it (a number, a Param or an expression, read
    /// per shape); as `Path.resample`.
    fn resample(&self, spacing: Num) -> PyResult<Self> {
        checked(self.0.resample(spacing.0))
    }

    /// Every shape with its points displaced sideways by a sine (each argument a number, a Param or an expression, read
    /// per shape); as `Path.wave`. The wave is anchored in the design frame, so the letters of a word share one wave.
    #[pyo3(
        signature = (*, amplitude=None, wavelength=None, angle=None, phase=None),
        text_signature = "($self, *, amplitude=8, wavelength=120, angle=0, phase=0)"
    )]
    fn wave(
        &self,
        amplitude: Option<Num>,
        wavelength: Option<Num>,
        angle: Option<Num>,
        phase: Option<Num>,
    ) -> PyResult<Self> {
        let or =
            |given: Option<Num>, default: f64| given.map_or_else(|| vector::Expr::from(default), |Num(value)| value);
        checked(self.0.wave(
            or(amplitude, 8.0),
            or(wavelength, 120.0),
            vector::Deg(or(angle, 0.0)),
            or(phase, 0.0),
        ))
    }

    /// Every shape with its points displaced by smooth seeded noise (each argument a number, a Param or an expression,
    /// read per shape); as `Path.wobble`. The noise is anchored in the design frame, so the letters of a word share one
    /// field.
    #[pyo3(
        signature = (*, amplitude=None, wavelength=None, phase=None, seed=None),
        text_signature = "($self, *, amplitude=4, wavelength=80, phase=0, seed=0)"
    )]
    fn wobble(
        &self,
        amplitude: Option<Num>,
        wavelength: Option<Num>,
        phase: Option<Num>,
        seed: Option<Num>,
    ) -> PyResult<Self> {
        let or =
            |given: Option<Num>, default: f64| given.map_or_else(|| vector::Expr::from(default), |Num(value)| value);
        checked(
            self.0
                .wobble(or(amplitude, 4.0), or(wavelength, 80.0), or(phase, 0.0), or(seed, 0.0)),
        )
    }

    /// Every shape with its points turned about a centre, less the farther out they are (each argument a number, a Param
    /// or an expression, read per shape); as `Path.twist`. The centre is `about`, else each shape's origin, else the middle
    /// of its bounds.
    #[pyo3(
        signature = (angle=None, *, radius=None, about=None),
        text_signature = "($self, angle=90, *, radius=None, about=None)"
    )]
    fn twist(&self, angle: Option<Num>, radius: Option<Num>, about: Option<Pair>) -> PyResult<Self> {
        let mut twist = vector::Twist::new(vector::Deg(
            angle.map_or_else(|| vector::Expr::from(90.0), |Num(angle)| angle),
        ));
        if let Some(Num(radius)) = radius {
            twist = twist.radius(radius);
        }
        if let Some(Pair(x, y)) = about {
            twist = twist.about((x, y));
        }
        checked(self.0.twist(twist))
    }

    /// Every shape with its points pushed out from, or in towards, a centre, less the farther out they are (each
    /// argument a number, a Param or an expression, read per shape); as `Path.bulge`. The centre is `about`, else each
    /// shape's origin, else the middle of its bounds.
    #[pyo3(
        signature = (amount=None, *, radius=None, about=None),
        text_signature = "($self, amount=0.5, *, radius=None, about=None)"
    )]
    fn bulge(&self, amount: Option<Num>, radius: Option<Num>, about: Option<Pair>) -> PyResult<Self> {
        let mut bulge = vector::Bulge::new(amount.map_or_else(|| vector::Expr::from(0.5), |Num(amount)| amount));
        if let Some(Num(radius)) = radius {
            bulge = bulge.radius(radius);
        }
        if let Some(Pair(x, y)) = about {
            bulge = bulge.about((x, y));
        }
        checked(self.0.bulge(bulge))
    }

    /// Every shape bent by one frame (a `Rect` or `(x, y, width, height)`; the bounds of all the shapes by default, so that
    /// a line of text keeps its place in the block) mapped onto the region between the curves `top` and `bottom` (the
    /// frame's straight bottom edge by default); as `Path.envelope`. `amount` is a number, a Param or an expression, read
    /// per shape.
    #[pyo3(
        signature = (top, bottom=None, *, frame=None, amount=None),
        text_signature = "($self, top, bottom=None, *, frame=None, amount=1)"
    )]
    fn envelope(
        &self,
        top: &Bound<'_, PyPath>,
        bottom: Option<&Bound<'_, PyPath>>,
        frame: Option<Frame>,
        amount: Option<Num>,
    ) -> PyResult<Self> {
        let mut options = vector::Envelope::new();
        if let Some(bottom) = bottom {
            options = options.bottom(&bottom.get().0);
        }
        if let Some(Frame(frame)) = frame {
            options = options.frame(frame);
        }
        if let Some(Num(amount)) = amount {
            options = options.amount(amount);
        }
        checked(self.0.envelope(&top.get().0, options))
    }

    /// Every shape arched by one frame (a `Rect` or `(x, y, width, height)`; the bounds of all the shapes by default): the
    /// frame's whole band is lifted by a parabola whose middle is `bend` times the frame's width above its corners (below if
    /// negative), top and bottom alike; as `Path.arch`. `bend` is a number, a Param or an expression, read per shape.
    #[pyo3(signature = (bend, *, frame=None))]
    fn arch(&self, bend: Num, frame: Option<Frame>) -> PyResult<Self> {
        let mut arch = vector::Arch::new(bend.0);
        if let Some(Frame(frame)) = frame {
            arch = arch.frame(frame);
        }
        checked(self.0.arch(arch))
    }

    /// Every shape morphed `fraction` (a number, a Param or an expression, read per shape) of the way to the shape at the
    /// same index of `target` (a Drawing, a Path or a Shape): 0 is the source, 1 the target, any other number the lerp
    /// (outside 0 to 1 it extrapolates); as `Path.morph` for each pair, so the result is made of straight lines. A pair
    /// keeps the source's style and attributes; a surplus source shape shrinks to its own centre and a surplus target shape
    /// grows from its own centre, each keeping its own style.
    fn morph(&self, target: Item, fraction: Num) -> PyResult<Self> {
        checked(self.0.morph(target.0, fraction.0))
    }

    /// Every shape replaced by evenly spaced lines along `flow` (default `Flow.angle(45)`) inside the region it fills, as
    /// `Path.hatch` makes them: `spacing` pixels apart, or a pair read from `tone`; the lines keep out of `avoid` (a Path, a
    /// Shape or a Drawing). With `below` (a number or a `Param`), lines only where the tone is at most that level (needs a
    /// `tone`; 0 is dark: hatch steps at other angles and levels, stacked, crosshatch). The lines are open, so stroke
    /// them. A shape keeps its style and attributes.
    #[pyo3(signature = (spacing, *, tone=None, flow=None, avoid=None, below=None))]
    fn hatch(
        &self,
        spacing: Spacing,
        tone: Option<Tone>,
        flow: Option<&PyFlow>,
        avoid: Option<Item>,
        below: Option<Num>,
    ) -> PyResult<Self> {
        let hatch = hatch_of(spacing, tone, flow, below)?;
        checked(self.0.hatch(&hatch, avoid.map(|item| item.0).as_ref()))
    }

    /// Every shape replaced by ridgelines inside the region it fills, as `Path.rows` makes them for its path.
    #[pyo3(signature = (spacing, *, tone, amplitude))]
    fn rows(&self, spacing: Num, tone: Tone, amplitude: Num) -> PyResult<Self> {
        checked(self.0.rows(&rows_of(spacing, tone, amplitude)?))
    }

    /// Lines whose width follows `tone`: every shape replaced by their closed outlines, as `Path.bands` makes them for its
    /// path; fill them. A `Param` in a number is read with the values of the draw.
    #[expect(clippy::too_many_arguments, reason = "the arguments of one Python call")]
    #[pyo3(signature = (spacing, *, tone, width, family="lines", angle=None, centre=None, wavelength=None, amplitude=None))]
    fn bands(
        &self,
        spacing: Num,
        tone: Tone,
        width: (Num, Num),
        family: &str,
        angle: Option<Num>,
        centre: Option<crate::Pt>,
        wavelength: Option<Num>,
        amplitude: Option<Num>,
    ) -> PyResult<Self> {
        checked(self.0.bands(&bands_of(
            spacing, tone, width, family, angle, centre, wavelength, amplitude,
        )?))
    }

    /// Every shape replaced by round dots whose density follows `tone`, as `Path.stipple` makes them for its path; fill them.
    #[pyo3(signature = (spacing, *, tone, radius, density=None, seed=None))]
    fn stipple(
        &self,
        spacing: Num,
        tone: Tone,
        radius: Num,
        density: Option<(Num, Num)>,
        seed: Option<Num>,
    ) -> PyResult<Self> {
        checked(self.0.stipple(&stipple_of(spacing, tone, radius, density, seed)?))
    }

    /// A field (`map`: a `Field.map`, 0 black to 1 white, or any `Field`) traced at `levels` (numbers, Params or
    /// expressions, read once) as filled shapes: one for each level it reaches, in the order given, the region at or
    /// above the level, with the level as its `level` attribute and no fill (`.only(level=l).fill(...)`). The field is
    /// read at the centres of `box` (`(x, y, width, height)`) cut into equal cells no wider than `cell` design pixels;
    /// `None` is a map's own box and one map pixel, and any other field needs both. Shapes that
    /// meet the border go on past it by the larger of `open` and `close` plus `tolerance`, plus two cells: crop
    /// with `.intersect(Path.rect(...))` for a plotter. `smooth` is the blur's standard deviation in design pixels;
    /// `close` then `open` (design pixels) fill and remove what is narrower than twice the radius; `min_hole` and
    /// `min_area` (square pixels) fill holes and drop parts; `tolerance` fits curves within that many design pixels
    /// (0 keeps the polyline). A padded lattice past 2^25 nodes raises naming `open`, `close` or `tolerance`.
    #[staticmethod]
    #[pyo3(
        signature = (map, levels, *, r#box=None, cell=None, smooth=None, tolerance=None, open=None, close=None, min_area=None, min_hole=None),
        text_signature = "(map, levels, *, box=None, cell=None, smooth=0, tolerance=0.5, open=0, close=0, min_area=0, min_hole=0)"
    )]
    #[expect(clippy::too_many_arguments, reason = "the arguments of one Python call")]
    fn trace_levels(
        map: &PyField,
        levels: Levels,
        r#box: Option<Frame>,
        cell: Option<f64>,
        smooth: Option<Num>,
        tolerance: Option<Num>,
        open: Option<Num>,
        close: Option<Num>,
        min_area: Option<Num>,
        min_hole: Option<Num>,
    ) -> PyResult<Self> {
        let trace = trace_of(smooth, tolerance, open, close, min_area, min_hole);
        let trace = match r#box {
            Some(frame) => trace.frame(frame.0),
            None => trace,
        };
        let trace = match cell {
            Some(cell) => trace.cell(cell),
            None => trace,
        };
        checked(vector::Drawing::trace_levels(&map.0, levels.0, &trace))
    }

    /// A map of colour labels traced as filled shapes, one for each byte it holds (attribute `label`; more than
    /// `max_colors` raises). They tile the frame and its padding, never overlapping or leaving a gap: the commonest
    /// label (lowest byte of a tie) is first and is what the others leave, the others follow by byte, each cut by those
    /// after it (polylines within 0.01 design pixels). Too many outlines to cut raises naming `max_colors`. The other
    /// arguments are those of `trace_levels`.
    #[staticmethod]
    #[pyo3(
        signature = (map, *, max_colors=None, smooth=None, tolerance=None, open=None, close=None, min_area=None, min_hole=None),
        text_signature = "(map, *, max_colors=32, smooth=0, tolerance=0.5, open=0, close=0, min_area=0, min_hole=0)"
    )]
    #[expect(clippy::too_many_arguments, reason = "the arguments of one Python call")]
    fn trace_labels(
        map: &PyField,
        max_colors: Option<&Bound<'_, PyAny>>,
        smooth: Option<Num>,
        tolerance: Option<Num>,
        open: Option<Num>,
        close: Option<Num>,
        min_area: Option<Num>,
        min_hole: Option<Num>,
    ) -> PyResult<Self> {
        let trace = trace_of(smooth, tolerance, open, close, min_area, min_hole);
        checked(vector::Drawing::trace_labels(&map.0, colors(max_colors)?, &trace))
    }

    /// A map of square cells (`cell` map pixels, each one byte, a palette index) traced exactly: one shape for each
    /// colour (attribute `label` the byte) with straight edges on the cell sides, touching cells merged, holes kept;
    /// the areas are the cells' and the shapes neither overlap nor leave a gap. At most `max_colors` colours.
    #[staticmethod]
    #[pyo3(signature = (map, cell, *, max_colors=None), text_signature = "(map, cell, *, max_colors=32)")]
    fn trace_cells(map: &PyField, cell: &Bound<'_, PyAny>, max_colors: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let (cell, colors) = (whole("cell", cell)?, colors(max_colors)?);
        checked(vector::Drawing::trace_cells(&map.0, cell, colors))
    }

    /// Every shape run the other way, as `Path.reverse`: `text(...).reverse().slice(0, progress)` draws on from the
    /// other end. Style, attributes and `origin` are kept.
    fn reverse(&self) -> PyResult<Self> {
        checked(self.0.reverse())
    }

    /// Every shape as few smooth curves within `tolerance` pixels of it (a number, a Param or an expression, read
    /// per shape); as `Path.simplify`.
    #[pyo3(signature = (*, tolerance=None), text_signature = "($self, *, tolerance=0.5)")]
    fn simplify(&self, tolerance: Option<Num>) -> PyResult<Self> {
        let tolerance = tolerance.map_or_else(|| vector::Expr::from(0.5), |Num(tolerance)| tolerance);
        checked(self.0.simplify(tolerance))
    }

    /// Every shape filled with `color` (`"#rgb"`, `"#rrggbb"`, `"#rrggbbaa"` or `"none"`) or a `Gradient`.
    pub(crate) fn fill(&self, color: PaintArg) -> PyResult<Self> {
        checked(self.0.fill(color.0))
    }

    /// Every outline stroked with `color` (`"#rgb"`, `"#rrggbb"`, `"#rrggbbaa"`) or a `Gradient`; what is left out takes the
    /// defaults (2 pixels, round, round, 4). `profile`, a list of `(position, factor)` pairs (numbers, Params or
    /// expressions), makes the width `width` times the factor at each place along each sub-path, drawn as a filled
    /// outline with round joins and ends: a taper is `[(0, 0), (0.15, 1), (0.85, 1), (1, 0)]`.
    #[pyo3(signature = (color, *, width=None, join=None, cap=None, miter_limit=None, profile=None))]
    pub(crate) fn stroke(
        &self,
        color: PaintArg,
        width: Option<Num>,
        join: Option<&str>,
        cap: Option<&str>,
        miter_limit: Option<Num>,
        profile: Option<Vec<Pair>>,
    ) -> PyResult<Self> {
        checked(self.0.stroke(stroke_of(color, width, join, cap, miter_limit, profile)?))
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

    /// Every shape filled with a screen of dots of `color` (replacing any fill, as `fill` replaces it). Each dot covers
    /// `1 - tone` of its own cell: `tone` from 0 (solid) to 1 (no dots) is a number (a `Param`, a number for each shape)
    /// or a `Field` read at the centre of each dot. `pitch` (8) is the distance between rows and dots in design pixels
    /// and `angle` (45) the turn of the layout in degrees about the origin of the design frame. `shape` is the dot:
    /// `"dot"`, `"square"`, `"ellipse"` (`aspect` times as long as wide, at least 1), `"diamond"`, `"euclidean"` (round,
    /// a diamond at tone 0.5, then a round hole) or a closed `Path` (its first sub-path, on its centroid, scaled to the
    /// area). `screen` is the layout: `"grid"`, `"wave"` (rows swung `amplitude` pitches (1) over `wavelength` pitches
    /// (6)), `"rings"`, `"spiral"` or `"rays"` (about `centre`, default the middle of the shape's bounds). The grid and
    /// the wave are anchored to the design frame.
    #[expect(clippy::too_many_arguments, reason = "the arguments of one Python call")]
    #[pyo3(signature = (color, *, tone, pitch=None, angle=None, shape=None, aspect=None, screen=None, centre=None, amplitude=None, wavelength=None))]
    pub(crate) fn halftone(
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
    ) -> PyResult<Self> {
        checked(self.0.halftone(halftone_of(
            color, tone, pitch, angle, shape, aspect, screen, centre, amplitude, wavelength,
        )?))
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

    /// Only the shapes whose attributes equal these (a list means any of). The list is renumbered. `class` is a Python
    /// keyword: `only(**{"class": "c"})`.
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

    /// A drawing that comes with the library, by name; `Drawing.preset_names()` lists them. Its `parameters()` are the
    /// sliders.
    #[staticmethod]
    fn preset(name: &str) -> PyResult<Self> {
        checked(raised(vector::Drawing::preset(name))?)
    }

    /// The one-sentence description of the preset `name`, what its look is; an unknown name is refused as in
    /// `Drawing.preset`.
    #[staticmethod]
    fn preset_description(name: &str) -> PyResult<&'static str> {
        raised(vector::Drawing::preset_description(name))
    }

    /// The names of the drawings that come with the library, sorted.
    #[staticmethod]
    fn preset_names() -> Vec<&'static str> {
        vector::Drawing::preset_names()
    }

    /// The drawing an SVG document describes, so that `Drawing.from_svg(text).only(id="ring")` picks the part the SVG
    /// calls "ring". A shape for each `path`, `rect`, `circle`, `ellipse`, `line`, `polyline`
    /// and `polygon`, in document order, with the attributes `id`, `class` and `group` (the ids of the enclosing `g`
    /// elements joined by `/`, as `"logo/mark"`) besides `index` and `order`, so `only(id="ring")` picks a part by name.
    /// `only(group=...)` and `only(**{"class": ...})` match the whole attribute text (`class="st0 st1"` is selected by
    /// `"st0 st1"`, not `"st0"`); `class` is a Python keyword, so it goes in a dict. The `viewBox` is the size; `transform`s are worked into the points.
    /// `fill` (black unless set), `stroke`, `stroke-width`, `stroke-opacity`, `fill-opacity`, `stroke-linejoin`,
    /// `stroke-linecap` and `stroke-miterlimit` are read as attributes or in `style` and handed down groups; `opacity`
    /// on shapes; colours as `#rgb`, `#rrggbb`, `#rrggbbaa` or `none`; lengths in pixels.
    ///
    /// Every imported shape has a fill, so `.stroke(c)` alone leaves it showing: outline everything with
    /// `Drawing.from_svg(text).fill("none").stroke(c)` (call `.only(...)` first to keep some fills).
    ///
    /// What is not read raises a `ParameterError` naming the element or attribute: `fill-rule="evenodd"` (the renderer
    /// fills nonzero only: split the shape into paths that need no evenodd), `use`, `defs` holding anything, gradients,
    /// patterns, `clip-path`, `mask`, `filter`, markers, dashes, `paint-order`, `vector-effect`, `transform-origin`, a `<!DOCTYPE>` (remove the line), `text`, `image`, `<style>`
    /// elements and `<?xml-stylesheet?>`, a `transform` in `style`, nested `svg`,
    /// `opacity` on a `g`, colour names and `rgb()`, units other than pixels, a stroke under a transform that
    /// stretches one way. Past 20,000 elements or 128 levels of nesting it raises too.
    #[staticmethod]
    fn from_svg(py: Python<'_>, text: &Bound<'_, PyString>) -> PyResult<Self> {
        // a `str` holding a lone surrogate has no UTF-8 form: a problem in the text, so a `ParameterError`
        let text = text.to_str().map_err(|_| {
            exception(&vector::Error::Parameter {
                parameter: "svg".to_owned(),
                message: "the text is not valid UTF-8 (it holds a lone surrogate)".to_owned(),
            })
        })?;
        checked(raised(py.detach(|| vector::Drawing::from_svg(text)))?)
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

    /// The bounds of the geometry: the smallest rectangle that holds every shape the drawing draws (the union of their
    /// path bounds), with each Param at the value given by name. Not the painted extent: strokes, shadows and effects
    /// reach past it.
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

    /// The drawing as an SVG document for a pen plotter of `width` by `height` in `unit` (`"px"`, `"mm"` or `"in"`; the
    /// document then says `width="210mm"`): strokes only, one layer per pen (a stroke or fill colour), the lines of a
    /// layer ordered to cut pen-up travel, simplified within `tolerance` design pixels (default a quarter of
    /// `pen_width`) and every fill hatched at `pen_width`. A keyword named `pen_width`, `tolerance` or `unit` is the
    /// plot's, so a Param cannot be called any of them.
    #[expect(clippy::too_many_arguments, reason = "the arguments of one Python call")]
    #[pyo3(signature = (width, height, /, *, pen_width, tolerance=None, unit="px", **given))]
    fn to_plot_svg(
        &self,
        py: Python<'_>,
        width: f64,
        height: f64,
        pen_width: f64,
        tolerance: Option<f64>,
        unit: &str,
        given: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<String> {
        let values = values(given)?;
        let unit = vector::PlotUnit::from_name(unit)
            .ok_or_else(|| crate::ParameterError::new_err(format!("unit: must be one of px, mm, in, got {unit:?}")))?;
        let plot = raised(vector::Plot::new(pen_width))?.unit(unit);
        let plot = tolerance.map_or(plot, |tolerance| plot.tolerance(tolerance));
        raised(py.detach(|| self.0.to_plot_svg(vector::Size::px(width, height), &values, &plot)))
    }

    /// The drawing as straight-alpha RGBA bytes of `width` by `height` pixels, rows from the top, with each Param at the
    /// value given by name. With a `window` `(x, y, width, height)` of whole pixels inside the output, only that
    /// rectangle: its rows and columns of the whole render, where `width` and `height` may be up to 100,000.
    #[pyo3(signature = (width, height, /, *, window=None, **given))]
    fn to_rgba<'py>(
        &self,
        py: Python<'py>,
        width: f64,
        height: f64,
        window: Option<(f64, f64, f64, f64)>,
        given: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let (size, values) = (vector::Size::px(width, height), values(given)?);
        let bytes = raised(py.detach(|| match window {
            Some((x, y, w, h)) => self.0.to_rgba_window(size, vector::Window::px(x, y, w, h), &values),
            None => self.0.to_rgba(size, &values),
        }))?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// The drawing as a PNG file of `width` by `height` pixels (8-bit RGBA: the pixels of `to_rgba`), with each Param at
    /// the value given by name; of the `window` `(x, y, width, height)` only, when there is one.
    #[pyo3(signature = (width, height, /, *, window=None, **given))]
    fn to_png<'py>(
        &self,
        py: Python<'py>,
        width: f64,
        height: f64,
        window: Option<(f64, f64, f64, f64)>,
        given: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let (size, values) = (vector::Size::px(width, height), values(given)?);
        let bytes = raised(py.detach(|| match window {
            Some((x, y, w, h)) => self.0.to_png_window(size, vector::Window::px(x, y, w, h), &values),
            None => self.0.to_png(size, &values),
        }))?;
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
