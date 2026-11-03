//! The lines a text is set in: which part of the content each shows.

use super::font::Font;
use super::layout::{Em, Measured, Places, Settings, feature_tags, shapers};
use crate::Error;
use std::ops::Range;

/// What each line is made to do to reach the width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fit {
    /// Scale the line: its size is set so that the line is exactly the width.
    Size,
}

/// What the lines of a text are made to fit: the width they wrap to, how each is made exactly that wide if at all, and
/// the settings at the reference pose (every Param at its default), which the plan is made at, so that lines never
/// change from frame to frame.
pub(crate) struct Wrap {
    pub(crate) pose: Settings,
    pub(crate) width: f64,
    pub(crate) fit: Option<Fit>,
}

/// A line of the content: the bytes it shows, and which characters of the content they are.
pub(super) struct Line {
    /// The bytes of the content the line shows.
    pub(super) bytes: Range<usize>,
    /// The characters of the content the line shows, counted from 0 (a line break is one).
    pub(super) characters: Range<usize>,
    /// Pixels to the em the plan fixed for the line, or `None` for the size of the text.
    pub(super) size: Option<f64>,
}

/// The lines `content` is set in: its `"\n"`-separated parts, wrapped to `wrap`'s width if there is one.
///
/// A part breaks at whitespace, greedily: a line takes words while it stays within the width, the whitespace at a break
/// is dropped, and so is the whitespace a line ends in. A word wider than the width stands alone. A part is measured
/// shaped whole, at the pose of `wrap`, so a kern across a break is counted though the line is shaped again alone.
///
/// # Errors
/// [`Error::Parameter`] naming `content` if a part cannot be shaped.
pub(super) fn plan(font: &Font, content: &str, wrap: Option<&Wrap>) -> Result<Vec<Line>, Error> {
    let parts = parts(content);
    let Some(wrap) = wrap else { return Ok(parts) };
    let pose = &wrap.pose;
    let em = Em::new(font, pose.size, pose.tracking)?;
    let features = feature_tags(pose)?;
    let places = Places::new(font, &pose.axes, content.chars().count())?;
    let shapers = shapers(&places, &features);
    let mut lines = Vec::new();
    for part in parts {
        let text = &content[part.bytes.clone()];
        let measured = Measured::of(
            text,
            &places.of_character[part.characters.clone()],
            &shapers,
            em.scale,
            em.gap,
        )?;
        for mut line in wrapped(text, &part, &measured, wrap.width) {
            if wrap.fit == Some(Fit::Size) && !line.bytes.is_empty() {
                let alone = Measured::of(
                    &content[line.bytes.clone()],
                    &places.of_character[line.characters.clone()],
                    &shapers,
                    em.scale,
                    em.gap,
                )?;
                line.size = Some(sized(&alone, pose.size, wrap.width, lines.len())?);
            }
            lines.push(line);
        }
    }
    Ok(lines)
}

/// The size at which `line`, measured at `size`, is `width` wide: a line's width grows in proportion to its size, the
/// tracking between glyphs being part of the size too. `number` is the line's place, for the error.
///
/// # Errors
/// [`Error::Parameter`] naming `fit` if the line has no width.
fn sized(line: &Measured, size: f64, width: f64, number: usize) -> Result<f64, Error> {
    if line.width > 0.0 {
        Ok(width * size / line.width)
    } else {
        Err(Error::parameter(
            "fit",
            format!("cannot make line {number} {width} wide: it has no width"),
        ))
    }
}

/// A run of characters that are not whitespace.
struct Word {
    bytes: Range<usize>,
    characters: Range<usize>,
}

/// The words of `text`.
fn words(text: &str) -> Vec<Word> {
    let mut words = Vec::new();
    let mut open: Option<(usize, usize)> = None;
    let mut count = 0;
    for (character, (byte, one)) in text.char_indices().enumerate() {
        count = character + 1;
        match (one.is_whitespace(), open) {
            (false, None) => open = Some((byte, character)),
            (true, Some((start, first))) => {
                words.push(Word {
                    bytes: start..byte,
                    characters: first..character,
                });
                open = None;
            }
            _ => {}
        }
    }
    if let Some((start, first)) = open {
        words.push(Word {
            bytes: start..text.len(),
            characters: first..count,
        });
    }
    words
}

/// The lines of `part`, whose text is `text` and which is shaped as `measured`, filled to `width`.
fn wrapped(text: &str, part: &Line, measured: &Measured, width: f64) -> Vec<Line> {
    // what a glyph adds to the width at the byte of its cluster, and the tracking that stands before a cluster (between
    // it and the cluster before it in the text, whichever way the line reads)
    let mut advance = vec![0.0; text.len() + 1];
    let mut seam = vec![0.0; text.len() + 1];
    for (at, (one, _)) in measured.glyphs.iter().enumerate() {
        advance[one.cluster as usize] += measured.advances[at];
        if let Some((next, _)) = measured.glyphs.get(at + 1) {
            seam[one.cluster.max(next.cluster) as usize] += measured.gaps[at];
        }
    }
    let mut reach = vec![0.0; text.len() + 2];
    for byte in 0..=text.len() {
        reach[byte + 1] = reach[byte] + advance[byte] + seam[byte];
    }
    // the width of the bytes from `start` to `end`: no tracking before the first cluster of a line
    let wide = |start: usize, end: usize| reach[end] - reach[start] - seam[start];
    let shown = |start: (usize, usize), last: &Word| Line {
        bytes: part.bytes.start + start.0..part.bytes.start + last.bytes.end,
        characters: part.characters.start + start.1..part.characters.start + last.characters.end,
        size: None,
    };
    let mut lines = Vec::new();
    // where the line being filled begins, in bytes and characters: the first keeps the whitespace the text starts with
    let mut start = (0, 0);
    let mut last: Option<&Word> = None;
    let words = words(text);
    for word in &words {
        if let Some(end) = last
            && wide(start.0, word.bytes.end) > width
        {
            lines.push(shown(start, end));
            start = (word.bytes.start, word.characters.start);
        }
        last = Some(word);
    }
    lines.push(last.map_or_else(
        || Line {
            bytes: part.bytes.start..part.bytes.start,
            characters: part.characters.start..part.characters.start,
            size: None,
        },
        |end| shown(start, end),
    ));
    lines
}

/// One line for every `"\n"`-separated part of `content`.
fn parts(content: &str) -> Vec<Line> {
    let mut start = 0;
    let mut before = 0;
    content
        .split('\n')
        .map(|text| {
            let count = text.chars().count();
            let line = Line {
                bytes: start..start + text.len(),
                characters: before..before + count,
                size: None,
            };
            start += text.len() + 1;
            before += count + 1;
            line
        })
        .collect()
}
