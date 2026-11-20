//! The sources that trace a map: `trace_cells` (exact), and the levels and labels of V14a.

use super::args::Item;
use super::{Args, Context, Inputs, Node};
use crate::field::Shapes;
use crate::geometry::MAX_COORDINATE;
use crate::trace::Plan;
use crate::{Arg, Domain, Error, Field, Kind, Literal, Reads, Rect, Role, Shape, Verb};

/// The most colours a trace may be asked to keep apart.
// constant: a label is one byte, so a map has at most 256 colours
pub(crate) const MAX_COLORS: f64 = 256.0;

/// The colours a trace keeps apart unless told otherwise.
// constant: a dithered picture of a palette of 32 colours is already finer than a cut-paper or plotter piece wants
pub(crate) const DEFAULT_COLORS: f64 = 32.0;

/// The largest cell a map holds.
// constant: a map has at most 2^24 pixels, so a cell of more pixels than that is never a divisor of one
const MAX_CELL_SIZE: f64 = 16_777_216.0;

const MAP: Arg = Arg {
    name: "map",
    kind: Kind::Field,
    default: None,
    reads: Reads::Literal,
    domain: Domain::Any,
};

const MAX_COLORS_ARG: Arg = Arg {
    name: "max_colors",
    kind: Kind::Number,
    default: Some(Literal::Number(DEFAULT_COLORS)),
    reads: Reads::Literal,
    domain: Domain::WholeBetween(1.0, MAX_COLORS),
};

/// A number read once, default `default`, from 0.
const fn tidy_arg(name: &'static str, default: f64) -> Arg {
    Arg {
        name,
        kind: Kind::Number,
        default: Some(Literal::Number(default)),
        reads: Reads::Once,
        domain: Domain::AtLeast(0.0),
    }
}

/// The tidying of a call: each number from 0 and at most a coordinate.
fn plan(args: &Args, cx: &Context<'_>) -> Result<Plan, Error> {
    let read = |name: &str| -> Result<f64, Error> {
        let value = args.once_number(cx, name)?;
        if value > MAX_COORDINATE {
            return Err(Error::parameter(
                name,
                format!("must be at most {MAX_COORDINATE:e}, got {value:e}"),
            ));
        }
        Ok(value)
    };
    Ok(Plan {
        smooth: read("smooth")?,
        tolerance: read("tolerance")?,
        open: read("open")?,
        close: read("close")?,
        min_area: read("min_area")?,
        min_hole: read("min_hole")?,
    })
}

/// The map of `args`: a `Field.map`.
fn map_of(args: &Args) -> Result<&crate::field::MapData, Error> {
    let field: &Field = args
        .field("map")?
        .ok_or_else(|| Error::parameter("map", "is missing"))?;
    field
        .map_data()
        .ok_or_else(|| Error::parameter("map", "must be a Field.map: a grayscale or label map of bytes"))
}

/// The whole number of `name`, which its domain keeps from 1.
fn whole(args: &Args, cx: &Context<'_>, name: &str) -> Result<usize, Error> {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "whole, from 1, below 2^53"
    )]
    args.once_number(cx, name).map(|value| value as usize)
}

/// One shape for each colour of a map of cells.
#[derive(Debug)]
pub(crate) struct TraceCells;

pub(crate) static TRACE_CELLS: Verb = Verb {
    name: "trace_cells",
    role: Role::Source,
    summary: "traces a map of square cells exactly: the `map` (a Field.map) is read as a lattice of cells of `cell` map pixels, each one colour (its byte, a palette index), and every colour becomes one shape (attribute `label` the colour byte, in order of the byte) whose straight edges lie on cell sides, cells that touch along a side merged, holes kept; nothing is fitted, so the shapes' areas are the cells' and they neither overlap nor leave a gap; more than `max_colors` colours is refused",
    args: &[
        MAP,
        Arg {
            name: "cell",
            kind: Kind::Number,
            default: None,
            reads: Reads::Literal,
            domain: Domain::WholeBetween(1.0, MAX_CELL_SIZE),
        },
        MAX_COLORS_ARG,
    ],
    check: None,
};

impl Node for TraceCells {
    fn verb(&self) -> &'static Verb {
        &TRACE_CELLS
    }

    fn evaluate(&self, args: &Args, _: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        let outlines = (|| {
            let (cell, colors) = (whole(args, cx, "cell")?, whole(args, cx, "max_colors")?);
            crate::trace::trace_cells(map_of(args)?, cell, colors, &mut charge)
        })()
        .map_err(|error| placed(error, cx))?;
        shapes(outlines, Attribute::Label, cx)
    }
}

/// `error` placed at this step, unless it is the charge of the draw's budget, which names the step itself.
fn placed(error: Error, cx: &Context<'_>) -> Error {
    match &error {
        Error::Parameter { parameter, .. } if *parameter == cx.place.to_string() => error,
        _ => error.at_argument(&cx.place, None),
    }
}

/// What a traced shape's number is.
#[derive(Clone, Copy)]
pub(crate) enum Attribute {
    /// A colour byte of a map of labels or cells.
    Label,
    /// A tone level.
    Level,
}

/// The shapes of `outlines`, each with its number as the attribute `attribute`.
pub(crate) fn shapes<N: Into<f64>>(
    outlines: Vec<(N, crate::Path)>,
    attribute: Attribute,
    cx: &Context<'_>,
) -> Result<Vec<Shape>, Error> {
    cx.work.made_shapes(outlines.len(), &cx.place)?;
    Ok(outlines
        .into_iter()
        .enumerate()
        .map(|(index, (number, path))| {
            let mut shape = Shape::from_path(path, index);
            let number: f64 = number.into();
            match attribute {
                Attribute::Level => shape.attributes_mut().level = Some(number),
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a byte")]
                Attribute::Label => shape.attributes_mut().label = Some(number as u32),
            }
            shape
        })
        .collect())
}

