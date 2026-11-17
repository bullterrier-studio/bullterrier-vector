//! `Field::contours`: the lines where a field equals a level, by marching squares over a grid of samples.
//!
//! A cell has four corners, each at or above the level ("high") or below it. Where two corners of an edge differ the
//! line crosses that edge, at the linear interpolation of the two samples. A cell with two crossings joins them; one
//! with four (a saddle, opposite corners high) is decided by the mean of its corners, so the high corners are joined
//! when the mean is high. A line is followed from edge to edge through the cells, never stored as loose segments: a
//! line that reaches the box ends there, one that comes back to its first edge is closed.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the grid has at most MAX_NODES nodes, checked before any index is made"
)]

use super::map::frame_of;
use super::{Field, length, segment_budget};
use crate::Error;
use crate::geometry::{Point, Rect, finite};
use crate::path::Path;

/// The most grid nodes a contour run samples.
// constant: 4 million samples are 32 MB of numbers and a bitmap of visited edges; a 2000 by 2000 map at one node a
// pixel
pub(crate) const MAX_NODES: f64 = 4_000_000.0;

/// The most levels one call traces.
// constant: each level is a pass over the whole grid, and no picture reads more than a few dozen lines of tone
pub(crate) const MAX_LEVELS: usize = 256;

/// The edge between two nodes: horizontal ones are even, vertical ones odd, so one number names each.
#[derive(Clone, Copy, PartialEq)]
struct Edge(usize);

struct Grid {
    columns: usize,
    rows: usize,
    frame: Rect,
    values: Vec<f64>,
}

impl Grid {
    fn nodes(&self) -> usize {
        (self.columns + 1) * (self.rows + 1)
    }

    fn value(&self, i: usize, j: usize) -> f64 {
        self.values[j * (self.columns + 1) + i]
    }

    fn at(&self, i: usize, j: usize) -> Point {
        Point::new(
            self.frame.x + i as f64 * self.frame.width / self.columns as f64,
            self.frame.y + j as f64 * self.frame.height / self.rows as f64,
        )
    }

    /// The edge's two end nodes `(i, j)`.
    fn ends(&self, edge: Edge) -> ((usize, usize), (usize, usize)) {
        let node = edge.0 / 2;
        let (i, j) = (node % (self.columns + 1), node / (self.columns + 1));
        if edge.0.is_multiple_of(2) {
            ((i, j), (i + 1, j))
        } else {
            ((i, j), (i, j + 1))
        }
    }

    /// Whether the edge is one of the grid's: nodes on the last column or row have no edge forward.
    fn exists(&self, edge: Edge) -> bool {
        let ((_, _), (i, j)) = self.ends(edge);
        i <= self.columns && j <= self.rows
    }

    fn crossed(&self, edge: Edge, level: f64) -> bool {
        let (a, b) = self.ends(edge);
        (self.value(a.0, a.1) >= level) != (self.value(b.0, b.1) >= level)
    }

    fn crossing(&self, edge: Edge, level: f64) -> Point {
        let ((ai, aj), (bi, bj)) = self.ends(edge);
        let (fa, fb) = (self.value(ai, aj), self.value(bi, bj));
        let t = ((level - fa) / (fb - fa)).clamp(0.0, 1.0);
        let (a, b) = (self.at(ai, aj), self.at(bi, bj));
        Point::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y))
    }

    fn horizontal(&self, i: usize, j: usize) -> Edge {
        Edge(2 * (j * (self.columns + 1) + i))
    }

    fn vertical(&self, i: usize, j: usize) -> Edge {
        Edge(2 * (j * (self.columns + 1) + i) + 1)
    }

    /// The cells an edge belongs to: one on the box, else two.
    fn cells(&self, edge: Edge) -> [Option<(usize, usize)>; 2] {
        let node = edge.0 / 2;
        let (i, j) = (node % (self.columns + 1), node / (self.columns + 1));
        if edge.0.is_multiple_of(2) {
            [(j > 0).then(|| (i, j - 1)), (j < self.rows).then_some((i, j))]
        } else {
            [(i > 0).then(|| (i - 1, j)), (i < self.columns).then_some((i, j))]
        }
    }

    /// The edge a line leaves the cell by when it came in by `entry`.
    fn exit(&self, cell: (usize, usize), entry: Edge, level: f64) -> Edge {
        let (i, j) = cell;
        let [top, right, bottom, left] = [
            self.horizontal(i, j),
            self.vertical(i + 1, j),
            self.horizontal(i, j + 1),
            self.vertical(i, j),
        ];
        let crossed: Vec<Edge> = [top, right, bottom, left]
            .into_iter()
            .filter(|edge| self.crossed(*edge, level))
            .collect();
        if crossed.len() == 2 {
            return if crossed[0] == entry { crossed[1] } else { crossed[0] };
        }
        // a saddle: the corners are alternately high and low; the pairs of edges that cut off a corner are
        // (top, right) and (bottom, left) when the corners at the top left and bottom right are the ones kept
        // together, else (top, left) and (right, bottom)
        let mean = (self.value(i, j) + self.value(i + 1, j) + self.value(i, j + 1) + self.value(i + 1, j + 1)) / 4.0;
        let top_left_high = self.value(i, j) >= level;
        let centre_high = mean >= level;
        let pairs = if top_left_high == centre_high {
            [(top, right), (bottom, left)]
        } else {
            [(top, left), (right, bottom)]
        };
        for (a, b) in pairs {
            if entry == a {
                return b;
            }
            if entry == b {
                return a;
            }
        }
        entry
    }
}

