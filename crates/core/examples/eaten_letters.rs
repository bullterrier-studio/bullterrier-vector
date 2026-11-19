//! Eaten letters: the drawing of `examples/eaten_letters.py` in Rust, a word eaten at the rim and grown over by a rust
//! and a pale halo: the signed distance to the letters bent by seeded noise, traced at three distances.
//!
//! `cargo run -p bullterrier-vector --example eaten_letters` prints the size of the SVG of each frame (still, fine);
//! `... --example eaten_letters -- still` writes the SVG of one to standard output.

use bullterrier_vector::{
    Align, Color, Drawing, Error, Expr, Field, Font, Param, Path, Rect, Select, Size, Text, Trace, Unit, Values, text,
};
use std::io::Write;

const WIDTH: f64 = 1200.0;
const HEIGHT: f64 = 600.0;

fn number(value: f64) -> Result<Field, Error> {
    Field::constant(value)
}

fn eaten_letters() -> Result<Drawing, Error> {
    let font = Font::bundled("fraunces")?;
    let word = text(
        "SPORE",
        Text::new(&font)
            .size(300.0)
            .axes([
                ("wght", Expr::from(900.0)),
                ("opsz", Expr::from(144.0)),
                ("WONK", Expr::from(0.0)),
            ])
            .anchor((600.0, 395.0))
            .align(Align::Center),
    )
    .resolve(0.01);
    let wavelength = Param::new("wavelength", 45.0, 10.0, 120.0)?
        .unit(Unit::Px)?
        .meaning("the size of a bite");
    let core = Param::new("core", 0.0, -20.0, 20.0)?
        .unit(Unit::Px)?
        .meaning("how far the letter is eaten in");

    let edge = Field::distance(&word, false)?;
    let bite = Field::noise(&wavelength, 0.0, 3.0, 3)?;
    // the noise grows from nothing on the edge to its full 40 pixels at 60 pixels from it
    let grow = edge.abs()?.min(&number(60.0)?)?.mul(&number(1.0 / 60.0)?)?;
    let eaten = edge.add(&bite.mul(&number(40.0)?)?.mul(&grow)?)?;

    // trace_levels keeps where a field is at or above a level, so the letters are where minus the eaten distance is; the
    // three levels are read in one pass, and each shape, the region of one level in order, is painted by its index
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        width: WIDTH,
        height: HEIGHT,
    };
    let levels = [Expr::from(-40.0), Expr::from(-18.0), -Expr::from(&core)];
    let tidy = Trace::new().smooth(1.5).tolerance(0.4).min_area(20.0);
    let traced = Drawing::trace_levels(&eaten.mul(&number(-1.0)?)?, levels, Some(frame), Some(2.0), &tidy);
    let mut items = vec![Path::rect(0.0, 0.0, WIDTH, HEIGHT, 0.0)?.fill(Color::hex(0xef_e8_d6))];
    for (index, color) in [(0_u32, 0xdc_d2_b6), (1, 0xc8_42_1e), (2, 0x16_21_1c)] {
        items.push(
            traced
                .only(Select::new().attribute("index", index))
                .fill(Color::hex(color)),
        );
    }
    Ok(Drawing::new(items).with_size(WIDTH, HEIGHT))
}

/// The SVG of the frame `name`: the noise at its defaults, or finer with the core eaten 6 pixels in.
fn svg(drawing: &Drawing, name: &str) -> Result<String, Error> {
    let values = if name == "fine" {
        Values::new().set("wavelength", 30.0).set("core", -6.0)
    } else {
        Values::new()
    };
    drawing.to_svg(Size::px(WIDTH, HEIGHT), &values)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let drawing = eaten_letters()?;
    let mut out = std::io::stdout();
    match std::env::args().nth(1).as_deref() {
        Some(name @ ("still" | "fine")) => out.write_all(svg(&drawing, name)?.as_bytes())?,
        Some(name) => return Err(format!("expected still or fine, got {name}").into()),
        None => {
            for name in ["still", "fine"] {
                writeln!(out, "{name}: svg {} bytes", svg(&drawing, name)?.len())?;
            }
        }
    }
    Ok(())
}
