//! Kinetic type: the drawing of `examples/kinetic_type.py` in Rust, NO SLEEP turning into GO DREAM in two riso inks.
//!
//! `cargo run -p bullterrier-vector --example kinetic_type` prints the size of the SVG of each of four frames
//! (hairlines, kick, bend, morph); `... --example kinetic_type -- bend` writes the SVG of that frame to standard
//! output. The easing and the beat are the caller's, so they are here, in plain Rust: `values_at(t)`.

use bullterrier_vector::{
    Align, Arch, Blend, Color, Deg, Drawing, Error, Expr, Font, Param, Path, Random, Size, Stagger, Stroke, Text, Unit,
    Values, text,
};
use std::io::Write;

const WIDTH: f64 = 1080.0;
const HEIGHT: f64 = 1350.0;
const MEASURE: f64 = 980.0; // the width each line is stretched to fill
const BEAT: f64 = 0.5; // seconds: 120 bpm
const CHARACTERS: u32 = 8; // "NO\nSLEEP": `pulse` has an entry for each, the line break too
/// The frames, as (name, seconds): the same four as the Python and JavaScript programs.
const FRAMES: [(&str, f64); 4] = [("hairlines", 1.0), ("kick", 4.14), ("bend", 6.5), ("morph", 8.8)];

/// A Param from 0 to 1, a fraction, that is 0 unless a frame says otherwise.
fn fraction(name: &str, meaning: &str) -> Result<Param, Error> {
    Ok(Param::new(name, 0.0, 0.0, 1.0)?.unit(Unit::Fraction)?.meaning(meaning))
}

/// `content` set in the common layout: each line stretched to `MEASURE` by the width axis, once, at the calm pose.
fn headline(content: &str, wght: impl Into<Expr>, font: &Font) -> Drawing {
    text(
        content,
        Text::new(font)
            .size(800.0)
            .axes([("wght", wght)])
            .leading(640.0)
            .anchor((WIDTH / 2.0, 620.0))
            .align(Align::Center)
            .width(MEASURE)
            .fit("wdth"),
    )
}

fn kinetic_type() -> Result<Drawing, Error> {
    let font = &Font::new(include_bytes!("../../../examples/fonts/anybody-subset.ttf"))?;
    let weight = Param::new("weight", 300.0, 100.0, 900.0)?.meaning("how heavy the letters are, before a kick");
    let rise = fraction("rise", "how far the blue plane has risen")?;
    let ink = fraction("ink", "0: hairlines only; 1: the riso print")?;
    let burst = fraction("burst", "how hard the last hit scattered the word")?;
    let pulse = fraction("pulse", "a kick, per character of the text")?;
    let bend = fraction("bend", "how far the headline arches and ripples")?;
    let phase = Param::new("phase", 0.0, 0.0, 1.0)?
        .unit(Unit::Cycle)?
        .meaning("where the ripple and the shaking are");
    let turn = fraction("turn", "how far NO SLEEP is GO DREAM")?;

    let first = headline("NO\nSLEEP", &weight + 400.0 * &pulse, font);
    let second = headline("GO\nDREAM", &weight, font);

    // The arch is anchored to the calm bounds, or it would breathe with every kick.
    let calm = first.bounds(&Values::new())?;
    let letters = first
        .resolve(0.01)
        .morph(second.resolve(0.01), Stagger::new(&turn).spread(0.6))
        .arch(Arch::new(0.06 * &bend).frame(calm))
        .wave(16.0 * &bend, 320.0, Deg(0.0), &phase)
        .wobble(3.0 * &bend, 60.0, &phase, 7.0)
        .rotate(Deg(Random::new(-10.0, 10.0).seed(7.0) * &burst))
        .translate(
            Random::new(-30.0, 30.0).seed(8.0) * &burst,
            Random::new(-50.0, 50.0).seed(9.0) * &burst,
        );

    let page = Path::rect(0.0, 0.0, WIDTH, HEIGHT, 0.0)?;
    let misregistration = 3.0 + 14.0 * &burst; // the blue plate slips off the pink on a hit
    let plane = Drawing::from(page.clone()).translate(0.0, HEIGHT * (1.0 - &rise));
    let blue = plane
        .subtract(letters.translate(&misregistration, 0.6 * &misregistration), 0.01)
        .fill(Color::hex(0x2a_5b_ff))
        .blend(Blend::Multiply);
    let pink = letters
        .fill(Color::hex(0xff_4f_9a))
        .blend(Blend::Multiply)
        .opacity(&ink);
    let hairlines = letters
        .stroke(Stroke::new(Color::hex(0x16_15_1a)).width(1.5))
        .opacity(1.0 - &ink);
    let paper = page.fill(Color::hex(0xf3_ee_e4));
    Ok(Drawing::new([paper, blue, pink, hairlines]).with_size(WIDTH, HEIGHT))
}

/// 0 up to 1 over `x` from 0 to 1, easing in and out.
fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// 1 on a beat, falling to 0 within 0.3 s, and again on every beat.
fn kick(t: f64) -> f64 {
    let falling = (1.0 - t.rem_euclid(BEAT) / 0.3).max(0.0);
    falling * falling * falling
}

/// The values `t` seconds in (0 to 12), as the timeline in the docstring of `examples/kinetic_type.py`.
fn values_at(t: f64) -> Values {
    let printed = (4.0..12.0).contains(&t);
    let hitting = printed && t < 8.0;
    let hit = |at: f64| if hitting { kick(at) } else { 0.0 };
    Values::new()
        .set("rise", smooth((t - 2.0) / 2.0))
        .set("ink", if printed { 1.0 - smooth((t - 11.5) / 0.5) } else { 0.0 })
        .set(
            "weight",
            if printed {
                300.0 - 200.0 * smooth((t - 10.0) / 1.5)
            } else {
                100.0
            },
        )
        .set("burst", hit(t - 4.0))
        .set(
            "pulse",
            (0..CHARACTERS)
                .map(|i| hit(t - 4.0 - 0.045 * f64::from(i)))
                .collect::<Vec<_>>(),
        )
        .set("bend", smooth(t - 5.5) * (1.0 - smooth(t - 7.5)))
        .set("phase", (t * 0.25).rem_euclid(1.0))
        .set("turn", smooth((t - 8.0) / 1.5))
}

fn svg(drawing: &Drawing, t: f64) -> Result<String, Error> {
    drawing.to_svg(Size::px(WIDTH, HEIGHT), &values_at(t))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let drawing = kinetic_type()?;
    let mut out = std::io::stdout();
    match std::env::args().nth(1).as_deref() {
        Some(name) => {
            let (_, t) = FRAMES
                .iter()
                .find(|frame| frame.0 == name)
                .ok_or_else(|| format!("expected hairlines, kick, bend or morph, got {name}"))?;
            out.write_all(svg(&drawing, *t)?.as_bytes())?;
        }
        None => {
            for (name, t) in FRAMES {
                writeln!(out, "{name}: svg {} bytes", svg(&drawing, t)?.len())?;
            }
        }
    }
    Ok(())
}
