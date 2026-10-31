//! The list of every verb a drawing can be built from.

use crate::node::Verb;

/// Every verb the library can build a drawing from. This is the declaration only (name, arguments, ranges); a tool
/// reads it to describe the verbs, and what a verb does stays inside the crate.
pub fn verbs() -> impl Iterator<Item = &'static Verb> {
    VERBS.iter().copied()
}

/// The declared verbs.
const VERBS: &[&Verb] = &[];
