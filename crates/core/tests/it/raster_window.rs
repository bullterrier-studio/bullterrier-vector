//! `to_rgba_window` and `to_png_window`: a window of the output is the same bytes as the same rows and columns of the whole
//! render. The oracle is the whole render itself (`to_rgba`), cut by hand; windows are 1-pixel ones, ones on each edge of
//! the canvas, ones at offsets that are no multiple of a tile or a blur's decimation, and a seeded random spread, over a
//! drawing for each effect. A band loop that stitches a 4 by 3 grid of windows gives the whole render back.

use crate::common::fonts::fraunces;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{
    Blend, Cap, Color, Deg, Drawing, Error, Gradient, Halftone, Join, Path, Shadow, Size, Stroke, Text, Values, Window,
    text,
};

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

type Row = (&'static str, Drawing);

fn paper() -> Result<Drawing, Error> {
    solid(rect(0.0, 0.0, 200.0, 150.0)?, "#e8a060")
}

/// The drawings: (what it exercises, drawing), each in a frame of 200 by 150 design pixels: fills, strokes of each join
/// and end, booleans, opacity groups, blurred and hard shadows, masks, clips, gradients, blends, knockouts, halftones and text.
fn corpus() -> Result<Vec<Row>, Error> {
    let rows = [shapes()?, shadows()?, blurred()?, cuts()?, paints()?].concat();
    Ok(rows
        .into_iter()
        .map(|(name, drawing)| (name, drawing.with_size(200.0, 150.0)))
        .collect())
}

/// Fills, strokes by join and end, booleans and opacity groups, each apart.
fn shapes() -> Result<Vec<Row>, Error> {
    let zigzag = || {
        Path::polyline(
            [
                (20.0, 120.0),
                (70.0, 30.0),
                (110.0, 110.0),
                (150.0, 25.0),
                (185.0, 100.0),
            ],
            false,
        )
    };
    let stroked = |join: Join, cap: Cap| -> Result<Drawing, Error> {
        Ok(Drawing::new([
            paper()?,
            Drawing::from(zigzag()?).stroke(Stroke::new(color("#102040c0")?).width(11.0).join(join).cap(cap)),
        ]))
    };
    let rows = vec![
        (
            "fill",
            Drawing::new([paper()?, solid(Path::star((100.0, 75.0), 60.0, 25.0, 5)?, "#3070c0")?]),
        ),
        (
            "fill across the 256th column",
            Drawing::new([paper()?, solid(Path::circle((160.0, 75.0), 30.0)?, "#3070c0")?]),
        ),
        (
            "opacity across the 256th column",
            Drawing::new([
                paper()?,
                solid(Path::circle((160.0, 75.0), 30.0)?, "#3070c0")?.opacity(0.5),
            ]),
        ),
        (
            "multiply across the 256th column",
            Drawing::new([
                paper()?,
                solid(Path::circle((160.0, 75.0), 30.0)?, "#3070c0")?.blend(Blend::Multiply),
            ]),
        ),
        (
            "clip across the 256th column",
            Drawing::new([
                paper()?,
                solid(rect(100.0, 20.0, 100.0, 100.0)?, "#3070c0")?.clip(Path::circle((160.0, 75.0), 30.0)?),
            ]),
        ),
        ("stroke, round join and end", stroked(Join::Round, Cap::Round)?),
        ("stroke, miter join and butt end", stroked(Join::Miter, Cap::Butt)?),
        ("stroke, bevel join and square end", stroked(Join::Bevel, Cap::Square)?),
        (
            "opacity group",
            Drawing::new([
                paper()?,
                Drawing::from(Path::circle((90.0, 70.0), 40.0)?)
                    .fill(color("#c03030")?)
                    .stroke(Stroke::new(color("#102040")?).width(12.0))
                    .opacity(0.5),
            ]),
        ),
        (
            "boolean shapes",
            Drawing::new([
                paper()?,
                solid(Path::circle((80.0, 75.0), 50.0)?, "#3070c0")?
                    .union(solid(rect(100.0, 40.0, 70.0, 60.0)?, "#3070c0")?, 0.1)
                    .subtract(Path::circle((100.0, 75.0), 20.0)?, 0.1),
            ]),
        ),
    ];
    Ok(rows)
}

/// Fills, strokes and the shadows of shapes.
fn shadows() -> Result<Vec<Row>, Error> {
    let rows = vec![
        (
            "fill and stroke",
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
            Drawing::new([
                paper()?,
                solid(rect(50.0, 40.0, 70.0, 50.0)?, "#3070c0")?
                    .shadow(Shadow::new().dx(8.0).dy(12.0).blur(24.0).color(color("#000000b0")?)),
            ]),
        ),
        (
            "shadow with a wide blur at the canvas edge",
            Drawing::new([
                paper()?,
                solid(rect(150.0, 100.0, 60.0, 50.0)?, "#3070c0")?
                    .shadow(Shadow::new().dx(-14.0).dy(-6.0).blur(90.0).color(color("#000000")?)),
            ]),
        ),
        (
            "shadow of a shape that runs off the canvas",
            Drawing::new([
                paper()?,
                solid(rect(100.0, 60.0, 400.0, 400.0)?, "#3070c0")?
                    .shadow(Shadow::new().dx(0.0).dy(0.0).blur(40.0).color(color("#000000")?)),
            ]),
        ),
        (
            "shadow offset far past its blur",
            Drawing::new([
                paper()?,
                solid(rect(20.0, 20.0, 30.0, 20.0)?, "#3070c0")?
                    .shadow(Shadow::new().dx(90.0).dy(70.0).blur(6.0).color(color("#000000")?)),
            ]),
        ),
        (
            "shadow offset far up and left past its blur",
            Drawing::new([
                paper()?,
                solid(rect(140.0, 100.0, 30.0, 20.0)?, "#3070c0")?
                    .shadow(Shadow::new().dx(-90.0).dy(-70.0).blur(6.0).color(color("#000000")?)),
            ]),
        ),
        (
            "shadow without a blur",
            Drawing::new([
                paper()?,
                solid(Path::circle((100.0, 75.0), 30.0)?, "#3070c0")?
                    .shadow(Shadow::new().dx(5.5).dy(3.25).blur(0.0).color(color("#00000080")?)),
            ]),
        ),
        (
            "shadow of a stroke under opacity",
            Drawing::new([
                paper()?,
                Drawing::from(Path::circle((100.0, 75.0), 30.0)?)
                    .stroke(Stroke::new(color("#3070c0")?).width(9.0))
                    .opacity(0.7)
                    .shadow(Shadow::new().dx(0.0).dy(9.0).blur(18.0).color(color("#000000")?)),
            ]),
        ),
    ];
    Ok(rows)
}

/// Shadows of shapes whose corners lie off every grid, and under opacity.
fn blurred() -> Result<Vec<Row>, Error> {
    let rows = vec![
        (
            "gradient and miter stroke under opacity, with a shadow",
            Drawing::new([
                paper()?,
                Drawing::from(Path::star((100.0, 75.0), 60.0, 25.0, 5)?)
                    .fill(fade()?)
                    .stroke(Stroke::new(color("#102040c0")?).width(9.0).join(Join::Miter))
                    .opacity(0.7)
                    .shadow(Shadow::new().dx(3.0).dy(8.0).blur(24.0).color(color("#000000b0")?)),
            ]),
        ),
        (
            "shadow of a star, a corner off every grid",
            Drawing::new([
                paper()?,
                Drawing::from(Path::star((100.0, 75.0), 60.0, 25.0, 5)?)
                    .fill(color("#3070c0")?)
                    .shadow(Shadow::new().dx(3.0).dy(8.0).blur(24.0).color(color("#000000b0")?)),
            ]),
        ),
        (
            "shadows of two shapes whose corners are off every grid, a small blur and a large one",
            Drawing::new([
                paper()?,
                solid(rect(33.3, 41.7, 40.0, 30.0)?, "#3070c0")?
                    .shadow(Shadow::new().dx(4.0).dy(5.0).blur(6.0).color(color("#000000c0")?)),
                solid(Path::circle((141.3, 97.9), 22.0)?, "#c03030")?
                    .shadow(Shadow::new().dx(-7.0).dy(9.0).blur(60.0).color(color("#000000c0")?)),
            ]),
        ),
    ];
    Ok(rows)
}

/// Masks and clips.
fn cuts() -> Result<Vec<Row>, Error> {
    let rows = vec![
        (
            "mask",
            Drawing::new([
                paper()?,
                solid(Path::circle((100.0, 75.0), 60.0)?, "#3070c0")?
                    .mask(Drawing::from(rect(0.0, 0.0, 200.0, 150.0)?).fill(fade()?)),
            ]),
        ),
        (
            "mask constant",
            Drawing::new([
                paper()?,
                solid(Path::circle((160.0, 75.0), 60.0)?, "#3070c0")?
                    .mask(solid(rect(0.0, 0.0, 200.0, 150.0)?, "#d8d8d8")?),
            ]),
        ),
        (
            "mask constant inside",
            Drawing::new([
                paper()?,
                solid(Path::circle((176.0, 75.0), 12.0)?, "#3070c0")?
                    .mask(solid(rect(0.0, 0.0, 200.0, 150.0)?, "#d8d8d8")?),
            ]),
        ),
        (
            "mask rect gradient",
            Drawing::new([
                paper()?,
                solid(rect(0.0, 0.0, 200.0, 150.0)?, "#3070c0")?
                    .mask(Drawing::from(rect(0.0, 0.0, 200.0, 150.0)?).fill(fade()?)),
            ]),
        ),
        (
            "mask with a blurred shadow in it",
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
            Drawing::new([
                paper()?,
                solid(Path::circle((100.0, 75.0), 60.0)?, "#3070c0")?
                    .shadow(Shadow::new().dx(6.0).dy(6.0).blur(14.0).color(color("#000000")?))
                    .clip(Path::circle((80.0, 60.0), 45.0)?),
            ]),
        ),
    ];
    Ok(rows)
}

/// Gradients, blends, knockouts, halftones and text.
fn paints() -> Result<Vec<Row>, Error> {
    let font = fraunces()?;
    let rows = vec![
        (
            "gradients",
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
            Drawing::new([
                paper()?,
                solid(Path::star((100.0, 75.0), 55.0, 20.0, 6)?, "#000000")?.knockout(),
            ]),
        ),
        (
            "halftone",
            Drawing::new([
                paper()?,
                Drawing::from(Path::circle((100.0, 75.0), 60.0)?)
                    .halftone(Halftone::new(color("#102040")?, 0.4).pitch(7.0).angle(Deg(30.0)))
                    .stroke(Stroke::new(color("#102040")?).width(3.0)),
            ]),
        ),
        (
            "text",
            Drawing::new([
                paper()?,
                text("HIFCTAVeflo", Text::new(&font).size(34.0).width(180.0))
                    .translate(10.0, 40.0)
                    .fill(color("#102040")?)
                    .shadow(Shadow::new().dx(2.0).dy(4.0).blur(8.0).color(color("#00000090")?)),
            ]),
        ),
    ];
    Ok(rows)
}

/// The size of a whole output, in pixels: two wide tiles of the rasteriser across, or three and a bit.
#[derive(Clone, Copy)]
struct Canvas {
    width: usize,
    height: usize,
}

const NARROW: Canvas = Canvas {
    width: 331,
    height: 251,
};
const WIDE: Canvas = Canvas {
    width: 700,
    height: 300,
};

impl Canvas {
    fn size(self) -> Size {
        #[expect(clippy::cast_precision_loss, reason = "small")]
        Size::px(self.width as f64, self.height as f64)
    }

    fn whole(self, drawing: &Drawing) -> Result<Vec<u8>, Error> {
        drawing.to_rgba(self.size(), &Values::new())
    }

    fn window(self, drawing: &Drawing, (x, y, width, height): Rect) -> Result<Vec<u8>, Error> {
        #[expect(clippy::cast_precision_loss, reason = "small")]
        let window = Window::px(x as f64, y as f64, width as f64, height as f64);
        drawing.to_rgba_window(self.size(), window, &Values::new())
    }

    /// The rows and columns `at` of the whole picture `bytes`.
    fn cut(self, bytes: &[u8], (x, y, width, height): Rect) -> Vec<u8> {
        (y..y + height)
            .flat_map(|row| {
                bytes[(row * self.width + x) * 4..(row * self.width + x + width) * 4]
                    .iter()
                    .copied()
            })
            .collect()
    }

    /// A deterministic spread of windows: 1-pixel ones, ones on each edge, ones at every offset that is no multiple
    /// of a tile, of a wide tile or of a blur's decimation, ones that cut what is drawn, and a seeded random
    /// spread (a linear congruential generator, so that a failure is the same on every run).
    fn windows(self) -> Vec<Rect> {
        let (w, h) = (self.width, self.height);
        let mut windows = vec![
            (0, 0, 1, 1),
            (w - 1, h - 1, 1, 1),
            (w - 1, 0, 1, 1),
            (0, h - 1, 1, 1),
            (123, 77, 1, 1),
            (0, 0, w, 1),
            (0, h - 1, w, 1),
            (0, 0, 1, h),
            (w - 1, 0, 1, h),
            (0, 0, w, h),
            (1, 1, w - 2, h - 2),
            (3, 5, 64, 48),
            (131, 99, 67, 61),
            (w - 17, h - 13, 17, 13),
            (255, 255 - 100, 41, 33),
            (257, 129, 33, 17),
            (256, 0, 40, 40),
            (300, 100, 31, 50),
            (0, 128, 40, 40),
            (5, 130, 40, 40),
            (256, 128, 40, 40),
            (255, 0, 2, h),
            (252, 3, 9, 9),
            (508, 3, 9, h - 3),
        ];
        let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = |below: usize| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let high = (state >> 33) as usize;
            high % below
        };
        for _ in 0..80 {
            let (width, height) = (1 + next(w.min(120)), 1 + next(h.min(120)));
            windows.push((next(w - width + 1), next(h - height + 1), width, height));
        }
        windows.retain(|(x, y, width, height)| x + width <= w && y + height <= h);
        windows
    }
}

