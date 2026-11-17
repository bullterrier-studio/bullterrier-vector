//! The sources that trace a map: `trace_cells` (exact), and the levels and labels of V14a.

use super::args::Item;
use super::{Args, Context, Inputs, Node};
use crate::geometry::MAX_COORDINATE;
use crate::trace::Plan;
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

/// One shape for each level of a map.
#[derive(Debug)]
pub(crate) struct TraceLevels;

pub(crate) static TRACE_LEVELS: Verb = Verb {
    name: "trace_levels",
    role: Role::Source,
    summary: "traces a grayscale map at chosen levels: the `map` (a Field.map, 0 black to 1 white) gives one filled shape for each of `levels` (0 to 1) that it reaches, in the order given, the region where the map is at or above the level, with the level as its `level` attribute; the shapes are for stacking, one over the other, each painted by its level; a shape that meets the map's border goes on past it and closes outside, so no edge lies along the border; `smooth` blurs the map by that many pixels first, `close` then `open` (design pixels) fill and remove what is narrower than twice the radius, `min_hole` and `min_area` (square pixels) fill small holes and drop small parts, `tolerance` fits curves within that many pixels (0 keeps the polyline)",
    args: &[
        MAP,
        Arg {
            name: "levels",
            kind: Kind::Numbers,
            default: None,
            reads: Reads::Once,
            domain: Domain::Finite,
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

/// From 1 to `MAX_LEVELS` levels.
fn has_levels(args: &Args) -> Result<(), Error> {
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

    fn evaluate(&self, args: &Args, _: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let placed = |error: Error| error.at_argument(&cx.place, None);
        let levels = args
            .numbers(
                cx,
                "levels",
                Item {
                    index: 0,
                    count: 1,
                    order: 0.0,
                },
            )
            .map_err(placed)?;
        let plan = plan(args, cx).map_err(placed)?;
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        let outlines =
            crate::trace::trace_levels(map_of(args).map_err(placed)?, &levels, &plan, &mut charge).map_err(placed)?;
        shapes(outlines.into_iter(), cx)
    }
}

/// One shape for each label of a map.
#[derive(Debug)]
pub(crate) struct TraceLabels;

pub(crate) static TRACE_LABELS: Verb = Verb {
    name: "trace_labels",
    role: Role::Source,
    summary: "traces a map of colour labels: the `map` (a Field.map) holds a palette index, its byte, at every pixel, and each index it holds becomes one filled shape (attribute `level` the byte, in order of the byte) of the pixels with that index; more than `max_colors` indexes is refused; a shape that meets the map's border goes on past it and closes outside; `smooth`, `close`, `open`, `min_hole`, `min_area` and `tolerance` as for trace_levels, so the shapes of neighbouring labels meet within `tolerance` and can leave a hairline between them (stack them over a shape of the commonest colour)",
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
        let placed = |error: Error| error.at_argument(&cx.place, None);
        let colors = whole(args, cx, "max_colors").map_err(placed)?;
        let plan = plan(args, cx).map_err(placed)?;
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        let outlines =
            crate::trace::trace_labels(map_of(args).map_err(placed)?, colors, &plan, &mut charge).map_err(placed)?;
        shapes(outlines.into_iter().map(|(label, path)| (f64::from(label), path)), cx)
    }
}
