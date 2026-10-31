//! Chains of steps as plain data, so that a test can build the same drawing twice and the two have separate caches.

use bullterrier_vector::{Color, Deg, Drawing, Error, Expr, Param, Path, Unit, Values};
use proptest::prelude::*;

/// The numbers a step may take: a literal, or the Param the step is about, alone or times a factor.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Amount {
    Literal(f64),
    Read,
    Scaled(f64),
}

/// A step, or a way of stacking, with what it needs to be built.
#[derive(Debug, Clone)]
pub(crate) enum Op {
    Translate(Amount, Amount),
    Rotate(Amount),
    Scale(Amount),
    Opacity(Amount),
    Fill(u32),
    /// Fixes the Param with this index to the entry with this index of its pool.
    Fix(usize, usize),
    /// The drawing stacked with itself.
    Double,
    /// The drawing stacked over a square.
    Beside,
}

/// The values a Param is given by the edits, each within its range: its pool.
pub(crate) const POOLS: [[f64; 3]; 4] = [[-20.0, 0.0, 7.5], [0.0, 90.0, 400.0], [0.5, 1.0, 1.5], [0.0, 0.5, 1.0]];

/// The four Params the ops read: `x` for translate, `turn` (cyclic) for rotate, `k` for scale, `alpha` for opacity.
pub(crate) fn params() -> Result<[Param; 4], Error> {
    Ok([
        Param::new("x", 0.0, -100.0, 100.0)?.unit(Unit::Px)?,
        Param::new("turn", 0.0, 0.0, 360.0)?.unit(Unit::Deg)?,
        Param::new("k", 1.0, 0.5, 2.0)?,
        Param::new("alpha", 1.0, 0.0, 1.0)?.unit(Unit::Fraction)?,
    ])
}

fn expr(amount: Amount, param: &Param, factor: f64) -> Expr {
    match amount {
        Amount::Literal(number) => Expr::from(number * factor),
        Amount::Read => Expr::from(param),
        Amount::Scaled(by) => param * (by * factor),
    }
}

/// The drawing of a square with `ops` applied in order; the Params are made here, so each call is a drawing of its own
/// with its own cache.
pub(crate) fn materialise(ops: &[Op]) -> Result<Drawing, Error> {
    let [x, turn, k, alpha] = params()?;
    let square = || Path::rect(0.0, 0.0, 10.0, 10.0, 0.0);
    let mut drawing = Drawing::from(square()?);
    let all = [&x, &turn, &k, &alpha];
    for op in ops {
        drawing = match *op {
            Op::Translate(dx, dy) => drawing.translate(expr(dx, &x, 1.0), expr(dy, &x, 1.0)),
            Op::Rotate(angle) => drawing.rotate(Deg(expr(angle, &turn, 1.0))),
            Op::Scale(factor) => drawing.scale(expr(factor, &k, 1.0)),
            Op::Opacity(alpha_by) => drawing.opacity(expr(alpha_by, &alpha, 0.5)),
            Op::Fill(rgb) => drawing.fill(Color::hex(rgb)),
            Op::Fix(param, entry) => {
                // a Param nothing reads (or already fixed) cannot be fixed: such an op does nothing
                let name = all[param % 4].name();
                if drawing.parameters()?.iter().any(|free| free.name() == name) {
                    drawing.with_values(&Values::new().set(name, POOLS[param % 4][entry % 3]))
                } else {
                    drawing
                }
            }
            Op::Double => Drawing::new([drawing.clone(), drawing]),
            Op::Beside => Drawing::new([drawing, Drawing::from(square()?)]),
        };
    }
    Ok(drawing)
}

fn amount() -> impl Strategy<Value = Amount> {
    let pool = prop::sample::select(vec![0.5, 1.0, 1.5, 2.0]);
    prop_oneof![
        pool.clone().prop_map(Amount::Literal),
        Just(Amount::Read),
        pool.prop_map(Amount::Scaled),
    ]
}

/// A chain of one to nine ops, drawn from small pools so that equal steps, and equal chains, come up often.
pub(crate) fn chain() -> impl Strategy<Value = Vec<Op>> {
    let op = prop_oneof![
        3 => (amount(), amount()).prop_map(|(dx, dy)| Op::Translate(dx, dy)),
        2 => amount().prop_map(Op::Rotate),
        2 => amount().prop_map(Op::Scale),
        2 => amount().prop_map(Op::Opacity),
        1 => prop::sample::select(vec![0xff_00_00, 0x00_ff_00]).prop_map(Op::Fill),
        1 => (0..4_usize, 0..3_usize).prop_map(|(param, entry)| Op::Fix(param, entry)),
        1 => Just(Op::Double),
        1 => Just(Op::Beside),
    ];
    prop::collection::vec(op, 1..10)
}

/// A few edits of values: each names some Params by index and gives each an entry of its pool.
pub(crate) fn edits() -> impl Strategy<Value = Vec<Vec<(usize, usize)>>> {
    prop::collection::vec(prop::collection::vec((0..4_usize, 0..3_usize), 0..4), 2..9)
}

/// The values of an edit, for the Params the drawing reads.
pub(crate) fn values_of(edit: &[(usize, usize)], free: &[Param]) -> Values {
    let mut values = Values::new();
    for &(param, entry) in edit {
        let name = ["x", "turn", "k", "alpha"][param % 4];
        if free.iter().any(|known| known.name() == name) {
            values = values.set(name, POOLS[param % 4][entry % 3]);
        }
    }
    values
}
