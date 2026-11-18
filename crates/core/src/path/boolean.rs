//! Booleans on paths: `resolve` (a path's own overlaps removed) and `union`, `subtract`, `intersect` and `exclude`
//! (two paths combined). With `offset.rs`, the only user of `i_overlay`.
//!
//! Every verb is one run of the engine over the flattened outlines of both paths under the nonzero rule, so each
//! result is a resolved region: the outlines of a path that overlap or cross themselves are merged, and an open
//! sub-path fills as if closed. The engine snaps coordinates to an integer grid it picks from the bounding box of both
//! paths (about 2^-60 of its half-extent, below what a number holds), which is why details far smaller than the shapes
//! beside them can vanish. Its work grows with the crossings of the outlines, which no input size bounds: the caller
//! charges the edges before the run, the pairs of edges that may cross (`box_pairs`) just before it, and the vertices
//! after it.

use super::{Builder, Path};
use crate::Error;
use crate::geometry::{Point, coordinate};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

pub(super) type Contour = Vec<[f64; 2]>;

/// How two paths combine.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Combine {
    Union,
    Subtract,
    Intersect,
    Exclude,
}

impl Path {
    /// This path with its overlaps removed, as a set of closed polygons: the region it fills by the nonzero rule, a
    /// self-crossing outline cut into the pieces it encloses and a contour inside another that runs the same way
    /// absorbed. The result is in canonical form: outer contours run clockwise and holes counter-clockwise, each
    /// contour starts at its topmost point (the leftmost of a tie), outer contours come in order of their topmost
    /// point (top to bottom, then left to right), each followed by its holes in the same order. Curves become straight
    /// edges that stray from them by at most `tolerance` design pixels; an open sub-path fills as if closed.
    ///
    /// Coordinates are snapped to a grid finer than a number resolves at the size of the path, so a feature much
    /// smaller than the path is its neighbour's rounding; the work grows with the number of places the outlines cross.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `tolerance` is not above 0 and within ±1e100, or the result has a coordinate beyond
    /// ±1e100.
    pub fn resolve(&self, tolerance: f64) -> Result<Self, Error> {
        self.resolved(tolerance, &mut |_| Ok(()))
    }

    /// The region either path fills, as in [`Path::resolve`]: both paths together.
    ///
    /// # Errors
    /// As [`Path::resolve`].
    pub fn union(&self, other: &Self, tolerance: f64) -> Result<Self, Error> {
        self.combined(other, Combine::Union, tolerance, &mut |_| Ok(()))
    }

    /// The region this path fills and `other` does not.
    ///
    /// # Errors
    /// As [`Path::resolve`].
    pub fn subtract(&self, other: &Self, tolerance: f64) -> Result<Self, Error> {
        self.combined(other, Combine::Subtract, tolerance, &mut |_| Ok(()))
    }

    /// The region both paths fill. An empty result is an empty path.
    ///
    /// # Errors
    /// As [`Path::resolve`].
    pub fn intersect(&self, other: &Self, tolerance: f64) -> Result<Self, Error> {
        self.combined(other, Combine::Intersect, tolerance, &mut |_| Ok(()))
    }

    /// The region one of the paths fills and the other does not (the symmetric difference).
    ///
    /// # Errors
    /// As [`Path::resolve`].
    pub fn exclude(&self, other: &Self, tolerance: f64) -> Result<Self, Error> {
        self.combined(other, Combine::Exclude, tolerance, &mut |_| Ok(()))
    }

    /// As [`Path::resolve`], calling `charge` with the number of edges each curve is flattened into, then with the
    /// number of pairs of edges whose boxes overlap (both before the engine runs), and with the number of vertices of
    /// the result (before it is built); its error stops the verb.
    pub(crate) fn resolved(
        &self,
        tolerance: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        run(self, &Self::new(), OverlayRule::Subject, tolerance, charge)
    }

    /// As [`Path::union`] and the others, charging as [`Path::resolved`].
    pub(crate) fn combined(
        &self,
        other: &Self,
        how: Combine,
        tolerance: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let rule = match how {
            Combine::Union => OverlayRule::Union,
            Combine::Subtract => OverlayRule::Difference,
            Combine::Intersect => OverlayRule::Intersect,
            Combine::Exclude => OverlayRule::Xor,
        };
        run(self, other, rule, tolerance, charge)
    }
}

