//! `to_rgba_window` and `to_png_window`: a window of the output has the pixels of the same rows and columns of the whole
//! render. The oracle is the whole render itself (`to_rgba`), cut by hand; windows are 1-pixel ones, ones on each edge of
//! the canvas, ones at offsets that are no multiple of a tile or a blur's decimation, and a seeded random spread, over a
//! drawing for each effect. A band loop that stitches a 4 by 3 grid of windows gives the whole render back.

use crate::common::fonts::fraunces;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{
    Blend, Color, Deg, Drawing, Error, Gradient, Halftone, Path, Shadow, Size, Stroke, Text, Values, Window, text,
};

/// The size of the whole output: not a multiple of any tile, and not the frame's shape, so that the view has an offset.
const WIDTH: usize = 331;
const HEIGHT: usize = 251;

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(x, y, width, height, 0.0)
}

fn solid(path: Path, fill: &str) -> Result<Drawing, Error> {
    Ok(Drawing::from(path).fill(color(fill)?))
}

fn fade() -> Result<Gradient, Error> {
    Gradient::linear(
        (20.0, 0.0),
        (180.0, 0.0),
        [(0.0, color("#000000")?), (1.0, color("#ffffff")?)],
    )
}

/// The drawings: (what it exercises, the most a window's pixel may differ from the whole's, drawing), each in a frame of
/// 200 by 150 design pixels. The tolerances are what 400 random windows over each showed (`docs/design-decisions.md`,
/// "Render a window (V10e)"): a level or two where the edge of a shape is placed from f32 coordinates a window moves.
fn corpus() -> Result<Vec<(&'static str, u8, Drawing)>, Error> {
    let font = fraunces()?;
    let paper = || solid(rect(0.0, 0.0, 200.0, 150.0)?, "#e8a060");
    let rows = vec![
        (
            "fill and stroke",
            1,
            Drawing::new([
                paper()?,
                Drawing::from(Path::star((100.0, 75.0), 60.0, 25.0, 5)?)
                    .fill(color("#3070c0")?)
                    .stroke(Stroke::new(color("#102040c0")?).width(7.0)),
                solid(Path::circle((40.0, 110.0), 20.0)?, "#c03030")?.opacity(0.6),
            ]),
        ),
        (
            "shadow with a blur",
            0,
            Drawing::new([
                paper()?,
                solid(rect(50.0, 40.0, 70.0, 50.0)?, "#3070c0")?
                    .shadow(Shadow::new().dx(8.0).dy(12.0).blur(24.0).color(color("#000000b0")?)),
            ]),
        ),
        (
            "shadow with a wide blur at the canvas edge",
            0,
            Drawing::new([
                paper()?,
                solid(rect(150.0, 100.0, 60.0, 50.0)?, "#3070c0")?
                    .shadow(Shadow::new().dx(-14.0).dy(-6.0).blur(90.0).color(color("#000000")?)),
            ]),
        ),
        (
            "shadow without a blur",
            0,
            Drawing::new([
                paper()?,
                solid(Path::circle((100.0, 75.0), 30.0)?, "#3070c0")?
                    .shadow(Shadow::new().dx(5.5).dy(3.25).blur(0.0).color(color("#00000080")?)),
            ]),
        ),
        (
            "shadow of a stroke under opacity",
            1,
            Drawing::new([
                paper()?,
                Drawing::from(Path::circle((100.0, 75.0), 30.0)?)
                    .stroke(Stroke::new(color("#3070c0")?).width(9.0))
                    .opacity(0.7)
                    .shadow(Shadow::new().dx(0.0).dy(9.0).blur(18.0).color(color("#000000")?)),
            ]),
        ),
        (
            "mask",
            1,
            Drawing::new([
                paper()?,
                solid(Path::circle((100.0, 75.0), 60.0)?, "#3070c0")?
                    .mask(Drawing::from(rect(0.0, 0.0, 200.0, 150.0)?).fill(fade()?)),
            ]),
        ),
        (
            "mask with a blurred shadow in it",
            0,
            Drawing::new([
                paper()?,
                solid(rect(0.0, 0.0, 200.0, 150.0)?, "#3070c0")?.mask(
                    solid(Path::circle((100.0, 75.0), 40.0)?, "#ffffff")?
                        .shadow(Shadow::new().dx(10.0).dy(10.0).blur(30.0).color(color("#ffffff")?)),
                ),
            ]),
        ),
        (
            "clip",
            0,
            Drawing::new([
                paper()?,
                solid(Path::circle((100.0, 75.0), 60.0)?, "#3070c0")?
                    .shadow(Shadow::new().dx(6.0).dy(6.0).blur(14.0).color(color("#000000")?))
                    .clip(Path::circle((80.0, 60.0), 45.0)?),
            ]),
        ),
        (
            "gradients",
            1,
            Drawing::new([
                Drawing::from(rect(0.0, 0.0, 200.0, 150.0)?).fill(fade()?),
                Drawing::from(Path::circle((100.0, 75.0), 50.0)?)
                    .fill(Gradient::radial(
                        (100.0, 75.0),
                        50.0,
                        [(0.0, color("#ff0000")?), (1.0, color("#0000ff40")?)],
                    )?)
                    .stroke(Stroke::new(fade()?).width(10.0)),
            ]),
        ),
        (
            "blends",
            2,
            Drawing::new([
                paper()?,
                solid(Path::circle((80.0, 75.0), 50.0)?, "#3070c0")?.blend(Blend::Multiply),
                solid(Path::circle((120.0, 75.0), 50.0)?, "#c07030")?.blend(Blend::Screen),
                solid(rect(60.0, 20.0, 80.0, 30.0)?, "#808080")?
                    .blend(Blend::Difference)
                    .opacity(0.8),
            ]),
        ),
        (
            "knockout",
            0,
            Drawing::new([
                paper()?,
                solid(Path::star((100.0, 75.0), 55.0, 20.0, 6)?, "#000000")?.knockout(),
            ]),
        ),
        (
            "halftone",
            1,
            Drawing::new([
                paper()?,
                Drawing::from(Path::circle((100.0, 75.0), 60.0)?)
                    .halftone(Halftone::new(color("#102040")?, 0.4).pitch(7.0).angle(Deg(30.0)))
                    .stroke(Stroke::new(color("#102040")?).width(3.0)),
            ]),
        ),
        (
            "text",
            0,
            Drawing::new([
                paper()?,
                text("HIFCTAVeflo", Text::new(&font).size(34.0).width(180.0))
                    .translate(10.0, 40.0)
                    .fill(color("#102040")?)
                    .shadow(Shadow::new().dx(2.0).dy(4.0).blur(8.0).color(color("#00000090")?)),
            ]),
        ),
    ];
    Ok(rows
        .into_iter()
        .map(|(name, tolerance, drawing)| (name, tolerance, drawing.with_size(200.0, 150.0)))
        .collect())
}

