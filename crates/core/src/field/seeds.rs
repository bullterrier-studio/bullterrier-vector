//! The samples of the lines made so far, in a grid of cells as wide as the narrowest spacing: "is any sample of
//! another line closer than this" looks at the cells round the point, never at every sample.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    reason = "cell indexes are clamped into a grid of at most 2048 by 2048 cells before they are used"
)]

use crate::geometry::Point;

/// One point of a line.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Sample {
    pub(crate) at: Point,
    /// The line it belongs to.
    pub(crate) line: u32,
    /// How far along its line it lies from the seed, in pixels: forward positive, backward negative.
    pub(crate) along: f64,
}

/// The cell of a point and the samples of each cell, as a linked list through `next`.
#[derive(Debug)]
pub(crate) struct Samples {
    left: f64,
    top: f64,
    cell: f64,
    columns: usize,
    rows: usize,
    head: Vec<u32>,
    items: Vec<(Sample, u32)>,
}

const NONE: u32 = u32::MAX;

impl Samples {
    /// A grid of `cell`-wide cells over the rectangle from `(left, top)` of `width` by `height`.
    pub(crate) fn new(left: f64, top: f64, width: f64, height: f64, cell: f64) -> Self {
        let across = |extent: f64| ((extent / cell).floor() as usize).saturating_add(1);
        let (columns, rows) = (across(width), across(height));
        Self {
            left,
            top,
            cell,
            columns,
            rows,
            head: vec![NONE; columns * rows],
            items: Vec::new(),
        }
    }

    /// The number of cells a grid over this rectangle would have.
    pub(crate) fn cells(width: f64, height: f64, cell: f64) -> f64 {
        ((width / cell).floor() + 1.0) * ((height / cell).floor() + 1.0)
    }

    fn column(&self, x: f64) -> i64 {
        ((x - self.left) / self.cell).floor() as i64
    }

    fn row(&self, y: f64) -> i64 {
        ((y - self.top) / self.cell).floor() as i64
    }

    /// Adds a sample. Its point lies inside the rectangle the grid covers.
    pub(crate) fn add(&mut self, sample: Sample) {
        let column = self.column(sample.at.x).clamp(0, self.columns as i64 - 1) as usize;
        let row = self.row(sample.at.y).clamp(0, self.rows as i64 - 1) as usize;
        let cell = row * self.columns + column;
        self.items.push((sample, self.head[cell]));
        self.head[cell] = u32::try_from(self.items.len() - 1).unwrap_or(NONE - 1);
    }

    /// Whether some sample that `counts` says counts lies closer than `radius` to `at`.
    pub(crate) fn any_within(&self, at: Point, radius: f64, counts: impl Fn(&Sample) -> bool) -> bool {
        let reach = (radius / self.cell).ceil() as i64;
        let (column, row) = (self.column(at.x), self.row(at.y));
        let squared = radius * radius;
        for y in (row - reach).max(0)..=(row + reach).min(self.rows as i64 - 1) {
            for x in (column - reach).max(0)..=(column + reach).min(self.columns as i64 - 1) {
                let mut at_item = self.head[y as usize * self.columns + x as usize];
                while at_item != NONE {
                    let (sample, next) = self.items[at_item as usize];
                    let (dx, dy) = (sample.at.x - at.x, sample.at.y - at.y);
                    if dx * dx + dy * dy < squared && counts(&sample) {
                        return true;
                    }
                    at_item = next;
                }
            }
        }
        false
    }
}
