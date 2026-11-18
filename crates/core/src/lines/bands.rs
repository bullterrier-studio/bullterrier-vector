//! `Path::bands`: lines whose width follows a tone. A carrier (straight lines, waves, rings, a spiral or rays) is laid
//! over a region, cut to it, and each piece becomes a closed outline as wide at a point as the tone there asks.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the counts of lines and samples are checked against MAX_SAMPLES before they are used"
)]

use super::layout::{Distances, Family, Layout, corners, middle, polar};
use crate::field::{Field, length, once, read};
use crate::geometry::{MAX_COORDINATE, Point, coordinate_point};
use crate::path::Path;
use crate::{Error, Expr, Values};
use std::f64::consts::TAU;

/// How far apart the points of a carrier are, as a share of the spacing between lines.
// constant: a quarter spacing follows a tone that changes over about a spacing, and keeps a wave or a ring smooth
const STEP: f64 = 0.25;

/// The fewest points on a ring, so that a small ring is round.
// constant: 32 points are 11 degrees apart: a polygon the eye reads as a circle at any width the ring can have
const RING_POINTS: f64 = 32.0;

/// The most points all the carriers of one call may have.
// constant: 2 million points are made, cut and outlined, a few hundred MB at the most; a print at 300 dpi of a
// letter page at a 0.25 spacing step stays under it
const MAX_SAMPLES: f64 = 2_000_000.0;

/// What to draw: a carrier, the spacing between its lines, the tone that sets their width and the width at tone 0 and 1.
#[derive(Debug, Clone)]
pub struct Bands {
    family: Family,
    spacing: Expr,
    tone: Field,
    width: (Expr, Expr),
    angle: Option<Expr>,
    wavelength: Option<Expr>,
    amplitude: Option<Expr>,
    centre: Option<(Expr, Expr)>,
}

impl Bands {
    fn made(
        family: Family,
        spacing: impl Into<Expr>,
        tone: Field,
        width: (impl Into<Expr>, impl Into<Expr>),
    ) -> Result<Self, Error> {
        Ok(Self {
            family,
            spacing: once("spacing", &spacing.into())?,
            tone,
            width: (once("width", &width.0.into())?, once("width", &width.1.into())?),
            angle: None,
            wavelength: None,
            amplitude: None,
            centre: None,
        })
    }

    /// Whether the carrier `family` (`"lines"`, `"waves"`, `"rings"`, `"spiral"` or `"rays"`) reads the optional
    /// argument `argument` (`angle`, `centre`, `wavelength` or `amplitude`): the one place the rule lives, for the
    /// builder, a document and the bindings.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `argument` if the family does not read it.
    pub fn refuse_unread(family: &str, argument: &str) -> Result<(), Error> {
        Family::from_name(family).map_or(Ok(()), |known| known.refuse_unread(argument, "family", family))
    }

    /// Straight lines `spacing` design pixels apart, along the direction of [`Bands::angle`] (0: left to right), the
    /// first half a spacing from the region's side. Each is as wide at a point as `width` says: `width.0` where
    /// the `tone` is 0, `width.1` where it is 1, linear between and held beyond (a tone is read where the centre of the
    /// line is). Every number is a number or an expression of Params, read once.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `spacing` or `width` if it is a per-item number.
    pub fn lines(
        spacing: impl Into<Expr>,
        tone: Field,
        width: (impl Into<Expr>, impl Into<Expr>),
    ) -> Result<Self, Error> {
        Self::made(Family::Lines, spacing, tone, width)
    }

    /// As [`Bands::lines`], each line a sine across the lines: `amplitude` design pixels high, one `wavelength`
    /// long, in phase at the origin of the lines' direction.
    ///
    /// # Errors
    /// As [`Bands::lines`], and naming `wavelength` or `amplitude` if it is a per-item number.
    pub fn waves(
        spacing: impl Into<Expr>,
        tone: Field,
        width: (impl Into<Expr>, impl Into<Expr>),
        wavelength: impl Into<Expr>,
        amplitude: impl Into<Expr>,
    ) -> Result<Self, Error> {
        let mut bands = Self::made(Family::Waves, spacing, tone, width)?;
        bands.wavelength = Some(once("wavelength", &wavelength.into())?);
        bands.amplitude = Some(once("amplitude", &amplitude.into())?);
        Ok(bands)
    }

