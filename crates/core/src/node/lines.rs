//! The line-family steps: `rows` (ridgelines), each the `Path` method of the same name applied to each shape's path.

use super::style::word;
use super::{Args, Context, Inputs, Node, each_shape};
use crate::field::Shapes;
use crate::geometry::MAX_COORDINATE;
use crate::lines::{Family, Plan, StipplePlan};
use crate::{Arg, Bands, Domain, Error, Kind, Literal, Point, Reads, Role, Shape, Verb};

/// A number read once for the whole call, within `domain`.
const fn once(name: &'static str, domain: Domain) -> Arg {
    Arg {
        name,
        kind: Kind::Number,
        default: None,
        reads: Reads::Once,
        domain,
    }
}

const fn required_field(name: &'static str) -> Arg {
    Arg {
        name,
        kind: Kind::Field,
        default: None,
        reads: Reads::Literal,
        domain: Domain::Any,
    }
}

/// Every shape replaced by ridgelines.
#[derive(Debug)]
pub(crate) struct RowsStep;

pub(crate) static ROWS: Verb = Verb {
    name: "rows",
    role: Role::Step,
    summary: "replaces every shape by horizontal rows `spacing` design pixels apart inside the region it fills (the first half a spacing below its top), each lifted towards the top by `amplitude` times the `tone` field at the point of the row it lifts (as Path.rows: a negative amplitude pushes down), and cut where a row in front (lower in the picture) passes over it, so a map of tone reads as a landscape of lines; the result is open lines, so set the stroke after it",
    args: &[
        once("spacing", Domain::PositiveUpTo(MAX_COORDINATE)),
        required_field("tone"),
        once("amplitude", Domain::Coordinate),
    ],
    check: None,
};

impl Node for RowsStep {
    fn verb(&self) -> &'static Verb {
        &ROWS
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let placed = |error: Error| error.at_argument(&cx.place, None);
        let spacing = args.once_number(cx, "spacing").map_err(placed)?;
        let amplitude = args.once_number(cx, "amplitude").map_err(placed)?;
        let mut shapes = Shapes::taken(inputs.taken);
        let tone = args
            .field("tone")?
            .ok_or_else(|| Error::parameter("tone", "is missing"))?
            .bind(cx.env, &mut shapes)
            .map_err(placed)?;
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        each_shape(inputs.previous, cx, |shape, _| {
            Ok(shape.with_path(shape.path().rows_charged(spacing, amplitude, &tone, &mut charge)?))
        })
    }
}

/// A number read once, which a call may leave out.
const fn optional(name: &'static str, domain: Domain) -> Arg {
    Arg {
        name,
        kind: Kind::Number,
        default: Some(Literal::Unset),
        reads: Reads::Once,
        domain,
    }
}

/// Every shape replaced by bands.
#[derive(Debug)]
pub(crate) struct BandsStep;

pub(crate) static BANDS: Verb = Verb {
    name: "bands",
    role: Role::Step,
    summary: "lines whose width follows a tone: replaces every shape by closed outlines of lines laid over the region it fills and cut to it (as Path.bands), as wide at each point as the `tone` field says: `width` is a pair, the width where the tone is 0 and where it is 1; `family` is the carrier: `lines` `spacing` apart along `angle` degrees, `waves` (`wavelength` long and `amplitude` high, in rows: multiples of `spacing`), `rings` round `centre`, a `spiral` out of `centre` with arms `spacing` apart, or `rays` out of `centre` as many as are `spacing` apart at the farthest corner; `centre` defaults to the middle of the region's bounding box; the result is closed outlines, so set the fill after it",
    args: &[
        once("spacing", Domain::PositiveUpTo(MAX_COORDINATE)),
        required_field("tone"),
        Arg {
            name: "width",
            kind: Kind::Pair,
            default: None,
            reads: Reads::Once,
            domain: Domain::AtLeast(0.0),
        },
        word("family", Some("lines"), Family::NAMES),
        optional("angle", Domain::Coordinate),
        optional("wavelength", Domain::PositiveUpTo(MAX_COORDINATE)),
        optional("amplitude", Domain::Coordinate),
        Arg {
            name: "centre",
            kind: Kind::Point,
            default: Some(Literal::Unset),
            reads: Reads::Once,
            domain: Domain::Coordinate,
        },
    ],
    check: Some(waves_need_their_size),
};