fn size() -> Size {
    #[expect(clippy::cast_precision_loss, reason = "small")]
    Size::px(WIDTH as f64, HEIGHT as f64)
}

fn whole(drawing: &Drawing) -> Result<Vec<u8>, Error> {
    drawing.to_rgba(size(), &Values::new())
}

fn window(drawing: &Drawing, (x, y, width, height): (usize, usize, usize, usize)) -> Result<Vec<u8>, Error> {
    #[expect(clippy::cast_precision_loss, reason = "small")]
    let window = Window::px(x as f64, y as f64, width as f64, height as f64);
    drawing.to_rgba_window(size(), window, &Values::new())
}

/// The rows `y..y + height` and columns `x..x + width` of the whole picture `bytes`.
fn cut(bytes: &[u8], (x, y, width, height): (usize, usize, usize, usize)) -> Vec<u8> {
    (y..y + height)
        .flat_map(|row| {
            bytes[(row * WIDTH + x) * 4..(row * WIDTH + x + width) * 4]
                .iter()
                .copied()
        })
        .collect()
}

/// `[red, green, blue, alpha]` as the product of each colour with the alpha, as a faint pixel's colour is only known
/// to within what its alpha leaves of it (a straight colour moves by several levels for one level of the product).
fn premultiplied(pixel: &[u8]) -> [u8; 4] {
    let alpha = u16::from(pixel[3]);
    #[expect(clippy::cast_possible_truncation, reason = "at most 255")]
    let times = |channel: u8| ((u16::from(channel) * alpha + 127) / 255) as u8;
    [times(pixel[0]), times(pixel[1]), times(pixel[2]), pixel[3]]
}

