//! Simplifying: a path replaced by few smooth curves that stay within a tolerance of it. Each sub-path is
//! flattened (as drawing flattens it), split at its sharp corners, and every run between corners is fitted with cubic
//! Beziers by Schneider's method ("An Algorithm for Automatically Fitting Digitized Curves", Graphics Gems, 1990):
//! chord-length parameters, a least-squares fit with the end tangents fixed, Newton reparametrisation, and a split
//! at the worst point where the fit is not yet close enough.

use std::f64::consts::FRAC_1_SQRT_2;

use super::{Path, Segment, SubPath};
use crate::Error;
use crate::geometry::{Point, coordinate};
use crate::limits::MAX_SEGMENTS;

/// The cosine of the turn from one edge to the next that makes a vertex a corner: a turn sharper than 45 degrees.
// constant: 45 degrees is where a bend stops reading as a curve and starts reading as a corner; the cosine, so no trig
const CORNER_COS: f64 = FRAC_1_SQRT_2;
/// How many times a fit is reparametrised by Newton's method before the run is split instead.
// constant: Schneider's own limit; each round roughly halves the error, so four is enough to rescue a near fit
const NEWTON_ROUNDS: usize = 4;
/// The most interval checks between points, per point of a run, before a fit is given up for a split.
// constant: a fit that is only just inside the tolerance needs checks in proportion to its length over its slack; 256
// per point keeps one fit linear and a fit that close to the limit is better split
const COVER_CHECKS: usize = 256;
/// A fit within this many tolerances is worth reparametrising; a worse one is split at once.
// constant: Schneider's heuristic (his `iterationError`): beyond it Newton's method rarely rescues a fit
const RESCUE: f64 = 4.0;
/// How far, as a share of the chord, a control point may be from its end before the fit is taken as degenerate.
// constant: below a millionth of the chord a tangent length is rounding, not a curve (Schneider's epsilon)
const DEGENERATE: f64 = 1e-6;

impl Path {
    /// The path as few smooth curves as the fit finds within `tolerance` design pixels of it. Each sub-path is flattened to
    /// within 0.01 pixel, as drawing does, and cut where the direction turns by more than 45 degrees (a corner,
    /// kept exactly as a point). Each run between corners is fitted by cubic Beziers, joined smoothly, or by one
    /// straight segment where all its points lie within `tolerance` of the chord: every input point is then within
    /// `tolerance` of the result, and every point of the result within `tolerance` of the input (a cubic is accepted
    /// only when a bound on its speed proves it, so a fit that cannot be proven is split, down to the input's own
    /// segments). The ends of an open sub-path and the start of a closed one are
    /// exact; a closed sub-path stays closed, and one with no corner is fitted as a smooth loop from its start.
    ///
    /// A sub-path never gets more segments than it had (it is kept as it was when the fit would have more), and a
    /// sub-path with no length stays a lone point; an empty path stays empty. Work is `O(n log n)` in the flattened
    /// edges, which are counted against `MAX_SEGMENTS` as each curve is flattened.
    ///
    /// A turn is measured between neighbouring edges, so a trace whose jitter turns sharply at every point keeps
    /// that jitter as corners: simplify after resampling, or after a smoothing step, for such input.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `tolerance` if it is not a number above 0 and at most 1e100, if the flattened
    /// path has more than `MAX_SEGMENTS` (10,000,000) edges, or if a fitted point would be beyond ±1e100.
    pub fn simplify(&self, tolerance: f64) -> Result<Self, Error> {
        let mut total = 0_usize;
        self.simplify_charged(tolerance, &mut |count| {
            total = total.saturating_add(count);
            if total > MAX_SEGMENTS {
                Err(Error::parameter(
                    "tolerance",
                    format!("MAX_SEGMENTS {MAX_SEGMENTS} passed"),
                ))
            } else {
                Ok(())
            }
        })
    }

    /// As [`Path::simplify`], calling `charge` with the edges of each curve as it is flattened; its error stops
    /// the verb.
    pub(crate) fn simplify_charged(
        &self,
        tolerance: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        if tolerance <= 0.0 {
            return Err(Error::parameter(
                "tolerance",
                format!("must be a number above 0, got {tolerance}"),
            ));
        }
        let tolerance = coordinate("tolerance", tolerance)?;
        let mut subpaths = Vec::with_capacity(self.subpaths.len());
        for (polyline, original) in self.flattened(charge)?.into_iter().zip(&self.subpaths) {
            let fitted = fit_subpath(polyline.points, polyline.closed, tolerance);
            subpaths.push(match fitted {
                Some(fitted) if fitted.segments.len() <= original.segments.len() => fitted,
                _ => original.clone(),
            });
        }
        Self { subpaths }.within_limit("tolerance")
    }
}

