//! Forking threads grown from the outline of a region: the kernel of the `branch` step.

use super::warp::WAVE_PIECES;
use super::{FLATNESS, Path};
use crate::Error;
use crate::expr::{splitmix, whole_seed};
use crate::geometry::Point;
use crate::limits::{MAX_SEGMENTS, MAX_SHAPES};
use crate::math::{atan2, sin_cos_deg};
use crate::noise::{self, Noise};

/// What to grow: the numbers of one shape's call, checked by the step.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BranchPlan {
    /// Design pixels between roots along an outline.
    pub(crate) spacing: f64,
    /// The length of every root-to-tip path of a root whose spread factor is 1.
    pub(crate) length: f64,
    /// Distance from the root per fork, at a spread factor of 1.
    pub(crate) fork_spacing: f64,
    /// The most forks one path takes.
    pub(crate) forks: u64,
    /// How much shorter than `length` a root's tree may be, as a share of it.
    pub(crate) spread: f64,
    /// Degrees a fork turns from its parent.
    pub(crate) angle: f64,
    /// Degrees of the heading's drift.
    pub(crate) wander: f64,
    /// Design pixels along a thread over which the heading drifts.
    pub(crate) wavelength: f64,
    /// The seed, a whole number.
    pub(crate) seed: f64,
}

/// One root-to-tip path.
#[derive(Debug)]
pub(crate) struct Thread {
    /// From the root to the tip.
    pub(crate) points: Vec<Point>,
    /// The root's number on the shape, from 0.
    pub(crate) root: u32,
    /// The forks the path takes.
    pub(crate) forks: u32,
}

/// A thread still to grow: where it starts and what it inherits.
struct Pending {
    /// The thread it forks from and the index of the parent's point it starts at; `None` for a trunk.
    parent: Option<(usize, usize)>,
    start: Point,
    /// Distance from the root at the start.
    from: f64,
    heading: f64,
    /// The first stretch it may fork in.
    stretch: u64,
    forks: u64,
}

/// A grown thread: its own points (the first is its parent's point it starts at) and where it hangs.
struct Grown {
    parent: Option<(usize, usize)>,
    points: Vec<Point>,
    forks: u64,
}

/// A number from 0 up to 1, the same for the same seed and keys on every target: `random`'s generator.
#[expect(clippy::cast_precision_loss, reason = "the 53 bits kept fit a float exactly")]
fn uniform(seed: i64, first: u64, second: u64) -> f64 {
    // constant: 2^-53, so that 53 bits make a float exactly
    const SCALE: f64 = 1.0 / 9_007_199_254_740_992.0;
    (splitmix(splitmix(seed, first).cast_signed(), second) >> 11) as f64 * SCALE
}

/// The sum of `C(m, k)` for `k` from 0 to `most`, stopping once it passes `cap`.
fn tips(m: f64, most: f64, cap: f64) -> f64 {
    let (mut sum, mut term, mut k) = (1.0, 1.0, 0.0);
    while k < most.min(m) && sum <= cap {
        term = term * (m - k) / (k + 1.0);
        sum += term;
        k += 1.0;
    }
    sum
}

/// The closed polygons of a resolved path, each as its vertices.
fn polygons(resolved: &Path) -> Vec<Vec<Point>> {
    resolved
        .subpaths
        .iter()
        .map(|subpath| {
            let mut vertices = vec![subpath.start];
            vertices.extend(subpath.segments.iter().map(|segment| segment.end()));
            vertices
        })
        .collect()
}

fn perimeter(polygon: &[Point]) -> f64 {
    (0..polygon.len())
        .map(|i| distance(polygon[i], polygon[(i + 1) % polygon.len()]))
        .sum()
}

fn distance(a: Point, b: Point) -> f64 {
    ((b.x - a.x) * (b.x - a.x) + (b.y - a.y) * (b.y - a.y)).sqrt()
}