fn run(
    subject: &Path,
    clip: &Path,
    rule: OverlayRule,
    tolerance: f64,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Path, Error> {
    let tolerance = checked_tolerance(tolerance)?;
    let subject = contours(subject, tolerance, charge)?;
    let clip = contours(clip, tolerance, charge)?;
    overlay(&subject, &clip, rule, charge)
}

/// `tolerance`, or the error naming it unless it is a number above 0 and within the limit of a coordinate (so that its
/// square is a number, which flattening needs; a path holds finite coordinates within the limit, and so does every point
/// of a curve, which is what the engine needs).
pub(super) fn checked_tolerance(tolerance: f64) -> Result<f64, Error> {
    if tolerance <= 0.0 {
        return Err(Error::parameter(
            "tolerance",
            format!("must be a number above 0, got {tolerance}"),
        ));
    }
    coordinate("tolerance", tolerance)
}

/// The region `subject` and `clip` make under `rule` (both by the nonzero rule), charging the pairs of edges that may
/// cross before the engine runs and the vertices of the result after.
pub(super) fn overlay(
    subject: &[Contour],
    clip: &[Contour],
    rule: OverlayRule,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Path, Error> {
    charge(box_pairs(subject, clip))?;
    let shapes = subject.overlay_as::<i64>(&clip, rule, FillRule::NonZero);
    charge(shapes.iter().flatten().map(Vec::len).sum())?;
    canonical(shapes, largest(subject).max(largest(clip)))
}

/// The largest coordinate of `contours`: the size the engine's frame, and so the rounding of its result, follows.
pub(super) fn largest(contours: &[Contour]) -> f64 {
    contours
        .iter()
        .flatten()
        .flatten()
        .fold(0.0_f64, |most, c| most.max(c.abs()))
}

/// The closed polygons `path` fills with: each sub-path flattened. One of fewer than 3 corners (a lone move or a
/// line) encloses nothing and is left out: the engine would ignore it, but it would still widen the frame it rounds
/// to, and so move the corners of everything else.
pub(super) fn contours(
    path: &Path,
    tolerance: f64,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Vec<Contour>, Error> {
    Ok(path
        .flattened_within(tolerance, charge)?
        .into_iter()
        .filter(|polyline| polyline.points.len() >= 3)
        .map(|polyline| polyline.points.iter().map(|point| [point.x, point.y]).collect())
        .collect())
}

/// How many pairs of edges of `subject` and `clip` have boxes that overlap: at least as many as the places the outlines
/// cross, which the engine's time and memory grow with (about 140 bytes a pair for a scribble; the edges alone say
/// nothing: 16,000 of them cross 30 million times). Counted in `O(n log n)` by a sweep from left to right over the
/// boxes, with a count of the active boxes by top and by bottom.
fn box_pairs(subject: &[Contour], clip: &[Contour]) -> usize {
    // (left, right, top, bottom) of every edge
    let boxes: Vec<[f64; 4]> = subject
        .iter()
        .chain(clip)
        .flat_map(|contour| {
            contour
                .iter()
                .zip(contour.iter().cycle().skip(1))
                .map(|(a, b)| [a[0].min(b[0]), a[0].max(b[0]), a[1].min(b[1]), a[1].max(b[1])])
        })
        .collect();
    let mut heights: Vec<f64> = boxes.iter().flat_map(|b| [b[2], b[3]]).collect();
    heights.sort_by(f64::total_cmp);
    heights.dedup();
    let rank = |y: f64| heights.partition_point(|h| h.total_cmp(&y).is_lt());
    // a box starts at its left and ends at its right; at a tie the starts come first, as boxes that touch overlap
    let mut events: Vec<(f64, bool, usize)> = Vec::with_capacity(2 * boxes.len());
    for (index, b) in boxes.iter().enumerate() {
        events.push((b[0], false, index));
        events.push((b[1], true, index));
    }
    events.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let (mut tops, mut bottoms) = (Counts::new(heights.len()), Counts::new(heights.len()));
    let (mut active, mut pairs) = (0_isize, 0_usize);
    for (_, ending, index) in events {
        let (top, bottom) = (rank(boxes[index][2]), rank(boxes[index][3]));
        if ending {
            tops.add(top, -1);
            bottoms.add(bottom, -1);
            active -= 1;
        } else {
            // an active box misses this one if it ends above its top or starts below its bottom, never both
            let apart = bottoms.below(top) + (active - tops.below(bottom + 1));
            pairs = pairs.saturating_add((active - apart).unsigned_abs());
            tops.add(top, 1);
            bottoms.add(bottom, 1);
            active += 1;
        }
    }
    pairs
}

/// Counts at positions `0..n` that can change by one and be summed over a prefix, in `O(log n)` (a Fenwick tree).
struct Counts(Vec<isize>);

impl Counts {
    fn new(positions: usize) -> Self {
        Self(vec![0; positions + 1])
    }

    fn add(&mut self, position: usize, change: isize) {
        let mut at = position + 1;
        while at < self.0.len() {
            self.0[at] += change;
            at += at.isolate_lowest_one();
        }
    }

    /// The sum of the counts at positions before `position`.
    fn below(&self, position: usize) -> isize {
        let (mut at, mut sum) = (position, 0);
        while at > 0 {
            sum += self.0[at];
            at -= at.isolate_lowest_one();
        }
        sum
    }
}

/// `shapes` (an outer contour and its holes each, as the engine returns them: the outer running clockwise on a `y`-down
/// screen, holes the other way) as a path in canonical form. `input` is the largest coordinate that went into the engine:
/// its frame is as wide as all of its input, so a result that is only a small piece of it is rounded as coarsely as
/// the input is large.
pub(super) fn canonical(mut shapes: Vec<Vec<Contour>>, input: f64) -> Result<Path, Error> {
    for shape in &mut shapes {
        shape.iter_mut().for_each(start_at_top);
        if let Some((_, holes)) = shape.split_first_mut() {
            holes.sort_by(by_top);
        }
    }
    let scale = shapes
        .iter()
        .flatten()
        .flatten()
        .flatten()
        .fold(input, |most, c| most.max(c.abs()));
    shapes.retain_mut(|shape| {
        if shape.first().is_none_or(|outer| is_flat(outer, scale)) {
            return false;
        }
        shape.retain(|contour| !is_flat(contour, scale));
        true
    });
    shapes.sort_by(|a, b| match (a.first(), b.first()) {
        (Some(a), Some(b)) => by_top(a, b),
        _ => std::cmp::Ordering::Equal,
    });
    let mut builder = Builder::new();
    for contour in shapes.iter().flatten() {
        let mut points = contour.iter().map(|&[x, y]| Point::new(x, y));
        if let Some(first) = points.next() {
            builder.move_to(first)?;
            for point in points {
                builder.line_to(point)?;
            }
            builder.close();
        }
    }
    Ok(builder.finish())
}

/// Whether `contour` is no thicker than a few units in the last place of `scale`, the largest coordinate of the input
/// or the result (twice its area over its perimeter, about its thickness when it is thin). The engine's grid is finer than a number,
/// and it works in a frame as wide as all of its input, so a hair-thin piece of an outline that hardly leaves a line
/// (or a spike beside a far larger shape) comes back with its corners rounded to numbers: in line, or so far out of
/// it that the side it runs round has changed. Such a piece is no contour, and its area is below what a number holds.
fn is_flat(contour: &Contour, scale: f64) -> bool {
    // constant: a corner is rounded to half a unit in the last place of its coordinate in each direction, so a piece
    // up to about 1.5 of them thick can end up flat or flipped; 4 leaves room for the corners of both sides
    const FLAT_ULPS: f64 = 4.0;
    if scale == 0.0 {
        return true;
    }
    let [ox, oy] = contour.first().copied().unwrap_or_default();
    // in units of the largest coordinate, so that the products below are numbers whatever the size of the contour
    let unit = |&[x, y]: &[f64; 2]| [(x - ox) / scale, (y - oy) / scale];
    let (mut doubled, mut perimeter) = (0.0_f64, 0.0_f64);
    for (from, to) in contour.iter().map(unit).zip(contour.iter().cycle().skip(1).map(unit)) {
        doubled += from[0] * to[1] - to[0] * from[1];
        perimeter += ((to[0] - from[0]) * (to[0] - from[0]) + (to[1] - from[1]) * (to[1] - from[1])).sqrt();
    }
    doubled.abs() <= FLAT_ULPS * f64::EPSILON * perimeter
}

/// The order of two contours, each starting at its topmost corner: top to bottom, then left to right.
fn by_top(a: &Contour, b: &Contour) -> std::cmp::Ordering {
    match (a.first(), b.first()) {
        (Some(a), Some(b)) => a[1].total_cmp(&b[1]).then(a[0].total_cmp(&b[0])),
        _ => std::cmp::Ordering::Equal,
    }
}

/// `contour` turned to start at its topmost corner, the leftmost of those.
fn start_at_top(contour: &mut Contour) {
    let top = contour
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a[1].total_cmp(&b[1]).then(a[0].total_cmp(&b[0])))
        .map_or(0, |(index, _)| index);
    contour.rotate_left(top);
}

#[cfg(test)]
mod tests {
    //! The charges only the node of a drawing sees, so no public route reaches them.
    use super::*;

    fn square(x: f64, size: f64) -> Result<Path, Error> {
        Path::rect(x, x, size, size, 0.0)
    }

    #[test]
    fn the_edges_flattened_the_pairs_of_edges_and_the_vertices_made_are_charged() -> Result<(), Error> {
        let (a, b) = (square(0.0, 4.0)?, square(2.0, 4.0)?);
        let mut charged = Vec::new();
        a.combined(&b, Combine::Union, 0.01, &mut |count| {
            charged.push(count);
            Ok(())
        })?;
        // four edges for each square (one charge per curve), the 10 pairs of edges with boxes that overlap (the corners
        // of each square: 4 and 4; the right edge of the first meets the top of the second, the bottom of the first
        // the left of the second), then the eight corners of the union
        assert_eq!(charged, [vec![1; 8], vec![10, 8]].concat());
        Ok(())
    }

    #[test]
    fn a_refused_charge_stops_the_verb_before_and_after_the_engine_runs() -> Result<(), Error> {
        let (a, b) = (square(0.0, 4.0)?, square(2.0, 4.0)?);
        let refuse = |after: usize| {
            let mut calls = 0;
            a.combined(&b, Combine::Union, 0.01, &mut |_| {
                calls += 1;
                if calls > after {
                    Err(Error::parameter("work", "over the limit"))
                } else {
                    Ok(())
                }
            })
        };
        // eight curves are charged, then the pairs, then the result: refusing the first, or any later one, stops the verb
        for after in [0, 8, 9] {
            assert_eq!(
                refuse(after),
                Err(Error::parameter("work", "over the limit")),
                "after {after}"
            );
        }
        assert!(refuse(10).is_ok());
        Ok(())
    }

    #[test]
    fn the_pairs_of_overlapping_boxes_are_what_a_comparison_of_every_pair_counts() {
        // 300 points of a 1000 square by an xorshift: edges of every length, some boxes that only touch (a grid)
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = |grid: f64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            #[expect(clippy::cast_precision_loss, reason = "the top 50 bits")]
            let share = (state >> 14) as f64 / (1_u64 << 50) as f64;
            (share * grid).floor() * 1000.0 / grid
        };
        for grid in [1000.0, 8.0] {
            let subject: Vec<Contour> = vec![(0..300).map(|_| [next(grid), next(grid)]).collect()];
            let clip: Vec<Contour> = vec![(0..40).map(|_| [next(grid), next(grid)]).collect()];
            let boxes: Vec<[f64; 4]> = subject
                .iter()
                .chain(&clip)
                .flat_map(|c| {
                    (0..c.len()).map(|i| {
                        let (a, b) = (c[i], c[(i + 1) % c.len()]);
                        [a[0].min(b[0]), a[0].max(b[0]), a[1].min(b[1]), a[1].max(b[1])]
                    })
                })
                .collect();
            let mut wanted = 0;
            for (i, p) in boxes.iter().enumerate() {
                for q in &boxes[i + 1..] {
                    wanted += usize::from(p[0] <= q[1] && q[0] <= p[1] && p[2] <= q[3] && q[2] <= p[3]);
                }
            }
            assert_eq!(box_pairs(&subject, &clip), wanted, "grid {grid}");
        }
    }
}
