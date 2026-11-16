//! The typed step `place_along`.

use super::Drawing;
use super::transform::Pivot;
use crate::node::{Given, place};
use crate::{Expr, Path, Spacing};

/// Options of [`Drawing::place_along`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlaceAlong {
    start: Option<Expr>,
    near: Option<Pivot>,
    still: bool,
}

impl PlaceAlong {
    /// Copies from the start of the path, turned to its direction.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Start `start` path lengths along.
    #[must_use]
    pub fn start(mut self, start: impl Into<Expr>) -> Self {
        self.start = Some(start.into());
        self
    }
    /// Walk a closed path from the point nearest `point`.
    #[must_use]
    pub fn near(mut self, point: impl Into<Pivot>) -> Self {
        self.near = Some(point.into());
        self
    }
    /// Whether copies turn to the path's direction.
    #[must_use]
    pub fn turn(mut self, turn: bool) -> Self {
        self.still = !turn;
        self
    }
}

impl Drawing {
    /// This drawing with a copy of all its shapes at each stop of `spacing` along `guide` (a [`Path`]), as a stamp is
    /// pressed: the copy's shapes keep their layout, the middle of their bounds (or the origin they all share) goes to
    /// the stop, and the copy is turned so its x axis runs along the guide there, unless [`PlaceAlong::turn`] is
    /// `false`. Nothing is bent ([`Drawing::along`] bends). Each shape keeps its style and attributes, its `origin`
    /// carried with it, and gets `copy` (the stop, from 0); a drawing of one shape also gets as `order` the share
    /// 0 to 1 of its copy, so `by_order` and `by_index` vary per copy.
    ///
    /// [`Spacing::Count`] and [`Spacing::Every`] are fixed when the step is written (the number of shapes does not
    /// depend on a Param); `start`, a number, a `Param` or an expression, moves the copies on by that share of the
    /// guide's length (it wraps on a closed guide, and goes on straight past an open one's ends).
    ///
    /// A count of 0, or a spacing not above 0 and within 1e100, leaves the drawing with an
    /// [`error`](Drawing::error) naming the step (`step 2 (place_along).count`); a guide that is empty or has no
    /// length is refused when drawn, and so is a draw that would make more than `MAX_SHAPES` shapes or
    /// `MAX_SEGMENTS` segments (checked before any copy is made).
    #[must_use]
    pub fn place_along(&self, guide: &Path, spacing: Spacing, options: PlaceAlong) -> Self {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count past 2^53 is refused by the limits anyway"
        )]
        let mut given = vec![
            ("path", Given::Path(guide.clone())),
            ("near", Pivot::given(options.near)),
            match spacing {
                Spacing::Count(count) => ("count", Given::Number(Expr::from(count as f64))),
                Spacing::Every(every) => ("spacing", Given::Number(Expr::from(every))),
            },
        ];
        if let Some(start) = options.start {
            given.push(("start", Given::Number(start)));
        }
        if options.still {
            given.push(("turn", Given::Text(String::from("none"))));
        }
        self.then(&place::PlaceAlong, given)
    }
}
