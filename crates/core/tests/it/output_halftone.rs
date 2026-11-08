//! A halftone in the pixels `Drawing::to_rgba` draws: how much of the shape the dots cover, that nothing outside the
//! shape is painted, tone 0 and tone 1, a translucent colour where dots overlap, and what the other style steps do to a
//! screen. Values are worked out by hand from the pitch and the tone, never taken from the library.

use crate::common::pixels::{Image, render};
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Blend, Color, Deg, Drawing, Error, Halftone, Mesh, Path, Shadow, Size, Stroke, Values};
use std::f64::consts::PI;

const RED: Color = Color::hex(0xff_00_00);
const BLUE: Color = Color::hex(0x00_00_ff);
const BLACK: Color = Color::hex(0x00_00_00);
const NOTHING: [u8; 4] = [0, 0, 0, 0];

fn rect(left: f64, top: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(left, top, width, height, 0.0)
}

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

/// An error unless the pixel at `(x, y)` is `rgba`.
fn is(image: &Image, (x, y): (usize, usize), rgba: [u8; 4]) -> Outcome {
    let found = image.pixel(x, y);
    if found == rgba {
        Ok(())
    } else {
        Err(format!("pixel ({x}, {y}) is {found:?}, not {rgba:?}").into())
    }
}

#[test]
fn the_dots_cover_the_share_of_the_cells_the_tone_leaves_over() -> Outcome {
    // a square 90 wide with a dot of pitch 10 at each of the 81 lattice points 10 to 90, none within 5 of the edge
    // (a dot is under 5 across the radius: tones above a fifth): the covered area is 81 cells' share of 100
    for tone in [0.3, 0.5, 0.8, 0.95] {
        let drawing =
            Drawing::from(rect(5.0, 5.0, 90.0, 90.0)?).halftone(Halftone::new(BLACK, tone).pitch(10.0).angle(Deg(0.0)));
        let covered = render(&drawing, 100, 100)?.covered_area();
        let expected = 81.0 * (1.0 - tone) * 100.0;
        assert!(
            (covered - expected).abs() < 0.015 * expected,
            "tone {tone}: covered {covered}, expected {expected}"
        );
    }
    Ok(())
}

#[test]
fn a_turned_screen_clipped_to_a_disc_covers_the_share_of_the_disc() -> Outcome {
    for (tone, angle) in [(0.5, 45.0), (0.7, 20.0), (0.4, 80.0)] {
        let drawing = Drawing::from(Path::circle((50.0, 50.0), 40.0)?)
            .halftone(Halftone::new(BLACK, tone).pitch(4.0).angle(Deg(angle)));
        let covered = render(&drawing, 100, 100)?.covered_area();
        let expected = (1.0 - tone) * PI * 40.0 * 40.0;
        assert!(
            (covered - expected).abs() < 0.03 * expected,
            "tone {tone}, angle {angle}: covered {covered}, expected {expected}"
        );
    }
    Ok(())
}

#[test]
fn no_pixel_outside_the_shape_is_painted() -> Outcome {
    // the dots at the edge are half in; a pixel wholly outside the disc (by more than its half diagonal) is clear
    for (pitch, angle, tone) in [(8.0, 20.0, 0.4), (13.0, 0.0, 0.05), (5.0, 45.0, 0.9)] {
        let drawing = Drawing::from(Path::circle((50.0, 50.0), 30.0)?)
            .halftone(Halftone::new(BLACK, tone).pitch(pitch).angle(Deg(angle)));
        let image = render(&drawing, 100, 100)?;
        let mut painted_inside = 0;
        for y in 0..100 {
            for x in 0..100 {
                #[expect(clippy::cast_precision_loss, reason = "pixel coordinates")]
                let from_centre = libm::hypot(x as f64 + 0.5 - 50.0, y as f64 + 0.5 - 50.0);
                if from_centre > 30.0 + 0.75 {
                    assert_eq!(
                        image.alpha(x, y),
                        0,
                        "pitch {pitch}: pixel ({x}, {y}) is outside the disc"
                    );
                } else if image.alpha(x, y) > 0 {
                    painted_inside += 1;
                }
            }
        }
        assert!(
            painted_inside > 100,
            "pitch {pitch}: only {painted_inside} pixels painted"
        );
    }
    Ok(())
}

