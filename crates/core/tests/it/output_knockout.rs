//! A knockout in the pixels `Drawing::to_rgba` draws: the hole it cuts, what it leaves alone, and what it ignores of the
//! shape that cuts. Pixel values are worked out by hand on shapes with whole-pixel edges (one output pixel to the design
//! pixel), never taken from the library.

use crate::common::Outcome;
use crate::common::pixels::{Image, render};
use bullterrier_vector::{Blend, Cap, Color, Drawing, Error, Path, Shadow, Stroke};

const RED: Color = Color::hex(0xff_00_00);
const GREEN: Color = Color::hex(0x00_ff_00);
const BLUE: Color = Color::hex(0x00_00_ff);
const NOTHING: [u8; 4] = [0, 0, 0, 0];
const SIDE: usize = 100;

fn rect(left: f64, top: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(left, top, width, height, 0.0)
}

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

/// A filled rectangle.
fn filled(left: f64, top: f64, width: f64, height: f64, color: Color) -> Result<Drawing, Error> {
    Ok(Drawing::from(rect(left, top, width, height)?).fill(color))
}

/// A filled rectangle that knocks out.
fn hole(left: f64, top: f64, width: f64, height: f64) -> Result<Drawing, Error> {
    Ok(filled(left, top, width, height, BLUE)?.knockout())
}

