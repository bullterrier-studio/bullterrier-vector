//! Documents and drawings through JSON and back: the examples of docs/examples/json whose calls are all registered
//! load, and write the canonical text; a drawing written to JSON and read again draws the same shapes and writes the
//! same text.

mod common;

use bullterrier_vector::{Drawing, Values};
use common::Outcome;

const SERIES_BARS: &str = include_str!("../../../docs/examples/json/series_bars.json");
/// `series_bars.json` in canonical form, made by Python's `json` module (sorted keys, compact) with the one shared
/// drawing, `bars`, named `d1`: an oracle that shares no code with the writer.
const SERIES_BARS_CANONICAL: &str = include_str!("golden/series_bars.canonical.json");

#[test]
fn the_series_bars_example_loads_and_writes_its_canonical_text() -> Outcome {
    let drawing = Drawing::from_json(SERIES_BARS)?;
    assert_eq!(drawing.to_json()?, SERIES_BARS_CANONICAL.trim_end());
    Ok(())
}

#[test]
fn the_series_bars_example_draws_the_same_shapes_after_a_round_trip() -> Outcome {
    let loaded = Drawing::from_json(SERIES_BARS)?;
    let again = Drawing::from_json(&loaded.to_json()?)?;
    let shapes = loaded.shapes(&Values::new())?;
    assert_eq!(shapes, again.shapes(&Values::new())?);
    // twelve bars: the sixth in red, the others in ink
    assert_eq!(shapes.len(), 12);
    Ok(())
}
