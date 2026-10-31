//! Constructors: the shapes a person asks for by name.

use super::{Path, Segment, SubPath};
use crate::Error;
use crate::geometry::{Point, finite, finite_point};
use crate::math::sin_cos_deg;

/// The most corners a polygon or a star may have.
// constant: a cap so a wrong call cannot allocate without bound; no outline needs more corners than this
const MAX_CORNERS: i64 = 100_000;
/// The fewest sides a polygon may have.
const MIN_SIDES: i64 = 3;
/// The fewest points a star may have.
const MIN_POINTS: i64 = 2;
/// The handle length, as a fraction of the radius, that makes a cubic Bezier quarter arc follow a circle.
// constant: 4/3 (sqrt 2 - 1), the standard value; the arc then strays from the circle by at most 2.7e-4 of the radius
const KAPPA: f64 = 0.552_284_749_830_793_6;

impl Path {
    /// One open straight segment from `start` to `end`.
    ///
    /// # Errors
    /// [`Error::Parameter`] if a coordinate is NaN or infinite.
    pub fn line(start: impl Into<Point>, end: impl Into<Point>) -> Result<Self, Error> {
        let start = finite_point("start", start)?;
        let end = finite_point("end", end)?;
        Ok(Self::of(SubPath {
            start,
            segments: vec![Segment::Line(end)],
            closed: false,
        }))
    }

    /// Straight segments through `points` in order, closed (with the line back to the first point implied) or open.
    /// A closed polyline whose last point is its first does not store that point twice.
    ///
    /// # Errors
    /// [`Error::Parameter`] if there are fewer than 2 points or a coordinate is NaN or infinite.
    pub fn polyline<P: Into<Point>>(points: impl IntoIterator<Item = P>, closed: bool) -> Result<Self, Error> {
        let points = points
            .into_iter()
            .map(|point| finite_point("points", point))
            .collect::<Result<Vec<_>, _>>()?;
        let Some((&start, rest)) = points.split_first().filter(|(_, rest)| !rest.is_empty()) else {
            return Err(Error::parameter(
                "points",
                format!("needs at least 2 points, got {}", points.len()),
            ));
        };
        let open = SubPath {
            start,
            segments: rest.iter().map(|&point| Segment::Line(point)).collect(),
            closed: false,
        };
        Ok(Self::of(if closed { open.closed() } else { open }))
    }

    /// A closed rectangle with its top-left corner at `(x, y)`, starting at the left end of its top edge and running
    /// clockwise. A `radius` above 0 rounds the corners with quarter arcs.
    ///
    /// # Errors
    /// [`Error::Parameter`] if a number is NaN or infinite, `width` or `height` is negative, or `radius` is negative
    /// or more than half the shorter side.
    pub fn rect(x: f64, y: f64, width: f64, height: f64, radius: f64) -> Result<Self, Error> {
        let (x, y) = (finite("x", x)?, finite("y", y)?);
        let (width, height) = (finite("width", width)?, finite("height", height)?);
        let radius = finite("radius", radius)?;
        for (name, size) in [("width", width), ("height", height)] {
            if size < 0.0 {
                return Err(Error::parameter(name, format!("must be 0 or more, got {size}")));
            }
        }
        let half = width.min(height) / 2.0;
        if !(0.0..=half).contains(&radius) {
            return Err(Error::parameter(
                "radius",
                format!("must be from 0 to half the shorter side ({half}), got {radius}"),
            ));
        }
        let (right, bottom) = (x + width, y + height);
        let handle = KAPPA * radius;
        let at = Point::new;
        let segments = if radius == 0.0 {
            vec![
                Segment::Line(at(right, y)),
                Segment::Line(at(right, bottom)),
                Segment::Line(at(x, bottom)),
            ]
        } else {
            vec![
                Segment::Line(at(right - radius, y)),
                Segment::Cubic(
                    at(right - radius + handle, y),
                    at(right, y + radius - handle),
                    at(right, y + radius),
                ),
                Segment::Line(at(right, bottom - radius)),
                Segment::Cubic(
                    at(right, bottom - radius + handle),
                    at(right - radius + handle, bottom),
                    at(right - radius, bottom),
                ),
                Segment::Line(at(x + radius, bottom)),
                Segment::Cubic(
                    at(x + radius - handle, bottom),
                    at(x, bottom - radius + handle),
                    at(x, bottom - radius),
                ),
                Segment::Line(at(x, y + radius)),
                Segment::Cubic(
                    at(x, y + radius - handle),
                    at(x + radius - handle, y),
                    at(x + radius, y),
                ),
            ]
        };
        let sub = SubPath {
            start: at(x + radius, y),
            segments,
            closed: false,
        };
        Self::of(sub.closed()).finite("width")
    }

