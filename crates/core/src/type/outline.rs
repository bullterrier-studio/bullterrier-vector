//! Glyph outlines: a glyph's contours in font units, and the `Path` they make at a place and size.

use crate::geometry::{MAX_COORDINATE, is_coordinate};
use crate::path::Builder;
use crate::{Error, Path, Point};
use read_fonts::model::Font as Model;
use read_fonts::model::glyph::outline::{DrawMemory, DrawOptions, OutlineGlyphs, PathElement};
use read_fonts::types::GlyphId;
use std::collections::BTreeMap;
use std::rc::Rc;

/// The outlines of a font at one location, each drawn from the font once and kept for the glyphs that use it again.
pub(super) struct Outlines<'a> {
    glyphs: OutlineGlyphs<'a>,
    drawn: BTreeMap<u32, Option<Rc<Vec<PathElement>>>>,
}

impl<'a> Outlines<'a> {
    pub(super) fn new(font: &'a Model) -> Self {
        Self {
            glyphs: font.outline_glyphs(),
            drawn: BTreeMap::new(),
        }
    }

    /// The contours of glyph `glyph` in font units, or `None` if it has none (a space).
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `font` if the font's data for the glyph is broken or past the limits of the reader.
    pub(super) fn of(&mut self, glyph: u32) -> Result<Option<Rc<Vec<PathElement>>>, Error> {
        if let Some(found) = self.drawn.get(&glyph) {
            return Ok(found.clone());
        }
        let drawn = match self.glyphs.get(GlyphId::new(glyph)) {
            None => None,
            Some(outline) => {
                let mut elements = Vec::new();
                outline
                    .draw(&DrawOptions::new(), DrawMemory::Scratch, &mut elements)
                    .map_err(|error| Error::parameter("font", format!("glyph {glyph} cannot be drawn: {error}")))?;
                (!elements.is_empty()).then(|| Rc::new(elements))
            }
        };
        self.drawn.insert(glyph, drawn.clone());
        Ok(drawn)
    }
}

/// The path of `elements`, put at `origin` with `scale` design pixels to a font unit and the y axis turned down.
///
/// # Errors
/// [`Error::Parameter`] naming `size` if a coordinate comes out beyond ±1e100: the origin is within it, so it is the
/// size of the glyph that carries the outline out.
pub(super) fn path(elements: &[PathElement], origin: Point, scale: f64) -> Result<Path, Error> {
    let at = |x: f32, y: f32| -> Result<Point, Error> {
        let point = Point::new(origin.x + f64::from(x) * scale, origin.y - f64::from(y) * scale);
        if is_coordinate(point.x) && is_coordinate(point.y) {
            Ok(point)
        } else {
            Err(Error::parameter(
                "size",
                format!(
                    "puts a glyph at ({:e}, {:e}), beyond ±{MAX_COORDINATE:e}",
                    point.x, point.y
                ),
            ))
        }
    };
    let mut builder = Builder::new();
    for element in elements {
        match *element {
            PathElement::MoveTo { x, y } => builder.move_to(at(x, y)?)?,
            PathElement::LineTo { x, y } => builder.line_to(at(x, y)?)?,
            PathElement::QuadTo { cx0, cy0, x, y } => builder.quad_to(at(cx0, cy0)?, at(x, y)?)?,
            PathElement::CurveTo {
                cx0,
                cy0,
                cx1,
                cy1,
                x,
                y,
            } => builder.curve_to(at(cx0, cy0)?, at(cx1, cy1)?, at(x, y)?)?,
            PathElement::Close => builder.close(),
        }
    }
    Ok(builder.finish())
}
