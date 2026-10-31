//! `Args`: the values of one call, in the order the verb declares its arguments, and the accessors a node reads them
//! with.

use super::{Arg, Context, Domain, Kind, Reads, Verb};
use crate::geometry::{coordinate, finite, fraction};
use crate::key::Key;
use crate::limits::MAX_DEPTH;
use crate::{Color, Drawing, Error, Expr, Literal, Param, Path, Point, Select, Shape, Values};

/// The place of a shape in the list a step works on, and how long the list is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Item {
    /// The shape's index in the list, from 0.
    pub(crate) index: usize,
    /// How many shapes the list has.
    pub(crate) count: usize,
}

/// The value a call gives one argument.
#[derive(Debug, Clone)]
pub(crate) enum Given {
    /// A number, or an expression.
    Number(Expr),
    /// Two numbers: a point, a pair, a scale factor.
    Pair(Expr, Expr),
    /// A word.
    Text(String),
    /// A colour.
    Color(Color),
    /// A path.
    Path(Path),
    /// A shape.
    Shape(Shape),
    /// Drawings, bottom first.
    Drawings(Vec<Drawing>),
    /// Attributes to select by.
    Select(Select),
    /// Values for Params.
    Values(Values),
    /// Left out.
    Unset,
}

/// The Params a call reads.
pub(crate) struct ParamsRead {
    /// Every one.
    pub(crate) all: Vec<Param>,
    /// Those not fixed by a `with_values` inside a drawing the call takes.
    pub(crate) free: Vec<Param>,
}

/// The values of a call, in declaration order, defaults applied.
#[derive(Debug)]
pub(crate) struct Args {
    verb: &'static Verb,
    values: Vec<Given>,
}

/// An error that reads an expression's failure as the argument's own.
fn relabel(error: Error, argument: &str) -> Error {
    match error {
        Error::Parameter { parameter, message } if parameter == "expression" => Error::parameter(argument, message),
        Error::Parameter { parameter, message } => Error::parameter(argument, format!("{parameter}: {message}")),
        other => other,
    }
}

/// The error for a number that breaks a rule of its argument.
fn refuse(name: &str, rule: &str, value: f64) -> Error {
    Error::parameter(name, format!("must be {rule}, got {value}"))
}

impl Domain {
    /// `value`, or the error naming `name` unless the domain accepts it.
    pub(crate) fn number(self, name: &str, value: f64) -> Result<f64, Error> {
        match self {
            Self::Any => Ok(value),
            Self::Coordinate => coordinate(name, value),
            Self::Fraction => fraction(name, value),
            Self::Choice(_) => Err(Error::parameter(name, "takes one of a set of words, not a number")),
            Self::Finite | Self::NonZero | Self::Positive | Self::AtLeast(_) | Self::PositiveUpTo(_) => {
                let value = finite(name, value)?;
                let accepted = match self {
                    Self::NonZero => value != 0.0,
                    Self::Positive => value > 0.0,
                    Self::AtLeast(least) => value >= least,
                    Self::PositiveUpTo(most) => value > 0.0 && value <= most,
                    _ => true,
                };
                if accepted {
                    Ok(value)
                } else {
                    Err(refuse(name, &self.rule(), value))
                }
            }
        }
    }

    /// The rule in words, for a refusal.
    fn rule(self) -> String {
        match self {
            Self::NonZero => String::from("a number other than 0"),
            Self::Positive => String::from("above 0"),
            Self::AtLeast(least) => format!("at least {least}"),
            Self::PositiveUpTo(most) => format!("above 0 and at most {most}"),
            Self::Any | Self::Finite | Self::Coordinate | Self::Fraction | Self::Choice(_) => String::from("accepted"),
        }
    }
}

