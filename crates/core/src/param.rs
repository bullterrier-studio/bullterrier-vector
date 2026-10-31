//! `Param`: a number a drawing reads that the caller sets when the drawing is drawn.

use crate::Error;
use crate::geometry::finite;
use crate::key::{Key, Keyed};

/// The names a Param may not take: in Python an output option of the same name sits beside the Param values.
// constant: the output options of api.md "Outputs" (`to_mesh`'s `into`, `to_plotter_svg`'s `pen_mm`)
const RESERVED_NAMES: [&str; 2] = ["into", "pen_mm"];

/// The span of one turn in degrees.
// constant: a full turn
const DEGREES_PER_TURN: f64 = 360.0;

/// The span of one turn in cycles.
// constant: a cycle is one turn
const CYCLES_PER_TURN: f64 = 1.0;

/// What a Param's number means, which a tool shows beside a slider. `Deg` and `Cycle` are cyclic: their range spans
/// one turn and any value wraps into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// A plain number: a font axis, a count, a seed.
    None,
    /// Design pixels.
    Px,
    /// Degrees, clockwise; cyclic over 360.
    Deg,
    /// Cycles (a phase); cyclic over 1.
    Cycle,
    /// A fraction, such as a progress from 0 to 1.
    Fraction,
}

impl Unit {
    /// The size of one turn if the unit is cyclic.
    fn turn(self) -> Option<f64> {
        match self {
            Self::Deg => Some(DEGREES_PER_TURN),
            Self::Cycle => Some(CYCLES_PER_TURN),
            Self::None | Self::Px | Self::Fraction => None,
        }
    }
}

/// A named number with a unit, a range, a default and a meaning. A step may read it wherever it takes a number, alone
/// or inside an expression; its value is given when the drawing is drawn, and the defaults of all Params together
/// are the reference pose.
///
/// A Param in [`Unit::Deg`] or [`Unit::Cycle`] is cyclic: its range spans one turn and any value wraps into it. Every
/// other Param is bounded: a value outside `min..=max` is refused. Two Params are equal when all six fields are.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    name: String,
    default: f64,
    min: f64,
    max: f64,
    unit: Unit,
    meaning: String,
}

impl Keyed for Param {
    /// What decides the number a Param reads: its name, default, range and unit. Its meaning is for people.
    fn write_key(&self, key: &mut Key) {
        key.text(&self.name);
        key.number(self.default);
        key.number(self.min);
        key.number(self.max);
        key.word(match self.unit {
            Unit::None => 0,
            Unit::Px => 1,
            Unit::Deg => 2,
            Unit::Cycle => 3,
            Unit::Fraction => 4,
        });
    }
}

impl Param {
    /// A Param called `name` that takes `default` unless given a value, from `min` to `max`. It has no unit
    /// ([`Unit::None`]) and no meaning until [`Param::unit`] and [`Param::meaning`] say so.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `name` if it is not letters, digits and `_` starting with a letter, or is `into` or
    /// `pen_mm` (output options in Python); naming `default`, `min` or `max` if it is NaN or infinite; naming `min`
    /// if it is not below `max`; naming `default` if it is outside `min..=max`.
    pub fn new(name: &str, default: f64, min: f64, max: f64) -> Result<Self, Error> {
        let mut chars = name.chars();
        let well_formed = chars.next().is_some_and(|first| first.is_ascii_alphabetic())
            && chars.all(|next| next.is_ascii_alphanumeric() || next == '_');
        if !well_formed {
            return Err(Error::parameter(
                "name",
                format!("must be letters, digits and _, starting with a letter, got {name:?}"),
            ));
        }
        if RESERVED_NAMES.contains(&name) {
            return Err(Error::parameter(
                "name",
                format!("{name:?} is the name of an output option, which a Param may not take"),
            ));
        }
        // a negative zero enters as zero: equal values then have equal bits
        let (default, min, max) = (
            finite("default", default)? + 0.0,
            finite("min", min)? + 0.0,
            finite("max", max)? + 0.0,
        );
        if min >= max {
            return Err(Error::parameter("min", format!("must be below max ({max}), got {min}")));
        }
        if !(min..=max).contains(&default) {
            return Err(Error::parameter(
                "default",
                format!("must lie within {min} to {max}, got {default}"),
            ));
        }
        Ok(Self {
            name: name.to_owned(),
            default,
            min,
            max,
            unit: Unit::None,
            meaning: String::new(),
        })
    }