    /// Rings round [`Bands::centre`] (the middle of the region's bounding box by default), `spacing` apart, the first
    /// half a spacing out.
    ///
    /// # Errors
    /// As [`Bands::lines`].
    pub fn rings(
        spacing: impl Into<Expr>,
        tone: Field,
        width: (impl Into<Expr>, impl Into<Expr>),
    ) -> Result<Self, Error> {
        Self::made(Family::Rings, spacing, tone, width)
    }

    /// An Archimedean spiral out of [`Bands::centre`], its arms `spacing` apart along any ray from the centre; the
    /// spiral is at radius 0 at angle [`Bands::angle`], and turns clockwise on the screen.
    ///
    /// # Errors
    /// As [`Bands::lines`].
    pub fn spiral(
        spacing: impl Into<Expr>,
        tone: Field,
        width: (impl Into<Expr>, impl Into<Expr>),
    ) -> Result<Self, Error> {
        Self::made(Family::Spiral, spacing, tone, width)
    }

    /// Straight rays out of [`Bands::centre`], as many as are `spacing` apart round the farthest corner of the
    /// region's bounding box, the first at angle [`Bands::angle`].
    ///
    /// # Errors
    /// As [`Bands::lines`].
    pub fn rays(
        spacing: impl Into<Expr>,
        tone: Field,
        width: (impl Into<Expr>, impl Into<Expr>),
    ) -> Result<Self, Error> {
        Self::made(Family::Rays, spacing, tone, width)
    }

    /// The direction of the lines and waves, or the turn of the spiral and the rays, in degrees (default 0), clockwise
    /// on the screen.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `angle` for rings, which have no direction; or if it is a per-item number.
    pub fn angle(mut self, degrees: impl Into<Expr>) -> Result<Self, Error> {
        Self::refuse_unread(self.family.name(), "angle")?;
        self.angle = Some(once("angle", &degrees.into())?);
        Ok(self)
    }

    /// The centre of rings, the spiral and rays (default: the middle of the region's bounding box).
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `centre` for lines and waves, which have none.
    pub fn centre(mut self, centre: impl Into<Point>) -> Result<Self, Error> {
        Self::refuse_unread(self.family.name(), "centre")?;
        let centre = centre.into();
        self.centre = Some((Expr::from(centre.x), Expr::from(centre.y)));
        Ok(self)
    }

    pub(crate) fn family(&self) -> Family {
        self.family
    }

    pub(crate) fn spacing_of(&self) -> &Expr {
        &self.spacing
    }

    pub(crate) fn tone_of(&self) -> &Field {
        &self.tone
    }

    pub(crate) fn width_of(&self) -> &(Expr, Expr) {
        &self.width
    }

    pub(crate) fn angle_of(&self) -> Option<&Expr> {
        self.angle.as_ref()
    }

    pub(crate) fn wavelength_of(&self) -> Option<&Expr> {
        self.wavelength.as_ref()
    }

    pub(crate) fn amplitude_of(&self) -> Option<&Expr> {
        self.amplitude.as_ref()
    }

    pub(crate) fn centre_of(&self) -> Option<&(Expr, Expr)> {
        self.centre.as_ref()
    }

