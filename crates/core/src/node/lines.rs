//! The line-family steps: `rows` (ridgelines), each the `Path` method of the same name applied to each shape's path.

use super::{Args, Context, Inputs, Node, each_shape};
use crate::field::Shapes;
use crate::geometry::MAX_COORDINATE;
use crate::{Arg, Domain, Error, Kind, Reads, Role, Shape, Verb};

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
