//! Per-item numbers: `by_index`, `by_order`, `random` and `stagger`, numbers whose value is worked out for each shape
//! of a list from the shape's own attributes (api.md, "Per-item numbers"). They are the one place that knows what
//! each reads, what it refuses and how it is worked out; [`Expr`] holds one as a node and asks nothing more of it than
//! its operands, its value and its words.
//!
//! The value of `random` is a function of `seed` and the shape's `index` alone, by the `SplitMix64` generator: the
//! `index + 1`-th output of the generator that starts at `seed` (negative seeds as two's complement), whose top 53 bits
//! make a number from 0 up to but not including 1 exactly. It is integer arithmetic and one exact conversion, so it is
//! the same on every target and uses no clock, no global state and no transcendental maths.

use super::Expr;
use crate::Error;
use crate::node::Item;

/// The `seed` of `random` when none is given.
pub(crate) const DEFAULT_SEED: f64 = 0.0;
/// The `spread` of `stagger` when none is given.
pub(crate) const DEFAULT_SPREAD: f64 = 0.5;

/// The largest whole number a `f64` holds exactly, which bounds a seed.
// constant: 2^53 - 1, the largest whole number a 64-bit float holds together with every whole number below it
const MAX_SEED: f64 = 9_007_199_254_740_991.0;

/// One operand of a per-item number as the JSON format spells it: its name, and the value it takes when left out.
pub(crate) struct Field {
    pub(crate) name: &'static str,
    pub(crate) default: Option<f64>,
}

const fn required(name: &'static str) -> Field {
    Field { name, default: None }
}

const fn optional(name: &'static str, default: f64) -> Field {
    Field {
        name,
        default: Some(default),
    }
}

/// The per-item numbers, by the word the JSON format calls each, with the operands it takes in order.
pub(crate) const FUNCTIONS: [(&str, &[Field]); 4] = [
    ("by_index", &[required("first"), required("last")]),
    ("by_order", &[required("first"), required("last")]),
    (
        "random",
        &[required("low"), required("high"), optional("seed", DEFAULT_SEED)],
    ),
    ("stagger", &[required("progress"), optional("spread", DEFAULT_SPREAD)]),
];

/// A per-item number and its operands, each itself a number.
#[derive(Debug, PartialEq)]
pub(crate) enum Call {
    /// `first` on the first shape of the list, `last` on the last, linear between.
    ByIndex([Expr; 2]),
    /// As `ByIndex`, read from each shape's `order`.
    ByOrder([Expr; 2]),
    /// `low`, `high`, `seed`.
    Random([Expr; 3]),
    /// `progress`, `spread`.
    Stagger([Expr; 2]),
}

impl Call {
    /// The call `word` names with these operands, or `None` for a word that names none or a wrong number of operands.
    pub(crate) fn new(word: &str, operands: Vec<Expr>) -> Option<Self> {
        match word {
            "by_index" => operands.try_into().ok().map(Self::ByIndex),
            "by_order" => operands.try_into().ok().map(Self::ByOrder),
            "random" => operands.try_into().ok().map(Self::Random),
            "stagger" => operands.try_into().ok().map(Self::Stagger),
            _ => None,
        }
    }

    /// The word the JSON format calls this number.
    pub(crate) fn word(&self) -> &'static str {
        match self {
            Self::ByIndex(_) => FUNCTIONS[0].0,
            Self::ByOrder(_) => FUNCTIONS[1].0,
            Self::Random(_) => FUNCTIONS[2].0,
            Self::Stagger(_) => FUNCTIONS[3].0,
        }
    }

    /// The operands' names, in the order of [`Call::operands`].
    pub(crate) fn fields(&self) -> &'static [Field] {
        match self {
            Self::ByIndex(_) => FUNCTIONS[0].1,
            Self::ByOrder(_) => FUNCTIONS[1].1,
            Self::Random(_) => FUNCTIONS[2].1,
            Self::Stagger(_) => FUNCTIONS[3].1,
        }
    }

    /// The operands, in the order of [`Call::fields`].
    pub(crate) fn operands(&self) -> &[Expr] {
        match self {
            Self::ByIndex(operands) | Self::ByOrder(operands) | Self::Stagger(operands) => operands,
            Self::Random(operands) => operands,
        }
    }

    /// The operands, taken out.
    pub(super) fn into_operands(self) -> Vec<Expr> {
        match self {
            Self::ByIndex(operands) | Self::ByOrder(operands) | Self::Stagger(operands) => Vec::from(operands),
            Self::Random(operands) => Vec::from(operands),
        }
    }

    /// The refusals that need only operands that are numbers already: those of `constant` operands are checked, the
    /// others wait until they are worked out. The error is unplaced.
    pub(super) fn check_literals(&self) -> Result<(), Error> {
        match self {
            Self::ByIndex(_) | Self::ByOrder(_) => Ok(()),
            Self::Random([low, high, seed]) => {
                if let (Some(low), Some(high)) = (low.constant(), high.constant()) {
                    check_range(low, high)?;
                }
                seed.constant().map_or(Ok(()), check_seed)
            }
            Self::Stagger([_, spread]) => spread.constant().map_or(Ok(()), check_spread),
        }
    }

    /// The value for `item`, with each operand worked out by `operand` (for the same item). The error is unplaced.
    pub(super) fn evaluate(
        &self,
        mut operand: impl FnMut(&Expr) -> Result<f64, Error>,
        item: Item,
    ) -> Result<f64, Error> {
        Ok(match self {
            Self::ByIndex([first, last]) => {
                let (first, last) = (operand(first)?, operand(last)?);
                blend(first, last, share(item.index, item.count))
            }
            Self::ByOrder([first, last]) => {
                let (first, last) = (operand(first)?, operand(last)?);
                blend(first, last, item.order)
            }
            Self::Random([low, high, seed]) => {
                let (low, high, seed) = (operand(low)?, operand(high)?, operand(seed)?);
                check_range(low, high)?;
                check_seed(seed)?;
                low + (high - low) * unit(seed, item.index)
            }
            Self::Stagger([progress, spread]) => {
                let (progress, spread) = (operand(progress)?, operand(spread)?);
                check_spread(spread)?;
                // the shape at order `p` runs while progress goes from `p * spread` for a length of `1 - spread`
                let start = item.order * spread;
                ((progress - start) / (1.0 - spread)).clamp(0.0, 1.0)
            }
        })
    }
}

