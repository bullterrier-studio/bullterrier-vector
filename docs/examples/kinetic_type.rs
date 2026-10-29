//! Kinetic type in Rust: the same program as `kinetic_type.py`, writing a PNG sequence.
//!
//! Two lines of condensed type in a variable font start as hairlines, a blue plane rises with the letters cut out of
//! it, the drop bursts them into a two-ink riso print misregistered more on each hit, the letters pulse one after
//! another, and the print thins back to hairlines. Every number that changes is a `Param`; the loop at the bottom is
//! the caller's, and `pulse` is passed as one value per letter.

use std::error::Error;
use std::fs;

use btvec::{Align, Blend, Color, Deg, Drawing, Font, Param, Path, Size, Stroke, Text, Unit, Values, ease};
use bullterrier_vector as btvec;

const WIDTH: f64 = 1080.0;
const HEIGHT: f64 = 1350.0;
const PAPER: Color = Color::hex(0xf3eee4);
const INK: Color = Color::hex(0x16151a);
const PINK: Color = Color::hex(0xff4f9a);
const BLUE: Color = Color::hex(0x2a5bff);

fn main() -> Result<(), Box<dyn Error>> {
    let font = Font::new(fs::read("fonts/Anybody[wdth,wght].ttf")?)?;

    let weight = Param::new("weight", 600.0, 100.0..=780.0)?.meaning("how heavy the letters are, before a pulse");
    let rise = Param::new("rise", 0.0, 0.0..=1.0)?
        .unit(Unit::Fraction)
        .meaning("how far the blue plane has risen");
    let ink = Param::new("ink", 0.0, 0.0..=1.0)?
        .unit(Unit::Fraction)
        .meaning("0: hairlines only; 1: the riso print");
    let burst = Param::new("burst", 0.0, 0.0..=1.0)?
        .unit(Unit::Fraction)
        .meaning("how hard the last hit was");
    let pulse = Param::new("pulse", 0.0, 0.0..=1.0)?
        .unit(Unit::Fraction)
        .meaning("a kick, per letter");

    // One shape per letter; each line fitted to the width by the width axis once, at the calm pose.
    let letters = btvec::text(
        "NO\nSLEEP",
        Text::new(&font)
            .size(560.0)
            .axis("wght", &weight + 120.0 * &pulse)
            .leading(520.0)
            .at((WIDTH / 2.0, 600.0))
            .align(Align::Center)
            .width(980.0)
            .fit("wdth"),
    )
    .scale(1.0 + 0.08 * &pulse)
    .rotate(Deg(btvec::random(-10.0, 10.0, 7) * &burst))
    .translate(
        btvec::random(-30.0, 30.0, 8) * &burst,
        btvec::random(-50.0, 50.0, 9) * &burst,
    );

    let frame = Path::rect(0.0, 0.0, WIDTH, HEIGHT);
    let misregistration = 3.0 + 14.0 * &burst; // a constant offset looks like a mistake; it grows on a hit

    let paper = frame.fill(PAPER);
    let plane = Drawing::from(frame).translate(0.0, HEIGHT * (1.0 - &rise));
    let blue = plane
        .subtract(&letters.translate(misregistration.clone(), 0.6 * misregistration))
        .fill(BLUE)
        .blend(Blend::Multiply);
    let pink = letters.fill(PINK).blend(Blend::Multiply).opacity(&ink);
    let hairlines = letters.stroke(Stroke::new(INK).width(1.5)).opacity(1.0 - &ink);

    let piece = Drawing::new([paper, blue, pink, hairlines]).with_size(WIDTH, HEIGHT);
    let count = letters.shapes(&Values::new())?.len(); // 7: a space or a line break gives no shape

    fs::create_dir_all("frames")?;
    for n in 0..12 * 30 {
        let t = f64::from(n) / 30.0;
        let png = piece.to_png(Size::px(1080, 1350), &values_at(t, count))?;
        fs::write(format!("frames/no-sleep-{n:04}.png"), png)?;
    }
    Ok(())
}

/// A kick on every beat at 120 bpm: 1 on the beat, falling to 0 within 0.3 s.
fn kick(t: f64) -> f64 {
    1.0 - ease::out_cubic((t % 0.5) / 0.3)
}

/// The values `t` seconds in: hairlines, the plane rises from 4 s, the drop at 6 s, thinning out from 10 s.
fn values_at(t: f64, count: usize) -> Values {
    let printed = (6.0..11.0).contains(&t);
    let heavy = ease::out_back((t - 6.0) / 0.3) - ease::in_out_cubic((t - 10.0) / 1.5);
    let hit = |at: f64| if printed { kick(at) } else { 0.0 };
    Values::new()
        .set("rise", ease::in_out_cubic((t - 4.0) / 2.0))
        .set("ink", if printed { 1.0 } else { 0.0 })
        .set("weight", 100.0 + 500.0 * heavy)
        .set("burst", hit(t))
        .set(
            "pulse",
            (0..count).map(|i| hit(t - 0.045 * i as f64)).collect::<Vec<_>>(),
        )
}
