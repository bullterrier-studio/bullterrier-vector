//! `Path::within`: the parts of a path's lines that lie inside a region.

use crate::Error;
use crate::field::{Outline, segment_budget};
use crate::geometry::Point;
use crate::path::Path;

/// The most edges one call may weigh against its segments and the points of its parts.
// constant: measured at about 1.4e8 a second, so half a billion is a few seconds, as for `hatch`
const MAX_VISITS: f64 = 5e8;

impl Path {
    /// The parts of this path's lines that lie inside the region `region` fills (nonzero rule), as open lines cut
    /// where they cross the region's outline (exactly, at the intersection of the two edges); a closed
    /// sub-path wholly inside stays closed, one that leaves and comes back starts at its first point inside and its
    /// two ends are joined, one wholly outside is dropped. Curves are flattened first.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `region` if either path flattens to more than `MAX_SEGMENTS` edges, or if cutting
    /// would weigh more than 500 million edges of the region against the segments and parts of this path (a region of
    /// very many edges crossed by very many long lines).
    pub fn within(&self, region: &Path) -> Result<Path, Error> {
        self.within_named(region, "region")
    }

    /// As [`Path::within`], a refusal naming `name`.
    pub(crate) fn within_named(&self, region: &Path, name: &'static str) -> Result<Path, Error> {
        let mut charge = segment_budget(name);
        // a region with no edge keeps nothing; a refusal of the flattening is the caller's
        let polylines = region.flattened(&mut charge)?;
        let Ok(outline) = Outline::from_polylines(&polylines, "within") else {
            return Ok(Path::new());
        };
        let mut visits = 0.0;
        let mut work = |edges: usize| {
            #[expect(clippy::cast_precision_loss, reason = "a count of edges is far below 2^53")]
            {
                visits += edges as f64;
            }
            if visits > MAX_VISITS {
                Err(Error::parameter(
                    name,
                    format!("is too detailed to cut lines to: it would weigh over {MAX_VISITS} edges"),
                ))
            } else {
                Ok(())
            }
        };
        let mut pieces: Vec<Path> = Vec::new();
        for line in self.flattened(&mut charge)? {
            if !line.drawn {
                continue;
            }
            for (points, closed) in cut(&line.points, line.closed, &outline, &mut work)? {
                pieces.push(Path::polyline(points, closed)?);
            }
        }
        Ok(Path::joined(pieces.iter()))
    }
}

/// The pieces of one polyline inside `outline`, each with whether it is closed: every edge is cut where it crosses the
/// outline, and each part between two cuts is inside or out by its middle.
fn cut(
    points: &[Point],
    closed: bool,
    outline: &Outline,
    work: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Vec<(Vec<Point>, bool)>, Error> {
    let count = points.len();
    let edges = if closed { count } else { count.saturating_sub(1) };
    let mut inside_all = true;
    // the parts of the polyline in order: (from, to, inside)
    let mut parts: Vec<(Point, Point, bool)> = Vec::new();
    for i in 0..edges {
        let (from, to) = (points[i], points[(i + 1) % count]);
        let mut shares = vec![0.0];
        shares.extend(outline.crossings(from, to, work)?.into_iter().filter(|t| *t > 0.0 && *t < 1.0));
        shares.push(1.0);
        let at = |t: f64| Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
        for pair in shares.windows(2) {
            let (a, b) = (at(pair[0]), at(pair[1]));
            let middle = Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y));
            work(outline.contains_cost(middle))?;
            let inside = outline.contains(middle);
            inside_all &= inside;
            parts.push((a, b, inside));
        }
    }
    if count == 1 {
        return Ok(Vec::new());
    }
    if inside_all {
        return Ok(vec![(points.to_vec(), closed)]);
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
    Ok(pieces
        .into_iter()
        .filter(|piece| piece.len() > 1)
        .map(|piece| (piece, false))
        .collect())
}
