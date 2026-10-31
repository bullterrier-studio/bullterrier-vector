//! How `Drawing::to_rgba` lays shapes over each other: normal and multiply, opacity, and a shape's fill and stroke as
//! one group. The oracle is the W3C Compositing formulas worked in f64 in this file, over rectangles whose edges fall
//! on pixel boundaries so that every pixel is wholly in or out of each.

mod common;

use bullterrier_vector::{Blend, Cap, Color, Drawing, Join, Path, Stroke};
use common::Outcome;
use common::pixels::{Image, render};
use proptest::prelude::*;

const SIDE: usize = 12;

/// One rectangle of a random stack: its pixel columns and rows, fill, opacity and blend.
#[derive(Debug, Clone)]
struct Layer {
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    rgba: [u8; 4],
    opacity: f64,
    multiply: bool,
}

fn layers() -> impl Strategy<Value = Vec<Layer>> {
    let layer = (
        0..SIDE,
        0..SIDE,
        1..=SIDE,
        1..=SIDE,
        any::<[u8; 4]>(),
        0.0f64..=1.0,
        any::<bool>(),
    )
        .prop_map(|(left, top, width, height, rgba, opacity, multiply)| Layer {
            left,
            top,
            width,
            height,
            rgba,
            opacity,
            multiply,
        });
    prop::collection::vec(layer, 1..=10)
}

fn fail(error: impl std::fmt::Display) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

/// The drawing of `layers`, each a filled rectangle on pixel boundaries.
fn drawing(layers: &[Layer]) -> Result<Drawing, bullterrier_vector::Error> {
    let mut items = Vec::new();
    for layer in layers {
        #[expect(clippy::cast_precision_loss, reason = "small pixel indices")]
        let rect = Path::rect(
            layer.left as f64,
            layer.top as f64,
            layer.width as f64,
            layer.height as f64,
            0.0,
        )?;
        let [red, green, blue, alpha] = layer.rgba;
        let color = Color::parse(&format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}"))?;
        let mode = if layer.multiply { Blend::Multiply } else { Blend::Normal };
        items.push(Drawing::from(rect).fill(color).opacity(layer.opacity).blend(mode));
    }
    Ok(Drawing::new(items))
}

/// What the layers leave on the pixel in column `x` and row `y`: its premultiplied colour (0..=255 per channel) and
/// alpha (0..=255), by the W3C formulas, and how many layers covered it.
fn reference(layers: &[Layer], x: usize, y: usize) -> ([f64; 3], f64, usize) {
    // the backdrop, as straight colour (0..=1) and alpha (0..=1)
    let (mut backdrop, mut backdrop_alpha, mut covering) = ([0.0; 3], 0.0, 0);
    for layer in layers {
        let covers =
            (layer.left..layer.left + layer.width).contains(&x) && (layer.top..layer.top + layer.height).contains(&y);
        if !covers {
            continue;
        }
        covering += 1;
        let source = [0, 1, 2].map(|channel| f64::from(layer.rgba[channel]) / 255.0);
        let alpha = f64::from(layer.rgba[3]) / 255.0 * layer.opacity;
        // co = as (1 - ab) Cs + as ab B(Cb, Cs) + (1 - as) ab Cb, then back to a straight colour
        let result_alpha = alpha + backdrop_alpha - alpha * backdrop_alpha;
        let mut premultiplied = [0.0; 3];
        for channel in 0..3 {
            let blended = if layer.multiply {
                backdrop[channel] * source[channel]
            } else {
                source[channel]
            };
            premultiplied[channel] = alpha * (1.0 - backdrop_alpha) * source[channel]
                + alpha * backdrop_alpha * blended
                + (1.0 - alpha) * backdrop_alpha * backdrop[channel];
        }
        if result_alpha > 0.0 {
            backdrop = premultiplied.map(|value| value / result_alpha);
        }
        backdrop_alpha = result_alpha;
    }
    (
        backdrop.map(|value| value * backdrop_alpha * 255.0),
        backdrop_alpha * 255.0,
        covering,
    )
}

