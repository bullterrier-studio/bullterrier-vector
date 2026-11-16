//! The geometry steps: `translate`, `rotate`, `scale`, `along`, `slice`, `resample`, `dash`, `wave`, `wobble`. Each applies the `Path` verb of the same name to every
//! shape.

use super::{Args, Context, Inputs, Node, each_shape};
use crate::geometry::MAX_COORDINATE;
use crate::limits::MAX_DASH_PATTERN;
use crate::path::{Guide, rotate_point, scale_point};
use crate::{Arg, Domain, Error, Kind, Literal, Point, Reads, Role, Shape, Verb};

/// A number read for each shape, within `domain`.
const fn per_shape(name: &'static str, kind: Kind, default: Option<Literal>, domain: Domain) -> Arg {
    Arg {
        name,
        kind,
        default,
        reads: Reads::PerItem,
        domain,
    }
}

/// The point a shape turns or scales about: `about` if given, else its `origin` attribute, else (`None`) the middle of its
/// bounds.
fn pivot(shape: &Shape, about: Option<Point>) -> Option<Point> {
    about.or(shape.attributes().origin())
}

/// Moves every shape.
#[derive(Debug)]
pub(crate) struct Translate;

pub(crate) static TRANSLATE: Verb = Verb {
    name: "translate",
    role: Role::Step,
    summary: "moves every shape, and its origin, by dx to the right and dy down, in design pixels",
    args: &[
        per_shape("dx", Kind::Number, None, Domain::Coordinate),
        per_shape("dy", Kind::Number, None, Domain::Coordinate),
    ],
    check: None,
};

impl Node for Translate {
    fn verb(&self) -> &'static Verb {
        &TRANSLATE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        each_shape(inputs.previous, cx, |shape, item| {
            let (dx, dy) = (args.number(cx, "dx", item)?, args.number(cx, "dy", item)?);
            let mut moved = shape.with_path(shape.path().translate(dx, dy)?);
            if let Some(origin) = shape.attributes().origin() {
                moved.attributes_mut().origin = Some(Point::new(origin.x + dx, origin.y + dy));
            }
            Ok(moved)
        })
    }
}

/// Turns every shape.
#[derive(Debug)]
pub(crate) struct Rotate;

pub(crate) static ROTATE: Verb = Verb {
    name: "rotate",
    role: Role::Step,
    summary: "turns every shape clockwise on the screen by angle degrees, any size, about a point (default: its origin, else the middle of its bounds); the origin turns with it",
    args: &[
        per_shape("angle", Kind::Number, None, Domain::Finite),
        per_shape("about", Kind::Point, Some(Literal::Unset), Domain::Coordinate),
    ],
    check: None,
};

impl Node for Rotate {
    fn verb(&self) -> &'static Verb {
        &ROTATE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        each_shape(inputs.previous, cx, |shape, item| {
            let angle = args.number(cx, "angle", item)?;
            let about = pivot(shape, args.point(cx, "about", item)?);
            let mut turned = shape.with_path(shape.path().rotate(angle, about)?);
            if let (Some(origin), Some(about)) = (shape.attributes().origin(), about) {
                turned.attributes_mut().origin = Some(rotate_point(origin, angle, about)?);
            }
            Ok(turned)
        })
    }
}

/// Scales every shape.
#[derive(Debug)]
pub(crate) struct Scale;

pub(crate) static SCALE: Verb = Verb {
    name: "scale",
    role: Role::Step,
    summary: "scales every shape by a number, or by an (x, y) pair, about a point (default: its origin, else the middle of its bounds); the origin scales with it; a negative factor mirrors",
    args: &[
        per_shape("by", Kind::Factor, None, Domain::NonZero),
        per_shape("about", Kind::Point, Some(Literal::Unset), Domain::Coordinate),
    ],
    check: None,
};

impl Node for Scale {
    fn verb(&self) -> &'static Verb {
        &SCALE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        each_shape(inputs.previous, cx, |shape, item| {
            let by = args.pair(cx, "by", item)?;
            let about = pivot(shape, args.point(cx, "about", item)?);
            let mut scaled = shape.with_path(shape.path().scale(by, about)?);
            if let (Some(origin), Some(about)) = (shape.attributes().origin(), about) {
                scaled.attributes_mut().origin = Some(scale_point(origin, by.into(), about)?);
            }
            Ok(scaled)
        })
    }
}

/// Carries every shape along a guide.
#[derive(Debug)]
pub(crate) struct Along;

pub(crate) static ALONG: Verb = Verb {
    name: "along",
    role: Role::Step,
    summary: "carries every shape along a guide path: x runs along it from start (a fraction of its length, a closed guide wraps, an open one goes on straight), y to its right; a closed guide is walked from the point nearest near (default: its top)",
    args: &[
        Arg {
            name: "path",
            kind: Kind::Path,
            default: None,
            reads: Reads::Literal,
            domain: Domain::Any,
        },
        per_shape("start", Kind::Number, Some(Literal::Number(0.0)), Domain::Finite),
        per_shape("near", Kind::Point, Some(Literal::Unset), Domain::Coordinate),
    ],
    check: None,
};

