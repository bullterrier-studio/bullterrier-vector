//! Building the drawing from a document that passed the checks of `checks.rs`, and the two checks that need the
//! calls made: `with-values` (a key names a Param the drawing before it reads, a value its Param accepts) while the
//! calls are made, and `call-checks` (what api.md checks when a call is made) once all are made, so that the first
//! `with-values` failure of the document is reported before the first `call-checks` failure.

use super::bare;
use super::checks::Checked;
use super::doc::{Call, Doc, Ex, Value, VerbCall, What};
use super::read::reason;
use super::value::pointer_token;
use crate::expr::Op;
use crate::node::{Args, Given};
use crate::{Drawing, Error, Expr, Font, Param, Path, Shape, Values, Verb};
use std::collections::BTreeMap;

fn refuse(pointer: &str, message: impl Into<String>) -> Error {
    Error::Format {
        pointer: pointer.to_owned(),
        message: message.into(),
    }
}

/// A call to check once the whole document is built.
struct Pending {
    pointer: String,
    verb: &'static Verb,
    given: Vec<(&'static str, Given)>,
}

/// The drawing the document describes.
pub(super) fn build(doc: &Doc, checked: &Checked, fonts: &BTreeMap<String, Font>) -> Result<Drawing, Error> {
    let mut builder = Builder {
        params: &checked.params,
        fonts,
        built: BTreeMap::new(),
        pending: Vec::new(),
    };
    for entry in &checked.order {
        let (name, calls) = &doc.drawings[*entry];
        let drawing = builder.chain(calls)?;
        builder.built.insert(name, drawing);
    }
    let root = builder.chain(&doc.root)?;
    builder.check_calls()?;
    Ok(root)
}

struct Builder<'a> {
    params: &'a BTreeMap<String, Param>,
    fonts: &'a BTreeMap<String, Font>,
    built: BTreeMap<&'a str, Drawing>,
    pending: Vec<Pending>,
}

/// Where in the call at `call` the argument or name `parameter` is: the call's value when it is the one argument of a
/// bare call, else the argument inside the call's object.
fn locate(call: &str, verb: &Verb, parameter: &str) -> String {
    let body = format!("{call}/{}", pointer_token(verb.name));
    let is_argument = verb.args.iter().any(|arg| arg.name == parameter);
    // the one argument of a bare call is the call's value; a name inside it (an attribute, a Param) is a member
    if bare(verb) == is_argument {
        body
    } else {
        format!("{body}/{}", pointer_token(parameter))
    }
}

/// The argument or name an error of the core is about: the last part of `step 3 (rotate).angle`.
fn subject(parameter: &str) -> &str {
    parameter.rsplit('.').next().unwrap_or(parameter)
}

/// `error` of the call at `call` as the error of the document, at the value it is about.
fn placed(call: &str, verb: &Verb, error: &Error) -> Error {
    match error {
        Error::Parameter { parameter, message } => refuse(&locate(call, verb, subject(parameter)), message.clone()),
        other => refuse(&locate(call, verb, ""), reason(other)),
    }
}

impl<'a> Builder<'a> {
    /// The drawing of an array of calls.
    fn chain(&mut self, calls: &'a [Call]) -> Result<Drawing, Error> {
        let mut drawing: Option<Drawing> = None;
        for call in calls {
            drawing = Some(self.call(drawing, call)?);
        }
        drawing.ok_or_else(|| refuse("", "a drawing has at least a source"))
    }

    fn call(&mut self, before: Option<Drawing>, call: &'a Call) -> Result<Drawing, Error> {
        match &call.what {
            What::Reference { name, pointer } => self
                .built
                .get(name.as_str())
                .cloned()
                .ok_or_else(|| refuse(pointer, "is built after the drawing that names it")),
            What::Verb(VerbCall { node, verb, args }) => {
                let mut given = Vec::new();
                for (name, value) in args {
                    given.push((*name, self.given(value)?));
                }
                self.pending.push(Pending {
                    pointer: call.pointer.clone(),
                    verb,
                    given: given.clone(),
                });
                let drawing = match before {
                    None => Drawing::source_unchecked(*node, given),
                    Some(before) => before.then_unchecked(*node, given),
                };
                match drawing.error() {
                    Some(error) => Err(placed(&call.pointer, verb, error)),
                    None => Ok(drawing),
                }
            }
        }
    }

