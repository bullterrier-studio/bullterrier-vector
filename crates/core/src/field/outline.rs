//! A path as straight edges in a grid, built once and asked many times: is a point inside (nonzero rule), and which
//! edge is nearest. Streamlines and the fields of a path read it at every step, so a question never walks all edges.

use crate::Error;
use crate::geometry::Point;
use crate::path::{Path, Polyline};

/// The most cells along one side of the grid.
// constant: 2048 x 2048 cells of a few bytes is tens of megabytes at most, and an edge is never further than a cell
// from its neighbours then, whatever the path's size
const MAX_CELLS_PER_SIDE: f64 = 2048.0;

/// What the nearest edge says at a point.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Nearest {
    /// The distance to the edge, from 0.
    pub(crate) distance: f64,
    /// The unit direction of the edge (its sign is the edge's own).
    pub(crate) tangent: (f64, f64),
}

/// The straight edges of a path, every sub-path closed as the fill closes it.
#[derive(Debug)]
pub(crate) struct Outline {
    edges: Vec<(Point, Point)>,
    left: f64,
    top: f64,
    size: f64,
    columns: usize,
    rows: usize,
    /// The edges that touch each cell, row by row.
    cells: Vec<Vec<u32>>,
    /// The edges that cross each horizontal band (one cell high), for the winding of a point.
    bands: Vec<Vec<u32>>,
}

impl Outline {
    /// The outline of `path`; `charge` is told the edges of each curve, as for [`Path::flattened`].
    ///
    /// # Errors
    /// [`Error::Geometry`] naming `verb` if the path has no edge; what `charge` returns.
    pub(crate) fn of(
        path: &Path,
        verb: &str,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let polylines = path.flattened(charge)?;
        Self::from_polylines(&polylines, verb)
    }

    pub(crate) fn from_polylines(polylines: &[Polyline], verb: &str) -> Result<Self, Error> {
        let mut edges = Vec::new();
        for line in polylines.iter().filter(|line| line.drawn) {
            let count = line.points.len();
            // every sub-path closes onto its start, as the fill closes it; two points make one edge
            let closing = if count > 2 { count } else { count - 1 };
            for index in 0..closing {
                let (from, to) = (line.points[index], line.points[(index + 1) % count]);
                if from != to {
                    edges.push((from, to));
                }
            }
        }
        if edges.is_empty() {
            return Err(Error::geometry(verb, "the path has no edge"));
        }
        let (mut left, mut top, mut right, mut bottom) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for (a, b) in &edges {
            for p in [a, b] {
                left = left.min(p.x);
                right = right.max(p.x);
                top = top.min(p.y);
                bottom = bottom.max(p.y);
            }
        }
        let (width, height) = (right - left, bottom - top);
        #[expect(clippy::cast_precision_loss, reason = "an edge count far below 2^53")]
        let count = edges.len() as f64;
        let wanted = if width * height > 0.0 {
            (width * height / count).sqrt()
        } else {
            width.max(height) / count
        };
        let size = wanted.max(width.max(height) / MAX_CELLS_PER_SIDE).max(f64::MIN_POSITIVE);
        let side = |extent: f64| -> usize { cells_across(extent, size) };
        let (columns, rows) = (side(width), side(height));
        let mut outline = Self {
            edges,
            left,
            top,
            size,
            columns,
            rows,
            cells: vec![Vec::new(); columns * rows],
            bands: vec![Vec::new(); rows],
        };
        outline.fill();
        Ok(outline)
    }

    /// Registers every edge in each cell it touches and each band it spans.
    fn fill(&mut self) {
        for index in 0..self.edges.len() {
            let (a, b) = self.edges[index];
            let id = u32::try_from(index).unwrap_or(u32::MAX);
            let (low, high) = if a.y <= b.y { (a.y, b.y) } else { (b.y, a.y) };
            for row in self.row_of(low)..=self.row_of(high) {
                self.bands[row].push(id);
            }
            let (first, last) = (self.column_of(a.x.min(b.x)), self.column_of(a.x.max(b.x)));
            for column in first..=last {
                // the part of the edge inside this column strip
                let strip_left = self.left + column as f64 * self.size;
                let strip_right = strip_left + self.size;
                let (ya, yb) = strip_span(a, b, strip_left, strip_right);
                for row in self.row_of(ya.min(yb))..=self.row_of(ya.max(yb)) {
                    self.cells[row * self.columns + column].push(id);
                }
            }
        }
    }