impl Node for Along {
    fn verb(&self) -> &'static Verb {
        &ALONG
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let path = args.path("path")?;
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        // a guide walked from its own start is the same for every shape: walk it once
        let mut from_start: Option<Guide> = None;
        each_shape(inputs.previous, cx, |shape, item| {
            let start = args.number(cx, "start", item)?;
            let near = args.point(cx, "near", item)?;
            if near.is_none() && from_start.is_none() {
                from_start = Some(Guide::new("along", path, None, &mut charge)?);
            }
            let own;
            let guide = match (near, &from_start) {
                (None, Some(guide)) => guide,
                (near, _) => {
                    own = Guide::new("along", path, near, &mut charge)?;
                    &own
                }
            };
            Ok(shape.with_path(shape.path().bent(guide, start, &mut charge)?))
        })
    }
}

/// Keeps the piece of every shape between two positions along it.
#[derive(Debug)]
pub(crate) struct Slice;

pub(crate) static SLICE: Verb = Verb {
    name: "slice",
    role: Role::Step,
    summary: "keeps the piece of every shape between the positions start and end (fractions 0 to 1 of its length, start not after end): slice(0, progress) draws it on; a closed outline is measured from its top, or from the point nearest near; a shape with nothing in it stays",
    args: &[
        per_shape("start", Kind::Number, None, Domain::Fraction),
        per_shape("end", Kind::Number, None, Domain::Fraction),
        per_shape("near", Kind::Point, Some(Literal::Unset), Domain::Coordinate),
    ],
    check: None,
};

impl Node for Slice {
    fn verb(&self) -> &'static Verb {
        &SLICE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        each_shape(inputs.previous, cx, |shape, item| {
            let (start, end) = (args.number(cx, "start", item)?, args.number(cx, "end", item)?);
            let near = args.point(cx, "near", item)?;
            // a shape with nothing in it has no piece to keep, and stays so the list keeps its numbering
            let weight = shape.path().weight();
            if weight == 0 {
                return Ok(shape.clone());
            }
            cx.work.made_segments(weight, &place)?;
            Ok(shape.with_path(shape.path().slice(start, end, near)?))
        })
    }
}

/// Replaces every shape by straight segments with corners a fixed distance apart along it.
#[derive(Debug)]
pub(crate) struct Resample;

pub(crate) static RESAMPLE: Verb = Verb {
    name: "resample",
    role: Role::Step,
    summary: "replaces every shape by straight segments with corners spacing design pixels apart along it, from the start of each sub-path (a closed one stays closed, an open one keeps its end), so dots placed on the corners are evenly spaced",
    args: &[per_shape(
        "spacing",
        Kind::Number,
        None,
        Domain::PositiveUpTo(MAX_COORDINATE),
    )],
    check: None,
};

impl Node for Resample {
    fn verb(&self) -> &'static Verb {
        &RESAMPLE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        each_shape(inputs.previous, cx, |shape, item| {
            let spacing = args.number(cx, "spacing", item)?;
            Ok(shape.with_path(shape.path().resample_charged(spacing, &mut charge)?))
        })
    }
}

/// Cuts every shape into the pieces a dash pattern leaves.
#[derive(Debug)]
pub(crate) struct Dash;

pub(crate) static DASH: Verb = Verb {
    name: "dash",
    role: Role::Step,
    summary: "cuts every shape into the pieces a dash pattern [on, off, ...] (design pixels, each above 0; an odd list repeats itself) leaves, from the start of each sub-path (a closed one from its top, or the point nearest near); phase, in cycles of the whole pattern, moves the dashes forward along the path and wraps; the pieces are open, a closed outline wholly inside one dash stays closed",
    args: &[
        per_shape("pattern", Kind::Numbers, None, Domain::PositiveUpTo(MAX_COORDINATE)),
        per_shape("phase", Kind::Number, Some(Literal::Number(0.0)), Domain::Finite),
        per_shape("near", Kind::Point, Some(Literal::Unset), Domain::Coordinate),
    ],
    check: Some(pattern_has_lengths),
};

/// The `pattern` has from 1 to `MAX_DASH_PATTERN` lengths.
fn pattern_has_lengths(args: &Args) -> Result<(), Error> {
    let count = args.count("pattern")?;
    if count == 0 || count > MAX_DASH_PATTERN {
        return Err(Error::parameter(
            "pattern",
            format!("must have from 1 to {MAX_DASH_PATTERN} lengths, got {count}"),
        ));
    }
    Ok(())
}

impl Node for Dash {
    fn verb(&self) -> &'static Verb {
        &DASH
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        each_shape(inputs.previous, cx, |shape, item| {
            let pattern = args.numbers(cx, "pattern", item)?;
            let phase = args.number(cx, "phase", item)?;
            let near = args.point(cx, "near", item)?;
            Ok(shape.with_path(shape.path().dash_charged(&pattern, phase, near, &mut charge)?))
        })
    }
}

