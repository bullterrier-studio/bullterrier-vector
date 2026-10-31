//! The nesting of a document, counted through references (formats.md, "Limits and security").

use super::value::Json;
use std::collections::{BTreeMap, BTreeSet};

/// The name a `{"drawing": name}` call refers to, if `value` is one.
pub(crate) fn reference(value: &Json) -> Option<&str> {
    match value {
        Json::Object(members) if members.len() == 1 => match members.get("drawing") {
            Some(Json::String(name)) => Some(name),
            _ => None,
        },
        _ => None,
    }
}

/// Every name `value` refers to, in document order of the sorted keys.
fn references<'a>(value: &'a Json, into: &mut Vec<&'a str>) {
    if let Some(name) = reference(value) {
        into.push(name);
        return;
    }
    match value {
        Json::Array(items) => items.iter().for_each(|item| references(item, into)),
        Json::Object(members) => members.values().for_each(|member| references(member, into)),
        Json::Null | Json::Bool(_) | Json::Number(_) | Json::String(_) => {}
    }
}

/// The nesting of `value` in place, a reference counting as the array it names: scalars add nothing, an array or an
/// object one level over its deepest member. The recursion is as deep as the text, which the parser bounds.
fn nesting(value: &Json, named: &BTreeMap<&str, usize>) -> usize {
    if let Some(name) = reference(value) {
        return named.get(name).copied().unwrap_or_default();
    }
    match value {
        Json::Array(items) => 1 + items.iter().map(|item| nesting(item, named)).max().unwrap_or(0),
        Json::Object(members) => 1 + members.values().map(|member| nesting(member, named)).max().unwrap_or(0),
        Json::Null | Json::Bool(_) | Json::Number(_) | Json::String(_) => 0,
    }
}

/// How deep the document `top` nests, the top-level value counting 1 and a `{"drawing": name}` call counting as the
/// array `top`'s `drawings` name. Each shared drawing is measured once, the ones it refers to first and without
/// recursion through them, so a long chain of references costs a loop, not stack. A cycle or a name that is not
/// declared (the loader refuses both before it asks) counts as nothing.
pub(crate) fn of(top: &Json) -> usize {
    let Json::Object(members) = top else { return 0 };
    let entries: BTreeMap<&str, &Json> = match members.get("drawings") {
        Some(Json::Object(entries)) => entries.iter().map(|(name, entry)| (name.as_str(), entry)).collect(),
        _ => BTreeMap::new(),
    };
    let mut named: BTreeMap<&str, usize> = BTreeMap::new();
    let mut expanded: BTreeSet<&str> = BTreeSet::new();
    for first in entries.keys() {
        let mut pending = vec![(*first, false)];
        while let Some((name, ready)) = pending.pop() {
            let Some(entry) = entries.get(name) else { continue };
            if named.contains_key(name) {
                continue;
            }
            if ready {
                named.insert(name, nesting(entry, &named));
            } else if expanded.insert(name) {
                pending.push((name, true));
                let mut refers = Vec::new();
                references(entry, &mut refers);
                pending.extend(
                    refers
                        .into_iter()
                        .filter(|other| !named.contains_key(other))
                        .map(|other| (other, false)),
                );
            }
        }
    }
    nesting(top, &named)
}
