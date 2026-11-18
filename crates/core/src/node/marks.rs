//! `marks`: the overlay marks of a heads-up display, corner brackets round boxes and crosshairs on points, each with an
//! optional label on a leader line.

use super::{Args, Context, Inputs, Node};
use crate::geometry::{MAX_COORDINATE, coordinate as within};
use crate::limits::{MAX_GLYPHS, MAX_MARKS};
use crate::r#type::{Vertical, string};
use crate::{Arg, Domain, Error, Kind, Literal, Path, Point, Reads, Role, Shape, Verb};

/// How far the label's pen stands to the right of the end of its leader, in ems of the label.
// constant: a quarter em is the width of a space in the common faces; it only keeps the leader's end off the first glyph
const LABEL_GAP: f64 = 0.25;

const fn coordinate(name: &'static str) -> Arg {
    Arg {
        name,
        kind: Kind::Number,
        default: None,
        reads: Reads::Once,
        domain: Domain::Coordinate,
    }
}

const fn extent(name: &'static str) -> Arg {
    Arg {
        name,
        kind: Kind::Number,
        default: None,
        reads: Reads::Once,
        domain: Domain::PositiveUpTo(MAX_COORDINATE),
    }
}

const LABEL: Arg = Arg {
    name: "text",
    kind: Kind::Text,
    default: Some(Literal::Unset),
    reads: Reads::Literal,
    domain: Domain::Any,
};

/// One box of `marks`: the arguments of a record of `boxes`, not a call of its own. It is never registered.
pub(crate) static BOX: Verb = Verb {
    name: "box",
    role: Role::Source,
    summary: "a box to bracket: its top left corner, its width and height, and an optional label",
    args: &[
        coordinate("x"),
        coordinate("y"),
        extent("width"),
        extent("height"),
        LABEL,
    ],
    check: Some(text_fits),
};

/// One point of `marks`: the arguments of a record of `points`, not a call of its own. It is never registered.
pub(crate) static POINT: Verb = Verb {
    name: "point",
    role: Role::Source,
    summary: "a point to mark with a crosshair: where it is, and an optional label",
    args: &[coordinate("x"), coordinate("y"), LABEL],
    check: Some(text_fits),
};

/// A label is one line of printable characters.
fn text_fits(args: &Args) -> Result<(), Error> {
    match args.text_opt("text")? {
        Some(label) if label.chars().any(char::is_control) => Err(Error::parameter(
            "text",
            format!("is {label:?}, but a label is one line of printable characters"),
        )),
        _ => Ok(()),
    }
}

/// The overlay marks of boxes and points.
#[derive(Debug)]
pub(crate) struct MarksNode;

