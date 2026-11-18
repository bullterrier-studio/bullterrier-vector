//! `characters`: a grid of characters, one outlined glyph in each cell.

use super::{Args, Context, Inputs, Node};
use crate::geometry::{MAX_COORDINATE, coordinate as within};
use crate::limits::MAX_GLYPHS;
use crate::r#type::{Vertical, string};
use crate::{Arg, Domain, Error, Font, Kind, Paint, Point, Reads, Role, Shape, Verb};
use std::collections::BTreeMap;

/// The characters of `rows` by row: `\n` ends a row.
fn lines(rows: &str) -> Vec<Vec<char>> {
    rows.split('\n').map(|line| line.chars().collect()).collect()
}

/// A grid of glyphs on a lattice of cells.
#[derive(Debug)]
pub(crate) struct CharactersNode;

pub(crate) static CHARACTERS: Verb = Verb {
    name: "characters",
    role: Role::Source,
    summary: "a grid of characters, one outlined glyph in each cell: `content` is the text, a row to a line (`\\n`), `cell` the width and height of a cell (one number for a square one); each character that has an outline becomes one shape (the glyph as `text` draws it, a character the font lacks as its notdef glyph; a space has none) standing in its cell with the middle of its advance on the middle of the cell across and the middle of a capital on the middle of the cell down, so a full stop stays on the baseline of the row; every shape has `char`, `row`, `column` (from 0), `order` (its place among the shapes, 0 to 1) and `origin` (the middle of its cell), and no paint unless `colors` gives one (rows of colours, one for each character of each row, spaces included: what bullterrier-image's `characters` returns); `font` is the bundled monospaced JetBrains Mono unless given; every character is set alone, so no ligature joins two cells; `size` is pixels to the em, by default the largest at which the widest glyph is within the cell across and the line box of the font (ascender to descender) within the cell down; a control character, more than 100,000 characters or colours that do not match the rows are refused",
    args: &[
        Arg {
            name: "content",
            kind: Kind::Text,
            default: None,
            reads: Reads::Literal,
            domain: Domain::Any,
        },
        Arg {
            name: "cell",
            kind: Kind::Factor,
            default: None,
            reads: Reads::Literal,
            domain: Domain::PositiveUpTo(MAX_COORDINATE),
        },
        Arg {
            name: "font",
            kind: Kind::Font,
            default: Some(crate::Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::Any,
        },
        Arg {
            name: "size",
            kind: Kind::Number,
            default: Some(crate::Literal::Unset),
            reads: Reads::Once,
            domain: Domain::Positive,
        },
        Arg {
            name: "colors",
            kind: Kind::Colors,
            default: Some(crate::Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::Any,
        },
    ],
    check: Some(grid_fits),
};

/// How far the lattice of `lines` reaches right and down, in the cell's own terms: a grid whose last cell lies beyond the
/// coordinates a path holds is refused naming `cell`.
fn lattice_reach(lines: &[Vec<char>], (width, height): (f64, f64)) -> Result<(f64, f64), Error> {
    let columns = lines.iter().map(Vec::len).max().unwrap_or(0);
    #[expect(clippy::cast_precision_loss, reason = "at most MAX_GLYPHS cells, far below 2^53")]
    let reach = (width * columns as f64, height * lines.len() as f64);
    within("cell", reach.0)?;
    within("cell", reach.1)?;
    Ok(reach)
}

/// The rows are at most `MAX_GLYPHS` characters with no control character, `colors` has a colour for each, and the
/// lattice stays within the coordinates.
fn grid_fits(args: &Args) -> Result<(), Error> {
    let rows = args.text("content")?;
    let characters = rows.chars().count();
    if characters > MAX_GLYPHS {
        return Err(Error::parameter(
            "content",
            format!("has {characters} characters, but MAX_GLYPHS {MAX_GLYPHS} is the most accepted"),
        ));
    }
    let lines = lines(rows);
    lattice_reach(
        &lines,
        args.literal_pair("cell")?
            .ok_or_else(|| Error::parameter("cell", "is missing"))?,
    )?;
    for (row, line) in lines.iter().enumerate() {
        if let Some(column) = line.iter().position(|character| character.is_control()) {
            return Err(Error::parameter(
                "content",
                format!(
                    "has the control character {:?} at row {row}, column {column}: a row has printable characters only",
                    line[column]
                ),
            ));
        }
    }
    if let Some(colors) = args.colors("colors")? {
        if colors.len() != lines.len() {
            return Err(Error::parameter(
                "colors",
                format!(
                    "has {} rows of colours, but rows has {} rows",
                    colors.len(),
                    lines.len()
                ),
            ));
        }
        for (row, (line, colors)) in lines.iter().zip(colors).enumerate() {
            if line.len() != colors.len() {
                return Err(Error::parameter(
                    "colors",
                    format!(
                        "row {row} has {} colours, but the row has {} characters",
                        colors.len(),
                        line.len()
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// The glyph shapes of each distinct character of the grid at `size`, standing at the origin.
struct Glyphs<'a> {
    font: &'a Font,
    size: f64,
    shapes: BTreeMap<char, Vec<Shape>>,
}

impl Glyphs<'_> {
    /// The shapes of `character` at the origin, set once.
    fn of(&mut self, character: char, cx: &Context<'_>) -> Result<&[Shape], Error> {
        if !self.shapes.contains_key(&character) {
            let set = string(
                self.font,
                &character.to_string(),
                ("size", self.size),
                Point::new(0.0, 0.0),
                cx,
            )?;
            self.shapes.insert(character, set);
        }
        Ok(&self.shapes[&character])
    }
}

/// The largest size at which the widest glyph of `characters` fits `cell` across and the line box down.
fn fitted(
    font: &Font,
    characters: impl Iterator<Item = char>,
    cell: (f64, f64),
    cx: &Context<'_>,
) -> Result<f64, Error> {
    let mut unit = Glyphs {
        font,
        size: 1.0,
        shapes: BTreeMap::new(),
    };
    let mut widest = 0.0_f64;
    for character in characters {
        // the pen stands at 0 and `origin` is the middle of the advance
        for shape in unit.of(character, cx)? {
            widest = widest.max(2.0 * shape.attributes().origin().map_or(0.0, |origin| origin.x));
        }
    }
    let vertical = Vertical::of(font);
    let reach = (vertical.above - vertical.cap / 2.0).max(vertical.below + vertical.cap / 2.0);
    Ok((cell.0 / widest).min(cell.1 / (2.0 * reach)))
}

impl Node for CharactersNode {
    fn verb(&self) -> &'static Verb {
        &CHARACTERS
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "a grid has at most MAX_GLYPHS cells, far below 2^32 and 2^53"
    )]
    fn evaluate(&self, args: &Args, _: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        // `own` places an error of this call's own arguments and is applied to nothing else: what takes `cx` (the glyph
        // layout, the work charges) returns errors placed already, and placing one twice would name the step twice
        let own = |error: Error| error.at_argument(&cx.place, None);
        let lines = lines(args.text("content").map_err(own)?);
        let font = &args.font_or_monospaced("font").map_err(own)?;
        let (width, height) = args
            .literal_pair("cell")
            .map_err(own)?
            .ok_or_else(|| own(Error::parameter("cell", "is missing")))?;
        let colors = args.colors("colors").map_err(own)?;
        let reach = lattice_reach(&lines, (width, height)).map_err(own)?;
        let used: std::collections::BTreeSet<char> = lines.iter().flatten().copied().collect();
        let size = match args.once_number_opt(cx, "size").map_err(own)? {
            Some(size) => size,
            None => fitted(font, used.iter().copied(), (width, height), cx)?,
        };
        let mut glyphs = Glyphs {
            font,
            size,
            shapes: BTreeMap::new(),
        };
        // charge every shape and outline segment before any is made
        let (mut shapes, mut segments, mut extent) = (0_usize, 0_usize, 0.0_f64);
        for character in lines.iter().flatten() {
            for shape in glyphs.of(*character, cx)? {
                shapes += 1;
                segments += shape.path().weight();
                if let Ok(bounds) = shape.path().bounds() {
                    extent = extent
                        .max(bounds.x.abs())
                        .max((bounds.x + bounds.width).abs())
                        .max(bounds.y.abs())
                        .max((bounds.y + bounds.height).abs());
                }
            }
        }
        // a glyph stands at its cell: no coordinate leaves the range if the lattice and the glyph (twice, for its own
        // `origin` taken off) are within it, which the size can break and the cell alone cannot
        within("size", reach.0.max(reach.1) + 2.0 * extent).map_err(own)?;
        cx.work.made_shapes(shapes, &cx.place)?;
        cx.work.made_outline(segments, &cx.place)?;
        let mut made = Vec::with_capacity(shapes);
        for (row, line) in lines.iter().enumerate() {
            for (column, character) in line.iter().enumerate() {
                let centre = Point::new(width * (column as f64 + 0.5), height * (row as f64 + 0.5));
                for shape in &glyphs.shapes[character] {
                    let origin = shape.attributes().origin().unwrap_or(centre);
                    let path = shape.path().translate(centre.x - origin.x, centre.y - origin.y)?;
                    let mut one = shape.with_path(path).at(made.len());
                    let attributes = one.attributes_mut();
                    attributes.glyph = None;
                    attributes.word = None;
                    attributes.line = None;
                    attributes.run = None;
                    attributes.row = Some(row as u32);
                    attributes.column = Some(column as u32);
                    attributes.order = Some(if shapes < 2 {
                        0.0
                    } else {
                        made.len() as f64 / (shapes - 1) as f64
                    });
                    attributes.origin = Some(centre);
                    if let Some(colors) = colors {
                        one.style_mut().set_fill(Paint::from(colors[row][column]));
                    }
                    made.push(one);
                }
            }
        }
        Ok(made)
    }
}
