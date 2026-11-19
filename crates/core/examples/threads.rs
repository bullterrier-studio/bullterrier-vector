//! Threads: the drawing of `examples/threads.py` in Rust, forking threads grown out of the edges of a heavy letter,
//! tapered and drawn on from their roots.
//!
//! `cargo run -p bullterrier-vector --example threads` prints the size of the SVG of each frame (grown, half);
//! `... --example threads -- half` writes the SVG of one to standard output.

use bullterrier_vector::{
    Align, Branch, Color, Deg, Drawing, Error, Expr, Font, Outline, Param, Path, Select, Size, Slice, Text, Values,
    text,
};
use std::io::Write;

const WIDTH: f64 = 800.0;
const HEIGHT: f64 = 800.0;
const TAPER: [(f64, f64); 2] = [(0.0, 1.0), (1.0, 0.15)];

fn threads() -> Result<Drawing, Error> {
    let font = Font::bundled("fraunces")?;
    let letter = text(
        "R",
        Text::new(&font)
            .size(560.0)
            .axes([("wght", Expr::from(900.0))])
            .anchor((400.0, 600.0))
            .align(Align::Center),
    )
    .resolve(0.01);
    let growth = Param::new("growth", 1.0, 0.0, 1.0)?.meaning("how far the threads have grown");
    let threads = letter.branch(
        &Branch::new(70.0, 220.0)
            .fork_spacing(60.0)
            .forks(2)
            .shorter(0.5)
            .angle(Deg(40.0))
            .wander(Deg(35.0))
            .seed(7.0),
    );
    let trunk = || Select::new().attribute("forks", 0_u32);
    let forked = threads
        .without(trunk())
        .slice(0.0, &growth, Slice::new())
        .outline(3.0, Outline::new().profile(TAPER));
    let trunks = threads
        .only(trunk())
        .slice(0.0, &growth, Slice::new())
        .outline(6.0, Outline::new().profile(TAPER));
    Ok(Drawing::new([
        Path::rect(0.0, 0.0, WIDTH, HEIGHT, 0.0)?.fill(Color::hex(0xf2_c2_30)),
        Drawing::new([letter, forked, trunks]).fill(Color::hex(0x14_14_14)),
    ])
    .with_size(WIDTH, HEIGHT))
}

/// The SVG of the frame `name`: the threads grown, or halfway.
fn svg(drawing: &Drawing, name: &str) -> Result<String, Error> {
    let values = if name == "half" {
        Values::new().set("growth", 0.5)
    } else {
        Values::new()
    };
    drawing.to_svg(Size::px(WIDTH, HEIGHT), &values)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let drawing = threads()?;
    let mut out = std::io::stdout();
    match std::env::args().nth(1).as_deref() {
        Some(name @ ("grown" | "half")) => out.write_all(svg(&drawing, name)?.as_bytes())?,
        Some(name) => return Err(format!("expected grown or half, got {name}").into()),
        None => {
            for name in ["grown", "half"] {
                writeln!(out, "{name}: svg {} bytes", svg(&drawing, name)?.len())?;
            }
        }
    }
    Ok(())
}
