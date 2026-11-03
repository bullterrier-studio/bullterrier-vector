//! The checks of formats.md "Checked by the loader, not the schema" that read the document alone, in the order the
//! document lists them. The first to fail is the error. (`json-text` and `limits` run while the text is read, and
//! `with-values` and `call-checks` while the drawing is built: see `build.rs`.)

use super::depth;
use super::doc::{Call, Doc, Ex, InputKind, Value, What};
use super::read::reason;
use super::value::{Json, pointer_token};
use crate::limits::MAX_DEPTH;
use crate::{Error, Font, Param, Path};
use std::collections::{BTreeMap, BTreeSet};

fn refuse(pointer: &str, message: impl Into<String>) -> Error {
    Error::Format {
        pointer: pointer.to_owned(),
        message: message.into(),
    }
}

/// What the checks found out that the build needs.
pub(super) struct Checked {
    /// The Params, by name.
    pub(super) params: BTreeMap<String, Param>,
    /// The indexes into `Doc::drawings` of the shared drawings, each after the ones it refers to.
    pub(super) order: Vec<usize>,
}

/// Visits `calls` and the calls of the drawings they take, each call before the calls inside it.
fn each_call<'a>(calls: &'a [Call], visit: &mut impl FnMut(&'a Call) -> Result<(), Error>) -> Result<(), Error> {
    for call in calls {
        visit(call)?;
        if let What::Verb(verb) = &call.what {
            for (_, value) in &verb.args {
                if let Value::Drawings(items) = value {
                    for item in items {
                        each_call(item, visit)?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// The drawings of the document in the order the checks look at them: the shared ones by name, then the root.
fn arrays(doc: &Doc) -> impl Iterator<Item = &[Call]> {
    doc.drawings
        .iter()
        .map(|(_, calls)| calls.as_slice())
        .chain(std::iter::once(doc.root.as_slice()))
}

/// Every call of the document in that order.
fn each_call_of<'a>(doc: &'a Doc, mut visit: impl FnMut(&'a Call) -> Result<(), Error>) -> Result<(), Error> {
    arrays(doc).try_for_each(|calls| each_call(calls, &mut visit))
}

/// Every Param a call reads or fixes itself, with the pointer: its own expressions and `with_values` keys, not those of
/// the drawings it takes.
fn params_of(call: &Call) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    if let What::Verb(verb) = &call.what {
        for (_, value) in &verb.args {
            match value {
                Value::Number(ex) => ex.params(&mut found),
                Value::Pair(first, second) => {
                    first.params(&mut found);
                    second.params(&mut found);
                }
                Value::Axes(axes) => axes.iter().for_each(|axis| axis.value.params(&mut found)),
                Value::Values(fixed) => found.extend(fixed.iter().map(|one| (one.name.as_str(), one.pointer.as_str()))),
                _ => {}
            }
        }
    }
    found
}

pub(super) fn run(doc: &Doc, top: &Json) -> Result<Checked, Error> {
    let order = drawings(doc, top)?;
    let params = params(doc)?;
    inputs(doc)?;
    drawing_used(doc)?;
    path_data(doc)?;
    Ok(Checked { params, order })
}

/// `drawing-declared`, `drawing-acyclic` and `depth`; the order the shared drawings can be built in.
fn drawings(doc: &Doc, top: &Json) -> Result<Vec<usize>, Error> {
    let index: BTreeMap<&str, usize> = doc
        .drawings
        .iter()
        .enumerate()
        .map(|(at, (name, _))| (name.as_str(), at))
        .collect();
    // drawing-declared
    each_call_of(doc, |call| match &call.what {
        What::Reference { name, pointer } if !index.contains_key(name.as_str()) => {
            Err(refuse(pointer, format!("{name:?} is not an entry of `drawings`")))
        }
        _ => Ok(()),
    })?;
    // drawing-acyclic: a walk of the references that keeps its own stack, since a chain of them has no limit yet
    let mut edges: Vec<Vec<(usize, &str)>> = Vec::new();
    for (_, calls) in &doc.drawings {
        let mut found = Vec::new();
        each_call(calls, &mut |call| {
            if let What::Reference { name, pointer } = &call.what
                && let Some(target) = index.get(name.as_str())
            {
                found.push((*target, pointer.as_str()));
            }
            Ok(())
        })?;
        edges.push(found);
    }
    let mut state = vec![0_u8; doc.drawings.len()];
    let mut order = Vec::new();
    for start in 0..doc.drawings.len() {
        if state[start] != 0 {
            continue;
        }
        state[start] = 1;
        let mut stack = vec![(start, 0_usize)];
        while let Some((node, next)) = stack.last().copied() {
            if let Some(&(target, pointer)) = edges[node].get(next) {
                if let Some(top) = stack.last_mut() {
                    top.1 += 1;
                }
                match state[target] {
                    0 => {
                        state[target] = 1;
                        stack.push((target, 0));
                    }
                    1 => {
                        let from = stack.iter().position(|(entry, _)| *entry == target).unwrap_or_default();
                        let mut names: Vec<&str> = stack[from..]
                            .iter()
                            .map(|(entry, _)| doc.drawings[*entry].0.as_str())
                            .collect();
                        names.push(doc.drawings[target].0.as_str());
                        return Err(refuse(
                            pointer,
                            format!("closes a cycle of shared drawings: {}", names.join(" -> ")),
                        ));
                    }
                    _ => {}
                }
            } else {
                state[node] = 2;
                order.push(node);
                stack.pop();
            }
        }
    }
    // depth
    let nesting = depth::of(top);
    if nesting > MAX_DEPTH {
        return Err(refuse(
            "",
            format!(
                "the document nests {nesting} levels deep through its references, but MAX_DEPTH {MAX_DEPTH} is the most accepted"
            ),
        ));
    }
    Ok(order)
}

/// `param-range`, `param-unique`, `param-declared`, `param-order` and `param-used`.
fn params(doc: &Doc) -> Result<BTreeMap<String, Param>, Error> {
    let mut params = BTreeMap::new();
    let mut made = Vec::new();
    for decl in &doc.params {
        let param = Param::new(&decl.name, decl.default, decl.min, decl.max)
            .and_then(|param| param.unit(decl.unit))
            .map_err(|error| match &error {
                Error::Parameter { parameter, message }
                    if ["name", "default", "min", "max", "unit"].contains(&parameter.as_str()) =>
                {
                    refuse(&format!("{}/{parameter}", decl.pointer), message.clone())
                }
                other => refuse(&decl.pointer, reason(other)),
            })?;
        made.push(param.meaning(&decl.meaning));
    }
    for (decl, param) in doc.params.iter().zip(made) {
        if params.insert(decl.name.clone(), param).is_some() {
            return Err(refuse(
                &format!("{}/name", decl.pointer),
                format!("{:?} names an earlier Param too: a name appears once", decl.name),
            ));
        }
    }
    each_call_of(doc, |call| {
        match params_of(call)
            .into_iter()
            .find(|(name, _)| !params.contains_key(*name))
        {
            Some((name, pointer)) => Err(refuse(pointer, format!("{name:?} is not an entry of `params`"))),
            None => Ok(()),
        }
    })?;
    let entries: BTreeMap<&str, &[Call]> = doc
        .drawings
        .iter()
        .map(|(name, calls)| (name.as_str(), calls.as_slice()))
        .collect();
    let mut read = Reads::default();
    let mut walked = BTreeSet::new();
    first_reads(&doc.root, &entries, &mut walked, &mut read);
    let in_document: Vec<&str> = doc
        .params
        .iter()
        .map(|decl| decl.name.as_str())
        .filter(|name| read.seen.contains(name))
        .collect();
    let read = read.order;
    if in_document != read {
        return Err(refuse(
            "/params",
            format!(
                "are not in the order the drawing first reads them; that order is {}",
                read.join(", ")
            ),
        ));
    }
    let read: BTreeSet<&str> = read.into_iter().collect();
    if let Some(unused) = doc.params.iter().find(|decl| !read.contains(decl.name.as_str())) {
        return Err(refuse(&unused.pointer, "is read by no call"));
    }
    Ok(params)
}

/// The Params read so far: in the order they were first read, and as a set to ask in.
#[derive(Default)]
struct Reads<'a> {
    order: Vec<&'a str>,
    seen: BTreeSet<&'a str>,
}

/// The Params the walk of formats.md reads, each once, in the order it first reads them: a drawing's calls from its
/// source to its last step, a call's arguments in the verb's order, a shared drawing when it is first reached. The
/// recursion is as deep as the references nest, which `depth` has bounded.
fn first_reads<'a>(
    calls: &'a [Call],
    entries: &BTreeMap<&str, &'a [Call]>,
    walked: &mut BTreeSet<&'a str>,
    read: &mut Reads<'a>,
) {
    for call in calls {
        match &call.what {
            What::Reference { name, .. } => {
                if let Some(entry) = entries.get(name.as_str())
                    && walked.insert(name)
                {
                    first_reads(entry, entries, walked, read);
                }
            }
            What::Verb(verb) => {
                for (_, value) in &verb.args {
                    let mut found = Vec::new();
                    match value {
                        Value::Number(ex) => ex.params(&mut found),
                        Value::Pair(first, second) => {
                            first.params(&mut found);
                            second.params(&mut found);
                        }
                        Value::Axes(axes) => axes.iter().for_each(|axis| axis.value.params(&mut found)),
                        Value::Drawings(items) => {
                            for item in items {
                                first_reads(item, entries, walked, read);
                            }
                        }
                        _ => {}
                    }
                    for (name, _) in found {
                        if read.seen.insert(name) {
                            read.order.push(name);
                        }
                    }
                }
            }
        }
    }
}

/// `input-declared` and `input-used`.
fn inputs(doc: &Doc) -> Result<(), Error> {
    let mut read = BTreeSet::new();
    each_call_of(doc, |call| {
        let What::Verb(verb) = &call.what else { return Ok(()) };
        for (_, value) in &verb.args {
            if let Value::Font { name, pointer } = value {
                match doc.inputs.iter().find(|input| &input.name == name) {
                    None => return Err(refuse(pointer, format!("{name:?} is not an entry of `inputs`"))),
                    Some(input) if input.kind != InputKind::Font => {
                        return Err(refuse(pointer, format!("{name:?} is declared as a map, not a font")));
                    }
                    Some(_) => {
                        read.insert(name.as_str());
                    }
                }
            }
        }
        Ok(())
    })?;
    match doc.inputs.iter().find(|input| !read.contains(input.name.as_str())) {
        Some(input) => Err(refuse(&input.pointer, "is read by no call")),
        None => Ok(()),
    }
}

/// `font-checks`: with the fonts supplied, the axes and features a call names (and the axis a `fit` names) are the font's,
/// and an axis value written as a number lies in its range.
///
/// # Errors
/// [`Error::Parameter`] naming `inputs.` and the input: the caller supplied a font the document was not written for.
pub(super) fn fonts(doc: &Doc, fonts: &BTreeMap<String, Font>) -> Result<(), Error> {
    each_call_of(doc, |call| {
        let What::Verb(verb) = &call.what else { return Ok(()) };
        let Some((name, font)) = verb.args.iter().find_map(|(_, value)| match value {
            Value::Font { name, .. } => fonts.get(name).map(|font| (name, font)),
            _ => None,
        }) else {
            return Ok(());
        };
        let wrong = |error: Error, pointer: &str| match error {
            Error::Parameter { message, .. } => Error::parameter(
                &format!("inputs.{name}"),
                format!("{message}; the document uses it at {pointer}"),
            ),
            other => other,
        };
        for (argument, value) in &verb.args {
            match value {
                Value::Text(fit) if *argument == "fit" && fit != "size" => {
                    font.axis_range(fit, "fit")
                        .map_err(|error| wrong(error, &format!("{}/{}/fit", call.pointer, verb.verb.name)))?;
                }
                Value::Axes(axes) => {
                    for axis in axes {
                        let number = match axis.value {
                            Ex::Number(number) => Some(number),
                            _ => None,
                        };
                        font.check_axis(&axis.tag, number)
                            .map_err(|error| wrong(error, &axis.pointer))?;
                    }
                }
                Value::Features(features) => {
                    for feature in features {
                        font.check_feature(&feature.tag)
                            .map_err(|error| wrong(error, &feature.pointer))?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    })
}

/// `drawing-used`.
fn drawing_used(doc: &Doc) -> Result<(), Error> {
    let mut used = BTreeSet::new();
    each_call_of(doc, |call| {
        if let What::Reference { name, .. } = &call.what {
            used.insert(name.as_str());
        }
        Ok(())
    })?;
    match doc.drawings.iter().find(|(name, _)| !used.contains(name.as_str())) {
        Some((name, _)) => Err(refuse(
            &format!("/drawings/{}", pointer_token(name)),
            "is used by no call",
        )),
        None => Ok(()),
    }
}

/// `path-data`.
fn path_data(doc: &Doc) -> Result<(), Error> {
    each_call_of(doc, |call| {
        let What::Verb(verb) = &call.what else { return Ok(()) };
        for (_, value) in &verb.args {
            if let Value::Path { data, pointer } | Value::Shape { data, pointer, .. } = value {
                Path::from_svg_path(data).map_err(|error| refuse(pointer, reason(&error)))?;
            }
        }
        Ok(())
    })
}
