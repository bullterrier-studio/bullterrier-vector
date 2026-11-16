//! Bent type: the drawing of `examples/bent_type.py` in Rust, a line that arches, ripples, twists and wobbles.
//!
//! `cargo run -p bullterrier-vector --example bent_type` prints the size of the SVG of each of three frames (calm,
//! half, bent); `... --example bent_type -- bent` writes the SVG of that frame to standard output.

use bullterrier_vector::{Align, Color, Deg, Drawing, Error, Font, Param, Path, Size, Text, Twist, Unit, Values};
use std::io::Write;

const WIDTH: f64 = 1080.0;
const HEIGHT: f64 = 560.0;
/// The frames, as (name, progress, phase): the same three as the Python and JavaScript programs.
const FRAMES: [(&str, f64, f64); 3] = [("calm", 0.0, 0.0), ("half", 0.5, 0.125), ("bent", 1.0, 0.25)];

fn bent_type() -> Result<Drawing, Error> {
    let font = Font::bundled("bricolage_grotesque")?;
    let progress = Param::new("progress", 0.0, 0.0, 1.0)?
        .unit(Unit::Fraction)?
        .meaning("how far the line is bent");
    let phase = Param::new("phase", 0.0, 0.0, 1.0)?
        .unit(Unit::Cycle)?
        .meaning("where the ripple and the shaking are");

    let line = bullterrier_vector::text(
        "NO SLEEP",
        Text::new(&font)
            .size(210.0)
            .axes([("opsz", 96.0), ("wdth", 75.0), ("wght", 800.0)])
            .anchor((WIDTH / 2.0, 380.0))
            .align(Align::Center),
    );
    let bent = line
        .arch(0.25 * &progress)
        .wave(10.0 * &progress, 240.0, Deg(0.0), &phase)
        .twist(
            Twist::new(Deg(20.0 * &progress))
                .about((WIDTH / 2.0, 300.0))
                .radius(520.0),
        )
        .wobble(2.0 * &progress, 40.0, &phase, 7.0);
    let paper = Path::rect(0.0, 0.0, WIDTH, HEIGHT, 0.0)?.fill(Color::hex(0xf3_ee_e4));
    Ok(Drawing::new([paper, bent.fill(Color::hex(0x16_15_1a))]).with_size(WIDTH, HEIGHT))
}

fn svg(drawing: &Drawing, progress: f64, phase: f64) -> Result<String, Error> {
    let values = Values::new().set("progress", progress).set("phase", phase);
    drawing.to_svg(Size::px(WIDTH, HEIGHT), &values)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let drawing = bent_type()?;
    let mut out = std::io::stdout();
    match std::env::args().nth(1).as_deref() {
        Some(name) => {
            let (_, progress, phase) = FRAMES
                .iter()
                .find(|frame| frame.0 == name)
                .ok_or_else(|| format!("expected calm, half or bent, got {name}"))?;
            out.write_all(svg(&drawing, *progress, *phase)?.as_bytes())?;
        }
        None => {
            for (name, progress, phase) in FRAMES {
                writeln!(out, "{name}: svg {} bytes", svg(&drawing, progress, phase)?.len())?;
            }
        }
    }
    Ok(())
}
