//! One segment together with the point it starts from: the Bezier maths measuring is built on, and the flattening
//! that turns a curve into straight edges.

use super::{Segment, SubPath};
use crate::geometry::Point;

/// How far, in design pixels, a flattened curve may stray from the curve it stands for.
// constant: a tolerance below any visible effect (a hundredth of a pixel) and independent of the output size
pub(crate) const FLATNESS: f64 = 0.01;
/// How often a curve is halved at most while it is flattened, so one segment gives at most 2^16 edges.
// constant: a cap so flattening ends on huge or degenerate curves; a curve that needs more is larger than any drawing
const MAX_FLATTEN_DEPTH: u32 = 16;

/// A segment with its start point: the Bezier of two, three or four points.
#[derive(Debug, Clone, Copy)]
pub(super) enum Curve {
    Line(Point, Point),
    Quad(Point, Point, Point),
    Cubic(Point, Point, Point, Point),
}

/// A straight edge of a flattened curve, and the parameter range `t0..t1` of segment number `segment` that it stands
/// for.
#[derive(Debug, Clone, Copy)]
pub(super) struct Flat {
    pub(super) from: Point,
    pub(super) to: Point,
    pub(super) segment: usize,
    pub(super) t0: f64,
    pub(super) t1: f64,
}

impl Curve {
    /// The point a fraction `t` of the way along the curve's parameter.
    pub(super) fn at(self, t: f64) -> Point {
        self.split(t).0.end()
    }

    /// The direction of travel at parameter `t`: the curve's derivative, not scaled to a length. It is `(0, 0)` where
    /// the curve stops for a moment, such as the start of a cubic whose first control point is its start.
    pub(super) fn velocity(self, t: f64) -> Point {
        let (u, w) = (1.0 - t, t);
        match self {
            Self::Line(start, end) => Point::new(end.x - start.x, end.y - start.y),
            Self::Quad(start, control, end) => Point::new(
                2.0 * (u * (control.x - start.x) + w * (end.x - control.x)),
                2.0 * (u * (control.y - start.y) + w * (end.y - control.y)),
            ),
            Self::Cubic(start, control1, control2, end) => Point::new(
                3.0 * (u * u * (control1.x - start.x)
                    + 2.0 * u * w * (control2.x - control1.x)
                    + w * w * (end.x - control2.x)),
                3.0 * (u * u * (control1.y - start.y)
                    + 2.0 * u * w * (control2.y - control1.y)
                    + w * w * (end.y - control2.y)),
            ),
        }
    }

    pub(super) fn end(self) -> Point {
        match self {
            Self::Line(_, end) | Self::Quad(_, _, end) | Self::Cubic(_, _, _, end) => end,
        }
    }

    /// The same curve travelled the other way.
    pub(super) fn reversed(self) -> Self {
        match self {
            Self::Line(start, end) => Self::Line(end, start),
            Self::Quad(start, control, end) => Self::Quad(end, control, start),
            Self::Cubic(start, control1, control2, end) => Self::Cubic(end, control2, control1, start),
        }
    }

    /// The part of the curve between parameters `t0` and `t1` (0 to 1, `t0` below `t1`). A part that reaches the
    /// curve's own start or end keeps that point exactly.
    pub(super) fn between(self, t0: f64, t1: f64) -> Self {
        let head = if t1 >= 1.0 { self } else { self.split(t1).0 };
        if t0 <= 0.0 { head } else { head.split(t0 / t1).1 }
    }

    /// The segment that, after the point this curve starts from, draws it.
    pub(super) fn segment(self) -> Segment {
        match self {
            Self::Line(_, end) => Segment::Line(end),
            Self::Quad(_, control, end) => Segment::Quad(control, end),
            Self::Cubic(_, control1, control2, end) => Segment::Cubic(control1, control2, end),
        }
    }