/// `waves` need a `wavelength` and an `amplitude`, and no family is given what it does not read.
fn waves_need_their_size(args: &Args) -> Result<(), Error> {
    let family = args.text("family")?;
    if family == "waves" {
        for name in ["wavelength", "amplitude"] {
            if !args.given(name)? {
                return Err(Error::parameter(name, "is needed when `family` is \"waves\""));
            }
        }
    }
    for name in ["angle", "wavelength", "amplitude", "centre"] {
        if args.given(name)? {
            Bands::refuse_unread(family, name)?;
        }
    }
    Ok(())
}

impl Node for BandsStep {
    fn verb(&self) -> &'static Verb {
        &BANDS
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let placed = |error: Error| error.at_argument(&cx.place, None);
        let family = Family::from_name(args.text("family")?)
            .ok_or_else(|| Error::parameter("family", "is not one of the carriers"))?;
        let plan = Plan {
            family,
            spacing: args.once_number(cx, "spacing").map_err(placed)?,
            width: args
                .once_pair(cx, "width")
                .map_err(placed)?
                .ok_or_else(|| Error::parameter("width", "is missing"))?,
            angle: args.once_number_opt(cx, "angle").map_err(placed)?.unwrap_or(0.0),
            wavelength: args.once_number_opt(cx, "wavelength").map_err(placed)?,
            amplitude: args.once_number_opt(cx, "amplitude").map_err(placed)?,
            centre: args.once_point(cx, "centre").map_err(placed)?.map(|p: Point| p),
        };
        let mut shapes = Shapes::taken(inputs.taken);
        let tone = args
            .field("tone")?
            .ok_or_else(|| Error::parameter("tone", "is missing"))?
            .bind(cx.env, &mut shapes)
            .map_err(placed)?;
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        each_shape(inputs.previous, cx, |shape, _| {
            Ok(shape.with_path(shape.path().bands_charged(&plan, &tone, &mut charge)?))
        })
    }
}

/// Every shape replaced by dots.
#[derive(Debug)]
pub(crate) struct StippleStep;

pub(crate) static STIPPLE: Verb = Verb {
    name: "stipple",
    role: Role::Step,
    summary: "replaces every shape by round dots inside the region it fills (as Path.stipple): a square lattice `spacing` apart with one dot at most in each cell, standing within 0.4 spacings of the cell's middle; `radius` is the dot's radius in design pixels, one number or a pair (the radius where the `tone` field is 0 and where it is 1, read at the dot's centre); `density` is a pair, the share of cells that get a dot where the `tone` field is 0 and where it is 1 (default [1, 0]: dark is dense), and the dots of a tone are spread evenly; `seed` moves them; the result is closed dots, so set the fill after it",
    args: &[
        once("spacing", Domain::PositiveUpTo(MAX_COORDINATE)),
        required_field("tone"),
        Arg {
            name: "radius",
            kind: Kind::Factor,
            default: None,
            reads: Reads::Once,
            domain: Domain::PositiveUpTo(MAX_COORDINATE),
        },
        Arg {
            name: "density",
            kind: Kind::Pair,
            default: Some(Literal::Unset),
            reads: Reads::Once,
            domain: Domain::Fraction,
        },
        Arg {
            name: "seed",
            kind: Kind::Number,
            default: Some(Literal::Number(0.0)),
            reads: Reads::Once,
            domain: Domain::Coordinate,
        },
    ],
    check: None,
};

impl Node for StippleStep {
    fn verb(&self) -> &'static Verb {
        &STIPPLE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let placed = |error: Error| error.at_argument(&cx.place, None);
        let plan = StipplePlan {
            spacing: args.once_number(cx, "spacing").map_err(placed)?,
            radius: args
                .once_pair(cx, "radius")
                .map_err(placed)?
                .ok_or_else(|| Error::parameter("radius", "is missing"))?,
            density: args.once_pair(cx, "density").map_err(placed)?.unwrap_or((1.0, 0.0)),
            seed: args.once_number(cx, "seed").map_err(placed)?,
        };
        let mut shapes = Shapes::taken(inputs.taken);
        let tone = args
            .field("tone")?
            .ok_or_else(|| Error::parameter("tone", "is missing"))?
            .bind(cx.env, &mut shapes)
            .map_err(placed)?;
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        each_shape(inputs.previous, cx, |shape, _| {
            Ok(shape.with_path(shape.path().stipple_charged(&plan, &tone, &mut charge)?))
        })
    }
}