    /// This Param in `unit`. A cyclic unit stores a default at the end of the turn as the start of it, as
    /// [`Param::accept`] reads the same number.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `unit` if it is cyclic ([`Unit::Deg`], [`Unit::Cycle`]) and the range is not
    /// exactly one turn (360 or 1).
    #[expect(clippy::float_cmp, reason = "a range is one turn only if it is exactly one turn")]
    pub fn unit(mut self, unit: Unit) -> Result<Self, Error> {
        if let Some(turn) = unit.turn()
            && self.min + turn != self.max
        {
            return Err(Error::parameter(
                "unit",
                format!(
                    "a {unit:?} Param spans one turn ({turn}), but {} to {} does not",
                    self.min, self.max
                ),
            ));
        }
        self.unit = unit;
        if let Some(turn) = unit.turn() {
            self.default = self.wrapped(turn, self.default);
        }
        Ok(self)
    }

    /// This Param with `meaning`: the line a person reads beside a slider.
    #[must_use]
    pub fn meaning(mut self, meaning: &str) -> Self {
        meaning.clone_into(&mut self.meaning);
        self
    }

    /// The first field in which this Param differs from `other`, with both values, or `None` if they are equal.
    #[expect(
        clippy::float_cmp,
        reason = "two Params are equal only if their numbers are exactly equal"
    )]
    pub(crate) fn difference(&self, other: &Self) -> Option<String> {
        let (ours, theirs) = (self, other);
        if ours.default != theirs.default {
            Some(format!("default ({} against {})", ours.default, theirs.default))
        } else if ours.min != theirs.min {
            Some(format!("min ({} against {})", ours.min, theirs.min))
        } else if ours.max != theirs.max {
            Some(format!("max ({} against {})", ours.max, theirs.max))
        } else if ours.unit != theirs.unit {
            Some(format!("unit ({:?} against {:?})", ours.unit, theirs.unit))
        } else if ours.meaning != theirs.meaning {
            Some(format!("meaning ({:?} against {:?})", ours.meaning, theirs.meaning))
        } else {
            None
        }
    }

    /// The name, as a caller gives its value.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The value taken when none is given; for a cyclic Param, as wrapped into its turn.
    #[must_use]
    pub fn default(&self) -> f64 {
        self.default
    }

    /// The lowest value; the start of the turn of a cyclic Param.
    #[must_use]
    pub fn min(&self) -> f64 {
        self.min
    }

    /// The highest value; the end of the turn of a cyclic Param, which no value reaches.
    #[must_use]
    pub fn max(&self) -> f64 {
        self.max
    }

    /// The unit.
    #[must_use]
    pub fn unit_of(&self) -> Unit {
        self.unit
    }

    /// The meaning, or `""`.
    #[must_use]
    pub fn meaning_of(&self) -> &str {
        &self.meaning
    }

    /// `value` wrapped into the turn of `turn` from the minimum: `min` up to but not including `max`.
    fn wrapped(&self, turn: f64, value: f64) -> f64 {
        let wrapped = self.min + (value - self.min).rem_euclid(turn);
        // a value a hair below the start of the turn comes out as the end, which is the start of the next
        if wrapped < self.max { wrapped } else { self.min }
    }

    /// The value this Param takes when given `value`: a bounded Param keeps it, a cyclic one wraps it into one turn
    /// from its minimum (`min` up to but not including `max`).
    ///
    /// # Errors
    /// [`Error::Parameter`] naming the Param if `value` is NaN or infinite, or, for a bounded Param, outside
    /// `min..=max`.
    pub fn accept(&self, value: f64) -> Result<f64, Error> {
        let value = finite(&self.name, value)?;
        if let Some(turn) = self.unit.turn() {
            Ok(self.wrapped(turn, value))
        } else if (self.min..=self.max).contains(&value) {
            Ok(value)
        } else {
            Err(Error::parameter(
                &self.name,
                format!("must lie within {} to {}, got {value}", self.min, self.max),
            ))
        }
    }
}
