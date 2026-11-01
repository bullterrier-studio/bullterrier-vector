//! Laying out a string: lines, the pen along each, and one shape for every glyph that has an outline.

use super::font::Font;
use super::outline::{self, Outlines};
use super::shaping::Shaper;
use super::text::Align;
use crate::node::Context;
use crate::{Error, Point, Shape};
use read_fonts::model::glyph::outline::PathElement;
use std::rc::Rc;

/// Line spacing when `leading` is not given, in ems.
// constant: api.md, `text`: "leading: pixels between baselines; default 1.2 x size"
const LEADING: f64 = 1.2;

/// The height of a capital, in ems, for a font that states none (a font without `OS/2` version 2).
// constant: the cap height of the common text faces is 0.68 to 0.72 em; it only places the `origin` attribute
const CAP_HEIGHT: f64 = 0.7;

/// The arguments of `text`, read for the draw.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Settings {
    /// Pixels to the em.
    pub(crate) size: f64,
    /// Extra space between glyphs, in thousandths of an em.
    pub(crate) tracking: f64,
    /// Pixels between baselines; `None` for 1.2 times `size`.
    pub(crate) leading: Option<f64>,
    /// Where the lines stand against `anchor`.
    pub(crate) align: Align,
    /// The left end, middle or right end (by `align`) of the first baseline.
    pub(crate) anchor: Point,
}

/// A glyph to draw, with what the shape that draws it carries.
struct Drawn {
    outline: Rc<Vec<PathElement>>,
    /// Where the pen stands for the glyph, on its baseline.
    origin: Point,
    glyph: u32,
    character: char,
    /// The word the glyph is in, none for a space that has an outline.
    word: Option<u32>,
    line: u32,
    /// The point the glyph turns and scales about.
    centre: Point,
}

/// The shapes of `content` in `font`: one for every glyph that has an outline, left to right, line by line.
///
/// # Errors
/// [`Error::Parameter`] naming the step if the text cannot be shaped, a glyph cannot be drawn, a coordinate comes out
/// beyond ±1e100, or the draw has passed `MAX_SHAPES` or `MAX_OUTLINE_SEGMENTS`.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "the text has at most MAX_GLYPHS characters, far fewer than 2^32 glyphs and lines, or 2^53 shapes"
)]
pub(crate) fn typeset(font: &Font, content: &str, settings: &Settings, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
    let placed = |error: Error| error.at_argument(&cx.place, None);
    let metrics = font.model().metrics();
    let units_per_em = f64::from(metrics.units_per_em);
    let scale = settings.size / units_per_em;
    let gap = settings.tracking * settings.size / 1000.0;
    let leading = settings.leading.unwrap_or(LEADING * settings.size);
    let cap_height = metrics
        .cap_height
        .map_or(CAP_HEIGHT * units_per_em, read_fonts::types::F48Dot16::to_f64)
        * scale;
    let shaper = Shaper::new(font);
    let mut outlines = Outlines::new(font);

    let mut glyphs = Vec::new();
    // the words met so far, and the one the glyph being placed is in (none after a space)
    let mut words = 0_u32;
    for (number, text) in content.split('\n').enumerate() {
        let mut word = None;
        let line = number as u32;
        let baseline = settings.anchor.y + number as f64 * leading;
        let glyphs_of_line = shaper.line(text).map_err(placed)?;
        let advances: Vec<f64> = glyphs_of_line
            .iter()
            .map(|one| f64::from(one.advance) * scale)
            .collect();
        let width = advances.iter().sum::<f64>() + gap * glyphs_of_line.len().saturating_sub(1) as f64;
        let mut pen = settings.anchor.x
            - match settings.align {
                Align::Left => 0.0,
                Align::Center => width / 2.0,
                Align::Right => width,
            };
        for (one, advance) in glyphs_of_line.iter().zip(&advances) {
            if one.character.is_whitespace() {
                word = None;
            } else if word.is_none() {
                word = Some(words);
                words += 1;
            }
            if let Some(outline) = outlines.of(one.glyph).map_err(placed)? {
                let origin = Point::new(
                    pen + f64::from(one.offset.0) * scale,
                    baseline - f64::from(one.offset.1) * scale,
                );
                glyphs.push(Drawn {
                    outline,
                    origin,
                    glyph: glyphs.len() as u32,
                    character: one.character,
                    word,
                    line,
                    centre: Point::new(pen + advance / 2.0, baseline - cap_height / 2.0),
                });
            }
            pen += advance + gap;
        }
    }
    // charged before any shape is made, so that a refusal comes before the memory
    cx.work.made_shapes(glyphs.len(), &cx.place)?;
    cx.work
        .made_outline(glyphs.iter().map(|drawn| drawn.outline.len()).sum(), &cx.place)?;
    let last = glyphs.len().saturating_sub(1);
    glyphs
        .iter()
        .map(|drawn| {
            let path = outline::path(&drawn.outline, drawn.origin, scale).map_err(placed)?;
            let mut shape = Shape::from_path(path, 0);
            let attributes = shape.attributes_mut();
            attributes.glyph = Some(drawn.glyph);
            attributes.char = Some(drawn.character);
            attributes.word = drawn.word;
            attributes.line = Some(drawn.line);
            attributes.order = Some(if last == 0 {
                0.0
            } else {
                f64::from(drawn.glyph) / last as f64
            });
            attributes.origin = Some(drawn.centre);
            Ok(shape)
        })
        .collect()
}
