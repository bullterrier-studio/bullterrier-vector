//! The drawings that come with the library, by name: JSON documents (formats.md) that `Drawing::from_json` reads. The
//! table is the one owner of the names and descriptions; each text is the canonical `to_json` of a program in
//! `examples/presets/` (written by `tools/write_presets.py`).

use crate::{Drawing, Error, Inputs};

/// The names, one-sentence descriptions and texts, sorted by name.
const PRESETS: [(&str, &str, &str); 13] = [
    (
        "bent_type",
        "A line of heavy type that arches, ripples, twists and shakes as it bends.",
        include_str!("../presets/bent_type.json"),
    ),
    (
        "brush_taper",
        "A heavy serif word with a red brush stroke beneath it that swells from nothing and thins to a point.",
        include_str!("../presets/brush_taper.json"),
    ),
    (
        "draw_on",
        "A word set in outline, with a pink shadow, that draws itself on letter by letter.",
        include_str!("../presets/draw_on.json"),
    ),
    (
        "flow_hatch",
        "A headline filled with fine lines that follow the letters, crowding at the edges and opening in the middle.",
        include_str!("../presets/flow_hatch.json"),
    ),
    (
        "glyph_morph",
        "A letter that turns into a circle while one word turns into another.",
        include_str!("../presets/glyph_morph.json"),
    ),
    (
        "glyph_scatter",
        "A heavy word whose letters are broken apart, tipped and bounced by seeded random amounts.",
        include_str!("../presets/glyph_scatter.json"),
    ),
    (
        "gradient_type",
        "A headline filled with one rainbow gradient across its letters, with a glowing ring behind it.",
        include_str!("../presets/gradient_type.json"),
    ),
    (
        "halftone_screen",
        "A headline printed as a red dot screen, solid at its foot and fading to bare paper at its top.",
        include_str!("../presets/halftone_screen.json"),
    ),
    (
        "marching_dashes",
        "Four rings of dashes, black and red, that march round in turn on one slider.",
        include_str!("../presets/marching_dashes.json"),
    ),
    (
        "ring",
        "A near-black ring whose width is a slider.",
        include_str!("../presets/ring.json"),
    ),
    (
        "stipple",
        "A ball shaded only by dots, dense in its shadow and bare where the light falls.",
        include_str!("../presets/stipple.json"),
    ),
    (
        "two_fonts",
        "A serif headline over a heavy sans line, both fonts bundled, with a weight slider.",
        include_str!("../presets/two_fonts.json"),
    ),
    (
        "waves",
        "A headline made only of wavy lines, fat at the edge of each letter and thin at its heart.",
        include_str!("../presets/waves.json"),
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