#[test]
fn a_tone_of_zero_is_the_solid_fill_and_a_tone_of_one_paints_nothing() -> Outcome {
    let shape = || -> Result<Drawing, Error> { Ok(Drawing::from(Path::circle((50.0, 50.0), 30.0)?)) };
    let solid = render(&shape()?.fill(color("#33669980")?), 100, 100)?;
    let zero = render(&shape()?.halftone(Halftone::new(color("#33669980")?, 0.0)), 100, 100)?;
    assert!(zero.bytes == solid.bytes, "tone 0 is not the solid fill");
    assert!(render(&shape()?.halftone(Halftone::new(RED, 1.0)), 100, 100)?.all_pixels_are(NOTHING));
    // a stroke is all that is left of a bare screen
    let stroked = render(&shape()?.halftone(Halftone::new(RED, 1.0)).stroke(BLUE), 100, 100)?;
    let outline = render(&shape()?.fill(Color::NONE).stroke(BLUE), 100, 100)?;
    assert!(stroked.bytes == outline.bytes, "a bare screen is not the stroke alone");
    Ok(())
}

#[test]
fn a_translucent_colour_is_no_darker_where_two_dots_overlap() -> Outcome {
    // pitch 40 and tone 0.1: a dot has the radius 40 * sqrt(0.9 / pi) = 21.41 and the lens where two neighbours at 40
    // and 80 overlap is 58.59 to 61.41 across, so the pixels 59 and 60 of the row of their centres are in both
    let drawing = Drawing::from(rect(10.0, 10.0, 120.0, 120.0)?)
        .halftone(Halftone::new(color("#00000080")?, 0.1).pitch(40.0).angle(Deg(0.0)));
    let image = render(&drawing, 140, 140)?;
    for at in [(20, 40), (59, 39), (60, 40), (59, 41), (79, 79)] {
        is(&image, at, [0, 0, 0, 128])?;
    }
    Ok(())
}

#[test]
fn the_stroke_is_over_the_dots_and_the_opacity_applies_to_both_once() -> Outcome {
    // dots of radius 7.98 at 20, 40, ... on a square 10 to 90; a stroke 6 wide on its edge; half opacity
    let drawing = Drawing::from(rect(10.0, 10.0, 80.0, 80.0)?)
        .halftone(Halftone::new(RED, 0.5).pitch(20.0).angle(Deg(0.0)))
        .stroke(Stroke::new(BLACK).width(6.0))
        .opacity(0.5);
    let image = render(&drawing, 100, 100)?;
    for (at, rgba) in [
        // a dot's middle, a stroke over a dot's edge (the dot at 20, 20 reaches the edge at 10), a gap between dots
        ((40, 40), [255, 0, 0, 128]),
        ((50, 11), [0, 0, 0, 128]),
        ((30, 30), NOTHING),
        ((20, 11), [0, 0, 0, 128]),
    ] {
        is(&image, at, rgba)?;
    }
    Ok(())
}

#[test]
fn a_hard_shadow_is_a_copy_of_the_dots_moved_and_tinted() -> Outcome {
    // dots of radius 15.96 at 40 and 80 each way; the copy moved 20 right and down has its dots at 60 and 100
    let drawing = Drawing::from(rect(5.0, 5.0, 90.0, 90.0)?)
        .halftone(Halftone::new(RED, 0.5).pitch(40.0).angle(Deg(0.0)))
        .shadow(Shadow::new().dx(20.0).dy(20.0).blur(0.0).color(color("#0000ffff")?));
    let image = render(&drawing, 100, 100)?;
    for (at, rgba) in [
        ((40, 40), [255, 0, 0, 255]),
        ((60, 60), [0, 0, 255, 255]),
        ((60, 40), NOTHING),
    ] {
        is(&image, at, rgba)?;
    }
    Ok(())
}

