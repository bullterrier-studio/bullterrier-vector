//! The node contract: what a source or a step of a drawing is, and what it declares about itself.
//!
//! A node is a unit struct that implements a crate-private trait; its declaration is static data, so the registry
//! ([`verbs`](crate::verbs)) can list every verb, its arguments and their ranges without running anything.
//!
//! ```text
//! trait Node: Sync {
//!     fn verb(&self) -> &'static Verb;      // name, role, arguments: the declaration
//!     fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error>;
//! }
//! ```
//!
//! - **Inputs.** `Inputs { previous, taken }`: the shapes of the drawing this step builds on (none for a source), then
//!   of each drawing it takes (a stack's items). A node never sees a `Drawing`.
//! - **Context.** `Context { place, env, work }`: where the step is (for errors), the Param values as this place sees
//!   them (draw values under every `with_values` above it), and the counters. No clock, no caller state.
//! - **Whole list or shape by shape.** `evaluate` takes the whole list. A shape-by-shape node calls the helper
//!   `each_shape(previous, cx, |shape, item| ..)`, which loops, builds the `Item` (`index`, `count` and the shape's `order`) and prefixes
//!   `shape i` to errors, so the loop is written once and no node can leave a default method unimplemented.
//! - **Arguments.** `Args` holds the call's values in declaration order, defaults applied at construction. A node reads
//!   an argument only through an accessor (`args.number("dx", item)?`, `args.point("about", item)?`, ..): the accessor
//!   evaluates the expression for that shape, applies the argument's declared [`Domain`] and names the argument in
//!   the error, so each range is declared once and checked in one place.
//! - **Errors and place.** A step is named by its place: `step 3 (rotate)`; inside a stack
//!   `step 2 (stack) / item 1 / step 3 (rotate)`. An error from an argument or from a `Path` method is placed
//!   (`step 3 (rotate).angle`, `shape 7: ` before the message); an error that is already placed is joined with ` / `.
//!
//! What an argument is, decides who checks it ([`Kind`], [`Reads`]):
//!
//! | Argument | Meaning |
//! |---|---|
//! | expression ([`Reads::PerItem`]) | an `Expr`; a Param may carry one value per shape: `dx`, `angle`, `about`, `alpha`... |
//! | literal ([`Reads::Literal`]) | constants only, no expression: a colour, `join`, `cap`, a blend mode, a stack's `size` |
//!
//! | Argument | Meaning |
//! |---|---|
//! | read once ([`Reads::Once`]) | an `Expr` read once for the whole call: a Param that is given a list is refused. `text`'s `size`, `tracking`, `leading` and `anchor` |
//! | font ([`Kind::Font`]) | a [`Font`](crate::Font), by its bytes: a document names it and the caller supplies the bytes |
//! | axes ([`Kind::Axes`]) | tags to expressions, read per item where the items are the characters of the text: `text`'s `axes`. A Param given a list gives character `i` entry `i` |
//! | features ([`Kind::Features`]) | tags to whole numbers, constants: `text`'s `features` |
//! | drawing ([`Kind::Drawing`]) | one drawing, evaluated with the caller's Param values and handed over as the node's `taken`: the `other` of a boolean |
//!
//! Two more kinds of argument are declared by the first node that needs them, not before, so that no variant exists
//! without a consumer and a test: a whole number (an expression whose value must be whole within ±(2^53 - 1)),
//! first used by `repeat`; and the per-item numbers (`by_index`, `by_order`, `random`, `stagger`).

mod args;
pub(crate) mod boolean;
pub(crate) mod items;
pub(crate) mod select;
pub(crate) mod source;
pub(crate) mod stack;
pub(crate) mod style;
pub(crate) mod text;
pub(crate) mod transform;
pub(crate) mod values;

pub(crate) use args::{Args, Given, Item};

use crate::limits::Work;
use crate::{Error, Shape, Values};
use std::fmt;
use std::sync::Arc;

/// A source or a step of a drawing: its declaration and what it does. A unit struct implements it.
pub(crate) trait Node: Sync + fmt::Debug {
    /// The declaration: name, role, arguments.
    fn verb(&self) -> &'static Verb;

    /// The shapes after this node, from the call's `args`, the shapes before it and of the drawings it takes. An error
    /// returned here is already placed at the step: [`each_shape`] places the errors of a shape-by-shape node.
    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error>;
}

/// A rule across the arguments of a call, run when the call is made.
pub(crate) type Check = fn(&Args) -> Result<(), Error>;

/// What a node builds on.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Inputs<'a> {
    /// The shapes of the drawing this step builds on; empty for a source.
    pub(crate) previous: &'a [Shape],
    /// The shapes of each drawing the node takes, in the order of its arguments.
    pub(crate) taken: &'a [Arc<Vec<Shape>>],
}

/// Where a step is, for its errors: `step 3 (rotate)`, or `source (stack)` for the first call of a chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Place {
    /// The number of the step in its chain, from 1; 0 for the source.
    pub(crate) step: usize,
    /// The verb's name.
    pub(crate) verb: &'static str,
}

impl fmt::Display for Place {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.step == 0 {
            write!(formatter, "source ({})", self.verb)
        } else {
            write!(formatter, "step {} ({})", self.step, self.verb)
        }
    }
}

