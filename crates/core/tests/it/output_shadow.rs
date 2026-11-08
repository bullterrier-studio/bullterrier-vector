//! A shadow in the pixels `to_rgba` draws: its offset, its blur against the Gaussian worked in f64, its group with the
//! shape, and the limits that keep a blur from asking for unbounded work. Expected values are worked out by hand or
//! from the error function of `libm`, never from the library.

use crate::common::pixels::{Image, render};
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Blend, Cap, Color, Drawing, Error, Path, Shadow, Size, Stroke, Values};

const RED: Color = Color::hex(0xff_00_00);

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

/// A pixel number as a length.
#[expect(clippy::cast_precision_loss, reason = "test sizes are small")]
fn px(count: usize) -> f64 {
    count as f64
}

fn rect(left: f64, top: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(left, top, width, height, 0.0)
}

/// A filled rectangle with `shadow`.
fn shape(path: Path, shadow: Shadow) -> Drawing {
    Drawing::from(path).fill(RED).shadow(shadow)
}

/// The probability that a Gaussian of mean 0 and deviation 1 is below `z`.
fn cdf(z: f64) -> f64 {
    f64::midpoint(1.0, libm::erf(z / std::f64::consts::SQRT_2))
}

/// The share of a Gaussian of deviation `sigma` centred at `x` that lies between `from` and `to`.
fn between(x: f64, sigma: f64, from: f64, to: f64) -> f64 {
    cdf((to - x) / sigma) - cdf((from - x) / sigma)
}

/// An error unless the two pictures agree to `tolerance` levels in every pixel, in alpha and in each colour times its
/// alpha (a straight colour of a faint pixel moves by several levels for one level of the product), naming how many
/// pixels do not and the first.
fn same(left: &Image, right: &Image, tolerance: u8) -> Outcome {
    let premultiplied = |[red, green, blue, alpha]: [u8; 4]| {
        [red, green, blue]
            .map(|channel| (u16::from(channel) * u16::from(alpha) + 127) / 255)
            .into_iter()
            .chain([u16::from(alpha)])
    };
    let mut differing = Vec::new();
    for y in 0..left.height {
        for x in 0..left.width {
            let off = premultiplied(left.pixel(x, y))
                .zip(premultiplied(right.pixel(x, y)))
                .any(|(a, b)| a.abs_diff(b) > u16::from(tolerance));
            if off {
                differing.push((x, y));
            }
        }
    }
    match differing.first() {
        None => Ok(()),
        Some(&(x, y)) => Err(format!(
            "{} pixels differ, the first ({x}, {y}): {:?} against {:?}",
            differing.len(),
            left.pixel(x, y),
            right.pixel(x, y),
        )
        .into()),
    }
}

// The blur ----------------------------------------------------------------------------------------------------------

#[test]
fn the_blur_of_a_rectangle_follows_the_gaussian_across_each_edge() -> Outcome {
    // a rectangle 80 by 80 design pixels in the middle of a frame 200 across, no offset, a black shadow at `strength`;
    // the pixels beside the rectangle show the shadow alone. The Gaussian's deviation is half the blur, times the scale
    // of the output to the frame. (blur, strength, output side, the most a level may differ by)
    let rows = [
        (8.0, 255, 200, 2.0),
        (16.0, 255, 200, 2.0),
        (16.0, 128, 200, 2.0),
        (32.0, 255, 200, 3.0),
        (60.0, 255, 200, 4.0),
        (16.0, 255, 400, 2.0),
        (16.0, 255, 100, 2.0),
    ];
    for (blur, strength, side, tolerance) in rows {
        let scale = px(side) / 200.0;
        let sigma = blur / 2.0 * scale;
        let shadow = Shadow::new()
            .dx(0.0)
            .dy(0.0)
            .blur(blur)
            .color(color(&format!("#000000{strength:02x}"))?);
        let drawing = shape(rect(60.0, 60.0, 80.0, 80.0)?, shadow).with_size(200.0, 200.0);
        let image = render(&drawing, side, side)?;
        let (near, far) = (60.0 * scale, 140.0 * scale);
        let middle = side / 2;
        // the shadow alone, at a pixel centre: the share of the Gaussian over the rectangle in each axis
        let wanted = |x: usize, y: usize| {
            f64::from(strength) * between(px(x) + 0.5, sigma, near, far) * between(px(y) + 0.5, sigma, near, far)
        };
        // left and right of the rectangle along its middle row, above and below it along its middle column
        for at in (0..side).filter(|at| px(*at) < near - 1.0 || px(*at) > far + 1.0) {
            for (x, y) in [(at, middle), (middle, at)] {
                let alpha = f64::from(image.alpha(x, y));
                assert!(
                    (alpha - wanted(x, y)).abs() <= tolerance,
                    "blur {blur} strength {strength} side {side} at ({x}, {y}): alpha {alpha}, the Gaussian says {:.2}",
                    wanted(x, y)
                );
            }
        }
    }
    Ok(())
}