    /// The numbers of these bands with the Params at `values`.
    fn plan(&self, values: &Values) -> Result<Plan, Error> {
        Ok(Plan {
            family: self.family,
            spacing: read("spacing", &self.spacing, values)?,
            width: (
                read("width", &self.width.0, values)?,
                read("width", &self.width.1, values)?,
            ),
            angle: self
                .angle
                .as_ref()
                .map(|e| read("angle", e, values))
                .transpose()?
                .unwrap_or(0.0),
            wavelength: self
                .wavelength
                .as_ref()
                .map(|e| read("wavelength", e, values))
                .transpose()?,
            amplitude: self
                .amplitude
                .as_ref()
                .map(|e| read("amplitude", e, values))
                .transpose()?,
            centre: self
                .centre
                .as_ref()
                .map(|(x, y)| Ok::<_, Error>(Point::new(read("centre", x, values)?, read("centre", y, values)?)))
                .transpose()?,
        })
    }
}

/// The numbers of a [`Bands`] worked out.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Plan {
    pub(crate) family: Family,
    pub(crate) spacing: f64,
    pub(crate) width: (f64, f64),
    pub(crate) angle: f64,
    pub(crate) wavelength: Option<f64>,
    pub(crate) amplitude: Option<f64>,
    pub(crate) centre: Option<Point>,
}

impl Plan {
    /// Where the rows of this plan lie.
    fn layout(&self) -> Layout {
        Layout {
            family: self.family,
            spacing: self.spacing,
            angle: self.angle,
            wave: self.wavelength.zip(self.amplitude),
        }
    }

    fn check(&self) -> Result<(), Error> {
        if !(self.spacing.is_finite() && self.spacing > 0.0 && self.spacing <= MAX_COORDINATE) {
            return Err(Error::parameter(
                "spacing",
                format!("must be above 0, got {}", self.spacing),
            ));
        }
        for width in [self.width.0, self.width.1] {
            if !(width.is_finite() && (0.0..=MAX_COORDINATE).contains(&width)) {
                return Err(Error::parameter(
                    "width",
                    format!("must be from 0 to {MAX_COORDINATE}, got {width}"),
                ));
            }
        }
        if !(self.angle.is_finite() && self.angle.abs() <= MAX_COORDINATE) {
            return Err(Error::parameter(
                "angle",
                format!("must be a number, got {}", self.angle),
            ));
        }
        if let Some(wavelength) = self.wavelength
            && !(wavelength.is_finite() && wavelength > 0.0 && wavelength <= MAX_COORDINATE)
        {
            return Err(Error::parameter(
                "wavelength",
                format!("must be above 0, got {wavelength}"),
            ));
        }
        if let Some(amplitude) = self.amplitude
            && !(amplitude.is_finite() && amplitude.abs() <= MAX_COORDINATE)
        {
            return Err(Error::parameter(
                "amplitude",
                format!("must be a number, got {amplitude}"),
            ));
        }
        if let Some(centre) = self.centre {
            coordinate_point("centre", centre)?;
        }
        Ok(())
    }
}

impl Path {
    /// Lines whose width follows a tone, inside the region this path fills (nonzero rule): the carrier of `bands` laid over the region, cut to it
    /// (a line is kept where its centre is inside), each piece outlined as wide at every point as the tone there asks (see
    /// [`Bands::lines`]). A piece is one closed outline; a ring that lies whole inside the region is two loops, the inner
    /// one the other way, so that the nonzero rule leaves the hole open. A Param is read at its default here; a `bands`
    /// step reads it with the values it draws with. The result is for filling. An empty path gives an empty path.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `spacing` if it is not above 0 and a coordinate, if the carrier would need more than
    /// 2 million points, or if a step of it would not move at the region's coordinates; naming `width` unless both
    /// widths are numbers from 0 to 1e100; `angle`, `wavelength` (above 0), `amplitude` or `centre` unless it is a
    /// number within ±1e100; naming `spacing` past `MAX_SEGMENTS` points.
    pub fn bands(&self, bands: &Bands) -> Result<Path, Error> {
        let plan = bands.plan(&Values::new())?;
        self.bands_charged(&plan, &bands.tone, &mut crate::field::segment_budget("spacing"))
    }

