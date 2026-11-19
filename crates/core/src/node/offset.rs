//! The offset steps: `offset` grows or shrinks the region each shape fills, `outline` turns each shape into the region
//! a stroke along it paints, `sweep` into the region it covers moving along a vector. Each is the `Path` method of the same name applied to each shape's path; the shape keeps
//! its style and attributes.

use super::boolean::TOLERANCE;
use super::style::{number, word};
use super::{Args, Context, Inputs, Node, choice, each_shape};
use crate::geometry::MAX_COORDINATE;
use crate::limits::MAX_PROFILE_STOPS;
use crate::require_round_stroke;
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
    summary: "turns every shape into the closed outlines a stroke `width` design pixels wide along it paints (as Path.outline); `join` and `cap` shape the corners and the ends of open lines, a mitre sharper than `miter_limit` is cut flat; curves become straight edges within `tolerance` design pixels; `profile` [[position, factor], ...] varies the width along each sub-path (position 0 to 1 by arc length, factors multiply `width`, linear between stops), with round joins and caps",
    args: &[
        number("width", None, Domain::PositiveUpTo(MAX_COORDINATE)),
        word("join", Some("round"), Join::NAMES),
        word("cap", Some("round"), Cap::NAMES),
        MITER_LIMIT,
        TOLERANCE,
        PROFILE,
    ],
    check: Some(profile_is_round),
};

/// Every shape turned into the region it covers moving along a vector.
#[derive(Debug)]
pub(crate) struct Sweep;

pub(crate) static SWEEP: Verb = Verb {
    name: "sweep",
    role: Role::Step,
    summary: "turns every shape into the region it covers as it moves dx to the right and dy down in a straight line, in design pixels (as Path.sweep): the region it fills, its moved copy and what every edge passes over, so a hole it crosses fills in; curves become straight edges within `tolerance` design pixels; no move is the region it fills",
    args: &[
        number("dx", None, Domain::Coordinate),
        number("dy", None, Domain::Coordinate),
        TOLERANCE,
    ],
    check: None,
};

/// The `profile` of `outline` and `stroke`: `[position, factor]` pairs, each number read for each shape.
pub(super) const PROFILE: Arg = Arg {
    name: "profile",
    kind: Kind::Pairs,
    default: Some(Literal::Unset),
    reads: Reads::PerItem,
    domain: Domain::Finite,
};

/// A `profile` has from 1 to `MAX_PROFILE_STOPS` stops and is stroked with round joins and caps: a stroke that varies
/// along its length is the sweep of a round disc.
pub(super) fn profile_is_round(args: &Args) -> Result<(), Error> {
    let Some(count) = args.pair_count("profile")? else {
        return Ok(());
    };
    if count == 0 || count > MAX_PROFILE_STOPS {
        return Err(Error::parameter(
            "profile",
            format!("must have from 1 to {MAX_PROFILE_STOPS} stops, got {count}"),
        ));
    }
    require_round_stroke(choice(args, "join", Join::named)?, choice(args, "cap", Cap::named)?)
}

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
            let path = match args.pairs(cx, "profile", item)? {
                Some(profile) => shape
                    .path()
                    .tapered_outline_charged(width, &profile, tolerance, &mut charge)?,
                None => shape
                    .path()
                    .outline_charged(width, join, cap, miter_limit, tolerance, &mut charge)?,
            };
            Ok(shape.with_path(path))
        })
    }
}

impl Node for Sweep {
    fn verb(&self) -> &'static Verb {
        &SWEEP
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        let tolerance = args.once_number(cx, "tolerance")?;
        each_shape(inputs.previous, cx, |shape, item| {
            let (dx, dy) = (args.number(cx, "dx", item)?, args.number(cx, "dy", item)?);
            Ok(shape.with_path(shape.path().sweep_charged(dx, dy, tolerance, &mut charge)?))
        })
    }
}