/// A rectangle of pixels: column, row, width, height.
type Rect = (usize, usize, usize, usize);

/// The most two pixels differ by, in levels of `premultiplied`.
fn levels_apart(a: [u8; 4], b: [u8; 4]) -> u8 {
    premultiplied(a)
        .iter()
        .zip(premultiplied(b))
        .map(|(x, y)| x.abs_diff(y))
        .max()
        .unwrap_or(0)
}

/// The most any pixel of two same-sized pictures differs by, in levels of `premultiplied`.
fn worst_level(a: &[u8], b: &[u8]) -> u8 {
    a.as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .map(|(x, y)| levels_apart(*x, *y))
        .max()
        .unwrap_or(0)
}

/// `[red, green, blue, alpha]` as the product of each colour with the alpha, as a faint pixel's colour is only known
/// to within what its alpha leaves of it (a straight colour moves by several levels for one level of the product).
fn premultiplied(pixel: [u8; 4]) -> [u8; 4] {
    let alpha = u16::from(pixel[3]);
    #[expect(clippy::cast_possible_truncation, reason = "at most 255")]
    let times = |channel: u8| ((u16::from(channel) * alpha + 127) / 255) as u8;
    [times(pixel[0]), times(pixel[1]), times(pixel[2]), pixel[3]]
}

