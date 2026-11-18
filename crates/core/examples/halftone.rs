//! A halftone: the drawing of `examples/halftone.py` in Rust, discs and panels filled with a screen of dots over paper:
//! a row of discs whose tone steps from nearly solid to nearly bare, four panels that are each a shape of dot on a screen
//! with a ramp for a tone, and a big disc with a stroke over its dots.
//!
//! `cargo run -p bullterrier-vector --example halftone` prints the size of the SVG of the frame (still);
//! `... --example halftone -- still` writes it to standard output.

use bullterrier_vector::{
    Color, Deg, Drawing, Error, Field, Halftone, Mark, Path, Screen, Size, Stroke, Values, by_index,
};
use std::io::Write;

const WIDTH: f64 = 640.0;
const HEIGHT: f64 = 520.0;

fn halftone() -> Result<Drawing, Error> {
    let ink = Color::hex(0x1a_1a_1a);
    let paper = Drawing::from(Path::rect(0.0, 0.0, WIDTH, HEIGHT, 0.0)?).fill(Color::hex(0xf4_ec_d8));

    // a row of discs whose tone steps from nearly solid to nearly bare
    let discs = (0..7)
        .map(|i| Path::circle((60.0 + 90.0 * f64::from(i), 60.0), 38.0))
        .collect::<Result<Vec<_>, _>>()?;
    let ramp = Drawing::new(discs).halftone(Halftone::new(ink, by_index(0.1, 0.9)).pitch(6.0));

    // four panels, each a shape of dot on a screen, the tone a ramp from solid to bare
    let triangle = Path::from_svg_path("M0 -1 L1 1 L-1 1 Z")?;
    let panel = |index: u8, (start, end): ((f64, f64), (f64, f64)), dots: &dyn Fn(Field) -> Halftone| {
        let (left, top) = (30.0 + 150.0 * f64::from(index), 130.0);
        let at = |(x, y): (f64, f64)| (left + 120.0 * x, top + 120.0 * y);
        let tone = Field::ramp(at(start), at(end))?;
        Ok::<_, Error>(Drawing::from(Path::rect(left, top, 120.0, 120.0, 0.0)?).halftone(dots(tone)))
    };
    let dots = |tone: Field| Halftone::field(ink, tone);
    let panels = [
        panel(0, ((0.0, 0.0), (1.0, 0.0)), &|tone| {
            dots(tone).shape(Mark::Ellipse).aspect(2.0).angle(Deg(20.0)).pitch(7.0)
        })?,
        panel(1, ((0.0, 0.0), (0.0, 1.0)), &|tone| {
            dots(tone).shape(Mark::Euclidean).screen(Screen::Rings).pitch(6.0)
        })?,
        panel(2, ((0.0, 0.0), (1.0, 1.0)), &|tone| {
            dots(tone)
                .shape(Mark::Diamond)
                .screen(Screen::Wave)
                .amplitude(1.0)
                .wavelength(6.0)
                .angle(Deg(0.0))
                .pitch(7.0)
        })?,
        panel(3, ((0.0, 1.0), (1.0, 0.0)), &|tone| {
            dots(tone)
                .shape(Mark::path(triangle.clone()))
                .screen(Screen::Rays)
                .pitch(8.0)
        })?,
    ];
    let mut drawings = vec![paper, ramp];
    drawings.extend(panels);
    // one big disc with a coarser screen turned another way, and a stroke over its dots
    let red = Color::hex(0xc4_37_2b);
    drawings.push(
        Drawing::from(Path::circle((320.0, 390.0), 100.0)?)
            .halftone(Halftone::new(red, 0.45).pitch(9.0).angle(Deg(15.0)))
            .stroke(Stroke::new(red).width(3.0)),
    );
    Ok(Drawing::new(drawings).with_size(WIDTH, HEIGHT))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let drawing = halftone()?;
    let svg = drawing.to_svg(Size::px(WIDTH, HEIGHT), &Values::new())?;
    let mut out = std::io::stdout();
    match std::env::args().nth(1).as_deref() {
        Some("still") => out.write_all(svg.as_bytes())?,
        Some(name) => return Err(format!("expected still, got {name}").into()),
        None => writeln!(out, "still: svg {} bytes", svg.len())?,
    }
    Ok(())
}
