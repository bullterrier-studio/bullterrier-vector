//! The geometry steps: `translate`, `rotate`, `scale`. Each applies the `Path` verb of the same name to every shape.

use super::{Args, Context, Inputs, Node, each_shape};
use crate::{Arg, Domain, Error, Kind, Literal, Reads, Role, Shape, Verb};

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

/// Moves every shape.
#[derive(Debug)]
pub(crate) struct Translate;

pub(crate) static TRANSLATE: Verb = Verb {
    name: "translate",
    role: Role::Step,
    summary: "moves every shape by dx to the right and dy down, in design pixels",
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
            Ok(shape.clone().with_path(shape.path().translate(dx, dy)?))
        })
    }
}

/// Turns every shape.
#[derive(Debug)]
pub(crate) struct Rotate;

pub(crate) static ROTATE: Verb = Verb {
    name: "rotate",
    role: Role::Step,
    summary: "turns every shape clockwise on the screen by angle degrees, any size, about a point (default: the middle of its bounds)",
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
            let about = args.point(cx, "about", item)?;
            Ok(shape.clone().with_path(shape.path().rotate(angle, about)?))
        })
    }
}

/// Scales every shape.
#[derive(Debug)]
pub(crate) struct Scale;

pub(crate) static SCALE: Verb = Verb {
    name: "scale",
    role: Role::Step,
    summary: "scales every shape by a number, or by an (x, y) pair, about a point (default: the middle of its bounds); a negative factor mirrors",
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
            let about = args.point(cx, "about", item)?;
            Ok(shape.clone().with_path(shape.path().scale(by, about)?))
        })
    }
}
