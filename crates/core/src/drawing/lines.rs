//! The line-family steps.

use super::Drawing;
use crate::Rows;
use crate::node::{Given, lines};

impl Drawing {
    /// This drawing with every shape replaced by the ridgelines `rows` asks for inside the region it fills, as
    /// [`Path::rows`](crate::Path::rows) makes them for each shape's path; the shape keeps its style, attributes and
    /// `origin`. The lines are open, so set the stroke after this step.
    ///
    /// A refused number leaves the drawing with an [`error`](Drawing::error) naming the step. Rows that would pass
    /// `MAX_SEGMENTS` points are refused when the drawing is drawn.
    #[must_use]
    pub fn rows(&self, rows: &Rows) -> Self {
        self.then(
            &lines::RowsStep,
            vec![
                ("spacing", Given::Number(rows.spacing_of().clone())),
                ("tone", Given::Field(rows.tone_of().clone())),
                ("amplitude", Given::Number(rows.amplitude_of().clone())),
            ],
        )
    }
}
