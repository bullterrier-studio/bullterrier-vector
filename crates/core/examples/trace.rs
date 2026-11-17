//! Trace: the drawing of `examples/trace.py` in Rust, a relief traced into stacked cut-paper layers and a dithered strip
//! traced exactly.
//!
//! `cargo run -p bullterrier-vector --example trace` prints the size of the SVG of the frame (still);
//! `... --example trace -- still` writes the SVG to standard output.

use bullterrier_vector::{Color, Drawing, Error, Field, Rect, Select, Size, Trace, Values};
use std::io::Write;

const WIDTH: f64 = 960.0;
const HEIGHT: f64 = 1180.0;
const SIDE: usize = 96;
const COLUMNS: usize = 80;
const ROWS: usize = 12;
const BAYER: [[usize; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

fn trace() -> Result<Drawing, Error> {
    // whole numbers only, so the bytes equal the Python and JavaScript programs'
    let relief: Vec<u8> = (0..SIDE * SIDE)
        .map(|i| {
            let (x, y) = ((i % SIDE) as i64, (i / SIDE) as i64);
            let value = 0
                .max(255 - ((x - 34).pow(2) + (y - 40).pow(2)) / 6)
                .max(200 - ((x - 68).pow(2) + (y - 56).pow(2)) / 5);
            u8::try_from(value).unwrap_or(255)
        })
        .collect();
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        width: 960.0,
        height: 960.0,
    };
    let relief = Field::map(SIDE, SIDE, &relief, Some(frame), Some("relief"))?;
    let dithered: Vec<u8> = (0..COLUMNS * 2 * ROWS * 2)
        .map(|i| {
            let (x, y) = ((i % (COLUMNS * 2)) / 2, (i / (COLUMNS * 2)) / 2);
            u8::from(BAYER[y % 4][x % 4] * 16 + 8 < x * 255 / (COLUMNS - 1))
        })
        .collect();
    let frame = Rect {
        x: 60.0,
        y: 1000.0,
        width: 840.0,
        height: 126.0,
    };
    let strip = Field::map(COLUMNS * 2, ROWS * 2, &dithered, Some(frame), Some("dither"))?;

    let papers = [0xe9_dc_c3, 0xc9_a6_78, 0x8a_5a_3b];
    let tidy = Trace::new().smooth(12.0).tolerance(3.0).close(8.0).min_area(600.0);
    let mut items: Vec<Drawing> = [0.2, 0.45, 0.7]
        .iter()
        .zip(papers)
        .map(|(level, paper)| Drawing::trace_levels(&relief, &[*level], &tidy).fill(Color::hex(paper)))
        .collect();
    let dither = Drawing::trace_cells(&strip, 2, 2);
    items.push(
        dither
            .only(Select::new().attribute("level", 1))
            .fill(Color::hex(0xc8_34_1c)),
    );
    items.push(
        dither
            .only(Select::new().attribute("level", 0))
            .fill(Color::hex(0x16_15_1a)),
    );
    Ok(Drawing::new(items).with_size(WIDTH, HEIGHT))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let drawing = trace()?;
    let svg = drawing.to_svg(Size::px(WIDTH, HEIGHT), &Values::new())?;
    let mut out = std::io::stdout();
    match std::env::args().nth(1).as_deref() {
        Some("still") => out.write_all(svg.as_bytes())?,
        Some(name) => return Err(format!("expected still, got {name}").into()),
        None => writeln!(out, "still: svg {} bytes", svg.len())?,
    }
    Ok(())
}