impl Args {
    /// The arguments of `verb` from the `given` ones by name; an argument not given takes its default.
    pub(crate) fn new(verb: &'static Verb, mut given: Vec<(&'static str, Given)>) -> Self {
        let values = verb
            .args
            .iter()
            .map(|arg| {
                if let Some(at) = given.iter().position(|(name, _)| *name == arg.name) {
                    return given.swap_remove(at).1;
                }
                match arg.default {
                    Some(Literal::Number(number)) => Given::Number(Expr::from(number)),
                    Some(Literal::Text(text)) => Given::Text(text.to_owned()),
                    Some(Literal::Unset) | None => Given::Unset,
                }
            })
            .collect();
        Self { verb, values }
    }

    /// The call as numbers: the verb and every argument's value, which is everything but the steps it builds on that
    /// decides the shapes it makes. A drawing the call takes counts only as an item: its own key is its own.
    pub(crate) fn key(&self) -> Vec<u64> {
        let mut key = Key::new();
        key.text(self.verb.name);
        for given in &self.values {
            match given {
                Given::Number(expr) => {
                    key.word(0);
                    key.of(expr);
                }
                Given::Pair(first, second) => {
                    key.word(1);
                    key.of(first);
                    key.of(second);
                }
                Given::Text(text) => {
                    key.word(2);
                    key.text(text);
                }
                Given::Color(color) => {
                    key.word(3);
                    key.of(color);
                }
                Given::Path(path) => {
                    key.word(4);
                    key.of(path);
                }
                Given::Shape(shape) => {
                    key.word(5);
                    key.of(shape);
                }
                Given::Drawings(items) => {
                    key.word(6);
                    key.count(items.len());
                }
                Given::Select(select) => {
                    key.word(7);
                    key.of(select);
                }
                Given::Values(values) => {
                    key.word(8);
                    key.of(values);
                }
                Given::Unset => key.word(9),
            }
        }
        key.finish()
    }

    /// Each declared argument with its value.
    pub(crate) fn declared(&self) -> impl Iterator<Item = (&'static Arg, &Given)> {
        self.verb.args.iter().zip(&self.values)
    }

    /// Every expression the call gives, in declaration order, with its argument.
    pub(crate) fn expressions(&self) -> impl Iterator<Item = (&'static Arg, &Expr)> {
        self.declared().flat_map(|(arg, given)| {
            let found: Vec<&Expr> = match given {
                Given::Number(only) => vec![only],
                Given::Pair(first, second) => vec![first, second],
                _ => Vec::new(),
            };
            found.into_iter().map(move |expr| (arg, expr))
        })
    }

    /// The drawings the call takes, in order.
    pub(crate) fn taken(&self) -> impl Iterator<Item = &Drawing> {
        self.values.iter().flat_map(|given| match given {
            Given::Drawings(items) => items.as_slice(),
            _ => &[],
        })
    }

    /// This call with the Params it fixes checked against the `free` Params of the drawing it is called on, and
    /// stored as the Params take them. The error is unplaced.
    pub(crate) fn fixing(mut self, free: &[Param]) -> Result<Self, Error> {
        for given in &mut self.values {
            if let Given::Values(values) = given {
                *values = values.accepted_by(free)?;
            }
        }
        Ok(self)
    }

    /// The values the call fixes Params to, if it is one that does.
    pub(crate) fn fixes(&self) -> Option<&Values> {
        self.values.iter().find_map(|given| match given {
            Given::Values(values) => Some(values),
            _ => None,
        })
    }

    /// The Params the call reads, in declaration order and left to right: those of its expressions, then of each
    /// drawing it takes. `all` has every one, `free` leaves out what a `with_values` inside a drawing fixed.
    pub(crate) fn reads(&self) -> Result<ParamsRead, Error> {
        let mut reads = ParamsRead {
            all: Vec::new(),
            free: Vec::new(),
        };
        for given in &self.values {
            match given {
                Given::Number(expr) => {
                    expr.collect_params(&mut reads.all);
                    expr.collect_params(&mut reads.free);
                }
                Given::Pair(first, second) => {
                    for expr in [first, second] {
                        expr.collect_params(&mut reads.all);
                        expr.collect_params(&mut reads.free);
                    }
                }
                Given::Drawings(items) => {
                    for item in items {
                        reads.all.extend(item.read()?.iter().cloned());
                        reads.free.extend(item.parameters()?);
                    }
                }
                _ => {}
            }
        }
        Ok(reads)
    }

    /// The checks of a call: every expression is within the depth and size limits, every constant one is worked out and accepted
    /// by its argument's domain, drawings are not nested too deep, then the verb's own rule across arguments. The error is unplaced.
    pub(crate) fn validate(&self) -> Result<(), Error> {
        for (arg, expr) in self.expressions() {
            expr.check_limits().map_err(|error| relabel(error, arg.name))?;
            if !expr.reads_param() {
                let value = expr.value(&Values::new()).map_err(|error| relabel(error, arg.name))?;
                arg.domain.number(arg.name, value)?;
            }
        }
        for (arg, given) in self.declared() {
            if let Given::Drawings(items) = given {
                let nested = items.iter().map(Drawing::depth).max().unwrap_or(0) + 1;
                if nested > MAX_DEPTH {
                    return Err(Error::parameter(
                        arg.name,
                        format!("are nested {nested} drawings deep, but MAX_DEPTH {MAX_DEPTH} is the most accepted"),
                    ));
                }
            }
        }
        self.verb.check.map_or(Ok(()), |check| check(self))
    }

    /// The value and declaration of the argument `name`.
    fn find(&self, name: &str) -> Result<(&'static Arg, &Given), Error> {
        self.declared()
            .find(|(arg, _)| arg.name == name)
            .ok_or_else(|| Error::parameter(name, format!("is not an argument of {}", self.verb.name)))
    }

    /// The error for reading an argument as something it is not.
    fn misread(&self, name: &str, kind: &str) -> Error {
        Error::parameter(name, format!("of {} is not {kind}", self.verb.name))
    }

    /// The number of `name` for the shape `item`, within the argument's domain.
    pub(crate) fn number(&self, cx: &Context<'_>, name: &str, item: Item) -> Result<f64, Error> {
        match self.find(name)? {
            (arg, Given::Number(expr)) => {
                let value = expr.value_at(cx.env, item).map_err(|error| relabel(error, name))?;
                arg.domain.number(name, value)
            }
            _ => Err(self.misread(name, "a number")),
        }
    }

    /// The pair of `name` for the shape `item`, each number within the argument's domain.
    pub(crate) fn pair(&self, cx: &Context<'_>, name: &str, item: Item) -> Result<(f64, f64), Error> {
        match self.find(name)? {
            (arg, Given::Pair(first, second)) => {
                let read = |expr: &Expr| {
                    let value = expr.value_at(cx.env, item).map_err(|error| relabel(error, name))?;
                    arg.domain.number(name, value)
                };
                Ok((read(first)?, read(second)?))
            }
            _ => Err(self.misread(name, "a pair of numbers")),
        }
    }

    /// The point of `name` for the shape `item`, or `None` if the call left it out.
    pub(crate) fn point(&self, cx: &Context<'_>, name: &str, item: Item) -> Result<Option<Point>, Error> {
        match self.find(name)? {
            (_, Given::Unset) => Ok(None),
            (arg, Given::Pair(..)) if arg.kind == Kind::Point => {
                let (x, y) = self.pair(cx, name, item)?;
                Ok(Some(Point::new(x, y)))
            }
            _ => Err(self.misread(name, "a point")),
        }
    }

    /// The constant pair of `name`, or `None` if the call left it out: the argument is a literal, read without Params.
    pub(crate) fn literal_pair(&self, name: &str) -> Result<Option<(f64, f64)>, Error> {
        match self.find(name)? {
            (_, Given::Unset) => Ok(None),
            (arg, Given::Pair(first, second)) if arg.reads == Reads::Literal => {
                let read = |expr: &Expr| {
                    let value = expr.value(&Values::new()).map_err(|error| relabel(error, name))?;
                    arg.domain.number(name, value)
                };
                Ok(Some((read(first)?, read(second)?)))
            }
            _ => Err(self.misread(name, "a constant pair")),
        }
    }

    /// The `size` argument for a verb that declares one: the design frame it gives, if any.
    pub(crate) fn frame(&self) -> Result<Option<(f64, f64)>, Error> {
        if self.verb.args.iter().any(|arg| arg.name == "size") {
            self.literal_pair("size")
        } else {
            Ok(None)
        }
    }

    /// The colour of `name`.
    pub(crate) fn color(&self, name: &str) -> Result<Color, Error> {
        match self.find(name)? {
            (_, Given::Color(color)) => Ok(*color),
            _ => Err(self.misread(name, "a colour")),
        }
    }

    /// The word of `name`.
    pub(crate) fn text(&self, name: &str) -> Result<&str, Error> {
        match self.find(name)? {
            (_, Given::Text(text)) => Ok(text),
            _ => Err(self.misread(name, "a word")),
        }
    }

    /// The path of `name`.
    pub(crate) fn path(&self, name: &str) -> Result<&Path, Error> {
        match self.find(name)? {
            (_, Given::Path(path)) => Ok(path),
            _ => Err(self.misread(name, "a path")),
        }
    }

    /// The shape of `name`.
    pub(crate) fn shape(&self, name: &str) -> Result<&Shape, Error> {
        match self.find(name)? {
            (_, Given::Shape(shape)) => Ok(shape),
            _ => Err(self.misread(name, "a shape")),
        }
    }

    /// The selection of `name`.
    pub(crate) fn select(&self, name: &str) -> Result<&Select, Error> {
        match self.find(name)? {
            (_, Given::Select(select)) => Ok(select),
            _ => Err(self.misread(name, "a selection")),
        }
    }

    /// The drawings of `name`.
    pub(crate) fn drawings(&self, name: &str) -> Result<&[Drawing], Error> {
        match self.find(name)? {
            (_, Given::Drawings(items)) => Ok(items),
            _ => Err(self.misread(name, "drawings")),
        }
    }
}
