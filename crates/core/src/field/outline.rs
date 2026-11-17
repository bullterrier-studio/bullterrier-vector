//! A path as straight edges in a grid, built once and asked many times: is a point inside (nonzero rule), and which
//! edge is nearest. Streamlines and the fields of a path read it at every step, so a question never walks all edges.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    reason = "cell indexes are clamped into a grid of at most 2048 by 2048 cells before they are used"
)]

use super::length;
use crate::Error;
use crate::geometry::Point;
use crate::path::{Path, Polyline};

/// The most cells along one side of the grid.
// constant: 2048 x 2048 cells of a few bytes is tens of megabytes at most, and an edge is never further than a cell
// from its neighbours then, whatever the path's size
const MAX_CELLS_PER_SIDE: f64 = 2048.0;

/// How far past the nearest edge another still counts towards the direction, as a multiple of the nearest distance.
// constant: at 2 an edge twice as far as the nearest is not counted and one as near as the nearest counts fully; the
// two edges of a stroke are within it only around its middle, a third of its width, where the direction has to turn
// from one edge's to the other's
const BAND: f64 = 2.0;

const NO_EDGE: u32 = u32::MAX;

/// What the nearest edge says at a point.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Nearest {
    /// The distance to the edge, from 0.
    pub(crate) distance: f64,
    /// The direction of the outline there as a doubled-angle vector `(cos 2a, sin 2a)`: the tangent of the nearest edge,
    /// turned smoothly into its neighbours' towards each end of the edge, so a polygon that stands for a curve has a
    /// direction that changes along it instead of jumping at its corners.
    pub(crate) doubled: (f64, f64),
}