/// How the window `at` differs from the whole picture beyond the tolerance: nothing, or the count of pixels that differ,
/// the most they differ by and the first pixel's index.
fn difference(
    whole: &[u8],
    (drawing, tolerance): (&Drawing, u8),
    at: (usize, usize, usize, usize),
) -> Result<Option<String>, Error> {
    let expected = cut(whole, at);
    let got = window(drawing, at)?;
    if got.len() != expected.len() {
        return Ok(Some(format!(
            "{at:?}: {} bytes, expected {}",
            got.len(),
            expected.len()
        )));
    }
    let mut count = 0;
    let mut most = 0;
    let mut first = None;
    for (index, (a, b)) in got.chunks_exact(4).zip(expected.chunks_exact(4)).enumerate() {
        let off = premultiplied(a)
            .iter()
            .zip(premultiplied(b))
            .map(|(x, y)| x.abs_diff(y))
            .max()
            .unwrap_or(0);
        if off > tolerance {
            count += 1;
            most = most.max(off);
            first.get_or_insert(index);
        }
    }
    Ok(first.map(|index| format!("{at:?}: {count} pixels differ by up to {most} levels, the first at pixel {index}")))
}

/// A deterministic spread: a linear congruential generator, so that a failure is the same on every run.
fn windows() -> Vec<(usize, usize, usize, usize)> {
    let mut windows = vec![
        (0, 0, 1, 1),
        (WIDTH - 1, HEIGHT - 1, 1, 1),
        (WIDTH - 1, 0, 1, 1),
        (0, HEIGHT - 1, 1, 1),
        (123, 77, 1, 1),
        (0, 0, WIDTH, 1),
        (0, HEIGHT - 1, WIDTH, 1),
        (0, 0, 1, HEIGHT),
        (WIDTH - 1, 0, 1, HEIGHT),
        (0, 0, WIDTH, HEIGHT),
        (1, 1, WIDTH - 2, HEIGHT - 2),
        (3, 5, 64, 48),
        (131, 99, 67, 61),
        (WIDTH - 17, HEIGHT - 13, 17, 13),
        (255, 255 - 100, 41, 33),
        (257, 129, 33, 17),
    ];
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = |below: usize| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[expect(clippy::cast_possible_truncation, reason = "the top bits of a 64-bit state")]
        let high = (state >> 33) as usize;
        high % below
    };
    for _ in 0..40 {
        let (width, height) = (1 + next(WIDTH.min(120)), 1 + next(HEIGHT.min(120)));
        windows.push((next(WIDTH - width + 1), next(HEIGHT - height + 1), width, height));
    }
    windows
}

