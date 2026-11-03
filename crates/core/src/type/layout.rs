//! Laying out a string: lines, the pen along each, and one shape for every glyph that has an outline.

use super::font::{Font, tag_of};
use super::lines::{self, Boundary, Fit, Wrap};
use super::outline::{self, Outlines};
use super::shaping::{Placed, Segment, Shaper};
use super::text::Align;
use crate::geometry::{MAX_COORDINATE, is_coordinate};
use crate::node::Context;
use crate::{Error, Point, Shape};
use read_fonts::model::Font as Model;
use read_fonts::model::glyph::outline::PathElement;
use read_fonts::types::Tag;
use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;

/// Line spacing when `leading` is not given, in ems.
// constant: api.md, `text`: "leading: pixels between baselines; default 1.2 x size"
pub(super) const LEADING: f64 = 1.2;

/// The height of a capital, in ems, for a font that states none (no `OS/2` version 2, or a `sCapHeight` of 0).
// constant: the cap height of the common text faces is 0.68 to 0.72 em; it only places the `origin` attribute
const CAP_HEIGHT: f64 = 0.7;

/// The arguments of `text`, read for the draw.
#[derive(Debug, Clone)]
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
    /// The value of each axis given, by tag, for every character of the text (line breaks included): as many numbers as
    /// the text has characters.
    pub(crate) axes: Vec<(String, Vec<f64>)>,
    /// The features given, by tag, with their values.
    pub(crate) features: Vec<(String, u32)>,
}

/// `position`, or the error naming `argument`, whose value is `value`, unless it is within the coordinates a path holds.
fn within(argument: &str, value: f64, position: f64) -> Result<f64, Error> {
    if is_coordinate(position) {
        Ok(position)
    } else {
        Err(Error::parameter(
            argument,
            format!("{value:e} puts the text beyond ±{MAX_COORDINATE:e}"),
        ))
    }
}

/// The distinct locations of the font's axes a text asks for, and which one each character is at.
pub(super) struct Places {
    /// The font at each location.
    pub(super) models: Vec<Model>,
    /// For every character of the text, the index in `models` of its location.
    pub(super) of_character: Vec<usize>,
}

impl Places {
    /// The locations of `characters` characters whose axis values are `axes`. Characters with equal values (as the
    /// shaper reads them, in `f32`) share a location; a text without axes has one, the font's own.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `axes` if a tag is not one of the font's or a value is outside its range.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "an axis value is inside the range of the font's axis, which the font stores as 16.16 fixed point"
    )]
    pub(super) fn new(font: &Font, axes: &[(String, Vec<f64>)], characters: usize) -> Result<Self, Error> {
        let mut tags = Vec::new();
        for (tag, values) in axes {
            font.check_axis(tag, None)?;
            // the lowest and the highest decide whether every value is in range
            for value in [
                values.iter().copied().reduce(f64::min),
                values.iter().copied().reduce(f64::max),
            ]
            .into_iter()
            .flatten()
            {
                font.check_axis(tag, Some(value))?;
            }
            tags.push(tag_of(tag, "axes")?);
        }
        let mut found: BTreeMap<Vec<(Tag, u32)>, usize> = BTreeMap::new();
        let mut models = Vec::new();
        let mut of_character = Vec::with_capacity(characters);
        for at in 0..characters {
            let location: Vec<(Tag, f32)> = tags
                .iter()
                .zip(axes)
                .map(|(tag, (_, values))| (*tag, values[at] as f32))
                .collect();
            let key = location.iter().map(|&(tag, value)| (tag, value.to_bits())).collect();
            let id = *found.entry(key).or_insert_with(|| {
                models.push(font.at(&location));
                models.len() - 1
            });
            of_character.push(id);
        }
        Ok(Self { models, of_character })
    }
}

/// The runs of `line`: its consecutive characters at one location, as a byte range and the location. `of_character` has
/// the location of each character of the line.
fn runs(line: &str, of_character: &[usize]) -> Vec<(Range<usize>, usize)> {
    let mut runs: Vec<(Range<usize>, usize)> = Vec::new();
    for ((byte, character), &location) in line.char_indices().zip(of_character) {
        let end = byte + character.len_utf8();
        match runs.last_mut() {
            Some((range, last)) if *last == location => range.end = end,
            _ => runs.push((byte..end, location)),
        }
    }
    runs
}

/// The sizes that follow from a line's size in pixels to the em.
pub(super) struct Em {
    /// Pixels per font unit.
    pub(super) scale: f64,
    /// Pixels the tracking adds between clusters.
    pub(super) gap: f64,
    /// The height of a capital, in pixels.
    pub(super) cap_height: f64,
}

