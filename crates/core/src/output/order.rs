//! Putting strokes in an order that cuts pen-up travel: the nearest neighbour, each stroke drawn from whichever of its
//! ends is nearer the pen, a closed loop entered at the vertex nearest the pen.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the grid has at most MAX_SIDE cells along a side, and its indices are clamped to them"
)]

use crate::field::length;
use crate::geometry::Point;

/// The most cells along a side of the grid of ends.
// constant: 2048 by 2048 cells of a few entries each is the finest grid worth keeping for a few million entries
const MAX_SIDE: usize = 2048;

/// A stroke to draw: its points, and whether it comes back to its start (the last point is not repeated then).
pub(crate) struct Stroke {
    pub(crate) points: Vec<Point>,
    pub(crate) closed: bool,
}

/// One place the pen may enter a stroke: the stroke, the vertex, and where it lies.
struct End {
    stroke: u32,
    vertex: u32,
    at: Point,
}

/// `strokes` in the order of the nearest neighbour from the pen at `start`, each as the pen draws it: an open stroke
/// from the nearer end (reversed when that is its last), a closed one from the nearest vertex round to it again, its
/// first point repeated at the end. Ties go to the earlier stroke. Time is about linear in the strokes for strokes spread
/// over a page (a grid finds the nearest end), and at worst quadratic.
pub(crate) fn nearest_neighbour(strokes: &[Stroke], start: Point) -> Vec<Vec<Point>> {
    let mut ends: Vec<End> = Vec::new();
    for (index, stroke) in strokes.iter().enumerate() {
        let id = u32::try_from(index).unwrap_or(u32::MAX);
        if stroke.closed {
            ends.extend(stroke.points.iter().enumerate().map(|(vertex, at)| End {
                stroke: id,
                vertex: u32::try_from(vertex).unwrap_or(u32::MAX),
                at: *at,
            }));
        } else if let (Some(first), Some(last)) = (stroke.points.first(), stroke.points.last()) {
            ends.push(End {
                stroke: id,
                vertex: 0,
                at: *first,
            });
            ends.push(End {
                stroke: id,
                vertex: u32::try_from(stroke.points.len() - 1).unwrap_or(u32::MAX),
                at: *last,
            });
        }
    }
    let mut grid = Grid::new(&ends);
    let mut used = vec![false; strokes.len()];
    let mut order = Vec::with_capacity(strokes.len());
    let mut pen = start;
    for _ in 0..strokes.len() {
        let Some(found) = grid.nearest(&ends, &used, pen) else {
            break;
        };
        let (stroke, vertex) = (ends[found].stroke as usize, ends[found].vertex as usize);
        used[stroke] = true;
        let source = &strokes[stroke];
        let drawn: Vec<Point> = if source.closed {
            let n = source.points.len();
            (0..=n).map(|k| source.points[(vertex + k) % n]).collect()
        } else if vertex == 0 {
            source.points.clone()
        } else {
            source.points.iter().rev().copied().collect()
        };
        pen = drawn[drawn.len() - 1];
        order.push(drawn);
    }
    order
}

/// A grid of the entries, each cell the entries inside it.
struct Grid {
    left: f64,
    top: f64,
    cell: f64,
    columns: usize,
    rows: usize,
    cells: Vec<Vec<u32>>,
}

impl Grid {
    fn new(ends: &[End]) -> Self {
        let (mut left, mut top, mut right, mut bottom) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for end in ends {
            left = left.min(end.at.x);
            right = right.max(end.at.x);
            top = top.min(end.at.y);
            bottom = bottom.max(end.at.y);
        }
        if ends.is_empty() {
            (left, top, right, bottom) = (0.0, 0.0, 0.0, 0.0);
        }
        let (width, height) = (right - left, bottom - top);
        // about two entries a cell, never a cell smaller than a thousandth of the longer side
        let wanted = (ends.len() as f64 / 2.0).max(1.0);
        let longer = width.max(height).max(f64::MIN_POSITIVE);
        let cell = if width * height > 0.0 {
            (width * height / wanted).sqrt()
        } else {
            longer / wanted
        }
        .max(longer / MAX_SIDE as f64);
        let columns = ((width / cell).floor() as usize + 1).min(MAX_SIDE);
        let rows = ((height / cell).floor() as usize + 1).min(MAX_SIDE);
        let mut grid = Self {
            left,
            top,
            cell,
            columns,
            rows,
            cells: vec![Vec::new(); columns * rows],
        };
        for (index, end) in ends.iter().enumerate() {
            let (column, row) = grid.cell_of(end.at);
            grid.cells[row * columns + column].push(u32::try_from(index).unwrap_or(u32::MAX));
        }
        grid
    }

    /// The cell holding `at`, or the nearest cell of the grid to it.
    fn cell_of(&self, at: Point) -> (usize, usize) {
        let clamp = |value: f64, count: usize| (value.floor().max(0.0) as usize).min(count - 1);
        (
            clamp((at.x - self.left) / self.cell, self.columns),
            clamp((at.y - self.top) / self.cell, self.rows),
        )
    }

    /// The nearest entry of a stroke not yet `used` to `pen`: the least distance, then the earliest entry.
    fn nearest(&mut self, ends: &[End], used: &[bool], pen: Point) -> Option<usize> {
        let (column, row) = self.cell_of(pen);
        let reach = self.columns.max(self.rows);
        let mut best: Option<(f64, usize)> = None;
        for ring in 0..=reach {
            let low = column.saturating_sub(ring);
            let high = (column + ring).min(self.columns - 1);
            let (above, below) = (row.checked_sub(ring), (row + ring < self.rows).then_some(row + ring));
            for y in row.saturating_sub(ring)..=(row + ring).min(self.rows - 1) {
                // the cells of the ring: the whole top and bottom rows, only the two sides between
                let edge = Some(y) == above || Some(y) == below;
                let columns: Vec<usize> = if edge {
                    (low..=high).collect()
                } else {
                    [
                        column.checked_sub(ring),
                        (column + ring < self.columns).then_some(column + ring),
                    ]
                    .into_iter()
                    .flatten()
                    .collect()
                };
                for x in columns {
                    let cell = &mut self.cells[y * self.columns + x];
                    cell.retain(|entry| !used[ends[*entry as usize].stroke as usize]);
                    for &entry in cell.iter() {
                        let end = &ends[entry as usize];
                        let distance = length(end.at.x - pen.x, end.at.y - pen.y);
                        let better = best.is_none_or(|(d, e)| {
                            distance < d
                                || (distance.to_bits() == d.to_bits() && better_tie(end, &ends[e], entry as usize, e))
                        });
                        if better {
                            best = Some((distance, entry as usize));
                        }
                    }
                }
            }
            // a cell of the next ring is at least `ring` cells from the pen's, so no nearer than that
            if best.is_some_and(|(d, _)| d <= ring as f64 * self.cell) {
                break;
            }
        }
        best.map(|(_, entry)| entry)
    }
}

/// Whether the entry `a` (index `ai`) goes before `b` (`bi`) at equal distance: the earlier stroke, its first end first.
fn better_tie(a: &End, b: &End, ai: usize, bi: usize) -> bool {
    (a.stroke, a.vertex, ai) < (b.stroke, b.vertex, bi)
}
