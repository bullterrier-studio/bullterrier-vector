//! The morph step.

use super::Drawing;
use crate::Expr;
use crate::node::{Given, morph};

impl Drawing {
    /// This drawing with every shape morphed `fraction` of the way to the shape at the same index of `target` (a
    /// `Drawing`, a [`Shape`](crate::Shape) or a [`Path`](crate::Path)), as [`Path::morph`](crate::Path::morph) does
    /// for each pair: the shape itself at 0, the target's at 1, the straight lerp beyond. The shapes of `target` are
    /// taken as they are, one for one, never united as the `other` of a boolean is. A paired shape keeps its own style,
    /// attributes and `origin`.
    ///
    /// A shape with no partner, because `target` has fewer shapes, shrinks to its own center and keeps its style; a
    /// shape of `target` with no partner grows from its center in place, with the target shape's style and attributes.
    /// The result has as many shapes as the longer of the two drawings. `fraction` is a number for each shape (a
    /// number, a `Param`, an expression, or a list such as `by_index` or `stagger`), read with the shape's index among
    /// the shapes of the result and its own `order` attribute (the source shape's for a pair, the target shape's for
    /// one that grows), so a shape that grows reads the last indices.
    ///
    /// A `fraction` that is NaN or infinite leaves the drawing with an [`error`](Drawing::error) naming the step
    /// (`step 2 (morph).fraction`), and so does a `target` that holds an error. A `fraction` that reads a Param is
    /// refused when the drawing is drawn, as is one that sends a point beyond ±1e100 or a draw that makes more than
    /// `MAX_SEGMENTS` points.
    #[must_use]
    pub fn morph(&self, target: impl Into<Self>, fraction: impl Into<Expr>) -> Self {
        self.then(
            &morph::Morph,
            vec![
                ("target", Given::Drawing(target.into())),
                ("fraction", Given::Number(fraction.into())),
            ],
        )
    }
}
