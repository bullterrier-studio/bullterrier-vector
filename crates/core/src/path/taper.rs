//! Tapered outlines: the region a stroke whose width follows a profile along its length paints.
//!
//! The center line is the measured walk (`walk.rs`, so a position here is a position `dash` and `slice` measure), cut
//! further at the stops of the profile so that the radius changes linearly along every piece. The sweep of a disc
//! whose radius changes linearly along a straight line is the convex hull of its two end discs; the outline is the
//! union of these hulls, which `offset.rs` makes canonical. Joins and caps are therefore round.

use super::Path;
use super::boolean::{Contour, checked_tolerance};
use super::offset::united;
use super::walk::Run;
use crate::Error;
use crate::geometry::{Point, coordinate};
use crate::limits::{MAX_PROFILE_STOPS, MAX_SEGMENTS};
use crate::math;
use crate::shape::{Cap, Join};
use std::f64::consts::PI;

/// The most a factor of a profile may be.
// constant: as wide as a width may be, 1e100 design pixels; the radius is checked as a coordinate with the width
const MAX_FACTOR: f64 = 1e100;

impl Path {
    /// The region a stroke along this path paints when its width is `width` times the factor of `profile` at each place,
    /// as closed outlines. `profile` is a list of `(position, factor)`: the position runs from 0 at the start of each
    /// sub-path to 1 at its end, measured by arc length along that sub-path on its own (a closed one from its topmost
    /// point, as [`Path::slice`] and [`Path::dash`] walk it), so every open sub-path tapers at its own ends, each piece of a dashed
    /// line included. Factors are linear between stops and hold the first and last value outside them. Two stops at
    /// one position make a step. A closed sub-path has no free ends: the profile runs once round it, and where the
    /// end meets the start the width steps from the last factor to the first.
    ///
    /// The stroke is the sweep of a round disc of radius `width * factor / 2`, so joins and caps are round, and a
    /// factor of 0 at a free end closes the outline to a point. The curve is measured as [`Path::length`] measures it
    /// (flattened to 0.01 design pixels); `tolerance` is how far the round parts stray from true circles. A sub-path
    /// that is a lone move, or has no length (a line to its own start), paints nothing, as in [`Path::outline`]. The result is in canonical form and straight-edged.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `profile` if it is empty, has more than 1,000 stops, a position outside 0 to 1
    /// or smaller than the one before, or a factor not a number from 0 to 1e100; naming `width` if it is not above 0 and
    /// within 1e100, if a radius is, or if the corners of the discs, over all the pieces, are more than `MAX_SEGMENTS`
    /// (10,000,000), which is checked before any is made; naming `tolerance` if it is not above 0 and within 1e100;
    /// or the result has a coordinate beyond ±1e100.
    pub fn tapered_outline(&self, width: f64, profile: &[(f64, f64)], tolerance: f64) -> Result<Self, Error> {
        self.tapered_outline_charged(width, profile, tolerance, &mut |count| {
            if count > MAX_SEGMENTS {
                Err(Error::parameter("width", format!("MAX_SEGMENTS {MAX_SEGMENTS} passed")))
            } else {
                Ok(())
            }
        })
    }

    /// As [`Path::tapered_outline`], calling `charge` once with the most corners the hulls may have, before any is
    /// made, then as [`Path::resolved`] does for the pairs of edges and the vertices of the result.
    pub(crate) fn tapered_outline_charged(
        &self,
        width: f64,
        profile: &[(f64, f64)],
        tolerance: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let tolerance = checked_tolerance(tolerance)?;
        if width <= 0.0 {
            return Err(Error::parameter(
                "width",
                format!("must be a number above 0, got {width}"),
            ));
        }
        let width = coordinate("width", width)?;
        let profile = Profile::new(profile)?;
        let reach = coordinate("width", width * profile.peak / 2.0)?;
        let runs: Vec<Run<'_>> = self
            .subpaths
            .iter()
            .map(|subpath| Run::new(subpath, 0.0, None))
            // a lone move, and a line to its own start, paint nothing, as in `outline`
            .filter(|run| run.length > 0.0)
            .collect();
        if reach <= 0.0 || runs.is_empty() {
            return Ok(Self::new());
        }
        // the most pieces: each edge, cut at most at every stop
        #[expect(clippy::cast_precision_loss, reason = "counts far below 2^53")]
        let pieces: f64 = runs
            .iter()
            .map(|run| (run.edges.len() + profile.stops.len()) as f64)
            .sum();
        let wanted = pieces * (corners(reach, tolerance) + 4.0);
        // constant: above this the count is past any limit and would not fit the cast below
        if wanted > 1e15 {
            charge(MAX_SEGMENTS + 1)?;
        } else {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "positive and below 1e15"
            )]
            charge(wanted as usize)?;
        }
        let mut polygons: Vec<Contour> = Vec::new();
        for run in &runs {
            sweep(run, width / 2.0, &profile, tolerance, &mut polygons);
        }
        united(&polygons, charge)
    }
}