/// `first` at 0 and `last` at 1, exactly, and linear between.
fn blend(first: f64, last: f64, at: f64) -> f64 {
    first * (1.0 - at) + last * at
}

/// Where item `index` of `count` is, from 0 at the first to 1 at the last (0 for a list of one).
#[expect(clippy::cast_precision_loss, reason = "a list is far shorter than 2^53 shapes")]
pub(crate) fn share(index: usize, count: usize) -> f64 {
    if count < 2 {
        0.0
    } else {
        index as f64 / (count - 1) as f64
    }
}

fn check_range(low: f64, high: f64) -> Result<(), Error> {
    if low > high {
        return Err(Error::parameter(
            "low",
            format!("must not be above high, got {low} above {high}"),
        ));
    }
    Ok(())
}

fn check_seed(seed: f64) -> Result<(), Error> {
    if seed.fract() != 0.0 || seed.abs() > MAX_SEED {
        return Err(Error::parameter(
            "seed",
            format!("must be a whole number within ±{MAX_SEED}, got {seed}"),
        ));
    }
    Ok(())
}

fn check_spread(spread: f64) -> Result<(), Error> {
    if !(0.0..1.0).contains(&spread) {
        return Err(Error::parameter(
            "spread",
            format!("must be from 0 up to but not including 1, got {spread}"),
        ));
    }
    Ok(())
}

/// A number from 0 up to but not including 1, the same for the same `seed` and `index` on every target.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "the seed is a whole number within ±(2^53 - 1), and the 53 bits kept fit a float exactly"
)]
fn unit(seed: f64, index: usize) -> f64 {
    // SplitMix64 (Steele, Lea and Flood 2014): the state steps by an odd constant and is scrambled to the output
    const STEP: u64 = 0x9E37_79B9_7F4A_7C15;
    // constant: 2^-53, so that 53 bits make a float exactly
    const SCALE: f64 = 1.0 / 9_007_199_254_740_992.0;
    let state = (seed as i64)
        .cast_unsigned()
        .wrapping_add(STEP.wrapping_mul(index as u64 + 1));
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 * SCALE
}

/// A number from `first` on the first shape of the list to `last` on the last, linear between, by the shape's `index`;
/// `first` where the list has one shape.
pub fn by_index(first: impl Into<Expr>, last: impl Into<Expr>) -> Expr {
    Expr::item(Call::ByIndex([first.into(), last.into()]))
}

/// As [`by_index`], by the shape's `order`: `first` at order 0 and `last` at order 1. A shape that carries no `order`
/// (a lone path or shape given as it is) has 0.
pub fn by_order(first: impl Into<Expr>, last: impl Into<Expr>) -> Expr {
    Expr::item(Call::ByOrder([first.into(), last.into()]))
}

/// A number from `low` up to `high` for each shape, from the `seed` and the shape's `index` in the list the step works
/// on (so it shifts when an earlier step removes or renumbers shapes). The same seed and index give the same number on
/// every target. It converts into an [`Expr`] and combines with the operators like one.
///
/// A drawing refuses `low` above `high` and a `seed` that is not a whole number within ±(2^53 - 1), when the call is
/// made if they are plain numbers, else when drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Random {
    low: Expr,
    high: Expr,
    seed: Expr,
}

impl Random {
    /// Numbers from `low` to `high`, with seed 0.
    pub fn new(low: impl Into<Expr>, high: impl Into<Expr>) -> Self {
        Self {
            low: low.into(),
            high: high.into(),
            seed: DEFAULT_SEED.into(),
        }
    }

    /// The seed: a different seed gives different numbers.
    #[must_use]
    pub fn seed(mut self, seed: impl Into<Expr>) -> Self {
        self.seed = seed.into();
        self
    }
}

impl From<Random> for Expr {
    fn from(random: Random) -> Self {
        Self::item(Call::Random([random.low, random.high, random.seed]))
    }
}

/// Each shape's own progress from 0 to 1 out of one `progress`: the shape at order `p` runs while `progress` goes from
/// `p * spread` to `p * spread + 1 - spread`, and holds 0 before and 1 after. `spread` 0 runs all together, towards 1
/// one after another. It converts into an [`Expr`] and combines with the operators like one.
///
/// A drawing refuses a `spread` outside 0 up to but not including 1, when the call is made if it is a plain number,
/// else when drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Stagger {
    progress: Expr,
    spread: Expr,
}

impl Stagger {
    /// Staggered progress from `progress`, with spread 0.5.
    pub fn new(progress: impl Into<Expr>) -> Self {
        Self {
            progress: progress.into(),
            spread: DEFAULT_SPREAD.into(),
        }
    }

    /// The spread.
    #[must_use]
    pub fn spread(mut self, spread: impl Into<Expr>) -> Self {
        self.spread = spread.into();
        self
    }
}

impl From<Stagger> for Expr {
    fn from(stagger: Stagger) -> Self {
        Self::item(Call::Stagger([stagger.progress, stagger.spread]))
    }
}
