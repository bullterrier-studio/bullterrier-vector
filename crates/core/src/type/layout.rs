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

/// One run of a text as drawn: a stretch of its characters with a font, a size and the settings of that font.
#[derive(Debug, Clone)]
pub(crate) struct RunSettings {
    /// The font the run is shaped in.
    pub(crate) font: Font,
    /// The characters of the content (a line break is one) the run covers.
    pub(crate) characters: Range<usize>,
    /// Pixels to the em.
    pub(crate) size: f64,
    /// Extra space between glyphs, in thousandths of an em.
    pub(crate) tracking: f64,
    /// The value of each axis given, by tag, for every character of the run: as many numbers as the run has characters.
    pub(crate) axes: Vec<(String, Vec<f64>)>,
    /// The features given, by tag, with their values.
    pub(crate) features: Vec<(String, u32)>,
}

/// The arguments of `text`, read for the draw.
#[derive(Debug, Clone)]
pub(crate) struct Settings {
    /// The runs, in order; together they cover the characters of the content.
    pub(crate) runs: Vec<RunSettings>,
    /// Pixels between baselines; `None` for 1.2 times the size of the largest run on the line.
    pub(crate) leading: Option<f64>,
    /// Where the lines stand against `anchor`.
    pub(crate) align: Align,
    /// The left end, middle or right end (by `align`) of the first baseline.
    pub(crate) anchor: Point,
}

impl Settings {
    /// The largest of `sizes` (one per run) over the runs that have a character in `characters`, or, for an empty stretch, of the run that
    /// holds its place (the last run if it lies past the end).
    fn size_over(&self, characters: &Range<usize>, sizes: &[f64]) -> f64 {
        let mut runs = self.runs.iter().zip(sizes);
        let touched = runs
            .clone()
            .filter(|(run, _)| run.characters.start < characters.end && characters.start < run.characters.end)
            .map(|(_, size)| *size)
            .reduce(f64::max);
        touched
            .or_else(|| {
                let holder = runs.clone().find(|(run, _)| run.characters.end > characters.start);
                holder.or_else(|| runs.next_back()).map(|(_, size)| *size)
            })
            .unwrap_or(0.0)
    }

    /// The largest size of any run.
    pub(super) fn largest(&self) -> f64 {
        self.runs.iter().map(|run| run.size).fold(0.0, f64::max)
    }

    /// The part of the runs that lies in `characters`, counted from its start: each run cut to the stretch, its axes
    /// with it. A run with no character there is left out.
    pub(super) fn sliced(&self, characters: &Range<usize>) -> Vec<RunSettings> {
        self.runs
            .iter()
            .filter_map(|run| {
                let start = run.characters.start.max(characters.start);
                let end = run.characters.end.min(characters.end);
                let (from, to) = (
                    start.checked_sub(run.characters.start)?,
                    end.checked_sub(run.characters.start)?,
                );
                (start < end).then(|| RunSettings {
                    characters: start - characters.start..end - characters.start,
                    axes: run
                        .axes
                        .iter()
                        .map(|(tag, values)| (tag.clone(), values[from..to].to_vec()))
                        .collect(),
                    ..run.clone()
                })
            })
            .collect()
    }
}

/// `error` with the run it came from named, when the text has several.
fn in_run(error: Error, run: usize, runs: usize) -> Error {
    match error {
        Error::Parameter { parameter, message } if runs > 1 => {
            Error::parameter(&parameter, format!("run {run}: {message}"))
        }
        other => other,
    }
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

/// The distinct locations a text asks for, and which one each character is at. A location is a run and the values of its
/// font's axes: runs are shaped apart, so no two runs share one.
pub(super) struct Places {
    /// The font of each location, at its axis values.
    pub(super) models: Vec<Model>,
    /// The index in the text's runs of each location's run.
    pub(super) run_of: Vec<usize>,
    /// For every character of the text, the index in `models` of its location.
    pub(super) of_character: Vec<usize>,
}

impl Places {
    /// The locations of the characters of `runs`, which cover the text from its first character. Characters of a run
    /// with equal values (as the shaper reads them, in `f32`) share a location; a run without axes has one, its font's own.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `axes` (and the run, if there are several) if a tag is not one of the font's or a
    /// value is outside its range.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "an axis value is inside the range of the font's axis, which the font stores as 16.16 fixed point"
    )]
    pub(super) fn new(runs: &[RunSettings]) -> Result<Self, Error> {
        let mut models = Vec::new();
        let mut run_of = Vec::new();
        let mut of_character = Vec::new();
        for (number, run) in runs.iter().enumerate() {
            let mut tags = Vec::new();
            for (tag, values) in &run.axes {
                run.font
                    .check_axis(tag, None)
                    .map_err(|error| in_run(error, number, runs.len()))?;
                // the lowest and the highest decide whether every value is in range
                for value in [
                    values.iter().copied().reduce(f64::min),
                    values.iter().copied().reduce(f64::max),
                ]
                .into_iter()
                .flatten()
                {
                    run.font
                        .check_axis(tag, Some(value))
                        .map_err(|error| in_run(error, number, runs.len()))?;
                }
                tags.push(tag_of(tag, "axes")?);
            }
            let mut found: BTreeMap<Vec<(Tag, u32)>, usize> = BTreeMap::new();
            for at in 0..run.characters.len() {
                let location: Vec<(Tag, f32)> = tags
                    .iter()
                    .zip(&run.axes)
                    .map(|(tag, (_, values))| (*tag, values[at] as f32))
                    .collect();
                let key = location.iter().map(|&(tag, value)| (tag, value.to_bits())).collect();
                let id = *found.entry(key).or_insert_with(|| {
                    models.push(run.font.at(&location));
                    run_of.push(number);
                    models.len() - 1
                });
                of_character.push(id);
            }
        }
        Ok(Self {
            models,
            run_of,
            of_character,
        })
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
#[derive(Debug, Clone)]
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

    /// The sizes of each location of `places`, every run at its entry of `sizes`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `tracking` if a gap comes out beyond the coordinates a path holds.
    pub(super) fn of_places(runs: &[RunSettings], places: &Places, sizes: &[f64]) -> Result<Vec<Self>, Error> {
        let own = runs
            .iter()
            .zip(sizes)
            .map(|(run, size)| Self::new(&run.font, *size, run.tracking))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(places.run_of.iter().map(|&run| own[run].clone()).collect())
    }
}

