//! The drawings that come with the library, by name: JSON documents (formats.md) that `Drawing::from_json` reads. The
//! table is the one owner of the names and descriptions; each text is the canonical `to_json` of a program in
//! `examples/presets/` (written by `tools/write_presets.py`).

use crate::{Drawing, Error, Inputs};

/// The names, one-sentence descriptions and texts, sorted by name.
const PRESETS: [(&str, &str, &str); 3] = [
    (
        "bent_type",
        "A line of heavy type that arches, ripples, twists and shakes as it bends.",
        include_str!("../presets/bent_type.json"),
    ),
    (
        "gradient_type",
        "A headline filled with one rainbow gradient across its letters, with a glowing ring behind it.",
        include_str!("../presets/gradient_type.json"),
    ),
    (
        "ring",
        "A near-black ring whose width is a slider.",
        include_str!("../presets/ring.json"),
    ),
];

/// The table row of `name`; the one place that says what an unknown name is refused with.
fn find(name: &str) -> Result<&'static (&'static str, &'static str, &'static str), Error> {
    PRESETS.iter().find(|(own, ..)| *own == name).ok_or_else(|| {
        Error::parameter(
            "name",
            format!(
                "is {name:?}, but the presets are {}",
                Drawing::preset_names().join(", ")
            ),
        )
    })
}

impl Drawing {
    /// A drawing that comes with the library, by name; [`Drawing::preset_names`] lists them. It is an ordinary
    /// drawing: its [`parameters`](Drawing::parameters) are the sliders, and a preset needs no bytes from the caller
    /// (a font it uses is a bundled one).
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `name` if no preset has it (the message lists the names).
    pub fn preset(name: &str) -> Result<Self, Error> {
        Self::from_json(find(name)?.2, &Inputs::new())
    }

    /// The one-sentence description of the preset `name`: what its look is.
    ///
    /// # Errors
    /// The same as [`Drawing::preset`].
    pub fn preset_description(name: &str) -> Result<&'static str, Error> {
        Ok(find(name)?.1)
    }

    /// The names of the presets, sorted: what [`Drawing::preset`] takes.
    #[must_use]
    pub fn preset_names() -> Vec<&'static str> {
        PRESETS.iter().map(|(name, ..)| *name).collect()
    }
}