// Offset and colour -------------------------------------------------------------------------------------------------

/// The pixel of the picture a rectangle `[left, right) x [top, bottom)` of whole pixels paints in `rgba`.
fn inside(rectangle: (usize, usize, usize, usize), x: usize, y: usize) -> bool {
    let (left, top, right, bottom) = rectangle;
    (left..right).contains(&x) && (top..bottom).contains(&y)
}

#[test]
fn the_shadow_lies_where_dx_and_dy_move_it() -> Outcome {
    // a red square on columns and rows 20 to 40 of a frame 60 across, a black shadow with no blur: every pixel is the
    // square, or the square moved, or nothing; drawn at the size of the frame and at twice it. (dx, dy, scale)
    let rows = [
        (10.0, 0.0, 1),
        (0.0, 10.0, 1),
        (-10.0, 0.0, 1),
        (0.0, -10.0, 1),
        (7.0, -5.0, 1),
        (-3.0, 12.0, 1),
        (7.0, -5.0, 2),
    ];
    for (dx, dy, scale) in rows {
        let drawing = shape(
            rect(20.0, 20.0, 20.0, 20.0)?,
            Shadow::new().dx(dx).dy(dy).blur(0.0).color(color("#000000")?),
        )
        .with_size(60.0, 60.0);
        let side = 60 * scale;
        let image = render(&drawing, side, side)?;
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "whole pixels")]
        let moved = (
            ((20.0 + dx) as usize * scale),
            ((20.0 + dy) as usize * scale),
            ((40.0 + dx) as usize * scale),
            ((40.0 + dy) as usize * scale),
        );
        for y in 0..side {
            for x in 0..side {
                let wanted = if inside((20 * scale, 20 * scale, 40 * scale, 40 * scale), x, y) {
                    [255, 0, 0, 255]
                } else if inside(moved, x, y) {
                    [0, 0, 0, 255]
                } else {
                    [0, 0, 0, 0]
                };
                assert_eq!(image.pixel(x, y), wanted, "dx {dx} dy {dy} scale {scale} at ({x}, {y})");
            }
        }
    }
    Ok(())
}

#[test]
fn without_a_blur_the_shadow_is_the_silhouette_moved_and_tinted() -> Outcome {
    // the hand-built copy is a second shape, painted first, in the tint; edges that fall between pixels included. The
    // copy paints the colour with its alpha, the shadow paints the silhouette in a layer of that alpha: a level apart at
    // the anti-aliased edges
    let tint = color("#1020304d")?;
    let path = rect(20.0, 20.0, 20.5, 14.0)?;
    let filled = shape(path.clone(), Shadow::new().dx(3.5).dy(2.25).blur(0.0).color(tint));
    let copy = Drawing::new([
        Drawing::from(path.translate(3.5, 2.25)?).fill(tint),
        Drawing::from(path.clone()).fill(RED),
    ]);
    same(&render(&filled, 80, 80)?, &render(&copy, 80, 80)?, 1)?;
    // a stroke's silhouette is the stroke
    let line = Path::line((10.0, 10.0), (50.0, 30.0))?;
    let stroke = |colour| Stroke::new(colour).width(6.0).cap(Cap::Square);
    let stroked = Drawing::from(line.clone())
        .stroke(stroke(RED))
        .shadow(Shadow::new().dx(-4.0).dy(9.5).blur(0.0).color(tint));
    let copy = Drawing::new([
        Drawing::from(line.translate(-4.0, 9.5)?).stroke(stroke(tint)),
        Drawing::from(line).stroke(stroke(RED)),
    ]);
    same(&render(&stroked, 80, 80)?, &render(&copy, 80, 80)?, 1)?;
    Ok(())
}

// The group ---------------------------------------------------------------------------------------------------------

#[test]
fn opacity_applies_once_to_the_shadow_and_the_shape_together() -> Outcome {
    // a red square on columns 20 to 40, opaque but for the shape's opacity of a half; a black shadow of alpha 0.8 moved
    // 10 to the right, so that columns 30 to 40 hold both, 20 to 30 the shape alone and 40 to 50 the shadow alone
    let drawing = shape(
        rect(20.0, 20.0, 20.0, 20.0)?,
        Shadow::new().dx(10.0).dy(0.0).blur(0.0).color(color("#000000cc")?),
    )
    .opacity(0.5);
    let image = render(&drawing, 60, 60)?;
    // the shape hides the shadow beneath it, and then the group is halved: one alpha of 0.5 where the shape is, however
    // many layers lie there; 0.8 of a half where only the shadow is
    for x in 20..40 {
        let [red, green, blue, alpha] = image.pixel(x, 30);
        assert_eq!((red, green, blue), (255, 0, 0), "column {x}");
        assert!(alpha.abs_diff(128) <= 1, "column {x}: alpha {alpha}, wanted a half");
    }
    assert_eq!(
        image.pixel(25, 30),
        image.pixel(35, 30),
        "the shadow shows through the shape"
    );
    for x in 40..50 {
        let [red, green, blue, alpha] = image.pixel(x, 30);
        assert_eq!((red, green, blue), (0, 0, 0), "column {x}");
        assert!(
            alpha.abs_diff(102) <= 1,
            "column {x}: alpha {alpha}, wanted 0.8 of a half"
        );
    }
    Ok(())
}