/// `runs` with the axis `tag` added to each whose font has it, set to the fraction `along(character)` (0 to 1, the
/// characters counted from the first of `runs`) of the axis's range in that font. A run that gives the axis itself
/// is refused at the call.
///
/// # Errors
/// [`Error::Parameter`] naming `fit` if no run's font has the axis.
pub(super) fn with_axis(
    runs: &[RunSettings],
    tag: &str,
    along: impl Fn(usize) -> f64,
) -> Result<Vec<RunSettings>, Error> {
    let mut found = false;
    let mut fitted = Vec::with_capacity(runs.len());
    for run in runs {
        let mut run = run.clone();
        if let Ok((min, _, max)) = run.font.axis_range(tag, "fit") {
            found = true;
            let (min, max) = (f64::from(min), f64::from(max));
            let values = run.characters.clone().map(|at| min + (max - min) * along(at)).collect();
            run.axes.push((tag.to_owned(), values));
        }
        fitted.push(run);
    }
    if found {
        Ok(fitted)
    } else if let Some(first) = runs.first() {
        first.font.axis_range(tag, "fit").map(|_| fitted)
    } else {
        Ok(fitted)
    }
}

/// How far along its axis each character of a text of `characters` is set: the fraction fitted for each line on its
/// characters. The characters between lines (a break's whitespace) take the first line's, so that they ask for no
/// location of their own; with no fitted line, the end of the range.
fn fitted_fractions(lines: &[lines::Line], characters: usize) -> Vec<f64> {
    let first = lines.iter().find_map(|line| line.axis).unwrap_or(1.0);
    let mut values = vec![first; characters];
    for line in lines {
        if let Some(value) = line.axis {
            values[line.characters.clone()].fill(value);
        }
    }
    values
}

/// The features of `run` as the shaper takes them.
///
/// # Errors
/// [`Error::Parameter`] naming `features` if a tag is not four printable ASCII characters.
fn feature_tags(run: &RunSettings) -> Result<Vec<(Tag, u32)>, Error> {
    run.features
        .iter()
        .map(|(tag, value)| Ok((tag_of(tag, "features")?, *value)))
        .collect()
}

