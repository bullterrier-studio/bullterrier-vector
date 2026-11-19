//! The offset steps: `offset`, `outline` and `sweep`.

use super::Drawing;
use crate::node::{Given, offset};
use crate::{Cap, Expr, Join};

/// How a [`Drawing::outline`] strokes: the joins, caps, miter limit and tolerance if not the defaults (round, round,
/// 4, 0.01), and a width profile if the width varies. `Outline::new()` alone is the plain round stroke.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Outline {
    join: Option<Join>,
    cap: Option<Cap>,
    miter_limit: Option<Expr>,
    tolerance: Option<f64>,
    profile: Option<Vec<(Expr, Expr)>>,
}

impl Outline {
    /// A round stroke outlined to 0.01 design pixels.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// This outline with the corners that stick out joined as `join`.
    #[must_use]
    pub fn join(mut self, join: Join) -> Self {
        self.join = Some(join);
        self
    }

    /// This outline with the ends of an open line finished as `cap`.
    #[must_use]
    pub fn cap(mut self, cap: Cap) -> Self {
        self.cap = Some(cap);
        self
    }

    /// This outline with a mitre sharper than `miter_limit` (SVG's ratio, at least 1) cut flat: a number, a `Param`
    /// or an expression read for each shape, as [`Stroke::miter_limit`](crate::Stroke::miter_limit).
    #[must_use]
    pub fn miter_limit(mut self, miter_limit: impl Into<Expr>) -> Self {
        self.miter_limit = Some(miter_limit.into());
        self
    }

    /// This outline with curves made straight edges that stray from them by at most `tolerance` design pixels.
    #[must_use]
    pub fn tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = Some(tolerance);
        self
    }

    /// This outline with its width `width` times the factor of `profile` at each place, as
    /// [`Path::outline`](crate::Path::outline) takes a profile: `(position, factor)` stops, each number a number, a
    /// `Param` or an expression read for each shape. Joins and caps are then round (another join or cap leaves the
    /// drawing with an [`error`](Drawing::error)). A profile of no stops or more than 1,000 leaves the drawing with an
    /// error naming the step (`step 2 (outline).profile`); any other refusal of the profile comes when the drawing is
    /// drawn.
    #[must_use]
    pub fn profile<E: Into<Expr>>(mut self, profile: impl IntoIterator<Item = (E, E)>) -> Self {
        self.profile = Some(
            profile
                .into_iter()
                .map(|(at, factor)| (at.into(), factor.into()))
                .collect(),
        );
        self
    }
}

impl Drawing {
    /// This drawing with the edge of the region every shape fills moved outward by `distance` design pixels, inward
    /// when negative, as [`Path::offset`](crate::Path::offset) does. `distance` is a number for each shape: a number,
    /// a `Param`, or a list of them (`by_index`, `Random`) gives each shape its own. Each shape keeps its style and
    /// attributes; a shape that shrinks past nothing keeps an empty path.
    ///
    /// `join` shapes the corners that stick out; a mitre sharper than `miter_limit` (SVG's ratio, at least 1) is cut
    /// flat: a number for each shape, as `distance`. Curves become straight edges that stray from them by at most
    /// `tolerance` design pixels (0.01 is a hundredth of one). `join` and `tolerance` are fixed when the step is
    /// written.
    ///
    /// A `miter_limit` below 1 or a `tolerance` that is not above 0 and within ±1e100 leaves the drawing with an
    /// [`error`](Drawing::error) naming the step (`step 2 (offset).tolerance`); a `distance` or a `miter_limit` the
    /// library refuses that reads a Param is refused when the drawing is drawn. A draw is refused past `MAX_SEGMENTS`.
    #[must_use]
    pub fn offset(&self, distance: impl Into<Expr>, join: Join, miter_limit: impl Into<Expr>, tolerance: f64) -> Self {
        self.then(
            &offset::Offset,
            vec![
                ("distance", Given::Number(distance.into())),
                ("join", Given::Text(join.name().to_owned())),
                ("miter_limit", Given::Number(miter_limit.into())),
                ("tolerance", Given::Number(Expr::from(tolerance))),
            ],
        )
    }

    /// This drawing with every shape turned into the closed outlines a stroke `width` design pixels wide along it
    /// paints, as [`Path::outline`](crate::Path::outline) does: the shape keeps its style and attributes but is now
    /// the region, to fill. `width` is a number for each shape, above 0, as the `distance` of [`Drawing::offset`];
    /// `options` gives the joins, caps, miter limit, tolerance and a width profile, each refused as for `offset` or
    /// [`Outline::profile`].
    #[must_use]
    pub fn outline(&self, width: impl Into<Expr>, options: Outline) -> Self {
        let Outline {
            join,
            cap,
            miter_limit,
            tolerance,
            profile,
        } = options;
        let mut given = vec![("width", Given::Number(width.into()))];
        if let Some(join) = join {
            given.push(("join", Given::Text(join.name().to_owned())));
        }
        if let Some(cap) = cap {
            given.push(("cap", Given::Text(cap.name().to_owned())));
        }
        if let Some(miter_limit) = miter_limit {
            given.push(("miter_limit", Given::Number(miter_limit)));
        }
        if let Some(tolerance) = tolerance {
            given.push(("tolerance", Given::Number(Expr::from(tolerance))));
        }
        if let Some(profile) = profile {
            given.push(("profile", Given::Pairs(profile)));
        }
        self.then(&offset::Outline, given)
    }

    /// This drawing with every shape turned into the region it covers as it moves `dx` to the right and `dy` down in a
    /// straight line, in design pixels, as [`Path::sweep`](crate::Path::sweep) does: `dx` and `dy` are numbers for
    /// each shape, as [`Drawing::translate`]'s; `tolerance` is fixed when the step is written, and refused as for
    /// [`Drawing::offset`]. Each shape keeps its style and attributes (its `origin` is not moved). A draw is refused
    /// past `MAX_SEGMENTS`.
    #[must_use]
    pub fn sweep(&self, dx: impl Into<Expr>, dy: impl Into<Expr>, tolerance: f64) -> Self {
        self.then(
            &offset::Sweep,
            vec![
                ("dx", Given::Number(dx.into())),
                ("dy", Given::Number(dy.into())),
                ("tolerance", Given::Number(Expr::from(tolerance))),
            ],
        )
    }
}
