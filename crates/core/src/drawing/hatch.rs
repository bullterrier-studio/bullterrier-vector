//! The hatch step.

use super::Drawing;
use crate::node::{Given, hatch};
use crate::{Expr, Hatch};

impl Drawing {
    /// This drawing with every shape replaced by the lines `hatch` asks for inside the region it fills, as
    /// [`Path::hatch`](crate::Path::hatch) makes them for each shape's path; the shape keeps its style, attributes and
    /// `origin`. Lines never enter the shapes of `avoid`. The lines are open (closed where they come round to their
    /// own start), so set the stroke after this step.
    ///
    /// A refused spacing, or an `avoid` that holds an error, leaves the drawing with an [`error`](Drawing::error)
    /// naming the step. A hatch that would pass `MAX_SEGMENTS` points is refused when the drawing is drawn.
    #[must_use]
    pub fn hatch(&self, hatch: &Hatch, avoid: Option<&Self>) -> Self {
        let (zero, one) = hatch.spacings();
        self.then(
            &hatch::HatchStep,
            vec![
                ("spacing", Given::Pair(Expr::from(zero), Expr::from(one))),
                (
                    "tone",
                    hatch.tone().map_or(Given::Unset, |tone| Given::Field(tone.clone())),
                ),
                ("flow", Given::Flow(hatch.flow_of().clone())),
                (
                    "below",
                    hatch
                        .below_of()
                        .map_or(Given::Unset, |level| Given::Number(level.clone())),
                ),
                (
                    "avoid",
                    avoid.map_or(Given::Unset, |drawing| Given::Drawing(drawing.clone())),
                ),
            ],
        )
    }
}
