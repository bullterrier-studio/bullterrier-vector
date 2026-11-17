//! What a traced layer goes through after the lines are found: grow then shrink (close), shrink then grow (open),
//! fill the small holes and drop the small parts, then fit curves through the edges.

use super::Plan;
use crate::Error;
use crate::Path;
use crate::shape::Join;

/// How closely an offset's arcs follow the true circle.
// constant: a hundredth of a design pixel is below anything that shows, and is `offset`'s own default
const OFFSET_TOLERANCE: f64 = 0.01;

/// The corner a grown or shrunk edge gets.
// constant: round, so open and close behave as a ball rolled along the edge; the miter limit is unused by it
const MITER: f64 = 4.0;

/// The signed area of a closed ring of points, positive when it runs clockwise on the screen (y down): the shoelace sum.
fn signed_area(points: &[crate::Point]) -> f64 {
    let mut doubled = 0.0;
    for (index, from) in points.iter().enumerate() {
        let to = points[(index + 1) % points.len()];
        doubled += from.x * to.y - to.x * from.y;
    }
    doubled / 2.0
}

/// `path` with the holes under `min_hole` filled and the parts under `min_area` dropped, by area. The path is in canonical
/// form, so an outline is a ring that runs clockwise and a hole one that runs the other way.
fn without_small(path: &Path, plan: &Plan, charge: &mut dyn FnMut(usize) -> Result<(), Error>) -> Result<Path, Error> {
    let (mut small_parts, mut small_holes) = (Vec::new(), Vec::new());
    for ring in path.flattened(charge)? {
        if !ring.closed || ring.points.len() < 3 {
            continue;
        }
        let area = signed_area(&ring.points);
        if area >= 0.0 && area < plan.min_area {
            small_parts.push(Path::polyline(ring.points, true)?);
        } else if area < 0.0 && -area < plan.min_hole {
            small_holes.push(Path::polyline(ring.points, true)?);
        }
    }
    let mut kept = path.clone();
    if !small_holes.is_empty() {
        kept = kept.union(&Path::joined(small_holes.iter()), OFFSET_TOLERANCE)?;
    }
    if !small_parts.is_empty() {
        kept = kept.subtract(&Path::joined(small_parts.iter()), OFFSET_TOLERANCE)?;
    }
    Ok(kept)
}

/// The traced lines of one layer, tidied by `plan`.
pub(super) fn tidy(
    lines: &Path,
    plan: &Plan,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Path, Error> {
    let mut path = lines.clone();
    let filtering = plan.min_area > 0.0 || plan.min_hole > 0.0;
    if filtering && plan.open == 0.0 && plan.close == 0.0 {
        path = path.resolved(OFFSET_TOLERANCE, charge)?;
    }
    if plan.close > 0.0 {
        path = path
            .offset_charged(plan.close, Join::Round, MITER, OFFSET_TOLERANCE, charge)?
            .offset_charged(-plan.close, Join::Round, MITER, OFFSET_TOLERANCE, charge)?;
    }
    if plan.open > 0.0 {
        path = path
            .offset_charged(-plan.open, Join::Round, MITER, OFFSET_TOLERANCE, charge)?
            .offset_charged(plan.open, Join::Round, MITER, OFFSET_TOLERANCE, charge)?;
    }
    if filtering {
        path = without_small(&path, plan, charge)?;
    }
    if plan.tolerance > 0.0 {
        path = path.simplify_charged(plan.tolerance, charge)?;
    }
    Ok(path)
}
