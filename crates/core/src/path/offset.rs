//! Offsets on paths: `offset` (the region a path fills, grown or shrunk) and `outline` (the region a stroke paints).
//! With `boolean.rs`, the only user of `i_overlay`.
//!
//! Both verbs drop the corners of the flattened path that half of `tolerance` hides (`thinned`, one pass) before the
//! engine sees it. The engine joins every vertex of a flattened curve with an offset pair of its own, so without this a
//! smooth outline came back with about two corners for each it had, and repeated offsets doubled at every call; dropping
//! them before the engine, not after, leaves any crossing it could make to the engine, which resolves it.
//!
//! `offset` then resolves the path (the region it fills by the nonzero rule, as the booleans do), so the engine sees
//! simple polygons: outer contours clockwise on a `y`-down screen and holes counter-clockwise, which is the direction
//! it takes as filled. `outline` strokes each flattened sub-path on its own (open or closed) and unites the strokes.
//! Both results go through the canonical form of the booleans.
//!
//! The engine rounds to an integer grid it picks from the bounding box of its input and the reach of the offset, so a
//! feature much smaller than the offset or the shape is lost to rounding. Its miter join is cut to a bevel below a
//! turn of 5 degrees' worth of angle in integer maths: a `miter_limit` above about 23 (the ratio of a 5 degree
//! corner) acts as 23. Its arcs are steps of 1.8 to 45 degrees; a step of 1.8 degrees strays from its arc by
//! `1 - cos(0.9 degrees)` = 0.000123 of the radius, so a round join or cap of a radius above about 4,000 times
//! `tolerance` (the arcs' half of it over 0.000123) strays by more than its share, by up to 0.000123 of the radius.