/// Checks that `join` and `cap` are round: a stroke that varies along its length is the sweep of a round disc, so
/// [`Path::tapered_outline`] and a drawing's `outline` with a `profile` take no other.
///
/// # Errors
/// [`Error::Parameter`] naming `join` or `cap` if it is not `Round`.
pub fn require_round_stroke(join: Join, cap: Cap) -> Result<(), Error> {
    if join != Join::Round {
        return Err(Error::parameter(
            "join",
            format!("must be \"round\" with a profile, got \"{}\"", join.name()),
        ));
    }
    if cap != Cap::Round {
        return Err(Error::parameter(
            "cap",
            format!("must be \"round\" with a profile, got \"{}\"", cap.name()),
        ));
    }
    Ok(())
}

/// Checks a stroke's `profile` as [`Path::tapered_outline`] does, so that a step refuses it where it is read.
///
/// # Errors
/// [`Error::Parameter`] naming `profile`, as `tapered_outline` does.
pub(crate) fn check_profile(stops: &[(f64, f64)]) -> Result<(), Error> {
    Profile::new(stops).map(|_| ())
}

/// A width profile, checked: `(position, factor)` stops with positions from 0 to 1 that never go back.
struct Profile<'a> {
    stops: &'a [(f64, f64)],
    /// The largest factor.
    peak: f64,
}

impl<'a> Profile<'a> {
    fn new(stops: &'a [(f64, f64)]) -> Result<Self, Error> {
        let refuse = |message: String| Err(Error::parameter("profile", message));
        if stops.is_empty() || stops.len() > MAX_PROFILE_STOPS {
            return refuse(format!(
                "must have from 1 to {MAX_PROFILE_STOPS} stops, got {}",
                stops.len()
            ));
        }
        let mut before = 0.0;
        let mut peak = 0.0_f64;
        for &(position, factor) in stops {
            if !(0.0..=1.0).contains(&position) {
                return refuse(format!("every position must be a number from 0 to 1, got {position}"));
            }
            if position < before {
                return refuse(format!("positions must not go back, got {position} after {before}"));
            }
            if !(0.0..=MAX_FACTOR).contains(&factor) {
                return refuse(format!("every factor must be a number from 0 to 1e100, got {factor}"));
            }
            before = position;
            peak = peak.max(factor);
        }
        Ok(Self { stops, peak })
    }

    /// The factors at the positions `a` and `b` of a piece between two neighbouring places the profile changes
    /// direction (stops, or the ends of the walk): the line of the stop pair the piece lies in, so a step is read
    /// from the side the piece is on.
    fn between(&self, a: f64, b: f64) -> (f64, f64) {
        let middle = f64::midpoint(a, b);
        let next = self.stops.partition_point(|&(position, _)| position <= middle);
        match (next.checked_sub(1).map(|at| self.stops[at]), self.stops.get(next)) {
            (Some((p0, f0)), Some(&(p1, f1))) => {
                let at = |position: f64| f0 + (f1 - f0) * (position.clamp(p0, p1) - p0) / (p1 - p0);
                (at(a), at(b))
            }
            (Some((_, held)), None) | (None, Some(&(_, held))) => (held, held),
            (None, None) => (0.0, 0.0),
        }
    }
}