impl Em {
    /// The sizes of a line `size` pixels to the em with `tracking` thousandths of an em between clusters.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `tracking` if the gap comes out beyond the coordinates a path holds.
    pub(super) fn new(font: &Font, size: f64, tracking: f64) -> Result<Self, Error> {
        let metrics = font.model().metrics();
        let units_per_em = f64::from(metrics.units_per_em);
        let scale = size / units_per_em;
        Ok(Self {
            scale,
            gap: within("tracking", tracking, tracking * size / 1000.0)?,
            cap_height: metrics
                .cap_height
                .map(read_fonts::types::F48Dot16::to_f64)
                .filter(|height| *height > 0.0)
                .unwrap_or(CAP_HEIGHT * units_per_em)
                * scale,
        })
    }
}

/// The value of the axis `tag` for every character of a text of `characters`: the value fitted for each line on its
/// characters. The characters between lines (a break's whitespace) take the first line's, so that they ask for no
/// location of their own.
///
/// # Errors
/// [`Error::Parameter`] naming `fit` if the font has no such axis.
fn fitted_values(font: &Font, tag: &str, lines: &[lines::Line], characters: usize) -> Result<Vec<f64>, Error> {
    let first = match lines.iter().find_map(|line| line.axis) {
        Some(value) => value,
        None => f64::from(font.axis_range(tag, "fit")?.1),
    };
    let mut values = vec![first; characters];
    for line in lines {
        if let Some(value) = line.axis {
            values[line.characters.clone()].fill(value);
        }
    }
    Ok(values)
}

/// The features of `settings` as the shaper takes them.
///
/// # Errors
/// [`Error::Parameter`] naming `features` if a tag is not four printable ASCII characters.
pub(super) fn feature_tags(settings: &Settings) -> Result<Vec<(Tag, u32)>, Error> {
    settings
        .features
        .iter()
        .map(|(tag, value)| Ok((tag_of(tag, "features")?, *value)))
        .collect()
}

/// A shaper for the font at each of `places`.
pub(super) fn shapers<'a>(places: &'a Places, features: &[(Tag, u32)]) -> Vec<Shaper<'a>> {
    places.models.iter().map(|model| Shaper::new(model, features)).collect()
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
    /// Pixels per font unit: the size of its line over the font's units to the em.
    scale: f64,
}

/// A line shaped at one size: its glyphs, each with its location, how far the pen moves after each and the line's width.
pub(super) struct Measured {
    pub(super) glyphs: Vec<(Placed, usize)>,
    pub(super) advances: Vec<f64>,
    /// After each glyph: the tracking between clusters, not inside one, so that a mark stays over its base.
    pub(super) gaps: Vec<f64>,
    pub(super) width: f64,
}