/// A shaper for the font at each of `places`, with the features of its run.
///
/// # Errors
/// [`Error::Parameter`] naming `features` if a tag is not four printable ASCII characters.
pub(super) fn shapers<'a>(places: &'a Places, runs: &[RunSettings]) -> Result<Vec<Shaper<'a>>, Error> {
    let features = runs.iter().map(feature_tags).collect::<Result<Vec<_>, _>>()?;
    Ok(places
        .models
        .iter()
        .zip(&places.run_of)
        .map(|(model, &run)| Shaper::new(model, &features[run]))
        .collect())
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
    /// Pixels per font unit: the size of its run on its line over the font's units to the em.
    scale: f64,
    /// The run the glyph is in.
    run: u32,
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
    /// `line` shaped, each glyph scaled and set apart by the `ems` of its location. `of_character` has the location of
    /// every character of the line.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `content` if a run cannot be shaped.
    pub(super) fn of(line: &str, of_character: &[usize], shapers: &[Shaper<'_>], ems: &[Em]) -> Result<Self, Error> {
        let glyphs = shape_line(line, of_character, shapers)?;
        let advances: Vec<f64> = glyphs
            .iter()
            .map(|(one, location)| f64::from(one.advance) * ems[*location].scale)
            .collect();
        let gaps: Vec<f64> = glyphs
            .iter()
            .enumerate()
            .map(|(at, (one, location))| match glyphs.get(at + 1) {
                Some((next, _)) if next.cluster != one.cluster => ems[*location].gap,
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

/// The size of each run on `line`: its own, or, for a line the plan fitted, what the plan fixed at the reference pose
/// (the sizes drawn at do not move it).
fn line_sizes(line: &lines::Line, wrap: Option<&Wrap>, own: &[f64]) -> Vec<f64> {
    match (line.factor, wrap) {
        (Some(factor), Some(wrap)) => wrap.pose.runs.iter().map(|run| run.size * factor).collect(),
        _ => own.to_vec(),
    }
}

/// The leading of each of `planned`: the text's, or 1.2 times the size of the largest run on the line.
fn leadings(settings: &Settings, planned: &[lines::Line], wrap: Option<&Wrap>, own: &[f64]) -> Vec<f64> {
    planned
        .iter()
        .map(|line| {
            settings
                .leading
                .unwrap_or_else(|| LEADING * settings.size_over(&line.characters, &line_sizes(line, wrap, own)))
        })
        .collect()
}

/// The shapes of `content` set in the runs of `settings`: one for every glyph that has an outline, left to right, line
/// by line.
///
/// A line is cut into stretches of consecutive characters at one location (a run and its axis values), and each is
/// shaped alone at its location: kerning, ligatures and joins do not cross a change of location, so not a change of run.
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
    content: &str,
    settings: &Settings,
    wrap: Option<&Wrap>,
    cx: &Context<'_>,
) -> Result<Vec<Shape>, Error> {
    let placed = |error: Error| error.at_argument(&cx.place, None);
    let planned = lines::plan(content, wrap, cx).map_err(placed)?;
    let characters = content.chars().count();
    let fitted;
    let runs = match wrap.map(|wrap| &wrap.boundary) {
        Some(Boundary::Width {
            fit: Some(Fit::Axis(tag)),
            ..
        }) => {
            let along = fitted_fractions(&planned, characters);
            fitted = with_axis(&settings.runs, tag, |at| along[at]).map_err(placed)?;
            &fitted
        }
        _ => &settings.runs,
    };
    let locations = Places::new(runs).map_err(placed)?;
    let shapers = shapers(&locations, runs).map_err(placed)?;
    let mut outlines: Vec<Outlines<'_>> = locations.models.iter().map(Outlines::new).collect();
    let own_sizes: Vec<f64> = runs.iter().map(|run| run.size).collect();
    let text_ems = Em::of_places(runs, &locations, &own_sizes).map_err(placed)?;

    let mut glyphs = Vec::new();
    // the words met so far, and the one the glyph being placed is in (none after a space)
    let mut words = 0_u32;
    let leadings = leadings(settings, &planned, wrap, &own_sizes);
    // lines of one leading are that far apart, counted from the first so that the sum is exact; lines of their own
    // sizes (a fit, or runs of other sizes) stand 1.2 of their own size below the one before, unless the text gives a leading
    let evenly = leadings.windows(2).all(|pair| pair[0].total_cmp(&pair[1]).is_eq());
    let mut below = 0.0;
    for (number, line) in planned.iter().enumerate() {
        let mut word = None;
        let own;
        let ems = if line.factor.is_some() {
            own = Em::of_places(runs, &locations, &line_sizes(line, wrap, &own_sizes)).map_err(placed)?;
            &own
        } else {
            &text_ems
        };
        let leading = leadings[number];
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
            ems,
        )
        .map_err(placed)?;
        let mut pen = anchor_x
            - match settings.align {
                Align::Left => 0.0,
                Align::Center => measured.width / 2.0,
                Align::Right => measured.width,
            };
        for (((one, location), advance), gap) in measured.glyphs.iter().zip(&measured.advances).zip(&measured.gaps) {
            if lines::breaks(one.character) {
                word = None;
            } else if word.is_none() {
                word = Some(words);
                words += 1;
            }
            let em = &ems[*location];
            if let Some(outline) = outlines[*location].of(one.glyph).map_err(placed)? {
                // charged as each glyph is placed: a text that asks for a new location at every character cannot draw
                // an outline for each of them before the limit speaks
                cx.work.made_shapes(1, &cx.place)?;
                cx.work.made_outline(outline.len(), &cx.place)?;
                let origin = Point::new(
                    pen + f64::from(one.offset.0) * em.scale,
                    baseline - f64::from(one.offset.1) * em.scale,
                );
                glyphs.push(Drawn {
                    outline,
                    origin,
                    glyph: glyphs.len() as u32,
                    character: one.character,
                    word,
                    line: number as u32,
                    centre: Point::new(pen + advance / 2.0, baseline - em.cap_height / 2.0),
                    scale: em.scale,
                    run: locations.run_of[*location] as u32,
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
            attributes.run = Some(drawn.run);
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