    fn column_of(&self, x: f64) -> usize {
        clamp_cell((x - self.left) / self.size, self.columns)
    }

    fn row_of(&self, y: f64) -> usize {
        clamp_cell((y - self.top) / self.size, self.rows)
    }

    /// Whether the nonzero rule fills `point`.
    pub(crate) fn contains(&self, point: Point) -> bool {
        let extent = self.top + self.rows as f64 * self.size;
        if point.y < self.top || point.y > extent {
            return false;
        }
        let mut turns = 0_i32;
        for &id in &self.bands[self.row_of(point.y)] {
            let (from, to) = self.edges[id as usize];
            let side = (to.x - from.x) * (point.y - from.y) - (point.x - from.x) * (to.y - from.y);
            if from.y <= point.y {
                if to.y > point.y && side > 0.0 {
                    turns += 1;
                }
            } else if to.y <= point.y && side < 0.0 {
                turns -= 1;
            }
        }
        turns != 0
    }

    /// The nearest edge to `point`: its distance and direction.
    pub(crate) fn nearest(&self, point: Point) -> Nearest {
        // a point outside the grid is asked from the cell nearest it: the rings below stay valid lower bounds
        let (cx, cy) = (self.column_of(point.x) as i64, self.row_of(point.y) as i64);
        let (columns, rows) = (self.columns as i64, self.rows as i64);
        let last = columns.max(rows);
        let mut best = (f64::MAX, 0_usize);
        for ring in 0..=last {
            for (x, y) in ring_cells(cx, cy, ring) {
                if x < 0 || y < 0 || x >= columns || y >= rows {
                    continue;
                }
                for &id in &self.cells[(y * columns + x) as usize] {
                    let (a, b) = self.edges[id as usize];
                    let d = distance_to_edge(point, a, b);
                    if d < best.0 || (d == best.0 && (id as usize) < best.1) {
                        best = (d, id as usize);
                    }
                }
            }
            if best.0 <= ring as f64 * self.size {
                break;
            }
        }
        let (a, b) = self.edges[best.1];
        let length = (b.x - a.x).hypot(b.y - a.y);
        Nearest {
            distance: best.0,
            tangent: ((b.x - a.x) / length, (b.y - a.y) / length),
        }
    }

    /// The smallest rectangle round the edges: left, top, right, bottom.
    pub(crate) fn extent(&self) -> (f64, f64, f64, f64) {
        let (mut l, mut t, mut r, mut b) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for (p, q) in &self.edges {
            l = l.min(p.x.min(q.x));
            r = r.max(p.x.max(q.x));
            t = t.min(p.y.min(q.y));
            b = b.max(p.y.max(q.y));
        }
        (l, t, r, b)
    }
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a small positive cell count")]
fn cells_across(extent: f64, size: f64) -> usize {
    ((extent / size).floor() as usize).saturating_add(1)
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped into the grid first")]
fn clamp_cell(value: f64, count: usize) -> usize {
    value.floor().clamp(0.0, (count - 1) as f64) as usize
}

/// The `y` of the edge `a` to `b` at the two sides of a vertical strip, within the edge.
fn strip_span(a: Point, b: Point, left: f64, right: f64) -> (f64, f64) {
    if (b.x - a.x).abs() < f64::EPSILON {
        return (a.y, b.y);
    }
    let at = |x: f64| {
        let t = ((x - a.x) / (b.x - a.x)).clamp(0.0, 1.0);
        a.y + (b.y - a.y) * t
    };
    (at(left), at(right))
}

/// The cells at Chebyshev distance `ring` from `(cx, cy)`.
fn ring_cells(cx: i64, cy: i64, ring: i64) -> Vec<(i64, i64)> {
    if ring == 0 {
        return vec![(cx, cy)];
    }
    let mut cells = Vec::new();
    for dx in -ring..=ring {
        cells.push((cx + dx, cy - ring));
        cells.push((cx + dx, cy + ring));
    }
    for dy in (1 - ring)..ring {
        cells.push((cx - ring, cy + dy));
        cells.push((cx + ring, cy + dy));
    }
    cells
}

fn distance_to_edge(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let along = (((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (p.x - (a.x + dx * along)).hypot(p.y - (a.y + dy * along))
}
