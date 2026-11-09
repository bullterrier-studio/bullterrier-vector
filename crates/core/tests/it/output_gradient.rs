//! A gradient in the pixels `Drawing::to_rgba` draws, against the closed form: a linear one is its stops at
//! `t = ((p - a) . (b - a)) / |b - a|^2`, a radial one at `t = |p - c| / r`, both held at the end stops beyond, and the
//! alpha of the stops is interpolated apart from their colour. Pixel values are worked out by hand from the stops.

#![expect(clippy::cast_precision_loss, reason = "pixel indices are small")]

use crate::common::Outcome;
use crate::common::pixels::{Image, render};
use bullterrier_vector::{Cap, Color, Drawing, Error, Gradient, Path, Shadow, Stroke};

const RED: Color = Color::hex(0xff_00_00);
const BLUE: Color = Color::hex(0x00_00_ff);
const WHITE: Color = Color::hex(0xff_ff_ff);
const BLACK: Color = Color::hex(0x00_00_00);

type Point = (f64, f64);
type Stops = Vec<(f64, Color)>;

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

fn canvas() -> Result<Path, Error> {
    Path::rect(0.0, 0.0, 100.0, 100.0, 0.0)
}

/// The straight RGBA of `stops` at `t`, by hand: each channel interpolated between the two stops around `t` (alpha
/// apart from colour), the end stops beyond them.
fn ramp(stops: &[(f64, [u8; 4])], t: f64) -> [f64; 4] {
    let t = t.clamp(0.0, 1.0);
    let after = stops
        .iter()
        .position(|(offset, _)| *offset >= t)
        .unwrap_or(stops.len() - 1);
    let before = after.saturating_sub(1);
    let ((o0, c0), (o1, c1)) = (stops[before], stops[after]);
    let share = if o1 > o0 {
        ((t - o0) / (o1 - o0)).clamp(0.0, 1.0)
    } else {
        1.0
    };
    std::array::from_fn(|channel| f64::from(c0[channel]) + (f64::from(c1[channel]) - f64::from(c0[channel])) * share)
}

fn square(x: f64) -> f64 {
    x * x
}

