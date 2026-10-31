//! `Shape`: what a drawing is made of.

mod attributes;
mod color;
mod style;

pub use attributes::Attributes;
pub use color::Color;
pub use style::{Blend, Cap, Join, StrokeStyle, Style};
pub(crate) use style::{DEFAULT_MITER_LIMIT, DEFAULT_STROKE_WIDTH};

use crate::Path;

/// One path with the style it is painted in and the attributes its generator wrote: what [`Drawing::shapes`]
/// returns. A shape is a value to read; `Drawing::from` puts it back into a drawing with its style and attributes.
///
/// [`Drawing::shapes`]: crate::Drawing::shapes
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    path: Path,
    style: Style,
    attributes: Attributes,
    index: usize,
}

impl Shape {
    /// The shape of `path` that no step has painted, at place `index` of the list.
    pub(crate) fn from_path(path: Path, index: usize) -> Self {
        Self {
            path,
            style: Style::unpainted(),
            attributes: Attributes::default(),
            index,
        }
    }

    /// This shape put at place `index` of a list.
    pub(crate) fn at(mut self, index: usize) -> Self {
        self.index = index;
        self
    }

    /// This shape with its path replaced; the style and attributes stay.
    pub(crate) fn with_path(mut self, path: Path) -> Self {
        self.path = path;
        self
    }

    /// The style, to change.
    pub(crate) fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }

    /// The attributes, to change.
    pub(crate) fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }

    /// This shape with the default paint resolved if nothing painted it.
    pub(crate) fn painted(mut self) -> Self {
        self.style.resolve_paint(&self.path);
        self
    }

    /// The outline.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The style as drawn: the default paint is resolved, so an unstyled closed outline has a black fill.
    #[must_use]
    pub fn style(&self) -> &Style {
        &self.style
    }

    /// What the generators wrote.
    #[must_use]
    pub fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    /// The place in the list of shapes the drawing evaluates to, from 0; every step renumbers it.
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }
}
