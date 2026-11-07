//! A document as the reader understands it: every value typed by the registry's declaration of its verb, every name
//! unresolved, every position remembered as a JSON Pointer for the checks that come after.

use crate::expr::Op;
use crate::node::Node;
use crate::{Attributes, Color, Select, Unit, Verb};

/// A whole document.
pub(super) struct Doc {
    pub(super) params: Vec<ParamDecl>,
    pub(super) inputs: Vec<InputDecl>,
    /// The shared drawings by name, in name order.
    pub(super) drawings: Vec<(String, Vec<Call>)>,
    pub(super) root: Vec<Call>,
}

/// An entry of `params`, as written.
pub(super) struct ParamDecl {
    pub(super) pointer: String,
    pub(super) name: String,
    pub(super) default: f64,
    pub(super) min: f64,
    pub(super) max: f64,
    pub(super) unit: Unit,
    pub(super) meaning: String,
}

/// An entry of `inputs`, as written.
pub(super) struct InputDecl {
    pub(super) pointer: String,
    pub(super) name: String,
    pub(super) kind: InputKind,
    /// The SHA-256 the bytes must have, if the document says.
    pub(super) sha256: Option<String>,
}

/// What an input is read as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InputKind {
    Font,
    Map,
}

/// One call of a drawing.
pub(super) struct Call {
    /// Where the call is: `/root/2`.
    pub(super) pointer: String,
    pub(super) what: What,
}

pub(super) enum What {
    /// `{"drawing": name}`, with the pointer of the name.
    Reference { name: String, pointer: String },
    /// A registered verb with the arguments that were written, in the verb's declaration order.
    Verb(VerbCall),
}

pub(super) struct VerbCall {
    pub(super) node: &'static dyn Node,
    pub(super) verb: &'static Verb,
    pub(super) args: Vec<(&'static str, Value)>,
}

/// The value of one argument.
pub(super) enum Value {
    Number(Ex),
    Pair(Ex, Ex),
    Text(String),
    Color(Color),
    Path {
        data: String,
        pointer: String,
    },
    Shape {
        data: String,
        pointer: String,
        attributes: Attributes,
    },
    Drawings(Vec<Vec<Call>>),
    Select(Select),
    Values(Vec<Fixed>),
    /// The name of an input, with the pointer of the name.
    Font {
        name: String,
        pointer: String,
    },
    /// Axis values by tag, in tag order.
    Axes(Vec<Setting<Ex>>),
    /// Feature values by tag, in tag order.
    Features(Vec<Setting<u32>>),
}

/// An entry of an object keyed by a font tag: the tag, the pointer of the entry, and its value.
pub(super) struct Setting<T> {
    pub(super) tag: String,
    pub(super) pointer: String,
    pub(super) value: T,
}

/// A Param a `with_values` fixes, and the value it is fixed to.
pub(super) struct Fixed {
    pub(super) name: String,
    /// Where the name is: `/root/3/with_values/spin`.
    pub(super) pointer: String,
    pub(super) value: crate::Value,
}

/// A number or an expression, with the pointer of each Param it reads.
#[derive(Clone)]
pub(super) enum Ex {
    Number(f64),
    Param {
        name: String,
        pointer: String,
    },
    Neg(Box<Ex>),
    Binary(Op, Box<Ex>, Box<Ex>),
    /// A per-item number: its word and its operands in the order of its fields, those left out filled with their defaults.
    Item(&'static str, Vec<Ex>),
}

impl Ex {
    /// Whether this is a plain number: no Param is read and no per-item number is held.
    pub(super) fn is_plain(&self) -> bool {
        match self {
            Self::Number(_) => true,
            Self::Param { .. } | Self::Item(..) => false,
            Self::Neg(inner) => inner.is_plain(),
            Self::Binary(_, left, right) => left.is_plain() && right.is_plain(),
        }
    }

    /// Each Param read, left to right, with the pointer of the read.
    pub(super) fn params<'a>(&'a self, into: &mut Vec<(&'a str, &'a str)>) {
        match self {
            Self::Number(_) => {}
            Self::Param { name, pointer } => into.push((name, pointer)),
            Self::Neg(inner) => inner.params(into),
            Self::Binary(_, left, right) => {
                left.params(into);
                right.params(into);
            }
            Self::Item(_, operands) => operands.iter().for_each(|operand| operand.params(into)),
        }
    }
}
