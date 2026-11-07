//! The steps that reshape the list: `split` makes a shape of every part of a shape, `merge` makes one shape of the
//! shapes that agree on an attribute.

use super::{Args, Context, Inputs, Node};
use crate::key::Key;
use crate::shape::holds;
use crate::{Arg, Domain, Error, Kind, Literal, Path, Point, Reads, Role, Shape, Verb};
use std::collections::BTreeMap;

/// Every part of every shape, a shape of its own.
#[derive(Debug)]
pub(crate) struct Split;

pub(crate) static SPLIT: Verb = Verb {
    name: "split",
    role: Role::Step,
    summary: "makes a shape of every part of every shape (a closed outline with its holes, or an open line), each keeping the attributes and style of its shape, with part (from 0, within the shape) and the middle of its own bounds as origin; a shape with no parts stays",
    args: &[],
    check: None,
};

/// The shapes that agree on an attribute, one shape each.
#[derive(Debug)]
pub(crate) struct Merge;

pub(crate) static MERGE: Verb = Verb {
    name: "merge",
    role: Role::Step,
    summary: "makes one shape of the shapes whose attribute `by` is equal (all of them without `by`), holding their sub-paths in list order, with the attributes and style of the first; the shape sits where its first member was; sub-paths are joined, not united",
    args: &[Arg {
        name: "by",
        kind: Kind::Text,
        default: Some(Literal::Unset),
        reads: Reads::Literal,
        domain: Domain::Any,
    }],
    check: Some(by_is_an_attribute),
};

/// `by`, if given, names an attribute.
fn by_is_an_attribute(args: &Args) -> Result<(), Error> {
    if let Some(by) = args.text_opt("by")? {
        holds(by).map_err(|reason| Error::parameter("by", format!("{by:?} {reason}")))?;
    }
    Ok(())
}

/// The middle of the bounds of a path that has a sub-path.
fn middle(part: &Path) -> Result<Point, Error> {
    let bounds = part.bounds()?;
    Ok(Point::new(
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    ))
}

impl Node for Split {
    fn verb(&self) -> &'static Verb {
        &SPLIT
    }

    fn evaluate(&self, _: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut shapes = Vec::new();
        for (index, shape) in inputs.previous.iter().enumerate() {
            let placed = |error: Error| error.at_argument(&cx.place, Some(index));
            // charged before the shapes are made: the geometry they copy, then the parts
            cx.work.made_segments(shape.path().weight(), &place)?;
            let parts = shape.path().split();
            cx.work.made_shapes(parts.len().max(1), &cx.place)?;
            if parts.is_empty() {
                let mut empty = shape.clone();
                empty.attributes_mut().part = Some(0);
                shapes.push(empty);
            }
            for (number, part) in parts.into_iter().enumerate() {
                let origin = middle(&part).map_err(placed)?;
                let mut made = shape.with_path(part);
                made.attributes_mut().part = u32::try_from(number).ok();
                made.attributes_mut().origin = Some(origin);
                shapes.push(made);
            }
        }
        Ok(shapes)
    }
}

impl Node for Merge {
    fn verb(&self) -> &'static Verb {
        &MERGE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let previous = inputs.previous;
        let by = args.text_opt("by")?;
        if let Some(by) = by
            && !previous.is_empty()
            && previous.iter().all(|shape| shape.attribute(by).is_none())
        {
            return Err(Error::parameter(
                "by",
                format!("{by:?}: no shape carries it here, so there is nothing to merge by"),
            )
            .at_argument(&cx.place, None));
        }
        // the members of each group, groups in the order their first member appears; a shape without the attribute
        // is a group of its own with the others that lack it
        let mut groups: Vec<Vec<&Shape>> = Vec::new();
        let mut of_value: BTreeMap<Vec<u64>, usize> = BTreeMap::new();
        for shape in previous {
            let mut key = Key::new();
            if let Some(by) = by {
                key.of(&shape.attribute(by));
            }
            let group = *of_value.entry(key.finish()).or_insert_with(|| {
                groups.push(Vec::new());
                groups.len() - 1
            });
            groups[group].push(shape);
        }
        Ok(groups
            .iter()
            .map(|members| members[0].with_path(Path::joined(members.iter().map(|shape| shape.path()))))
            .collect())
    }
}
