//! Shaping one line of text: characters in, positioned glyphs out, with the font's kerning, ligatures and the rules of
//! the line's script.

use super::font::Font;
use crate::Error;
use harfrust::{Buffer, ShapeOptions, ShaperFont, shape};

/// A glyph of a shaped line, in the order it is drawn from left to right, in font units.
#[derive(Debug, Clone, Copy)]
pub(super) struct Placed {
    /// The glyph's number in the font.
    pub(super) glyph: u32,
    /// The first character the glyph stands for: a ligature stands for several.
    pub(super) character: char,
    /// The glyphs of one cluster (a base and its marks) share a number; a glyph of another cluster has another.
    pub(super) cluster: u32,
    /// How far the pen moves after the glyph.
    pub(super) advance: i32,
    /// How far the glyph is drawn to the right of the pen, and above the baseline.
    pub(super) offset: (i32, i32),
}

/// A font ready to shape lines.
pub(super) struct Shaper<'a> {
    font: ShaperFont<'a, 'a>,
}

impl<'a> Shaper<'a> {
    pub(super) fn new(font: &'a Font) -> Self {
        Self {
            font: ShaperFont::new(font.model()),
        }
    }

    /// The glyphs of `line`, shaped as one run: the script and the direction are those of its first strong character,
    /// with the font's default features.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `content` if the line cannot be shaped.
    pub(super) fn line(&self, line: &str) -> Result<Vec<Placed>, Error> {
        let mut buffer = Buffer::new();
        buffer.push_str(line);
        buffer.guess_segment_properties();
        shape(&self.font, &mut buffer, ShapeOptions::new())
            .map_err(|error| Error::parameter("content", format!("cannot be shaped: {error}")))?;
        if !buffer.allocation_successful() {
            return Err(Error::parameter("content", "cannot be shaped: out of memory"));
        }
        buffer
            .glyph_infos()
            .iter()
            .zip(buffer.glyph_positions())
            .map(|(info, position)| {
                let character = line
                    .get(info.cluster as usize..)
                    .and_then(|rest| rest.chars().next())
                    .ok_or_else(|| Error::parameter("content", "cannot be shaped: a glyph stands for no character"))?;
                Ok(Placed {
                    glyph: info.glyph_id,
                    character,
                    cluster: info.cluster,
                    advance: position.x_advance,
                    offset: (position.x_offset, position.y_offset),
                })
            })
            .collect()
    }
}