#[test]
fn a_knockout_of_dots_takes_the_dots_out_of_what_is_below() -> Outcome {
    let drawing = Drawing::new([
        Drawing::from(rect(0.0, 0.0, 100.0, 100.0)?).fill(RED),
        Drawing::from(rect(0.0, 0.0, 100.0, 100.0)?)
            .halftone(Halftone::new(BLACK, 0.5).pitch(20.0).angle(Deg(0.0)))
            .knockout(),
    ]);
    let image = render(&drawing, 100, 100)?;
    for (at, rgba) in [
        ((40, 40), NOTHING),
        ((30, 30), [255, 0, 0, 255]),
        ((50, 50), [255, 0, 0, 255]),
    ] {
        is(&image, at, rgba)?;
    }
    // red less 25 dots of 100 * 0.5 each, to the polygon
    let expected = 10_000.0 - 25.0 * 200.0;
    assert!((image.covered_area() - expected).abs() < 0.02 * expected);
    Ok(())
}

#[test]
fn dots_multiplied_over_paper_darken_it() -> Outcome {
    let drawing = Drawing::new([
        Drawing::from(rect(0.0, 0.0, 100.0, 100.0)?).fill(color("#808080")?),
        Drawing::from(rect(5.0, 5.0, 90.0, 90.0)?)
            .halftone(Halftone::new(color("#ff8000")?, 0.5).pitch(20.0).angle(Deg(0.0)))
            .blend(Blend::Multiply),
    ]);
    let image = render(&drawing, 100, 100)?;
    // grey 128 times (255, 128, 0) / 255 is (128, 64, 0)
    is(&image, (40, 40), [128, 64, 0, 255])?;
    is(&image, (30, 30), [128, 128, 128, 255])
}

#[test]
fn a_pitch_that_asks_for_millions_of_dots_is_refused_naming_the_pitch() -> Outcome {
    let drawing = Drawing::from(Path::circle((500.0, 500.0), 500.0)?).halftone(Halftone::new(BLACK, 0.5).pitch(0.01));
    let (name, message) = parameter_refusal(drawing.to_rgba(
        bullterrier_vector::Size::px(100.0, 100.0),
        &bullterrier_vector::Values::new(),
    ))?;
    assert!(
        name.contains("pitch") && message.contains("MAX_SEGMENTS"),
        "{name}: {message}"
    );
    Ok(())
}

#[test]
fn a_pitch_too_small_for_the_numbers_of_its_lattice_is_refused_naming_the_pitch() -> Outcome {
    // (pitch, where the shape is): the lattice coordinates, the position over the pitch, are 10^301 in the first row,
    // and beyond what a float holds in the others, which leaves no count of dots to charge
    for (pitch, at) in [(1e-300, 10.0), (1e-320, 10.0), (1e-310, 50.0)] {
        let drawing =
            Drawing::from(rect(at, at, 10.0, 10.0)?).halftone(Halftone::new(BLACK, 0.5).pitch(pitch).angle(Deg(0.0)));
        let (name, message) = parameter_refusal(drawing.to_rgba(
            bullterrier_vector::Size::px(100.0, 100.0),
            &bullterrier_vector::Values::new(),
        ))?;
        assert!(
            name.contains("pitch") && message.contains("MAX_SEGMENTS"),
            "pitch {pitch}: {name}: {message}"
        );
    }
    Ok(())
}

#[test]
fn a_pitch_too_small_for_a_shape_off_the_canvas_is_refused_in_every_output() -> Outcome {
    // 10^10 dots of a screen over a square nowhere near the 100 by 100 canvas
    let drawing =
        Drawing::from(rect(10_000.0, 10_000.0, 100.0, 100.0)?).halftone(Halftone::new(BLACK, 0.5).pitch(0.001));
    let size = Size::px(100.0, 100.0);
    let refusals = [
        ("to_svg", parameter_refusal(drawing.to_svg(size, &Values::new()))),
        ("to_rgba", parameter_refusal(drawing.to_rgba(size, &Values::new()))),
        (
            "to_mesh",
            parameter_refusal(drawing.to_mesh(size, &Values::new(), &mut Mesh::new())),
        ),
    ];
    for (output, refusal) in refusals {
        let (name, message) = refusal.map_err(|error| format!("{output}: {error}"))?;
        assert!(
            name.contains("pitch") && message.contains("MAX_SEGMENTS"),
            "{output}: {name}: {message}"
        );
    }
    Ok(())
}