impl Measured {
    /// `line` shaped, its glyphs scaled by `scale` and set `gap` pixels apart. `of_character` has the location of every
    /// character of the line.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `content` if a run cannot be shaped.
    pub(super) fn of(
        line: &str,
        of_character: &[usize],
        shapers: &[Shaper<'_>],
        scale: f64,
        gap: f64,
    ) -> Result<Self, Error> {
        let glyphs = shape_line(line, of_character, shapers)?;
        let advances: Vec<f64> = glyphs.iter().map(|(one, _)| f64::from(one.advance) * scale).collect();
        let gaps: Vec<f64> = glyphs
            .iter()
            .enumerate()
            .map(|(at, (one, _))| match glyphs.get(at + 1) {
                Some((next, _)) if next.cluster != one.cluster => gap,
                _ => 0.0,
            })
            .collect();
        let width = advances.iter().chain(&gaps).sum::<f64>();
        Ok(Self {
            glyphs,
            advances,
            gaps,
            width,
        })
    }
}

/// The shapes of `content` in `font`: one for every glyph that has an outline, left to right, line by line.
///
/// A line is cut into runs of consecutive characters at one location of the font's axes, and each run is shaped alone
/// at its location: kerning, ligatures and joins do not cross a change of location.
///
/// # Errors
/// [`Error::Parameter`] naming the step if the text cannot be shaped, a glyph cannot be drawn, a coordinate comes out
/// beyond ±1e100, an axis value is outside the font's range, or the draw has passed `MAX_SHAPES` or
/// `MAX_OUTLINE_SEGMENTS`.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "the text has at most MAX_GLYPHS characters, far fewer than 2^32 glyphs and lines, or 2^53 shapes"
)]
pub(crate) fn typeset(
    font: &Font,
    content: &str,
    settings: &Settings,
    wrap: Option<&Wrap>,
    cx: &Context<'_>,
) -> Result<Vec<Shape>, Error> {
    let placed = |error: Error| error.at_argument(&cx.place, None);
    let text_em = Em::new(font, settings.size, settings.tracking).map_err(placed)?;
    let features = feature_tags(settings).map_err(placed)?;
    let planned = lines::plan(font, content, wrap, cx).map_err(placed)?;
    let characters = content.chars().count();
    let fitted;
    let axes = match wrap.map(|wrap| &wrap.boundary) {
        Some(Boundary::Width {
            fit: Some(Fit::Axis(tag)),
            ..
        }) => {
            fitted = [
                &settings.axes[..],
                &[(
                    tag.clone(),
                    fitted_values(font, tag, &planned, characters).map_err(placed)?,
                )],
            ]
            .concat();
            &fitted
        }
        _ => &settings.axes,
    };
    let locations = Places::new(font, axes, characters).map_err(placed)?;
    let shapers = shapers(&locations, &features);
    let mut outlines: Vec<Outlines<'_>> = locations.models.iter().map(Outlines::new).collect();

    let mut glyphs = Vec::new();
    // the words met so far, and the one the glyph being placed is in (none after a space)
    let mut words = 0_u32;
    // lines of one size are one leading apart, counted from the first so that the sum is exact; lines of their own
    // sizes (a fit) stand 1.2 of their own size below the one before, unless the text gives a leading
    let evenly = settings.leading.is_some() || planned.iter().all(|line| line.size.is_none());
    let mut below = 0.0;
    for (number, line) in planned.iter().enumerate() {
        let mut word = None;
        let size = line.size.unwrap_or(settings.size);
        let own;
        let em = match line.size {
            Some(size) => {
                own = Em::new(font, size, settings.tracking).map_err(placed)?;
                &own
            }
            None => &text_em,
        };
        let (scale, gap, cap_height) = (em.scale, em.gap, em.cap_height);
        let leading = settings.leading.unwrap_or(LEADING * size);
        if number > 0 {
            below += leading;
        }
        // the plan puts a line in a shape; else it stands below the anchor by the leadings
        let (anchor_x, baseline) = if let Some(at) = line.at {
            (at.x, at.y)
        } else {
            let down = if evenly { number as f64 * leading } else { below };
            let baseline = within("leading", leading, down)
                .and_then(|lines| within("anchor", settings.anchor.y, settings.anchor.y + lines))
                .map_err(placed)?;
            (settings.anchor.x, baseline)
        };
        let measured = Measured::of(
            &content[line.bytes.clone()],
            &locations.of_character[line.characters.clone()],
            &shapers,
            scale,
            gap,
        )
        .map_err(placed)?;
        let mut pen = anchor_x
            - match settings.align {
                Align::Left => 0.0,
                Align::Center => measured.width / 2.0,
                Align::Right => measured.width,
            };
        for (((one, location), advance), gap) in measured.glyphs.iter().zip(&measured.advances).zip(&measured.gaps) {
            if one.character.is_whitespace() {
                word = None;
            } else if word.is_none() {
                word = Some(words);
                words += 1;
            }
            if let Some(outline) = outlines[*location].of(one.glyph).map_err(placed)? {
                // charged as each glyph is placed: a text that asks for a new location at every character cannot draw
                // an outline for each of them before the limit speaks
                cx.work.made_shapes(1, &cx.place)?;
                cx.work.made_outline(outline.len(), &cx.place)?;
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
                    line: number as u32,
                    centre: Point::new(pen + advance / 2.0, baseline - cap_height / 2.0),
                    scale,
                });
            }
            pen += advance + gap;
        }
    }
    shapes(&glyphs).map_err(placed)
}

/// The glyphs of `line`, run by run, each with the index of its location. `of_character` has the location of every
/// character of the line.
///
/// # Errors
/// [`Error::Parameter`] naming `content` if a run cannot be shaped.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a line has at most MAX_GLYPHS characters, far fewer than 2^32 bytes"
)]
fn shape_line(line: &str, of_character: &[usize], shapers: &[Shaper<'_>]) -> Result<Vec<(Placed, usize)>, Error> {
    let segment = Segment::of(line);
    let mut runs = runs(line, of_character);
    // a line read from the right has its first run rightmost, as its first character is
    if segment.backward() {
        runs.reverse();
    }
    let mut glyphs = Vec::new();
    for (range, location) in runs {
        let shaped = shapers[location].run(&line[range.clone()], range.start as u32, segment)?;
        glyphs.extend(shaped.into_iter().map(|one| (one, location)));
    }
    Ok(glyphs)
}

/// The shape of every glyph, put where its pen stood and carrying its attributes.
///
/// # Errors
/// [`Error::Parameter`] naming `size` if a coordinate comes out beyond ±1e100.
#[expect(
    clippy::cast_precision_loss,
    reason = "the text has at most MAX_GLYPHS characters, far fewer than 2^53 shapes"
)]
fn shapes(glyphs: &[Drawn]) -> Result<Vec<Shape>, Error> {
    let last = glyphs.len().saturating_sub(1);
    glyphs
        .iter()
        .map(|drawn| {
            let path = outline::path(&drawn.outline, drawn.origin, drawn.scale)?;
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