fn linear_t(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> f64 {
    let (along, length) = (
        (p.0 - a.0) * (b.0 - a.0) + (p.1 - a.1) * (b.1 - a.1),
        square(b.0 - a.0) + square(b.1 - a.1),
    );
    along / length
}

fn radial_t(c: (f64, f64), r: f64, p: (f64, f64)) -> f64 {
    (square(p.0 - c.0) + square(p.1 - c.1)).sqrt() / r
}

/// An error unless the pixel at `(x, y)` is `expected` within 3 levels a channel (the gradient is a table of 8 bits).
fn near(image: &Image, (x, y): (usize, usize), expected: [f64; 4]) -> Outcome {
    let found = image.pixel(x, y);
    let off = (0..4).any(|c| (f64::from(found[c]) - expected[c]).abs() > 3.0);
    if off {
        Err(format!("pixel ({x}, {y}) is {found:?}, expected about {expected:.1?}").into())
    } else {
        Ok(())
    }
}

fn centre(x: usize, y: usize) -> (f64, f64) {
    (x as f64 + 0.5, y as f64 + 0.5)
}

const SAMPLES: [(usize, usize); 7] = [(2, 3), (25, 60), (50, 50), (75, 10), (97, 97), (10, 90), (60, 33)];

#[test]
fn a_linear_gradient_is_its_stops_at_the_projection_of_the_pixel_on_the_line() -> Outcome {
    let rows: [(&str, Point, Point, Stops); 5] = [
        ("across", (0.0, 0.0), (100.0, 0.0), vec![(0.0, RED), (1.0, BLUE)]),
        (
            "down the diagonal, three stops",
            (0.0, 0.0),
            (100.0, 100.0),
            vec![(0.0, RED), (0.3, WHITE), (1.0, BLUE)],
        ),
        (
            "upwards, a short line (held beyond)",
            (0.0, 70.0),
            (0.0, 30.0),
            vec![(0.0, RED), (1.0, BLUE)],
        ),
        (
            "a steep line",
            (80.0, 10.0),
            (20.0, 60.0),
            vec![(0.0, WHITE), (0.5, RED), (1.0, BLACK)],
        ),
        (
            "stops that do not start at 0 or end at 1",
            (0.0, 0.0),
            (100.0, 0.0),
            vec![(0.25, RED), (0.75, BLUE)],
        ),
    ];
    for (case, a, b, stops) in rows {
        let gradient = Gradient::linear(a, b, stops.clone())?;
        let image = render(&Drawing::from(canvas()?).fill(gradient.clone()), 100, 100)?;
        let by_hand: Vec<_> = gradient.stops().to_vec();
        for (x, y) in SAMPLES {
            near(&image, (x, y), ramp(&by_hand, linear_t(a, b, centre(x, y)))).map_err(|e| format!("{case}: {e}"))?;
        }
    }
    Ok(())
}

#[test]
fn a_radial_gradient_is_its_stops_at_the_distance_from_the_centre_over_the_radius() -> Outcome {
    let rows: [(&str, Point, f64, Stops); 3] = [
        ("centred", (50.0, 50.0), 40.0, vec![(0.0, WHITE), (1.0, BLACK)]),
        (
            "off centre, three stops",
            (20.0, 70.0),
            60.0,
            vec![(0.0, RED), (0.5, WHITE), (1.0, BLUE)],
        ),
        (
            "a small circle held beyond its radius",
            (50.0, 50.0),
            10.0,
            vec![(0.0, RED), (1.0, BLUE)],
        ),
    ];
    for (case, c, r, stops) in rows {
        let gradient = Gradient::radial(c, r, stops)?;
        let image = render(&Drawing::from(canvas()?).fill(gradient.clone()), 100, 100)?;
        for (x, y) in SAMPLES {
            near(&image, (x, y), ramp(gradient.stops(), radial_t(c, r, centre(x, y))))
                .map_err(|e| format!("{case}: {e}"))?;
        }
    }
    Ok(())
}

#[test]
fn stops_with_alpha_interpolate_their_colour_and_alpha_apart_not_premultiplied() -> Outcome {
    // transparent red to opaque blue: halfway the straight colour is the mean (127.5, 0, 127.5) at alpha 127.5, where
    // a premultiplied blend would leave pure blue at that alpha
    let gradient = Gradient::linear((0.0, 0.0), (100.0, 0.0), [(0.0, color("#ff000000")?), (1.0, BLUE)])?;
    let image = render(&Drawing::from(canvas()?).fill(gradient.clone()), 100, 100)?;
    for x in [10, 30, 50, 70, 90] {
        near(&image, (x, 40), ramp(gradient.stops(), (x as f64 + 0.5) / 100.0))?;
    }
    let middle = image.pixel(50, 40);
    assert!(
        middle[0] > 100 && middle[0] < 150 && middle[2] < 150,
        "{middle:?} is not the straight mean"
    );
    Ok(())
}

#[test]
fn a_hard_edge_is_a_stop_pair_at_one_offset() -> Outcome {
    let gradient = Gradient::linear(
        (0.0, 0.0),
        (100.0, 0.0),
        [(0.0, RED), (0.5, RED), (0.5, BLUE), (1.0, BLUE)],
    )?;
    let image = render(&Drawing::from(canvas()?).fill(gradient), 100, 100)?;
    near(&image, (10, 10), [255.0, 0.0, 0.0, 255.0])?;
    near(&image, (48, 10), [255.0, 0.0, 0.0, 255.0])?;
    near(&image, (51, 10), [0.0, 0.0, 255.0, 255.0])?;
    near(&image, (90, 10), [0.0, 0.0, 255.0, 255.0])
}

#[test]
fn the_geometry_is_fixed_to_the_frame_so_a_moved_shape_shows_another_part_of_it() -> Outcome {
    let gradient = Gradient::linear((0.0, 0.0), (100.0, 0.0), [(0.0, RED), (1.0, BLUE)])?;
    let square = Path::rect(0.0, 0.0, 20.0, 20.0, 0.0)?;
    let at_left = render(
        &Drawing::from(square.clone())
            .fill(gradient.clone())
            .with_size(100.0, 100.0),
        100,
        100,
    )?;
    let moved = render(
        &Drawing::from(square)
            .translate(60.0, 0.0)
            .fill(gradient.clone())
            .with_size(100.0, 100.0),
        100,
        100,
    )?;
    near(&at_left, (10, 10), ramp(gradient.stops(), 0.105))?;
    near(&moved, (70, 10), ramp(gradient.stops(), 0.705))
}

#[test]
fn it_is_drawn_at_the_size_of_the_output_as_a_path_is() -> Outcome {
    let gradient = Gradient::linear((0.0, 0.0), (100.0, 0.0), [(0.0, RED), (1.0, BLUE)])?;
    let drawing = Drawing::from(canvas()?).fill(gradient.clone()).with_size(100.0, 100.0);
    let big = render(&drawing, 300, 300)?;
    for x in [15, 150, 285] {
        near(&big, (x, 100), ramp(gradient.stops(), (x as f64 + 0.5) / 300.0))?;
    }
    Ok(())
}

#[test]
fn a_fill_of_a_colour_after_a_gradient_draws_what_a_plain_fill_draws() -> Outcome {
    let gradient = Gradient::linear((0.0, 0.0), (100.0, 0.0), [(0.0, RED), (1.0, BLUE)])?;
    let path = Path::circle((50.0, 50.0), 40.0)?;
    let after = render(
        &Drawing::from(path.clone()).fill(gradient).fill(color("#33669980")?),
        100,
        100,
    )?;
    let plain = render(&Drawing::from(path).fill(color("#33669980")?), 100, 100)?;
    assert!(after.bytes == plain.bytes, "the pixels differ");
    Ok(())
}

#[test]
fn stops_listed_in_the_other_direction_on_the_reversed_line_draw_the_same_picture() -> Outcome {
    let forward = Gradient::linear((10.0, 20.0), (90.0, 60.0), [(0.0, RED), (0.3, WHITE), (1.0, BLUE)])?;
    let backward = Gradient::linear((90.0, 60.0), (10.0, 20.0), [(0.0, BLUE), (0.7, WHITE), (1.0, RED)])?;
    let (a, b) = (
        render(&Drawing::from(canvas()?).fill(forward), 100, 100)?,
        render(&Drawing::from(canvas()?).fill(backward), 100, 100)?,
    );
    let worst = a
        .bytes
        .iter()
        .zip(&b.bytes)
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap_or(0);
    assert!(worst <= 2, "the largest channel difference is {worst}");
    Ok(())
}

#[test]
fn a_gradient_stroke_is_the_gradient_inside_the_stroke_and_nothing_outside() -> Outcome {
    let gradient = Gradient::linear((0.0, 0.0), (100.0, 0.0), [(0.0, RED), (1.0, BLUE)])?;
    let line = Path::line((10.0, 50.0), (90.0, 50.0))?;
    let image = render(
        &Drawing::from(line).stroke(Stroke::new(gradient.clone()).width(20.0).cap(Cap::Butt)),
        100,
        100,
    )?;
    for x in [15, 40, 60, 85] {
        near(&image, (x, 45), ramp(gradient.stops(), (x as f64 + 0.5) / 100.0))?;
    }
    near(&image, (50, 30), [0.0; 4])?;
    near(&image, (50, 70), [0.0; 4])?;
    near(&image, (5, 50), [0.0; 4])
}

#[test]
fn a_stroke_that_overlaps_itself_is_not_painted_twice_where_it_does() -> Outcome {
    // two crossing lines and round ends, all of one gradient of alpha 128: wherever the stroke is, the alpha is 128, not
    // the 192 of two layers
    let half = Gradient::linear(
        (0.0, 0.0),
        (100.0, 0.0),
        [(0.0, color("#ff000080")?), (1.0, color("#0000ff80")?)],
    )?;
    let cross = Path::from_svg_path("M10 10 L90 90 M10 90 L90 10")?;
    let image = render(
        &Drawing::from(cross).stroke(Stroke::new(half).width(16.0).cap(Cap::Round)),
        100,
        100,
    )?;
    for (x, y) in [(50, 50), (30, 30), (70, 30), (12, 12), (88, 88)] {
        let alpha = image.alpha(x, y);
        assert!((127..=129).contains(&alpha), "({x}, {y}) has alpha {alpha}");
    }
    Ok(())
}

#[test]
fn the_shadow_of_a_gradient_is_as_strong_as_the_gradient_is_opaque_where_it_falls() -> Outcome {
    // black, fading in from nothing to opaque across the shape, with a black shadow straight down 40 pixels: the
    // shadow is the shape's alpha moved, so under the shape's x it is the gradient's alpha at that x
    let fade_in = Gradient::linear((10.0, 0.0), (50.0, 0.0), [(0.0, color("#00000000")?), (1.0, BLACK)])?;
    let rect = Path::rect(10.0, 10.0, 40.0, 20.0, 0.0)?;
    for blur in [0.0, 6.0] {
        let image = render(
            &Drawing::from(rect.clone())
                .fill(fade_in.clone())
                .shadow(Shadow::new().dx(0.0).dy(40.0).blur(blur).color(BLACK)),
            100,
            100,
        )?;
        for x in [15, 25, 35, 40] {
            let wanted = (x as f64 + 0.5 - 10.0) / 40.0 * 255.0;
            let found = f64::from(image.alpha(x, 60));
            assert!(
                (found - wanted).abs() <= 12.0,
                "blur {blur}, x {x}: shadow alpha {found}, gradient alpha {wanted}"
            );
        }
        // and little where the gradient is clear: the blur only spreads the alpha next to it
        assert!(image.alpha(11, 60) + 100 < image.alpha(40, 60), "blur {blur}");
    }
    Ok(())
}