/// The cells along one side: no wider than `cell`, equal. A quotient within rounding of a whole number is that number
/// (`(W - p) / p` is 2.0000000000000004 for a box of three pixels), so a grid laid on pixel centres stays on them.
fn cells_along(extent: f64, cell: f64) -> f64 {
    let n = extent / cell;
    let whole = n.round();
    (if (n - whole).abs() <= 1e-9 * whole {
        whole
    } else {
        n.ceil()
    })
    .max(1.0)
}

/// How to tidy contour lines of a noisy field (a photograph's tone): blur the readings before tracing, and drop the
/// lines that stay short. The default does neither.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Tidy {
    smooth: f64,
    min_length: f64,
}

impl Tidy {
    /// Blur the readings with a Gaussian of standard deviation `sigma` design pixels before tracing (0: not at all).
    /// Grass and grain make hundreds of tiny loops and jagged lines; a sigma of a few pixels removes them.
    #[must_use]
    pub fn smooth(mut self, sigma: f64) -> Self {
        self.smooth = sigma;
        self
    }

    /// Drop every line shorter than `length` design pixels (a closed line counts its closing edge).
    #[must_use]
    pub fn min_length(mut self, length: f64) -> Self {
        self.min_length = length;
        self
    }
}

/// The most multiply-adds the blur of one call may cost.
// constant: about two seconds measured at a nanosecond a tap; a sigma of 50 cells over a 2000 by 2000 grid is under it
const MAX_SMOOTH_WORK: f64 = 2e9;

/// Blurs `values` (columns + 1 by rows + 1 nodes) with a Gaussian of `sigma` nodes along each axis, the edge held.
fn blur(values: &mut [f64], width: usize, height: usize, sigma: (f64, f64)) {
    fn pass(values: &mut [f64], length: usize, stride: usize, lines: usize, line_step: usize, sigma: f64) {
        if sigma <= 0.0 {
            return;
        }
        let reach = (3.0 * sigma).ceil() as usize;
        let mut taps: Vec<f64> = (0..=2 * reach)
            .map(|i| {
                let d = i as f64 - reach as f64;
                crate::math::exp(-d * d / (2.0 * sigma * sigma))
            })
            .collect();
        let total: f64 = taps.iter().sum();
        for tap in &mut taps {
            *tap /= total;
        }
        let mut line = vec![0.0; length];
        for n in 0..lines {
            let base = n * line_step;
            for (i, out) in line.iter_mut().enumerate() {
                *out = taps
                    .iter()
                    .enumerate()
                    .map(|(t, tap)| {
                        let at = (i + t).saturating_sub(reach).min(length - 1);
                        tap * values[base + at * stride]
                    })
                    .sum();
            }
            for (i, out) in line.iter().enumerate() {
                values[base + i * stride] = *out;
            }
        }
    }
    pass(values, width, 1, height, width, sigma.0);
    pass(values, height, width, width, 1, sigma.1);
}

impl Field {
    /// The lines where this field equals each of `levels`, as one path: the grid is `frame` cut into equal cells no
    /// wider than `cell` (design pixels), the field is read at their corners, and a line crosses a cell edge at the
    /// linear interpolation of the two readings. A reading equal to the level counts as above it. A line that
    /// reaches the frame is open there; one that does not is closed. Lines are made level by level in the order of
    /// `levels`, each from the frame's edge first and then through the inside, so the same input gives the same path.
    /// A saddle cell joins its high corners when the mean of its four readings is high. A level the field never
    /// crosses gives nothing, and an empty path is the answer when no level crosses.
    ///
    /// To trace a map along its pixels, give `frame` the box of the pixel centres (the map's box inset by half a
    /// pixel on each side) and a `cell` of one pixel.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `levels` if there are more than 256 or one is not finite; naming `box` unless it is
    /// finite, within ±1e100 and has a width and a height above 0; naming `cell` unless it is above 0 and finite, if
    /// the grid would need more than 4 million nodes or if reading the field at them would be too much work (see
    /// [`Field::to_map`]); naming `levels` past `MAX_SEGMENTS` points.
    pub fn contours(&self, levels: &[f64], frame: Rect, cell: f64) -> Result<Path, Error> {
        self.contours_with(levels, frame, cell, &Tidy::default())
    }

