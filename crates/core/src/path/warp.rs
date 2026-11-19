//! Warping: every point of a path moved by a map of the plane. The kernel [`Path::warp`] owns the part every deformer
//! shares (cutting the edges so a bend shows, keeping the corners, closedness, the limits); a verb is a map and a step
//! length. [`Path::wave`] is the first.

use super::curve::lerp_number;
use super::{Path, Polyline, Segment, SubPath};
use crate::Error;
use crate::geometry::{Point, coordinate, finite};
use crate::limits::MAX_SEGMENTS;
use crate::math::{sin_cos, sin_cos_deg};
use crate::noise::{self, Noise};
use std::f64::consts::TAU;

/// The pieces of a wavelength a wave's edges are cut into at most.
// constant: a chord of a sine of amplitude A that spans a 24th of a wavelength strays from it by at most
// A (2 pi / 24)^2 / 8 = 0.86% of A, under the hundredth of the amplitude that a hand-drawn wave can be told from a
// sine by; fewer pieces (16: 1.9%) show as facets on a large amplitude
const WAVE_PIECES: f64 = 24.0;

/// The chord error of a wave cut into `WAVE_PIECES`, as a share of the amplitude.
// constant: A (2 pi / 24)^2 / 8, worked out above
const WAVE_ERROR: f64 = 0.0086;

/// The pieces of a wavelength a wobble's edges are cut into at most.
// constant: the noise changes by at most 3.75 amplitudes per wavelength per axis and bends by at most 25.6 amplitudes per
// wavelength squared along a line (2 * 5.77 for the fade's second derivative, 10 sqrt(3) / 3, plus the mixed term 4 (15/8)^2),
// two fields at most sqrt(2) times that, so a chord over a step h strays by at most sqrt(2) 25.6 (h / wavelength)^2 / 8 of
// the amplitude: 0.79% for a 24th of the wavelength, under the hundredth (the bound is loose; 16 pieces would give 1.8%)
const WOBBLE_PIECES: f64 = 24.0;

/// The chord error of a wobble cut into `WOBBLE_PIECES`, as a share of the amplitude.
// constant: the bound worked out above (the real error measured is far under it)
const WOBBLE_ERROR: f64 = 0.0079;

/// The most a chord of any deformer's result may stray from the map it stands for, in design pixels.
// constant: a relative error (a share of the amplitude, radius or width) is a bigger absolute one on a big shape, and a
// zoomed render shows it as facets: 0.5% of a 190 px radius is 0.95 px. A twentieth of a pixel is 0.4 device pixels at
// 8x zoom, below what shows; a deformer takes the smaller of this and its own relative bound, so a small shape costs
// what it did and only a big one gets more points
const DEFORM_ERROR_PX: f64 = 0.05;

/// The step a verb cuts with when its own `step` leaves chords `error` design pixels off the map: `step` where that is
/// within `DEFORM_ERROR_PX`, else the step shortened so it is (a chord's error goes as the square of the step).
fn within_budget(step: f64, error: f64) -> f64 {
    if error > DEFORM_ERROR_PX {
        step * libm::sqrt(DEFORM_ERROR_PX / error)
    } else {
        step
    }
}

impl Path {
    /// This path with its points displaced sideways by a sine travelling towards `angle` (degrees, clockwise on the
    /// screen as for [`Path::rotate`], `y` down: 0 travels towards +x): a point `(x, y)` at distance
    /// `d = x cos(angle) + y sin(angle)` along the direction of travel moves by
    /// `amplitude * sin(2 pi (d / wavelength - phase))` along the perpendicular `(-sin(angle), cos(angle))`. At angle 0 a
    /// point moves down the screen where the sine is positive. `phase` is in cycles and wraps (`phase + 1` is `phase`);
    /// `amplitude` may be 0 or negative.
    ///
    /// The wave is anchored in the design frame, the point's own coordinates, not the shape's: the letters of a word
    /// waved in one step share one wave. A shape translated after the wave carries it along; one translated before it
    /// slides through the wave.
    ///
    /// The path is flattened to straight edges, as for drawing, and each edge is cut into equal pieces no longer than
    /// a 24th of the wavelength, so that every point of the result is within 0.86% of `amplitude` of the sine it
    /// stands for and within 0.05 design pixels of it (a shorter step where 0.86% of `amplitude` is more); every vertex of the flattening is kept, so corners stay corners. The result is made of straight
    /// lines; each sub-path keeps its closedness, a sub-path of a single point stays one (moved), an empty path stays
    /// empty.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `wavelength` if it is not a number from 1e-200 to 1e100, or if the pieces it
    /// would make, over all the sub-paths, are more than `MAX_SEGMENTS` (10,000,000), counted before any is made;
    /// naming `amplitude`, `angle` or `phase` if it is NaN or infinite, and `amplitude` if a displaced point would be
    /// beyond ±1e100.
    pub fn wave(&self, amplitude: f64, wavelength: f64, angle: f64, phase: f64) -> Result<Self, Error> {
        self.wave_charged(amplitude, wavelength, angle, phase, &mut segment_budget("wavelength"))
    }