use super::Path;
use super::boolean::{Contour, canonical, checked_tolerance, contours, largest, overlay};
use super::taper::require_round_stroke;
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
    /// Curves are flattened to `tolerance`, then corners are dropped, each only where it lies within half of `tolerance`
    /// of the edge between the corners kept on either side, so offsetting a result again does not double its corners;
    /// the result is the offset of what is left. With round joins it lies between the offsets of the flattened path by
    /// `distance` less and more `tolerance`. With miter and bevel joins a dropped corner throws no point of its own, and
    /// the joins beside it follow the edge that replaced it.
    ///
    /// A miter at a hidden corner is lost with it: a corner that a line between its kept neighbours hides within half
    /// of `tolerance` throws no miter point, so with miter joins the result can differ from the exact offset of the
    /// unflattened polygon by up to `miter_limit` times `distance` there (a square of 10 with a spike 0.0049 high on an
    /// edge, its apex just wider than the miter cut-off, at `tolerance` 0.01: the lowest y is -10.00, where the exact
    /// offset reaches -39.34 for limit 4 and distance 10, -209.05 for limit 23 and distance 10, -38.34 for limit 10 and
    /// distance 4). Polygons and polylines lose their tiny real corners the same way; bevel joins are not affected. An
    /// input whose every tiny corner must keep its miter is expressed with a smaller `tolerance`.
    ///
    /// `join` shapes the corners that stick out: `Round` an arc of the radius `distance`, cut into steps that stray from
    /// it by at most half of `tolerance` (the engine's arc steps run from 1.8 to 45 degrees, so a radius above about
    /// 4,000 times `tolerance` strays by up to 0.000123 of itself), `Bevel` a flat cut, `Miter` a point, cut to a bevel
    /// where the corner is sharper than `miter_limit`, SVG's ratio of the length of the point to the half-width (the
    /// corner angle `2 asin(1 / miter_limit)`). A `miter_limit` above about 23 acts as 23. Corners that point inward
    /// are sharp whatever the join. An open sub-path fills as if closed.
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
    /// sub-path of one point (a lone move) paints nothing. Corners are dropped as in [`Path::offset`], except the first
    /// two and last two of an open sub-path, which set the direction of its caps. The result is in canonical form and
    /// straight-edged.
    ///
    /// A miter at a hidden corner is lost with it: as for [`Path::offset`], with miter joins the result can differ from
    /// the exact outline of the unflattened path by up to `miter_limit` times `width / 2` at a corner that half of
    /// `tolerance` hides (numbers there). An input whose every tiny corner must keep its miter is expressed with a smaller
    /// `tolerance`.
    ///
    /// With a `profile` (`None` for none) the width is `width` times the factor of `profile` at each place.
    /// `profile` is a list of `(position, factor)`: the position runs from 0 at the start of each sub-path to 1 at its
    /// end, measured by arc length along that sub-path on its own (a closed one from its topmost point, as
    /// [`Path::slice`] and [`Path::dash`] walk it), so every open sub-path tapers at its own ends, each piece of a
    /// dashed line included. Factors are linear between stops and hold the first and last value outside them. Two
    /// stops at one position make a step. A closed sub-path has no free ends: the profile runs once round it, and
    /// where the end meets the start the width steps from the last factor to the first. The stroke is then the sweep
    /// of a round disc of radius `width * factor / 2`, so `join` and `cap` must be round, `miter_limit` is unused, and
    /// a factor of 0 at a free end closes the outline to a point. The curve is measured as [`Path::length`] measures
    /// it (flattened to 0.01 design pixels); `tolerance` is how far the round parts stray from true circles; a
    /// sub-path of no length paints nothing.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `width` is not a number above 0 and within ±1e100, or as [`Path::offset`] for
    /// `miter_limit`, `tolerance` and the result. With a `profile`: naming `join` or `cap` if it is not round;
    /// naming `profile` if it is empty, has more than 1,000 stops, a position outside 0 to 1 or smaller than the one
    /// before, or a factor not a number from 0 to 1e100; naming `width` if a radius is not within 1e100 or the
    /// corners of the discs, over all the pieces, are more than `MAX_SEGMENTS` (10,000,000), which is checked before
    /// any is made.
    pub fn outline(
        &self,
        width: f64,
        join: Join,
        cap: Cap,
        miter_limit: f64,
        tolerance: f64,
        profile: Option<&[(f64, f64)]>,
    ) -> Result<Self, Error> {
        match profile {
            Some(profile) => {
                require_round_stroke(join, cap)?;
                self.tapered_outline(width, profile, tolerance)
            }
            None => self.outline_charged(width, join, cap, miter_limit, tolerance, &mut |_| Ok(())),
        }
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
        if distance == 0.0 {
            return self.resolved(tolerance, charge);
        }
        let spend = Spend::of(tolerance);
        let flat: Vec<Contour> = contours(self, tolerance, charge)?
            .iter()
            .map(|contour| thinned(contour, true, spend.corners))
            .collect();
        let region = overlay(&flat, &[], OverlayRule::Subject, charge)?;
        let style = OutlineStyle::new(distance).line_join(line_join(join, miter_limit, distance.abs(), spend.arcs));
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
        let spend = Spend::of(tolerance);
        let cap = match cap {
            Cap::Butt => LineCap::Butt,
            Cap::Round => LineCap::Round(arc_step(radius, spend.arcs)),
            Cap::Square => LineCap::Square,
        };
        let style = StrokeStyle::new(width)
            .line_join(line_join(join, miter_limit, radius, spend.arcs))
            .start_cap(cap.clone())
            .end_cap(cap);
        let mut strokes: Vec<Contour> = Vec::new();
        for polyline in self.flattened_within(tolerance, charge)? {
            if polyline.points.len() > 1 {
                let points: Contour = polyline.points.iter().map(|p| [p.x, p.y]).collect();
                strokes.extend(
                    thinned(&points, polyline.closed, spend.corners)
                        .stroke_as::<i64>(style.clone(), polyline.closed)
                        .into_iter()
                        .flatten(),
                );
            }
        }
        overlay(&strokes, &[], OverlayRule::Subject, charge)
    }
}

/// How a verb spends `tolerance` beyond flattening: on dropping the corners of the flattened input it cannot see, and on
/// cutting arcs into chords. Dropping corners moves the outline by at most its share, which moves a point's distance to
/// it by no more, and the chords cut inside the arcs by at most theirs, so a round-joined result lies between the
/// exact offsets of the flattened path by `distance` less and more `tolerance`.
struct Spend {
    corners: f64,
    arcs: f64,
}

