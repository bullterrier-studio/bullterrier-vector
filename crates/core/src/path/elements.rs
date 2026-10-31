//! A path as a flat list of drawing commands, for the output that hands it to the rasteriser.

use super::{Path, Segment};
use crate::geometry::Point;

/// One drawing command of a path, in the order the path is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Element {
    /// Starts a sub-path.
    Move(Point),
    Line(Point),
    /// A quadratic curve: the control point, then the end.
    Quad(Point, Point),
    /// A cubic curve: the two control points, then the end.
    Cubic(Point, Point, Point),
    /// Ends a sub-path with the line back to its start.
    Close,
}

impl Element {
    /// The largest absolute coordinate among the points of this command.
    pub(crate) fn farthest(self) -> f64 {
        let reach = |point: Point| point.x.abs().max(point.y.abs());
        match self {
            Self::Move(to) | Self::Line(to) => reach(to),
            Self::Quad(control, to) => reach(control).max(reach(to)),
            Self::Cubic(first, second, to) => reach(first).max(reach(second)).max(reach(to)),
            Self::Close => 0.0,
        }
    }
}

impl Path {
    /// The drawing commands of this path: each sub-path is a `Move`, its segments and a `Close` if it is closed.
    pub(crate) fn elements(&self) -> Vec<Element> {
        let mut elements = Vec::new();
        for subpath in &self.subpaths {
            elements.push(Element::Move(subpath.start));
            elements.extend(subpath.segments.iter().map(|segment| match *segment {
                Segment::Line(end) => Element::Line(end),
                Segment::Quad(control, end) => Element::Quad(control, end),
                Segment::Cubic(first, second, end) => Element::Cubic(first, second, end),
            }));
            if subpath.closed {
                elements.push(Element::Close);
            }
        }
        elements
    }
}
