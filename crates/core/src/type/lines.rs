//! The lines a text is set in: which part of the content each shows.

use super::font::Font;
use super::layout::{Em, LEADING, Measured, Places, Settings, feature_tags, shapers};
use super::shaping::Shaper;
use super::text::Align;
use crate::limits::MAX_GLYPHS;
use crate::node::Context;
use crate::path::Region;
use crate::{Error, Point};
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

/// What bounds the lines of a text.
pub(crate) enum Boundary {
    /// Lines wrap to this width, and each is made exactly that wide by `fit`, if there is one.
    Width { width: f64, fit: Option<Fit> },
    /// Lines are set in the stretches of this region.
    Inside(Region),
}

/// What the lines of a text are made to fit, and the settings at the reference pose (every Param at its default), which
/// the plan is made at, so that the lines change from frame to frame with the boundary alone.
pub(crate) struct Wrap {
    pub(crate) pose: Settings,
    pub(crate) boundary: Boundary,
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
    /// Where the plan put the line: the left end, middle or right end (by `align`) of its baseline, or `None` for the
    /// anchor of the text and its leading.
    pub(super) at: Option<Point>,
}

impl Line {
    /// A line showing `bytes`, which are `characters`, with nothing fixed for it.
    fn new(bytes: Range<usize>, characters: Range<usize>) -> Self {
        Self {
            bytes,
            characters,
            size: None,
            axis: None,
            at: None,
        }
    }
}