/// How the window `at` of `drawing` differs from the same rows and columns of `whole`: nothing if it is the same
/// bytes, else the count of bytes that differ, the most a pixel differs by and the first pixel's column and row.
fn difference(canvas: Canvas, whole: &[u8], drawing: &Drawing, at: Rect) -> Result<Option<String>, Error> {
    let expected = canvas.cut(whole, at);
    let got = canvas.window(drawing, at)?;
    if got == expected {
        return Ok(None);
    }
    if got.len() != expected.len() {
        return Ok(Some(format!(
            "{at:?}: {} bytes, expected {}",
            got.len(),
            expected.len()
        )));
    }
    let pixels = || got.as_chunks::<4>().0.iter().zip(expected.as_chunks::<4>().0);
    let first = pixels().position(|(a, b)| a != b).unwrap_or(0);
    let count = pixels().filter(|(a, b)| a != b).count();
    let most = pixels().map(|(a, b)| levels_apart(*a, *b)).max().unwrap_or(0);
    Ok(Some(format!(
        "{at:?}: {count} pixels differ by up to {most} levels, the first at column {} row {}",
        at.0 + first % at.2,
        at.1 + first / at.2
    )))
}

#[test]
fn a_window_is_the_same_bytes_as_the_same_rows_and_columns_of_the_whole_render() -> Outcome {
    let mut wrong = Vec::new();
    for canvas in [NARROW, WIDE] {
        for (name, drawing) in corpus()? {
            let whole = canvas.whole(&drawing)?;
            for at in canvas.windows() {
                if let Some(found) = difference(canvas, &whole, &drawing, at)? {
                    wrong.push(format!("{name} on {}x{}: {found}", canvas.width, canvas.height));
                }
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
    let canvas = NARROW;
    let (band_width, band_height) = (canvas.width.div_ceil(columns), canvas.height.div_ceil(rows));
    for (name, drawing) in corpus()? {
        let mut stitched = vec![0_u8; canvas.width * canvas.height * 4];
        for row in 0..rows {
            for column in 0..columns {
                let (x, y) = (column * band_width, row * band_height);
                let (width, height) = (band_width.min(canvas.width - x), band_height.min(canvas.height - y));
                let part = canvas.window(&drawing, (x, y, width, height))?;
                for line in 0..height {
                    let to = ((y + line) * canvas.width + x) * 4;
                    stitched[to..to + width * 4].copy_from_slice(&part[line * width * 4..(line + 1) * width * 4]);
                }
            }
        }
        assert!(
            stitched == canvas.whole(&drawing)?,
            "{name}: the stitched grid is not the whole render"
        );
    }
    Ok(())
}

#[test]
fn a_png_window_is_a_file_of_the_window_size() -> Outcome {
    let drawing = Drawing::from(Path::circle((100.0, 75.0), 40.0)?)
        .fill(color("#3070c0")?)
        .with_size(200.0, 150.0);
    let file = drawing.to_png_window(NARROW.size(), Window::px(10.0, 20.0, 30.0, 40.0), &Values::new())?;
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
            parameter_refusal(drawing.to_rgba_window(NARROW.size(), Window::px(x, y, width, height), &Values::new()))?;
        assert_eq!(parameter, "window", "{case}");
        let (parameter, _) =
            parameter_refusal(drawing.to_png_window(NARROW.size(), Window::px(x, y, width, height), &Values::new()))?;
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
        near.as_chunks::<4>().0.iter().any(|pixel| pixel[3] > 0),
        "the drawing is not blank"
    );
    let window = Window::px(99_000.0, 99_000.0, 1000.0, 1000.0);
    let far = big.to_rgba_window(Size::px(100_000.0, 100_000.0), window, &Values::new())?;
    assert_eq!(far.len(), near.len());
    let worst = worst_level(&far, &near);
    assert!(
        worst <= 2,
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

#[test]
fn a_whole_render_of_the_largest_sides_with_a_blurred_shadow_keeps_its_limits() -> Outcome {
    // the region of a blur is cut to the canvas, or a whole render of the limit (the window that is the canvas) would
    // be refused for the margin it asks beyond it
    let drawing = |width: f64, height: f64| -> Result<Drawing, Error> {
        Ok(solid(rect(0.0, 0.0, width / 2.0, height / 2.0)?, "#3070c0")?
            .shadow(Shadow::new().dx(1.0).dy(1.0).blur(20.0).color(color("#000000")?))
            .with_size(width, height))
    };
    for (width, height) in [(65_000.0, 1.0), (1.0, 65_000.0)] {
        let pixels = drawing(width, height)?.to_rgba(Size::px(width, height), &Values::new())?;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "whole test sizes"
        )]
        let expected = width as usize * height as usize * 4;
        assert_eq!(pixels.len(), expected, "{width} by {height}");
    }
    Ok(())
}