/// The straight edges of a path, every sub-path closed as the fill closes it.
#[derive(Debug)]
pub(crate) struct Outline {
    edges: Vec<(Point, Point)>,
    /// The edge before and after each edge along its sub-path, `NO_EDGE` where the sub-path is two points.
    neighbours: Vec<(u32, u32)>,
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
    /// How many straight edges the outline has.
    pub(crate) fn edge_count(&self) -> usize {
        self.edges.len()
    }

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
        let mut neighbours = Vec::new();
        for line in polylines.iter().filter(|line| line.drawn) {
            let count = line.points.len();
            // every sub-path closes onto its start, as the fill closes it: two points make two opposite edges, which
            // wind to nothing, so a line encloses no area
            let first = edges.len();
            for index in 0..count {
                let (from, to) = (line.points[index], line.points[(index + 1) % count]);
                if from != to {
                    edges.push((from, to));
                }
            }
            let made = edges.len() - first;
            for at in 0..made {
                neighbours.push(if made > 2 {
                    (
                        u32::try_from(first + (at + made - 1) % made).unwrap_or(NO_EDGE),
                        u32::try_from(first + (at + 1) % made).unwrap_or(NO_EDGE),
                    )
                } else {
                    (NO_EDGE, NO_EDGE)
                });
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
        let count = edges.len() as f64;
        let wanted = if width * height > 0.0 {
            (width * height / count).sqrt()
        } else {
            width.max(height) / count
        };
        let size = wanted
            .max(width.max(height) / MAX_CELLS_PER_SIDE)
            .max(f64::MIN_POSITIVE);
        let (columns, rows) = (cells_across(width, size), cells_across(height, size));
        let mut outline = Self {
            edges,
            neighbours,
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
        let mut best = (f64::MAX, 0_usize, 0.0_f64);
        for ring in 0..=last {
            for (x, y) in ring_cells(cx, cy, ring) {
                if x < 0 || y < 0 || x >= columns || y >= rows {
                    continue;
                }
                for &id in &self.cells[(y * columns + x) as usize] {
                    let (a, b) = self.edges[id as usize];
                    let (d, along) = distance_to_edge(point, a, b);
                    if d < best.0 || (d.total_cmp(&best.0).is_eq() && (id as usize) < best.1) {
                        best = (d, id as usize, along);
                    }
                }
            }
            if best.0 <= ring as f64 * self.size {
                break;
            }
        }
        Nearest {
            distance: best.0,
            doubled: self.doubled_at(best.1, best.2),
        }
    }

    /// The doubled-angle direction of `edge` at `along` it (0 to 1): its own tangent, and from the middle towards a
    /// vertex turning half way into the neighbour's there.
    fn doubled_at(&self, edge: usize, along: f64) -> (f64, f64) {
        let own = self.doubled_tangent(edge);
        let (neighbour, share) = if along < 0.5 {
            (self.neighbours[edge].0, 0.5 - along)
        } else {
            (self.neighbours[edge].1, along - 0.5)
        };
        if neighbour == NO_EDGE {
            own
        } else {
            let other = self.doubled_tangent(neighbour as usize);
            (
                own.0 * (1.0 - share) + other.0 * share,
                own.1 * (1.0 - share) + other.1 * share,
            )
        }
    }

    /// The direction of the outline at `point` as a doubled-angle vector, blended over every edge about as near as the
    /// nearest: an edge `d` away counts for `(1 - (d - d0) / ((BAND - 1) d0))^2` while `d` is under `BAND * d0`, `d0`
    /// being the nearest distance. Along an edge the nearest one counts alone, so a line follows its edge; at the middle
    /// of a stroke the two edges count alike, so the direction turns smoothly between them instead of jumping from one
    /// edge's to the other's.
    pub(crate) fn direction(&self, point: Point) -> (f64, f64) {
        let nearest = self.nearest(point);
        let reach = BAND * nearest.distance;
        if reach <= 0.0 {
            return nearest.doubled;
        }
        let (cx, cy) = (self.column_of(point.x) as i64, self.row_of(point.y) as i64);
        let (columns, rows) = (self.columns as i64, self.rows as i64);
        let rings = ((reach / self.size).ceil() as i64 + 1).min(columns.max(rows));
        let mut seen: Vec<u32> = Vec::new();
        let (mut x_sum, mut y_sum, mut weights) = (0.0, 0.0, 0.0);
        for ring in 0..=rings {
            for (x, y) in ring_cells(cx, cy, ring) {
                if x < 0 || y < 0 || x >= columns || y >= rows {
                    continue;
                }
                for &id in &self.cells[(y * columns + x) as usize] {
                    if seen.contains(&id) {
                        continue;
                    }
                    seen.push(id);
                    let (a, b) = self.edges[id as usize];
                    let (d, along) = distance_to_edge(point, a, b);
                    if d >= reach {
                        continue;
                    }
                    let fall = 1.0 - (d - nearest.distance) / ((BAND - 1.0) * nearest.distance);
                    let weight = fall * fall;
                    let (tx, ty) = self.doubled_at(id as usize, along);
                    x_sum += weight * tx;
                    y_sum += weight * ty;
                    weights += weight;
                }
            }
        }
        if weights > 0.0 {
            (x_sum / weights, y_sum / weights)
        } else {
            nearest.doubled
        }
    }

    fn doubled_tangent(&self, edge: usize) -> (f64, f64) {
        let (a, b) = self.edges[edge];
        let size = length(b.x - a.x, b.y - a.y);
        let (dx, dy) = ((b.x - a.x) / size, (b.y - a.y) / size);
        (dx * dx - dy * dy, 2.0 * dx * dy)
    }

    /// Where the segment `from` to `to` crosses an edge of the outline, as shares of the segment (0 to 1), ascending
    /// and without repeats: every edge of the bands the segment spans is tried.
    pub(crate) fn crossings(&self, from: Point, to: Point) -> Vec<f64> {
        let (low, high) = if from.y <= to.y { (from.y, to.y) } else { (to.y, from.y) };
        let (min_x, max_x) = (from.x.min(to.x), from.x.max(to.x));
        let mut shares = Vec::new();
        let (first, last) = (self.row_of(low), self.row_of(high));
        for row in first..=last {
            for &id in &self.bands[row] {
                // an edge spanning several bands is met once: in the first of them that the segment spans
                let (start, end) = self.edges[id as usize];
                if row != self.row_of(start.y.min(end.y)).max(first) {
                    continue;
                }
                if start.x.max(end.x) < min_x || start.x.min(end.x) > max_x {
                    continue;
                }
                let (aim, side) = ((to.x - from.x, to.y - from.y), (end.x - start.x, end.y - start.y));
                let denominator = aim.0 * side.1 - aim.1 * side.0;
                if denominator == 0.0 {
                    continue;
                }
                let (dx, dy) = (start.x - from.x, start.y - from.y);
                let t = (dx * side.1 - dy * side.0) / denominator;
                let u = (dx * aim.1 - dy * aim.0) / denominator;
                if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                    shares.push(t);
                }
            }
        }
        shares.sort_by(f64::total_cmp);
        shares.dedup();
        shares
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

fn cells_across(extent: f64, size: f64) -> usize {
    ((extent / size).floor() as usize).saturating_add(1)
}

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

/// The distance from `p` to the edge `a` to `b` and where along the edge (0 to 1) the nearest point lies.
fn distance_to_edge(p: Point, a: Point, b: Point) -> (f64, f64) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let along = (((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (length(p.x - (a.x + dx * along), p.y - (a.y + dy * along)), along)
}