/// The lines `content` is set in: its `"\n"`-separated parts, wrapped to `wrap`'s width or set in its region if it has one.
///
/// A part breaks at whitespace, except no-break spaces, greedily: a line takes words while it stays within the width, the whitespace at a break
/// is dropped, and so is the whitespace a line ends in. A word wider than the width stands alone. A part is measured
/// shaped whole, at the pose of `wrap`, so a kern across a break is counted though the line is shaped again alone.
///
/// # Errors
/// [`Error::Parameter`] naming `content` if a part cannot be shaped, and the errors of [`Fitting::fit`] and [`inside`].
pub(super) fn plan(font: &Font, content: &str, wrap: Option<&Wrap>, cx: &Context<'_>) -> Result<Vec<Line>, Error> {
    let parts = parts(content);
    let Some(wrap) = wrap else { return Ok(parts) };
    let pose = &wrap.pose;
    let em = Em::new(font, pose.size, pose.tracking)?;
    let features = feature_tags(pose)?;
    let places = Places::new(font, &pose.axes, content.chars().count())?;
    let shapers = shapers(&places, &features);
    let measure = |part: &Line| {
        Measured::of(
            &content[part.bytes.clone()],
            &places.of_character[part.characters.clone()],
            &shapers,
            em.scale,
            em.gap,
        )
    };
    let (width, fit) = match &wrap.boundary {
        Boundary::Width { width, fit } => (*width, fit.as_ref()),
        Boundary::Inside(region) => return inside(region, parts, content, pose, &em, measure, cx),
    };
    let fitting = Fitting {
        font,
        content,
        pose,
        width,
        fit,
        em: &em,
        features: &features,
        places: &places,
        shapers: &shapers,
    };
    let mut lines = Vec::new();
    for part in parts {
        let measured = measure(&part)?;
        for mut line in wrapped(&content[part.bytes.clone()], &part, &measured, width) {
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
    pose: &'a Settings,
    width: f64,
    fit: Option<&'a Fit>,
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
        let pose = self.pose;
        match self.fit {
            None => {}
            Some(Fit::Size) => {
                let alone = Measured::of(
                    text,
                    &self.places.of_character[line.characters.clone()],
                    self.shapers,
                    self.em.scale,
                    self.em.gap,
                )?;
                line.size = Some(sized(&alone, pose.size, self.width, number)?);
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
                    self.width,
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

/// Whether `one` is a place to break a line: whitespace, except the three no-break spaces (U+00A0 no-break space, U+2007
/// figure space, U+202F narrow no-break space), which are part of a word.
fn breaks(one: char) -> bool {
    one.is_whitespace() && !matches!(one, '\u{a0}' | '\u{2007}' | '\u{202f}')
}

/// A run of characters between breaks.
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
        match (breaks(one), open) {
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

/// How far each byte of a part reaches along its line when it is shaped whole, to measure any stretch of it.
struct Reach {
    /// The width of the bytes before each byte, tracking included.
    reach: Vec<f64>,
    /// The tracking that stands before a cluster (between it and the cluster before it in the text, whichever way the
    /// line reads), at the byte of the cluster.
    seam: Vec<f64>,
}

impl Reach {
    /// The reach of `text`, which is shaped as `measured`.
    fn of(text: &str, measured: &Measured) -> Self {
        // what a glyph adds to the width at the byte of its cluster
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
        Self { reach, seam }
    }

    /// The width of the bytes from `start` to `end`: no tracking before the first cluster of a line.
    fn wide(&self, start: usize, end: usize) -> f64 {
        self.reach[end] - self.reach[start] - self.seam[start]
    }
}

/// The line of `part` that shows its text from `start` (a byte and a character) to the end of the word `last`.
fn shown(part: &Line, start: (usize, usize), last: &Word) -> Line {
    Line::new(
        part.bytes.start + start.0..part.bytes.start + last.bytes.end,
        part.characters.start + start.1..part.characters.start + last.characters.end,
    )
}

/// A line of `part` that shows nothing.
fn nothing(part: &Line) -> Line {
    Line::new(
        part.bytes.start..part.bytes.start,
        part.characters.start..part.characters.start,
    )
}

/// The lines of `part`, whose text is `text` and which is shaped as `measured`, filled to `width`.
fn wrapped(text: &str, part: &Line, measured: &Measured, width: f64) -> Vec<Line> {
    let reach = Reach::of(text, measured);
    let mut lines = Vec::new();
    // where the line being filled begins, in bytes and characters: the first keeps the whitespace the text starts with
    let mut start = (0, 0);
    let mut last: Option<&Word> = None;
    let words = words(text);
    for word in &words {
        if let Some(end) = last
            && reach.wide(start.0, word.bytes.end) > width
        {
            lines.push(shown(part, start, end));
            start = (word.bytes.start, word.characters.start);
        }
        last = Some(word);
    }
    lines.push(last.map_or_else(|| nothing(part), |end| shown(part, start, end)));
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

/// The error for the text outlasting the shape: `word`, the first of the line `number` that found no room.
fn outlasts(word: &str, number: usize) -> Error {
    Error::parameter(
        "inside",
        format!(
            "the text does not fit: {word:?}, the first word of line {number}, found no baseline in the shape where it fits"
        ),
    )
}

/// The lines of `parts` (the `"\n"`-separated parts of `content`) set in the stretches of `region`, at the reference
/// `pose`. `measure` shapes a part.
///
/// Each line takes a baseline: the first is a capital's height (`em`'s) below the top of the region, and each is a leading
/// below the one before. The chord of a baseline is the widest stretch of the region that holds both the baseline and the
/// top of a capital over it (the leftmost of equals); the line takes words, greedily, while they fit it, and `pose.align`
/// places it against the chord. A baseline whose chord does not hold the next word is skipped and gets no line, and an
/// empty part takes a baseline of its own.
///
/// # Errors
/// [`Error::Parameter`] naming `inside` if the region has no outline, the text does not fit it, or more than `MAX_GLYPHS`
/// baselines are tried; naming `leading` if it is not above 0; and `measure`'s.
#[expect(
    clippy::cast_precision_loss,
    reason = "at most MAX_GLYPHS baselines are tried, far fewer than 2^53"
)]
fn inside(
    region: &Region,
    parts: Vec<Line>,
    content: &str,
    pose: &Settings,
    em: &Em,
    measure: impl Fn(&Line) -> Result<Measured, Error>,
    cx: &Context<'_>,
) -> Result<Vec<Line>, Error> {
    let Some((top, bottom)) = region.extent() else {
        return Err(Error::parameter("inside", "the shape has no outline to hold text in"));
    };
    let leading = pose.leading.unwrap_or(LEADING * pose.size);
    if leading <= 0.0 {
        return Err(Error::parameter(
            "leading",
            format!("is {leading}, but a text inside a shape needs a leading above 0"),
        ));
    }
    let place = cx.place.to_string();
    let charge = |count| cx.work.made_segments(count, &place);
    // the baselines tried so far; each is a capital's height below the line of capital tops it belongs to, which stands
    // a leading below the one before, from the top of the shape
    let mut tried = 0_usize;
    let mut next = || {
        if tried >= MAX_GLYPHS {
            return Err(Error::parameter(
                "inside",
                format!("the text needs more than MAX_GLYPHS {MAX_GLYPHS} baselines tried in the shape"),
            ));
        }
        let capitals = top + tried as f64 * leading;
        tried += 1;
        Ok((capitals, capitals + em.cap_height))
    };
    let mut lines = Vec::new();
    for part in parts {
        let text = &content[part.bytes.clone()];
        let words = words(text);
        if words.is_empty() {
            // no glyph stands there, so neither end matters
            let (_, baseline) = next()?;
            lines.push(Line {
                at: Some(Point::new(0.0, baseline)),
                ..nothing(&part)
            });
            continue;
        }
        let reach = Reach::of(text, &measure(&part)?);
        let mut start = (0, 0);
        let mut at = 0;
        while at < words.len() {
            let (baseline, (left, right)) = loop {
                let (capitals, baseline) = next()?;
                if baseline >= bottom {
                    return Err(outlasts(&text[words[at].bytes.clone()], lines.len()));
                }
                if let Some(chord) = widest(&region.spans_at(baseline, charge)?, &region.spans_at(capitals, charge)?)
                    && reach.wide(start.0, words[at].bytes.end) <= chord.1 - chord.0
                {
                    break (baseline, chord);
                }
            };
            let mut last = at;
            while let Some(word) = words.get(last + 1)
                && reach.wide(start.0, word.bytes.end) <= right - left
            {
                last += 1;
            }
            let x = match pose.align {
                Align::Left => left,
                Align::Center => f64::midpoint(left, right),
                Align::Right => right,
            };
            lines.push(Line {
                at: Some(Point::new(x, baseline)),
                ..shown(&part, start, &words[last])
            });
            at = last + 1;
            if let Some(word) = words.get(at) {
                start = (word.bytes.start, word.characters.start);
            }
        }
    }
    Ok(lines)
}

/// The widest stretch both lists of spans (each left to right and apart) hold, the leftmost if several are as wide.
fn widest(a: &[(f64, f64)], b: &[(f64, f64)]) -> Option<(f64, f64)> {
    let (mut one, mut other) = (0, 0);
    let mut best: Option<(f64, f64)> = None;
    while let (Some(x), Some(y)) = (a.get(one), b.get(other)) {
        let (left, right) = (x.0.max(y.0), x.1.min(y.1));
        if right > left && best.is_none_or(|(l, r)| right - left > r - l) {
            best = Some((left, right));
        }
        if x.1 < y.1 {
            one += 1;
        } else {
            other += 1;
        }
    }
    best
}
