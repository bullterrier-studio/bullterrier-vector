//! Offsets on paths: `offset` (the region a path fills, grown or shrunk) and `outline` (the region a stroke paints).
//! With `boolean.rs`, the only user of `i_overlay`.
//!
//! `offset` resolves the path first (the region it fills by the nonzero rule, as the booleans do), so the engine sees
//! simple polygons: outer contours clockwise on a `y`-down screen and holes counter-clockwise, which is the direction
//! it takes as filled. `outline` strokes each flattened sub-path on its own (open or closed) and unites the strokes.
//! Both results go through the canonical form of the booleans.
//!
//! The engine rounds to an integer grid it picks from the bounding box of its input and the reach of the offset, so a
//! feature much smaller than the offset or the shape is lost to rounding. Its miter join is cut to a bevel below a
//! turn of 5 degrees' worth of angle in integer maths: a `miter_limit` above about 23 (the ratio of a 5 degree
//! corner) acts as 23. Its arcs are steps of 1.8 to 45 degrees, so a round join or cap of a radius above about
//! 80 times `tolerance` strays by more than it (at most 0.00012 of the radius).

use super::Path;
use super::boolean::{Contour, canonical, checked_tolerance, contours, largest, overlay};
use crate::Error;
use crate::geometry::coordinate;
use crate::math;
use crate::shape::{Cap, Join};
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::mesh::float::outline::offset::OutlineOffset;
use i_overlay::mesh::float::stroke::offset::StrokeOffset;
use i_overlay::mesh::float::style::{LineCap, LineJoin, OutlineStyle, StrokeStyle};

impl Path {
    /// The region this path fills (as in [`Path::resolve`]) moved outward by `distance`, inward when negative: every
    /// point within `distance` of the region is added, every point within `-distance` of its outside removed. Outer
    /// contours grow and holes shrink; a region that shrinks past nothing is an empty path. A `distance` of 0 gives the
    /// resolved path. The result is in canonical form and straight-edged.
    ///
    /// `join` shapes the corners that stick out: `Round` an arc of the radius `distance`, cut into steps that stray from
    /// it by at most `tolerance` (the engine's arc steps run from 1.8 to 45 degrees, so a radius above about 80 times
    /// `tolerance` strays by up to 0.00012 of itself), `Bevel` a flat cut, `Miter` a point, cut to a bevel where the
    /// corner is sharper than `miter_limit`, SVG's ratio of the length of the point to the half-width (the corner angle
    /// `2 asin(1 / miter_limit)`). A `miter_limit` above about 23 acts as 23. Corners that point inward are sharp
    /// whatever the join. Curves are flattened to `tolerance`; an open sub-path fills as if closed.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `distance` is not a number within ±1e100, `miter_limit` is not a number from 1 up,
    /// `tolerance` is not above 0 and within ±1e100, or the result has a coordinate beyond ±1e100.
    pub fn offset(&self, distance: f64, join: Join, miter_limit: f64, tolerance: f64) -> Result<Self, Error> {
        self.offset_charged(distance, join, miter_limit, tolerance, &mut |_| Ok(()))
    }

    /// The region a stroke of `width` along this path paints, as closed outlines: each sub-path is stroked on its own
    /// (a closed one into a ring) and the strokes are united. `join` and `miter_limit` shape the corners as in
    /// [`Path::offset`], with the radius `width / 2`; `cap` finishes the ends of open sub-paths: `Butt` flat at the end,
    /// `Round` an arc of the same radius and steps as a round join, `Square` flat, half a width past the end. A
    /// sub-path of one point (a lone move) paints nothing. The result is in canonical form and straight-edged.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `width` is not a number above 0 and within ±1e100, or as [`Path::offset`] for
    /// `miter_limit`, `tolerance` and the result.
    pub fn outline(&self, width: f64, join: Join, cap: Cap, miter_limit: f64, tolerance: f64) -> Result<Self, Error> {
        self.outline_charged(width, join, cap, miter_limit, tolerance, &mut |_| Ok(()))
    }

    /// As [`Path::offset`], calling `charge` as [`Path::resolved`] does for the region, then with the number of
    /// vertices of the result (before it is built); its error stops the verb.
    pub(crate) fn offset_charged(
        &self,
        distance: f64,
        join: Join,
        miter_limit: f64,
        tolerance: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let tolerance = checked_tolerance(tolerance)?;
        let distance = coordinate("distance", distance)?;
        let miter_limit = checked_miter_limit(miter_limit)?;
        let region = self.resolved(tolerance, charge)?;
        if distance == 0.0 {
            return Ok(region);
        }
        let style = OutlineStyle::new(distance).line_join(line_join(join, miter_limit, distance.abs(), tolerance));
        // the region is lines, so there is nothing to flatten or charge
        let region = contours(&region, tolerance, &mut |_| Ok(()))?;
        let shapes = region.outline_as::<i64>(&style);
        charge(shapes.iter().flatten().map(Vec::len).sum())?;
        canonical(shapes, largest(&region))
    }

