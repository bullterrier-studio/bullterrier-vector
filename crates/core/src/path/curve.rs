//! One segment together with the point it starts from: the Bezier maths measuring is built on, and the flattening
//! that turns a curve into straight edges.

use super::{Segment, SubPath};
use crate::geometry::Point;

/// How far, in design pixels, a flattened curve may stray from the curve it stands for.
// constant: a tolerance below any visible effect (a hundredth of a pixel) and independent of the output size
const FLATNESS: f64 = 0.01;
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

    pub(super) fn end(self) -> Point {
        match self {
            Self::Line(_, end) | Self::Quad(_, _, end) | Self::Cubic(_, _, _, end) => end,
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

    /// Whether the curve stays within [`FLATNESS`] of the straight edge between its ends: its control points do
    /// (and a curve lies within the hull of its control points).
    fn is_flat(self) -> bool {
        let (a, b, controls): (Point, Point, &[Point]) = match &self {
            Self::Line(..) => return true,
            Self::Quad(a, c, b) => (*a, *b, &[*c]),
            Self::Cubic(a, c1, c2, b) => (*a, *b, &[*c1, *c2]),
        };
        controls
            .iter()
            .all(|&control| squared_distance_to_edge(control, a, b) <= FLATNESS * FLATNESS)
    }

    /// Appends the straight edges that stand for this curve, as segment number `segment` over the parameter range
    /// `t0..t1`, halving it until each edge is flat. Edges of no length are left out.
    pub(super) fn flatten(self, segment: usize, (t0, t1): (f64, f64), depth: u32, edges: &mut Vec<Flat>) {
        if depth >= MAX_FLATTEN_DEPTH || self.is_flat() {
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
            first.flatten(segment, (t0, middle), depth + 1, edges);
            second.flatten(segment, (middle, t1), depth + 1, edges);
        }
    }

    fn start(self) -> Point {
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

/// The squared distance from `point` to the nearest point of the straight edge from `a` to `b`.
fn squared_distance_to_edge(point: Point, a: Point, b: Point) -> f64 {
    let (along_x, along_y) = (b.x - a.x, b.y - a.y);
    let squared_length = along_x * along_x + along_y * along_y;
    let share = if squared_length == 0.0 {
        0.0
    } else {
        (((point.x - a.x) * along_x + (point.y - a.y) * along_y) / squared_length).clamp(0.0, 1.0)
    };
    let (dx, dy) = (point.x - (a.x + along_x * share), point.y - (a.y + along_y * share));
    dx * dx + dy * dy
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

/// The straight edges that stand for `curves`, in order, each tagged with its curve's index.
pub(super) fn flatten(curves: &[Curve]) -> Vec<Flat> {
    let mut edges = Vec::new();
    for (segment, curve) in curves.iter().enumerate() {
        curve.flatten(segment, (0.0, 1.0), 0, &mut edges);
    }
    edges
}
