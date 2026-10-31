//! The list of every verb a drawing can be built from.

use crate::node::{Node, Verb, select, source, stack, style, transform};

/// Every verb the library can build a drawing from. This is the declaration only (name, arguments, ranges); a tool
/// reads it to describe the verbs, and what a verb does stays inside the crate.
pub fn verbs() -> impl Iterator<Item = &'static Verb> {
    NODES.iter().map(|node| node.verb())
}

/// The nodes: sources first.
const NODES: &[&dyn Node] = &[
    &source::PathSource,
    &source::ShapeSource,
    &stack::Stack,
    &transform::Translate,
    &transform::Rotate,
    &transform::Scale,
    &style::Fill,
    &style::Stroke,
    &style::Opacity,
    &style::Blend,
    &select::Only,
    &select::Without,
];
