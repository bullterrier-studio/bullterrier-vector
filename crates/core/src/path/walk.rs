//! The measured walk: a path flattened into straight edges and laid end to end, sub-path after sub-path, so that a
//! distance along the path is a place on an edge. Length, point, direction, nearest point and slice all read this one
//! walk, so they agree.

use std::cmp::Ordering;

use super::curve::{Curve, Flat, flatten, lerp_number};
use super::{Path, SubPath};
use crate::geometry::Point;

/// A flat edge laid on the walk: where it lies, which parameter range of which curve it stands for, and how far from
/// the start of its sub-path it begins and how long it is.
#[derive(Debug, Clone, Copy)]
pub(super) struct Edge {
    pub(super) from: Point,
    pub(super) to: Point,
    pub(super) segment: usize,
    pub(super) t0: f64,
    pub(super) t1: f64,
    pub(super) start: f64,
    pub(super) length: f64,
}

/// One sub-path on the walk.
#[derive(Debug)]
pub(super) struct Run<'a> {
    pub(super) subpath: &'a SubPath,
    /// The curves of the sub-path, closing line included, that the edges' `segment` indexes.
    pub(super) curves: Vec<Curve>,
    /// The edges in walking order. A closed sub-path's walk starts at its topmost point, not where it was written.
    pub(super) edges: Vec<Edge>,
    /// How far along the whole path this run begins.
    pub(super) offset: f64,
    pub(super) length: f64,
}

/// A place on the walk: an edge of a run and how far along it, 0 to 1.
#[derive(Debug, Clone, Copy)]
pub(super) struct Spot {
    pub(super) run: usize,
    pub(super) edge: usize,
    pub(super) along: f64,
}

#[derive(Debug)]
pub(super) struct Walk<'a> {
    pub(super) runs: Vec<Run<'a>>,
    pub(super) length: f64,
}

impl Path {
    /// The walk of this path. A closed sub-path starts at its topmost point, or with `near` at the point of it
    /// nearest `near`.
    pub(super) fn walk(&self, near: Option<Point>) -> Walk<'_> {
        let mut runs = Vec::with_capacity(self.subpaths.len());
        let mut offset = 0.0;
        for subpath in &self.subpaths {
            let run = Run::new(subpath, offset, near);
            offset += run.length;
            runs.push(run);
        }
        Walk { runs, length: offset }
    }
}

impl<'a> Run<'a> {
    fn new(subpath: &'a SubPath, offset: f64, near: Option<Point>) -> Self {
        let curves = subpath.curves();
        let mut flats = flatten(&curves);
        if subpath.closed {
            let place = near.and_then(|near| nearest(flats.iter().map(|flat| (flat.from, flat.to)), near));
            if let Some(place) = place {
                start_at(&mut flats, place);
            } else {
                let first = topmost(&flats);
                flats.rotate_left(first);
            }
        }
        let mut length = 0.0;
        let edges = flats
            .into_iter()
            .map(|flat| {
                let edge = lay(flat, length);
                length += edge.length;
                edge
            })
            .collect();
        Self {
            subpath,
            curves,
            edges,
            offset,
            length,
        }
    }

    /// The point of the run's curves `along` (0 to 1) the edge number `edge`.
    fn point(&self, edge: usize, along: f64) -> Point {
        let edge = &self.edges[edge];
        self.curves[edge.segment].at(lerp_number(edge.t0, edge.t1, along))
    }
}

/// `flat` laid on the walk `start` from the beginning of its run.
fn lay(flat: Flat, start: f64) -> Edge {
    let (dx, dy) = (flat.to.x - flat.from.x, flat.to.y - flat.from.y);
    Edge {
        from: flat.from,
        to: flat.to,
        segment: flat.segment,
        t0: flat.t0,
        t1: flat.t1,
        start,
        length: (dx * dx + dy * dy).sqrt(),
    }
}

/// The index of the edge that begins at the topmost point of the outline: the smallest `y`, and of those the smallest
/// `x`. The first of equal points wins.
fn topmost(flats: &[Flat]) -> usize {
    let mut best = 0;
    for (index, flat) in flats.iter().enumerate() {
        let (top, found) = (flats[best].from, flat.from);
        if (found.y, found.x)
            .partial_cmp(&(top.y, top.x))
            .is_some_and(Ordering::is_lt)
        {
            best = index;
        }
    }
    best
}

