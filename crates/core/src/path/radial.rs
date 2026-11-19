//! Radial deformers: [`Path::twist`] and [`Path::bulge`], maps of the plane about a center. Each is a map and a step
//! length for the warp kernel ([`Path::warp`]); the step is a share of the radius, the length scale of the map.

use super::Path;
use super::warp::segment_budget;
use crate::Error;
use crate::geometry::{Point, coordinate, coordinate_point, finite};
use crate::limits::MAX_SEGMENTS;
use crate::math::sin_cos_deg;

/// The share of the radius a chord of the map may stray from the map by at most, for every point of a twisted or
/// bulged edge.
// constant: a hundredth of the radius is the tolerance the tests hold the output to (and the visible limit of a
// smooth curve drawn in straight lines); the pieces are chosen for half of it, so that the flattening of curves
// (0.01 pixel) and rounding do not tip it over
const CHORD_ERROR: f64 = 0.005;

/// The share of the pieces of a radius a twist of `angle` degrees needs: `1 / step` in radii. A chord over a step `h` of
/// a line strays from its image by at most `h^2 / 8` times the largest second derivative of the image along the line.
/// With `a` the angle in radians, the turn `phi = a f(r)` has `|f'| <= 1.5 / radius` and `|f''| <= 6 / radius^2`, and
/// the image `R(phi) d` of the line `d(u)` (`|d'| = 1`, `|d| <= radius`) has `|image''| <= |phi''| |d| + phi'^2 |d| +
/// 2 |phi'| <= (10.5 a + 2.25 a^2) / radius`. The chord error `h^2 (10.5 a + 2.25 a^2) / (8 radius)` is within
/// `CHORD_ERROR * radius` for `h = radius / n`, `n = sqrt((10.5 a + 2.25 a^2) / (8 CHORD_ERROR))` = `7.5 sqrt(a^2 + 14 a / 3)`.
fn twist_pieces(angle: f64) -> f64 {
    let a = angle.abs().to_radians();
    libm::sqrt((10.5 * a + 2.25 * a * a) / (8.0 * CHORD_ERROR))
}

/// The pieces of a radius a bulge of `amount` needs. The image of a line `d(u)` is `m(r) d`, `m = 1 + amount f(r)`, so
/// `|image''| <= |amount| (|f''| |d| + 2 |f'|)` plus the Hessian terms of the perpendicular `|f'| / r |d|`, together at
/// most `9 |amount| / radius` (`|f'| <= 1.5 / radius`, `|f''| and |f'| / r <= 6 / radius^2`); with `h = radius / n` the
/// chord error is within `CHORD_ERROR * radius` for `n = sqrt(9 |amount| / (8 CHORD_ERROR))` = `15 sqrt(|amount|)`.
fn bulge_pieces(amount: f64) -> f64 {
    libm::sqrt(9.0 * amount.abs() / (8.0 * CHORD_ERROR))
}

/// The amount a bulge stays below: past it the map `r -> r'` folds over itself.
// constant: 16/11, from the slope of `r -> r'` at its lowest, `1 - amount * 11/16` at s = 3/4 (see `Path::bulge`)
pub(crate) const STRONGEST_BULGE: f64 = 16.0 / 11.0;

/// The words that refuse an `amount` outside its range, on a `Path` and on a `Drawing` alike.
// constant: the bound is written once, as a fraction and rounded, with the reason it is the bound
pub(crate) const STRONGEST_BULGE_RULE: &str = "above -1 and below 16/11 (1.4545), so that the shape does not fold over";

/// The weight of the effect at distance `r` from the center: 1 at the center, falling smoothly (slope 0 at both ends)
/// to 0 at `radius`. Called only for `r` under `radius`; a farther point is not moved.
fn falloff(r: f64, radius: f64) -> f64 {
    let s = r / radius;
    1.0 - s * s * (3.0 - 2.0 * s)
}

impl Path {
    /// This path with every point turned about `about` by `angle * falloff(r)` degrees, `r` its distance from `about`
    /// and `falloff(r) = 1 - s^2 (3 - 2s)`, `s = min(r / radius, 1)`: the center turns by the whole `angle`, the turn
    /// dies away smoothly, and a point at `radius` or beyond does not move. The turn is clockwise on the screen (`y`
    /// down) as for [`Path::rotate`], so a point near the center moves as `rotate(angle)` moves it. Points keep their
    /// distance to `about`.
    ///
    /// `about` defaults to the middle of the bounds; `radius` to the distance from `about` to the farthest corner of the
    /// bounds, so the whole shape twists and its far corner stays (a path with no extent from `about` has radius 1). An
    /// angle of any size and sign is allowed and does not wrap: 360 turns the center once round against the rim.
    ///
    /// The path is flattened and each edge cut into equal pieces of at most `radius / n`, `n` growing with the angle
    /// (`7.5 sqrt(a^2 + 14 a / 3)` for `a` radians), so that every point of the result is within a hundredth of the
    /// radius of the image it stands for and within 0.05 design pixels of it (a shorter step where a hundredth of the radius
    /// is more; the bound is worked in the code); every vertex of the flattening is kept, so
    /// corners stay corners. The result is made of straight lines; each sub-path keeps its closedness, a sub-path of a
    /// single point stays one (moved), an empty path stays empty.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `radius` if it is not a number above 0 and at most 1e100, or if the pieces the cut
    /// would make, over all the sub-paths, are more than `MAX_SEGMENTS` (10,000,000), counted before any is made;
    /// naming `angle` if it is NaN, infinite or so large that a radius could not be cut into `MAX_SEGMENTS` pieces;
    /// naming `about` if it is NaN, infinite or beyond ±1e100.
    pub fn twist(&self, angle: f64, radius: Option<f64>, about: Option<Point>) -> Result<Self, Error> {
        self.twist_charged(angle, radius, about, &mut segment_budget("radius"))
    }

