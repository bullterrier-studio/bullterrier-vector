//! Shaping one run of text: characters in, positioned glyphs out, with the font's kerning, ligatures and the rules of
//! the run's script, at one location of the font's axes and with the features given.

use crate::Error;
use harfrust::{Buffer, Direction, Feature, Script, ShapeOptions, ShaperFont, Tag, shape};
use read_fonts::model::Font as Model;

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

/// The script and direction of a line, taken from its first strong character: every run of the line is shaped in them,
/// so that a run of digits or spaces reads as the line does, and the runs stand in the line's order.
#[derive(Debug, Clone, Copy)]
pub(super) struct Segment {
    direction: Direction,
    script: Option<Script>,
}

impl Segment {
    /// The script and direction of `line`.
    pub(super) fn of(line: &str) -> Self {
        let mut buffer = Buffer::new();
        buffer.push_str(line);
        buffer.guess_segment_properties();
        Self {
            direction: buffer.direction(),
            script: buffer.script(),
        }
    }

    /// Whether the line reads from right to left: its runs stand from the right, each run drawn as shaped.
    pub(super) fn backward(self) -> bool {
        self.direction == Direction::RightToLeft
    }
}

/// A font at one location, ready to shape runs.
pub(super) struct Shaper<'a> {
    font: ShaperFont<'a, 'a>,
    features: Vec<Feature>,
}

impl<'a> Shaper<'a> {
    /// A shaper for `font`, which carries its location, applying `features` (a tag and a value, 0 to switch off) to
    /// every run on top of the font's default ones.
    pub(super) fn new(font: &'a Model, features: &[(Tag, u32)]) -> Self {
        Self {
            font: ShaperFont::new(font),
            features: features
                .iter()
                .map(|&(tag, value)| Feature::new(tag, value, ..))
                .collect(),
        }
    }

    /// The glyphs of `run`, shaped as one run in the script and direction of its line, `segment`.
    /// The clusters of the glyphs count from `offset`, the place of the run in its line.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `content` if the run cannot be shaped.
    pub(super) fn run(&self, run: &str, offset: u32, segment: Segment) -> Result<Vec<Placed>, Error> {
        let mut buffer = Buffer::new();
        buffer.push_str(run);
        buffer.set_direction(segment.direction);
        buffer.set_script(segment.script);
        shape(&self.font, &mut buffer, ShapeOptions::new().features(&self.features))
            .map_err(|error| Error::parameter("content", format!("cannot be shaped: {error}")))?;
        if !buffer.allocation_successful() {
            return Err(Error::parameter("content", "cannot be shaped: out of memory"));
        }
        buffer
            .glyph_infos()
            .iter()
            .zip(buffer.glyph_positions())
            .map(|(info, position)| {
                let character = run
                    .get(info.cluster as usize..)
                    .and_then(|rest| rest.chars().next())
                    .ok_or_else(|| Error::parameter("content", "cannot be shaped: a glyph stands for no character"))?;
                Ok(Placed {
                    glyph: info.glyph_id,
                    character,
                    cluster: info.cluster + offset,
                    advance: position.x_advance,
                    offset: (position.x_offset, position.y_offset),
                })
            })
            .collect()
    }
}
