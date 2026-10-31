//! `Style`: how a shape is painted.

use super::color::Color;
use crate::Path;

/// The width of a stroke when none is given, in design pixels.
// constant: api.md "Style" (`stroke(color, *, width=2, ...)`), which is also the width of the default paint of an
// open line
pub(crate) const DEFAULT_STROKE_WIDTH: f64 = 2.0;

/// How sharp a mitred join may be before it is cut to a bevel, as SVG's ratio, when none is given.
// constant: api.md "Style" (`miter_limit=4`), SVG's own default
pub(crate) const DEFAULT_MITER_LIMIT: f64 = 4.0;

/// The colour of the default paint.
const BLACK: Color = Color::hex(0x00_00_00);

/// How the corners of a stroke are joined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Join {
    /// Rounded.
    Round,
    /// Pointed, cut to a bevel where sharper than the miter limit.
    Miter,
    /// Cut flat.
    Bevel,
}

/// How the ends of an open stroke are finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cap {
    /// Flat at the end.
    Butt,
    /// Rounded past the end.
    Round,
    /// Flat, half a width past the end.
    Square,
}

/// How a shape is painted over what is below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// Over, as ordinary paint.
    Normal,
    /// Multiplied with what is below.
    Multiply,
}

/// A stroke: its colour, width in design pixels and the finish of its corners and ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeStyle {
    pub(crate) color: Color,
    pub(crate) width: f64,
    pub(crate) join: Join,
    pub(crate) cap: Cap,
    pub(crate) miter_limit: f64,
}

impl StrokeStyle {
    /// A stroke of `color` with the default width, joins, caps and miter limit.
    pub(crate) fn new(color: Color) -> Self {
        Self {
            color,
            width: DEFAULT_STROKE_WIDTH,
            join: Join::Round,
            cap: Cap::Round,
            miter_limit: DEFAULT_MITER_LIMIT,
        }
    }

    /// The colour.
    #[must_use]
    pub fn color(&self) -> Color {
        self.color
    }

    /// The width in design pixels.
    #[must_use]
    pub fn width(&self) -> f64 {
        self.width
    }

    /// How corners are joined.
    #[must_use]
    pub fn join(&self) -> Join {
        self.join
    }

    /// How the ends of an open outline are finished.
    #[must_use]
    pub fn cap(&self) -> Cap {
        self.cap
    }

    /// The ratio at which a mitred join is cut to a bevel.
    #[must_use]
    pub fn miter_limit(&self) -> f64 {
        self.miter_limit
    }
}

/// The paint of a shape as it is drawn: `None` and [`Color::NONE`] mean not painted.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub(crate) fill: Color,
    pub(crate) stroke: Option<StrokeStyle>,
    pub(crate) opacity: f64,
    pub(crate) blend: Blend,
    /// Whether a fill or a stroke was set, `fill("none")` included. Until then the default paint applies.
    painted: bool,
}

impl Style {
    /// The style of a shape no step has painted: the default paint is resolved when the drawing is drawn.
    pub(crate) fn unpainted() -> Self {
        Self {
            fill: Color::NONE,
            stroke: None,
            opacity: 1.0,
            blend: Blend::Normal,
            painted: false,
        }
    }

    /// This style with the default paint of `path` if nothing was painted: a black fill when every sub-path is
    /// closed (an empty path is filled and draws nothing), else a black stroke of the default width. Afterwards it
    /// counts as painted, so a shape put back into a drawing is drawn as it was.
    pub(crate) fn resolve_paint(&mut self, path: &Path) {
        if !self.painted {
            if path.is_closed() {
                self.fill = BLACK;
            } else {
                self.stroke = Some(StrokeStyle::new(BLACK));
            }
            self.painted = true;
        }
    }

    /// The fill, [`Color::NONE`] where the shape is not filled.
    #[must_use]
    pub fn fill(&self) -> Color {
        self.fill
    }

    /// The stroke, `None` where the shape is not stroked.
    #[must_use]
    pub fn stroke(&self) -> Option<&StrokeStyle> {
        self.stroke.as_ref()
    }

    /// The opacity from 0 to 1, the product of every `opacity` step.
    #[must_use]
    pub fn opacity(&self) -> f64 {
        self.opacity
    }

    /// How it is painted over what is below.
    #[must_use]
    pub fn blend(&self) -> Blend {
        self.blend
    }
}
