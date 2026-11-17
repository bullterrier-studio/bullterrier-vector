//! The sources that trace a map: `trace_cells` (exact), and the levels and labels of V14a.

use super::{Args, Context, Inputs, Node};
use crate::{Arg, Domain, Error, Field, Kind, Literal, Reads, Role, Shape, Verb};

/// The most colours a trace may be asked to keep apart.
// constant: a label is one byte, so a map has at most 256 colours
pub(crate) const MAX_COLORS: f64 = 256.0;

/// The colours a trace keeps apart unless told otherwise.
// constant: a dithered picture of a palette of 32 colours is already finer than a cut-paper or plotter piece wants
pub(crate) const DEFAULT_COLORS: f64 = 32.0;

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
    domain: Domain::PositiveUpTo(MAX_COLORS),
};

/// The map of `args`: a `Field.map`.
fn map_of(args: &Args) -> Result<&crate::field::MapData, Error> {
    let field: &Field = args
        .field("map")?
        .ok_or_else(|| Error::parameter("map", "is missing"))?;
    field
        .map_data()
        .ok_or_else(|| Error::parameter("map", "must be a Field.map: a grayscale or label map of bytes"))
}

/// The whole number of `name`, from 1.
fn whole(args: &Args, cx: &Context<'_>, name: &str) -> Result<usize, Error> {
    let value = args.once_number(cx, name)?;
    if value.fract() != 0.0 || value < 1.0 {
        return Err(Error::parameter(
            name,
            format!("must be a whole number from 1, got {value}"),
        ));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "whole, from 1, below 2^53"
    )]
    Ok(value as usize)
}

/// One shape for each colour of a map of cells.
#[derive(Debug)]
pub(crate) struct TraceCells;

pub(crate) static TRACE_CELLS: Verb = Verb {
    name: "trace_cells",
    role: Role::Source,
    summary: "traces a map of square cells exactly: the `map` (a Field.map) is read as a lattice of cells of `cell` pixels, each one colour (its byte, a palette index), and every colour becomes one shape (attribute `level` the colour byte, in order of the byte) whose straight edges lie on cell sides, cells that touch along a side merged, holes kept; nothing is fitted, so the shapes' areas are the cells' and they neither overlap nor leave a gap; more than `max_colors` colours is refused",
    args: &[
        MAP,
        Arg {
            name: "cell",
            kind: Kind::Number,
            default: None,
            reads: Reads::Literal,
            domain: Domain::AtLeast(1.0),
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
        let placed = |error: Error| error.at_argument(&cx.place, None);
        let (cell, colors) = (
            whole(args, cx, "cell").map_err(placed)?,
            whole(args, cx, "max_colors").map_err(placed)?,
        );
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        let outlines =
            crate::trace::trace_cells(map_of(args).map_err(placed)?, cell, colors, &mut charge).map_err(placed)?;
        shapes(outlines.into_iter().map(|(colour, path)| (f64::from(colour), path)), cx)
    }
}

/// The shapes of `outlines`, each with its `level`.
pub(crate) fn shapes(
    outlines: impl Iterator<Item = (f64, crate::Path)>,
    cx: &Context<'_>,
) -> Result<Vec<Shape>, Error> {
    let outlines: Vec<_> = outlines.collect();
    cx.work.made_shapes(outlines.len(), &cx.place)?;
    Ok(outlines
        .into_iter()
        .enumerate()
        .map(|(index, (level, path))| {
            let mut shape = Shape::from_path(path, index);
            shape.attributes_mut().level = Some(level);
            shape
        })
        .collect())
}
