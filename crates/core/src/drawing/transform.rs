//! The geometry steps of a drawing: `translate`, `rotate`, `scale`, `along`, `slice`.

use super::Drawing;
use crate::node::{Given, transform};
use crate::{Expr, Path, Point};

/// A number of degrees, clockwise on the screen: anything that converts into an [`Expr`] (a number, a `Param`, an
/// expression) can be an angle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Deg<T>(pub T);

/// A point a step turns or scales about, or starts from: a [`Point`] or an `(x, y)` pair of anything that converts into an [`Expr`].
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
    pub(super) fn given(pivot: Option<Self>) -> Given {
        pivot.map_or(Given::Unset, |pivot| Given::Pair(pivot.x, pivot.y))
    }
}

/// A turn: the angle, and the point to turn about if not each shape's own origin. `Deg(30.0)` alone converts into one.
#[derive(Debug, Clone, PartialEq)]
pub struct Rotate {
    angle: Expr,
    about: Option<Pivot>,
}

impl Rotate {
    /// A turn by `angle` degrees about each shape's origin (the middle of its bounds if it has none).
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
/// shape's own origin. A number or a pair alone converts into one.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaleBy {
    x: Expr,
    y: Expr,
    about: Option<Pivot>,
}

impl ScaleBy {
    /// Scaling by `by` about each shape's origin (the middle of its bounds if it has none).
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

/// How shapes are carried along a guide: where on it they start, and from which point of a closed guide it is walked.
/// `Along::new()` alone starts at the beginning of the guide.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Along {
    start: Option<Expr>,
    near: Option<Pivot>,
}

impl Along {
    /// Shapes carried from the beginning of the guide.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// This bend starting `start` guide lengths along: a number, a `Param` or an expression, read for each shape
    /// (a `Param` given a list moves each shape to its own place). 1 is the whole guide; a closed guide wraps.
    #[must_use]
    pub fn start(mut self, start: impl Into<Expr>) -> Self {
        self.start = Some(start.into());
        self
    }

    /// This bend walking a closed guide from the point of it nearest `point`, not from its top.
    #[must_use]
    pub fn near(mut self, point: impl Into<Pivot>) -> Self {
        self.near = Some(point.into());
        self
    }
}

impl Drawing {
    /// This drawing with every shape, and its `origin` attribute, moved `dx` to the right and `dy` down, in design pixels.
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

    /// This drawing with every shape turned clockwise on the screen (`y` down): `Deg(30.0)` about each shape's `origin`
    /// attribute (the middle of its bounds if it has none), or a [`Rotate`] with the point to turn about. The `origin`
    /// turns with the shape. Any angle is accepted and wraps; multiples of 90 degrees are exact.
    #[must_use]
    pub fn rotate(&self, turn: impl Into<Rotate>) -> Self {
        let Rotate { angle, about } = turn.into();
        self.then(
            &transform::Rotate,
            vec![("angle", Given::Number(angle)), ("about", Pivot::given(about))],
        )
    }

    /// This drawing with every shape scaled by a number, or an `(x, y)` pair, or a [`ScaleBy`] with the point to
    /// scale about (by default each shape's `origin`, else the middle of its bounds), which moves the `origin` with the
    /// shape. A negative factor mirrors.
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

    /// This drawing with every shape carried along `guide` as [`Path::along`] does, text included: a text becomes
    /// text on a path, each glyph bent with the rest. The shapes keep their style and attributes (`origin` stays the
    /// glyph's centre before the bend).
    ///
    /// A `start` that is not a finite number leaves the drawing with an [`error`](Drawing::error), or, when it
    /// reads a Param, is refused when the drawing is drawn; so is a guide that is empty or has no length.
    #[must_use]
    pub fn along(&self, guide: &Path, options: Along) -> Self {
        let mut given = vec![
            ("path", Given::Path(guide.clone())),
            ("near", Pivot::given(options.near)),
        ];
        if let Some(start) = options.start {
            given.push(("start", Given::Number(start)));
        }
        self.then(&transform::Along, given)
    }

    /// This drawing with every shape cut to the piece of its outline between the positions `start` and `end`
    /// (fractions 0 to 1 of its length, as [`Path::slice`] does): `slice(0, progress)` draws it on. A number, a
    /// `Param` or an expression, read for each shape, so a per-item number draws the shapes on one after another:
    /// `text.slice(0, Stagger::new(&progress).spread(0.6))`. A closed outline is measured from its top. A shape with
    /// nothing in it stays as it is.
    ///
    /// A position outside 0 to 1 leaves the drawing with an [`error`](Drawing::error) naming the step
    /// (`step 2 (slice).end`); so does a number that reads a Param, when the drawing is drawn, and a `start` after
    /// the `end` of its shape.
    #[must_use]
    pub fn slice(&self, start: impl Into<Expr>, end: impl Into<Expr>) -> Self {
        self.then(
            &transform::Slice,
            vec![
                ("start", Given::Number(start.into())),
                ("end", Given::Number(end.into())),
            ],
        )
    }

    /// This drawing with every shape replaced by straight segments with corners `spacing` design pixels apart along
    /// it, as [`Path::resample`](crate::Path::resample) does: the corners are where dots, or the points a deformer
    /// moves, go. `spacing` is a number for each shape (a number, a `Param`, or a list of them). Each shape keeps its
    /// style and attributes, its `origin` included.
    ///
    /// A `spacing` not above 0 and within 1e100 leaves the drawing with an [`error`](Drawing::error) naming the
    /// step (`step 2 (resample).spacing`), or, when it reads a Param, is refused when the drawing is drawn; so is a
    /// draw that would make more than `MAX_SEGMENTS` corners.
    #[must_use]
    pub fn resample(&self, spacing: impl Into<Expr>) -> Self {
        self.then(&transform::Resample, vec![("spacing", Given::Number(spacing.into()))])
    }