    /// The curve cut at parameter `t`: the part before it and the part after it, by de Casteljau's blending. The two
    /// parts keep the exact start and end of this curve.
    fn split(self, t: f64) -> (Self, Self) {
        match self {
            Self::Line(start, end) => {
                let cut = lerp(start, end, t);
                (Self::Line(start, cut), Self::Line(cut, end))
            }
            Self::Quad(start, control, end) => {
                let (before, after) = (lerp(start, control, t), lerp(control, end, t));
                let cut = lerp(before, after, t);
                (Self::Quad(start, before, cut), Self::Quad(cut, after, end))
            }
            Self::Cubic(start, control1, control2, end) => {
                let (first, middle, last) = (
                    lerp(start, control1, t),
                    lerp(control1, control2, t),
                    lerp(control2, end, t),
                );
                let (before, after) = (lerp(first, middle, t), lerp(middle, last, t));
                let cut = lerp(before, after, t);
                (
                    Self::Cubic(start, first, before, cut),
                    Self::Cubic(cut, after, last, end),
                )
            }
        }
    }

    /// Whether the curve is, to within `flatness`, the straight edge between its ends walked at an even pace: its
    /// control points lie within `flatness` of the evenly spaced points of the edge. Then the curve at every
    /// parameter `t` is within `flatness` of the edge at fraction `t`, so a position along the edge is a position
    /// along the curve.
    fn is_flat(self, flatness: f64) -> bool {
        let (even, controls): (Vec<Point>, Vec<Point>) = match self {
            Self::Line(..) => return true,
            Self::Quad(start, control, end) => (vec![lerp(start, end, 0.5)], vec![control]),
            Self::Cubic(start, control1, control2, end) => (
                vec![lerp(start, end, 1.0 / 3.0), lerp(start, end, 2.0 / 3.0)],
                vec![control1, control2],
            ),
        };
        even.iter().zip(&controls).all(|(even, control)| {
            let (dx, dy) = (control.x - even.x, control.y - even.y);
            dx * dx + dy * dy <= flatness * flatness
        })
    }

    /// Appends the straight edges that stand for this curve, as segment number `segment` over the parameter range
    /// `t0..t1`, halving it until each edge is flat to within `flatness`. Edges of no length are left out.
    pub(super) fn flatten(
        self,
        segment: usize,
        (t0, t1): (f64, f64),
        depth: u32,
        flatness: f64,
        edges: &mut Vec<Flat>,
    ) {
        if depth >= MAX_FLATTEN_DEPTH || self.is_flat(flatness) {
            let (from, to) = (self.start(), self.end());
            if from != to {
                edges.push(Flat {
                    from,
                    to,
                    segment,
                    t0,
                    t1,
                });
            }
        } else {
            let (first, second) = self.split(0.5);
            let middle = lerp_number(t0, t1, 0.5);
            first.flatten(segment, (t0, middle), depth + 1, flatness, edges);
            second.flatten(segment, (middle, t1), depth + 1, flatness, edges);
        }
    }

    pub(super) fn start(self) -> Point {
        match self {
            Self::Line(start, _) | Self::Quad(start, _, _) | Self::Cubic(start, _, _, _) => start,
        }
    }
}

/// `a` at `t = 0` and `b` at `t = 1`, exactly.
pub(super) fn lerp_number(a: f64, b: f64, t: f64) -> f64 {
    a * (1.0 - t) + b * t
}

fn lerp(a: Point, b: Point, t: f64) -> Point {
    Point::new(lerp_number(a.x, b.x, t), lerp_number(a.y, b.y, t))
}

impl Segment {
    /// This segment as a curve starting at `from`.
    fn curve(self, from: Point) -> Curve {
        match self {
            Self::Line(end) => Curve::Line(from, end),
            Self::Quad(control, end) => Curve::Quad(from, control, end),
            Self::Cubic(control1, control2, end) => Curve::Cubic(from, control1, control2, end),
        }
    }
}

impl SubPath {
    /// The curves of the segments in order and, for a closed sub-path, the line back to the start as the last one
    /// (a curve of no length when the last segment already ends there).
    pub(super) fn curves(&self) -> Vec<Curve> {
        let mut from = self.start;
        let mut curves = Vec::with_capacity(self.segments.len() + 1);
        for segment in &self.segments {
            curves.push(segment.curve(from));
            from = segment.end();
        }
        if self.closed {
            curves.push(Curve::Line(from, self.start));
        }
        curves
    }
}

/// The straight edges that stand for `curves`, within [`FLATNESS`], in order, each tagged with its curve's index.
pub(super) fn flatten(curves: &[Curve]) -> Vec<Flat> {
    let mut edges = Vec::new();
    for (segment, curve) in curves.iter().enumerate() {
        curve.flatten(segment, (0.0, 1.0), 0, FLATNESS, &mut edges);
    }
    edges
}
