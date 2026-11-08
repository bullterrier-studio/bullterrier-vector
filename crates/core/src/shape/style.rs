//! `Style`: how a shape is painted.

use super::color::Color;
use crate::key::{Key, Keyed};
use crate::{Error, Path};

/// The width of a stroke when none is given, in design pixels.
// constant: api.md "Style" (`stroke(color, *, width=2, ...)`), which is also the width of the default paint of an
// open line
pub(crate) const DEFAULT_STROKE_WIDTH: f64 = 2.0;

/// How sharp a mitred join may be before it is cut to a bevel, as SVG's ratio, when none is given.
// constant: api.md "Style" (`miter_limit=4`), SVG's own default
pub(crate) const DEFAULT_MITER_LIMIT: f64 = 4.0;

/// The colour of the default paint.
const BLACK: Color = Color::hex(0x00_00_00);

/// The refusal of `text` for the argument `argument`, which takes one of `words`.
pub(crate) fn refused(argument: &str, words: &[&str], text: &str) -> Error {
    let listed = words
        .iter()
        .map(|word| format!("{word:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    Error::parameter(argument, format!("must be one of {listed}, got {text:?}"))
}

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

impl Join {
    /// The words a join is written as, in the order of the variants.
    pub(crate) const NAMES: &'static [&'static str] = &["round", "miter", "bevel"];

    /// The word this join is written as.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Round => "round",
            Self::Miter => "miter",
            Self::Bevel => "bevel",
        }
    }

    /// The join written as `text`, one of `"round"`, `"miter"` or `"bevel"`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `join` if `text` is none of them.
    pub fn parse(text: &str) -> Result<Self, Error> {
        Self::named(text).ok_or_else(|| refused("join", Self::NAMES, text))
    }

    /// The join written as `name`.
    pub(crate) fn named(name: &str) -> Option<Self> {
        [Self::Round, Self::Miter, Self::Bevel]
            .into_iter()
            .find(|join| join.name() == name)
    }
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

impl Cap {
    /// The words a cap is written as.
    pub(crate) const NAMES: &'static [&'static str] = &["butt", "round", "square"];

    /// The word this cap is written as.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Butt => "butt",
            Self::Round => "round",
            Self::Square => "square",
        }
    }

    /// The cap written as `text`, one of `"butt"`, `"round"` or `"square"`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `cap` if `text` is none of them.
    pub fn parse(text: &str) -> Result<Self, Error> {
        Self::named(text).ok_or_else(|| refused("cap", Self::NAMES, text))
    }

    /// The cap written as `name`.
    pub(crate) fn named(name: &str) -> Option<Self> {
        [Self::Butt, Self::Round, Self::Square]
            .into_iter()
            .find(|cap| cap.name() == name)
    }
}

/// How a shape is painted over what is below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// Over, as ordinary paint.
    Normal,
    /// Multiplied with what is below.
    Multiply,
}

impl Blend {
    /// The words a blend is written as.
    pub(crate) const NAMES: &'static [&'static str] = &["normal", "multiply"];

    /// The word this blend is written as.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Multiply => "multiply",
        }
    }

    /// The blend written as `text`, one of `"normal"` or `"multiply"`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `mode` if `text` is none of them.
    pub fn parse(text: &str) -> Result<Self, Error> {
        Self::named(text).ok_or_else(|| refused("mode", Self::NAMES, text))
    }

    /// The blend written as `name`.
    pub(crate) fn named(name: &str) -> Option<Self> {
        [Self::Normal, Self::Multiply]
            .into_iter()
            .find(|blend| blend.name() == name)
    }
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

    /// This stroke with its width, join, cap and miter limit.
    pub(crate) fn finished(mut self, width: f64, join: Join, cap: Cap, miter_limit: f64) -> Self {
        (self.width, self.join, self.cap, self.miter_limit) = (width, join, cap, miter_limit);
        self
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

/// A drop shadow: the silhouette of a shape's fill and stroke, moved by `dx` and `dy` design pixels, blurred and
/// tinted, painted behind the shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowStyle {
    pub(crate) dx: f64,
    pub(crate) dy: f64,
    pub(crate) blur: f64,
    pub(crate) color: Color,
}

impl ShadowStyle {
    /// The offset to the right in design pixels.
    #[must_use]
    pub fn dx(&self) -> f64 {
        self.dx
    }

    /// The offset down in design pixels.
    #[must_use]
    pub fn dy(&self) -> f64 {
        self.dy
    }

    /// The blur radius in design pixels, as CSS `box-shadow` has it: the Gaussian's standard deviation is half of it.
    /// 0 is a hard copy.
    #[must_use]
    pub fn blur(&self) -> f64 {
        self.blur
    }

    /// The colour the silhouette is tinted, its alpha included.
    #[must_use]
    pub fn color(&self) -> Color {
        self.color
    }
}

/// The paint of a shape as it is drawn: `None` and [`Color::NONE`] mean not painted.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub(crate) fill: Color,
    pub(crate) stroke: Option<StrokeStyle>,
    pub(crate) opacity: f64,
    pub(crate) blend: Blend,
    pub(crate) shadow: Option<ShadowStyle>,
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
            shadow: None,
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

    /// Sets the fill; from now on only what is set is painted.
    pub(crate) fn set_fill(&mut self, color: Color) {
        self.fill = color;
        self.painted = true;
    }

    /// Sets the stroke; from now on only what is set is painted.
    pub(crate) fn set_stroke(&mut self, stroke: StrokeStyle) {
        self.stroke = Some(stroke);
        self.painted = true;
    }

    /// Multiplies the opacity by `alpha`.
    pub(crate) fn multiply_opacity(&mut self, alpha: f64) {
        self.opacity *= alpha;
    }

    /// Sets how the shape is painted over what is below.
    pub(crate) fn set_blend(&mut self, blend: Blend) {
        self.blend = blend;
    }

    /// Sets the shadow, replacing the one before.
    pub(crate) fn set_shadow(&mut self, shadow: ShadowStyle) {
        self.shadow = Some(shadow);
    }

    /// Whether a fill or a stroke was set, or the default paint resolved: false only for a shape no step has painted.
    pub(crate) fn is_painted(&self) -> bool {
        self.painted
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

    /// The shadow cast behind it, `None` where there is none.
    #[must_use]
    pub fn shadow(&self) -> Option<&ShadowStyle> {
        self.shadow.as_ref()
    }
}

impl Keyed for ShadowStyle {
    fn write_key(&self, key: &mut Key) {
        key.number(self.dx);
        key.number(self.dy);
        key.number(self.blur);
        key.of(&self.color);
    }
}

impl Keyed for StrokeStyle {
    fn write_key(&self, key: &mut Key) {
        key.of(&self.color);
        key.number(self.width);
        key.text(self.join.name());
        key.text(self.cap.name());
        key.number(self.miter_limit);
    }
}

impl Keyed for Style {
    fn write_key(&self, key: &mut Key) {
        key.of(&self.fill);
        key.of(&self.stroke);
        key.number(self.opacity);
        key.text(self.blend.name());
        key.of(&self.shadow);
        key.word(u64::from(self.painted));
    }
}
