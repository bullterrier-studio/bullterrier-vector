//! Building a path in one pass: what a generator that draws many outlines (text) uses, since the public `Path::move_to`
//! and the rest copy the whole path at each call.

use super::{Path, Segment, SubPath, no_start};
use crate::Error;
use crate::geometry::{Point, coordinate_point};

/// A path under construction: sub-paths added in order, each a start, segments, and a close.
#[derive(Debug, Default)]
pub(crate) struct Builder {
    subpaths: Vec<SubPath>,
}

impl Builder {
    /// A builder with no sub-paths.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Starts a sub-path at `point`.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `point` is NaN, infinite or beyond ±1e100.
    pub(crate) fn move_to(&mut self, point: Point) -> Result<(), Error> {
        self.subpaths.push(SubPath::open(coordinate_point("point", point)?));
        Ok(())
    }

    /// Adds `segment` to the current sub-path.
    fn add(&mut self, verb: &str, segment: Segment) -> Result<(), Error> {
        self.subpaths
            .last_mut()
            .ok_or_else(|| no_start(verb))?
            .segments
            .push(segment);
        Ok(())
    }

    /// A straight segment to `point`.
    ///
    /// # Errors
    /// As [`Builder::move_to`]; also if there is no sub-path to continue.
    pub(crate) fn line_to(&mut self, point: Point) -> Result<(), Error> {
        self.add("line_to", Segment::Line(coordinate_point("point", point)?))
    }

    /// A quadratic segment to `point`, bent towards `control`.
    ///
    /// # Errors
    /// As [`Builder::move_to`].
    pub(crate) fn quad_to(&mut self, control: Point, point: Point) -> Result<(), Error> {
        self.add(
            "quad_to",
            Segment::Quad(coordinate_point("control", control)?, coordinate_point("point", point)?),
        )
    }

    /// A cubic segment to `point`.
    ///
    /// # Errors
    /// As [`Builder::move_to`].
    pub(crate) fn curve_to(&mut self, first: Point, second: Point, point: Point) -> Result<(), Error> {
        self.add(
            "curve_to",
            Segment::Cubic(
                coordinate_point("control1", first)?,
                coordinate_point("control2", second)?,
                coordinate_point("point", point)?,
            ),
        )
    }

    /// Closes the current sub-path, as [`Path::close`] does.
    pub(crate) fn close(&mut self) {
        if let Some(current) = self.subpaths.pop() {
            self.subpaths.push(current.closed());
        }
    }

    /// The path built.
    pub(crate) fn finish(self) -> Path {
        Path {
            subpaths: self.subpaths,
        }
    }
}