// Vectors --------------------------------------------------------------------------------------------------------------

fn minus(a: Point, b: Point) -> Point {
    Point::new(a.x - b.x, a.y - b.y)
}

fn plus(a: Point, b: Point) -> Point {
    Point::new(a.x + b.x, a.y + b.y)
}

fn times(a: Point, factor: f64) -> Point {
    Point::new(a.x * factor, a.y * factor)
}

fn dot(a: Point, b: Point) -> f64 {
    a.x * b.x + a.y * b.y
}

fn length(a: Point) -> f64 {
    dot(a, a).sqrt()
}

/// `a` scaled to length 1, or `(0, 0)` where it has none.
fn unit(a: Point) -> Point {
    let size = length(a);
    if size > 0.0 && size.is_finite() {
        times(a, 1.0 / size)
    } else {
        Point::new(0.0, 0.0)
    }
}

/// The distance from `point` to the segment `from` to `to`.
fn to_segment(point: Point, from: Point, to: Point) -> f64 {
    let edge = minus(to, from);
    let square = dot(edge, edge);
    let along = if square > 0.0 {
        (dot(minus(point, from), edge) / square).clamp(0.0, 1.0)
    } else {
        0.0
    };
    length(minus(point, plus(from, times(edge, along))))
}

// Corners and runs -----------------------------------------------------------------------------------------------------

/// Whether the path turns by more than 45 degrees at `at`, coming from `before` and going to `after`.
fn is_corner(before: Point, at: Point, after: Point) -> bool {
    dot(unit(minus(at, before)), unit(minus(after, at))) < CORNER_COS
}

/// The sub-path through `points` (consecutive equal points dropped) as few curves as the fit finds within `tolerance`; `None`
/// where there is no length to fit.
fn fit_subpath(mut points: Vec<Point>, closed: bool, tolerance: f64) -> Option<SubPath> {
    points.dedup();
    if closed && points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    let count = points.len();
    if count < 2 {
        return None;
    }
    let at = |index: usize| points[index % count];
    let corner = |index: usize| is_corner(at(index + count - 1), at(index), at(index + 1));
    // where the runs break: the ends of an open sub-path, the written start of a closed one, and every corner
    let mut breaks = vec![0];
    breaks.extend((1..count).filter(|&index| (closed || index + 1 < count) && corner(index)));
    breaks.push(if closed { count } else { count - 1 });
    // a closed sub-path's start is smooth, with a tangent from both neighbours, unless it is itself a corner
    let smooth_start = closed && !corner(0);
    let mut subpath = SubPath::open(points[0]);
    for pair in breaks.windows(2) {
        let (lo, hi) = (pair[0], pair[1]);
        let run: Vec<Point> = (lo..=hi).map(at).collect();
        let centre = unit(minus(at(1), at(count - 1)));
        let left = if lo == 0 && smooth_start {
            centre
        } else {
            unit(minus(run[1], run[0]))
        };
        let right = if hi == count && smooth_start {
            times(centre, -1.0)
        } else {
            unit(minus(run[run.len() - 2], run[run.len() - 1]))
        };
        fit_run(&run, left, right, tolerance, &mut subpath.segments);
    }
    Some(if closed { subpath.closed() } else { subpath })
}

/// One run still to fit: the points `lo` to `hi` of the run, with the unit tangents pointing into it from its ends.
struct Job {
    lo: usize,
    hi: usize,
    left: Point,
    right: Point,
}

/// Appends the segments that fit `run` within `tolerance`. An explicit stack, not recursion, so the depth of the
/// splitting costs no stack; each split lies in the middle half of its job, so the stack holds at most about
/// `log(n)` jobs and the work is `O(n log n)`.
fn fit_run(run: &[Point], left: Point, right: Point, tolerance: f64, into: &mut Vec<Segment>) {
    let mut stack = vec![Job {
        lo: 0,
        hi: run.len() - 1,
        left,
        right,
    }];
    while let Some(job) = stack.pop() {
        let points = &run[job.lo..=job.hi];
        let last = points[points.len() - 1];
        if points.len() == 2 || is_flat(points, tolerance) {
            into.push(Segment::Line(last));
            continue;
        }
        let split = match fit_cubic(points, job.left, job.right, tolerance) {
            Fit::Within(first, second) => {
                into.push(Segment::Cubic(first, second, last));
                continue;
            }
            Fit::Worst(worst) => worst + job.lo,
        };
        // inside the middle half, so a split never peels one point off a long run
        let margin = ((job.hi - job.lo) / 4).max(1);
        let split = split.clamp(job.lo + margin, job.hi - margin);
        let centre = unit(minus(run[split - 1], run[split + 1]));
        stack.push(Job {
            lo: split,
            hi: job.hi,
            left: times(centre, -1.0),
            right: job.right,
        });
        stack.push(Job {
            lo: job.lo,
            hi: split,
            left: job.left,
            right: centre,
        });
    }
}

