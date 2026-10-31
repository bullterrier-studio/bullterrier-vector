//! `only` and `without`, and the selection they take.

use super::Drawing;
use crate::Point;
use crate::key::{Key, Keyed};
use crate::node::{Given, select};
use crate::shape::Single;
use std::collections::BTreeMap;

/// What a selection asks of one attribute: one value, or a list of which a shape's must be one.
#[derive(Debug, Clone, PartialEq)]
pub struct Wanted(Vec<Single>);

impl From<u32> for Wanted {
    fn from(number: u32) -> Self {
        Self(vec![Single::Number(f64::from(number))])
    }
}

impl From<f64> for Wanted {
    fn from(number: f64) -> Self {
        // a negative zero enters as zero: equal values then have equal bits
        Self(vec![Single::Number(number + 0.0)])
    }
}

impl From<char> for Wanted {
    fn from(character: char) -> Self {
        Self(vec![Single::Char(character)])
    }
}

impl From<Point> for Wanted {
    fn from(point: Point) -> Self {
        Self(vec![Single::Point(point)])
    }
}

impl<T: Into<Wanted>> From<Vec<T>> for Wanted {
    fn from(list: Vec<T>) -> Self {
        Self(list.into_iter().flat_map(|one| one.into().0).collect())
    }
}

impl<T: Into<Wanted>, const N: usize> From<[T; N]> for Wanted {
    fn from(list: [T; N]) -> Self {
        Self(list.into_iter().flat_map(|one| one.into().0).collect())
    }
}

/// Which shapes `only` and `without` act on: the attributes to compare (`index`, `glyph`, `char`, `word`, `line`,
/// `part`, `copy`, `level`, `order`, `origin`) and the values a shape's must equal. A shape matches when every
/// attribute asked about has one of the values given for it.
///
/// Values are constants: a number, a character, a [`Point`], or a list of them for any of. Naming something that is
/// not an attribute, or a value it cannot hold, leaves the drawing with an [`error`](Drawing::error).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Select {
    by_name: BTreeMap<String, Wanted>,
}

impl Keyed for Select {
    fn write_key(&self, key: &mut Key) {
        key.count(self.by_name.len());
        for (name, wanted) in &self.by_name {
            key.text(name);
            key.count(wanted.0.len());
            for single in &wanted.0 {
                key.of(single);
            }
        }
    }
}

impl Select {
    /// A selection that asks nothing yet; `only` and `without` refuse it until an attribute is added.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// This selection also asking that the attribute `name` be `wanted`: `attribute("index", 2)`,
    /// `attribute("char", 'O')`, `attribute("order", [0.0, 1.0])`. Asking about a name twice keeps the last.
    #[must_use]
    pub fn attribute(mut self, name: &str, wanted: impl Into<Wanted>) -> Self {
        self.by_name.insert(name.to_owned(), wanted.into());
        self
    }

    /// Whether nothing is asked.
    pub(crate) fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    /// Each attribute asked about, in name order, with the values of which a shape's must be one.
    pub(crate) fn asked(&self) -> impl Iterator<Item = (&str, &[Single])> {
        self.by_name
            .iter()
            .map(|(name, wanted)| (name.as_str(), wanted.0.as_slice()))
    }
}

impl Drawing {
    /// This drawing with only the shapes that match `select`. The list is renumbered: the shape that was at index 3
    /// after `without(index=2)` is at index 2.
    ///
    /// An unknown attribute name or a value its attribute cannot hold leaves the drawing with an
    /// [`error`](Drawing::error). An attribute that no shape of the list carries is refused when the drawing is drawn,
    /// naming it (`step 2 (only).char`); an empty list has nothing to ask.
    #[must_use]
    pub fn only(&self, select: Select) -> Self {
        self.then(&select::Only, vec![("attributes", Given::Select(select))])
    }

    /// This drawing without the shapes that match `select`: the rest of what [`Drawing::only`] keeps, and refused in
    /// the same cases.
    #[must_use]
    pub fn without(&self, select: Select) -> Self {
        self.then(&select::Without, vec![("attributes", Given::Select(select))])
    }
}
