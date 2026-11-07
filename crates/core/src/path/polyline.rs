//! A path as straight lines: the flattening that pixels (and, later, the mesh) are drawn from, done once in design
//! pixels with the same walk that measuring uses, so that every output sees the same polygon.

use super::{FLATNESS, Path};
use crate::Error;
use crate::geometry::Point;

/// One sub-path flattened: the points of its straight edges in design pixels, from its start on.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Polyline {
    /// The start and the end of each edge in order. A closed sub-path does not repeat its start at the end.
    pub(crate) points: Vec<Point>,
    pub(crate) closed: bool,
    /// Whether the sub-path has a segment or a close, however short: a lone move is not drawn, while a line to its own
    /// start is a sub-path with one point that a round cap still marks.
    pub(crate) drawn: bool,
}

impl Path {
    /// The sub-paths of this path as polylines, every curve flattened to within [`FLATNESS`] design pixels. After each
    /// curve `charge` is called with the number of edges it made, and its error stops the flattening: a curve makes at
    /// most 2^16 edges, so a caller that counts them keeps the memory bounded.
    ///
    /// # Errors
    /// What `charge` returns.
    pub(crate) fn flattened(&self, charge: &mut dyn FnMut(usize) -> Result<(), Error>) -> Result<Vec<Polyline>, Error> {
        self.flattened_within(FLATNESS, charge)
    }

    /// As [`Path::flattened`], with every curve within `flatness` design pixels of its edges. `flatness` is above 0
    /// and a coordinate (the caller checks), so its square is a number.
    ///
    /// # Errors
    /// What `charge` returns.
    pub(crate) fn flattened_within(
        &self,
        flatness: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Vec<Polyline>, Error> {
        let mut polylines = Vec::with_capacity(self.subpaths.len());
        for subpath in &self.subpaths {
            let curves = subpath.curves();
            let mut points = vec![subpath.start];
            let mut edges = Vec::new();
            for (index, curve) in curves.iter().enumerate() {
                edges.clear();
                curve.flatten(index, (0.0, 1.0), 0, flatness, &mut edges);
                charge(edges.len())?;
                points.extend(edges.iter().map(|edge| edge.to));
            }
            if subpath.closed && points.len() > 1 && points.last() == Some(&subpath.start) {
                points.pop();
            }
            polylines.push(Polyline {
                points,
                closed: subpath.closed,
                drawn: !curves.is_empty(),
            });
        }
        Ok(polylines)
    }
}
