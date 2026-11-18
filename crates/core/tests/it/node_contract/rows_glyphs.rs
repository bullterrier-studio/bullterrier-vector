//! The rows of the verbs that stand glyphs on a lattice or beside a mark: `characters` and `marks`.

use super::rows::{Row, unknown};
use bullterrier_vector::{BoxMark, Characters, Drawing, Marks, PointMark, characters, marks};

pub(super) fn rows() -> Vec<Row> {
    vec![
        Row {
            verb: "characters",
            sample: || Ok(characters("#@\n.#", Characters::new((6.0, 12.0)))),
            with: Some(|argument, value| match argument {
                "size" => Ok(characters("#@\n.#", Characters::new((6.0, 12.0)).size(value))),
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
        },
        Row {
            verb: "marks",
            sample: || Ok(sample(Marks::new())),
            with: Some(|argument, value| {
                let options = Marks::new();
                match argument {
                    "bracket" => Ok(sample(options.bracket(value))),
                    "cross" => Ok(sample(options.cross(value))),
                    "text_size" => Ok(sample(options.text_size(value))),
                    _ => Err(unknown("marks", argument)),
                }
            }),
            ranges: &[
                ("bracket", 0.5, true),
                ("bracket", 0.01, true),
                ("bracket", 0.0, false),
                ("bracket", 0.51, false),
                ("bracket", f64::NAN, false),
                ("cross", 1.0, true),
                ("cross", 0.0, false),
                ("cross", -1.0, false),
                ("cross", f64::INFINITY, false),
                ("text_size", 1.0, true),
                ("text_size", 0.0, false),
                ("text_size", f64::NAN, false),
            ],
            geometry: None,
        },
    ]
}

/// A box and a point, each labelled, with `options`.
fn sample(options: Marks) -> Drawing {
    marks(
        options
            .boxes([BoxMark::new(10.0, 10.0, 40.0, 30.0).text("A")])
            .points([PointMark::new(80.0, 20.0).text("B")]),
    )
}
