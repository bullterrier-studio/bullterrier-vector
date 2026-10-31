//! `Drawing`: a tree of steps that evaluates to a list of shapes.

mod eval;
mod select;
mod style;
mod transform;
mod values;

pub use select::{Select, Wanted};
pub use style::Stroke;
pub use transform::{Deg, Pivot, Rotate, ScaleBy};

use crate::node::{self, Args, Given, Node, Place};
use crate::{Error, Expr, Param, Path, Shape};
use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

/// An immutable tree of steps that evaluates to a list of [`Shape`]s: a source (a [`Path`], a [`Shape`] or a stack of
/// drawings), and the steps that build on it. A drawing holds no callbacks, so it draws again with new values. It is
/// cheap to clone and can be used again, in several places.
///
/// A step is a method that returns the next drawing and never fails: a drawing keeps the first error of its chain
/// ([`Drawing::error`]) and returns it when it is drawn or its Params are asked for.
#[derive(Clone)]
pub struct Drawing {
    last: Result<Arc<Step>, Failure>,
}

/// Why a chain holds no step: its first error, and, for a stack whose items disagree on a size, the items, so that
/// [`Drawing::with_size`] can still give the stack a frame.
#[derive(Debug, Clone)]
struct Failure {
    error: Error,
    items: Option<Vec<Drawing>>,
}

impl From<Error> for Failure {
    fn from(error: Error) -> Self {
        Self { error, items: None }
    }
}

/// The last call of a chain; the calls before it are reached through it.
struct Step {
    node: &'static dyn Node,
    args: Args,
    previous: Option<Arc<Step>>,
    /// The number of the step in the chain; 0 for the source.
    number: usize,
    size: Option<(f64, f64)>,
    /// How many drawings deep the drawing nests: 1 for one that takes none, else 1 more than the deepest it takes.
    depth: usize,
    /// The Params read, in the order they were first used, those a `with_values` fixed included.
    reads: Vec<Param>,
    /// The names of the Params fixed at this step: a `with_values` fixed them for the steps before it, or for the
    /// drawings the step takes, and no step since (nor another drawing it takes) has read them.
    fixed: BTreeSet<String>,
}

impl Step {
    fn place(&self) -> Place {
        Place {
            step: self.number,
            verb: self.node.verb().name,
        }
    }

    /// The Params read here and not fixed, in the order they were first used.
    fn free(&self) -> Vec<Param> {
        self.reads
            .iter()
            .filter(|param| !self.fixed.contains(param.name()))
            .cloned()
            .collect()
    }

    /// The call of `node` with the `given` arguments, on `previous` if it is a step.
    fn build(
        previous: Option<&Arc<Step>>,
        node: &'static dyn Node,
        given: Vec<(&'static str, Given)>,
    ) -> Result<Self, Error> {
        let verb = node.verb();
        let place = Place {
            step: previous.map_or(0, |step| step.number + 1),
            verb: verb.name,
        };
        let args = Args::new(verb, given)
            .fixing(&previous.map_or_else(Vec::new, |step| step.free()))
            .map_err(|error| error.at_argument(&place, None))?;
        for (at, item) in args.taken().enumerate() {
            if let Some(error) = item.error() {
                return Err(error.clone().inside(&format!("{place} / item {}", at + 1)));
            }
        }
        args.validate().map_err(|error| error.at_argument(&place, None))?;
        let read = args.reads()?;
        let mut reads = previous.map_or_else(Vec::new, |step| step.reads.clone());
        for param in read.all {
            match reads.iter().find(|known| known.name() == param.name()) {
                None => reads.push(param),
                Some(known) => {
                    if let Some(difference) = known.difference(&param) {
                        return Err(Error::parameter(
                            param.name(),
                            format!("is read by two Params of this name that differ in their {difference}"),
                        ));
                    }
                }
            }
        }
        // what the drawings the call takes left fixed is fixed here too, unless another of them reads it free
        let mut fixed = previous.map_or_else(BTreeSet::new, |step| step.fixed.clone());
        for item in args.taken() {
            if let Ok(step) = &item.last {
                fixed.extend(step.fixed.iter().cloned());
            }
        }
        for param in &read.free {
            fixed.remove(param.name());
        }
        if let Some(values) = args.fixes() {
            fixed.extend(values.names().map(str::to_owned));
        }
        let size = args
            .frame()
            .map_err(|error| error.at_argument(&place, None))?
            .or_else(|| previous.and_then(|step| step.size))
            .or_else(|| args.taken().find_map(Drawing::size));
        let depth = previous
            .map_or(1, |step| step.depth)
            .max(args.taken().map(Drawing::depth).max().map_or(1, |deepest| deepest + 1));
        Ok(Self {
            node,
            args,
            previous: previous.cloned(),
            number: place.step,
            size,
            depth,
            reads,
            fixed,
        })
    }
}

impl fmt::Debug for Drawing {
    /// A summary: a chain has no limit on its length and the drawings it takes nest, so printing it step by step would
    /// overflow the stack.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.last {
            Ok(step) => formatter
                .debug_struct("Drawing")
                .field("steps", &(step.number + 1))
                .field("size", &step.size)
                .finish(),
            Err(failure) => formatter
                .debug_struct("Drawing")
                .field("error", &failure.error)
                .finish(),
        }
    }
}