/// The widest step (radians) an arc of `radius` may be cut into so that its chords stray from it by at most `tolerance`,
/// at most an eighth of a turn.
fn arc_step(radius: f64, tolerance: f64) -> f64 {
    // a chord of step s is radius * (1 - cos(s / 2)) from the arc; 1 - cos(h) = 2 sin(h / 2)^2 keeps small ratios exact
    // constant: at most an eighth of a turn, so a disc has at least 8 sides however coarse the tolerance
    (4.0 * math::asin(((tolerance / radius).min(1.0) / 2.0).sqrt())).min(PI / 4.0)
}

/// How many corners a whole circle of `radius` has (infinite when the step is below what a number holds).
fn corners(radius: f64, tolerance: f64) -> f64 {
    (2.0 * PI / arc_step(radius, tolerance)).ceil()
}

/// The hulls of the discs along `run`, with radius `half * factor`, added to `polygons`.
fn sweep(run: &Run<'_>, half: f64, profile: &Profile<'_>, tolerance: f64, polygons: &mut Vec<Contour>) {
    let length = run.length;
    let stops: Vec<f64> = profile.stops.iter().map(|&(position, _)| position * length).collect();
    let mut next = 0;
    for edge in &run.edges {
        if edge.length <= 0.0 {
            continue;
        }
        let end = edge.start + edge.length;
        while next < stops.len() && stops[next] <= edge.start {
            next += 1;
        }
        let place = |at: f64| {
            let share = (at - edge.start) / edge.length;
            Point::new(
                edge.from.x + (edge.to.x - edge.from.x) * share,
                edge.from.y + (edge.to.y - edge.from.y) * share,
            )
        };
        let (mut cursor, mut at_start) = (edge.start, true);
        loop {
            let cut = next < stops.len() && stops[next] < end;
            let finish = if cut { stops[next] } else { end };
            next += usize::from(cut);
            if finish > cursor {
                let (r0, r1) = profile.between(cursor / length, finish / length);
                let from = if at_start { edge.from } else { place(cursor) };
                let to = if cut { place(finish) } else { edge.to };
                hull(from, half * r0, to, half * r1, tolerance, polygons);
                (cursor, at_start) = (finish, false);
            }
            if !cut {
                break;
            }
        }
    }
}

/// The convex hull of the discs `(a, r0)` and `(b, r1)` as one polygon, every corner on a circle: the part of the
/// first disc facing away from the second, the part of the second facing away from the first, and the two common
/// tangents between them. A disc that holds the other is the polygon of that disc alone; two discs of no radius make
/// nothing. All polygons turn the same way, so the union by the nonzero rule is the union of regions.
fn hull(a: Point, r0: f64, b: Point, r1: f64, tolerance: f64, polygons: &mut Vec<Contour>) {
    if r0 <= 0.0 && r1 <= 0.0 {
        return;
    }
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let distance = (dx * dx + dy * dy).sqrt();
    let step = arc_step(r0.max(r1), tolerance);
    let mut corners: Contour = Vec::new();
    if distance <= (r0 - r1).abs() || distance <= 0.0 {
        let (center, radius) = if r0 >= r1 { (a, r0) } else { (b, r1) };
        arc(&mut corners, center, radius, 0.0, 2.0 * PI, step);
        corners.pop();
    } else {
        let (direction, spread) = (math::atan2(dy, dx), math::acos((r0 - r1) / distance));
        arc(
            &mut corners,
            a,
            r0,
            direction + spread,
            direction + 2.0 * PI - spread,
            step,
        );
        arc(&mut corners, b, r1, direction - spread, direction + spread, step);
    }
    polygons.push(corners);
}

/// The corners of the arc of the circle `(center, radius)` from angle `from` to `to` (increasing), both ends included,
/// no step wider than `step`; a circle of no radius is its center.
fn arc(corners: &mut Contour, center: Point, radius: f64, from: f64, to: f64, step: f64) {
    if radius <= 0.0 {
        corners.push([center.x, center.y]);
        return;
    }
    let count = ((to - from) / step).ceil().max(1.0);
    let mut index = 0.0;
    while index <= count {
        let (sin, cos) = math::sin_cos(from + (to - from) * index / count);
        corners.push([center.x + radius * cos, center.y + radius * sin]);
        index += 1.0;
    }
}