/// The point `along` pixels round `polygon` from its first vertex, and the outward heading there in degrees: the left
/// of the way the edge runs, which is outside the region for every contour of a resolved path.
fn on_outline(polygon: &[Point], along: f64) -> (Point, f64) {
    let mut left = along;
    let count = polygon.len();
    for i in 0..count {
        let (a, b) = (polygon[i], polygon[(i + 1) % count]);
        let edge = distance(a, b);
        if edge > 0.0 && (left < edge || i + 1 == count) {
            let share = (left / edge).min(1.0);
            let point = Point::new(a.x + (b.x - a.x) * share, a.y + (b.y - a.y) * share);
            // the left of (dx, dy) on screen, y down, is (dy, -dx)
            return (point, atan2(a.x - b.x, b.y - a.y).to_degrees());
        }
        left -= edge;
    }
    (polygon[0], 0.0)
}

impl Path {
    /// The threads `plan` grows from the outline of the region this path fills, for the shape `stream` (its index,
    /// which keeps equal shapes apart), charging `charge` with the resolve's work and then with every point before any
    /// thread is made.
    pub(crate) fn branch_charged(
        &self,
        plan: &BranchPlan,
        stream: u64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Vec<Thread>, Error> {
        let wavelength = noise::wavelength(plan.wavelength)?;
        let noise = Noise::new(plan.seed, stream)?;
        let seed = splitmix(whole_seed(plan.seed)?, stream).cast_signed();
        let resolved = self.resolved(FLATNESS, charge)?;
        let outlines: Vec<(Vec<Point>, f64)> = polygons(&resolved)
            .into_iter()
            .map(|polygon| {
                let around = perimeter(&polygon);
                (polygon, around)
            })
            .filter(|(_, around)| *around > 0.0)
            .collect();
        let step = wavelength / WAVE_PIECES;
        let stretches = (plan.length / plan.fork_spacing).floor();
        // the bound, before anything is grown: every path has a point per step from its root to its tip
        let per_path = (plan.length / step).ceil() + 1.0;
        let (limit_points, limit_shapes) = (as_f64(MAX_SEGMENTS), as_f64(MAX_SHAPES));
        if per_path > limit_points {
            return Err(too_many("wavelength", "a path would have", per_path));
        }
        let per_root = tips(stretches, as_f64_u64(plan.forks), limit_shapes);
        if per_root > limit_shapes || per_root * per_path > limit_points {
            return Err(too_many(
                "fork_spacing",
                "one root's paths would have",
                per_root * per_path,
            ));
        }
        let roots: f64 = outlines
            .iter()
            .map(|(_, around)| (around / plan.spacing).floor().max(1.0))
            .sum();
        if roots * per_root > limit_shapes || roots * per_root * per_path > limit_points {
            return Err(too_many(
                "spacing",
                "the roots' paths would have",
                roots * per_root * per_path,
            ));
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a count checked against the limits above"
        )]
        charge((roots * per_root * per_path) as usize)?;
        let mut threads = Vec::new();
        let mut root = 0_u32;
        for (polygon, around) in &outlines {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a count checked against the limits above"
            )]
            let count = (around / plan.spacing).floor().max(1.0) as u64;
            let stretch = around / as_f64_u64(count);
            for j in 0..count {
                let key = 2 * u64::from(root) + 1;
                let along = (as_f64_u64(j) + uniform(seed, key, 0)) * stretch;
                let (start, heading) = on_outline(polygon, along);
                let factor = 1.0 - plan.spread * uniform(seed, key, 1);
                let tree = Tree {
                    plan,
                    noise: &noise,
                    seed,
                    step,
                    length: plan.length * factor,
                    fork_spacing: plan.fork_spacing * factor,
                    stretches,
                };
                tree.grow(start, heading, root, &mut threads);
                root += 1;
            }
        }
        Ok(threads)
    }
}

/// One root's tree: what its threads share.
struct Tree<'a> {
    plan: &'a BranchPlan,
    noise: &'a Noise,
    seed: i64,
    step: f64,
    /// This root's length: `length` times its spread factor.
    length: f64,
    fork_spacing: f64,
    stretches: f64,
}

