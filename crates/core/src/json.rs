//! The JSON format of formats.md: `to_json` writes the canonical text of a drawing, `from_json` reads any spelling of
//! it and runs the loader's checks. A call is an object with one key, the verb's name, and what its arguments look like
//! comes from the registry's declaration of the verb (`Kind`, `Domain`, defaults); this module has no list of calls.

mod depth;
mod value;
mod write;

use crate::{Kind, Verb};

/// Whether a call's value is its one argument's own, not an object of arguments: `{"path": "M0 0 L1 1"}`,
/// `{"only": {"char": "O"}}`, `{"with_values": {"spin": 90}}`, `{"shape": {"path": "M0 0"}}`.
pub(crate) fn bare(verb: &Verb) -> bool {
    matches!(verb.args, [only] if matches!(only.kind, Kind::Path | Kind::Shape | Kind::Attributes | Kind::Values))
}
