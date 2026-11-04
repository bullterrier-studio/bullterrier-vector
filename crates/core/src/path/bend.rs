//! Bending: carrying a path along a guide. The guide is laid end to end like a measuring tape ([`Walk`]); a point
//! `(x, y)` goes to the place `x` along it (after `start` lengths), `y` to the right of the way it runs.

use super::curve::{Curve, lerp_number};
use super::walk::Walk;
use super::{Path, Polyline, Segment, SubPath};
use crate::Error;
use crate::geometry::{Point, finite};

/// How far, in design pixels, a bent edge may stray from the bend it stands for.
// constant: a hundredth of a pixel, the tolerance every flattening in this crate keeps (`FLATNESS`)
const BEND_FLATNESS: f64 = 0.01;
/// How often an edge is halved at most, so one edge gives at most 2^8 pieces.
// constant: a cap so bending ends where the map folds or jumps (a tight corner of the guide, a point at the guide's
// centre of curvature); 256 pieces per edge is far finer than the flatness needs on any guide a drawing has
const MAX_BEND_DEPTH: u32 = 8;

impl Path {
    /// This path carried along `guide`: a point `(x, y)` goes to the place `start * length + x` along the guide
    /// (`length` is [`Path::length`] of the guide), then `y` to the right of the way the guide runs there. On a
    /// guide that runs to the right that is down the screen (`y` points down), so a text laid along it reads along
    /// the guide with its tops on the left of the way.
    ///
    /// The guide's sub-paths are walked in the order they were built, as one length. A guide whose sub-paths are all
    /// closed wraps: a place past the end is the same place a whole length back. Any other guide goes on straight
    /// past its ends, in the direction it ends or begins in, so every `start` and `x` has a place. Where two
    /// segments of the guide meet in a corner the direction is the one leaving it. With `near`, a closed sub-path
    /// of the guide is walked from the point of it nearest `near`, as for [`Path::point_at`].
    ///
    /// The path is flattened to straight edges, as for drawing, and each edge is halved, up to eight times, until
    /// the bent middle and quarter points are within a hundredth of a pixel of those of the bent ends. The result is made of
    /// straight lines; each sub-path keeps its closedness. An edge longer than half a closed guide can be bent
    /// too coarsely (its ends, middle and quarter points can agree by chance).
    ///
    /// # Errors
    /// [`Error::Parameter`] if `start` is NaN or infinite, `near` is NaN, infinite or beyond ±1e100, or a bent point
    /// would be beyond ±1e100 (naming `start`); [`Error::Geometry`] if the guide is empty or has no length, and so
    /// no direction.
    pub fn along(&self, guide: &Self, start: f64, near: Option<Point>) -> Result<Self, Error> {
        self.bent(guide, start, near, &mut |_| Ok(()))
    }

    /// As [`Path::along`], calling `charge` with the number of edges before it makes them, as for
    /// [`Path::flattened`]; its error stops the bending.
    pub(crate) fn bent(
        &self,
        guide: &Self,
        start: f64,
        near: Option<Point>,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let start = finite("start", start)?;
        let guide = Guide::of(&guide.nonempty_walk("along", near)?)?;
        let offset = start * guide.length;
        let mut subpaths = Vec::with_capacity(self.subpaths.len());
        for polyline in self.flattened(charge)? {
            subpaths.push(guide.bend(&polyline, offset, charge)?);
        }
        Self { subpaths }.within_limit("start")
    }
}

/// A straight edge of the guide laid on its length: the curve it stands for, the parameters that edge spans of it,
/// and the unit direction of the edge itself.
struct GuideEdge {
    /// How far along the guide the edge begins, and how long it is.
    begin: f64,
    length: f64,
    curve: Curve,
    t0: f64,
    t1: f64,
    chord: Point,
}

/// The edges of a guide that have a length, end to end, and what to do past its ends.
struct Guide {
    edges: Vec<GuideEdge>,
    length: f64,
    /// Whether every sub-path with a length is closed: then the guide wraps instead of going on straight.
    closed: bool,
}

impl Guide {
    fn of(walk: &Walk<'_>) -> Result<Self, Error> {
        let mut edges = Vec::new();
        let mut closed = true;
        for run in &walk.runs {
            let before = edges.len();
            for edge in run.edges.iter().filter(|edge| edge.length > 0.0) {
                edges.push(GuideEdge {
                    begin: run.offset + edge.start,
                    length: edge.length,
                    curve: run.curves[edge.segment],
                    t0: edge.t0,
                    t1: edge.t1,
                    chord: Point::new(
                        (edge.to.x - edge.from.x) / edge.length,
                        (edge.to.y - edge.from.y) / edge.length,
                    ),
                });
            }
            closed &= edges.len() == before || run.subpath.closed;
        }
        if edges.is_empty() {
            return Err(Error::geometry("along", "the guide has no length, so no direction"));
        }
        Ok(Self {
            edges,
            length: walk.length,
            closed,
        })
    }