    /// This drawing with the points of every shape displaced sideways by a sine, as [`Path::wave`](crate::Path::wave)
    /// does: `amplitude` design pixels, one `wavelength` long, travelling towards `angle` (`Deg(0.0)` is towards +x
    /// and displaces down the screen), `phase` in cycles (a `Param` going 0 to 1 sets the wave travelling). Each is a
    /// number for each shape (a number, a `Param`, an expression or a list of them). The wave is anchored in the design
    /// frame: the letters of one word waved in one step share one wave. Each shape keeps its style and attributes, its
    /// `origin` included.
    ///
    /// A `wavelength` not above 0 and within 1e100, or a number that is NaN or infinite, leaves the drawing with an
    /// [`error`](Drawing::error) naming the step (`step 2 (wave).wavelength`), or, when it reads a Param, is refused
    /// when the drawing is drawn; so is a draw that would make more than `MAX_SEGMENTS` points.
    #[must_use]
    pub fn wave<A: Into<Expr>>(
        &self,
        amplitude: impl Into<Expr>,
        wavelength: impl Into<Expr>,
        angle: Deg<A>,
        phase: impl Into<Expr>,
    ) -> Self {
        self.then(
            &transform::Wave,
            vec![
                ("amplitude", Given::Number(amplitude.into())),
                ("wavelength", Given::Number(wavelength.into())),
                ("angle", Given::Number(angle.0.into())),
                ("phase", Given::Number(phase.into())),
            ],
        )
    }

    /// This drawing with every shape run the other way, as [`Path::reverse`](crate::Path::reverse) does: `text(..)
    /// .reverse().slice(0, progress)` draws on from the other end, and a stroke's direction decides where a `dash` or
    /// a taper begins. Each shape keeps its style and attributes, its `origin` included. Takes no arguments, so it
    /// cannot fail.
    #[must_use]
    pub fn reverse(&self) -> Self {
        self.then(&transform::Reverse, Vec::new())
    }

    /// This drawing with every shape replaced by few smooth curves within `tolerance` design pixels of it, as
    /// [`Path::simplify`](crate::Path::simplify) does: a dense polyline becomes a few Beziers, sharp corners kept.
    /// `tolerance` is a number for each shape (a number, a `Param`, or a list of them). Each shape keeps its style
    /// and attributes, its `origin` included.
    ///
    /// A `tolerance` not above 0 and within 1e100 leaves the drawing with an [`error`](Drawing::error) naming the
    /// step (`step 2 (simplify).tolerance`), or, when it reads a Param, is refused when the drawing is drawn; so is
    /// a draw that would flatten more than `MAX_SEGMENTS` edges.
    #[must_use]
    pub fn simplify(&self, tolerance: impl Into<Expr>) -> Self {
        self.then(
            &transform::Simplify,
            vec![("tolerance", Given::Number(tolerance.into()))],
        )
    }

    /// This drawing with every shape cut into the pieces a dash `pattern` (`[on, off, ...]` in design pixels) leaves, as
    /// [`Path::dash`](crate::Path::dash) does. Each number of `pattern` and the `phase` (in cycles of the pattern, which
    /// moves the dashes forward along the shape) is a number, a `Param` or an expression, read for each shape:
    /// marching ants are `drawing.dash([10.0, 6.0], &phase)`. A closed outline is measured from its top. Each shape
    /// keeps its style and attributes.
    ///
    /// A pattern that is empty, has more than 1,000 lengths or a length not above 0 and within 1e100 leaves the
    /// drawing with an [`error`](Drawing::error) naming the step (`step 2 (dash).pattern`), or, when it reads a Param, is
    /// refused when the drawing is drawn; so is a draw that would make more than `MAX_SEGMENTS` pieces.
    #[must_use]
    pub fn dash<E: Into<Expr>>(&self, pattern: impl IntoIterator<Item = E>, phase: impl Into<Expr>) -> Self {
        self.then(
            &transform::Dash,
            vec![
                ("pattern", Given::Numbers(pattern.into_iter().map(Into::into).collect())),
                ("phase", Given::Number(phase.into())),
            ],
        )
    }

    /// This drawing dashed as [`Drawing::dash`] does, with a closed outline measured from its point nearest `near`
    /// instead of its top.
    #[must_use]
    pub fn dash_near<E: Into<Expr>>(
        &self,
        pattern: impl IntoIterator<Item = E>,
        phase: impl Into<Expr>,
        near: impl Into<Pivot>,
    ) -> Self {
        self.then(
            &transform::Dash,
            vec![
                ("pattern", Given::Numbers(pattern.into_iter().map(Into::into).collect())),
                ("phase", Given::Number(phase.into())),
                ("near", Pivot::given(Some(near.into()))),
            ],
        )
    }

    /// This drawing sliced as [`Drawing::slice`] does, with a closed outline measured from its point nearest `near`
    /// instead of its top.
    #[must_use]
    pub fn slice_near(&self, start: impl Into<Expr>, end: impl Into<Expr>, near: impl Into<Pivot>) -> Self {
        self.then(
            &transform::Slice,
            vec![
                ("start", Given::Number(start.into())),
                ("end", Given::Number(end.into())),
                ("near", Pivot::given(Some(near.into()))),
            ],
        )
    }
}