/// Replaces every shape by few smooth curves within a tolerance of it.
#[derive(Debug)]
pub(crate) struct Simplify;

pub(crate) static SIMPLIFY: Verb = Verb {
    name: "simplify",
    role: Role::Step,
    summary: "replaces every shape by few smooth curves (sharp corners kept, a turn past 45 degrees) within tolerance design pixels of it, so a dense polyline becomes a few smooth curves",
    args: &[per_shape(
        "tolerance",
        Kind::Number,
        Some(Literal::Number(0.5)),
        Domain::PositiveUpTo(MAX_COORDINATE),
    )],
    check: None,
};

impl Node for Simplify {
    fn verb(&self) -> &'static Verb {
        &SIMPLIFY
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        each_shape(inputs.previous, cx, |shape, item| {
            let tolerance = args.number(cx, "tolerance", item)?;
            Ok(shape.with_path(shape.path().simplify_charged(tolerance, &mut charge)?))
        })
    }
}

/// Runs every shape the other way.
#[derive(Debug)]
pub(crate) struct Reverse;

pub(crate) static REVERSE: Verb = Verb {
    name: "reverse",
    role: Role::Step,
    summary: "runs every shape the other way: its sub-paths in the opposite order, each travelled backwards, so a draw-on starts at the other end; style, attributes and origin are kept",
    args: &[],
    check: None,
};

impl Node for Reverse {
    fn verb(&self) -> &'static Verb {
        &REVERSE
    }

    fn evaluate(&self, _: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        each_shape(inputs.previous, cx, |shape, _| {
            Ok(shape.with_path(shape.path().reverse()))
        })
    }
}

/// Displaces the points of every shape sideways by a sine.
#[derive(Debug)]
pub(crate) struct Wave;

pub(crate) static WAVE: Verb = Verb {
    name: "wave",
    role: Role::Step,
    summary: "displaces the points of every shape sideways by a sine of amplitude design pixels and the given wavelength travelling towards angle degrees (clockwise on the screen, 0 towards +x, so down the screen is the sideways), phase in cycles moving it; the wave is anchored in the design frame, so one wave runs across all the letters of a word, and the edges are cut so the result is smooth, corners kept",
    args: &[
        per_shape("amplitude", Kind::Number, Some(Literal::Number(8.0)), Domain::Finite),
        per_shape(
            "wavelength",
            Kind::Number,
            Some(Literal::Number(120.0)),
            Domain::PositiveUpTo(MAX_COORDINATE),
        ),
        per_shape("angle", Kind::Number, Some(Literal::Number(0.0)), Domain::Finite),
        per_shape("phase", Kind::Number, Some(Literal::Number(0.0)), Domain::Finite),
    ],
    check: None,
};

impl Node for Wave {
    fn verb(&self) -> &'static Verb {
        &WAVE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        each_shape(inputs.previous, cx, |shape, item| {
            let (amplitude, wavelength) = (
                args.number(cx, "amplitude", item)?,
                args.number(cx, "wavelength", item)?,
            );
            let (angle, phase) = (args.number(cx, "angle", item)?, args.number(cx, "phase", item)?);
            Ok(shape.with_path(
                shape
                    .path()
                    .wave_charged(amplitude, wavelength, angle, phase, &mut charge)?,
            ))
        })
    }
}

/// Displaces the points of every shape by smooth seeded noise.
#[derive(Debug)]
pub(crate) struct Wobble;

pub(crate) static WOBBLE: Verb = Verb {
    name: "wobble",
    role: Role::Step,
    summary: "displaces the points of every shape by smooth seeded noise of amplitude design pixels (each of x and y at most that) and the given wavelength, phase in cycles moving it and seed (a whole number) choosing it; the noise is anchored in the design frame, so one field runs across all the letters of a word, and the edges are cut so the result is smooth, corners kept",
    args: &[
        per_shape("amplitude", Kind::Number, Some(Literal::Number(4.0)), Domain::Finite),
        per_shape(
            "wavelength",
            Kind::Number,
            Some(Literal::Number(80.0)),
            Domain::PositiveUpTo(MAX_COORDINATE),
        ),
        per_shape("phase", Kind::Number, Some(Literal::Number(0.0)), Domain::Finite),
        per_shape("seed", Kind::Number, Some(Literal::Number(0.0)), Domain::Finite),
    ],
    check: None,
};

impl Node for Wobble {
    fn verb(&self) -> &'static Verb {
        &WOBBLE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        each_shape(inputs.previous, cx, |shape, item| {
            let (amplitude, wavelength) = (
                args.number(cx, "amplitude", item)?,
                args.number(cx, "wavelength", item)?,
            );
            let (phase, seed) = (args.number(cx, "phase", item)?, args.number(cx, "seed", item)?);
            Ok(shape.with_path(
                shape
                    .path()
                    .wobble_charged(amplitude, wavelength, phase, seed, &mut charge)?,
            ))
        })
    }
}
