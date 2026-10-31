//! `Attributes`: what a generator wrote about a shape.

use crate::Point;

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

impl Attributes {
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
