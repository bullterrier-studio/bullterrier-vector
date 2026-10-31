//! The region a path fills, and its separate parts.

use super::curve::flatten;
use super::{Path, SubPath};
use crate::Error;
use crate::geometry::{Point, coordinate_point};

impl Path {
    /// Whether `point` is inside the region the outline fills, by the nonzero rule: the outline winds round it at
    /// least once, in either direction. Where outlines overlap or cross themselves, round twice is inside; a hole
    /// is an outline that runs the other way. An open sub-path fills as if closed, as in SVG. A point exactly on
    /// the outline may be either.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `point` is NaN, infinite or beyond ±1e100.
    pub fn contains(&self, point: impl Into<Point>) -> Result<bool, Error> {
        let point = coordinate_point("point", point)?;
        let winding: i32 = self
            .subpaths
            .iter()
            .map(|subpath| winding(&subpath.outline(), point))
            .sum();
        Ok(winding != 0)
    }

    /// The separate parts of the path, in path order: each closed outline together with the holes inside it (a
    /// closed sub-path whose start lies inside the outline), and each open sub-path alone. An outline inside a hole
    /// is a part of its own, and holes inside it are its. A hole written before its outline stays with it, and a part
    /// keeps its sub-paths in the order they were written. An empty path has no parts.
    #[must_use]
    pub fn split(&self) -> Vec<Self> {
        let outlines: Vec<Vec<Point>> = self.subpaths.iter().map(SubPath::outline).collect();
        // how many other closed outlines each closed sub-path lies inside, judged by where it starts
        let containers: Vec<Vec<usize>> = self
            .subpaths
            .iter()
            .enumerate()
            .map(|(index, subpath)| {
                if !subpath.closed {
                    return Vec::new();
                }
                (0..outlines.len())
                    .filter(|&other| {
                        other != index && self.subpaths[other].closed && winding(&outlines[other], subpath.start) != 0
                    })
                    .collect()
            })
            .collect();
        // shallowest first, so that the outline a hole belongs to is placed before it
        let mut order: Vec<usize> = (0..self.subpaths.len()).collect();
        order.sort_by_key(|&index| containers[index].len());
        // the index of the sub-path that opens each one's part
        let mut part: Vec<Option<usize>> = vec![None; self.subpaths.len()];
        for index in order {
            let inside = &containers[index];
            let is_hole = inside.len() % 2 == 1;
            let outline = if is_hole {
                inside
                    .iter()
                    .copied()
                    .filter(|&other| part[other].is_some())
                    .max_by_key(|&other| (containers[other].len(), std::cmp::Reverse(other)))
            } else {
                None
            };
            part[index] = Some(outline.and_then(|outline| part[outline]).unwrap_or(index));
        }
        let mut slots: Vec<Option<usize>> = vec![None; self.subpaths.len()];
        let mut parts: Vec<Self> = Vec::new();
        for (index, subpath) in self.subpaths.iter().enumerate() {
            let Some(opener) = part[index] else { continue };
            let slot = *slots[opener].get_or_insert_with(|| {
                parts.push(Self::new());
                parts.len() - 1
            });
            parts[slot].subpaths.push(subpath.clone());
        }
        parts
    }
}

impl SubPath {
    /// The corners of the flattened outline, with the line from the last back to the first implied, as the region
    /// is filled.
    fn outline(&self) -> Vec<Point> {
        let edges = flatten(&self.curves());
        let mut corners: Vec<Point> = edges.iter().map(|edge| edge.from).collect();
        if let (Some(first), Some(last)) = (edges.first(), edges.last())
            && last.to != first.from
        {
            corners.push(last.to);
        }
        corners
    }
}

/// How many times the closed polygon through `corners` winds round `point`, counted by the edges that cross the
/// horizontal line through the point to its right: up counts one way, down the other.
fn winding(corners: &[Point], point: Point) -> i32 {
    let mut turns = 0;
    for (index, from) in corners.iter().enumerate() {
        let to = corners[(index + 1) % corners.len()];
        // positive when `point` is left of the edge from `from` to `to`
        let side = (to.x - from.x) * (point.y - from.y) - (point.x - from.x) * (to.y - from.y);
        if from.y <= point.y {
            if to.y > point.y && side > 0.0 {
                turns += 1;
            }
        } else if to.y <= point.y && side < 0.0 {
            turns -= 1;
        }
    }
    turns
}
