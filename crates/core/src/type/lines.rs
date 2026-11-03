//! The lines a text is set in: which part of the content each shows.

use super::font::Font;
use super::layout::{Em, Measured, Places, Settings, feature_tags, shapers};
use super::shaping::Shaper;
use crate::Error;
use read_fonts::types::Tag;
use std::ops::Range;

/// What each line is made to do to reach the width.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Fit {
    /// Scale the line: its size is set so that the line is exactly the width.
    Size,
    /// Set the variable-font axis with this tag for the line, so that it is exactly the width.
    Axis(String),
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
    /// The value of the axis `fit` names that the plan fixed for the line.
    pub(super) axis: Option<f64>,
}

impl Line {
    /// A line showing `bytes`, which are `characters`, with nothing fixed for it.
    fn new(bytes: Range<usize>, characters: Range<usize>) -> Self {
        Self {
            bytes,
            characters,
            size: None,
            axis: None,
        }
    }
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
    let fitting = Fitting {
        font,
        content,
        wrap,
        em: &em,
        features: &features,
        places: &places,
        shapers: &shapers,
    };
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
            if !line.bytes.is_empty() {
                fitting.fit(&mut line, lines.len())?;
            }
            lines.push(line);
        }
    }
    Ok(lines)
}

/// What fitting a line to the width needs: the text and the way it is shaped at the reference pose.
struct Fitting<'a, 'b> {
    font: &'a Font,
    content: &'a str,
    wrap: &'a Wrap,
    em: &'a Em,
    features: &'a [(Tag, u32)],
    places: &'a Places,
    shapers: &'a [Shaper<'b>],
}

impl Fitting<'_, '_> {
    /// Fixes for `line`, the line number `number`, what makes it exactly the width wide, if the wrap asks for a fit.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `fit` if the line cannot be made that wide.
    fn fit(&self, line: &mut Line, number: usize) -> Result<(), Error> {
        let text = &self.content[line.bytes.clone()];
        let pose = &self.wrap.pose;
        match &self.wrap.fit {
            None => {}
            Some(Fit::Size) => {
                let alone = Measured::of(
                    text,
                    &self.places.of_character[line.characters.clone()],
                    self.shapers,
                    self.em.scale,
                    self.em.gap,
                )?;
                line.size = Some(sized(&alone, pose.size, self.wrap.width, number)?);
            }
            Some(Fit::Axis(tag)) => {
                let (min, _, max) = self.font.axis_range(tag, "fit")?;
                let count = line.characters.len();
                let others: Vec<(String, Vec<f64>)> = pose
                    .axes
                    .iter()
                    .map(|(own, values)| (own.clone(), values[line.characters.clone()].to_vec()))
                    .collect();
                // the line shaped alone with the axis at `value`
                let width_at = |value: f64| {
                    let mut axes = others.clone();
                    axes.push((tag.clone(), vec![value; count]));
                    let places = Places::new(self.font, &axes, count)?;
                    let shapers = shapers(&places, self.features);
                    Ok(Measured::of(text, &places.of_character, &shapers, self.em.scale, self.em.gap)?.width)
                };
                line.axis = Some(solved(
                    width_at,
                    (f64::from(min), f64::from(max)),
                    self.wrap.width,
                    tag,
                    number,
                )?);
            }
        }
        Ok(())
    }
}

/// How many times the range of an axis is halved to find the value that makes a line the width: the range of a font's axis
/// is at most a few thousand units and the shaper rounds an advance to a unit, so the value is found well within it.
// constant: 2^-32 of the widest axis range (65535) is below 2^-16, far below the unit an advance is rounded to
const FIT_STEPS: u32 = 32;

/// The value of an axis in `range` at which `width_at` first reaches `target`, going from the end of the range where the
/// width is least to the end where it is most (a width may rise or fall along an axis, and is taken to move one way). The
/// line is the least width at the value, or a unit of an advance above it. `number` is the line's place, `tag` the axis.
///
/// # Errors
/// [`Error::Parameter`] naming `fit` if `target` is outside the widths at the two ends of the range, and `width_at`'s
/// errors.
fn solved(
    mut width_at: impl FnMut(f64) -> Result<f64, Error>,
    (min, max): (f64, f64),
    target: f64,
    tag: &str,
    number: usize,
) -> Result<f64, Error> {
    let (at_min, at_max) = (width_at(min)?, width_at(max)?);
    let rising = at_max >= at_min;
    let (least, most) = if rising { (at_min, at_max) } else { (at_max, at_min) };
    if target < least || target > most {
        let (nearest, at) = match (target < least, rising) {
            (true, true) | (false, false) => (at_min, min),
            (true, false) | (false, true) => (at_max, max),
        };
        return Err(Error::parameter(
            "fit",
            format!("cannot make line {number} {target} wide: the nearest {tag:?} reaches is {nearest}, at {at}"),
        ));
    }
    let reached = |width: f64| if rising { width >= target } else { width <= target };
    let (mut low, mut high) = (min, max);
    for _ in 0..FIT_STEPS {
        let middle = low + (high - low) / 2.0;
        if reached(width_at(middle)?) {
            high = middle;
        } else {
            low = middle;
        }
    }
    Ok(high)
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
    let shown = |start: (usize, usize), last: &Word| {
        Line::new(
            part.bytes.start + start.0..part.bytes.start + last.bytes.end,
            part.characters.start + start.1..part.characters.start + last.characters.end,
        )
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
        || {
            Line::new(
                part.bytes.start..part.bytes.start,
                part.characters.start..part.characters.start,
            )
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
            let line = Line::new(start..start + text.len(), before..before + count);
            start += text.len() + 1;
            before += count + 1;
            line
        })
        .collect()
}