/// The premultiplied colour, 0..=255 per channel, a straight-alpha pixel stands for.
fn premultiplied(pixel: [u8; 4]) -> [f64; 3] {
    [0, 1, 2].map(|channel| f64::from(pixel[channel]) * f64::from(pixel[3]) / 255.0)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn a_stack_of_translucent_shapes_matches_the_w3c_formulas_to_two_levels_a_layer(layers in layers()) {
        let image = render(&drawing(&layers).map_err(fail)?, SIDE, SIDE).map_err(fail)?;
        for y in 0..SIDE {
            for x in 0..SIDE {
                let (colour, alpha, covering) = reference(&layers, x, y);
                // the rasteriser works in 8 bits and rounds each layer's colour and then its opacity: measured over
                // thousands of stacks, the error stays within two levels for each layer covering the pixel
                #[expect(clippy::cast_precision_loss, reason = "at most ten layers")]
                let allowed = 2.0 * covering as f64 + 1.0;
                let pixel = image.pixel(x, y);
                prop_assert!(
                    (f64::from(pixel[3]) - alpha).abs() <= allowed,
                    "alpha at column {} row {}: got {}, expected {} within {} ({:?})", x, y, pixel[3], alpha, allowed, layers
                );
                for (got, want) in premultiplied(pixel).into_iter().zip(colour) {
                    prop_assert!(
                        (got - want).abs() <= allowed,
                        "colour at column {} row {}: got {:?}, expected {:?} of alpha {} within {} ({:?})",
                        x, y, pixel, colour, alpha, allowed, layers
                    );
                }
            }
        }
    }
}

fn near(got: u8, want: u8, within: i32) -> bool {
    (i32::from(got) - i32::from(want)).abs() <= within
}

/// A solid rectangle of columns and rows `[from, to)` in `colour`, with `opacity` and `blend`.
fn block(from: f64, to: f64, colour: &str, opacity: f64, blend: Blend) -> Result<Drawing, Box<dyn std::error::Error>> {
    let rect = Path::rect(from, from, to - from, to - from, 0.0)?;
    Ok(Drawing::from(rect)
        .fill(Color::parse(colour)?)
        .opacity(opacity)
        .blend(blend))
}

#[test]
fn multiply_darkens_by_the_product_and_on_nothing_it_is_the_colour_itself() -> Outcome {
    // (case, backdrop, multiplied colour, expected pixel where the two overlap), worked by hand: 200 x 128 / 255 = 100.4
    let rows = [
        ("opaque over opaque", "#c86432", "#80ff00", [100, 100, 0, 255]),
        ("white changes nothing", "#c86432", "#ffffff", [200, 100, 50, 255]),
        ("black is black", "#c86432", "#000000", [0, 0, 0, 255]),
    ];
    for (case, backdrop, source, expected) in rows {
        let drawing = Drawing::new([
            block(0.0, 8.0, backdrop, 1.0, Blend::Normal)?,
            block(4.0, 12.0, source, 1.0, Blend::Multiply)?,
        ]);
        let image = render(&drawing, 12, 12)?;
        let got = image.pixel(6, 6);
        assert!(
            expected.iter().zip(got).all(|(want, got)| near(got, *want, 1)),
            "{case}: {got:?}"
        );
        // where the multiplied shape has nothing under it, it is itself
        let [red, green, blue, _] = Color::parse(source)?.rgba().ok_or("no colour")?;
        assert_eq!(image.pixel(10, 10), [red, green, blue, 255], "{case}: over nothing");
    }
    Ok(())
}

#[test]
fn opacity_is_the_alpha_of_the_whole_shape_and_zero_draws_nothing() -> Outcome {
    let half = render(&block(2.0, 10.0, "#336699", 0.5, Blend::Normal)?, 12, 12)?;
    assert!(near(half.alpha(5, 5), 128, 1));
    assert!(
        [0x33, 0x66, 0x99]
            .iter()
            .zip(half.pixel(5, 5))
            .all(|(want, got)| near(got, *want, 1)),
        "{:?}",
        half.pixel(5, 5)
    );
    let none = render(&block(2.0, 10.0, "#336699", 0.0, Blend::Normal)?, 12, 12)?;
    assert!(none.bytes.iter().all(|&byte| byte == 0));
    // opacity multiplies the alpha of the paint
    let both = render(&block(2.0, 10.0, "#33669980", 0.5, Blend::Normal)?, 12, 12)?;
    assert!(near(both.alpha(5, 5), 64, 1), "{}", both.alpha(5, 5));
    Ok(())
}

/// A 20 by 20 square from (10, 10), filled `fill`, stroked 10 wide with mitres (so that the stroke covers the band
/// from 5 to 15 on each side of its edges), with `opacity`.
fn framed(fill: &str, stroke: &str, opacity: f64) -> Result<Image, Box<dyn std::error::Error>> {
    let square = Path::rect(10.0, 10.0, 20.0, 20.0, 0.0)?;
    let outline = Stroke::new(Color::parse(stroke)?)
        .width(10.0)
        .join(Join::Miter)
        .cap(Cap::Butt);
    let drawing = Drawing::from(square)
        .fill(Color::parse(fill)?)
        .stroke(outline)
        .opacity(opacity);
    Ok(render(&drawing, 40, 40)?)
}

