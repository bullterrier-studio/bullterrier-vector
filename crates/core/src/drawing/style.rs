//! The style steps of a drawing: `fill`, `stroke`, `opacity`, `blend`, `shadow`, `knockout`; and the same on a `Path`.

use super::Drawing;
use crate::node::{Given, style};
use crate::{Blend, Cap, Color, Expr, Join, Path};

/// A stroke: its colour, and the width, joins, caps and miter limit if not the defaults (2 pixels, round, round, 4). A
/// [`Color`] alone converts into one.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    color: Color,
    width: Option<Expr>,
    join: Option<Join>,
    cap: Option<Cap>,
    miter_limit: Option<Expr>,
}

impl Stroke {
    /// A stroke of `color`.
    #[must_use]
    pub fn new(color: Color) -> Self {
        Self {
            color,
            width: None,
            join: None,
            cap: None,
            miter_limit: None,
        }
    }

    /// This stroke `width` design pixels wide, above 0.
    #[must_use]
    pub fn width(mut self, width: impl Into<Expr>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// This stroke with its corners joined as `join`.
    #[must_use]
    pub fn join(mut self, join: Join) -> Self {
        self.join = Some(join);
        self
    }

    /// This stroke with the ends of an open outline finished as `cap`.
    #[must_use]
    pub fn cap(mut self, cap: Cap) -> Self {
        self.cap = Some(cap);
        self
    }

    /// This stroke with a mitred join cut to a bevel past `miter_limit` (SVG's ratio), at least 1.
    #[must_use]
    pub fn miter_limit(mut self, miter_limit: impl Into<Expr>) -> Self {
        self.miter_limit = Some(miter_limit.into());
        self
    }
}

/// A drop shadow: what is left out is the default, no offset to the side, 8 pixels down, a blur of 16 and the colour
/// `#00000059`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shadow {
    dx: Option<Expr>,
    dy: Option<Expr>,
    blur: Option<Expr>,
    color: Option<Color>,
}

impl Shadow {
    /// A shadow with the default offset, blur and colour.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// This shadow `dx` design pixels to the right of its shape.
    #[must_use]
    pub fn dx(mut self, dx: impl Into<Expr>) -> Self {
        self.dx = Some(dx.into());
        self
    }

    /// This shadow `dy` design pixels below its shape.
    #[must_use]
    pub fn dy(mut self, dy: impl Into<Expr>) -> Self {
        self.dy = Some(dy.into());
        self
    }

    /// This shadow blurred by `blur` design pixels, at least 0: the radius of a CSS `box-shadow`, so the Gaussian's
    /// standard deviation is half of it. 0 is a hard copy.
    #[must_use]
    pub fn blur(mut self, blur: impl Into<Expr>) -> Self {
        self.blur = Some(blur.into());
        self
    }

    /// This shadow tinted `color`, which paints (not [`Color::NONE`]); its alpha is the shadow's strength.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl From<Color> for Stroke {
    fn from(color: Color) -> Self {
        Self::new(color)
    }
}

impl Drawing {
    /// This drawing with every shape filled with `color` ([`Color::NONE`] for no fill). Once a fill or a stroke is
    /// set, only what is set is painted.
    #[must_use]
    pub fn fill(&self, color: Color) -> Self {
        self.then(&style::Fill, vec![("color", Given::Color(color))])
    }

    /// This drawing with every outline stroked: a [`Color`] for the default stroke, or a [`Stroke`].
    ///
    /// A colour of none, a width not above 0 and a miter limit below 1 leave the drawing with an
    /// [`error`](Drawing::error), or, when they read a Param, are refused when the drawing is drawn.
    #[must_use]
    pub fn stroke(&self, stroke: impl Into<Stroke>) -> Self {
        let stroke = stroke.into();
        let mut given = vec![("color", Given::Color(stroke.color))];
        if let Some(width) = stroke.width {
            given.push(("width", Given::Number(width)));
        }
        if let Some(join) = stroke.join {
            given.push(("join", Given::Text(join.name().to_owned())));
        }
        if let Some(cap) = stroke.cap {
            given.push(("cap", Given::Text(cap.name().to_owned())));
        }
        if let Some(limit) = stroke.miter_limit {
            given.push(("miter_limit", Given::Number(limit)));
        }
        self.then(&style::Stroke, given)
    }

    /// This drawing with the opacity of every shape multiplied by `alpha`, from 0 to 1.
    #[must_use]
    pub fn opacity(&self, alpha: impl Into<Expr>) -> Self {
        self.then(&style::Opacity, vec![("alpha", Given::Number(alpha.into()))])
    }

    /// This drawing with every shape painted over what is below it as `mode`.
    #[must_use]
    pub fn blend(&self, mode: Blend) -> Self {
        self.then(&style::Blend, vec![("mode", Given::Text(mode.name().to_owned()))])
    }

    /// This drawing with a drop shadow under every shape, replacing any it has: the silhouette of the shape's fill and
    /// stroke, moved, blurred and tinted, painted behind the shape inside the group that its opacity and blend apply to.
    /// A shape with an empty path or nothing painted casts none.
    ///
    /// A `blur` below 0 or a colour of none leave the drawing with an [`error`](Drawing::error), or, when they read a
    /// Param, are refused when the drawing is drawn.
    #[must_use]
    pub fn shadow(&self, shadow: Shadow) -> Self {
        let mut given = Vec::new();
        for (name, number) in [("dx", shadow.dx), ("dy", shadow.dy), ("blur", shadow.blur)] {
            if let Some(number) = number {
                given.push((name, Given::Number(number)));
            }
        }
        if let Some(color) = shadow.color {
            given.push(("color", Given::Color(color)));
        }
        self.then(&style::Shadow, given)
    }

    /// This drawing with every shape removing what is below it in the drawing instead of painting: the area its fill
    /// and stroke cover is cut out of the shapes before it, their shadows included; the shapes after it paint over the
    /// hole. A shape that paints nothing, or has an empty path, removes nothing.
    #[must_use]
    pub fn knockout(&self) -> Self {
        self.then(&style::Knockout, Vec::new())
    }
}

impl Path {
    /// A [`Drawing`] of this path filled with `color`.
    #[must_use]
    pub fn fill(&self, color: Color) -> Drawing {
        Drawing::from(self.clone()).fill(color)
    }

    /// A [`Drawing`] of this path stroked.
    #[must_use]
    pub fn stroke(&self, stroke: impl Into<Stroke>) -> Drawing {
        Drawing::from(self.clone()).stroke(stroke)
    }

    /// A [`Drawing`] of this path with its opacity multiplied by `alpha`, from 0 to 1.
    #[must_use]
    pub fn opacity(&self, alpha: impl Into<Expr>) -> Drawing {
        Drawing::from(self.clone()).opacity(alpha)
    }

    /// A [`Drawing`] of this path painted over what is below it as `mode`.
    #[must_use]
    pub fn blend(&self, mode: Blend) -> Drawing {
        Drawing::from(self.clone()).blend(mode)
    }

    /// A [`Drawing`] of this path with a drop shadow.
    #[must_use]
    pub fn shadow(&self, shadow: Shadow) -> Drawing {
        Drawing::from(self.clone()).shadow(shadow)
    }

    /// A [`Drawing`] of this path as a knockout.
    #[must_use]
    pub fn knockout(&self) -> Drawing {
        Drawing::from(self.clone()).knockout()
    }
}
