//! `Path::within`: the parts of a path's lines that lie inside a region.

use crate::Error;
use crate::field::{Outline, segment_budget};
use crate::geometry::Point;
use crate::path::Path;

impl Path {
    /// The parts of this path's lines that lie inside the region `region` fills (nonzero rule), as open lines cut
    /// where they cross the region's outline (exactly, at the intersection of the two edges); a closed
    /// sub-path wholly inside stays closed, one that leaves and comes back starts at its first point inside and its
    /// two ends are joined, one wholly outside is dropped. Curves are flattened first.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `region` if either path flattens to more than `MAX_SEGMENTS` edges.
    pub fn within(&self, region: &Path) -> Result<Path, Error> {
        let mut charge = segment_budget("region");
        let Ok(outline) = Outline::of(region, "within", &mut charge) else {
            return Ok(Path::new());
        };
        let mut pieces: Vec<Path> = Vec::new();
        for line in self.flattened(&mut charge)? {
            if !line.drawn {
                continue;
            }
            for (points, closed) in cut(&line.points, line.closed, &outline) {
                pieces.push(Path::polyline(points, closed)?);
            }
        }
        Ok(Path::joined(pieces.iter()))
    }
}

/// The pieces of one polyline inside `outline`, each with whether it is closed: every edge is cut where it crosses the
/// outline, and each part between two cuts is inside or out by its middle.
fn cut(points: &[Point], closed: bool, outline: &Outline) -> Vec<(Vec<Point>, bool)> {
    let count = points.len();
    let edges = if closed { count } else { count.saturating_sub(1) };
    let mut inside_all = true;
    // the parts of the polyline in order: (from, to, inside)
    let mut parts: Vec<(Point, Point, bool)> = Vec::new();
    for i in 0..edges {
        let (from, to) = (points[i], points[(i + 1) % count]);
        let mut shares = vec![0.0];
        shares.extend(outline.crossings(from, to).into_iter().filter(|t| *t > 0.0 && *t < 1.0));
        shares.push(1.0);
        let at = |t: f64| Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
        for pair in shares.windows(2) {
            let (a, b) = (at(pair[0]), at(pair[1]));
            let inside = outline.contains(Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)));
            inside_all &= inside;
            parts.push((a, b, inside));
        }
    }
    if count == 1 {
        return Vec::new();
    }
    if inside_all {
        return vec![(points.to_vec(), closed)];
    }
    let mut pieces: Vec<Vec<Point>> = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    for (a, b, inside) in &parts {
        if *inside {
            if current.is_empty() {
                current.push(*a);
            }
            current.push(*b);
        } else if !current.is_empty() {
            pieces.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        let joins = closed && parts.first().is_some_and(|part| part.2) && !pieces.is_empty();
        if joins {
            // the loop started inside and ended inside: its last piece runs on into the first
            let first = pieces.remove(0);
            current.extend(first.into_iter().skip(1));
            pieces.insert(0, current);
        } else {
            pieces.push(current);
        }
    }
    pieces
        .into_iter()
        .filter(|piece| piece.len() > 1)
        .map(|piece| (piece, false))
        .collect()
}
