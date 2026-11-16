//! Glyph morph: the drawing of `examples/glyph_morph.py` in Rust, a letter that turns into a circle and one word that
//! turns into another.
//!
//! `cargo run -p bullterrier-vector --example glyph_morph` prints the size of the SVG of each of three frames (start,
//! half, end); `... --example glyph_morph -- half` writes the SVG of that frame to standard output.

use bullterrier_vector::{Align, Color, Drawing, Error, Font, Param, Path, Size, Stagger, Text, Unit, Values, text};
use std::io::Write;

const WIDTH: f64 = 1080.0;
const HEIGHT: f64 = 560.0;
/// The frames, as (name, progress): the same three as the Python and JavaScript programs.
const FRAMES: [(&str, f64); 3] = [("start", 0.0), ("half", 0.5), ("end", 1.0)];

fn word(content: &str, font: &Font) -> Drawing {
    let glyphs = text(
        content,
        Text::new(font)
            .size(190.0)
            .axes([("opsz", 96.0), ("wdth", 75.0), ("wght", 800.0)])
            .anchor((570.0, 350.0)),
    );
    glyphs.resolve(0.01) // one outline for each glyph: an E made of four bars would grow as four bars
}

fn glyph_morph() -> Result<Drawing, Error> {
    let font = Font::bundled("bricolage_grotesque")?;
    let progress = Param::new("progress", 0.0, 0.0, 1.0)?
        .unit(Unit::Fraction)?
        .meaning("how far the shapes have turned");

    let letter = text(
        "R",
        Text::new(&font)
            .size(440.0)
            .axes([("opsz", 96.0), ("wght", 800.0)])
            .anchor((270.0, 440.0))
            .align(Align::Center),
    );
    let circle = Path::circle((270.0, 285.0), 150.0)?;

    let paper = Path::rect(0.0, 0.0, WIDTH, HEIGHT, 0.0)?.fill(Color::hex(0xf3_ee_e4));
    let turned_word = word("CAT", &font).morph(word("HORSE", &font), Stagger::new(&progress));
    Ok(Drawing::new([
        paper,
        letter.morph(circle, &progress).fill(Color::hex(0x16_15_1a)),
        turned_word.fill(Color::hex(0xd4_45_2b)),
    ])
    .with_size(WIDTH, HEIGHT))
}

fn svg(drawing: &Drawing, progress: f64) -> Result<String, Error> {
    let values = Values::new().set("progress", progress);
    drawing.to_svg(Size::px(WIDTH, HEIGHT), &values)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let drawing = glyph_morph()?;
    let mut out = std::io::stdout();
    match std::env::args().nth(1).as_deref() {
        Some(name) => {
            let (_, progress) = FRAMES
                .iter()
                .find(|frame| frame.0 == name)
                .ok_or_else(|| format!("expected start, half or end, got {name}"))?;
            out.write_all(svg(&drawing, *progress)?.as_bytes())?;
        }
        None => {
            for (name, progress) in FRAMES {
                writeln!(out, "{name}: svg {} bytes", svg(&drawing, progress)?.len())?;
            }
        }
    }
    Ok(())
}
