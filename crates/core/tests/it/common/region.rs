//! Oracles for the regions the booleans return, written here and not taken from the library: the canonical form, an
//! exact area of the set rule over polygons, a search for crossing edges, and the polygons a path's outline is sampled
//! into.

use super::{Corners, Ring, shoelace_area};
use bullterrier_vector::Path;

/// Whether the point is in the result of verb `verb` (union, subtract, intersect, exclude) when it is `in_a` the first
/// region and `in_b` the second.
pub(crate) fn wanted(verb: usize, in_a: bool, in_b: bool) -> bool {
    match verb {
        0 => in_a || in_b,
        1 => in_a && !in_b,
        2 => in_a && in_b,
        _ => in_a != in_b,
    }
}

/// The closed polygons the outline of `path` is sampled into: a curve at `steps` points, a sub-path of fewer than
/// three corners left out (it encloses nothing).
pub(crate) fn sampled(path: &Path, steps: usize) -> Result<Vec<Corners>, Box<dyn std::error::Error>> {
    let mut found: Vec<Corners> = Vec::new();
    let mut at = (0.0, 0.0);
    for (letter, n) in super::commands(&path.to_svg_path())? {
        match (letter, &n[..]) {
            ('M', &[x, y]) => {
                found.push(vec![(x, y)]);
                at = (x, y);
            }
            ('L', &[x, y]) => {
                found.last_mut().ok_or("a line before a move")?.push((x, y));
                at = (x, y);
            }
            ('Q', &[cx, cy, x, y]) => {
                let ring = found.last_mut().ok_or("a curve before a move")?;
                for k in 1..=steps {
                    #[expect(clippy::cast_precision_loss, reason = "a few hundred steps")]
                    let t = k as f64 / steps as f64;
                    let s = 1.0 - t;
                    ring.push((
                        s * s * at.0 + 2.0 * s * t * cx + t * t * x,
                        s * s * at.1 + 2.0 * s * t * cy + t * t * y,
                    ));
                }
                at = (x, y);
            }
            ('C', &[ax, ay, bx, by, x, y]) => {
                let ring = found.last_mut().ok_or("a curve before a move")?;
                for k in 1..=steps {
                    #[expect(clippy::cast_precision_loss, reason = "a few hundred steps")]
                    let t = k as f64 / steps as f64;
                    let s = 1.0 - t;
                    ring.push((
                        s * s * s * at.0 + 3.0 * s * s * t * ax + 3.0 * s * t * t * bx + t * t * t * x,
                        s * s * s * at.1 + 3.0 * s * s * t * ay + 3.0 * s * t * t * by + t * t * t * y,
                    ));
                }
                at = (x, y);
            }
            ('Z', []) => {}
            _ => return Err(format!("not a path command: {letter} {n:?}").into()),
        }
    }
    found.retain(|ring| ring.len() >= 3);
    Ok(found)
}

/// Which set an edge belongs to, where it starts and where it ends.
type P = (f64, f64);
type Edge = (bool, P, P);

/// For each rule in `keep`, the area of the points it keeps, given whether each is inside the nonzero fill of polygons
/// `a` and of polygons `b`. Exact for straight edges: the plane is cut into slabs at every height where an edge ends
/// or two edges cross, inside a slab the edges do not cross, and the width of what is kept at the middle of a slab
/// times its height is its area.
pub(crate) fn exact_areas(a: &[Corners], b: &[Corners], keep: &[fn(bool, bool) -> bool]) -> Vec<f64> {
    // (set, from, to) of every edge that is not level
    let mut edges: Vec<Edge> = Vec::new();
    for (set, polygons) in [(false, a), (true, b)] {
        for corners in polygons {
            for (index, &from) in corners.iter().enumerate() {
                let to = corners[(index + 1) % corners.len()];
                if (from.1 - to.1).abs() > 0.0 {
                    edges.push((set, from, to));
                }
            }
        }
    }
    let mut heights: Vec<f64> = edges.iter().flat_map(|e| [e.1.1, e.2.1]).collect();
    for (index, e) in edges.iter().enumerate() {
        for f in &edges[index + 1..] {
            if let Some(y) = crossing_height((e.1, e.2), (f.1, f.2)) {
                heights.push(y);
            }
        }
    }
    heights.sort_by(f64::total_cmp);
    heights.dedup();
    let mut areas = vec![0.0; keep.len()];
    for pair in heights.windows(2) {
        let middle = f64::midpoint(pair[0], pair[1]);
        let mut events: Vec<(f64, bool, i32)> = edges
            .iter()
            .filter(|e| e.1.1.min(e.2.1) < middle && middle < e.1.1.max(e.2.1))
            .map(|&(set, from, to)| {
                let x = from.0 + (middle - from.1) * (to.0 - from.0) / (to.1 - from.1);
                (x, set, if to.1 > from.1 { 1 } else { -1 })
            })
            .collect();
        events.sort_by(|p, q| p.0.total_cmp(&q.0));
        let (mut winding_a, mut winding_b) = (0, 0);
        let mut widths = vec![0.0; keep.len()];
        for (index, &(x, set, direction)) in events.iter().enumerate() {
            if set {
                winding_b += direction;
            } else {
                winding_a += direction;
            }
            if let Some(next) = events.get(index + 1) {
                for (width, rule) in widths.iter_mut().zip(keep) {
                    if rule(winding_a != 0, winding_b != 0) {
                        *width += next.0 - x;
                    }
                }
            }
        }
        for (area, width) in areas.iter_mut().zip(widths) {
            *area += (pair[1] - pair[0]) * width;
        }
    }
    areas
}