#[test]
fn a_shapes_fill_and_stroke_are_one_group_that_opacity_applies_to_once() -> Outcome {
    // the stroke band covers 5..15 and 25..35 across; the fill covers 10..30, so 10..15 and 25..30 hold both and
    // 15..25 only the fill, 5..10 and 30..35 only the stroke. Probed: (12,20) both, (20,20) fill only, (7,20) stroke
    // only, (2,20) neither.
    // (case, fill, stroke, opacity, expected pixel at both, fill only, stroke only)
    let rows = [
        (
            "opaque, opacity 0.5: the stroke covers the fill, alpha stays 0.5",
            "#ff0000",
            "#0000ff",
            0.5,
            ([0, 0, 255, 128], [255, 0, 0, 128], [0, 0, 255, 128]),
        ),
        (
            "paint alpha 0.5, opacity 1: the stroke over the fill makes 0.75",
            "#ff000080",
            "#0000ff80",
            1.0,
            ([85, 0, 170, 191], [255, 0, 0, 128], [0, 0, 255, 128]),
        ),
        (
            "paint alpha 0.5, opacity 0.5: the group, then half of it",
            "#ff000080",
            "#0000ff80",
            0.5,
            ([85, 0, 170, 96], [255, 0, 0, 64], [0, 0, 255, 64]),
        ),
    ];
    for (case, fill, stroke, opacity, (both, fill_only, stroke_only)) in rows {
        let image = framed(fill, stroke, opacity)?;
        for (place, (x, y), expected) in [
            ("both", (12, 20), both),
            ("fill only", (20, 20), fill_only),
            ("stroke only", (7, 20), stroke_only),
        ] {
            let got = image.pixel(x, y);
            // the colour is compared premultiplied: a straight colour of a faint pixel is coarse
            let wanted = premultiplied(expected);
            assert!(
                near(got[3], expected[3], 2)
                    && premultiplied(got)
                        .into_iter()
                        .zip(wanted)
                        .all(|(got, want)| (got - want).abs() <= 2.0),
                "{case}: {place}: got {got:?}, expected {expected:?}"
            );
        }
        assert_eq!(image.alpha(2, 20), 0, "{case}: neither");
    }
    Ok(())
}

#[test]
fn a_filled_and_stroked_shape_at_half_opacity_is_as_even_as_one_shape_of_the_union() -> Outcome {
    // opaque paint at opacity 0.5: every pixel of the union of fill and stroke is 128, the overlap no darker
    let image = framed("#ff0000", "#0000ff", 0.5)?;
    for y in 5..35 {
        for x in 5..35 {
            assert!(
                near(image.alpha(x, y), 128, 1),
                "column {x} row {y}: {}",
                image.alpha(x, y)
            );
        }
    }
    Ok(())
}

#[test]
fn a_multiplied_shape_multiplies_with_its_fill_and_stroke_as_one() -> Outcome {
    // a grey backdrop, 0xc0, then a shape of fill 0x80 and stroke 0x40 multiplied: where fill and stroke overlap only
    // the stroke shows, and it multiplies once: 0xc0 x 0x40 / 255 = 48.2; fill alone: 0xc0 x 0x80 / 255 = 96.4
    let backdrop = block(0.0, 40.0, "#c0c0c0", 1.0, Blend::Normal)?;
    let square = Path::rect(10.0, 10.0, 20.0, 20.0, 0.0)?;
    let outline = Stroke::new(Color::parse("#404040")?)
        .width(10.0)
        .join(Join::Miter)
        .cap(Cap::Butt);
    let shape = Drawing::from(square)
        .fill(Color::parse("#808080")?)
        .stroke(outline)
        .blend(Blend::Multiply);
    let image = render(&Drawing::new([backdrop, shape]), 40, 40)?;
    assert!(near(image.pixel(12, 20)[0], 48, 1), "both: {:?}", image.pixel(12, 20));
    assert!(
        near(image.pixel(20, 20)[0], 96, 1),
        "fill only: {:?}",
        image.pixel(20, 20)
    );
    assert!(
        near(image.pixel(7, 20)[0], 48, 1),
        "stroke only: {:?}",
        image.pixel(7, 20)
    );
    assert_eq!(image.pixel(2, 20), [192, 192, 192, 255], "neither");
    Ok(())
}
