//! Drawing a `Drawing`: its steps evaluated, in order, to a list of shapes.

use super::{Drawing, Step};
use crate::node::{Context, Inputs};
use crate::{Error, Shape, Values};
use std::sync::Arc;

impl Drawing {
    /// The shapes the drawing evaluates to, in paint order (the bottom first), in design pixels, with the Params at
    /// the `values` given and at their defaults where none is given. The default paint is resolved: an unpainted
    /// closed outline has a black fill, an unpainted open line a black stroke.
    ///
    /// # Errors
    /// The first error of the chain that built the drawing; [`Error::Parameter`] naming the Param if `values` gives
    /// one the drawing does not read, or one a list of the wrong length or outside its range; naming the step and the
    /// argument (`step 3 (rotate).angle`) if a number is outside what the argument accepts, with the index of the
    /// shape it was read for.
    pub fn shapes(&self, values: &Values) -> Result<Vec<Shape>, Error> {
        let step = self.last.as_ref().map_err(Clone::clone)?;
        if let Some(name) = values
            .names()
            .find(|name| step.reads.iter().all(|param| param.name() != *name))
        {
            let reads: Vec<&str> = step.reads.iter().map(crate::Param::name).collect();
            return Err(Error::parameter(
                name,
                if reads.is_empty() {
                    String::from("the drawing reads no Params, so there is no value to give it")
                } else {
                    format!("the drawing reads no Param of this name, only {}", reads.join(", "))
                },
            ));
        }
        Ok(evaluate(step, values)?.into_iter().map(Shape::painted).collect())
    }

    /// The shapes before the default paint is resolved.
    fn evaluate(&self, values: &Values) -> Result<Vec<Shape>, Error> {
        evaluate(self.last.as_ref().map_err(Clone::clone)?, values)
    }
}

/// The shapes after `last` and every step before it, in order, with each shape's `index` its place in the list.
fn evaluate(last: &Arc<Step>, values: &Values) -> Result<Vec<Shape>, Error> {
    let mut chain = Vec::new();
    let mut cursor = Some(last);
    while let Some(step) = cursor {
        chain.push(step);
        cursor = step.previous.as_ref();
    }
    let mut shapes: Vec<Shape> = Vec::new();
    for step in chain.into_iter().rev() {
        let place = step.place();
        let taken = step
            .args
            .taken()
            .enumerate()
            .map(|(at, item)| {
                item.evaluate(values)
                    .map_err(|error| error.inside(&format!("{place} / item {}", at + 1)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let cx = Context { place, env: values };
        let inputs = Inputs {
            previous: &shapes,
            taken: &taken,
        };
        shapes = step.node.evaluate(&step.args, inputs, &cx)?;
        shapes = shapes
            .into_iter()
            .enumerate()
            .map(|(index, shape)| shape.at(index))
            .collect();
    }
    Ok(shapes)
}
