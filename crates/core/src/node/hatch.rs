//! The `hatch` step: every shape replaced by evenly spaced lines inside the region it fills.

use super::{Args, Context, Inputs, Node, each_shape};
use crate::field::Shapes;
use crate::geometry::MAX_COORDINATE;
use crate::{Arg, Domain, Error, Hatch, Kind, Literal, Path, Reads, Role, Shape, Verb};

const SPACING: Arg = Arg {
    name: "spacing",
    kind: Kind::Factor,
    default: None,
    reads: Reads::Literal,
    domain: Domain::PositiveUpTo(MAX_COORDINATE),
};

const TONE: Arg = Arg {
    name: "tone",
    kind: Kind::Field,
    default: Some(Literal::Unset),
    reads: Reads::Literal,
    domain: Domain::Any,
};

const FLOW: Arg = Arg {
    name: "flow",
    kind: Kind::Flow,
    default: Some(Literal::Unset),
    reads: Reads::Literal,
    domain: Domain::Any,
};

const AVOID: Arg = Arg {
    name: "avoid",
    kind: Kind::Drawing,
    default: Some(Literal::Unset),
    reads: Reads::Literal,
    domain: Domain::Any,
};

const BELOW: Arg = Arg {
    name: "below",
    kind: Kind::Number,
    default: Some(Literal::Unset),
    reads: Reads::Once,
    domain: Domain::Finite,
};

/// Every shape hatched.
#[derive(Debug)]
pub(crate) struct HatchStep;

pub(crate) static HATCH: Verb = Verb {
    name: "hatch",
    role: Role::Step,
    summary: "replaces every shape by evenly spaced lines along `flow` (default: 45 degrees) inside the region it fills (as Path.hatch): `spacing` design pixels apart, or a pair (spacing where `tone` is 0, where it is 1) read from the field `tone`; no two lines come within half a spacing; with `below`, only where the tone is at most that level (hatch passes at other angles and levels over each other crosshatch); lines never enter the drawing `avoid` and close where they come round to their own start; the result is open lines, so set the stroke after it",
    args: &[SPACING, TONE, FLOW, AVOID, BELOW],
    check: Some(pair_needs_tone),
};

/// A spacing that differs between tone 0 and tone 1 needs a tone to read.
fn pair_needs_tone(args: &Args) -> Result<(), Error> {
    if args.given("below")? && !args.given("tone")? {
        return Err(Error::parameter("below", "needs a `tone` to read"));
    }
    match args.literal_pair("spacing")? {
        Some(spacing) => Hatch::checked(spacing, args.field("tone")?.cloned()).map(drop),
        None => Ok(()),
    }
}

impl Node for HatchStep {
    fn verb(&self) -> &'static Verb {
        &HATCH
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let (zero, one) = args
            .literal_pair("spacing")?
            .ok_or_else(|| Error::parameter("spacing", "is missing"))?;
        // the drawings of the tone and of the flow come first, then `avoid`: the order `Args::taken` lists them
        let placed = |error: Error| error.at_argument(&cx.place, None);
        let mut shapes = Shapes::taken(inputs.taken);
        let tone = args
            .field("tone")?
            .map(|tone| tone.bind(cx.env, &mut shapes))
            .transpose()
            .map_err(placed)?;
        let mut hatch = Hatch::checked((zero, one), tone)?;
        if let Some(flow) = args.flow("flow")? {
            hatch = hatch.flow(flow.bind(cx.env, &mut shapes).map_err(placed)?);
        }
        if let Some(level) = args
            .once_number_opt(cx, "below")
            .map_err(|error| error.at_argument(&cx.place, None))?
        {
            hatch = hatch.below(level);
        }
        let avoid: Vec<Path> = if args.given("avoid")? {
            shapes.next_drawn()?.iter().map(|shape| shape.path().clone()).collect()
        } else {
            Vec::new()
        };
        let avoid: Vec<&Path> = avoid.iter().collect();
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        each_shape(inputs.previous, cx, |shape, _| {
            Ok(shape.with_path(shape.path().hatch_charged(&hatch, &avoid, &mut charge)?))
        })
    }
}
