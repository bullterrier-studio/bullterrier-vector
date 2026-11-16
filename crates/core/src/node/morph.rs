//! The `morph` step: every shape carried part of the way to the shape at the same index of another drawing.

use super::{Args, Context, Inputs, Node, each_shape};
use crate::{Arg, Domain, Error, Kind, Path, Reads, Role, Shape, Verb};
use std::borrow::Cow;

/// The drawing each shape is carried towards.
const TARGET: Arg = Arg {
    name: "target",
    kind: Kind::Drawing,
    default: None,
    reads: Reads::Literal,
    domain: Domain::Any,
};

/// How far each shape goes, 0 the shape itself, 1 its partner; beyond that the lerp goes on.
const FRACTION: Arg = Arg {
    name: "fraction",
    kind: Kind::Number,
    default: None,
    reads: Reads::PerItem,
    domain: Domain::Finite,
};

/// Every shape morphed towards the shape of another drawing at its index.
#[derive(Debug)]
pub(crate) struct Morph;

pub(crate) static MORPH: Verb = Verb {
    name: "morph",
    role: Role::Step,
    summary: "morphs every shape `fraction` of the way to the shape at the same index of the drawing `target` (as Path.morph); `fraction` 0 is the shape, 1 the target's, any finite number goes on along the same line; a shape with no partner shrinks to its centre, a target shape with no partner grows from its centre with its own style",
    args: &[TARGET, FRACTION],
    check: None,
};

impl Node for Morph {
    fn verb(&self) -> &'static Verb {
        &MORPH
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let target = &inputs.taken[0];
        let surplus = target.len().saturating_sub(inputs.previous.len());
        cx.work.made_shapes(surplus, &cx.place)?;
        // a target shape with no partner grows from nothing: it is its own shape with an empty path to start from, so
        // that the step reads its numbers over one list of every shape the result holds
        let shapes: Cow<'_, [Shape]> = if surplus == 0 {
            Cow::Borrowed(inputs.previous)
        } else {
            let grown = target[inputs.previous.len()..]
                .iter()
                .map(|shape| shape.with_path(Path::new()));
            Cow::Owned(inputs.previous.iter().cloned().chain(grown).collect())
        };
        let nothing = Path::new();
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        each_shape(&shapes, cx, |shape, item| {
            let fraction = args.number(cx, "fraction", item)?;
            let partner = target.get(item.index).map_or(&nothing, Shape::path);
            Ok(shape.with_path(shape.path().morph_charged(partner, fraction, &mut charge)?))
        })
    }
}
