//! Moving, turning and scaling a path, and passing its points through a function.

use super::{Path, Segment, SubPath};
use crate::Error;
use crate::geometry::{Point, Scale, coordinate, coordinate_point, finite};
use crate::math::sin_cos_deg;

impl Path {
    /// This path moved `dx` to the right and `dy` down.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `dx` or `dy` is NaN, infinite or beyond ±1e100, or the move would carry a point beyond
    /// ±1e100.
    pub fn translate(&self, dx: f64, dy: f64) -> Result<Self, Error> {
        let (dx, dy) = (coordinate("dx", dx)?, coordinate("dy", dy)?);
        self.map_each(|point| Ok(Point::new(point.x + dx, point.y + dy)))?
            .within_limit("dx")
    }

    /// This path turned `angle` degrees clockwise (on the screen, with `y` down) about `about`, which defaults to the
    /// middle of its bounds. Turns of any size and sign are allowed and wrap; multiples of 90 degrees are exact. An
    /// empty path is returned as it is.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `angle` is NaN or infinite, `about` is NaN, infinite or beyond ±1e100, or the turn
    /// would carry a point beyond ±1e100.
    pub fn rotate(&self, angle: f64, about: Option<Point>) -> Result<Self, Error> {
        let angle = finite("angle", angle)?;
        let (sin, cos) = sin_cos_deg(angle);
        self.about(about, |from_pivot| {
            Point::new(
                from_pivot.x * cos - from_pivot.y * sin,
                from_pivot.x * sin + from_pivot.y * cos,
            )
        })?
        .within_limit("angle")
    }

    /// This path scaled by `by` about `about`, which defaults to the middle of its bounds: every point moves to
    /// `by` times its distance from the pivot. `by` is one factor for both directions or an `(x, y)` pair; a negative
    /// factor mirrors, and so turns a clockwise outline into a counter-clockwise one. An empty path is returned as
    /// it is.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `by` has a factor that is 0, NaN or infinite, `about` is NaN, infinite or beyond
    /// ±1e100, or the scaling would carry a point beyond ±1e100.
    pub fn scale(&self, by: impl Into<Scale>, about: Option<Point>) -> Result<Self, Error> {
        let Scale { x, y } = by.into();
        for factor in [x, y] {
            if finite("by", factor)? == 0.0 {
                return Err(Error::parameter("by", format!("must not be 0, got {factor}")));
            }
        }
        self.about(about, |from_pivot| Point::new(from_pivot.x * x, from_pivot.y * y))?
            .within_limit("by")
    }

    /// This path with every point, on and off the curve, replaced by what `map` returns for it: the start of each
    /// sub-path, then each segment's control points and end, in the order they were written. Straight segments stay
    /// straight and curves stay curves, whatever the function does to their points.
    ///
    /// # Errors
    /// What `map` returns as an error, unchanged (the first one, and no point after it is mapped);
    /// [`Error::Parameter`] naming `fn` if `map` returns a point with a coordinate that is NaN, infinite or beyond ±1e100.
    pub fn map_points(&self, mut map: impl FnMut(Point) -> Result<Point, Error>) -> Result<Self, Error> {
        self.map_each(|point| coordinate_point("fn", map(point)?))
    }

    /// This path with each point moved by `turn`, which is given the point's offset from the pivot and returns the
    /// new offset; the pivot is `about` or the middle of the bounds. An empty path has no pivot and no points.
    fn about(&self, about: Option<Point>, turn: impl Fn(Point) -> Point) -> Result<Self, Error> {
        let about = about.map(|point| coordinate_point("about", point)).transpose()?;
        if self.subpaths.is_empty() {
            return Ok(self.clone());
        }
        let pivot = if let Some(point) = about {
            point
        } else {
            let bounds = self.bounds()?;
            Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0)
        };
        self.map_each(|point| {
            let turned = turn(Point::new(point.x - pivot.x, point.y - pivot.y));
            Ok(Point::new(pivot.x + turned.x, pivot.y + turned.y))
        })
    }

    /// This path with every point replaced by what `map` returns for it, in the order they are written. A closed
    /// sub-path is closed again afterwards, so that a line the mapping brings back to the start is not kept.
    fn map_each(&self, mut map: impl FnMut(Point) -> Result<Point, Error>) -> Result<Self, Error> {
        let mut subpaths = Vec::with_capacity(self.subpaths.len());
        for subpath in &self.subpaths {
            let start = map(subpath.start)?;
            let mut segments = Vec::with_capacity(subpath.segments.len());
            for segment in &subpath.segments {
                segments.push(match *segment {
                    Segment::Line(end) => Segment::Line(map(end)?),
                    Segment::Quad(control, end) => Segment::Quad(map(control)?, map(end)?),
                    Segment::Cubic(control1, control2, end) => {
                        Segment::Cubic(map(control1)?, map(control2)?, map(end)?)
                    }
                });
            }
            let mapped = SubPath {
                start,
                segments,
                closed: false,
            };
            subpaths.push(if subpath.closed { mapped.closed() } else { mapped });
        }
        Ok(Self { subpaths })
    }
}
