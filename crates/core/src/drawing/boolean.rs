//! The steps that combine shapes as regions: `union`, `subtract`, `intersect`, `exclude` and `resolve`.

use super::Drawing;
use crate::Expr;
use crate::node::{Given, Node, boolean};

impl Drawing {
    /// One of the booleans that take another drawing, with `tolerance` for the edges curves become.
    fn combined(&self, node: &'static dyn Node, other: impl Into<Self>, tolerance: f64) -> Self {
        self.then(
            node,
            vec![
                ("other", Given::Drawing(other.into())),
                ("tolerance", Given::Number(Expr::from(tolerance))),
            ],
        )
    }

    /// This drawing with every shape filling the region it and `other` (a `Drawing` or a [`Path`](crate::Path)) fill
    /// together, as [`Path::union`](crate::Path::union) does. `other` is all its shapes at once: they are united first,
    /// each on its own, so they may overlap and run either way round. Each shape keeps its style and attributes. Curves
    /// become straight edges that stray from them by at most `tolerance` design pixels (0.01 is a hundredth of one).
    ///
    /// A `tolerance` that is not above 0 and within ±1e100 leaves the drawing with an [`error`](Drawing::error)
    /// naming the step (`step 2 (union).tolerance`), and so does an `other` that holds an error. The work grows with
    /// the places the outlines cross, which the limits do not bound; a draw is refused past `MAX_SEGMENTS`.
    #[must_use]
    pub fn union(&self, other: impl Into<Self>, tolerance: f64) -> Self {
        self.combined(&boolean::Union, other, tolerance)
    }

    /// This drawing with every shape filling the region it fills and `other` does not, as
    /// [`Path::subtract`](crate::Path::subtract) does; `other` as for [`Drawing::union`].
    #[must_use]
    pub fn subtract(&self, other: impl Into<Self>, tolerance: f64) -> Self {
        self.combined(&boolean::Subtract, other, tolerance)
    }

    /// This drawing with every shape filling the region it and `other` both fill, as
    /// [`Path::intersect`](crate::Path::intersect) does: a shape that meets none of `other` keeps an empty path.
    /// `other` as for [`Drawing::union`].
    #[must_use]
    pub fn intersect(&self, other: impl Into<Self>, tolerance: f64) -> Self {
        self.combined(&boolean::Intersect, other, tolerance)
    }

    /// This drawing with every shape filling the region only one of it and `other` fills, as
    /// [`Path::exclude`](crate::Path::exclude) does; `other` as for [`Drawing::union`].
    #[must_use]
    pub fn exclude(&self, other: impl Into<Self>, tolerance: f64) -> Self {
        self.combined(&boolean::Exclude, other, tolerance)
    }

    /// This drawing with the overlaps of every shape removed, as [`Path::resolve`](crate::Path::resolve) does: the
    /// shape becomes the closed polygons it fills, an outline that crosses itself cut into the pieces it encloses.
    /// `tolerance` as for [`Drawing::union`].
    #[must_use]
    pub fn resolve(&self, tolerance: f64) -> Self {
        self.then(
            &boolean::Resolve,
            vec![("tolerance", Given::Number(Expr::from(tolerance)))],
        )
    }
}
