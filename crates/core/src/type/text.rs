//! `text`: the drawing of a string in a font, and the options that shape it.

use super::font::Font;
use crate::node::{Given, text::TextNode};
use crate::shape::refused;
use crate::{Drawing, Error, Expr, Point};
use std::collections::BTreeMap;

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
    /// Every alignment: the one list the words below are made from.
    const ALL: [Self; 3] = [Self::Left, Self::Center, Self::Right];

    /// The words an alignment is written as, in the order of [`Align::ALL`].
    pub(crate) const NAMES: [&'static str; 3] = {
        let mut names = [""; 3];
        let mut at = 0;
        while at < names.len() {
            names[at] = Self::ALL[at].name();
            at += 1;
        }
        names
    };

    /// The word this alignment is written as.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }

    /// The alignment written as `text`, one of `"left"`, `"center"` or `"right"`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `align` if `text` is none of them.
    pub fn parse(text: &str) -> Result<Self, Error> {
        Self::named(text).ok_or_else(|| refused("align", &Self::NAMES, text))
    }

    /// The alignment written as `name`.
    pub(crate) fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|align| align.name() == name)
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
    width: Option<Expr>,
    fit: Option<String>,
    axes: BTreeMap<String, Expr>,
    features: BTreeMap<String, u32>,
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
            width: None,
            fit: None,
            axes: BTreeMap::new(),
            features: BTreeMap::new(),
        }
    }

    /// This text `size` pixels to the em, above 0. A number, a `Param` or an expression, read once for the whole text.
    #[must_use]
    pub fn size(mut self, size: impl Into<Expr>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// This text with `tracking` thousandths of an em added between glyphs (a negative number tightens it), but not
    /// between a base and its marks.
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

    /// This text with its lines wrapped to `width` pixels: a line breaks at whitespace, greedily, as late as it still
    /// fits, and the whitespace at the break is dropped. A word wider than `width` stands alone on its line and
    /// overflows. Above 0; a number, a `Param` or an expression, read once at the reference pose (every `Param` at its
    /// default, whatever [`Drawing::with_values`] fixed), so the lines never change from frame to frame.
    #[must_use]
    pub fn width(mut self, width: impl Into<Expr>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// This text with each line made exactly [`width`](Text::width) wide by setting its size: `"size"`. Each line gets
    /// the size at which it is as wide as `width`, worked out once at the reference pose, so lines of different lengths
    /// have different sizes; their default leading is 1.2 times their own size. The `size` of the text still sets where
    /// the lines break, and the size of an empty line. A line with no width (marks without a base) is refused, and a
    /// text without a `width` leaves the drawing with an [`error`](Drawing::error) naming `fit`.
    #[must_use]
    pub fn fit(mut self, fit: impl Into<String>) -> Self {
        self.fit = Some(fit.into());
        self
    }

    /// This text with the variable-font axes given, each a four-letter tag (`"wght"`) and a number, a `Param` or an
    /// expression in the font's own units: `Text::axes([("wght", &weight)])`. An axis not given is at its default. If a
    /// tag is given twice the last wins, and the axes given replace those of an earlier call.
    ///
    /// Each *character* of the text has its own value: a `Param` given a list when drawn gives character `i` entry `i`
    /// (a list has an entry for every character of the text, spaces and line breaks included), and the advances follow.
    /// Characters of one value are shaped together; where the value changes, the text is shaped again, so that kerning,
    /// ligatures, mark positioning and joining do not reach across the change.
    ///
    /// A tag the font lacks, a tag that is not four printable ASCII characters, more than 64 axes, a number that is not
    /// finite or outside the font's range for the axis leave the drawing with an [`error`](Drawing::error) naming
    /// `axes`; for a number that reads a `Param`, the draw is refused.
    #[must_use]
    pub fn axes<T: Into<String>, V: Into<Expr>>(mut self, axes: impl IntoIterator<Item = (T, V)>) -> Self {
        self.axes = axes
            .into_iter()
            .map(|(tag, value)| (tag.into(), value.into()))
            .collect();
        self
    }

    /// This text with the OpenType features given, each a four-character tag and a whole number: `0` switches the
    /// feature off (`("liga", 0)`), `1` on, and a larger number selects the alternate of a feature that has them. A
    /// feature not given is as the font and the shaper default it. If a tag is given twice the last wins, and the
    /// features given replace those of an earlier call.
    ///
    /// A tag the font lacks, a tag that is not four printable ASCII characters or more than 64 features leave the drawing
    /// with an [`error`](Drawing::error) naming `features`.
    #[must_use]
    pub fn features<T: Into<String>>(mut self, features: impl IntoIterator<Item = (T, u32)>) -> Self {
        self.features = features.into_iter().map(|(tag, value)| (tag.into(), value)).collect();
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
/// The text is drawn at the size, tracking, leading, alignment, anchor, axes and features of `options`. A line is
/// shaped as one run in the script and direction of its first strong character (a line that mixes directions is not
/// reordered), or as several where the axis values change between characters ([`Text::axes`]).
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
        ("width", options.width),
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
    if let Some(fit) = options.fit {
        given.push(("fit", Given::Text(fit)));
    }
    if !options.axes.is_empty() {
        given.push(("axes", Given::Axes(options.axes.into_iter().collect())));
    }
    if !options.features.is_empty() {
        given.push(("features", Given::Features(options.features.into_iter().collect())));
    }
    Drawing::source(&TextNode, given)
}
