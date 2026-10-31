//! The JSON format of formats.md: `to_json` writes the canonical text of a drawing, `from_json` reads any spelling of
//! it and runs the loader's checks. A call is an object with one key, the verb's name, and what its arguments look like
//! comes from the registry's declaration of the verb (`Kind`, `Domain`, defaults); this module has no list of calls.

mod build;
mod checks;
mod depth;
mod doc;
mod parse;
mod read;
mod schema;
mod value;
mod write;

use crate::limits::MAX_DOCUMENT_BYTES;
use crate::{Drawing, Error, Kind, Verb};

/// Whether a call's value is its one argument's own, not an object of arguments: `{"path": "M0 0 L1 1"}`,
/// `{"only": {"char": "O"}}`, `{"with_values": {"spin": 90}}`, `{"shape": {"path": "M0 0"}}`.
pub(crate) fn bare(verb: &Verb) -> bool {
    matches!(verb.args, [only] if matches!(only.kind, Kind::Path | Kind::Shape | Kind::Attributes | Kind::Values))
}

/// The operator an expression word names (`add`), if it names one.
pub(crate) fn expression_word(word: &str) -> Option<crate::expr::Op> {
    use crate::expr::Op;
    [Op::Add, Op::Sub, Op::Mul, Op::Div]
        .into_iter()
        .find(|op| op.word() == word)
}

/// The JSON Schema (draft 2020-12) of the drawing format, as compact JSON text, generated from the declarations of the
/// calls [`verbs`](crate::verbs) lists: the document's keys and the Params, then for each call its name, arguments,
/// defaults and the values each accepts. A document `from_json` reads is valid against it; the checks that relate
/// values across a document (an undeclared Param, a cycle of shared drawings) are the loader's and are not in it.
/// Editors and validators read it, and a call added to the registry appears in it.
#[must_use]
pub fn drawing_schema() -> String {
    schema::generate(crate::verbs())
}

impl Drawing {
    /// The drawing a JSON document describes (formats.md): any spelling the format allows, every check of the loader
    /// run in the order the format lists them. [`Drawing::to_json`] writes the canonical form.
    ///
    /// # Errors
    /// [`Error::Format`] with the JSON Pointer of the offending value if `text` is not a document this library reads:
    /// not UTF-8 I-JSON, past a limit (`MAX_DOCUMENT_BYTES`, `MAX_CALLS`, `MAX_DEPTH`), against the format's schema,
    /// or a check that relates values across the document fails. A value a call refuses keeps the call's message.
    pub fn from_json(text: &str) -> Result<Self, Error> {
        if text.len() > MAX_DOCUMENT_BYTES {
            return Err(Error::Format {
                pointer: String::new(),
                message: format!(
                    "the document is {} bytes, but MAX_DOCUMENT_BYTES {MAX_DOCUMENT_BYTES} is the most accepted",
                    text.len()
                ),
            });
        }
        let top = parse::parse(text)?;
        let doc = read::document(&top)?;
        let checked = checks::run(&doc, &top)?;
        build::build(&doc, &checked)
    }
}
