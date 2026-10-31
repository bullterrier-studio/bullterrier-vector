//! The geometry steps of a drawing: `translate`, `rotate`, `scale`.

use super::Drawing;
use crate::node::{Given, transform};
use crate::{Expr, Point};

/// A number of degrees, clockwise on the screen: anything that converts into an [`Expr`] (a number, a `Param`, an
/// expression) can be an angle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Deg<T>(pub T);

/// A point a step turns or scales about: a [`Point`] or an `(x, y)` pair of anything that converts into an [`Expr`].
#[derive(Debug, Clone, PartialEq)]
pub struct Pivot {
    x: Expr,
    y: Expr,
}

impl From<Point> for Pivot {
    fn from(point: Point) -> Self {
        Self {
            x: Expr::from(point.x),
            y: Expr::from(point.y),
        }
    }
}

impl<X: Into<Expr>, Y: Into<Expr>> From<(X, Y)> for Pivot {
    fn from((x, y): (X, Y)) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
        }
    }
}

impl Pivot {
    fn given(pivot: Option<Self>) -> Given {
        pivot.map_or(Given::Unset, |pivot| Given::Pair(pivot.x, pivot.y))
    }
}

/// A turn: the angle, and the point to turn about if not each shape's own centre. `Deg(30.0)` alone converts into one.
#[derive(Debug, Clone, PartialEq)]
pub struct Rotate {
    angle: Expr,
    about: Option<Pivot>,
}

impl Rotate {
    /// A turn by `angle` degrees about the middle of each shape's bounds.
    #[must_use]
    pub fn new<T: Into<Expr>>(angle: Deg<T>) -> Self {
        Self {
            angle: angle.0.into(),
            about: None,
        }
    }

    /// This turn about `point` instead.
    #[must_use]
    pub fn about(mut self, point: impl Into<Pivot>) -> Self {
        self.about = Some(point.into());
        self
    }
}

impl<T: Into<Expr>> From<Deg<T>> for Rotate {
    fn from(angle: Deg<T>) -> Self {
        Self::new(angle)
    }
}

/// A scaling: the factor, a number for both directions or an `(x, y)` pair, and the point to scale about if not each
/// shape's own centre. A number or a pair alone converts into one.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaleBy {
    x: Expr,
    y: Expr,
    about: Option<Pivot>,
}

impl ScaleBy {
    /// Scaling by `by` about the middle of each shape's bounds.
    #[must_use]
    pub fn new(by: impl Into<Self>) -> Self {
        by.into()
    }

    /// This scaling about `point` instead.
    #[must_use]
    pub fn about(mut self, point: impl Into<Pivot>) -> Self {
        self.about = Some(point.into());
        self
    }
}

impl<T: Into<Expr>> From<T> for ScaleBy {
    fn from(by: T) -> Self {
        let by = by.into();
        Self {
            x: by.clone(),
            y: by,
            about: None,
        }
    }
}

impl<X: Into<Expr>, Y: Into<Expr>> From<(X, Y)> for ScaleBy {
    fn from((x, y): (X, Y)) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            about: None,
        }
    }
}

impl Drawing {
    /// This drawing with every shape moved `dx` to the right and `dy` down, in design pixels.
    ///
    /// A number that is not accepted (NaN, infinite, beyond ±1e100) leaves the drawing with an
    /// [`error`](Drawing::error) naming the step (`step 2 (translate).dx`); one that reads a Param is checked when
    /// the drawing is drawn.
    #[must_use]
    pub fn translate(&self, dx: impl Into<Expr>, dy: impl Into<Expr>) -> Self {
        self.then(
            &transform::Translate,
            vec![("dx", Given::Number(dx.into())), ("dy", Given::Number(dy.into()))],
        )
    }

    /// This drawing with every shape turned clockwise on the screen (`y` down): `Deg(30.0)` about the middle of each
    /// shape's bounds, or a [`Rotate`] with the point to turn about. Any angle is accepted and wraps; multiples of 90
    /// degrees are exact.
    #[must_use]
    pub fn rotate(&self, turn: impl Into<Rotate>) -> Self {
        let Rotate { angle, about } = turn.into();
        self.then(
            &transform::Rotate,
            vec![("angle", Given::Number(angle)), ("about", Pivot::given(about))],
        )
    }

    /// This drawing with every shape scaled by a number, or an `(x, y)` pair, or a [`ScaleBy`] with the point to
    /// scale about. A negative factor mirrors.
    ///
    /// A factor of 0 (or a component) leaves the drawing with an [`error`](Drawing::error), or, when it reads a Param,
    /// is refused when the drawing is drawn.
    #[must_use]
    pub fn scale(&self, by: impl Into<ScaleBy>) -> Self {
        let ScaleBy { x, y, about } = by.into();
        self.then(
            &transform::Scale,
            vec![("by", Given::Pair(x, y)), ("about", Pivot::given(about))],
        )
    }
}