/// Whether every point is within `tolerance` of the chord from the first to the last, which has a length. The
/// points are joined end to end, so the chord is then within `tolerance` of the whole run too.
fn is_flat(points: &[Point], tolerance: f64) -> bool {
    let (first, last) = (points[0], points[points.len() - 1]);
    first != last && points.iter().all(|&point| to_segment(point, first, last) <= tolerance)
}

// The fit --------------------------------------------------------------------------------------------------------------

/// The result of fitting a cubic to a run.
enum Fit {
    /// The two inner control points of a cubic within the tolerance.
    Within(Point, Point),
    /// No cubic was close enough; the index in the run of the point it was furthest from.
    Worst(usize),
}

/// The cubic from the first point to the last with the given end tangents that follows `points` best, within
/// `tolerance` everywhere along it (`max_error` bounds it between points too), or where it fails the worst point.
fn fit_cubic(points: &[Point], left: Point, right: Point, tolerance: f64) -> Fit {
    let mut parameters = chord_lengths(points);
    for round in 0..=NEWTON_ROUNDS {
        let bezier = generate(points, &parameters, left, right);
        let (error, worst) = max_error(points, &parameters, &bezier, tolerance);
        if error <= tolerance {
            return Fit::Within(bezier[1], bezier[2]);
        }
        if error.is_nan() || error > RESCUE * tolerance || round == NEWTON_ROUNDS {
            return Fit::Worst(worst);
        }
        let Some(better) = reparametrised(points, &parameters, &bezier) else {
            return Fit::Worst(worst);
        };
        parameters = better;
    }
    Fit::Worst(points.len() / 2)
}

/// The share of the run's length at each point: 0 at the first, 1 at the last.
fn chord_lengths(points: &[Point]) -> Vec<f64> {
    let mut along = vec![0.0];
    let mut total = 0.0;
    for pair in points.windows(2) {
        total += length(minus(pair[1], pair[0]));
        along.push(total);
    }
    let mut shares: Vec<f64> = along.iter().map(|&distance| distance / total).collect();
    if let Some(last) = shares.last_mut() {
        *last = 1.0;
    }
    shares
}

/// The Bernstein weights of a cubic at `u`.
fn weights(u: f64) -> [f64; 4] {
    let v = 1.0 - u;
    [v * v * v, 3.0 * v * v * u, 3.0 * v * u * u, u * u * u]
}

fn at(bezier: &[Point; 4], u: f64) -> Point {
    let w = weights(u);
    let mut sum = Point::new(0.0, 0.0);
    for (point, weight) in bezier.iter().zip(w) {
        sum = plus(sum, times(*point, weight));
    }
    sum
}

