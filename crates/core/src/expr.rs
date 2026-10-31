//! `Expr`: arithmetic on Params and numbers, worked out when a drawing is drawn.

use crate::node::Item;
use crate::{Error, Param, Value, Values};
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::sync::Arc;

/// The deepest expression a call accepts, counting a number or a Param as one level and each operator as one more
/// than its deepest operand. It bounds the recursion that evaluates an expression and is the limit a saved drawing's
/// loader applies.
// constant: formats.md "Limits and security" sets MAX_DEPTH to 128 for documents; an expression follows it so a drawing
// written in code can be saved and loaded back
pub(crate) const MAX_DEPTH: usize = 128;

/// The four binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

impl Op {
    fn symbol(self) -> char {
        match self {
            Self::Add => '+',
            Self::Sub => '-',
            Self::Mul => '*',
            Self::Div => '/',
        }
    }
}

#[derive(Debug, PartialEq)]
enum Node {
    Const(f64),
    Param(Param),
    Neg(Expr),
    Binary(Op, Expr, Expr),
}

/// A node with what is known of the tree below it, worked out when it is built so that asking costs nothing.
#[derive(Debug, PartialEq)]
struct Tree {
    node: Node,
    /// Levels: a number or a Param is 1, an operator one more than its deepest operand.
    depth: usize,
    /// Whether any Param is read below.
    reads_param: bool,
}

impl Node {
    /// Moves the operands out of this node into `into`, leaving a number behind.
    fn take_operands(&mut self, into: &mut Vec<Expr>) {
        match std::mem::replace(self, Self::Const(0.0)) {
            Self::Neg(operand) => into.push(operand),
            Self::Binary(_, left, right) => into.extend([left, right]),
            Self::Const(_) | Self::Param(_) => {}
        }
    }
}

impl Drop for Tree {
    /// Drops the operands one by one instead of through each other: operators never fail, so a loop can build an
    /// expression a million levels deep, and dropping it level by level would overflow the stack.
    fn drop(&mut self) {
        let mut pending = Vec::new();
        self.node.take_operands(&mut pending);
        while let Some(operand) = pending.pop() {
            // an operand someone else still holds is theirs to drop; the last holder's operands are taken out first
            if let Ok(mut tree) = Arc::try_unwrap(operand.0) {
                tree.node.take_operands(&mut pending);
            }
        }
    }
}

/// A number made of Params, numbers and the operators `+`, `-`, `*`, `/` and unary `-`. It is a plain value and
/// cheap to clone; building one never fails, and its value is worked out when a drawing is drawn, with the values the
/// caller gives the Params.
///
/// Anywhere a step takes a number, an `Expr` goes too; so does a [`Param`] and a plain number. The operators work on
/// `Expr`, `&Expr`, `Param`, `&Param` and `f64` on either side (not between two plain numbers), so nothing needs to be
/// cloned: `1.0 + 0.08 * &pulse`. An expression prints infix with the fewest parentheses that keep its shape:
/// `(a + b) * c`.
///
/// Every number it works out is finite: a division by zero, an overflow or a NaN is a
/// [`ParameterError`](Error::Parameter) naming the expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr(Arc<Tree>);

impl Expr {
    fn new(node: Node, depth: usize, reads_param: bool) -> Self {
        Self(Arc::new(Tree {
            node,
            depth,
            reads_param,
        }))
    }

    fn binary(op: Op, left: Self, right: Self) -> Self {
        let depth = 1 + left.0.depth.max(right.0.depth);
        let reads_param = left.0.reads_param || right.0.reads_param;
        Self::new(Node::Binary(op, left, right), depth, reads_param)
    }

    fn negate(inner: Self) -> Self {
        let (depth, reads_param) = (1 + inner.0.depth, inner.0.reads_param);
        Self::new(Node::Neg(inner), depth, reads_param)
    }

    /// How tightly the expression binds when it is an operand: `+` and `-` loosest, then `*` and `/`, then
    /// everything that closes itself (a number, a Param, a negation, which prints its operand in parentheses).
    fn rank(&self) -> u8 {
        match &self.0.node {
            Node::Binary(Op::Add | Op::Sub, ..) => 1,
            Node::Binary(Op::Mul | Op::Div, ..) => 2,
            Node::Const(_) | Node::Param(_) | Node::Neg(_) => 3,
        }
    }

    /// The number this expression is, when it reads no Param and has a finite value; `None` if it reads a Param (its
    /// value is known only when the drawing is drawn) or if no finite number results (such as `1 / 0`; [`Expr::value`]
    /// says why).
    #[must_use]
    pub fn constant(&self) -> Option<f64> {
        if self.0.reads_param {
            None
        } else {
            self.value(&Values::new()).ok()
        }
    }

    /// The value of the expression with the Params at the values given, and at their defaults where none is given. A
    /// Param is read as [`Param::accept`] says: a cyclic one wraps its value, a bounded one refuses a value outside its
    /// range.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming the Param if a value given for it is not accepted, or if it was given a list (a
    /// list has an entry per shape, so it is read where there are shapes); naming the expression if it is deeper than
    /// 128 levels, if it divides by zero, or if any step of it is NaN or infinite.
    pub fn value(&self, values: &Values) -> Result<f64, Error> {
        self.check_depth()?;
        self.evaluate(values, None)
    }

    /// The value for the shape `item` of a list: a Param given a list reads the entry at the shape's index.
    pub(crate) fn value_at(&self, values: &Values, item: Item) -> Result<f64, Error> {
        self.check_depth()?;
        self.evaluate(values, Some(item))
    }

