//! The drawings that come with the library, by name: JSON documents (formats.md) that `Drawing::from_json` reads. The
//! table is the one owner of the names; each text is the canonical `to_json` of a program in `examples/`.

use crate::{Drawing, Error, Inputs};

/// The names and texts, sorted by name.
const PRESETS: [(&str, &str); 3] = [
    ("bent_type", include_str!("../presets/bent_type.json")),
    ("gradient_type", include_str!("../presets/gradient_type.json")),
    ("ring", include_str!("../presets/ring.json")),
];

impl Drawing {
    /// A drawing that comes with the library, by name; [`Drawing::preset_names`] lists them. It is an ordinary
    /// drawing: its [`parameters`](Drawing::parameters) are the sliders, and a preset needs no bytes from the caller
    /// (a font it uses is a bundled one).
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `name` if no preset has it (the message lists the names).
    pub fn preset(name: &str) -> Result<Self, Error> {
        let Some((_, text)) = PRESETS.iter().find(|(own, _)| *own == name) else {
            return Err(Error::parameter(
                "name",
                format!("is {name:?}, but the presets are {}", Self::preset_names().join(", ")),
            ));
        };
        Self::from_json(text, &Inputs::new())
    }

    /// The names of the presets, sorted: what [`Drawing::preset`] takes.
    #[must_use]
    pub fn preset_names() -> Vec<&'static str> {
        PRESETS.iter().map(|(name, _)| *name).collect()
    }
}
