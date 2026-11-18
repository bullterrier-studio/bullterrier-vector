//! One string set at a place, for the sources that stand glyphs on a grid or beside a mark: the single-line entry of
//! the one text layout, so a character grid, an overlay label and a `text` are shaped and outlined the same way.

use super::font::Font;
use super::layout::{RunSettings, Settings, typeset};
use super::text::Align;
use crate::node::Context;
use crate::{Error, Point, Shape};

/// The glyphs of `content` (one line) in `font`, `size` pixels to the em (the caller's argument `size_argument`, which a
/// refusal names), the left end of its baseline at `anchor`: what
/// [`typeset`] makes of it, so each shape carries `glyph`, `char`, `word`, `line`, `run`, `order` and `origin`.
///
/// # Errors
/// As [`typeset`]: [`Error::Parameter`] naming the step.
pub(crate) fn string(
    font: &Font,
    content: &str,
    (size_argument, size): (&str, f64),
    anchor: Point,
    cx: &Context<'_>,
) -> Result<Vec<Shape>, Error> {
    let settings = Settings {
        runs: vec![RunSettings {
            font: font.clone(),
            characters: 0..content.chars().count(),
            size,
            tracking: 0.0,
            axes: Vec::new(),
            features: Vec::new(),
        }],
        leading: None,
        align: Align::Left,
        anchor,
    };
    typeset(content, &settings, None, cx).map_err(|error| error.renamed("size", size_argument))
}
