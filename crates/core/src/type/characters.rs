//! `characters`: the typed call of a grid of glyphs.

use super::font::Font;
use crate::node::Given;
use crate::node::characters::CharactersNode;
use crate::{Color, Drawing, Expr, Scale};

/// The options of [`characters`]: the font and the size of a cell, and what is not the default of the rest.
#[derive(Debug, Clone)]
pub struct Characters {
    font: Option<Font>,
    cell: Scale,
    size: Option<Expr>,
    colors: Option<Vec<Vec<Color>>>,
}

impl Characters {
    /// A grid of cells `cell` wide and high: one number for square cells or an `(x, y)` pair, in the bundled
    /// monospaced font (`JetBrains Mono`). A glyph is sized to fit its cell.
    #[must_use]
    pub fn new(cell: impl Into<Scale>) -> Self {
        Self {
            font: None,
            cell: cell.into(),
            size: None,
            colors: None,
        }
    }

    /// These characters in `font` instead of the bundled monospaced one.
    #[must_use]
    pub fn font(mut self, font: &Font) -> Self {
        self.font = Some(font.clone());
        self
    }

    /// These characters `size` pixels to the em, above 0, instead of the size that fits the cell. A number, a
    /// `Param` or an expression, read once.
    #[must_use]
    pub fn size(mut self, size: impl Into<Expr>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// These characters filled with `colors`, a row of colours for each row of the text with a colour for each of its
    /// characters (spaces included): the rows of colour texts bullterrier-image's Python `characters(...).colors` and a
    /// document hold. Rows or colours that do not match leave the drawing with an [`error`](Drawing::error) naming
    /// `colors`.
    ///
    /// Image's Rust `Characters::colors()` is one flat slice, row after row; cut it by `columns()` and convert each
    /// triple of `f32` channels (0 to 1) to a [`Color`]:
    ///
    /// ```text
    /// let byte = |channel: f32| (channel.clamp(0.0, 1.0) * 255.0).round() as u32;
    /// let rows = art.colors().chunks(art.columns()).map(|row| {
    ///     row.iter().map(|[r, g, b]| Color::hex(byte(*r) << 16 | byte(*g) << 8 | byte(*b)))
    /// });
    /// let grid = characters(art.text(), Characters::new(cell).colors(rows));
    /// ```
    #[must_use]
    pub fn colors<R, C>(mut self, colors: R) -> Self
    where
        R: IntoIterator<Item = C>,
        C: IntoIterator<Item = Color>,
    {
        self.colors = Some(colors.into_iter().map(|row| row.into_iter().collect()).collect());
        self
    }
}

/// A grid of the characters of `rows` (a row to a line, `\n` between them) in the cells and font of `options`: one
/// shape for each character that has an outline, standing in its cell, with the attributes `char`, `row`, `column`,
/// `order` and `origin` (the middle of its cell).
///
/// Each character is set alone (no kerning, no ligature across cells) by the layout of [`text`](super::text): the
/// middle of its advance is the middle of its cell across, the middle of a capital the middle of the cell down, so a
/// full stop stays on the baseline of its row. A character the font lacks is its notdef glyph; a space has no outline
/// and makes no shape. The shapes have no paint unless [`Characters::colors`] gives one.
///
/// A cell that is not above 0 and finite, a size not above 0, a control character in `rows`, more than 100,000
/// characters, or colours that do not match the rows leave the drawing with an [`error`](Drawing::error).
#[must_use]
pub fn characters(rows: impl Into<String>, options: Characters) -> Drawing {
    let mut given = vec![
        ("content", Given::Text(rows.into())),
        (
            "cell",
            Given::Pair(Expr::from(options.cell.x), Expr::from(options.cell.y)),
        ),
    ];
    if let Some(font) = options.font {
        given.push(("font", Given::Font(font)));
    }
    if let Some(size) = options.size {
        given.push(("size", Given::Number(size)));
    }
    if let Some(colors) = options.colors {
        given.push(("colors", Given::Colors(colors)));
    }
    Drawing::source(&CharactersNode, given)
}
