//! The offset steps: `offset` and `outline`.

use super::Drawing;
use crate::node::{Given, offset};
use crate::{Cap, Expr, Join};

impl Drawing {
    /// This drawing with the edge of the region every shape fills moved outward by `distance` design pixels, inward
    /// when negative, as [`Path::offset`](crate::Path::offset) does. `distance` is a number for each shape: a number,
    /// a `Param`, or a list of them (`by_index`, `Random`) gives each shape its own. Each shape keeps its style and
    /// attributes; a shape that shrinks past nothing keeps an empty path.
    ///
    /// `join` shapes the corners that stick out; a mitre sharper than `miter_limit` (SVG's ratio, at least 1) is cut
    /// flat. Curves become straight edges that stray from them by at most `tolerance` design pixels (0.01 is a
    /// hundredth of one). `join`, `miter_limit` and `tolerance` are fixed when the step is written.
    ///
    /// A `miter_limit` below 1 or a `tolerance` that is not above 0 and within ±1e100 leaves the drawing with an
    /// [`error`](Drawing::error) naming the step (`step 2 (offset).tolerance`); a `distance` the library refuses is
    /// refused when the drawing is drawn. A draw is refused past `MAX_SEGMENTS`.
    #[must_use]
    pub fn offset(&self, distance: impl Into<Expr>, join: Join, miter_limit: f64, tolerance: f64) -> Self {
        self.then(
            &offset::Offset,
            vec![
                ("distance", Given::Number(distance.into())),
                ("join", Given::Text(join.name().to_owned())),
                ("miter_limit", Given::Number(Expr::from(miter_limit))),
                ("tolerance", Given::Number(Expr::from(tolerance))),
            ],
        )
    }

    /// This drawing with every shape turned into the closed outlines a stroke `width` design pixels wide along it
    /// paints, as [`Path::outline`](crate::Path::outline) does: the shape keeps its style and attributes but is now
    /// the region, to fill. `width` is a number for each shape, above 0, as the `distance` of [`Drawing::offset`];
    /// `join`, `cap`, `miter_limit` and `tolerance` are fixed when the step is written, and refused as for it.
    #[must_use]
    pub fn outline(&self, width: impl Into<Expr>, join: Join, cap: Cap, miter_limit: f64, tolerance: f64) -> Self {
        self.then(
            &offset::Outline,
            vec![
                ("width", Given::Number(width.into())),
                ("join", Given::Text(join.name().to_owned())),
                ("cap", Given::Text(cap.name().to_owned())),
                ("miter_limit", Given::Number(Expr::from(miter_limit))),
                ("tolerance", Given::Number(Expr::from(tolerance))),
            ],
        )
    }
}