impl Tree<'_> {
    /// Grows the tree of the root at `start`, heading `heading`, adding one thread per tip to `into`.
    fn grow(&self, start: Point, heading: f64, root: u32, into: &mut Vec<Thread>) {
        let first_id = into.len();
        let mut grown: Vec<Grown> = Vec::new();
        let mut pending = vec![Pending {
            parent: None,
            start,
            from: 0.0,
            heading,
            stretch: 0,
            forks: 0,
        }];
        // a work list, not recursion: each thread grows its points, then queues a fork in each stretch it may fork in
        while let Some(thread) = pending.pop() {
            let id = as_u64(first_id + grown.len());
            let at = |s: f64| self.noise.value(s / self.plan.wavelength, as_f64_u64(id), 0.0);
            let (mut point, mut from) = (thread.start, thread.from);
            let (base, start_noise) = (thread.heading, at(thread.from));
            let mut points = vec![point];
            let mut headings = Vec::new();
            while from < self.length {
                let heading = base + self.plan.wander * (at(from) - start_noise);
                let run = self.step.min(self.length - from);
                let (sin, cos) = sin_cos_deg(heading);
                point = Point::new(point.x + run * cos, point.y + run * sin);
                points.push(point);
                headings.push(heading);
                from += run;
            }
            let index = grown.len();
            if thread.forks < self.plan.forks {
                let mut stretch = thread.stretch;
                while as_f64_u64(stretch) < self.stretches {
                    let place = (as_f64_u64(stretch) + uniform(self.seed, 2 * id, 2 * stretch)) * self.fork_spacing;
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "a point of this thread, from 0"
                    )]
                    let vertex = (((place - thread.from) / self.step).floor().max(0.0) as usize)
                        .min(headings.len().saturating_sub(1));
                    let side = if uniform(self.seed, 2 * id, 2 * stretch + 1) < 0.5 {
                        -1.0
                    } else {
                        1.0
                    };
                    pending.push(Pending {
                        parent: Some((index, vertex)),
                        start: points[vertex],
                        from: thread.from + as_f64_usize(vertex) * self.step,
                        heading: headings.get(vertex).copied().unwrap_or(base) + side * self.plan.angle,
                        stretch: stretch + 1,
                        forks: thread.forks + 1,
                    });
                    stretch += 1;
                }
            }
            grown.push(Grown {
                parent: thread.parent,
                points,
                forks: thread.forks,
            });
        }
        for tip in 0..grown.len() {
            // the path from the root: each ancestor's points up to where the next one leaves it
            let mut chain = vec![(tip, grown[tip].points.len())];
            while let Some((parent, vertex)) = grown[chain[chain.len() - 1].0].parent {
                chain.push((parent, vertex + 1));
            }
            let mut points = Vec::new();
            for (k, &(thread, upto)) in chain.iter().rev().enumerate() {
                let own = &grown[thread].points[..upto];
                // a fork's first point is the parent's last one kept
                points.extend_from_slice(if k == 0 { own } else { &own[1..] });
            }
            into.push(Thread {
                points,
                root,
                forks: u32::try_from(grown[tip].forks).unwrap_or(u32::MAX),
            });
        }
    }
}

fn too_many(parameter: &str, what: &str, points: f64) -> Error {
    Error::parameter(
        parameter,
        format!("{what} {points:.0} points, past MAX_SEGMENTS {MAX_SEGMENTS} or MAX_SHAPES {MAX_SHAPES} paths"),
    )
}

#[expect(clippy::cast_precision_loss, reason = "a limit far below 2^53")]
fn as_f64(count: usize) -> f64 {
    count as f64
}

#[expect(clippy::cast_precision_loss, reason = "a count below the limits, far below 2^53")]
fn as_f64_u64(count: u64) -> f64 {
    count as f64
}

#[expect(clippy::cast_precision_loss, reason = "an index below the limits, far below 2^53")]
fn as_f64_usize(count: usize) -> f64 {
    count as f64
}

fn as_u64(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}