#[test]
fn a_window_has_the_pixels_of_the_same_rows_and_columns_of_the_whole_render() -> Outcome {
    let mut wrong = Vec::new();
    for (name, tolerance, drawing) in corpus()? {
        let whole = whole(&drawing)?;
        for at in windows() {
            if let Some(found) = difference(&whole, (&drawing, tolerance), at)? {
                wrong.push(format!("{name}: {found}"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} windows differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    Ok(())
}

#[test]
fn a_grid_of_four_by_three_windows_stitches_into_the_whole_render() -> Outcome {
    let (columns, rows) = (4, 3);
    let (band_width, band_height) = (WIDTH.div_ceil(columns), HEIGHT.div_ceil(rows));
    for (name, tolerance, drawing) in corpus()? {
        let mut stitched = vec![0_u8; WIDTH * HEIGHT * 4];
        for row in 0..rows {
            for column in 0..columns {
                let (x, y) = (column * band_width, row * band_height);
                let (width, height) = (band_width.min(WIDTH - x), band_height.min(HEIGHT - y));
                let part = window(&drawing, (x, y, width, height))?;
                for line in 0..height {
                    let to = ((y + line) * WIDTH + x) * 4;
                    stitched[to..to + width * 4].copy_from_slice(&part[line * width * 4..(line + 1) * width * 4]);
                }
            }
        }
        let whole = whole(&drawing)?;
        let first = stitched.chunks_exact(4).zip(whole.chunks_exact(4)).position(|(a, b)| {
            premultiplied(a)
                .iter()
                .zip(premultiplied(b))
                .any(|(x, y)| x.abs_diff(y) > tolerance)
        });
        assert_eq!(
            first, None,
            "{name}: the stitched grid differs from the whole at this pixel"
        );
    }
    Ok(())
}

#[test]
fn a_png_window_is_a_file_of_the_window_size() -> Outcome {
    let drawing = Drawing::from(Path::circle((100.0, 75.0), 40.0)?)
        .fill(color("#3070c0")?)
        .with_size(200.0, 150.0);
    let file = drawing.to_png_window(size(), Window::px(10.0, 20.0, 30.0, 40.0), &Values::new())?;
    let be32 = |at: usize| u32::from_be_bytes([file[at], file[at + 1], file[at + 2], file[at + 3]]);
    assert_eq!((be32(16), be32(20)), (30, 40));
    Ok(())
}

#[test]
fn a_window_outside_the_canvas_or_not_whole_is_refused_naming_window() -> Outcome {
    let drawing = Drawing::from(Path::circle((100.0, 75.0), 40.0)?).with_size(200.0, 150.0);
    // (case, x, y, width, height)
    let rows = [
        ("past the right edge", 300.0, 0.0, 32.0, 1.0),
        ("past the bottom edge", 0.0, 250.0, 1.0, 2.0),
        ("negative x", -1.0, 0.0, 5.0, 5.0),
        ("negative y", 0.0, -1.0, 5.0, 5.0),
        ("zero width", 0.0, 0.0, 0.0, 5.0),
        ("zero height", 0.0, 0.0, 5.0, 0.0),
        ("half a pixel", 0.5, 0.0, 5.0, 5.0),
        ("not a number", f64::NAN, 0.0, 5.0, 5.0),
        ("infinite width", 0.0, 0.0, f64::INFINITY, 5.0),
    ];
    for (case, x, y, width, height) in rows {
        let (parameter, _) =
            parameter_refusal(drawing.to_rgba_window(size(), Window::px(x, y, width, height), &Values::new()))?;
        assert_eq!(parameter, "window", "{case}");
        let (parameter, _) =
            parameter_refusal(drawing.to_png_window(size(), Window::px(x, y, width, height), &Values::new()))?;
        assert_eq!(parameter, "window", "{case}");
    }
    Ok(())
}

// Limits ------------------------------------------------------------------------------------------------------------

/// A drawing near the bottom right of a frame of 100,000 design pixels: a blurred shadow, a mask and a clip, each
/// inside the 1,000 pixel square that starts at (99,000, 99,000), moved by `by` design pixels.
fn far_corner(by: f64) -> Result<Drawing, Error> {
    let at = |x: f64, y: f64| (99_000.0 + x + by, 99_000.0 + y + by);
    let (x, y) = at(300.0, 300.0);
    let (mx, my) = at(600.0, 500.0);
    let (cx, cy) = at(250.0, 600.0);
    Ok(Drawing::new([
        solid(rect(x, y, 200.0, 150.0)?, "#3070c0")?
            .shadow(Shadow::new().dx(20.0).dy(30.0).blur(80.0).color(color("#000000b0")?)),
        solid(Path::circle((mx, my), 120.0)?, "#c07030")?.mask(Drawing::from(Path::circle((mx, my), 100.0)?).fill(
            Gradient::linear(
                (mx - 100.0, my),
                (mx + 100.0, my),
                [(0.0, color("#000000")?), (1.0, color("#ffffff")?)],
            )?,
        )),
        solid(rect(cx, cy, 200.0, 200.0)?, "#30c070")?.clip(Path::circle((cx + 100.0, cy + 100.0), 90.0)?),
    ]))
}

#[test]
fn a_small_window_of_the_largest_canvas_is_the_picture_translated_to_the_origin() -> Outcome {
    // the same drawing moved to the top left of a canvas of 1,000 pixels at the same scale is drawn whole: the oracle
    // for a canvas that is 10^10 pixels, which no whole render could be
    let big = far_corner(0.0)?.with_size(100_000.0, 100_000.0);
    let small = far_corner(-99_000.0)?.with_size(1000.0, 1000.0);
    let near = small.to_rgba(Size::px(1000.0, 1000.0), &Values::new())?;
    assert!(
        near.chunks_exact(4).any(|pixel| pixel[3] > 0),
        "the drawing is not blank"
    );
    let window = Window::px(99_000.0, 99_000.0, 1000.0, 1000.0);
    let far = big.to_rgba_window(Size::px(100_000.0, 100_000.0), window, &Values::new())?;
    assert_eq!(far.len(), near.len());
    let worst = far
        .chunks_exact(4)
        .zip(near.chunks_exact(4))
        .map(|(a, b)| {
            premultiplied(a)
                .iter()
                .zip(premultiplied(b))
                .map(|(x, y)| x.abs_diff(y))
                .max()
                .unwrap_or(0)
        })
        .max();
    assert!(
        worst <= Some(2),
        "the window differs from the translated picture by {worst:?} levels"
    );
    Ok(())
}

#[test]
fn a_window_past_the_limits_of_a_pixel_output_is_refused_naming_window() -> Outcome {
    let drawing = Drawing::from(Path::circle((100.0, 75.0), 40.0)?).with_size(200.0, 150.0);
    let big = Size::px(100_000.0, 100_000.0);
    // (case, width, height)
    let rows = [
        ("one past the side limit wide", 65_001.0, 10.0),
        ("one past the side limit high", 10.0, 65_001.0),
        ("one column past 2^27 pixels", 16_385.0, 8192.0),
    ];
    for (case, width, height) in rows {
        let (parameter, _) =
            parameter_refusal(drawing.to_rgba_window(big, Window::px(0.0, 0.0, width, height), &Values::new()))?;
        assert_eq!(parameter, "window", "{case}");
    }
    Ok(())
}

#[test]
fn a_window_whose_shadows_reach_past_the_limits_is_refused_naming_window() -> Outcome {
    // a window of 10,000 pixels square (10^8 pixels, within the limit) with a blur that reaches 2,000 pixels around it
    let drawing = solid(rect(0.0, 0.0, 50_000.0, 50_000.0)?, "#000000")?
        .shadow(Shadow::new().blur(2000.0).color(color("#000000")?))
        .with_size(100_000.0, 100_000.0);
    let (parameter, _) = parameter_refusal(drawing.to_rgba_window(
        Size::px(100_000.0, 100_000.0),
        Window::px(20_000.0, 20_000.0, 10_000.0, 10_000.0),
        &Values::new(),
    ))?;
    assert_eq!(parameter, "window");
    Ok(())
}

#[test]
fn the_size_of_a_whole_render_keeps_its_limits() -> Outcome {
    let drawing = Drawing::from(Path::circle((100.0, 75.0), 40.0)?).with_size(200.0, 150.0);
    let (parameter, _) = parameter_refusal(drawing.to_rgba(Size::px(65_001.0, 10.0), &Values::new()))?;
    assert_eq!(parameter, "width");
    Ok(())
}
