//! Arithmetic on fields: `add`, `mul`, `min`, `max` and `abs`, read point by point. Subtraction, negation and clamping
//! are spelled with these (`a + -1 * b`, `max(min(f, high), low)`), so they have no node of their own.

use super::flow::MAX_MIX_DEPTH;
use super::{Field, FieldKind};
use crate::Error;
use crate::limits::MAX_EXPR_NODES;
use std::fmt;
use std::sync::Arc;

/// An operation on fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    Add,
    Mul,
    Min,
    Max,
    Abs,
}

impl Op {
    /// The word a document calls the operation, after `Field.`.
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Mul => "mul",
            Self::Min => "min",
            Self::Max => "max",
            Self::Abs => "abs",
        }
    }

    /// The operation the document word `word` names.
    pub(crate) fn of_word(word: &str) -> Option<Self> {
        [Self::Add, Self::Mul, Self::Min, Self::Max, Self::Abs]
            .into_iter()
            .find(|op| op.word() == word)
    }

    /// The operation on the readings of its operands (`abs` reads only the first).
    pub(super) fn apply(self, first: f64, second: f64) -> f64 {
        match self {
            Self::Add => first + second,
            Self::Mul => first * second,
            Self::Min => first.min(second),
            Self::Max => first.max(second),
            Self::Abs => first.abs(),
        }
    }

    /// The operation as the Python package spells it.
    pub(super) fn write(self, f: &mut fmt::Formatter<'_>, operands: &[Field]) -> fmt::Result {
        let wrapped = |field: &Field| match field.kind() {
            FieldKind::Arithmetic {
                op: Self::Add | Self::Mul,
                ..
            } => format!("({field})"),
            FieldKind::Constant(value) if value.constant().is_none() => format!("({field})"),
            _ => field.to_string(),
        };
        match (self, operands) {
            (Self::Add, [a, b]) => write!(f, "{} + {}", wrapped(a), wrapped(b)),
            (Self::Mul, [a, b]) => write!(f, "{} * {}", wrapped(a), wrapped(b)),
            (Self::Min | Self::Max, [a, b]) => write!(f, "{}.{}({b})", wrapped(a), self.word()),
            (_, operands) => write!(
                f,
                "abs({})",
                operands.first().map(ToString::to_string).unwrap_or_default()
            ),
        }
    }
}

/// The depth and the nodes of an arithmetic over `operands`.
pub(super) fn sizes(operands: &[Field]) -> (usize, usize) {
    (
        1 + operands.iter().map(|operand| operand.depth).max().unwrap_or(0),
        operands
            .iter()
            .fold(1_usize, |nodes, operand| nodes.saturating_add(operand.nodes)),
    )
}

impl Field {
    /// This field plus `other`, point by point. A number is a [`Field::constant`]; two numbers give the number, and a
    /// number is kept as the first operand (addition commutes exactly).
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `other` if the result would nest arithmetic more than 16 deep or read more than
    /// 10,000 fields and operations.
    pub fn add(&self, other: &Self) -> Result<Self, Error> {
        self.combined(Op::Add, other)
    }

    /// This field times `other`, point by point; `mul` by -1 negates, so `a - b` is `a.add(&b.mul(&minus_one)?)`.
    /// Numbers as for [`Field::add`].
    ///
    /// # Errors
    /// As [`Field::add`].
    pub fn mul(&self, other: &Self) -> Result<Self, Error> {
        self.combined(Op::Mul, other)
    }

    /// The smaller of this field and `other` at each point; with [`Field::max`] it clamps.
    ///
    /// # Errors
    /// As [`Field::add`].
    pub fn min(&self, other: &Self) -> Result<Self, Error> {
        self.combined(Op::Min, other)
    }

    /// The larger of this field and `other` at each point.
    ///
    /// # Errors
    /// As [`Field::add`].
    pub fn max(&self, other: &Self) -> Result<Self, Error> {
        self.combined(Op::Max, other)
    }

    /// The size of this field at each point, never negative.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `field` if the result would nest arithmetic more than 16 deep or read more than
    /// 10,000 fields and operations.
    pub fn abs(&self) -> Result<Self, Error> {
        if let Some(number) = self.number() {
            return Self::constant(number.abs());
        }
        Self::arithmetic(Op::Abs, vec![self.clone()], "field")
    }

    /// `self op other`, numbers folded and a number first.
    fn combined(&self, op: Op, other: &Self) -> Result<Self, Error> {
        if let (FieldKind::Constant(a), FieldKind::Constant(b)) = (self.kind(), other.kind()) {
            match (op, self.number(), other.number()) {
                // two numbers are the number they make; an operand reading a Param keeps the expression
                (_, Some(a), Some(b)) => return Self::constant(op.apply(a, b)),
                (Op::Add, ..) => return Self::constant(a + b),
                (Op::Mul, ..) => return Self::constant(a * b),
                _ => {}
            }
        }
        let swap = matches!(op, Op::Add | Op::Mul) && other.number().is_some() && self.number().is_none();
        let operands = if swap {
            vec![other.clone(), self.clone()]
        } else {
            vec![self.clone(), other.clone()]
        };
        Self::arithmetic(op, operands, "other")
    }

    fn arithmetic(op: Op, operands: Vec<Self>, name: &'static str) -> Result<Self, Error> {
        let (depth, nodes) = sizes(&operands);
        if depth > MAX_MIX_DEPTH || nodes > MAX_EXPR_NODES {
            return Err(Error::parameter(
                name,
                format!(
                    "arithmetic on fields nests at most {MAX_MIX_DEPTH} deep and reads at most {MAX_EXPR_NODES} fields and operations"
                ),
            ));
        }
        Self::made(&FieldKind::Arithmetic {
            op,
            operands: Arc::from(operands),
        })
    }

    /// The number this field is, if it is a constant that reads no Param.
    fn number(&self) -> Option<f64> {
        match self.kind() {
            FieldKind::Constant(value) => value.constant(),
            _ => None,
        }
    }
}