    /// As [`Path::outline`], calling `charge` with the number of edges each curve is flattened into, then as
    /// [`Path::resolved`] does for the pairs of edges of the strokes and the vertices of the result.
    pub(crate) fn outline_charged(
        &self,
        width: f64,
        join: Join,
        cap: Cap,
        miter_limit: f64,
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
        let miter_limit = checked_miter_limit(miter_limit)?;
        let radius = width / 2.0;
        let cap = match cap {
            Cap::Butt => LineCap::Butt,
            Cap::Round => LineCap::Round(arc_step(radius, tolerance)),
            Cap::Square => LineCap::Square,
        };
        let style = StrokeStyle::new(width)
            .line_join(line_join(join, miter_limit, radius, tolerance))
            .start_cap(cap.clone())
            .end_cap(cap);
        let mut strokes: Vec<Contour> = Vec::new();
        for polyline in self.flattened_within(tolerance, charge)? {
            if polyline.points.len() > 1 {
                let points: Contour = polyline.points.iter().map(|p| [p.x, p.y]).collect();
                strokes.extend(
                    points
                        .stroke_as::<i64>(style.clone(), polyline.closed)
                        .into_iter()
                        .flatten(),
                );
            }
        }
        overlay(&strokes, &[], OverlayRule::Subject, charge)
    }
}

/// `miter_limit`, or the error naming it unless it is a number from 1 up.
fn checked_miter_limit(miter_limit: f64) -> Result<f64, Error> {
    if miter_limit >= 1.0 && miter_limit.is_finite() {
        Ok(miter_limit)
    } else {
        Err(Error::parameter(
            "miter_limit",
            format!("must be a number from 1 up, got {miter_limit}"),
        ))
    }
}

/// The engine's join for `join` round a corner at distance `radius`: a miter is cut to a bevel below the corner angle
/// `2 asin(1 / miter_limit)`, an arc is cut into steps no wider than [`arc_step`].
fn line_join(join: Join, miter_limit: f64, radius: f64, tolerance: f64) -> LineJoin<f64> {
    match join {
        Join::Round => LineJoin::Round(arc_step(radius, tolerance)),
        Join::Miter => LineJoin::Miter(2.0 * math::asin(1.0 / miter_limit)),
        Join::Bevel => LineJoin::Bevel,
    }
}

/// The widest step (radians) an arc of `radius` may be cut into so that its chords stray from it by at most
/// `tolerance`: a chord of a step `s` is `radius * (1 - cos(s / 2))` from its arc at the middle. A `tolerance` of the
/// radius or more is a half turn, which the engine cuts to its widest step.
fn arc_step(radius: f64, tolerance: f64) -> f64 {
    2.0 * math::acos(1.0 - (tolerance / radius).min(1.0))
}

#[cfg(test)]
mod tests {
    //! The charges only the node of a drawing sees, so no public route reaches them.
    use super::*;

    fn charges(
        run: impl FnOnce(&mut dyn FnMut(usize) -> Result<(), Error>) -> Result<Path, Error>,
    ) -> Result<Vec<usize>, Error> {
        let mut charged = Vec::new();
        run(&mut |count| {
            charged.push(count);
            Ok(())
        })?;
        Ok(charged)
    }

    #[test]
    fn the_edges_the_pairs_of_edges_and_the_vertices_made_are_charged() -> Result<(), Error> {
        let square = Path::rect(0.0, 0.0, 4.0, 4.0, 0.0)?;
        // a square: four edges (one charge per curve), the 4 pairs of edges with boxes that overlap (its corners) and its
        // 4 corners as the region; then the 4 corners of the grown square
        let grown = charges(|charge| square.offset_charged(1.0, Join::Miter, 4.0, 0.01, charge))?;
        assert_eq!(grown, [1, 1, 1, 1, 4, 4, 4]);
        // a line is one edge, the stroke around it 4 pairs of edges and 4 corners
        let line = Path::line((0.0, 0.0), (4.0, 0.0))?;
        let stroked = charges(|charge| line.outline_charged(2.0, Join::Miter, Cap::Butt, 4.0, 0.01, charge))?;
        assert_eq!(stroked, [1, 4, 4]);
        Ok(())
    }

    /// A charge that accepts `after` calls and refuses the next.
    fn refusing(after: usize) -> impl FnMut(usize) -> Result<(), Error> {
        let mut calls = 0;
        move |_| {
            calls += 1;
            if calls > after {
                Err(Error::parameter("work", "over the limit"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn a_refused_charge_stops_either_verb_at_any_charge() -> Result<(), Error> {
        let square = Path::rect(0.0, 0.0, 4.0, 4.0, 0.0)?;
        let line = Path::line((0.0, 0.0), (4.0, 0.0))?;
        let refused = Err(Error::parameter("work", "over the limit"));
        // seven charges for the offset, three for the outline: refusing any of them stops the verb, refusing none does not
        for after in 0..7 {
            assert_eq!(
                square.offset_charged(1.0, Join::Miter, 4.0, 0.01, &mut refusing(after)),
                refused,
                "offset after {after}"
            );
        }
        assert!(
            square
                .offset_charged(1.0, Join::Miter, 4.0, 0.01, &mut refusing(7))
                .is_ok()
        );
        for after in 0..3 {
            assert_eq!(
                line.outline_charged(2.0, Join::Miter, Cap::Butt, 4.0, 0.01, &mut refusing(after)),
                refused,
                "outline after {after}"
            );
        }
        assert!(
            line.outline_charged(2.0, Join::Miter, Cap::Butt, 4.0, 0.01, &mut refusing(3))
                .is_ok()
        );
        Ok(())
    }
}
