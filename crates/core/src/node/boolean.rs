//! The booleans of the shapes of a drawing: `union`, `subtract`, `intersect` and `exclude` combine each shape with the
//! shapes of another drawing, `resolve` removes the overlaps of each shape. Every verb is the `Path` method of the
//! same name applied to each shape's path; the shape keeps its style and attributes.

use super::{Args, Context, Inputs, Node, each_shape};
use crate::geometry::MAX_COORDINATE;
use crate::path::{Combine, FLATNESS};
use crate::{Arg, Domain, Error, Kind, Literal, Path, Reads, Role, Shape, Verb};
use std::borrow::Cow;

/// The drawing each shape is combined with.
const OTHER: Arg = Arg {
    name: "other",
    kind: Kind::Drawing,
    default: None,
    reads: Reads::Literal,
    domain: Domain::Any,
};

/// How far a curve may stray from the straight edges it becomes, in design pixels.
pub(super) const TOLERANCE: Arg = Arg {
    name: "tolerance",
    kind: Kind::Number,
    default: Some(Literal::Number(FLATNESS)),
    reads: Reads::Literal,
    domain: Domain::PositiveUpTo(MAX_COORDINATE),
};

/// Every shape with the region of another drawing added.
#[derive(Debug)]
pub(crate) struct Union;

pub(crate) static UNION: Verb = Verb {
    name: "union",
    role: Role::Step,
    summary: "fills every shape with the region it and the drawing `other` fill together (as Path.union); `other` is all its shapes at once; curves become straight edges within `tolerance` design pixels",
    args: &[OTHER, TOLERANCE],
    check: None,
};

/// Every shape with the region of another drawing taken away.
#[derive(Debug)]
pub(crate) struct Subtract;

pub(crate) static SUBTRACT: Verb = Verb {
    name: "subtract",
    role: Role::Step,
    summary: "fills every shape with the region it fills and the drawing `other` does not (as Path.subtract); `other` is all its shapes at once; curves become straight edges within `tolerance` design pixels",
    args: &[OTHER, TOLERANCE],
    check: None,
};

/// Every shape cut to the region of another drawing.
#[derive(Debug)]
pub(crate) struct Intersect;

pub(crate) static INTERSECT: Verb = Verb {
    name: "intersect",
    role: Role::Step,
    summary: "fills every shape with the region it and the drawing `other` both fill (as Path.intersect), an empty path where they do not meet; `other` is all its shapes at once; curves become straight edges within `tolerance` design pixels",
    args: &[OTHER, TOLERANCE],
    check: None,
};

/// Every shape with the region of another drawing toggled.
#[derive(Debug)]
pub(crate) struct Exclude;

pub(crate) static EXCLUDE: Verb = Verb {
    name: "exclude",
    role: Role::Step,
    summary: "fills every shape with the region only one of it and the drawing `other` fills (as Path.exclude); `other` is all its shapes at once; curves become straight edges within `tolerance` design pixels",
    args: &[OTHER, TOLERANCE],
    check: None,
};

/// Every shape with its overlaps removed.
#[derive(Debug)]
pub(crate) struct Resolve;

pub(crate) static RESOLVE: Verb = Verb {
    name: "resolve",
    role: Role::Step,
    summary: "removes the overlaps of every shape, so it is the closed polygons it fills (as Path.resolve); curves become straight edges within `tolerance` design pixels",
    args: &[TOLERANCE],
    check: None,
};

/// The region the shapes of `other` fill together: one shape's own path, else each shape resolved (a path that runs the
/// other way would take the overlap out under the nonzero rule) and joined.
fn region<'a>(
    other: &'a [Shape],
    tolerance: f64,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Cow<'a, Path>, Error> {
    Ok(match other {
        [] => Cow::Owned(Path::new()),
        [only] => Cow::Borrowed(only.path()),
        many => {
            let resolved = many
                .iter()
                .map(|shape| shape.path().resolved(tolerance, charge))
                .collect::<Result<Vec<_>, _>>()?;
            Cow::Owned(Path::joined(&resolved))
        }
    })
}

/// Each shape of `inputs.previous` combined as `how` with the region of the drawing the call takes.
fn combine(how: Combine, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
    let place = cx.place.to_string();
    let mut charge = |count| cx.work.made_segments(count, &place);
    let tolerance = args
        .once_number(cx, "tolerance")
        .map_err(|error| error.at_argument(&cx.place, None))?;
    // the shapes of `other` are united once, not for each shape
    let other = region(&inputs.taken[0], tolerance, &mut charge).map_err(|error| error.at_argument(&cx.place, None))?;
    each_shape(inputs.previous, cx, |shape, _| {
        Ok(shape.with_path(shape.path().combined(&other, how, tolerance, &mut charge)?))
    })
}

impl Node for Union {
    fn verb(&self) -> &'static Verb {
        &UNION
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        combine(Combine::Union, args, inputs, cx)
    }
}

impl Node for Subtract {
    fn verb(&self) -> &'static Verb {
        &SUBTRACT
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        combine(Combine::Subtract, args, inputs, cx)
    }
}

impl Node for Intersect {
    fn verb(&self) -> &'static Verb {
        &INTERSECT
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        combine(Combine::Intersect, args, inputs, cx)
    }
}

impl Node for Exclude {
    fn verb(&self) -> &'static Verb {
        &EXCLUDE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        combine(Combine::Exclude, args, inputs, cx)
    }
}

impl Node for Resolve {
    fn verb(&self) -> &'static Verb {
        &RESOLVE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        let tolerance = args
            .once_number(cx, "tolerance")
            .map_err(|error| error.at_argument(&cx.place, None))?;
        each_shape(inputs.previous, cx, |shape, _| {
            Ok(shape.with_path(shape.path().resolved(tolerance, &mut charge)?))
        })
    }
}