/// One shape for each level of a map.
#[derive(Debug)]
pub(crate) struct TraceLevels;

pub(crate) static TRACE_LEVELS: Verb = Verb {
    name: "trace_levels",
    role: Role::Source,
    summary: "traces a field at chosen levels: the `map` (a Field.map, 0 black to 1 white, or any field) gives one filled shape for each of `levels` that it reaches, in the order given, the region where the map is at or above the level, with the level as its `level` attribute; the shapes are for stacking, one over the other, each painted by its level; a shape that meets the map's border goes on past it and closes outside, so no edge lies along the border; `smooth` blurs the map first with a Gaussian of that standard deviation in design pixels, `close` then `open` (design pixels) fill and remove what is narrower than twice the radius, `min_hole` and `min_area` (square pixels) fill small holes and drop small parts, `tolerance` fits curves within that many design pixels (0 keeps the polyline); `levels` are numbers, Params or expressions, read once; the field is read at the centers of `frame` (`[x, y, width, height]`) cut into equal cells no wider than `cell` design pixels, by default a Field.map's own frame and pixels (any other field needs both); shapes reach past the frame by the padding (the larger of `open` and `close` plus `tolerance`, plus two cells), so crop them to the frame for plotting or cutting",
    args: &[
        MAP,
        Arg {
            name: "levels",
            kind: Kind::Numbers,
            default: None,
            reads: Reads::Once,
            domain: Domain::Finite,
        },
        Arg {
            name: "frame",
            kind: Kind::Rect,
            default: Some(Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::Coordinate,
        },
        Arg {
            name: "cell",
            kind: Kind::Number,
            default: Some(Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::PositiveUpTo(MAX_COORDINATE),
        },
        tidy_arg("smooth", 0.0),
        tidy_arg("tolerance", 0.5),
        tidy_arg("open", 0.0),
        tidy_arg("close", 0.0),
        tidy_arg("min_area", 0.0),
        tidy_arg("min_hole", 0.0),
    ],
    check: Some(has_levels),
};

/// From 1 to `MAX_LEVELS` levels, and a `frame` of four numbers if one is given.
fn has_levels(args: &Args) -> Result<(), Error> {
    if args.given("frame")? && args.count("frame")? != 4 {
        return Err(Error::parameter(
            "frame",
            format!(
                "must be four numbers (x, y, width, height), got {}",
                args.count("frame")?
            ),
        ));
    }
    let count = args.count("levels")?;
    if count == 0 || count > crate::trace::MAX_LEVELS {
        return Err(Error::parameter(
            "levels",
            format!("must have from 1 to {} levels, got {count}", crate::trace::MAX_LEVELS),
        ));
    }
    Ok(())
}

impl Node for TraceLevels {
    fn verb(&self) -> &'static Verb {
        &TRACE_LEVELS
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        let outlines = (|| {
            let item = Item {
                index: 0,
                count: 1,
                order: 0.0,
            };
            let levels = args.numbers(cx, "levels", item)?;
            // the field again with the values of the draw and the drawings it reads, as `hatch` reads its tone
            let field = args
                .field("map")?
                .ok_or_else(|| Error::parameter("map", "is missing"))?
                .bind(cx.env, &mut Shapes::taken(inputs.taken))?;
            let frame = if args.given("frame")? {
                match args.numbers(cx, "frame", item)?[..] {
                    [x, y, width, height] => Some(Rect { x, y, width, height }),
                    _ => return Err(Error::parameter("frame", "must be four numbers (x, y, width, height)")),
                }
            } else {
                None
            };
            let cell = args.once_number_opt(cx, "cell")?;
            crate::trace::trace_levels(&field, (frame, cell), &levels, &plan(args, cx)?, cx.grids, &mut charge)
        })()
        .map_err(|error| placed(error, cx))?;
        shapes(outlines, Attribute::Level, cx)
    }
}

/// One shape for each label of a map.
#[derive(Debug)]
pub(crate) struct TraceLabels;

pub(crate) static TRACE_LABELS: Verb = Verb {
    name: "trace_labels",
    role: Role::Source,
    summary: "traces a map of colour labels: the `map` (a Field.map) holds a palette index, its byte, at every pixel, and each index it holds becomes one filled shape (attribute `label` the byte) of the pixels with that index; the shapes tile the frame and its padding, never overlapping and leaving no gap: the commonest label comes first and is whatever the others leave, the others follow in order of the byte, each cut by the labels after it (so a tidied shape is a polyline within 0.01 pixels of its curve); more than `max_colors` indexes is refused, and so is cutting more outlines apart than the draw's segment budget allows; a shape that meets the map's border goes on past it and closes outside; `smooth`, `close`, `open`, `min_hole`, `min_area` and `tolerance` as for trace_levels",
    args: &[
        MAP,
        MAX_COLORS_ARG,
        tidy_arg("smooth", 0.0),
        tidy_arg("tolerance", 0.5),
        tidy_arg("open", 0.0),
        tidy_arg("close", 0.0),
        tidy_arg("min_area", 0.0),
        tidy_arg("min_hole", 0.0),
    ],
    check: None,
};

impl Node for TraceLabels {
    fn verb(&self) -> &'static Verb {
        &TRACE_LABELS
    }

    fn evaluate(&self, args: &Args, _: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        let outlines = (|| {
            let colors = whole(args, cx, "max_colors")?;
            crate::trace::trace_labels(map_of(args)?, colors, &plan(args, cx)?, &mut charge)
        })()
        .map_err(|error| placed(error, cx))?;
        shapes(outlines, Attribute::Label, cx)
    }
}