/// The place on the closed ring of `flats` that is the new start: the edge, and how far along it.
fn start_at(flats: &mut Vec<Flat>, place: Nearest) {
    let Nearest { index, along, .. } = place;
    if along <= 0.0 {
        flats.rotate_left(index);
    } else if along >= 1.0 {
        let next = (index + 1) % flats.len();
        flats.rotate_left(next);
    } else {
        let edge = flats[index];
        let middle = Point::new(
            lerp_number(edge.from.x, edge.to.x, along),
            lerp_number(edge.from.y, edge.to.y, along),
        );
        let t = lerp_number(edge.t0, edge.t1, along);
        flats[index] = Flat {
            to: middle,
            t1: t,
            ..edge
        };
        flats.insert(
            index + 1,
            Flat {
                from: middle,
                t0: t,
                ..edge
            },
        );
        flats.rotate_left(index + 1);
    }
}

/// The nearest place on a row of straight edges to a point: which edge, how far along it, how far away (squared).
#[derive(Debug, Clone, Copy)]
struct Nearest {
    index: usize,
    along: f64,
    squared_distance: f64,
}

/// The place on the straight edge from `a` to `b` nearest `point`: how far along (0 to 1) and the squared distance.
fn project(point: Point, a: Point, b: Point) -> (f64, f64) {
    let (along_x, along_y) = (b.x - a.x, b.y - a.y);
    let squared_length = along_x * along_x + along_y * along_y;
    let share = ((point.x - a.x) * along_x + (point.y - a.y) * along_y) / squared_length;
    // the ends are the edge's own points, so that edges meeting in a corner agree on the distance to it
    let nearest = if share <= 0.0 {
        a
    } else if share >= 1.0 {
        b
    } else {
        Point::new(a.x + along_x * share, a.y + along_y * share)
    };
    let (dx, dy) = (point.x - nearest.x, point.y - nearest.y);
    (share.clamp(0.0, 1.0), dx * dx + dy * dy)
}

/// The nearest place to `point` on the edges `from, to` in order; of equally near places, the earliest.
fn nearest(edges: impl Iterator<Item = (Point, Point)>, point: Point) -> Option<Nearest> {
    let mut best: Option<Nearest> = None;
    for (index, (a, b)) in edges.enumerate() {
        let (along, squared_distance) = project(point, a, b);
        if best.is_none_or(|found| squared_distance < found.squared_distance) {
            best = Some(Nearest {
                index,
                along,
                squared_distance,
            });
        }
    }
    best
}

impl Walk<'_> {
    /// The position along the walk, as a fraction of its length, of the point of the outline nearest `point`; of
    /// equally near places, the smallest position. 0 if the walk has no length.
    pub(super) fn locate(&self, point: Point) -> f64 {
        let mut best: Option<(f64, f64)> = None;
        for run in &self.runs {
            let Some(place) = nearest(run.edges.iter().map(|edge| (edge.from, edge.to)), point) else {
                continue;
            };
            if best.is_none_or(|(squared_distance, _)| place.squared_distance < squared_distance) {
                let edge = &run.edges[place.index];
                best = Some((
                    place.squared_distance,
                    run.offset + edge.start + place.along * edge.length,
                ));
            }
        }
        match best {
            Some((_, distance)) if self.length > 0.0 => distance / self.length,
            _ => 0.0,
        }
    }

    /// The place `distance` along the walk, or `None` if the walk has no length. A boundary belongs to what follows
    /// it; the end of the walk is the end of the last edge of an open sub-path, and the start of a closed one.
    pub(super) fn spot(&self, distance: f64) -> Option<Spot> {
        let mut last = None;
        for (index, run) in self.runs.iter().enumerate() {
            if run.length <= 0.0 {
                continue;
            }
            last = Some(index);
            if distance < run.offset + run.length {
                let local = distance - run.offset;
                let edge = run
                    .edges
                    .partition_point(|edge| edge.start + edge.length <= local)
                    .min(run.edges.len() - 1);
                let on = &run.edges[edge];
                // rounding can leave the share a hair over 1 on the last edge of a run
                let along = ((local - on.start) / on.length).min(1.0);
                return Some(Spot {
                    run: index,
                    edge,
                    along,
                });
            }
        }
        let run = last?;
        Some(if self.runs[run].subpath.closed {
            Spot {
                run,
                edge: 0,
                along: 0.0,
            }
        } else {
            Spot {
                run,
                edge: self.runs[run].edges.len() - 1,
                along: 1.0,
            }
        })
    }

    /// The point of the outline at `spot`.
    pub(super) fn point(&self, spot: Spot) -> Point {
        self.runs[spot.run].point(spot.edge, spot.along)
    }

    /// The unit direction of the edge at `spot`.
    pub(super) fn direction(&self, spot: Spot) -> Point {
        let edge = &self.runs[spot.run].edges[spot.edge];
        Point::new(
            (edge.to.x - edge.from.x) / edge.length,
            (edge.to.y - edge.from.y) / edge.length,
        )
    }
}
