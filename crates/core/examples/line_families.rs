//! Line families: the drawing of `examples/line_families.py` in Rust, three hills drawn only from lines whose look follows
//! a tone: ridgelines, rings whose width falls towards the middle, and stippled dots.
//!
//! `cargo run -p bullterrier-vector --example line_families` prints the size of the SVG of each frame (still, plot);
//! `... --example line_families -- still` writes the SVG of one to standard output.

use bullterrier_vector::{
    Bands, Color, Drawing, Error, Family, Field, Path, Plot, Rows, Size, Stipple, Stroke, Values,
};
use std::io::Write;

const WIDTH: f64 = 1080.0;
const HEIGHT: f64 = 420.0;

fn line_families() -> Result<Drawing, Error> {
    let hill = |i: f64| Path::circle((180.0 + 360.0 * i, 215.0), 160.0);
    let tone = |i: f64| Field::distance(hill(i)?, true);
    let ink = Color::hex(0x16_15_1a);
    Ok(Drawing::new([
        Drawing::from(hill(0.0)?)
            .rows(&Rows::new(8.0, tone(0.0)?, 70.0)?)
            .stroke(Stroke::new(ink).width(1.2)),
        Drawing::from(hill(1.0)?)
            .bands(&Bands::new(8.0, tone(1.0)?, (5.0, 0.6))?.family(Family::Rings))
            .fill(ink),
        Drawing::from(hill(2.0)?)
            .stipple(&Stipple::new(6.0, tone(2.0)?, (1.6, 1.6))?.seed(2.0))
            .fill(Color::hex(0xc8_34_1c)),
    ])
    .with_size(WIDTH, HEIGHT))
}

/// The SVG of the frame `name`: the picture, or what a pen plotter reads (a pen 0.8 pixels wide).
fn svg(drawing: &Drawing, name: &str) -> Result<String, Error> {
    let size = Size::px(WIDTH, HEIGHT);
    if name == "plot" {
        drawing.to_plot_svg(size, &Values::new(), &Plot::new(0.8)?)
    } else {
        drawing.to_svg(size, &Values::new())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let drawing = line_families()?;
    let mut out = std::io::stdout();
    match std::env::args().nth(1).as_deref() {
        Some(name @ ("still" | "plot")) => out.write_all(svg(&drawing, name)?.as_bytes())?,
        Some(name) => return Err(format!("expected still or plot, got {name}").into()),
        None => {
            for name in ["still", "plot"] {
                writeln!(out, "{name}: svg {} bytes", svg(&drawing, name)?.len())?;
            }
        }
    }
    Ok(())
}