    /// As [`Path::wave`], calling `charge` with the number of points the result would have, before any is made; its
    /// error stops the verb.
    pub(crate) fn wave_charged(
        &self,
        amplitude: f64,
        wavelength: f64,
        angle: f64,
        phase: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let amplitude = finite("amplitude", amplitude)?;
        if wavelength <= 0.0 {
            return Err(Error::parameter(
                "wavelength",
                format!("must be a number above 0, got {wavelength}"),
            ));
        }
        let wavelength = coordinate("wavelength", wavelength)?;
        let angle = finite("angle", angle)?;
        // the phase is reduced before any maths, so that a whole cycle more is the same bits
        let phase = finite("phase", phase)?.rem_euclid(1.0);
        let (sin, cos) = sin_cos_deg(angle);
        let map = |point: Point| {
            let along = point.x * cos + point.y * sin;
            let sideways = amplitude * sin_cos(TAU * (along / wavelength - phase)).0;
            Point::new(point.x - sideways * sin, point.y + sideways * cos)
        };
        self.warp(
            "amplitude",
            wavelength / WAVE_PIECES,
            WAVE_ERROR * amplitude.abs(),
            &map,
            charge,
        )
    }

    /// This path with its points displaced by smooth seeded noise: a point `(x, y)` moves by `(dx, dy)`, two independent
    /// fields (streams 0 and 1 of `seed`) of value noise whose lattice is `wavelength` wide, each scaled so that
    /// `|dx|` and `|dy|` are at most `amplitude`. The field is smooth: it changes by at most `3.75` amplitudes per
    /// wavelength along either axis. `phase` is in cycles, reduced to 0 up to 1 first (`phase + 1` is `phase`, to the
    /// bit), and moves the noise smoothly, four lattice cells to the cycle. `amplitude` may be 0 or negative; `seed`
    /// is a whole number within ±(2^53 - 1) as for `random`.
    ///
    /// The field is anchored in the design frame, the point's own coordinates (as for [`Path::wave`]): the letters of
    /// a word wobbled in one step share one field. The path is flattened and each edge cut into equal pieces no longer
    /// than a 24th of the wavelength, every vertex kept (corners stay corners); a chord strays from the displaced
    /// edge by at most 0.79% of `amplitude` and by at most 0.05 design pixels (a shorter step where that is more). The result is made of straight lines, each sub-path keeping its
    /// closedness.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `wavelength` if it is not a number from 1e-200 to 1e100, or if the pieces it
    /// would make, over all the sub-paths, are more than `MAX_SEGMENTS`, counted before any is made; naming
    /// `amplitude` or `phase` if it is NaN or infinite, and `amplitude` if a displaced point would be beyond ±1e100;
    /// naming `seed` unless it is a whole number within ±(2^53 - 1).
    pub fn wobble(&self, amplitude: f64, wavelength: f64, phase: f64, seed: f64) -> Result<Self, Error> {
        self.wobble_charged(amplitude, wavelength, phase, seed, &mut segment_budget("wavelength"))
    }

    /// As [`Path::wobble`], calling `charge` with the number of points the result would have, before any is made.
    pub(crate) fn wobble_charged(
        &self,
        amplitude: f64,
        wavelength: f64,
        phase: f64,
        seed: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let amplitude = finite("amplitude", amplitude)?;
        let wavelength = noise::wavelength(wavelength)?;
        let phase = finite("phase", phase)?;
        let (across, down) = (Noise::new(seed, 0)?, Noise::new(seed, 1)?);
        let map = |point: Point| {
            let (x, y) = (point.x / wavelength, point.y / wavelength);
            Point::new(
                point.x + amplitude * across.value(x, y, phase),
                point.y + amplitude * down.value(x, y, phase),
            )
        };
        self.warp(
            "amplitude",
            wavelength / WOBBLE_PIECES,
            WOBBLE_ERROR * amplitude.abs(),
            &map,
            charge,
        )
    }