/// Whether `point` is inside the nonzero fill of `polys`, by winding numbers.
pub(crate) fn inside(polys: &[Corners], point: (f64, f64)) -> bool {
    polys.iter().map(|corners| super::winding(corners, point)).sum::<i32>() != 0
}

/// The distance from `point` to the nearest edge of any of `polys`.
pub(crate) fn nearest_edge(polys: &[Corners], point: (f64, f64)) -> f64 {
    polys
        .iter()
        .map(|corners| super::clearance(corners, point))
        .fold(f64::INFINITY, f64::min)
}

/// The height at which the open segments `a`-`b` and `c`-`d` cross each other, if they do.
fn crossing_height(first: (P, P), second: (P, P)) -> Option<f64> {
    let ((a, b), (c, d)) = (first, second);
    if a.0.min(b.0) > c.0.max(d.0)
        || c.0.min(d.0) > a.0.max(b.0)
        || a.1.min(b.1) > c.1.max(d.1)
        || c.1.min(d.1) > a.1.max(b.1)
    {
        return None;
    }
    let (along, across) = ((b.0 - a.0, b.1 - a.1), (d.0 - c.0, d.1 - c.1));
    let denominator = along.0 * across.1 - along.1 * across.0;
    if denominator == 0.0 {
        return None;
    }
    let gap = (c.0 - a.0, c.1 - a.1);
    let on_first = (gap.0 * across.1 - gap.1 * across.0) / denominator;
    let on_second = (gap.0 * along.1 - gap.1 * along.0) / denominator;
    (0.0 < on_first && on_first < 1.0 && 0.0 < on_second && on_second < 1.0).then_some(a.1 + on_first * along.1)
}

/// A place where two edges of `rings` cross each other (their interiors meet at a point and neither end of one lies
/// within `slack` of the other's line), if there is one: a result must not overlap itself.
pub(crate) fn crossing_fault(rings: &[Ring], slack: f64) -> Option<String> {
    let mut edges: Vec<((f64, f64), (f64, f64))> = Vec::new();
    for ring in rings {
        for (index, &from) in ring.iter().enumerate() {
            edges.push((from, ring[(index + 1) % ring.len()]));
        }
    }
    // the distance from `p` to the line through `a` and `b`, signed
    let side = |a: (f64, f64), b: (f64, f64), p: (f64, f64)| {
        ((b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)) / super::distance(a, b)
    };
    for (index, &(a, b)) in edges.iter().enumerate() {
        for &(c, d) in &edges[index + 1..] {
            if a == b || c == d {
                continue;
            }
            let apart = |x: f64, y: f64| (x > slack && y < -slack) || (x < -slack && y > slack);
            if apart(side(a, b, c), side(a, b, d)) && apart(side(c, d, a), side(c, d, b)) {
                return Some(format!("edge {a:?}-{b:?} crosses edge {c:?}-{d:?}"));
            }
        }
    }
    None
}

/// What is wrong with the order, direction and start of `rings`, if anything: docs/api.md "Path", the canonical form.
pub(crate) fn canonical_fault(rings: &[Ring]) -> Option<String> {
    // the engine rounds to the width of all of its input, so that is the size a thin ring is judged against
    let scale = rings
        .iter()
        .flatten()
        .fold(0.0_f64, |most, &(x, y)| most.max(x.abs()).max(y.abs()));
    // the key a ring is ordered and started by: topmost, then leftmost
    let key = |point: (f64, f64)| (point.1, point.0);
    let mut last_outer: Option<(f64, f64)> = None;
    let mut last_hole: Option<(f64, f64)> = None;
    let mut outer_box: Option<(f64, f64, f64, f64)> = None;
    for (number, ring) in rings.iter().enumerate() {
        if ring.len() < 3 {
            return Some(format!("ring {number} has {} corners", ring.len()));
        }
        let first = key(ring[0]);
        if ring.iter().any(|&point| key(point) < first) {
            return Some(format!("ring {number} does not start at its topmost, leftmost corner"));
        }
        let area = shoelace_area(ring);
        // a ring thinner than two units in the last place of the result's largest coordinate (area over half the perimeter) is
        // rounding, not a contour: its side can have flipped
        let perimeter: f64 = ring
            .iter()
            .enumerate()
            .map(|(i, &p)| super::distance(p, ring[(i + 1) % ring.len()]))
            .sum();
        if 2.0 * area.abs() <= 2.0 * f64::EPSILON * scale * perimeter {
            return Some(format!("ring {number} is thinner than its coordinates hold ({area})"));
        }
        if area > 0.0 {
            if last_outer.is_some_and(|before| first < before) {
                return Some(format!("outer ring {number} is out of order"));
            }
            last_outer = Some(first);
            last_hole = None;
            let xs = ring.iter().map(|p| p.0);
            let ys = ring.iter().map(|p| p.1);
            outer_box = Some((
                xs.clone().fold(f64::INFINITY, f64::min),
                ys.clone().fold(f64::INFINITY, f64::min),
                xs.fold(f64::NEG_INFINITY, f64::max),
                ys.fold(f64::NEG_INFINITY, f64::max),
            ));
        } else if area < 0.0 {
            let Some((left, top, right, bottom)) = outer_box else {
                return Some(format!("hole {number} comes before any outer ring"));
            };
            if last_hole.is_some_and(|before| first < before) {
                return Some(format!("hole {number} is out of order"));
            }
            last_hole = Some(first);
            let (x, y) = ring[0];
            if x < left || x > right || y < top || y > bottom {
                return Some(format!("hole {number} lies outside the outer ring before it"));
            }
        } else {
            return Some(format!("ring {number} has no area"));
        }
    }
    None
}
