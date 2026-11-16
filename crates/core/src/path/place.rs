//! Placing: copies of shapes at even spacing along a guide path, each carried to its stop without bending. The guide is
//! the one [`Path::along`] walks, so a place and its direction are the same there.

use super::Path;
use super::bend::Guide;
use crate::Error;
use crate::geometry::{MAX_COORDINATE, Point, coordinate_point};

/// How many copies a drawing is placed in along a path, or how far apart they are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Spacing {
    /// This many copies, at least 1. On an open path they include both ends (one copy is at the start); on a closed
    /// path they are evenly round it, none repeated at the end.
    Count(usize),
    /// One copy every this many design pixels, from the start: at 0, s, 2s, ... as far as the path goes (an open path's
    /// end only where it falls on one). On a closed path as many as fit without two closer than the spacing,
    /// at least one. The spacing is above 0 and at most 1e100.
    Every(f64),
}

/// The relative slack that lets a position a rounding error short of the end count as on it.
// constant: a trillionth of the length, far below any visible distance and far above f64 rounding of a sum of lengths
const SLACK: f64 = 1e-12;

/// Where a copy goes: the point of the guide and, if the copy turns, the unit direction there.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Stop {
    at: Point,
    direction: Option<Point>,
}

impl Stop {
    /// `point`, as the stamp whose anchor is `anchor` is carried here: moved so the anchor is at the stop, turned
    /// about it so its x axis runs along the direction.
    fn carry(&self, anchor: Point, point: Point) -> Point {
        let (u, v) = (point.x - anchor.x, point.y - anchor.y);
        let (u, v) = self
            .direction
            .map_or((u, v), |d| (u * d.x - v * d.y, u * d.y + v * d.x));
        Point::new(self.at.x + u, self.at.y + v)
    }

    /// `point` carried and checked to be a coordinate.
    pub(crate) fn carried(&self, anchor: Point, point: Point) -> Result<Point, Error> {
        coordinate_point("start", self.carry(anchor, point))
    }
}

impl Path {
    /// This path carried to `stop`, the stamp's `anchor` going there.
    pub(crate) fn placed(&self, stop: &Stop, anchor: Point) -> Result<Self, Error> {
        self.map_points(|point| Ok(stop.carry(anchor, point)))
            .map_err(|_| Error::parameter("start", "a placed point would be beyond ±1e100"))
    }
}

impl Guide {
    /// How many copies `spacing` makes along this guide.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `count` if it is 0, or `spacing` if it is not above 0 and at most 1e100.
    pub(crate) fn stops(&self, spacing: Spacing) -> Result<usize, Error> {
        match spacing {
            Spacing::Count(0) => Err(Error::parameter("count", "must be at least 1, got 0")),
            Spacing::Count(count) => Ok(count),
            Spacing::Every(every) => {
                if !(every > 0.0 && every <= MAX_COORDINATE) {
                    return Err(Error::parameter(
                        "spacing",
                        format!("must be above 0 and at most {MAX_COORDINATE}, got {every}"),
                    ));
                }
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a saturating cast of a non-negative number; the caller charges it against the limits"
                )]
                let whole = (self.length / every * (1.0 + SLACK)).floor() as usize;
                Ok(if self.closed {
                    whole.max(1)
                } else {
                    whole.saturating_add(1)
                })
            }
        }
    }

    /// The stop number `index` of `count` (from `stops`) of `spacing`, `start` lengths of the guide on, turning to the
    /// guide's direction there if `turn`.
    pub(crate) fn stop(&self, spacing: Spacing, count: usize, index: usize, start: f64, turn: bool) -> Stop {
        #[expect(clippy::cast_precision_loss, reason = "counts are far below 2^53")]
        let (index, count) = (index as f64, count as f64);
        let along = match spacing {
            Spacing::Every(every) => index * every,
            Spacing::Count(_) if self.closed => self.length * index / count,
            Spacing::Count(_) if count > 1.0 => self.length * index / (count - 1.0),
            Spacing::Count(_) => 0.0,
        };
        let (at, direction) = self.frame(start * self.length + along);
        Stop {
            at,
            direction: turn.then_some(direction),
        }
    }
}