    /// This path with every point moved by `map`: flattened to straight edges, each edge cut into `ceil(length /
    /// step)` equal pieces (never more than the edge is long in pieces of `step`, one piece for a shorter edge), every
    /// vertex of the flattening kept, and every point of the result `map` of the point it comes from. A closed
    /// sub-path stays closed; a sub-path of a single point is kept and mapped; an empty path stays empty.
    ///
    /// The pieces of all the sub-paths are counted and `charge`d once before any is made, so a `step` that is small
    /// next to the path is refused, not allocated; `charge` is also told the edges of each curve as it is flattened.
    /// `parameter` is what an error names when `map` carries a point beyond ±1e100 or to a NaN.
    ///
    /// `step` is the verb's own cut and `error` the most a chord of that cut strays from the map, in design pixels; a
    /// step whose chords stray by more than `DEFORM_ERROR_PX` is shortened until they do not, so the result of any
    /// deformer is smooth to a twentieth of a pixel at any size.
    ///
    /// # Errors
    /// What `charge` returns; [`Error::Parameter`] naming `parameter` for a point out of range.
    pub(crate) fn warp(
        &self,
        parameter: &'static str,
        step: f64,
        error: f64,
        map: &dyn Fn(Point) -> Point,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let polylines = self.flattened(charge)?;
        let count = |step: f64| -> f64 {
            polylines
                .iter()
                .map(|polyline| 1.0 + edges(polyline).map(|(from, to)| pieces(from, to, step)).sum::<f64>())
                .sum()
        };
        // a step that is small next to the path is refused first, as the verb's own cut would be (a count that is not
        // a number, from a step that is not, is refused with the rest)
        #[expect(clippy::cast_precision_loss, reason = "10 million is exact in f64")]
        let fits = |wanted: f64| wanted <= MAX_SEGMENTS as f64;
        if !fits(count(step)) {
            charge(MAX_SEGMENTS + 1)?;
        }
        // then a map that sends a vertex out of range is refused by name: a huge amplitude would shorten the step to
        // nothing below, and the count would name the wavelength for it
        let moved: Vec<Segment> = polylines
            .iter()
            .flat_map(|polyline| &polyline.points)
            .map(|&point| Segment::Line(map(point)))
            .collect();
        Self {
            subpaths: vec![SubPath {
                start: Point::new(0.0, 0.0),
                segments: moved,
                closed: false,
            }],
        }
        .within_limit(parameter)?;
        let step = within_budget(step, error);
        let wanted = count(step);
        if fits(wanted) {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "0 to MAX_SEGMENTS"
            )]
            charge(wanted as usize)?;
        } else {
            charge(MAX_SEGMENTS + 1)?;
        }
        let subpaths = polylines.iter().map(|polyline| warped(polyline, step, map)).collect();
        Self { subpaths }.within_limit(parameter)
    }
}

/// A count of the edges of a path that fails with `parameter` past `MAX_SEGMENTS`, over all it is told.
pub(super) fn segment_budget(parameter: &'static str) -> impl FnMut(usize) -> Result<(), Error> {
    let mut used = 0_usize;
    move |count| {
        used = used.saturating_add(count);
        if used > MAX_SEGMENTS {
            Err(Error::parameter(
                parameter,
                format!("MAX_SEGMENTS {MAX_SEGMENTS} passed"),
            ))
        } else {
            Ok(())
        }
    }
}

/// The straight edges of a polyline, the one that closes a ring last.
fn edges(polyline: &Polyline) -> impl Iterator<Item = (Point, Point)> + '_ {
    let corners = &polyline.points;
    let count = if polyline.closed {
        corners.len()
    } else {
        corners.len() - 1
    };
    (0..count).map(|index| (corners[index], corners[(index + 1) % corners.len()]))
}

/// How many equal pieces the edge is cut into: as many as the step goes into its length, rounded up, and one for an
/// edge with no length. Not a number when `step` is not.
fn pieces(from: Point, to: Point, step: f64) -> f64 {
    let length = libm::hypot(to.x - from.x, to.y - from.y);
    if length > 0.0 {
        (length / step).ceil().max(1.0)
    } else {
        1.0
    }
}

/// The sub-path `polyline` with each edge cut and every point mapped.
fn warped(polyline: &Polyline, step: f64, map: &dyn Fn(Point) -> Point) -> SubPath {
    let mut points = vec![map(polyline.points[0])];
    let count = polyline.points.len();
    for (index, (from, to)) in edges(polyline).enumerate() {
        let cuts = pieces(from, to, step);
        let mut cut = 1.0;
        while cut < cuts {
            let share = cut / cuts;
            points.push(map(Point::new(
                lerp_number(from.x, to.x, share),
                lerp_number(from.y, to.y, share),
            )));
            cut += 1.0;
        }
        // the edge that closes a ring ends where the ring began
        if !(polyline.closed && index + 1 == count) {
            points.push(map(to));
        }
    }
    let mut segments: Vec<Segment> = points[1..].iter().map(|&point| Segment::Line(point)).collect();
    if polyline.drawn && segments.is_empty() {
        // a line to its own start still marks a place
        segments.push(Segment::Line(points[0]));
    }
    let subpath = SubPath {
        start: points[0],
        segments,
        closed: false,
    };
    if polyline.closed { subpath.closed() } else { subpath }
}
