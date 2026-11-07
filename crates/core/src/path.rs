//! `Path`: a Bezier outline as a plain value.

use crate::Error;
use crate::geometry::{Point, coordinate_point};
use crate::key::{Key, Keyed};

mod arc;
mod bend;
mod bounds;
mod build;
mod curve;
mod measure;
mod polyline;
mod read;
mod region;
mod shapes;
mod slice;
mod transform;
mod walk;
mod write;

pub(crate) use bend::Guide;
pub(crate) use build::Builder;
pub(crate) use curve::FLATNESS;
pub(crate) use polyline::Polyline;
pub(crate) use region::Region;
pub(crate) use transform::{rotate_point, scale_point};
pub(crate) use write::number;

/// One piece of a sub-path, ending at the point it names last. The pieces are absolute: each starts where the one
/// before ended.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Segment {
    Line(Point),
    Quad(Point, Point),
    Cubic(Point, Point, Point),
}

impl Segment {
    fn end(self) -> Point {
        match self {
            Self::Line(end) | Self::Quad(_, end) | Self::Cubic(_, _, end) => end,
        }
    }
}

/// A start, the segments that follow and whether the outline is closed. A closed sub-path never stores a last
/// `Line` back to its start: closing implies it.
#[derive(Debug, Clone, PartialEq)]
struct SubPath {
    start: Point,
    segments: Vec<Segment>,
    closed: bool,
}

impl SubPath {
    fn open(start: Point) -> Self {
        Self {
            start,
            segments: Vec::new(),
            closed: false,
        }
    }

    /// This sub-path closed, without the last straight segments that only return to the start.
    fn closed(mut self) -> Self {
        while self.segments.last() == Some(&Segment::Line(self.start)) {
            self.segments.pop();
        }
        self.closed = true;
        self
    }
}

/// A Bezier outline: one or more sub-paths, each open or closed, made of straight, quadratic and cubic segments with
/// coordinates that are finite and within ±1e100 (design pixels, `y` pointing down): the squares and sums that
/// measuring does stay finite.
///
/// A `Path` is a plain value. Every verb leaves the path it is called on alone and returns a new one. A path is built
/// from [`Path::start`] (or [`Path::new`] and [`Path::move_to`]) with [`Path::line_to`], [`Path::quad_to`],
/// [`Path::curve_to`] and [`Path::close`], or made by a constructor such as [`Path::circle`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path {
    subpaths: Vec<SubPath>,
}

impl Keyed for Path {
    fn write_key(&self, key: &mut Key) {
        key.count(self.subpaths.len());
        for subpath in &self.subpaths {
            key.of(&subpath.start);
            key.word(u64::from(subpath.closed));
            key.count(subpath.segments.len());
            for segment in &subpath.segments {
                match *segment {
                    Segment::Line(end) => {
                        key.word(0);
                        key.of(&end);
                    }
                    Segment::Quad(control, end) => {
                        key.word(1);
                        key.of(&control);
                        key.of(&end);
                    }
                    Segment::Cubic(first, second, end) => {
                        key.word(2);
                        key.of(&first);
                        key.of(&second);
                        key.of(&end);
                    }
                }
            }
        }
    }
}

impl Path {
    /// An empty path: no sub-paths.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A path with one sub-path that holds only `point`.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `point` is NaN, infinite or beyond ±1e100.
    pub fn start(point: impl Into<Point>) -> Result<Self, Error> {
        Self::new().move_to(point)
    }

    /// This path followed by a new sub-path that holds only `point`.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `point` is NaN, infinite or beyond ±1e100.
    pub fn move_to(&self, point: impl Into<Point>) -> Result<Self, Error> {
        let point = coordinate_point("point", point)?;
        let mut next = self.clone();
        next.subpaths.push(SubPath::open(point));
        Ok(next)
    }

    /// This path with a straight segment to `point`.
    ///
    /// After [`Path::close`] the segment starts a new sub-path at the start of the closed one, as in SVG.
    ///
    /// # Errors
    /// [`Error::Parameter`] if there is no sub-path yet (call [`Path::move_to`] first) or `point` is
    /// NaN, infinite or beyond ±1e100.
    pub fn line_to(&self, point: impl Into<Point>) -> Result<Self, Error> {
        let point = coordinate_point("point", point)?;
        self.with_segment("line_to", Segment::Line(point))
    }

    /// This path with a quadratic Bezier segment to `point`, bent towards `control`.
    ///
    /// # Errors
    /// As [`Path::line_to`], naming `control` or `point`.
    pub fn quad_to(&self, control: impl Into<Point>, point: impl Into<Point>) -> Result<Self, Error> {
        let control = coordinate_point("control", control)?;
        let point = coordinate_point("point", point)?;
        self.with_segment("quad_to", Segment::Quad(control, point))
    }

    /// This path with a cubic Bezier segment to `point`, leaving towards `control1` and arriving from `control2`.
    ///
    /// # Errors
    /// As [`Path::line_to`], naming `control1`, `control2` or `point`.
    pub fn curve_to(
        &self,
        control1: impl Into<Point>,
        control2: impl Into<Point>,
        point: impl Into<Point>,
    ) -> Result<Self, Error> {
        let control1 = coordinate_point("control1", control1)?;
        let control2 = coordinate_point("control2", control2)?;
        let point = coordinate_point("point", point)?;
        self.with_segment("curve_to", Segment::Cubic(control1, control2, point))
    }

    /// This path with its last sub-path closed. The line back to the start is implied, not a segment of its own: a
    /// last straight segments that already end at the start are dropped. Closing a closed sub-path changes nothing.
    ///
    /// # Errors
    /// [`Error::Parameter`] if there is no sub-path yet (call [`Path::move_to`] first).
    pub fn close(&self) -> Result<Self, Error> {
        let mut next = self.clone();
        let current = next.subpaths.pop().ok_or_else(|| no_start("close"))?;
        next.subpaths.push(current.closed());
        Ok(next)
    }

    /// Whether every sub-path is closed; an empty path has none that is open.
    pub(crate) fn is_closed(&self) -> bool {
        self.subpaths.iter().all(|subpath| subpath.closed)
    }

    fn with_segment(&self, verb: &str, segment: Segment) -> Result<Self, Error> {
        let mut next = self.clone();
        let mut current = next.subpaths.pop().ok_or_else(|| no_start(verb))?;
        if current.closed {
            let restart = SubPath::open(current.start);
            next.subpaths.push(current);
            current = restart;
        }
        current.segments.push(segment);
        next.subpaths.push(current);
        Ok(next)
    }
}

fn no_start(verb: &str) -> Error {
    Error::parameter(verb, "there is no sub-path to continue: call move_to first")
}
