//! The sources that make one shape: a path, or a shape as `shapes()` returned it.

use super::{Args, Context, Inputs, Node};
use crate::{Arg, Domain, Error, Kind, Reads, Role, Shape, Verb};

/// A shape of a path, not yet painted.
#[derive(Debug)]
pub(crate) struct PathSource;

pub(crate) static PATH: Verb = Verb {
    name: "path",
    role: Role::Source,
    summary: "one shape with this outline, painted by default",
    args: &[Arg {
        name: "path",
        kind: Kind::Path,
        default: None,
        reads: Reads::Literal,
        domain: Domain::Any,
    }],
    check: None,
};

impl Node for PathSource {
    fn verb(&self) -> &'static Verb {
        &PATH
    }

    fn evaluate(&self, args: &Args, _: Inputs<'_>, _: &Context<'_>) -> Result<Vec<Shape>, Error> {
        Ok(vec![Shape::from_path(args.path("path")?.clone(), 0)])
    }
}

/// A shape put back into a drawing with its style and attributes.
#[derive(Debug)]
pub(crate) struct ShapeSource;

pub(crate) static SHAPE: Verb = Verb {
    name: "shape",
    role: Role::Source,
    summary: "one shape as it was drawn, with its style and attributes",
    args: &[Arg {
        name: "shape",
        kind: Kind::Shape,
        default: None,
        reads: Reads::Literal,
        domain: Domain::Any,
    }],
    check: None,
};

impl Node for ShapeSource {
    fn verb(&self) -> &'static Verb {
        &SHAPE
    }

    fn evaluate(&self, args: &Args, _: Inputs<'_>, _: &Context<'_>) -> Result<Vec<Shape>, Error> {
        Ok(vec![args.shape("shape")?.clone()])
    }
}
