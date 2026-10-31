//! `Values`: the numbers a caller gives the Params of a drawing when drawing it.

use crate::key::{Key, Keyed};
use crate::{Error, Param};
use std::collections::BTreeMap;

/// What a caller gives one Param: a number, or a list with one number per shape.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// One number, read by every shape.
    Number(f64),
    /// One number per shape: entry `i` goes to the shape whose `index` is `i` at the step that reads the Param.
    List(Vec<f64>),
}

impl Keyed for Value {
    fn write_key(&self, key: &mut Key) {
        match self {
            Self::Number(number) => {
                key.word(0);
                key.number(*number);
            }
            Self::List(numbers) => {
                key.word(1);
                key.count(numbers.len());
                for number in numbers {
                    key.number(*number);
                }
            }
        }
    }
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

impl Keyed for Values {
    fn write_key(&self, key: &mut Key) {
        key.count(self.by_name.len());
        for (name, value) in &self.by_name {
            key.text(name);
            key.of(value);
        }
    }
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

    /// An error unless every name given is one of the `free` Params, listing them.
    pub(crate) fn names_within(&self, free: &[Param]) -> Result<(), Error> {
        match self.names().find(|name| free.iter().all(|param| param.name() != *name)) {
            None => Ok(()),
            Some(name) if free.is_empty() => Err(Error::parameter(
                name,
                "the drawing reads no Params, so there is no value to give it",
            )),
            Some(name) => {
                let known: Vec<&str> = free.iter().map(Param::name).collect();
                Err(Error::parameter(
                    name,
                    format!("the drawing reads no Param of this name, only {}", known.join(", ")),
                ))
            }
        }
    }

    /// These values, each as its Param takes it (a cyclic one wrapped), or the error for the first that is not
    /// accepted. Every name must be one of the `free` Params.
    pub(crate) fn accepted_by(&self, free: &[Param]) -> Result<Self, Error> {
        self.names_within(free)?;
        let mut accepted = Self::new();
        for (name, value) in &self.by_name {
            let Some(param) = free.iter().find(|param| param.name() == name) else {
                continue;
            };
            let value = match value {
                Value::Number(number) => Value::Number(param.accept(*number)?),
                Value::List(numbers) => Value::List(
                    numbers
                        .iter()
                        .map(|number| param.accept(*number))
                        .collect::<Result<_, _>>()?,
                ),
            };
            accepted.by_name.insert(name.clone(), value);
        }
        Ok(accepted)
    }

    /// These values with `fixed` laid over them: where both give a name, `fixed` wins.
    pub(crate) fn over(&self, fixed: &Self) -> Self {
        let mut values = self.clone();
        values
            .by_name
            .extend(fixed.by_name.iter().map(|(name, value)| (name.clone(), value.clone())));
        values
    }
}