impl Drop for Step {
    /// Drops the steps before this one in a loop instead of through each other: a chain has no limit on its length,
    /// and dropping it step by step would overflow the stack.
    fn drop(&mut self) {
        let mut earlier = self.previous.take();
        while let Some(step) = earlier {
            // a step someone else still holds is theirs to drop
            earlier = Arc::try_unwrap(step).ok().and_then(|mut owned| owned.previous.take());
        }
    }
}

impl Drawing {
    /// How many drawings deep this one nests; 0 for one that holds an error.
    pub(crate) fn depth(&self) -> usize {
        self.last.as_ref().map_or(0, |step| step.depth)
    }

    /// The drawing of a source `node` called with `given`.
    pub(crate) fn source(node: &'static dyn Node, given: Vec<(&'static str, Given)>) -> Self {
        Self {
            last: Step::build(None, node, given).map(Arc::new).map_err(Failure::from),
        }
    }

    /// This drawing with `node` called on it with `given`: the next step of the chain, or the first error.
    pub(crate) fn then(&self, node: &'static dyn Node, given: Vec<(&'static str, Given)>) -> Self {
        let Ok(step) = &self.last else { return self.clone() };
        Self {
            last: Step::build(Some(step), node, given)
                .map(Arc::new)
                .map_err(Failure::from),
        }
    }

    /// The items stacked in order, the first at the bottom, as a drawing of `size` if one is given.
    fn stack(items: Vec<Self>, size: Option<(f64, f64)>) -> Self {
        let given = size.map_or(Given::Unset, |(width, height)| {
            Given::Pair(Expr::from(width), Expr::from(height))
        });
        let kept = items.clone();
        let mut stack = Self::source(
            &node::stack::Stack,
            vec![("items", Given::Drawings(items)), ("size", given)],
        );
        if let (Err(failure), None) = (&mut stack.last, size) {
            failure.items = Some(kept);
        }
        stack
    }

    /// The items stacked in order, the first at the bottom, each a [`Path`], a [`Shape`] or a `Drawing`. Shapes that
    /// carry no `order` get one, from 0 to 1 in list order. One item and nothing else returns that item as it is.
    ///
    /// The stack has the size its items have; give it one with [`Drawing::with_size`]. Items whose sizes differ leave
    /// the stack with an [`error`](Drawing::error).
    pub fn new<I: Into<Self>>(items: impl IntoIterator<Item = I>) -> Self {
        let mut items: Vec<Self> = items.into_iter().map(Into::into).collect();
        if items.len() == 1
            && let Some(only) = items.pop()
        {
            return only;
        }
        Self::stack(items, None)
    }

    /// This drawing in a design frame of `width` by `height` design pixels. A drawing that is a stack is given the
    /// frame, which also settles a stack whose items differ in size; any other is stacked, alone, in one.
    ///
    /// Sizes above 0 and at most 100,000 are accepted; others leave the drawing with an [`error`](Drawing::error).
    #[must_use]
    pub fn with_size(&self, width: f64, height: f64) -> Self {
        let size = Some((width, height));
        let step = match &self.last {
            Ok(step) => step,
            Err(Failure { items: Some(items), .. }) => return Self::stack(items.clone(), size),
            Err(_) => return self.clone(),
        };
        if step.previous.is_none() && std::ptr::eq(step.node.verb(), std::ptr::from_ref(&node::stack::STACK)) {
            return match step.args.drawings("items") {
                Ok(items) => Self::stack(items.to_vec(), size),
                Err(error) => Self {
                    last: Err(error.into()),
                },
            };
        }
        Self::stack(vec![self.clone()], size)
    }

    /// The first error of the chain that built this drawing, or `None`. A drawing with an error draws nothing: every
    /// step called on it keeps the error.
    #[must_use]
    pub fn error(&self) -> Option<&Error> {
        self.last.as_ref().err().map(|failure| &failure.error)
    }

    /// The design frame `(width, height)` in design pixels, or `None` if neither the drawing nor anything it is made
    /// of has one.
    #[must_use]
    pub fn size(&self) -> Option<(f64, f64)> {
        self.last.as_ref().ok().and_then(|step| step.size)
    }

    /// Every Param the drawing reads, in the order it first uses them.
    ///
    /// # Errors
    /// The first error of the chain that built the drawing; [`Error::Parameter`] naming the Param if two Params of
    /// one name differ in a field.
    pub fn parameters(&self) -> Result<Vec<Param>, Error> {
        self.last
            .as_ref()
            .map(|step| step.free())
            .map_err(|failure| failure.error.clone())
    }

    /// Every Param the drawing reads, those a `with_values` fixed included.
    pub(crate) fn read(&self) -> Result<&[Param], Error> {
        self.last
            .as_ref()
            .map(|step| step.reads.as_slice())
            .map_err(|failure| failure.error.clone())
    }
}

impl From<Path> for Drawing {
    /// A drawing of the one shape that is `path`, painted by default: a closed outline filled black, an open line
    /// stroked black.
    fn from(path: Path) -> Self {
        Self::source(&node::source::PathSource, vec![("path", Given::Path(path))])
    }
}

impl From<Shape> for Drawing {
    /// A drawing of the one shape, with its style and attributes as `shapes()` returned them.
    fn from(shape: Shape) -> Self {
        Self::source(&node::source::ShapeSource, vec![("shape", Given::Shape(shape))])
    }
}

impl From<&Self> for Drawing {
    fn from(drawing: &Self) -> Self {
        drawing.clone()
    }
}
