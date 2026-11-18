//! Overlay: the drawing of `examples/overlay.py` in Rust, a character grid and the marks of a heads-up display on it.
//!
//! `cargo run -p bullterrier-vector --example overlay` prints the size of the SVG of the frame (still);
//! `... --example overlay -- still` writes the SVG to standard output.

use bullterrier_vector::{
    BoxMark, Characters, Color, Drawing, Error, Font, Marks, PointMark, Select, Size, Stroke, Values, characters, marks,
};
use std::io::Write;

const COLUMNS: i64 = 36;
const ROWS: i64 = 18;
const CELL: (f64, f64) = (12.0, 20.0);
const WIDTH: f64 = 432.0;
const HEIGHT: f64 = 360.0;
const RAMP: &[u8] = b" .:-=+*#%@";
const RADIUS_SQUARED: i64 = 170 * 170;

/// The character of a cell, from whole numbers only, so the rows equal the Python and JavaScript programs'.
fn tone(column: i64, row: i64) -> char {
    let across = 6 * (2 * column + 1 - COLUMNS);
    let down = 10 * (2 * row + 1 - ROWS);
    let shade = (9 - (across * across + down * down) * 9 / RADIUS_SQUARED).max(0);
    RAMP.get(usize::try_from(shade).unwrap_or(0))
        .map_or(' ', |byte| char::from(*byte))
}

fn overlay() -> Result<Drawing, Error> {
    let rows = (0..ROWS)
        .map(|row| (0..COLUMNS).map(|column| tone(column, row)).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    let font = Font::bundled("jetbrains_mono")?;
    let (ink, green) = (Color::hex(0x1f_2a_24), Color::hex(0x1c_9b_5a));
    let grid = characters(rows, Characters::new(&font, CELL)).fill(ink);
    let hud = marks(
        Marks::new(&font)
            .boxes([BoxMark::new(96.0, 60.0, 240.0, 240.0).label("DISC")])
            .points([PointMark::new(216.0, 180.0).label("CENTRE")])
            .bracket(0.2)
            .cross(10.0)
            .label_size(14.0),
    );
    let label = Select::new().attribute("role", "label");
    Ok(Drawing::new([
        grid,
        hud.only(label.clone()).fill(green),
        hud.without(label).stroke(Stroke::new(green).width(1.5)),
    ])
    .with_size(WIDTH, HEIGHT))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let drawing = overlay()?;
    let svg = drawing.to_svg(Size::px(WIDTH, HEIGHT), &Values::new())?;
    let mut out = std::io::stdout();
    match std::env::args().nth(1).as_deref() {
        Some("still") => out.write_all(svg.as_bytes())?,
        Some(name) => return Err(format!("expected still, got {name}").into()),
        None => writeln!(out, "still: svg {} bytes", svg.len())?,
    }
    Ok(())
}
