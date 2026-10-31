//! The selection steps: `only` and `without` keep or drop shapes by their attributes.

use super::{Args, Context, Inputs, Node};
use crate::shape::{Holds, SELECTABLE, Single};
use crate::{Arg, Domain, Error, Kind, Reads, Role, Select, Shape, Verb};

/// The selection both steps take.
const ATTRIBUTES: Arg = Arg {
    name: "attributes",
    kind: Kind::Attributes,
    default: None,
    reads: Reads::Literal,
    domain: Domain::Any,
};

/// Keeps the shapes that match.
#[derive(Debug)]
pub(crate) struct Only;

pub(crate) static ONLY: Verb = Verb {
    name: "only",
    role: Role::Step,
    summary: "keeps the shapes whose attributes equal these (a list means any of); the list is renumbered",
    args: &[ATTRIBUTES],
    check: Some(attributes_make_sense),
};

/// Drops the shapes that match.
#[derive(Debug)]
pub(crate) struct Without;

pub(crate) static WITHOUT: Verb = Verb {
    name: "without",
    role: Role::Step,
    summary: "drops the shapes whose attributes equal these (a list means any of); the list is renumbered",
    args: &[ATTRIBUTES],
    check: Some(attributes_make_sense),
};

/// Every name is an attribute, every value is one that attribute can hold, and something is asked.
fn attributes_make_sense(args: &Args) -> Result<(), Error> {
    let select = args.select("attributes")?;
    if select.is_empty() {
        return Err(Error::parameter(
            "attributes",
            "must name at least one attribute to select by",
        ));
    }
    for (name, wanted) in select.asked() {
        let Some((_, holds)) = SELECTABLE.iter().find(|(known, _)| *known == name) else {
            let known: Vec<&str> = SELECTABLE.iter().map(|(known, _)| *known).collect();
            return Err(Error::parameter(
                name,
                format!("is not an attribute, the attributes are {}", known.join(", ")),
            ));
        };
        for single in wanted {
            fits(name, *holds, single)?;
        }
    }
    Ok(())
}

/// An error unless `single` is a value the attribute `name`, which holds `holds`, can have.
fn fits(name: &str, holds: Holds, single: &Single) -> Result<(), Error> {
    let refuse = |rule: &str, got: String| Err(Error::parameter(name, format!("must be {rule}, got {got}")));
    match (holds, single) {
        (Holds::Whole, Single::Number(n)) if n.fract() == 0.0 && *n >= 0.0 && *n <= f64::from(u32::MAX) => Ok(()),
        (Holds::Whole, Single::Number(n)) => refuse("a whole number from 0", n.to_string()),
        (Holds::Whole, other) => refuse("a whole number from 0", other.to_string()),
        (Holds::Real, Single::Number(n)) if n.is_finite() => Ok(()),
        (Holds::Real, Single::Number(n)) => refuse("a finite number", n.to_string()),
        (Holds::Real, other) => refuse("a number", other.to_string()),
        (Holds::Char, Single::Char(_)) | (Holds::Point, Single::Point(_)) => Ok(()),
        (Holds::Char, other) => refuse("one character", other.to_string()),
        (Holds::Point, other) => refuse("a point", other.to_string()),
    }
}

/// The shapes of `previous` that match `select` if `matching`, else those that do not.
fn choose(select: &Select, previous: &[Shape], cx: &Context<'_>, matching: bool) -> Result<Vec<Shape>, Error> {
    for (name, _) in select.asked() {
        if !previous.is_empty() && previous.iter().all(|shape| shape.attribute(name).is_none()) {
            return Err(
                Error::parameter(name, "no shape carries it here, so there is nothing to select by")
                    .at_argument(&cx.place, None),
            );
        }
    }
    Ok(previous
        .iter()
        .filter(|shape| {
            let all = select
                .asked()
                .all(|(name, wanted)| shape.attribute(name).is_some_and(|value| wanted.contains(&value)));
            all == matching
        })
        .cloned()
        .collect())
}

impl Node for Only {
    fn verb(&self) -> &'static Verb {
        &ONLY
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        choose(args.select("attributes")?, inputs.previous, cx, true)
    }
}

impl Node for Without {
    fn verb(&self) -> &'static Verb {
        &WITHOUT
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        choose(args.select("attributes")?, inputs.previous, cx, false)
    }
}