    /// As [`Field::contours`], tidied by `tidy`: the readings blurred first, then short lines dropped. Every line runs
    /// with the high side (a reading at or above the level) on its left as drawn (y down), so the loops of a map
    /// that is high inside a region wind the same way and holes the other, and fill by the nonzero rule as that
    /// region.
    ///
    /// # Errors
    /// As [`Field::contours`]; also [`Error::Parameter`] naming `smooth` or `min_length` unless it is finite and at
    /// least 0, and `smooth` if the blur would cost over 2 billion multiply-adds.
    pub fn contours_with(&self, levels: &[f64], frame: Rect, cell: f64, tidy: &Tidy) -> Result<Path, Error> {
        for (name, value) in [("smooth", tidy.smooth), ("min_length", tidy.min_length)] {
            if !(value.is_finite() && value >= 0.0) {
                return Err(Error::parameter(name, format!("must be at least 0, got {value}")));
            }
        }
        if levels.len() > MAX_LEVELS {
            return Err(Error::parameter(
                "levels",
                format!("at most {MAX_LEVELS} levels, got {}", levels.len()),
            ));
        }
        for level in levels {
            finite("levels", *level)?;
        }
        let frame = frame_of(Some(frame), 1, 1)?;
        if !(cell.is_finite() && cell > 0.0) {
            return Err(Error::parameter("cell", format!("must be above 0, got {cell}")));
        }
        let (columns, rows) = (cells_along(frame.width, cell), cells_along(frame.height, cell));
        if (columns + 1.0) * (rows + 1.0) > MAX_NODES {
            return Err(Error::parameter(
                "cell",
                format!("is too small for a box this large: its grid would pass {MAX_NODES} nodes"),
            ));
        }
        self.charge_reads("cell", "contours", (columns + 1.0) * (rows + 1.0))?;
        let mut grid = Grid {
            columns: columns as usize,
            rows: rows as usize,
            frame,
            values: Vec::new(),
        };
        let mut values = Vec::with_capacity(grid.nodes());
        for j in 0..=grid.rows {
            for i in 0..=grid.columns {
                values.push(self.at(grid.at(i, j)));
            }
        }
        grid.values = values;
        if tidy.smooth > 0.0 {
            let sigma = (tidy.smooth * columns / frame.width, tidy.smooth * rows / frame.height);
            let taps = |s: f64| 2.0 * (3.0 * s).ceil() + 1.0;
            if grid.nodes() as f64 * (taps(sigma.0) + taps(sigma.1)) > MAX_SMOOTH_WORK {
                return Err(Error::parameter(
                    "smooth",
                    format!("is too wide for a grid this fine: the blur would pass {MAX_SMOOTH_WORK} steps"),
                ));
            }
            blur(&mut grid.values, grid.columns + 1, grid.rows + 1, sigma);
        }
        let mut charge = segment_budget("levels");
        let mut lines = Vec::new();
        for level in levels {
            trace(&grid, *level, tidy.min_length, &mut charge, &mut lines)?;
        }
        Ok(Path::joined(lines.iter()))
    }
}

/// Whether the line `points`, which started on `edge`, has its high side on the left as drawn (y down).
fn high_on_left(grid: &Grid, edge: Edge, level: f64, points: &[Point]) -> bool {
    let (a, b) = grid.ends(edge);
    let high = if grid.value(a.0, a.1) >= level { a } else { b };
    let (from, to, corner) = (points[0], points[1], grid.at(high.0, high.1));
    (to.x - from.x) * (corner.y - from.y) - (to.y - from.y) * (corner.x - from.x) < 0.0
}

/// Every line of `level`: those that start on the frame, then the closed ones.
fn trace(
    grid: &Grid,
    level: f64,
    min_length: f64,
    charge: &mut impl FnMut(usize) -> Result<(), Error>,
    lines: &mut Vec<Path>,
) -> Result<(), Error> {
    let mut seen = vec![false; 2 * grid.nodes()];
    let on_frame = |edge: Edge| grid.cells(edge).iter().flatten().count() == 1;
    for open in [true, false] {
        for number in 0..2 * grid.nodes() {
            let edge = Edge(number);
            if !grid.exists(edge) || seen[number] || !grid.crossed(edge, level) || on_frame(edge) != open {
                continue;
            }
            let Some(mut cell) = grid.cells(edge).into_iter().flatten().next() else {
                continue;
            };
            let mut points = vec![grid.crossing(edge, level)];
            seen[number] = true;
            let mut entry = edge;
            let closed = loop {
                let out = grid.exit(cell, entry, level);
                if out == edge {
                    break true;
                }
                if seen[out.0] {
                    break false;
                }
                seen[out.0] = true;
                points.push(grid.crossing(out, level));
                let Some(next) = grid.cells(out).into_iter().flatten().find(|next| *next != cell) else {
                    break false;
                };
                (cell, entry) = (next, out);
            };
            charge(points.len())?;
            if points.len() >= 2 {
                let closed = closed && !open;
                if !high_on_left(grid, edge, level, &points) {
                    points.reverse();
                }
                let along: f64 = points
                    .windows(2)
                    .map(|w| length(w[1].x - w[0].x, w[1].y - w[0].y))
                    .sum();
                let closing = if closed {
                    let (a, b) = (points[points.len() - 1], points[0]);
                    length(b.x - a.x, b.y - a.y)
                } else {
                    0.0
                };
                if along + closing >= min_length {
                    lines.push(Path::polyline(points, closed)?);
                }
            }
        }
    }
    Ok(())
}