    /// As [`Path::bands`] with the numbers worked out, calling `charge` with the points made before they are kept.
    pub(crate) fn bands_charged(
        &self,
        plan: &Plan,
        tone: &Field,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Path, Error> {
        plan.check()?;
        let Ok(bounds) = self.bounds() else {
            return Ok(Path::new());
        };
        if !self.may_fill() {
            return Ok(Path::new());
        }
        let carrier = carrier(plan, bounds, charge)?;
        let inside = carrier.within_named(self, "spacing")?;
        tone.charge_reads("spacing", "bands", inside.weight() as f64)?;
        let mut outlines = Vec::new();
        for line in inside.flattened(charge)? {
            if line.points.len() > 1 {
                for loop_ in outline(&line.points, line.closed, tone, plan.width) {
                    charge(loop_.len())?;
                    outlines.push(Path::polyline(loop_, true)?);
                }
            }
        }
        Ok(Path::joined(outlines.iter()))
    }
}

/// A carrier line: its points and whether it closes on itself.
type Carried = (Vec<Point>, bool);

/// The error of a carrier that would have more points than `MAX_SAMPLES`; `name` is the argument that made it so.
fn too_many(name: &str) -> Error {
    Error::parameter(
        name,
        format!("would need over {MAX_SAMPLES} points to cover a region this large"),
    )
}

/// The carrier's lines over the box `bounds`, whole, to be cut to the region.
fn carrier(
    plan: &Plan,
    bounds: crate::Rect,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Path, Error> {
    let step = STEP * plan.spacing;
    let corners = corners(bounds);
    let reach = corners.iter().map(|p| p.x.abs().max(p.y.abs())).fold(0.0, f64::max);
    if reach + step <= reach {
        return Err(Error::parameter(
            "spacing",
            format!("is too small for coordinates near {reach}: a step of it would not move"),
        ));
    }
    let lines = match plan.family {
        Family::Lines | Family::Waves => straight(plan, &corners, step)?,
        Family::Rings | Family::Spiral | Family::Rays => round(plan, bounds, &corners, step)?,
    };
    let mut paths = Vec::with_capacity(lines.len());
    for (points, closed) in lines {
        charge(points.len())?;
        paths.push(Path::polyline(points, closed)?);
    }
    Ok(Path::joined(paths.iter()))
}

/// Lines or waves across the box with these `corners`.
fn straight(plan: &Plan, corners: &[Point; 4], step: f64) -> Result<Vec<Carried>, Error> {
    let layout = plan.layout();
    let (u_min, u_max) = range(corners.iter().map(|p| layout.frame(*p).0));
    let (n_min, n_max) = range(corners.iter().map(|p| layout.frame(*p).1));
    let count = ((n_max - n_min - plan.spacing / 2.0) / plan.spacing).floor() + 1.0;
    let samples = ((u_max - u_min) / step).ceil() + 1.0;
    let points = count.max(1.0) * samples;
    if points.is_nan() || points > MAX_SAMPLES {
        return Err(too_many("spacing"));
    }
    let mut lines = Vec::new();
    for k in 0..count.max(0.0) as usize {
        let offset = n_min + plan.spacing / 2.0 + k as f64 * plan.spacing;
        let points = (0..samples as usize)
            .map(|j| layout.straight((u_min + j as f64 * step).min(u_max), offset))
            .collect();
        lines.push((points, false));
    }
    Ok(lines)
}

/// Rings, a spiral or rays about the centre.
fn round(plan: &Plan, bounds: crate::Rect, corners: &[Point; 4], step: f64) -> Result<Vec<Carried>, Error> {
    let layout = plan.layout();
    let centre = plan.centre.unwrap_or_else(|| middle(bounds));
    let distances = Distances::of(centre, bounds, corners);
    let (d_min, d_max) = (distances.near, distances.far);
    // the rows are k = first..=last at radius (k + 1/2) spacing, each of TAU radius / step points at least RING_POINTS;
    // counted in f64 and before any is made, with no difference of squares to cancel
    let round = layout.round_rows(centre, bounds, 0.0, step)?;
    let (first, rings) = (round.first, round.rows);
    let last = first + rings - 1.0;
    let (low, high) = (d_min / plan.spacing, d_max / plan.spacing);
    let samples = match plan.family {
        Family::Rings => rings * RING_POINTS + TAU / STEP * rings * (first + last + 1.0) / 2.0,
        Family::Spiral => std::f64::consts::PI * (d_max - d_min) * (d_max + d_min) / plan.spacing / step,
        _ => TAU * high.max(1.0) * ((d_max - d_min) / step + 1.0),
    };
    if samples.is_nan() || samples > MAX_SAMPLES {
        // a centre away from the region is what stretches a ring or an arm out: name it, when the default would do
        let name = if plan.centre.is_some() && d_min > 0.0 {
            "centre"
        } else {
            "spacing"
        };
        return Err(too_many(name));
    }
    let at = |radius: f64, angle: f64| polar(centre, radius, angle);
    let turn = layout.turn();
    let mut lines = Vec::new();
    match plan.family {
        Family::Rings => {
            for k in first as usize.. {
                let radius = layout.ring_radius(k as f64);
                if radius > d_max {
                    break;
                }
                let count = (TAU * radius / step).ceil().max(RING_POINTS) as usize;
                lines.push((
                    (0..count).map(|i| at(radius, TAU * i as f64 / count as f64)).collect(),
                    true,
                ));
            }
        }
        Family::Spiral => {
            let pitch = plan.spacing / TAU;
            let mut theta = low * TAU;
            let mut points = Vec::new();
            while theta < high * TAU {
                let radius = layout.spiral_radius(theta);
                points.push(at(radius, theta + turn));
                theta += step / length(radius, pitch);
            }
            points.push(at(d_max, high * TAU + turn));
            lines.push((points, false));
        }
        _ => {
            let count = Layout::ray_count(high) as usize;
            let along = ((d_max - d_min) / step).ceil() + 1.0;
            for j in 0..count {
                let angle = turn + TAU * j as f64 / count as f64;
                let ray = (0..along as usize).map(|i| at((d_min + i as f64 * step).min(d_max), angle));
                lines.push((ray.collect(), false));
            }
        }
    }
    Ok(lines)
}

fn range(values: impl Iterator<Item = f64>) -> (f64, f64) {
    values.fold((f64::MAX, f64::MIN), |(low, high), value| {
        (low.min(value), high.max(value))
    })
}

/// The closed outlines of a line of `points` as wide at each as `width` says for the tone there: one for an open line, two
/// for a closed one (the second the other way, a hole inside the first).
fn outline(points: &[Point], closed: bool, tone: &Field, width: (f64, f64)) -> Vec<Vec<Point>> {
    let count = points.len();
    let (mut left, mut right) = (Vec::with_capacity(count), Vec::with_capacity(count));
    let mut normal = (0.0, 1.0);
    for (i, point) in points.iter().enumerate() {
        let (before, after) = match (i, closed) {
            (0, false) => (points[0], points[1]),
            (_, false) if i == count - 1 => (points[i - 1], points[i]),
            _ => (points[(i + count - 1) % count], points[(i + 1) % count]),
        };
        let (dx, dy) = (after.x - before.x, after.y - before.y);
        let size = length(dx, dy);
        if size > 0.0 {
            normal = (-dy / size, dx / size);
        }
        let share = tone.at(*point).clamp(0.0, 1.0);
        let half = f64::midpoint(width.0, (width.1 - width.0) * share);
        left.push(Point::new(point.x + normal.0 * half, point.y + normal.1 * half));
        right.push(Point::new(point.x - normal.0 * half, point.y - normal.1 * half));
    }
    if closed {
        right.reverse();
        vec![left, right]
    } else {
        right.reverse();
        left.extend(right);
        vec![left]
    }
}
