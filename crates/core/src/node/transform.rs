//! The geometry steps: `translate`, `rotate`, `scale`, `along`, `slice`. Each applies the `Path` verb of the same name to every
//! shape.

use super::{Args, Context, Inputs, Node, each_shape};
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
            let mut moved = shape.clone().with_path(shape.path().translate(dx, dy)?);
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
            let mut turned = shape.clone().with_path(shape.path().rotate(angle, about)?);
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
            let mut scaled = shape.clone().with_path(shape.path().scale(by, about)?);
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
                from_start = Some(Guide::new(path, None, &mut charge)?);
            }
            let own;
            let guide = match (near, &from_start) {
                (None, Some(guide)) => guide,
                (near, _) => {
                    own = Guide::new(path, near, &mut charge)?;
                    &own
                }
            };
            Ok(shape.clone().with_path(shape.path().bent(guide, start, &mut charge)?))
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
            Ok(shape.clone().with_path(shape.path().slice(start, end, near)?))
        })
    }
}