#[test]
fn multiply_applies_once_to_the_shadow_and_the_shape_together() -> Outcome {
    // a paper of #c0c0c0 below; the same square and shadow, multiplied. By the W3C formula a source of alpha s and
    // colour Cs over a backdrop Cb gives (1 - s) Cb + s Cb Cs: the black shadow of alpha 128/255 darkens 192 to 96, and
    // the red square, opaque, gives (192, 0, 0) whether or not the shadow lies beneath it
    let paper = Drawing::from(rect(0.0, 0.0, 60.0, 60.0)?).fill(color("#c0c0c0")?);
    let ink = shape(
        rect(20.0, 20.0, 20.0, 20.0)?,
        Shadow::new().dx(10.0).dy(0.0).blur(0.0).color(color("#00000080")?),
    )
    .blend(Blend::Multiply);
    let image = render(&Drawing::new([paper, ink]), 60, 60)?;
    assert_eq!(image.pixel(10, 30), [192, 192, 192, 255], "paper");
    assert_eq!(image.pixel(25, 30), [192, 0, 0, 255], "the square alone");
    assert_eq!(image.pixel(35, 30), [192, 0, 0, 255], "the square over its shadow");
    for channel in 0..3 {
        assert!(
            image.pixel(45, 30)[channel].abs_diff(96) <= 1,
            "the shadow alone: {:?}",
            image.pixel(45, 30)
        );
    }
    assert_eq!(image.pixel(45, 30)[3], 255);
    Ok(())
}

// The silhouette ----------------------------------------------------------------------------------------------------

#[test]
fn the_shadow_is_the_silhouette_of_the_fill_and_the_stroke_once() -> Outcome {
    // a square filled red and stroked blue 10 wide, so that the stroke covers 15 to 25 and 55 to 65 and overlaps the
    // fill; a black shadow of alpha 0.5 moved 60 down: where the stroke overlaps the fill (row 80) the shadow is no
    // darker than where the fill is alone (row 100) or the stroke alone (row 77)
    let drawing = Drawing::from(rect(20.0, 20.0, 40.0, 40.0)?)
        .fill(RED)
        .stroke(Stroke::new(color("#0000ff")?).width(10.0).cap(Cap::Butt))
        .shadow(Shadow::new().dy(60.0).blur(0.0).color(color("#00000080")?));
    let image = render(&drawing, 80, 140)?;
    for (case, y) in [
        ("the stroke alone", 77),
        ("stroke over fill", 80),
        ("the fill alone", 100),
    ] {
        assert_eq!(image.pixel(40, y), [0, 0, 0, 128], "{case}");
    }
    // and none beside the silhouette
    assert_eq!(image.pixel(40, 70), [0, 0, 0, 0], "between the shape and its shadow");
    assert_eq!(image.pixel(12, 100), [0, 0, 0, 0], "beside the stroke");
    Ok(())
}

#[test]
fn the_alpha_of_the_shadow_is_the_alpha_of_the_shape_times_the_tint() -> Outcome {
    // the shape is half transparent, the tint opaque: the shadow, 20 to the right and clear of the shape, is half
    let drawing = Drawing::from(rect(10.0, 10.0, 20.0, 20.0)?)
        .fill(color("#ff000080")?)
        .shadow(Shadow::new().dx(20.0).dy(0.0).blur(0.0).color(color("#000000")?));
    let image = render(&drawing, 60, 60)?;
    assert_eq!(image.pixel(40, 20), [0, 0, 0, 128]);
    assert_eq!(image.pixel(20, 20), [255, 0, 0, 128]);
    Ok(())
}

#[test]
fn a_shape_that_paints_nothing_or_has_no_path_casts_no_shadow() -> Outcome {
    let shadow = || Shadow::new().blur(8.0).color(Color::hex(0));
    let rows = [
        (
            "no fill and no stroke",
            Drawing::from(rect(10.0, 10.0, 20.0, 20.0)?)
                .fill(Color::NONE)
                .shadow(shadow()),
        ),
        ("an empty path", Drawing::from(Path::new()).fill(RED).shadow(shadow())),
        (
            "an opacity of 0",
            shape(rect(10.0, 10.0, 20.0, 20.0)?, shadow()).opacity(0.0),
        ),
    ];
    for (case, drawing) in rows {
        let image = render(&drawing, 60, 60)?;
        assert!(image.all_pixels_are([0, 0, 0, 0]), "{case}");
    }
    Ok(())
}

