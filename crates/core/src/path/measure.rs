//! Measuring along a path: its length, the point and direction at a position, and where a point lies.

use super::Path;
use super::walk::Walk;
use crate::Error;
use crate::geometry::{Point, coordinate_point, fraction};

impl Path {
    /// The length of the outline in design pixels, measured along a flattening that stays within a hundredth of a
    /// pixel of every curve. A closed sub-path counts the line back to its start; an empty path has length 0.
    #[must_use]
    pub fn length(&self) -> f64 {
        self.walk(None).length
    }

    /// The point a fraction `at` (0 to 1) of the way along the outline. Sub-paths are walked in the order they were
    /// built; a position on the boundary between two belongs to the later one. A closed sub-path is walked from its
    /// topmost point (the leftmost of equally high ones), in the direction it is written, so the start does not move
    /// when the outline changes elsewhere. With `near`, a closed sub-path is walked from the point of it nearest
    /// `near` instead (of equally near places, the first as written); this holds for [`Path::tangent_at`],
    /// [`Path::locate`] and [`Path::slice`] too. A path with no length has one point, its first.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `at` is outside 0 to 1 or NaN, or `near` is NaN, infinite or beyond ±1e100; [`Error::Geometry`] if
    /// the path is empty.
    pub fn point_at(&self, at: f64, near: Option<Point>) -> Result<Point, Error> {
        let at = fraction("at", at)?;
        let walk = self.nonempty_walk("point_at", near)?;
        Ok(match walk.spot(at * walk.length) {
            Some(spot) => walk.point(spot),
            None => walk.runs[0].subpath.start,
        })
    }

    /// The unit direction of travel a fraction `at` of the way along the outline. At a corner it is the direction
    /// leaving it, the next segment's; at `at = 1` it is the direction of the last segment of an open sub-path, and on
    /// a closed one the same as at 0. Segments of no length are passed over.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `at` is outside 0 to 1 or NaN, or `near` is NaN, infinite or beyond ±1e100; [`Error::Geometry`] if
    /// the path is empty or has no length, and so no direction.
    pub fn tangent_at(&self, at: f64, near: Option<Point>) -> Result<Point, Error> {
        let at = fraction("at", at)?;
        let walk = self.nonempty_walk("tangent_at", near)?;
        let spot = walk
            .spot(at * walk.length)
            .ok_or_else(|| Error::geometry("tangent_at", "the path has no length, so no direction"))?;
        Ok(walk.direction(spot))
    }

    /// The walk of a path that has something on it, from the point nearest `near` if there is one.
    pub(super) fn nonempty_walk(&self, verb: &str, near: Option<Point>) -> Result<Walk<'_>, Error> {
        let near = near.map(|near| coordinate_point("near", near)).transpose()?;
        if self.subpaths.is_empty() {
            Err(Error::geometry(verb, "the path is empty"))
        } else {
            Ok(self.walk(near))
        }
    }

    /// The position, a fraction 0 to 1 of the length, of the place on the outline nearest `point`. Of equally near
    /// places the smallest position wins, and the start of a closed sub-path is 0, never 1. A path with no length
    /// is located at 0. `near` is as for [`Path::point_at`].
    ///
    /// # Errors
    /// [`Error::Parameter`] if `point` or `near` is NaN, infinite or beyond ±1e100; [`Error::Geometry`] if the path is empty.
    pub fn locate(&self, point: impl Into<Point>, near: Option<Point>) -> Result<f64, Error> {
        let point = coordinate_point("point", point)?;
        Ok(self.nonempty_walk("locate", near)?.locate(point))
    }
}