impl Spend {
    fn of(tolerance: f64) -> Self {
        Self {
            corners: tolerance / 2.0,
            arcs: tolerance / 2.0,
        }
    }
}

/// `points` (a closed contour when `closed`, its last corner joined to its first) without the corners that lie within
/// `within` of the edge between the corners kept on either side (a greedy pass: it drops no corner farther than that,
/// but may keep some it could drop). An open line keeps its first two and last two corners (they set the direction of
/// its caps); a closed contour that would keep fewer than 3 corners (a sliver no wider than twice `within`) stays as it
/// is, so it still offsets and strokes.
///
/// One pass, each corner looked at no more than twice, and once more round the start of a closed contour; the corners
/// are those flattening charged, so nothing more is charged. From a kept corner `a`, the edge reaches forward while the
/// next corner `q` lies in the cone of directions every corner passed allows and is no nearer `a` than any of them. A
/// corner `p` at `r = |p - a|` above `within` allows the directions within `asin(within / r)` of its own: `p` is then
/// within `r sin(asin(within / r)) = within` of the line from `a` towards `q`, on the side of `a` that `q` is (the
/// angle is under 90 degrees), and no farther along than `|q - a|`, which is at least `r`; so `p` is within `within` of
/// the edge `a`-`q` itself. A corner within `within` of `a` is within it of the edge already.
fn thinned(points: &[[f64; 2]], closed: bool, within: f64) -> Contour {
    let count = points.len();
    let at = |k: usize| points[k % count];
    let fixed = |k: usize| !closed && (k == 1 || k + 2 == count);
    // the farthest corner up to `stop` that the edge from `anchor` may reach
    let reach = |anchor: usize, stop: usize| {
        let a = at(anchor);
        let mut end = anchor + 1;
        // the directions every corner passed allows, from `low` round to `high` (counter-clockwise, under a half turn)
        let mut cone: Option<([f64; 2], [f64; 2])> = None;
        let mut farthest = 0.0_f64; // the largest squared distance from `a` of a corner passed
        while end < stop && !fixed(end) {
            let p = sub(at(end), a);
            let r = dot(p, p);
            if r > within * within {
                let along = (r - within * within).sqrt();
                let low = [p[0] * along + p[1] * within, p[1] * along - p[0] * within];
                let high = [p[0] * along - p[1] * within, p[1] * along + p[0] * within];
                cone = Some(match cone {
                    None => (low, high),
                    Some((was_low, was_high)) => (
                        if cross(was_low, low) > 0.0 { low } else { was_low },
                        if cross(high, was_high) > 0.0 { high } else { was_high },
                    ),
                });
            }
            farthest = farthest.max(r);
            let q = sub(at(end + 1), a);
            let inside = cone.is_none_or(|(low, high)| cross(low, q) >= 0.0 && cross(q, high) >= 0.0);
            if !inside || dot(q, q) < farthest {
                break;
            }
            end += 1;
        }
        end
    };
    let last = if closed { count } else { count - 1 };
    let mut kept = vec![0];
    while kept[kept.len() - 1] < last {
        kept.push(reach(kept[kept.len() - 1], last));
    }
    if closed {
        kept.pop(); // the first corner again
        // the first corner was kept only because the pass started there: drop it if the edge round it reaches over it
        if kept.len() > 3 && reach(kept[kept.len() - 1], kept[1] + count) == kept[1] + count {
            kept.remove(0);
        }
        if kept.len() < 3 {
            return points.to_vec();
        }
    }
    kept.into_iter().map(at).collect()
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// Positive when `b` turns counter-clockwise from `a` (in `y`-up terms).
fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

/// The union of closed `polygons` that all turn the same way, in canonical form (the taper's hulls).
pub(super) fn united(polygons: &[Contour], charge: &mut dyn FnMut(usize) -> Result<(), Error>) -> Result<Path, Error> {
    overlay(polygons, &[], OverlayRule::Subject, charge)
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