    /// A closed circle of four cubic arcs, starting at the top and running clockwise.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `center` is NaN or infinite, `radius` is not a finite number above 0, or the circle
    /// would reach beyond what a number holds.
    pub fn circle(center: impl Into<Point>, radius: f64) -> Result<Self, Error> {
        let center = finite_point("center", center)?;
        let radius = positive("radius", radius)?;
        let (cx, cy) = (center.x, center.y);
        let handle = KAPPA * radius;
        let at = Point::new;
        let sub = SubPath {
            start: at(cx, cy - radius),
            segments: vec![
                Segment::Cubic(
                    at(cx + handle, cy - radius),
                    at(cx + radius, cy - handle),
                    at(cx + radius, cy),
                ),
                Segment::Cubic(
                    at(cx + radius, cy + handle),
                    at(cx + handle, cy + radius),
                    at(cx, cy + radius),
                ),
                Segment::Cubic(
                    at(cx - handle, cy + radius),
                    at(cx - radius, cy + handle),
                    at(cx - radius, cy),
                ),
                Segment::Cubic(
                    at(cx - radius, cy - handle),
                    at(cx - handle, cy - radius),
                    at(cx, cy - radius),
                ),
            ],
            closed: false,
        };
        Self::of(sub.closed()).finite("radius")
    }

    /// A closed regular polygon of `sides` corners on a circle of `radius` round `center`: the first corner at the
    /// top, the rest clockwise.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `center` is NaN or infinite, `radius` is not a finite number above 0, `sides` is not
    /// from 3 to 100 000, or the corners would reach beyond what a number holds.
    pub fn polygon(center: impl Into<Point>, radius: f64, sides: i64) -> Result<Self, Error> {
        let center = finite_point("center", center)?;
        let radius = positive("radius", radius)?;
        let count = count("sides", sides, MIN_SIDES)?;
        Self::round_the_clock(center, count, |_| radius).finite("radius")
    }

    /// A closed star of `points` tips on a circle of radius `outer` round `center`, with a notch of radius `inner`
    /// between each two tips: a tip at the top, the rest clockwise. `inner` may exceed `outer`.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `center` is NaN or infinite, `outer` or `inner` is not a finite number above 0,
    /// `points` is not from 2 to 100 000, or the star would reach beyond what a number holds.
    pub fn star(center: impl Into<Point>, outer: f64, inner: f64, points: i64) -> Result<Self, Error> {
        let center = finite_point("center", center)?;
        let (outer, inner) = (positive("outer", outer)?, positive("inner", inner)?);
        let count = count("points", points, MIN_POINTS)?;
        Self::round_the_clock(center, 2 * count, |index| if index % 2 == 0 { outer } else { inner }).finite("outer")
    }

    /// A closed polygon of `corners` vertices evenly round `center`, the first straight up, vertex `index` at
    /// `radius(index)` from it.
    fn round_the_clock(center: Point, corners: u32, radius: impl Fn(u32) -> f64) -> Self {
        let mut vertices = (0..corners).map(|index| {
            let (sin, cos) = sin_cos_deg(360.0 * f64::from(index) / f64::from(corners));
            Point::new(center.x + radius(index) * sin, center.y - radius(index) * cos)
        });
        let start = vertices.next().unwrap_or(center);
        let segments = vertices.map(Segment::Line).collect();
        Self::of(
            SubPath {
                start,
                segments,
                closed: false,
            }
            .closed(),
        )
    }

    fn of(subpath: SubPath) -> Self {
        Self {
            subpaths: vec![subpath],
        }
    }

    /// This path, or the error naming `parameter` if a computed coordinate is NaN or infinite.
    pub(super) fn finite(self, parameter: &str) -> Result<Self, Error> {
        if self
            .all_points()
            .all(|point| point.x.is_finite() && point.y.is_finite())
        {
            Ok(self)
        } else {
            Err(Error::parameter(
                parameter,
                "too large: the outline would reach beyond what a finite number holds",
            ))
        }
    }

    /// Every point of every segment, on or off the curve.
    fn all_points(&self) -> impl Iterator<Item = Point> + '_ {
        self.subpaths.iter().flat_map(|sub| {
            std::iter::once(sub.start).chain(sub.segments.iter().flat_map(|segment| match *segment {
                Segment::Line(end) => vec![end],
                Segment::Quad(control, end) => vec![control, end],
                Segment::Cubic(control1, control2, end) => vec![control1, control2, end],
            }))
        })
    }
}

/// `value`, or the error naming `parameter` unless it is a finite number above 0.
fn positive(parameter: &str, value: f64) -> Result<f64, Error> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(Error::parameter(
            parameter,
            format!("must be a finite number above 0, got {value}"),
        ))
    }
}

/// `value` as a number of corners, or the error naming `parameter` unless it is from `min` to `MAX_CORNERS`.
fn count(parameter: &str, value: i64, min: i64) -> Result<u32, Error> {
    u32::try_from(value)
        .ok()
        .filter(|_| (min..=MAX_CORNERS).contains(&value))
        .ok_or_else(|| {
            Error::parameter(
                parameter,
                format!("must be a whole number from {min} to {MAX_CORNERS}, got {value}"),
            )
        })
}
