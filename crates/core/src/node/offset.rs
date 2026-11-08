//! The offset steps: `offset` grows or shrinks the region each shape fills, `outline` turns each shape into the region
//! a stroke along it paints. Each is the `Path` method of the same name applied to each shape's path; the shape keeps
//! its style and attributes.

use super::boolean::TOLERANCE;
use super::style::{number, word};
use super::{Args, Context, Inputs, Node, choice, each_shape};
use crate::geometry::MAX_COORDINATE;
use crate::shape::{Cap, DEFAULT_MITER_LIMIT, Join};
use crate::{Arg, Domain, Error, Kind, Literal, Reads, Role, Shape, Verb};

/// Where a mitred corner is cut flat, SVG's ratio of the length of the point to the half-width.
const MITER_LIMIT: Arg = Arg {
    name: "miter_limit",
    kind: Kind::Number,
    default: Some(Literal::Number(DEFAULT_MITER_LIMIT)),
    reads: Reads::Literal,
    domain: Domain::AtLeast(1.0),
};

/// Every shape grown or shrunk.
#[derive(Debug)]
pub(crate) struct Offset;

pub(crate) static OFFSET: Verb = Verb {
    name: "offset",
    role: Role::Step,
    summary: "moves the edge of the region every shape fills outward by `distance` design pixels, inward when negative (as Path.offset); `join` shapes the corners that stick out, a mitre sharper than `miter_limit` is cut flat; curves become straight edges within `tolerance` design pixels",
    args: &[
        number("distance", None, Domain::Coordinate),
        word("join", Some("round"), Join::NAMES),
        MITER_LIMIT,
        TOLERANCE,
    ],
    check: None,
};

/// Every shape turned into the region a stroke along it paints.
#[derive(Debug)]
pub(crate) struct Outline;

pub(crate) static OUTLINE: Verb = Verb {
    name: "outline",
    role: Role::Step,
    summary: "turns every shape into the closed outlines a stroke `width` design pixels wide along it paints (as Path.outline); `join` and `cap` shape the corners and the ends of open lines, a mitre sharper than `miter_limit` is cut flat; curves become straight edges within `tolerance` design pixels",
    args: &[
        number("width", None, Domain::PositiveUpTo(MAX_COORDINATE)),
        word("join", Some("round"), Join::NAMES),
        word("cap", Some("round"), Cap::NAMES),
        MITER_LIMIT,
        TOLERANCE,
    ],
    check: None,
};

impl Node for Offset {
    fn verb(&self) -> &'static Verb {
        &OFFSET
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        let join = choice(args, "join", Join::named)?;
        let miter_limit = args.once_number(cx, "miter_limit")?;
        let tolerance = args.once_number(cx, "tolerance")?;
        each_shape(inputs.previous, cx, |shape, item| {
            let distance = args.number(cx, "distance", item)?;
            let path = shape
                .path()
                .offset_charged(distance, join, miter_limit, tolerance, &mut charge)?;
            Ok(shape.with_path(path))
        })
    }
}

impl Node for Outline {
    fn verb(&self) -> &'static Verb {
        &OUTLINE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        let join = choice(args, "join", Join::named)?;
        let cap = choice(args, "cap", Cap::named)?;
        let miter_limit = args.once_number(cx, "miter_limit")?;
        let tolerance = args.once_number(cx, "tolerance")?;
        each_shape(inputs.previous, cx, |shape, item| {
            let width = args.number(cx, "width", item)?;
            let path = shape
                .path()
                .outline_charged(width, join, cap, miter_limit, tolerance, &mut charge)?;
            Ok(shape.with_path(path))
        })
    }
}
