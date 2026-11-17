//! Fields: a number at every point ([`Field`]) and a direction without sign at every point (Flow), the inputs of
//! hatched lines. Each is a plain value built once from literal numbers and paths, read by `at`, and saved in a
//! document by its parts.

mod contours;
mod distance;
mod flow;
mod hatch;
mod map;
mod outline;
mod seeds;

pub use contours::Tidy;
pub(crate) use contours::{Grid, MAX_NODES, MAX_SMOOTH_WORK, blur, trace as trace_level};
pub use distance::Field;
pub(crate) use distance::{FieldKind, MAX_READ_WORK};
pub use flow::Flow;
pub(crate) use flow::FlowKind;
pub use hatch::Hatch;
pub(crate) use map::MapData;
pub(crate) use outline::Outline;

use crate::limits::MAX_SEGMENTS;
use crate::path::Path;
use crate::{Drawing, Error, Expr, Shape, Values};
use std::sync::Arc;

/// A count of the edges a field or a hatch makes that fails naming `parameter` past `MAX_SEGMENTS`.
pub(crate) fn segment_budget(parameter: &'static str) -> impl FnMut(usize) -> Result<(), Error> {
    let mut used = 0_usize;
    move |count| {
        used = used.saturating_add(count);
        if used > MAX_SEGMENTS {
            Err(Error::parameter(
                parameter,
                format!("would make more than {MAX_SEGMENTS} points"),
            ))
        } else {
            Ok(())
        }
    }
}

/// The length of the vector `(x, y)`: `sqrt` is exact on every target, where `hypot` is the platform's.
pub(crate) fn length(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// Where a field or flow gets the shapes of the drawings it reads.
pub(crate) enum Shapes<'a> {
    /// The drawings drawn with their Params at the defaults: what a field sees outside a step.
    Defaults,
    /// The shapes a step was handed for them, in the order the field lists its drawings.
    Taken(std::slice::Iter<'a, Arc<Vec<Shape>>>),
}

impl<'a> Shapes<'a> {
    /// The shapes a step was handed, to read in the order the arguments take drawings.
    pub(crate) fn taken(shapes: &'a [Arc<Vec<Shape>>]) -> Self {
        Self::Taken(shapes.iter())
    }

    /// The next drawing's shapes as they were handed, for a step's own drawing (`avoid`).
    pub(crate) fn next_drawn(&mut self) -> Result<&'a Arc<Vec<Shape>>, Error> {
        match self {
            Self::Taken(taken) => taken
                .next()
                .ok_or_else(|| Error::parameter("drawing", "the step was not given the drawing")),
            Self::Defaults => Err(Error::parameter("drawing", "there is no step to give the drawing")),
        }
    }
}

impl Shapes<'_> {
    /// The outlines of the next drawing's shapes as one path.
    pub(crate) fn path_of(&mut self, drawing: &Drawing) -> Result<Path, Error> {
        let drawn;
        let shapes: &[Shape] = match self {
            Self::Defaults => {
                drawn = drawing.shapes(&Values::new())?;
                &drawn
            }
            Self::Taken(taken) => taken
                .next()
                .ok_or_else(|| Error::parameter("drawing", "the step was not given the drawing"))?,
        };
        Ok(Path::joined(shapes.iter().map(Shape::path)))
    }
}

/// `expr` as a number a field or flow reads once: no per-item number, within the expression limits.
pub(crate) fn once(name: &str, expr: &Expr) -> Result<Expr, Error> {
    expr.check_limits().map_err(|error| relabel(name, &error))?;
    if expr.is_per_item() {
        return Err(Error::parameter(
            name,
            format!("is read once, not per shape, so it takes no per-item number, got {expr}"),
        ));
    }
    Ok(expr.clone())
}

/// The finite value of `expr` with the Params at `env`, an error naming `name` otherwise.
pub(crate) fn read(name: &str, expr: &Expr, env: &Values) -> Result<f64, Error> {
    let value = expr.value(env).map_err(|error| relabel(name, &error))?;
    crate::geometry::finite(name, value)
}

fn relabel(name: &str, error: &Error) -> Error {
    match error {
        Error::Parameter { parameter, message } if parameter == "expression" => Error::parameter(name, message.clone()),
        Error::Parameter { parameter, message } => Error::parameter(name, format!("{parameter}: {message}")),
        other => other.clone(),
    }
}
