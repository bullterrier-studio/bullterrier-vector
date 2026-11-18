//! The rows of the verbs that stand glyphs on a lattice or beside a mark: `characters` and `marks`.

use super::rows::{Row, unknown};
use bullterrier_vector::{Characters, Font, characters};

fn mono() -> Result<Font, bullterrier_vector::Error> {
    Font::bundled("jetbrains_mono")
}

pub(super) fn rows() -> Vec<Row> {
    vec![Row {
        verb: "characters",
        sample: || Ok(characters("#@\n.#", Characters::new(&mono()?, (6.0, 12.0)))),
        with: Some(|argument, value| match argument {
            "size" => Ok(characters("#@\n.#", Characters::new(&mono()?, (6.0, 12.0)).size(value))),
            _ => Err(unknown("characters", argument)),
        }),
        ranges: &[
            ("size", 1.0, true),
            ("size", 100_000.0, true),
            ("size", 0.0, false),
            ("size", -1.0, false),
            ("size", f64::NAN, false),
            ("size", f64::INFINITY, false),
        ],
        geometry: None,
    }]
}
