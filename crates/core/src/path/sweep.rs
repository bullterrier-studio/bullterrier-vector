//! `Path::sweep`: the region a shape covers as it moves along a straight vector (the Minkowski sum of the region with
//! the segment from the origin to the vector). Built from the boolean engine: the region and its moved copy, united with
//! the parallelogram each edge of the region sweeps.

use super::Path;
use super::boolean::{Contour, checked_tolerance, contours, united};
use crate::Error;
use crate::field::segment_budget;
use crate::geometry::coordinate;

impl Path {
    /// The region the region this path fills (nonzero rule) covers as it moves `dx` to the right and `dy` down in a
    /// straight line: every point that some position of the way passes over. A hole the shape moves across far enough
    /// fills in. The result is in canonical form and straight-edged, as [`Path::resolve`]'s, curves flattened to within
    /// `tolerance` design pixels; with `dx` and `dy` both 0 it is that region.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `dx` or `dy` if it is not a number within ±1e100 or moves a point past it,
    /// `tolerance` if it is not above 0 and within ±1e100, and `dx` past `MAX_SEGMENTS` points (the parallelograms are
    /// charged before any is made, then the pairs of edges that may cross, as for the booleans).
    pub fn sweep(&self, dx: f64, dy: f64, tolerance: f64) -> Result<Self, Error> {
        self.sweep_charged(dx, dy, tolerance, &mut segment_budget("dx"))
    }

    /// As [`Path::sweep`], calling `charge` as [`Path::resolved`] does for the region, then with the corners of the
    /// moved copy and the parallelograms (before any is made), the pairs of edges that may cross and the vertices of
    /// the result; its error stops the verb.
    pub(crate) fn sweep_charged(
        &self,
        dx: f64,
        dy: f64,
        tolerance: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let tolerance = checked_tolerance(tolerance)?;
        let (dx, dy) = (coordinate("dx", dx)?, coordinate("dy", dy)?);
        // the region, so that every point it fills winds once: the copy and the parallelograms add, never cancel
        let region = self.resolved(tolerance, charge)?;
        if dx == 0.0 && dy == 0.0 {
            return Ok(region);
        }
        let mut subject = contours(&region, tolerance, &mut |_| Ok(()))?;
        let edges: usize = subject.iter().map(Vec::len).sum();
        // each edge is a corner of the moved copy and a parallelogram of four
        charge(edges.saturating_mul(5))?;
        let moved = contours(&region.translate(dx, dy)?, tolerance, &mut |_| Ok(()))?;
        let mut swept: Vec<Contour> = Vec::with_capacity(edges);
        for contour in &subject {
            for (at, &[ax, ay]) in contour.iter().enumerate() {
                let [bx, by] = contour[(at + 1) % contour.len()];
                let cross = (bx - ax) * dy - (by - ay) * dx;
                // an edge along the vector sweeps no area; the others are wound one way, so that they add
                if cross > 0.0 {
                    swept.push(vec![[ax, ay], [bx, by], [bx + dx, by + dy], [ax + dx, ay + dy]]);
                } else if cross < 0.0 {
                    swept.push(vec![[ax, ay], [ax + dx, ay + dy], [bx + dx, by + dy], [bx, by]]);
                }
            }
        }
        subject.extend(moved);
        united(&subject, &swept, charge)
    }
}