/// The cubic through the ends of `points` with unit tangents `left` and `right` whose control points lie along them
/// at the distances that best follow the points at `parameters` (least squares).
fn generate(points: &[Point], parameters: &[f64], left: Point, right: Point) -> [Point; 4] {
    let (first, last) = (points[0], points[points.len() - 1]);
    let (mut c00, mut c01, mut c11, mut x0, mut x1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (&point, &u) in points.iter().zip(parameters) {
        let w = weights(u);
        let (a0, a1) = (times(left, w[1]), times(right, w[2]));
        c00 += dot(a0, a0);
        c01 += dot(a0, a1);
        c11 += dot(a1, a1);
        let ends = plus(times(first, w[0] + w[1]), times(last, w[2] + w[3]));
        let rest = minus(point, ends);
        x0 += dot(a0, rest);
        x1 += dot(a1, rest);
    }
    let det = c00 * c11 - c01 * c01;
    let (alpha_left, alpha_right) = ((x0 * c11 - x1 * c01) / det, (c00 * x1 - c01 * x0) / det);
    let chord = length(minus(last, first));
    let scale = if chord > 0.0 {
        chord
    } else {
        // a loop: its ends meet, so a third of the way round is the reach of a tangent
        points
            .windows(2)
            .map(|pair| length(minus(pair[1], pair[0])))
            .sum::<f64>()
            / 2.0
    };
    // a control point behind its end or on it is a degenerate fit: take a third of the reach (Wu and Barsky)
    let usable = |alpha: f64| alpha.is_finite() && alpha >= DEGENERATE * scale;
    let (alpha_left, alpha_right) = if usable(alpha_left) && usable(alpha_right) {
        (alpha_left, alpha_right)
    } else {
        (scale / 3.0, scale / 3.0)
    };
    [
        first,
        plus(first, times(left, alpha_left)),
        plus(last, times(right, alpha_right)),
        last,
    ]
}

/// The largest distance seen and the index it belongs to.
struct Worst(f64, usize);

impl Worst {
    fn consider(&mut self, distance: f64, at: usize) {
        // a NaN distance is the worst of all
        if distance.is_nan() || distance > self.0 {
            *self = Self(distance, at);
        }
    }

    fn beyond(&self, tolerance: f64) -> bool {
        self.0.is_nan() || self.0 > tolerance
    }
}

/// An upper bound on how far the cubic is from the run, and the index of the point to split at.
///
/// First the exact distance from each point at its own parameter. If those are within `tolerance`, the rest of the
/// curve is covered by a speed bound: a cubic moves at most `3 * (its longest control leg)` per unit of parameter,
/// so every point of a parameter interval is within `speed * width / 2` of the curve at its middle. An interval of
/// an edge's parameter span is proven when the distance of that middle from the edge plus this reach is within
/// `tolerance`, and is halved when it is not, so a curve well inside the tolerance costs a few checks and one near
/// it more. A fit that needs more than `COVER_CHECKS` checks per point is reported unbounded (and so split), which
/// keeps one fit linear in the points of its run.
fn max_error(points: &[Point], parameters: &[f64], bezier: &[Point; 4], tolerance: f64) -> (f64, usize) {
    let mut worst = Worst(0.0, points.len() / 2);
    for (at_index, (&point, &u)) in points.iter().zip(parameters).enumerate() {
        worst.consider(length(minus(at(bezier, u), point)), at_index);
    }
    if worst.beyond(tolerance) {
        return (worst.0, worst.1);
    }
    let speed = 3.0
        * (1..4)
            .map(|leg| length(minus(bezier[leg], bezier[leg - 1])))
            .fold(0.0, f64::max);
    let mut budget = COVER_CHECKS * points.len();
    let mut open = Vec::new();
    for (edge, pair) in points.windows(2).enumerate() {
        open.push((parameters[edge], parameters[edge + 1]));
        while let Some((from, to)) = open.pop() {
            let distance = to_segment(at(bezier, f64::midpoint(from, to)), pair[0], pair[1]);
            worst.consider(distance, edge + 1);
            if worst.beyond(tolerance) {
                return (worst.0, worst.1);
            }
            if distance + speed * (to - from) / 2.0 > tolerance {
                budget = budget.saturating_sub(1);
                if budget == 0 {
                    return (f64::INFINITY, edge + 1);
                }
                let middle = f64::midpoint(from, to);
                open.extend([(from, middle), (middle, to)]);
            }
        }
    }
    (worst.0, worst.1)
}

/// The parameters moved one step of Newton's method towards the nearest point of the cubic to each point, or `None`
/// where that would not keep them in increasing order.
fn reparametrised(points: &[Point], parameters: &[f64], bezier: &[Point; 4]) -> Option<Vec<f64>> {
    let last = points.len() - 1;
    let mut better = parameters.to_vec();
    for index in 1..last {
        let u = parameters[index];
        let (v, w) = (1.0 - u, u);
        let first = times(
            plus(
                plus(
                    times(minus(bezier[1], bezier[0]), v * v),
                    times(minus(bezier[2], bezier[1]), 2.0 * v * w),
                ),
                times(minus(bezier[3], bezier[2]), w * w),
            ),
            3.0,
        );
        let second = times(
            plus(
                times(plus(minus(bezier[2], times(bezier[1], 2.0)), bezier[0]), v),
                times(plus(minus(bezier[3], times(bezier[2], 2.0)), bezier[1]), w),
            ),
            6.0,
        );
        let off = minus(at(bezier, u), points[index]);
        let (numerator, denominator) = (dot(off, first), dot(first, first) + dot(off, second));
        let step = numerator / denominator;
        if denominator != 0.0 && step.is_finite() {
            better[index] = (u - step).clamp(0.0, 1.0);
        }
    }
    better.windows(2).all(|pair| pair[0] < pair[1]).then_some(better)
}