/// What a node may know of the world besides its inputs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Context<'a> {
    /// Where the step is.
    pub(crate) place: Place,
    /// The values the Params take here.
    pub(crate) env: &'a Values,
    /// What the draw has made so far, to charge what this step creates.
    pub(crate) work: &'a Work,
}

/// The word of `name` read as one of the call's options, found by `named`.
pub(crate) fn choice<T>(args: &Args, name: &str, named: impl Fn(&str) -> Option<T>) -> Result<T, Error> {
    let text = args.text(name)?;
    named(text).ok_or_else(|| Error::parameter(name, format!("is not one of the words it takes, got {text:?}")))
}

/// `map` applied to every shape of `previous` in order, with its [`Item`]. An error from `map` is placed at the step
/// and prefixed with the index of the shape.
pub(crate) fn each_shape(
    previous: &[Shape],
    cx: &Context<'_>,
    mut map: impl FnMut(&Shape, Item) -> Result<Shape, Error>,
) -> Result<Vec<Shape>, Error> {
    let count = previous.len();
    previous
        .iter()
        .enumerate()
        .map(|(index, shape)| {
            let order = shape.attributes().order().unwrap_or(0.0);
            map(shape, Item { index, count, order }).map_err(|error| error.at_argument(&cx.place, Some(index)))
        })
        .collect()
}

/// Whether a declaration is a source of shapes or a step that builds on the shapes before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Makes shapes from nothing but its arguments (a path, a shape, a stack of drawings).
    Source,
    /// Takes the shapes of the drawing before it and returns the shapes after it.
    Step,
}

/// The declaration of one verb: what a drawing can be built from, listed by [`verbs`](crate::verbs).
#[derive(Debug)]
pub struct Verb {
    /// The verb's name, as a caller writes it (`rotate`).
    pub name: &'static str,
    /// Whether it is a source or a step.
    pub role: Role,
    /// One line that states what the verb promises, in the field's terms.
    pub summary: &'static str,
    /// The arguments, in declaration order.
    pub args: &'static [Arg],
    /// The rule across arguments, run when the call is made.
    pub(crate) check: Option<Check>,
}

/// The declaration of one argument of a verb.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arg {
    /// The argument's name, as a caller writes it (`angle`); unique within its verb.
    pub name: &'static str,
    /// The shape of the value.
    pub kind: Kind,
    /// The value taken when the call gives none; `None` when the argument is required. It satisfies [`Arg::domain`].
    pub default: Option<Literal>,
    /// Whether the value is an expression evaluated per shape or a constant.
    pub reads: Reads,
    /// The values the argument accepts, applied wherever the argument is read.
    pub domain: Domain,
}

/// The shape of an argument's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// One number.
    Number,
    /// A point, `(x, y)`.
    Point,
    /// A pair of numbers.
    Pair,
    /// A colour: `"#rrggbb"`, `"#rrggbbaa"` or `"none"`.
    Color,
    /// One of a fixed set of words ([`Domain::Choice`]).
    Text,
    /// One number for both directions, or an `(x, y)` pair (a scale factor).
    Factor,
    /// A path.
    Path,
    /// A shape, as `shapes()` returned it.
    Shape,
    /// Drawings, bottom first.
    Drawings,
    /// One drawing, written as an array of calls: the `other` of a boolean.
    Drawing,
    /// Attribute names with the values a shape's must equal (`only`, `without`).
    Attributes,
    /// Param names with a number, or a list of one number per shape, for each (`with_values`).
    Values,
    /// A [`Font`](crate::Font): a document names an input, and the caller supplies its bytes.
    Font,
    /// Variable-font axis values: four-character tags, each with a number or an expression ([`Reads::PerItem`]).
    Axes,
    /// OpenType feature values: four-character tags, each with a whole number from 0 to 4,294,967,295, never an
    /// expression.
    Features,
}

/// How an argument's value is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reads {
    /// An expression, which a Param can feed with one value per shape.
    PerItem,
    /// An expression read once for the whole call: a Param can feed it one number, never a list.
    Once,
    /// A constant, given when the call is made.
    Literal,
}

/// A constant that an argument defaults to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Literal {
    /// A number.
    Number(f64),
    /// A word.
    Text(&'static str),
    /// Nothing: the argument may be left out, and the step then does what its summary says.
    Unset,
}

/// The values an argument accepts. A number must be finite in every domain but [`Domain::Any`]; the other limits are
/// stated by the variants. Each limit is checked in one place, where the argument is read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Domain {
    /// Anything of the argument's kind.
    Any,
    /// A finite number.
    Finite,
    /// A number within ±1e100, the largest coordinate a path holds.
    Coordinate,
    /// A finite number other than 0.
    NonZero,
    /// A finite number above 0.
    Positive,
    /// A finite number at or above the limit.
    AtLeast(f64),
    /// A number from 0 to 1.
    Fraction,
    /// A finite number above 0 and at or below the limit.
    PositiveUpTo(f64),
    /// One of these words.
    Choice(&'static [&'static str]),
    /// A colour that paints: `"#rrggbb"` or `"#rrggbbaa"`, not `"none"`.
    Painting,
}