fn picture(drawing: &Drawing) -> Result<Image, Error> {
    render(drawing, SIDE, SIDE)
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
fn a_knockout_cuts_a_hole_and_leaves_the_rest_and_paints_nothing_itself() -> Outcome {
    let image = picture(&Drawing::new([
        filled(0.0, 0.0, 100.0, 100.0, RED)?,
        hole(30.0, 30.0, 40.0, 40.0)?,
    ]))?;
    for (at, rgba) in [
        ((50, 50), NOTHING),
        ((30, 30), NOTHING),
        ((69, 69), NOTHING),
        ((29, 50), [255, 0, 0, 255]),
        ((70, 50), [255, 0, 0, 255]),
        ((50, 29), [255, 0, 0, 255]),
        ((50, 70), [255, 0, 0, 255]),
        ((5, 5), [255, 0, 0, 255]),
    ] {
        is(&image, at, rgba)?;
    }
    // 100 * 100 less the 40 * 40 hole is covered, and nothing is blue
    assert_eq!(image.covered_area(), 10_000.0 - 1_600.0);
    Ok(())
}

#[test]
fn a_shape_after_the_knockout_paints_over_the_hole_and_one_before_it_is_cut() -> Outcome {
    let image = picture(&Drawing::new([
        filled(0.0, 0.0, 100.0, 100.0, RED)?,
        hole(30.0, 30.0, 40.0, 40.0)?,
        filled(40.0, 40.0, 20.0, 20.0, GREEN)?,
    ]))?;
    for (at, rgba) in [
        ((50, 50), [0, 255, 0, 255]),
        ((35, 35), NOTHING),
        ((65, 65), NOTHING),
        ((20, 20), [255, 0, 0, 255]),
    ] {
        is(&image, at, rgba)?;
    }
    Ok(())
}

#[test]
fn a_knockout_with_nothing_below_it_draws_nothing_at_all() -> Outcome {
    assert!(picture(&hole(30.0, 30.0, 40.0, 40.0)?)?.all_pixels_are(NOTHING));
    Ok(())
}

#[test]
fn the_edge_of_a_hole_is_anti_aliased_like_any_other_edge() -> Outcome {
    // the hole starts half way into column 30 and row 30, so the pixels there are cut by half and a quarter
    let image = picture(&Drawing::new([
        filled(0.0, 0.0, 100.0, 100.0, RED)?,
        hole(30.5, 30.5, 39.0, 39.0)?,
    ]))?;
    for (at, alpha) in [
        ((30, 50), 128),
        ((50, 30), 128),
        ((30, 30), 191),
        ((69, 50), 128),
        ((50, 50), 0),
    ] {
        let found = image.alpha(at.0, at.1);
        assert!(
            found.abs_diff(alpha) <= 1,
            "pixel {at:?} has alpha {found}, not about {alpha}"
        );
    }
    Ok(())
}

#[test]
fn what_is_left_of_a_translucent_shape_keeps_its_alpha() -> Outcome {
    let image = picture(&Drawing::new([
        filled(0.0, 0.0, 100.0, 100.0, color("#ff000080")?)?,
        hole(30.0, 30.0, 40.0, 40.0)?,
    ]))?;
    is(&image, (10, 10), [255, 0, 0, 128])?;
    is(&image, (50, 50), NOTHING)
}

#[test]
fn the_shadow_of_a_shape_below_is_cut_too() -> Outcome {
    // a red square 20..60 across with a shadow moved 20 down, black and solid: the shadow lies in 20..60 across and
    // 40..80 down, and the hole 0..100 across and 50..70 down cuts through it and the square. A blur of 0 is a copy of the
    // path, a blur of 10 a filter; both are cut.
    for (blur, case) in [(0.0, "a hard shadow"), (10.0, "a blurred shadow")] {
        let below =
            filled(20.0, 20.0, 40.0, 40.0, RED)?.shadow(Shadow::new().dy(20.0).blur(blur).color(color("#000000")?));
        let image = picture(&Drawing::new([below, hole(0.0, 50.0, 100.0, 20.0)?]))?;
        // inside the hole: nothing, in the square (55) and in the shadow beyond it (65)
        is(&image, (40, 55), NOTHING).map_err(|e| format!("{case}: {e}"))?;
        is(&image, (40, 65), NOTHING).map_err(|e| format!("{case}: {e}"))?;
        // above the hole the square is as it was, and the shadow below it shows
        is(&image, (40, 30), [255, 0, 0, 255]).map_err(|e| format!("{case}: {e}"))?;
        assert!(image.alpha(40, 75) > 0, "{case}: the shadow below the hole is gone");
    }
    Ok(())
}

#[test]
fn a_knockout_that_is_only_a_stroke_cuts_the_stroke_and_not_what_it_encloses() -> Outcome {
    // a ring of width 10 about the rectangle 30..70: it covers 25..35 and 65..75 across the edges
    let ring = Drawing::from(rect(30.0, 30.0, 40.0, 40.0)?)
        .fill(Color::NONE)
        .stroke(Stroke::new(BLUE).width(10.0).cap(Cap::Butt))
        .knockout();
    let image = picture(&Drawing::new([filled(0.0, 0.0, 100.0, 100.0, RED)?, ring]))?;
    for (at, rgba) in [
        ((30, 50), NOTHING),
        ((25, 50), NOTHING),
        ((34, 50), NOTHING),
        ((69, 50), NOTHING),
        ((74, 50), NOTHING),
        ((24, 50), [255, 0, 0, 255]),
        ((75, 50), [255, 0, 0, 255]),
        ((50, 50), [255, 0, 0, 255]),
        ((36, 50), [255, 0, 0, 255]),
    ] {
        is(&image, at, rgba)?;
    }
    Ok(())
}

#[test]
fn a_knockout_with_a_fill_and_a_stroke_cuts_both() -> Outcome {
    let both = Drawing::from(rect(30.0, 30.0, 40.0, 40.0)?)
        .fill(GREEN)
        .stroke(Stroke::new(BLUE).width(10.0).cap(Cap::Butt))
        .knockout();
    let image = picture(&Drawing::new([filled(0.0, 0.0, 100.0, 100.0, RED)?, both]))?;
    is(&image, (50, 50), NOTHING)?;
    is(&image, (27, 50), NOTHING)?;
    is(&image, (24, 50), [255, 0, 0, 255])
}

#[test]
fn a_knockout_in_the_shape_of_a_ring_leaves_what_its_hole_encloses() -> Outcome {
    // the inner square runs the other way, so by the nonzero rule it is a hole in the outer
    let ring = Drawing::from(Path::from_svg_path(
        "M20 20 L80 20 L80 80 L20 80 Z M40 40 L40 60 L60 60 L60 40 Z",
    )?)
    .fill(BLUE)
    .knockout();
    let image = picture(&Drawing::new([filled(0.0, 0.0, 100.0, 100.0, RED)?, ring]))?;
    is(&image, (30, 50), NOTHING)?;
    is(&image, (50, 30), NOTHING)?;
    is(&image, (50, 50), [255, 0, 0, 255])?;
    is(&image, (10, 10), [255, 0, 0, 255])
}

#[test]
fn two_knockouts_cut_what_is_below_each_and_not_what_is_above_it() -> Outcome {
    // red everywhere, a hole 20..40 across and down, blue over the top half, a hole 30..50 across and down: blue is cut
    // by the second hole only, red by both
    let image = picture(&Drawing::new([
        filled(0.0, 0.0, 100.0, 100.0, RED)?,
        hole(20.0, 20.0, 20.0, 20.0)?,
        filled(0.0, 0.0, 100.0, 50.0, BLUE)?,
        hole(30.0, 30.0, 20.0, 20.0)?,
    ]))?;
    for (at, rgba) in [
        // only the first hole, under the blue that comes after it
        ((22, 22), [0, 0, 255, 255]),
        // in both holes: the blue is cut by the second, the red by both
        ((35, 35), NOTHING),
        // only the second hole
        ((45, 45), NOTHING),
        // only the first hole, below the blue
        ((22, 60), [255, 0, 0, 255]),
        ((60, 22), [0, 0, 255, 255]),
        ((60, 60), [255, 0, 0, 255]),
    ] {
        is(&image, at, rgba)?;
    }
    Ok(())
}

#[test]
fn two_knockouts_in_a_row_cut_the_union_of_their_holes() -> Outcome {
    let image = picture(&Drawing::new([
        filled(0.0, 0.0, 100.0, 100.0, RED)?,
        hole(10.0, 10.0, 30.0, 30.0)?,
        hole(30.0, 30.0, 30.0, 30.0)?,
    ]))?;
    for (at, rgba) in [
        ((20, 20), NOTHING),
        ((50, 50), NOTHING),
        ((35, 35), NOTHING),
        ((20, 50), [255, 0, 0, 255]),
        ((50, 20), [255, 0, 0, 255]),
    ] {
        is(&image, at, rgba)?;
    }
    Ok(())
}

#[test]
fn the_colour_the_opacity_the_blend_and_the_shadow_of_the_shape_that_cuts_do_not_matter() -> Outcome {
    // the hole is the area covered: a faint colour, an opacity of 0.1 and a multiply cut as clean as an opaque one,
    // and the shadow the shape casts neither cuts nor paints
    let square = Drawing::from(rect(30.0, 30.0, 40.0, 40.0)?);
    let faint = square.fill(color("#0000ff10")?).opacity(0.1).blend(Blend::Multiply);
    let shadowed = square
        .fill(BLUE)
        .shadow(Shadow::new().dx(30.0).dy(30.0).blur(0.0).color(color("#000000")?));
    for (case, knocker) in [("faint", faint), ("shadowed", shadowed)] {
        let image = picture(&Drawing::new([
            filled(0.0, 0.0, 100.0, 100.0, RED)?,
            knocker.knockout(),
        ]))?;
        is(&image, (50, 50), NOTHING).map_err(|e| format!("{case}: {e}"))?;
        // where the shadow would be (80, 80) the red stays, untouched and unpainted
        is(&image, (80, 80), [255, 0, 0, 255]).map_err(|e| format!("{case}: {e}"))?;
    }
    Ok(())
}

#[test]
fn a_shape_that_paints_nothing_or_has_no_path_removes_nothing() -> Outcome {
    let red = || filled(0.0, 0.0, 100.0, 100.0, RED);
    let plain = picture(&red()?)?;
    let nothing_painted = Drawing::from(rect(30.0, 30.0, 40.0, 40.0)?)
        .fill(Color::NONE)
        .knockout();
    let no_path = Drawing::from(Path::new()).fill(BLUE).knockout();
    for (case, knocker) in [("no paint", nothing_painted), ("no path", no_path)] {
        let image = picture(&Drawing::new([red()?, knocker]))?;
        assert_eq!(image.bytes, plain.bytes, "{case}");
    }
    Ok(())
}

#[test]
fn a_knockout_beyond_the_canvas_cuts_only_what_reaches_the_canvas() -> Outcome {
    // a hole from far left of the canvas to its middle
    let image = picture(&Drawing::new([
        filled(0.0, 0.0, 100.0, 100.0, RED)?,
        hole(-1000.0, -1000.0, 1050.0, 2000.0)?,
    ]))?;
    is(&image, (0, 0), NOTHING)?;
    is(&image, (49, 99), NOTHING)?;
    is(&image, (50, 0), [255, 0, 0, 255])
}
