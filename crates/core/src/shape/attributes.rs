//! `Attributes`: what a generator wrote about a shape.

use crate::Point;
use crate::key::{Key, Keyed};

/// What the step that made a shape wrote about it, each present only where a step wrote it: a shape of a path has
/// none. The shape's `index` is not here: it is its place in the current list, which every step renumbers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Attributes {
    pub(crate) glyph: Option<u32>,
    pub(crate) char: Option<char>,
    pub(crate) word: Option<u32>,
    pub(crate) line: Option<u32>,
    pub(crate) part: Option<u32>,
    pub(crate) copy: Option<u32>,
    pub(crate) level: Option<f64>,
    pub(crate) order: Option<f64>,
    pub(crate) origin: Option<Point>,
}

/// One value an attribute holds, or a selection asks for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Single {
    Number(f64),
    Char(char),
    Point(Point),
}

/// How a value reads in an error message: a number as itself, a character quoted, a point as `(x, y)`.
impl std::fmt::Display for Single {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(number) => write!(formatter, "{number}"),
            Self::Char(character) => write!(formatter, "'{character}'"),
            Self::Point(point) => write!(formatter, "({}, {})", point.x, point.y),
        }
    }
}

/// What kind of value an attribute holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Holds {
    /// A whole number from 0.
    Whole,
    /// Any finite number.
    Real,
    /// One character.
    Char,
    /// A point.
    Point,
}

/// The attributes a selection can ask about, by name: the shape's `index` and the nine a generator writes.
pub(crate) const SELECTABLE: [(&str, Holds); 10] = [
    ("index", Holds::Whole),
    ("glyph", Holds::Whole),
    ("char", Holds::Char),
    ("word", Holds::Whole),
    ("line", Holds::Whole),
    ("part", Holds::Whole),
    ("copy", Holds::Whole),
    ("level", Holds::Real),
    ("order", Holds::Real),
    ("origin", Holds::Point),
];

/// What the attribute `name` holds, or why there is no such attribute.
pub(crate) fn holds(name: &str) -> Result<Holds, String> {
    SELECTABLE
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, holds)| *holds)
        .ok_or_else(|| {
            let known: Vec<&str> = SELECTABLE.iter().map(|(known, _)| *known).collect();
            format!("is not an attribute, the attributes are {}", known.join(", "))
        })
}

impl Attributes {
    /// The value of the attribute `name` that a generator wrote, if one did. `index` is the shape's own and is read
    /// from the shape, not here.
    pub(crate) fn get(&self, name: &str) -> Option<Single> {
        match name {
            "glyph" => self.glyph.map(|n| Single::Number(f64::from(n))),
            "char" => self.char.map(Single::Char),
            "word" => self.word.map(|n| Single::Number(f64::from(n))),
            "line" => self.line.map(|n| Single::Number(f64::from(n))),
            "part" => self.part.map(|n| Single::Number(f64::from(n))),
            "copy" => self.copy.map(|n| Single::Number(f64::from(n))),
            "level" => self.level.map(Single::Number),
            "order" => self.order.map(Single::Number),
            "origin" => self.origin.map(Single::Point),
            _ => None,
        }
    }

    /// The glyph's number in the text, from 0.
    #[must_use]
    pub fn glyph(&self) -> Option<u32> {
        self.glyph
    }

    /// The character the glyph draws.
    #[must_use]
    pub fn char(&self) -> Option<char> {
        self.char
    }

    /// The word it belongs to, from 0.
    #[must_use]
    pub fn word(&self) -> Option<u32> {
        self.word
    }

    /// The line of text it is on, from 0.
    #[must_use]
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    /// The part of a split shape it is, from 0.
    #[must_use]
    pub fn part(&self) -> Option<u32> {
        self.part
    }

    /// The copy of a repeated shape it is, from 0.
    #[must_use]
    pub fn copy(&self) -> Option<u32> {
        self.copy
    }

    /// The level of a traced shape.
    #[must_use]
    pub fn level(&self) -> Option<f64> {
        self.level
    }

    /// Its place among the shapes a generator made, from 0 to 1 (a single shape has 0).
    #[must_use]
    pub fn order(&self) -> Option<f64> {
        self.order
    }

    /// The point it turns and scales about.
    #[must_use]
    pub fn origin(&self) -> Option<Point> {
        self.origin
    }
}

impl Keyed for Single {
    fn write_key(&self, key: &mut Key) {
        match self {
            Self::Number(number) => {
                key.word(0);
                key.number(*number);
            }
            Self::Char(character) => {
                key.word(1);
                key.word(u64::from(u32::from(*character)));
            }
            Self::Point(point) => {
                key.word(2);
                key.of(point);
            }
        }
    }
}

impl Keyed for Attributes {
    fn write_key(&self, key: &mut Key) {
        for whole in [self.glyph, self.word, self.line, self.part, self.copy] {
            key.of(&whole.map(u64::from));
        }
        key.of(&self.char.map(u32::from).map(u64::from));
        key.of(&self.level);
        key.of(&self.order);
        key.of(&self.origin);
    }
}
