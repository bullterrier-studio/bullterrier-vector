//! `text`: the drawing of a string in a font, and the options that shape it.

use super::font::Font;
use crate::node::{Given, text::TextNode};
use crate::{Drawing, Expr, Point};

/// Where a line stands against the anchor of [`Text`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// The line starts at the anchor.
    Left,
    /// The line is centred on the anchor.
    Center,
    /// The line ends at the anchor.
    Right,
}

impl Align {
    /// The words an alignment is written as, in the order of the variants.
    pub(crate) const NAMES: &'static [&'static str] = &["left", "center", "right"];

    /// The word this alignment is written as.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }

    /// The alignment written as `name`.
    pub(crate) fn named(name: &str) -> Option<Self> {
        [Self::Left, Self::Center, Self::Right]
            .into_iter()
            .find(|align| align.name() == name)
    }
}

/// The options of [`text`]: the font, and what is not the default of the rest. A [`Font`] alone converts into one.
#[derive(Debug, Clone)]
pub struct Text {
    font: Font,
    size: Option<Expr>,
    tracking: Option<Expr>,
    leading: Option<Expr>,
    align: Option<Align>,
    anchor: Option<Point>,
}

impl Text {
    /// Text in `font`: 100 pixels to the em, no tracking, a line spacing of 1.2 times the size, aligned left, its first
    /// baseline starting at the origin.
    #[must_use]
    pub fn new(font: &Font) -> Self {
        Self {
            font: font.clone(),
            size: None,
            tracking: None,
            leading: None,
            align: None,
            anchor: None,
        }
    }

    /// This text `size` pixels to the em, above 0. A number, a `Param` or an expression, read once for the whole text.
    #[must_use]
    pub fn size(mut self, size: impl Into<Expr>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// This text with `tracking` thousandths of an em added between glyphs (a negative number tightens it).
    #[must_use]
    pub fn tracking(mut self, tracking: impl Into<Expr>) -> Self {
        self.tracking = Some(tracking.into());
        self
    }

    /// This text with `leading` pixels between baselines instead of 1.2 times the size.
    #[must_use]
    pub fn leading(mut self, leading: impl Into<Expr>) -> Self {
        self.leading = Some(leading.into());
        self
    }

    /// This text with its lines aligned as `align` against the anchor.
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }

    /// This text with the left end, middle or right end (by [`Text::align`]) of its first baseline at `anchor`.
    #[must_use]
    pub fn anchor(mut self, anchor: impl Into<Point>) -> Self {
        self.anchor = Some(anchor.into());
        self
    }
}

impl From<&Font> for Text {
    fn from(font: &Font) -> Self {
        Self::new(font)
    }
}

/// A drawing of `content` in the font of `options`: one shape for every glyph that has an outline (a space or a line
/// break has none), shaped with the font's kerning, ligatures and the rules of the script of each line, and written
/// with the attributes `glyph`, `char`, `word`, `line`, `order` and `origin`. A `"\n"` in `content` breaks a line. The
/// outlines keep the font's own contours, which may overlap; they fill as the font draws them.
///
/// The text is drawn at the size, tracking, leading, alignment and anchor of `options`. A line is shaped as one run in
/// the script and direction of its first strong character: a line that mixes directions is not reordered.
///
/// A text with more than 100,000 characters, a size not above 0, a number that is not finite, an anchor or a
/// coordinate beyond ±1e100 leaves the drawing with an [`error`](Drawing::error), or, for numbers that read a Param,
/// is refused when the drawing is drawn.
#[must_use]
pub fn text(content: impl Into<String>, options: impl Into<Text>) -> Drawing {
    let options = options.into();
    let mut given = vec![
        ("content", Given::Text(content.into())),
        ("font", Given::Font(options.font)),
    ];
    for (name, number) in [
        ("size", options.size),
        ("tracking", options.tracking),
        ("leading", options.leading),
    ] {
        if let Some(number) = number {
            given.push((name, Given::Number(number)));
        }
    }
    if let Some(align) = options.align {
        given.push(("align", Given::Text(align.name().to_owned())));
    }
    if let Some(anchor) = options.anchor {
        given.push(("anchor", Given::Pair(Expr::from(anchor.x), Expr::from(anchor.y))));
    }
    Drawing::source(&TextNode, given)
}