    /// As [`Path::twist`], calling `charge` with the number of points the result would have, before any is made; its
    /// error stops the verb.
    pub(crate) fn twist_charged(
        &self,
        angle: f64,
        radius: Option<f64>,
        about: Option<Point>,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let angle = finite("angle", angle)?;
        let frame = self.frame(radius, about)?;
        let pieces = twist_pieces(angle);
        // a turn so large that not even one radius can be cut is refused here, naming the angle that sets it
        #[expect(clippy::cast_precision_loss, reason = "10 million is exact in f64")]
        if pieces > MAX_SEGMENTS as f64 {
            return Err(Error::parameter(
                "angle",
                format!("a turn of {angle} degrees needs more than MAX_SEGMENTS {MAX_SEGMENTS} pieces to the radius"),
            ));
        }
        let Some((about, radius)) = frame else {
            return Ok(self.clone());
        };
        let map = |point: Point| {
            let (dx, dy) = (point.x - about.x, point.y - about.y);
            let r = libm::hypot(dx, dy);
            if r >= radius {
                return point;
            }
            let (sin, cos) = sin_cos_deg(angle * falloff(r, radius));
            Point::new(about.x + dx * cos - dy * sin, about.y + dx * sin + dy * cos)
        };
        self.warp("angle", radius / pieces, CHORD_ERROR * radius, &map, charge)
    }

    /// This path with every point moved along its ray from `about` to `r (1 + amount * falloff(r))`, `r` its distance
    /// from `about` and `falloff(r) = 1 - s^2 (3 - 2s)`, `s = min(r / radius, 1)`: a positive `amount` bulges the middle
    /// outward (a fish-eye), a negative one pinches it inward, 0 leaves it where it is, and a point at `radius` or
    /// beyond does not move (to the bit). The center stays; every other point keeps its direction from `about`.
    ///
    /// The map of `r` is strictly increasing, so no part of the shape folds over itself, exactly for
    /// `-1 < amount < 16/11`: its slope is `1 + amount (1 - 9 s^2 + 8 s^3)`, and `1 - 9 s^2 + 8 s^3` runs from 1 (at
    /// `s = 0`) down to -11/16 (at `s = 3/4`) and back to 0.
    ///
    /// `about` and `radius` default as for [`Path::twist`]. The path is flattened and each edge cut into equal pieces
    /// of at most `radius / (15 sqrt(|amount|))`, so that every point of the result is within a hundredth of the radius
    /// of the image it stands for and within 0.05 design pixels of it (a shorter step where that is more); every vertex of the flattening is kept, so corners stay corners. The result is made
    /// of straight lines; each sub-path keeps its closedness, a sub-path of a single point stays one, an empty path
    /// stays empty.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `amount` if it is not a number above -1 and below 16/11, or if a displaced point
    /// would be beyond ±1e100; naming `radius` if it is not a number above 0 and at most 1e100, or if the pieces the
    /// cut would make, over all the sub-paths, are more than `MAX_SEGMENTS` (10,000,000), counted before any is made;
    /// naming `about` if it is NaN, infinite or beyond ±1e100.
    pub fn bulge(&self, amount: f64, radius: Option<f64>, about: Option<Point>) -> Result<Self, Error> {
        self.bulge_charged(amount, radius, about, &mut segment_budget("radius"))
    }

    /// As [`Path::bulge`], calling `charge` with the number of points the result would have, before any is made; its
    /// error stops the verb.
    pub(crate) fn bulge_charged(
        &self,
        amount: f64,
        radius: Option<f64>,
        about: Option<Point>,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        if !(amount > -1.0 && amount < STRONGEST_BULGE) {
            return Err(Error::parameter(
                "amount",
                format!("must be {STRONGEST_BULGE_RULE}, got {amount}"),
            ));
        }
        let Some((about, radius)) = self.frame(radius, about)? else {
            return Ok(self.clone());
        };
        let map = |point: Point| {
            let (dx, dy) = (point.x - about.x, point.y - about.y);
            let r = libm::hypot(dx, dy);
            if r >= radius {
                return point;
            }
            let factor = 1.0 + amount * falloff(r, radius);
            Point::new(about.x + dx * factor, about.y + dy * factor)
        };
        self.warp(
            "amount",
            radius / bulge_pieces(amount),
            CHORD_ERROR * radius,
            &map,
            charge,
        )
    }

    /// The center and the radius of a radial deformer: `about` (the middle of the bounds if not given) and `radius`
    /// (the distance from `about` to the farthest corner of the bounds if not given; 1 if that is 0); `None` for an
    /// empty path, which has neither.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `about` or `radius` as `twist` says.
    fn frame(&self, radius: Option<f64>, about: Option<Point>) -> Result<Option<(Point, f64)>, Error> {
        let about = about.map(|point| coordinate_point("about", point)).transpose()?;
        let radius = radius
            .map(|radius| {
                if radius <= 0.0 {
                    return Err(Error::parameter(
                        "radius",
                        format!("must be a number above 0, got {radius}"),
                    ));
                }
                coordinate("radius", radius)
            })
            .transpose()?;
        if self.subpaths.is_empty() {
            return Ok(None);
        }
        let bounds = self.bounds()?;
        let about = about.unwrap_or_else(|| Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0));
        let radius = radius.unwrap_or_else(|| {
            let far = libm::hypot(
                (about.x - bounds.x)
                    .abs()
                    .max((bounds.x + bounds.width - about.x).abs()),
                (about.y - bounds.y)
                    .abs()
                    .max((bounds.y + bounds.height - about.y).abs()),
            );
            if far > 0.0 { far } else { 1.0 }
        });
        Ok(Some((about, radius)))
    }
}
