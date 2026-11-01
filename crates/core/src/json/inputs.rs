//! The inputs of a document: what the caller supplies by name (formats.md, "Inputs"), and the loader's check
//! `inputs-supplied`.

use super::doc::{Doc, InputDecl, InputKind};
use crate::{Error, Font};
use std::collections::BTreeMap;
use std::fmt;

/// The bytes a document's inputs name, by name: `Inputs::new().set("anybody", font_bytes)`. A document stays small
/// and holds no bytes; whoever loads it ([`Drawing::from_json`]) supplies them.
///
/// [`Drawing::from_json`]: crate::Drawing::from_json
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Inputs {
    by_name: BTreeMap<String, Vec<u8>>,
}

impl Inputs {
    /// No inputs: for a document that declares none.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// These inputs with `bytes` supplied as `name`, replacing an earlier input of that name.
    #[must_use]
    pub fn set(mut self, name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        self.by_name.insert(name.into(), bytes.into());
        self
    }
}

impl fmt::Debug for Inputs {
    /// The names and the lengths, never the bytes.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_map()
            .entries(self.by_name.iter().map(|(name, bytes)| (name, bytes.len())))
            .finish()
    }
}

/// `inputs-supplied`: the fonts of `doc`, each read from the bytes supplied under its name, or, for a font whose
/// `sha256` is that of a bundled font and which `supplied` does not give, from the bundled one.
///
/// # Errors
/// [`Error::Parameter`] naming `inputs` if the document declares an input that neither `supplied` nor the library gives, or
/// `supplied` gives a name the document does not declare, two names have equal bytes, bytes do not match the `sha256` of their
/// input, or a font cannot be read.
pub(super) fn fonts(doc: &Doc, supplied: &Inputs) -> Result<BTreeMap<String, Font>, Error> {
    let refuse = |name: &str, message: String| Error::parameter(&format!("inputs.{name}"), message);
    let mut given: Vec<(&InputDecl, &[u8])> = Vec::new();
    for declared in &doc.inputs {
        let bundled = declared
            .sha256
            .as_deref()
            .filter(|_| declared.kind == InputKind::Font)
            .and_then(Font::bundled_bytes);
        let Some(bytes) = supplied.by_name.get(&declared.name).map(Vec::as_slice).or(bundled) else {
            return Err(refuse(
                &declared.name,
                "is declared by the document, but `inputs` does not give it".into(),
            ));
        };
        given.push((declared, bytes));
    }
    if let Some(name) = supplied
        .by_name
        .keys()
        .find(|name| doc.inputs.iter().all(|declared| &declared.name != *name))
    {
        return Err(refuse(
            name,
            "is given, but the document declares no input of this name".into(),
        ));
    }
    let mut by_bytes: BTreeMap<&[u8], &str> = BTreeMap::new();
    for &(declared, bytes) in &given {
        if let Some(first) = by_bytes.insert(bytes, &declared.name) {
            return Err(refuse(
                &declared.name,
                format!("has the same bytes as {first:?}: one input is given one name"),
            ));
        }
    }
    let mut fonts = BTreeMap::new();
    for &(declared, bytes) in given.iter().filter(|(declared, _)| declared.kind == InputKind::Font) {
        let font = Font::new(bytes)
            .map_err(|error| refuse(&declared.name, super::read::reason(&error)))?
            .named(declared.name.as_str())?;
        if let Some(expected) = declared.sha256.as_deref()
            && expected != font.sha256()
        {
            return Err(refuse(
                &declared.name,
                format!("has the SHA-256 {}, but the document says {expected}", font.sha256()),
            ));
        }
        fonts.insert(declared.name.clone(), font);
    }
    Ok(fonts)
}