    fn given(&mut self, value: &'a Value) -> Result<Given, Error> {
        Ok(match value {
            Value::Number(ex) => Given::Number(self.expr(ex)?),
            Value::Pair(first, second) => Given::Pair(self.expr(first)?, self.expr(second)?),
            Value::Text(text) => Given::Text(text.clone()),
            Value::Color(color) => Given::Color(*color),
            Value::Paint(paint) => Given::Paint(paint.clone()),
            Value::Path { data, pointer } => {
                Given::Path(Path::from_svg_path(data).map_err(|error| refuse(pointer, reason(&error)))?)
            }
            Value::Shape {
                data,
                pointer,
                attributes,
            } => {
                let path = Path::from_svg_path(data).map_err(|error| refuse(pointer, reason(&error)))?;
                let mut shape = Shape::from_path(path, 0);
                *shape.attributes_mut() = attributes.clone();
                Given::Shape(Box::new(shape))
            }
            Value::Drawings(items) => {
                Given::Drawings(items.iter().map(|item| self.chain(item)).collect::<Result<_, _>>()?)
            }
            Value::Drawing(calls) => Given::Drawing(self.chain(calls)?),
            Value::Select(select) => Given::Select(select.clone()),
            Value::Font { name, pointer } => Given::Font(
                self.fonts
                    .get(name)
                    .ok_or_else(|| refuse(pointer, format!("{name:?} is not an entry of `inputs`")))?
                    .clone(),
            ),
            Value::Axes(axes) if axes.is_empty() => Given::Unset,
            Value::Axes(axes) => Given::Axes(
                axes.iter()
                    .map(|axis| Ok((axis.tag.clone(), self.expr(&axis.value)?)))
                    .collect::<Result<_, Error>>()?,
            ),
            Value::Features(features) if features.is_empty() => Given::Unset,
            Value::Features(features) => Given::Features(
                features
                    .iter()
                    .map(|feature| (feature.tag.clone(), feature.value))
                    .collect(),
            ),
            Value::Runs(runs) => Given::Runs(
                runs.iter()
                    .map(|run| {
                        let given = run
                            .iter()
                            .map(|(name, value)| Ok((*name, self.given(value)?)))
                            .collect::<Result<Vec<_>, Error>>()?;
                        Ok(Args::new(&crate::node::text::RUN, given))
                    })
                    .collect::<Result<_, Error>>()?,
            ),
            Value::Values(fixed) => Given::Values(fixed.iter().fold(Values::new(), |values, one| {
                values.set(one.name.clone(), one.value.clone())
            })),
        })
    }

    fn expr(&self, ex: &Ex) -> Result<Expr, Error> {
        Ok(match ex {
            Ex::Number(number) => Expr::from(*number),
            Ex::Param { name, pointer } => {
                let param = self
                    .params
                    .get(name)
                    .ok_or_else(|| refuse(pointer, format!("{name:?} is not an entry of `params`")))?;
                Expr::from(param)
            }
            Ex::Neg(inner) => -self.expr(inner)?,
            Ex::Item(word, operands) => {
                let operands = operands
                    .iter()
                    .map(|operand| self.expr(operand))
                    .collect::<Result<_, _>>()?;
                Expr::per_item_call(word, operands)
                    .ok_or_else(|| refuse(word, "is not a per-item number this library knows"))?
            }
            Ex::Binary(op, left, right) => {
                let (left, right) = (self.expr(left)?, self.expr(right)?);
                match op {
                    Op::Add => left + right,
                    Op::Sub => left - right,
                    Op::Mul => left * right,
                    Op::Div => left / right,
                }
            }
        })
    }

    /// `call-checks`: what each call checks when it is made.
    fn check_calls(&self) -> Result<(), Error> {
        for pending in &self.pending {
            Args::new(pending.verb, pending.given.clone())
                .validate()
                .map_err(|error| placed(&pending.pointer, pending.verb, &error))?;
        }
        Ok(())
    }
}
