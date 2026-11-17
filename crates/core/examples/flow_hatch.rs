//! Flow hatch: the drawing of `examples/flow_hatch.py` in Rust, a headline filled with evenly spaced lines that follow
//! its letters, close together at an edge and wider apart in the middle of a stroke.
//!
//! `cargo run -p bullterrier-vector --example flow_hatch` prints the size of the SVG of the frame (still);
//! `... --example flow_hatch -- still` writes the SVG to standard output.

use bullterrier_vector::{
    Align, Color, Drawing, Error, Field, Flow, Font, Hatch, Path, Size, Stroke, Text, Values, text,
};
use std::io::Write;

const WIDTH: f64 = 1080.0;
const HEIGHT: f64 = 560.0;

fn flow_hatch() -> Result<Drawing, Error> {
    let font = Font::bundled("bricolage_grotesque")?;
    let word = text(
        "FLOW",
        Text::new(&font)
            .size(330.0)
            .axes([("opsz", 96.0), ("wdth", 75.0), ("wght", 800.0)])
            .anchor((540.0, 420.0))
            .align(Align::Center),
    )
    .resolve(0.01);

    // every letter as one region: the union of the paths of the shapes the word draws
    let mut outline = Path::new();
    for shape in word.shapes(&Values::new())? {
        outline = outline.union(shape.path(), 0.01)?;
    }
    let flow = Flow::contour(&outline)?.mix(&Flow::noise(420.0, 0.0, 9.0)?, 0.3)?;
    let tone = Field::distance(&outline, true)?;

    let paper = Path::rect(0.0, 0.0, WIDTH, HEIGHT, 0.0)?.fill(Color::hex(0xf3_ee_e4));
    let lines = word
        .hatch(&Hatch::between(3.0, 9.0, tone).flow(flow), None)
        .stroke(Stroke::new(Color::hex(0x16_15_1a)).width(1.4));
    Ok(Drawing::new([paper, lines]).with_size(WIDTH, HEIGHT))
}

fn svg(drawing: &Drawing) -> Result<String, Error> {
    drawing.to_svg(Size::px(WIDTH, HEIGHT), &Values::new())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let drawing = flow_hatch()?;
    let mut out = std::io::stdout();
    match std::env::args().nth(1).as_deref() {
        Some("still") => out.write_all(svg(&drawing)?.as_bytes())?,
        Some(name) => return Err(format!("expected still, got {name}").into()),
        None => writeln!(out, "still: svg {} bytes", svg(&drawing)?.len())?,
    }
    Ok(())
}
