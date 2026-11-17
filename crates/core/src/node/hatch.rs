//! The `hatch` step: every shape replaced by evenly spaced lines inside the region it fills.

use super::{Args, Context, Inputs, Node, each_shape};
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

/// Every shape hatched.
#[derive(Debug)]
pub(crate) struct HatchStep;

pub(crate) static HATCH: Verb = Verb {
    name: "hatch",
    role: Role::Step,
    summary: "replaces every shape by evenly spaced lines along `flow` (default: 45 degrees) inside the region it fills (as Path.hatch): `spacing` design pixels apart, or a pair (spacing where `tone` is 0, where it is 1) read from the field `tone`; no two lines come within half a spacing; lines never enter the drawing `avoid` and close where they come round to their own start; the result is open lines, so set the stroke after it",
    args: &[SPACING, TONE, FLOW, AVOID],
    check: Some(pair_needs_tone),
};

/// A spacing that differs between tone 0 and tone 1 needs a tone to read.
fn pair_needs_tone(args: &Args) -> Result<(), Error> {
    if let Some((zero, one)) = args.literal_pair("spacing")?
        && zero.to_bits() != one.to_bits()
        && !args.given("tone")?
    {
        return Err(Error::parameter("tone", "is needed when `spacing` is a pair"));
    }
    Ok(())
}

impl Node for HatchStep {
    fn verb(&self) -> &'static Verb {
        &HATCH
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let (zero, one) = args
            .literal_pair("spacing")?
            .ok_or_else(|| Error::parameter("spacing", "is missing"))?;
        let mut hatch = match args.field("tone")? {
            Some(tone) => Hatch::between(zero, one, tone.clone()),
            None => Hatch::new(zero),
        };
        if let Some(flow) = args.flow("flow")? {
            hatch = hatch.flow(flow.clone());
        }
        let avoid: Vec<Path> = if args.given("avoid")? {
            inputs.taken[0].iter().map(|shape| shape.path().clone()).collect()
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