    /// The place `along` (0 to 1) of the edge number `index`, and the unit direction of the guide there: the
    /// curve's own, or the edge's where the curve stands still.
    fn at(&self, index: usize, along: f64) -> (Point, Point) {
        let edge = &self.edges[index];
        let t = lerp_number(edge.t0, edge.t1, along);
        let velocity = edge.curve.velocity(t);
        let speed = (velocity.x * velocity.x + velocity.y * velocity.y).sqrt();
        let direction = if speed > 0.0 {
            Point::new(velocity.x / speed, velocity.y / speed)
        } else {
            edge.chord
        };
        (edge.curve.at(t), direction)
    }

    /// The place `distance` along the guide and the unit direction there, wrapping or going on straight past the ends.
    fn frame(&self, distance: f64) -> (Point, Point) {
        let last = self.edges.len() - 1;
        let distance = if self.closed {
            distance.rem_euclid(self.length)
        } else {
            distance
        };
        if distance < 0.0 {
            let (point, direction) = self.at(0, 0.0);
            return (
                Point::new(point.x + distance * direction.x, point.y + distance * direction.y),
                direction,
            );
        }
        if distance > self.length {
            let (point, direction) = self.at(last, 1.0);
            let beyond = distance - self.length;
            return (
                Point::new(point.x + beyond * direction.x, point.y + beyond * direction.y),
                direction,
            );
        }
        // a boundary between two edges falls on the later one; rounding can leave the share a hair out of 0 to 1
        let index = self
            .edges
            .partition_point(|edge| edge.begin + edge.length <= distance)
            .min(last);
        let edge = &self.edges[index];
        self.at(index, ((distance - edge.begin) / edge.length).clamp(0.0, 1.0))
    }

    /// Where `point` goes when the guide is walked `offset` before its start.
    fn image(&self, offset: f64, point: Point) -> Point {
        let (place, direction) = self.frame(offset + point.x);
        // the right of the way on, with y down, is (-dy, dx)
        Point::new(place.x - point.y * direction.y, place.y + point.y * direction.x)
    }

    /// The bent sub-path of `polyline`, its edges halved where the bend is not straight, charging the edges it
    /// makes before it makes them.
    fn bend(
        &self,
        polyline: &Polyline,
        offset: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<SubPath, Error> {
        let corners = &polyline.points;
        let map = |point: Point| self.image(offset, point);
        let mut bent = vec![map(corners[0])];
        let edges = if polyline.closed {
            corners.len()
        } else {
            corners.len() - 1
        };
        let mut pieces = Vec::new();
        for index in 0..edges {
            let from = (corners[index], bent[bent.len() - 1]);
            // the edge that closes a ring ends where the ring began
            let closing = polyline.closed && index + 1 == edges;
            let to = if closing {
                (corners[0], bent[0])
            } else {
                let end = corners[index + 1];
                (end, map(end))
            };
            pieces.clear();
            halve(&map, from, to, MAX_BEND_DEPTH, &mut pieces);
            charge(pieces.len() + usize::from(!closing))?;
            bent.extend_from_slice(&pieces);
            if !closing {
                bent.push(to.1);
            }
        }
        let mut segments: Vec<Segment> = bent[1..].iter().map(|&point| Segment::Line(point)).collect();
        if polyline.drawn && segments.is_empty() {
            // a line to its own start still marks a place
            segments.push(Segment::Line(bent[0]));
        }
        let subpath = SubPath {
            start: bent[0],
            segments,
            closed: false,
        };
        Ok(if polyline.closed { subpath.closed() } else { subpath })
    }
}

/// Appends the bent points between the ends of an edge, in order: each end is a point and where it goes. The edge is
/// halved while the bend of its middle, or of the point a quarter or three quarters along it, is more than
/// [`BEND_FLATNESS`] from the same place on the chord of the bent ends, `depth` times at most. The middle alone would
/// take for straight a bend that is point-symmetric about it (an S).
fn halve(map: &impl Fn(Point) -> Point, from: (Point, Point), to: (Point, Point), depth: u32, out: &mut Vec<Point>) {
    if depth == 0 {
        return;
    }
    let middle = Point::new(f64::midpoint(from.0.x, to.0.x), f64::midpoint(from.0.y, to.0.y));
    let bent = map(middle);
    // how far the bend of the point `share` of the way along the edge is from `share` of the way along the bent chord
    let off = |bent: Point, share: f64| {
        let (dx, dy) = (
            bent.x - lerp_number(from.1.x, to.1.x, share),
            bent.y - lerp_number(from.1.y, to.1.y, share),
        );
        (dx * dx + dy * dy).sqrt()
    };
    let straight = off(bent, 0.5) <= BEND_FLATNESS
        && [0.25, 0.75].into_iter().all(|share| {
            let at = Point::new(
                lerp_number(from.0.x, to.0.x, share),
                lerp_number(from.0.y, to.0.y, share),
            );
            off(map(at), share) <= BEND_FLATNESS
        });
    if straight {
        return;
    }
    halve(map, from, (middle, bent), depth - 1, out);
    out.push(bent);
    halve(map, (middle, bent), to, depth - 1, out);
}