    /// Whether the expression reads a Param, so that its value is known only when drawn.
    pub(crate) fn reads_param(&self) -> bool {
        self.0.reads_param
    }

    /// An error unless the expression is at most [`MAX_DEPTH`] levels deep, which bounds the recursion that
    /// evaluates and prints it.
    pub(crate) fn check_depth(&self) -> Result<(), Error> {
        if self.0.depth > MAX_DEPTH {
            return Err(Error::parameter(
                "expression",
                format!("is {} levels deep, but at most {MAX_DEPTH} are accepted", self.0.depth),
            ));
        }
        Ok(())
    }

    /// Adds the Params this expression reads to `into`, left to right, each time it reads one. The depth must have
    /// been checked.
    pub(crate) fn collect_params(&self, into: &mut Vec<Param>) {
        match &self.0.node {
            Node::Const(_) => {}
            Node::Param(param) => into.push(param.clone()),
            Node::Neg(inner) => inner.collect_params(into),
            Node::Binary(_, left, right) => {
                left.collect_params(into);
                right.collect_params(into);
            }
        }
    }

    /// The value, with the recursion bounded by the depth that was checked.
    fn evaluate(&self, values: &Values, item: Option<Item>) -> Result<f64, Error> {
        let number = match &self.0.node {
            Node::Const(number) => *number,
            Node::Param(param) => match values.get(param.name()) {
                None => param.default(),
                Some(Value::Number(number)) => param.accept(*number)?,
                Some(Value::List(numbers)) => {
                    let entry = item
                        .filter(|item| item.count == numbers.len())
                        .and_then(|item| numbers.get(item.index));
                    match (entry, item) {
                        (Some(number), _) => param.accept(*number)?,
                        (None, Some(item)) => {
                            return Err(Error::parameter(
                                param.name(),
                                format!(
                                    "was given a list of {} numbers, but there are {} shapes here",
                                    numbers.len(),
                                    item.count
                                ),
                            ));
                        }
                        (None, None) => {
                            return Err(Error::parameter(
                                param.name(),
                                "was given a list, which has an entry per shape and cannot be read without shapes",
                            ));
                        }
                    }
                }
            },
            Node::Neg(inner) => -inner.evaluate(values, item)?,
            Node::Binary(op, left, right) => {
                let (left, right) = (left.evaluate(values, item)?, right.evaluate(values, item)?);
                match op {
                    Op::Add => left + right,
                    Op::Sub => left - right,
                    Op::Mul => left * right,
                    Op::Div if right == 0.0 => {
                        return Err(Error::parameter("expression", format!("division by zero in ({self})")));
                    }
                    Op::Div => left / right,
                }
            }
        };
        if number.is_finite() {
            Ok(number)
        } else {
            Err(Error::parameter(
                "expression",
                format!("({self}) is not a finite number, it is {number}"),
            ))
        }
    }
}

impl From<f64> for Expr {
    /// A plain number. A negative zero is stored as zero.
    fn from(number: f64) -> Self {
        Self::new(Node::Const(number + 0.0), 1, false)
    }
}

impl From<&Param> for Expr {
    fn from(param: &Param) -> Self {
        Self::new(Node::Param(param.clone()), 1, true)
    }
}

impl From<Param> for Expr {
    fn from(param: Param) -> Self {
        Self::new(Node::Param(param), 1, true)
    }
}

impl From<&Self> for Expr {
    fn from(expr: &Self) -> Self {
        expr.clone()
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0.node {
            Node::Const(number) => write!(formatter, "{number}"),
            Node::Param(param) => formatter.write_str(param.name()),
            Node::Neg(inner) => write!(formatter, "-({inner})"),
            Node::Binary(op, left, right) => {
                let rank = self.rank();
                // operators of one rank read left to right, so only a right operand of the same rank needs them
                if left.rank() < rank {
                    write!(formatter, "({left})")?;
                } else {
                    write!(formatter, "{left}")?;
                }
                write!(formatter, " {} ", op.symbol())?;
                if right.rank() <= rank {
                    write!(formatter, "({right})")
                } else {
                    write!(formatter, "{right}")
                }
            }
        }
    }
}

/// `left op right` for one pair of operand kinds, and for each pair but two plain numbers.
macro_rules! binary_operators {
    ($($trait:ident :: $method:ident => $op:ident),+ $(,)?) => {$(
        binary_operators!(@row $trait, $method, $op; Expr; Expr, &Expr, Param, &Param, f64);
        binary_operators!(@row $trait, $method, $op; &Expr; Expr, &Expr, Param, &Param, f64);
        binary_operators!(@row $trait, $method, $op; Param; Expr, &Expr, Param, &Param, f64);
        binary_operators!(@row $trait, $method, $op; &Param; Expr, &Expr, Param, &Param, f64);
        binary_operators!(@row $trait, $method, $op; f64; Expr, &Expr, Param, &Param);
    )+};
    (@row $trait:ident, $method:ident, $op:ident; $left:ty; $($right:ty),+) => {$(
        impl $trait<$right> for $left {
            type Output = Expr;

            fn $method(self, right: $right) -> Expr {
                Expr::binary(Op::$op, Expr::from(self), Expr::from(right))
            }
        }
    )+};
}

binary_operators!(Add::add => Add, Sub::sub => Sub, Mul::mul => Mul, Div::div => Div);

/// `-operand` for each kind of operand.
macro_rules! negations {
    ($($operand:ty),+) => {$(
        impl Neg for $operand {
            type Output = Expr;

            fn neg(self) -> Expr {
                Expr::negate(Expr::from(self))
            }
        }
    )+};
}

negations!(Expr, &Expr, Param, &Param);
