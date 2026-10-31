//! `with_values`: Params fixed to values for the steps before it.

use super::{Args, Context, Inputs, Node};
use crate::{Arg, Domain, Error, Kind, Reads, Role, Shape, Verb};

/// Passes the shapes through; the evaluator gives the steps before it these values.
#[derive(Debug)]
pub(crate) struct WithValues;

pub(crate) static WITH_VALUES: Verb = Verb {
    name: "with_values",
    role: Role::Step,
    summary: "fixes Params to these values for the steps before it and the drawings they take; the Params leave parameters()",
    args: &[Arg {
        name: "values",
        kind: Kind::Values,
        default: None,
        reads: Reads::Literal,
        domain: Domain::Any,
    }],
    check: None,
};

impl Node for WithValues {
    fn verb(&self) -> &'static Verb {
        &WITH_VALUES
    }

    fn evaluate(&self, _: &Args, inputs: Inputs<'_>, _: &Context<'_>) -> Result<Vec<Shape>, Error> {
        Ok(inputs.previous.to_vec())
    }
}
