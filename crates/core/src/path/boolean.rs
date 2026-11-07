//! Booleans on paths: `resolve` (a path's own overlaps removed) and `union`, `subtract`, `intersect` and `exclude`
//! (two paths combined). The only user of `i_overlay`.
//!
//! Every verb is one run of the engine over the flattened outlines of both paths under the nonzero rule, so each
//! result is a resolved region: the outlines of a path that overlap or cross themselves are merged, and an open
//! sub-path fills as if closed. The engine snaps coordinates to an integer grid it picks from the bounding box of both
//! paths (about 2^-60 of its half-extent, below what a number holds), which is why details far smaller than the shapes
//! beside them can vanish. Its work grows with the crossings of the outlines, which no input size bounds: the caller
//! charges the edges before the run and the vertices after it.

use super::{Builder, Path};
use crate::Error;
use crate::geometry::{Point, coordinate};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

type Contour = Vec<[f64; 2]>;

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

    /// As [`Path::resolve`], calling `charge` with the number of edges each curve is flattened into (before the
    /// engine runs) and with the number of vertices of the result (before it is built); its error stops the verb.
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
    if tolerance <= 0.0 {
        return Err(Error::parameter(
            "tolerance",
            format!("must be a number above 0, got {tolerance}"),
        ));
    }
    // a path holds finite coordinates within the limit (and so does every point of a curve), which is what the
    // engine needs; only the squared tolerance has to be a number
    let tolerance = coordinate("tolerance", tolerance)?;
    let subject = contours(subject, tolerance, charge)?;
    let clip = contours(clip, tolerance, charge)?;
    let shapes = subject.overlay_as::<i64>(&clip, rule, FillRule::NonZero);
    charge(shapes.iter().flatten().map(Vec::len).sum())?;
    canonical(shapes)
}

/// The closed polygons `path` fills with: each sub-path flattened (the engine ignores one of fewer than 3 corners, a
/// lone move or a line, which encloses nothing).
fn contours(
    path: &Path,
    tolerance: f64,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Vec<Contour>, Error> {
    Ok(path
        .flattened_within(tolerance, charge)?
        .into_iter()
        .map(|polyline| polyline.points.iter().map(|point| [point.x, point.y]).collect())
        .collect())
}

/// `shapes` (an outer contour and its holes each, as the engine returns them: the outer running clockwise on a `y`-down
/// screen, holes the other way) as a path in canonical form.
fn canonical(mut shapes: Vec<Vec<Contour>>) -> Result<Path, Error> {
    for shape in &mut shapes {
        shape.iter_mut().for_each(start_at_top);
        if let Some((_, holes)) = shape.split_first_mut() {
            holes.sort_by(by_top);
        }
    }
    shapes.retain_mut(|shape| {
        if shape.first().is_none_or(is_flat) {
            return false;
        }
        shape.retain(|contour| !is_flat(contour));
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

/// Whether `contour` (started at its topmost corner), written as numbers, encloses no area: the engine's grid is finer than a number, so a piece of
/// some cells can have its corners in line once they are rounded. It is no contour, and its area is below what a
/// number holds.
fn is_flat(contour: &Contour) -> bool {
    let [ox, oy] = contour.first().copied().unwrap_or_default();
    let doubled: f64 = contour
        .iter()
        .zip(contour.iter().cycle().skip(1))
        .map(|(&[x, y], &[next_x, next_y])| (x - ox) * (next_y - oy) - (next_x - ox) * (y - oy))
        .sum();
    doubled == 0.0
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
    fn the_edges_flattened_and_the_vertices_made_are_charged() -> Result<(), Error> {
        let (a, b) = (square(0.0, 4.0)?, square(2.0, 4.0)?);
        let mut charged = Vec::new();
        a.combined(&b, Combine::Union, 0.01, &mut |count| {
            charged.push(count);
            Ok(())
        })?;
        // four edges for each square (one charge per curve), then the eight corners of the union
        assert_eq!(charged.iter().sum::<usize>(), 4 + 4 + 8, "{charged:?}");
        assert_eq!(charged.last(), Some(&8));
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
        // eight curves are charged, then the result: refusing the first, or the last, stops the verb
        assert_eq!(refuse(0), Err(Error::parameter("work", "over the limit")));
        assert_eq!(refuse(8), Err(Error::parameter("work", "over the limit")));
        assert!(refuse(9).is_ok());
        Ok(())
    }
}
