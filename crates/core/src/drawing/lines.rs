//! The line-family steps.

use super::Drawing;
use crate::node::{Given, lines};
use crate::{Bands, Expr, Rows};

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

    /// This drawing with every shape replaced by the closed outlines `bands` asks for inside the region it fills, as
    /// [`Path::bands`](crate::Path::bands) makes them for each shape's path; the shape keeps its style, attributes and
    /// `origin`. The outlines are for filling, so set the fill after this step.
    ///
    /// A refused number leaves the drawing with an [`error`](Drawing::error) naming the step. Bands that would pass
    /// `MAX_SEGMENTS` points are refused when the drawing is drawn.
    #[must_use]
    pub fn bands(&self, bands: &Bands) -> Self {
        let number = |expr: Option<&Expr>| expr.map_or(Given::Unset, |expr| Given::Number(expr.clone()));
        let (thin, thick) = bands.width_of();
        self.then(
            &lines::BandsStep,
            vec![
                ("spacing", Given::Number(bands.spacing_of().clone())),
                ("tone", Given::Field(bands.tone_of().clone())),
                ("width", Given::Pair(thin.clone(), thick.clone())),
                ("family", Given::Text(bands.family().name().to_owned())),
                ("angle", Given::Number(bands.angle_of().clone())),
                ("wavelength", number(bands.wavelength_of())),
                ("amplitude", number(bands.amplitude_of())),
                (
                    "centre",
                    bands
                        .centre_of()
                        .map_or(Given::Unset, |(x, y)| Given::Pair(x.clone(), y.clone())),
                ),
            ],
        )
    }
}
