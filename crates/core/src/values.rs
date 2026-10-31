//! `Values`: the numbers a caller gives the Params of a drawing when drawing it.

use std::collections::BTreeMap;

/// What a caller gives one Param: a number, or a list with one number per shape.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// One number, read by every shape.
    Number(f64),
    /// One number per shape: entry `i` goes to the shape whose `index` is `i` at the step that reads the Param.
    List(Vec<f64>),
}

impl From<f64> for Value {
    fn from(number: f64) -> Self {
        // a negative zero enters as zero: equal values then have equal bits
        Self::Number(number + 0.0)
    }
}

impl From<Vec<f64>> for Value {
    fn from(numbers: Vec<f64>) -> Self {
        Self::List(numbers.into_iter().map(|number| number + 0.0).collect())
    }
}

/// The values for the Params of a drawing, by name; a Param not given takes its default. Values are equal when they
/// give the same names the same values, whatever the order they were set in.
///
/// Nothing is checked here: a name no Param has, a number outside a Param's range and a list of the wrong length are
/// refused when the drawing is drawn, naming what is wrong.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Values {
    by_name: BTreeMap<String, Value>,
}

impl Values {
    /// No values: every Param takes its default.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// These values with `name` set to `value` (a number or a `Vec<f64>`), replacing an earlier value of that name.
    #[must_use]
    pub fn set(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.by_name.insert(name.into(), value.into());
        self
    }

    /// The value given for `name`, if one was.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.by_name.get(name)
    }

    /// The names given a value, in order.
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().map(String::as_str)
    }
}
