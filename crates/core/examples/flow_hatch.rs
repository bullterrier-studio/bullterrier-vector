//! Flow hatch: the drawing of `examples/flow_hatch.py` in Rust, a headline filled with evenly spaced lines that follow
//! its letters, close together at an edge and wider apart in the middle of a stroke.
//!
//! `cargo run -p bullterrier-vector --example flow_hatch` prints the size of the SVG of each frame (still, light);
//! `... --example flow_hatch -- still` writes the SVG of one to standard output.

use bullterrier_vector::{
    Align, Color, Drawing, Error, Expr, Field, Flow, Font, Hatch, Param, Path, Size, Stroke, Text, Values, text,
};
use std::io::Write;

const WIDTH: f64 = 1080.0;
const HEIGHT: f64 = 560.0;

fn flow_hatch() -> Result<Drawing, Error> {
    let font = Font::bundled("bricolage_grotesque")?;
    let weight = Param::new("weight", 800.0, 300.0, 800.0)?.meaning("the weight of the letters");
    let word = text(
        "FLOW",
        Text::new(&font)
            .size(330.0)
            .axes([
                ("opsz", Expr::from(96.0)),
                ("wdth", Expr::from(75.0)),
                ("wght", Expr::from(&weight)),
            ])
            .anchor((540.0, 420.0))
            .align(Align::Center),
    )
    .resolve(0.01);

    // the field and the flow read the headline as a drawing: at the weight a draw gives it
    let flow = Flow::contour(&word)?.mix(&Flow::noise(420.0, 0.0, 9.0)?, 0.3)?;
    let tone = Field::distance(&word, true)?;

    let paper = Path::rect(0.0, 0.0, WIDTH, HEIGHT, 0.0)?.fill(Color::hex(0xf3_ee_e4));
    let lines = word
        .hatch(&Hatch::between(3.0, 9.0, tone).flow(flow), None)
        .stroke(Stroke::new(Color::hex(0x16_15_1a)).width(1.4));
    Ok(Drawing::new([paper, lines]).with_size(WIDTH, HEIGHT))
}

/// The SVG of the frame `name`: the headline at its weight, or at 300.
fn svg(drawing: &Drawing, name: &str) -> Result<String, Error> {
    let values = if name == "light" {
        Values::new().set("weight", 300.0)
    } else {
        Values::new()
    };
    drawing.to_svg(Size::px(WIDTH, HEIGHT), &values)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let drawing = flow_hatch()?;
    let mut out = std::io::stdout();
    match std::env::args().nth(1).as_deref() {
        Some(name @ ("still" | "light")) => out.write_all(svg(&drawing, name)?.as_bytes())?,
        Some(name) => return Err(format!("expected still or light, got {name}").into()),
        None => {
            for name in ["still", "light"] {
                writeln!(out, "{name}: svg {} bytes", svg(&drawing, name)?.len())?;
            }
        }
    }
    Ok(())
}