#[test]
fn a_shape_off_the_canvas_casts_its_shadow_onto_it() -> Outcome {
    // a square 1000 to the left of the canvas, its shadow moved 1010 right: a square on columns 10 to 30, rows 40 to 60;
    // blurred with a deviation of 4 it follows the Gaussian
    let path = rect(-1000.0, 40.0, 20.0, 20.0)?;
    let hard = render(
        &shape(
            path.clone(),
            Shadow::new().dx(1010.0).dy(0.0).blur(0.0).color(color("#000000")?),
        ),
        60,
        100,
    )?;
    assert_eq!(hard.pixel(20, 50), [0, 0, 0, 255]);
    assert_eq!(hard.pixel(5, 50), [0, 0, 0, 0]);
    let soft = render(
        &shape(
            path,
            Shadow::new().dx(1010.0).dy(0.0).blur(8.0).color(color("#000000")?),
        ),
        60,
        100,
    )?;
    for (x, y) in [(5, 50), (8, 50), (25, 50), (40, 50), (20, 38), (20, 62)] {
        let wanted = 255.0 * between(px(x) + 0.5, 4.0, 10.0, 30.0) * between(px(y) + 0.5, 4.0, 40.0, 60.0);
        let alpha = f64::from(soft.alpha(x, y));
        assert!(
            (alpha - wanted).abs() <= 2.0,
            "({x}, {y}): alpha {alpha}, the Gaussian says {wanted:.2}"
        );
    }
    Ok(())
}

// Limits ------------------------------------------------------------------------------------------------------------

fn pixels(drawing: &Drawing, side: f64) -> Result<Vec<u8>, Error> {
    drawing.to_rgba(Size::px(side, side), &Values::new())
}

#[test]
fn a_deviation_past_a_thousand_output_pixels_is_refused_naming_the_blur() -> Outcome {
    // the shape is far off the canvas, so that what is accepted costs nothing to draw. (blur, output side, accepted)
    let rows = [
        (2000.0, 100.0, true),
        (2000.5, 100.0, false),
        (4000.0, 100.0, false),
        (1000.0, 200.0, true),
        (1000.5, 200.0, false),
        (4000.0, 50.0, true),
    ];
    for (blur, side, accepted) in rows {
        let drawing = shape(rect(-90_000.0, 0.0, 10.0, 10.0)?, Shadow::new().blur(blur)).with_size(100.0, 100.0);
        let outcome = pixels(&drawing, side);
        if accepted {
            outcome?;
        } else {
            let (name, message) = parameter_refusal(outcome)?;
            assert!(
                name == "shape 0" && message.contains("blur of ") && message.contains(&format!("{blur} design")),
                "blur {blur} at {side}: {name}: {message}"
            );
        }
    }
    Ok(())
}

#[test]
fn an_offset_beyond_ten_million_output_pixels_is_refused() -> Outcome {
    let drawing = shape(rect(0.0, 0.0, 10.0, 10.0)?, Shadow::new().dx(1.0e7 + 1.0).blur(0.0));
    let (name, message) = parameter_refusal(pixels(&drawing, 100.0))?;
    assert!(name == "shape 0" && message.contains("offset"), "{name}: {message}");
    Ok(())
}

#[test]
fn the_layers_a_draw_blurs_are_counted_against_a_limit_of_134_million_pixels() -> Outcome {
    // a deviation of 1000 and an offset of 5000 reach 8000 pixels each way: a layer of 16,100 pixels a side, 259 million
    let blurred = |dx: f64, count: usize| {
        let each = shape(rect(40.0, 40.0, 10.0, 10.0)?, Shadow::new().dx(dx).blur(2000.0));
        Ok::<_, Error>(Drawing::new(vec![each; count]))
    };
    let (name, message) = parameter_refusal(pixels(&blurred(5000.0, 1)?, 100.0))?;
    assert!(
        name == "shape 0" && message.contains("MAX_FILTER_PIXELS 134217728"),
        "{name}: {message}"
    );
    // an offset of 1472 reaches 4472 each way, 8,964 a side, 80 million a layer: one fits and the second does not
    let (name, message) = parameter_refusal(pixels(&blurred(1472.0, 2)?, 100.0))?;
    assert!(
        name == "shape 1" && message.contains("MAX_FILTER_PIXELS"),
        "{name}: {message}"
    );
    Ok(())
}
