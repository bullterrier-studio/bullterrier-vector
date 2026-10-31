//! `stack`: drawings one above the other.

use super::{Args, Context, Inputs, Node};
use crate::{Arg, Domain, Error, Kind, Literal, Reads, Role, Shape, Verb};

/// The largest side of a design frame, in design pixels.
// constant: formats.md "Limits and security" bounds a stack's `size` at 100,000 design pixels
pub(crate) const MAX_FRAME: f64 = 100_000.0;

/// The items, bottom first.
#[derive(Debug)]
pub(crate) struct Stack;

pub(crate) static STACK: Verb = Verb {
    name: "stack",
    role: Role::Source,
    summary: "the items one above the other, the first at the bottom; shapes without an order get one from 0 to 1",
    args: &[
        Arg {
            name: "items",
            kind: Kind::Drawings,
            default: None,
            reads: Reads::Literal,
            domain: Domain::Any,
        },
        Arg {
            name: "size",
            kind: Kind::Pair,
            default: Some(Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::PositiveUpTo(MAX_FRAME),
        },
    ],
    check: Some(items_agree),
};

/// Items of a stack with no `size` of its own that have a size must have the same one.
fn items_agree(args: &Args) -> Result<(), Error> {
    if args.literal_pair("size")?.is_some() {
        return Ok(());
    }
    let mut sizes = args.drawings("items")?.iter().filter_map(crate::Drawing::size);
    if let Some(first) = sizes.next()
        && let Some(other) = sizes.find(|size| *size != first)
    {
        return Err(Error::parameter(
            "items",
            format!(
                "have different sizes, {} x {} and {} x {}; give the stack a size",
                first.0, first.1, other.0, other.1
            ),
        ));
    }
    Ok(())
}

impl Node for Stack {
    fn verb(&self) -> &'static Verb {
        &STACK
    }

    #[expect(clippy::cast_precision_loss, reason = "a list is far shorter than 2^53 shapes")]
    fn evaluate(&self, _: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        // the list is charged before it is built, so that a refusal comes before the memory
        cx.work
            .made_shapes(inputs.taken.iter().map(Vec::len).sum(), &cx.place)?;
        let mut shapes: Vec<Shape> = inputs.taken.iter().flatten().cloned().collect();
        let unordered = shapes
            .iter()
            .filter(|shape| shape.attributes().order().is_none())
            .count();
        let mut rank = 0_usize;
        for shape in &mut shapes {
            if shape.attributes().order().is_none() {
                let order = if unordered < 2 {
                    0.0
                } else {
                    rank as f64 / (unordered - 1) as f64
                };
                shape.attributes_mut().order = Some(order);
                rank += 1;
            }
        }
        Ok(shapes)
    }
}
