//! The list of every verb a drawing can be built from.

use crate::node::{Node, Verb, items, select, source, stack, style, text, transform, values};

/// Every verb the library can build a drawing from. This is the declaration only (name, arguments, ranges); a tool
/// reads it to describe the verbs, and what a verb does stays inside the crate.
pub fn verbs() -> impl Iterator<Item = &'static Verb> {
    NODES.iter().map(|node| node.verb())
}

/// The node called `name`, if there is one.
pub(crate) fn node(name: &str) -> Option<&'static dyn Node> {
    NODES.iter().copied().find(|node| node.verb().name == name)
}

/// The nodes: sources first.
const NODES: &[&dyn Node] = &[
    &source::PathSource,
    &source::ShapeSource,
    &stack::Stack,
    &text::TextNode,
    &transform::Translate,
    &transform::Rotate,
    &transform::Scale,
    &transform::Along,
    &style::Fill,
    &style::Stroke,
    &style::Opacity,
    &style::Blend,
    &items::Split,
    &items::Merge,
    &select::Only,
    &select::Without,
    &values::WithValues,
];