pub(crate) static MARKS: Verb = Verb {
    name: "marks",
    role: Role::Source,
    summary: "overlay marks in the look of a heads-up display, in design pixels: a corner bracket in each corner of each of `boxes` (records of `x`, `y`, `width`, `height` and an optional `text`; an arm is `bracket` times the smaller side long, `bracket` up to a half), a crosshair on each of `points` (records of `x`, `y` and an optional `text`; each of its two lines reaches `cross` either side of the point), and for a mark with a `text` a leader line and the label: the leader runs from the box's top right corner, or the point, up and to the right by `cross` plus `text_size` each way, and the label (`text_size` pixels to the em, in `font`, one line; `font` is the bundled monospaced JetBrains Mono unless given) stands to the right of the leader's end with the middle of a capital on its height, so a label lies clear of its own box and crosshair; brackets, crosshairs and leaders are open lines of two or three points (set a stroke on them), a label's glyphs are the ones the `text` verb makes; every shape has `role` (`bracket`, `cross`, `leader` or `text`) and `mark` (which mark: the boxes in order, then the points), none has a paint; at most 10,000 marks",
    args: &[
        Arg {
            name: "boxes",
            kind: Kind::Boxes,
            default: Some(Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::Any,
        },
        Arg {
            name: "points",
            kind: Kind::Points,
            default: Some(Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::Any,
        },
        Arg {
            name: "bracket",
            kind: Kind::Number,
            default: Some(Literal::Number(0.25)),
            reads: Reads::Once,
            domain: Domain::PositiveUpTo(0.5),
        },
        Arg {
            name: "cross",
            kind: Kind::Number,
            default: Some(Literal::Number(12.0)),
            reads: Reads::Once,
            domain: Domain::PositiveUpTo(MAX_COORDINATE),
        },
        Arg {
            name: "text_size",
            kind: Kind::Number,
            default: Some(Literal::Number(12.0)),
            reads: Reads::Once,
            domain: Domain::PositiveUpTo(MAX_COORDINATE),
        },
        Arg {
            name: "font",
            kind: Kind::Font,
            default: Some(Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::Any,
        },
    ],
    check: Some(marks_fit),
};

/// At most `MAX_MARKS` marks, with at most `MAX_GLYPHS` characters of texts between them: the list that takes the
/// running total past a limit is the argument the error names.
fn marks_fit(args: &Args) -> Result<(), Error> {
    let (mut count, mut characters) = (0_usize, 0_usize);
    for list in ["boxes", "points"] {
        for record in args.records(list)? {
            count += 1;
            characters += record.text_opt("text")?.map_or(0, |text| text.chars().count());
            for (total, limit, limit_name, what) in [
                (count, MAX_MARKS, "MAX_MARKS", "marks"),
                (characters, MAX_GLYPHS, "MAX_GLYPHS", "characters of texts"),
            ] {
                if total > limit {
                    return Err(Error::parameter(
                        list,
                        format!(
                            "with the marks before it has more than {limit} {what}, but {limit_name} {limit} is the most accepted"
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// What a mark is drawn with.
struct Look {
    bracket: f64,
    cross: f64,
    text_size: f64,
}

/// The open line through `points`, as a shape of `role` for mark `mark`.
fn line(points: &[Point], role: &str, mark: u32) -> Result<Shape, Error> {
    let mut path = Path::start(points[0])?;
    for point in &points[1..] {
        path = path.line_to(*point)?;
    }
    let mut shape = Shape::from_path(path, 0);
    shape.attributes_mut().role = Some(role.into());
    shape.attributes_mut().mark = Some(mark);
    Ok(shape)
}

impl Node for MarksNode {
    fn verb(&self) -> &'static Verb {
        &MARKS
    }

    #[expect(clippy::cast_possible_truncation, reason = "at most MAX_MARKS marks, far below 2^32")]
    fn evaluate(&self, args: &Args, _: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        // `own` places an error of this call's own arguments and is applied to nothing else: what takes `cx` (the
        // work charges, the text layout) returns errors placed already
        let own = |error: Error| error.at_argument(&cx.place, None);
        let look = Look {
            bracket: args.once_number(cx, "bracket").map_err(own)?,
            cross: args.once_number(cx, "cross").map_err(own)?,
            text_size: args.once_number(cx, "text_size").map_err(own)?,
        };
        let font = &args.font_or_monospaced("font")?;
        let (boxes, points) = (args.records("boxes")?, args.records("points")?);
        // the lines are charged before any is made; a label charges its own glyphs
        cx.work.made_shapes(boxes.len() * 5 + points.len() * 3, &cx.place)?;
        let cap = Vertical::of(font).cap * look.text_size;
        // every coordinate a line has is one of these sums, each checked against the argument that added to it
        let reached = |argument: &str, base: f64, by: f64| within(argument, base + by).map_err(own);
        let lead = reached("text_size", look.cross, look.text_size)?;
        let mut made = Vec::new();
        for (mark, record) in boxes.iter().chain(points).enumerate() {
            let number = |name: &str| record.once_number(cx, name).map_err(own);
            let mark = mark as u32;
            let (x, y) = (number("x")?, number("y")?);
            let anchor = if mark as usize >= boxes.len() {
                let (west, east) = (reached("cross", x, -look.cross)?, reached("cross", x, look.cross)?);
                let (north, south) = (reached("cross", y, -look.cross)?, reached("cross", y, look.cross)?);
                made.push(line(&[Point::new(west, y), Point::new(east, y)], "cross", mark)?);
                made.push(line(&[Point::new(x, north), Point::new(x, south)], "cross", mark)?);
                Point::new(x, y)
            } else {
                let (width, height) = (number("width")?, number("height")?);
                let (right, bottom) = (reached("width", x, width)?, reached("height", y, height)?);
                let arm = look.bracket * width.min(height);
                for (corner, across, down) in [
                    (Point::new(x, y), arm, arm),
                    (Point::new(right, y), -arm, arm),
                    (Point::new(right, bottom), -arm, -arm),
                    (Point::new(x, bottom), arm, -arm),
                ] {
                    let points = [
                        Point::new(corner.x + across, corner.y),
                        corner,
                        Point::new(corner.x, corner.y + down),
                    ];
                    made.push(line(&points, "bracket", mark)?);
                }
                Point::new(right, y)
            };
            let Some(text) = record.text_opt("text").map_err(own)?.filter(|text| !text.is_empty()) else {
                continue;
            };
            let end = Point::new(
                reached("text_size", anchor.x, lead)?,
                reached("text_size", anchor.y, -lead)?,
            );
            made.push(line(&[anchor, end], "leader", mark)?);
            let pen = Point::new(end.x + LABEL_GAP * look.text_size, end.y + cap / 2.0);
            for mut glyph in string(font, text, ("text_size", look.text_size), pen, cx)? {
                glyph.attributes_mut().role = Some("text".into());
                glyph.attributes_mut().mark = Some(mark);
                made.push(glyph);
            }
        }
        Ok(made
            .into_iter()
            .enumerate()
            .map(|(index, shape)| shape.at(index))
            .collect())
    }
}
