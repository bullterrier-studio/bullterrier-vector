//! What `Drawing::to_rgba` fills: edge pixels carry the exact overlap area of the shape with the pixel, the total
//! coverage of a circle and of a star is their closed-form area, and the colour comes out as straight alpha.
//! The oracles are geometry worked out in the test (clipping, the shoelace formula, pi r squared), not the rasteriser.

mod common;

use bullterrier_vector::{Color, Drawing, Path};
use common::pixels::{Image, overlap, render};
use common::{Outcome, clearance, shoelace_area, winding};
use libm::{cos, hypot, sin};
use proptest::prelude::*;
use std::f64::consts::PI;

/// The straight-alpha pixel's colour as it would be premultiplied, in levels of 0..=255 (a float, to compare with a
/// tolerance of one level): what the pixel adds to a picture.
fn premultiplied(pixel: [u8; 4]) -> [f64; 3] {
    let alpha = f64::from(pixel[3]) / 255.0;
    [0, 1, 2].map(|channel| f64::from(pixel[channel]) * alpha)
}

#[test]
fn edge_pixels_carry_the_area_the_shape_overlaps_them_by() -> Outcome {
    let (red, green, blue) = (0xff, 0x4f, 0x9a);
    // (rule, design frame, the rectangle (x, y, width, height), output side, scale and offset the view must give)
    let rows = [
        (
            "no frame: one to one",
            None,
            (2.25, 1.5, 5.5, 5.0),
            (12, 10),
            (1.0, 0.0, 0.0),
        ),
        ("inside one pixel", None, (3.2, 3.3, 0.4, 0.25), (8, 8), (1.0, 0.0, 0.0)),
        (
            "partly off the canvas",
            None,
            (-3.5, 2.5, 8.0, 3.0),
            (6, 8),
            (1.0, 0.0, 0.0),
        ),
        (
            "frame doubled",
            Some((10.0, 10.0)),
            (1.125, 0.75, 3.0, 4.2),
            (20, 20),
            (2.0, 0.0, 0.0),
        ),
        (
            "frame doubled and centred down by 5",
            Some((10.0, 5.0)),
            (0.3, 0.2, 9.0, 4.1),
            (20, 20),
            (2.0, 0.0, 5.0),
        ),
    ];
    for (rule, frame, (x, y, width, height), (columns, lines), (scale, dx, dy)) in rows {
        let rect = Path::rect(x, y, width, height, 0.0)?;
        let mut drawing = Drawing::from(rect).fill(Color::parse("#ff4f9a")?);
        if let Some((frame_width, frame_height)) = frame {
            drawing = drawing.with_size(frame_width, frame_height);
        }
        let image = render(&drawing, columns, lines)?;
        let device = (
            x * scale + dx,
            y * scale + dy,
            (x + width) * scale + dx,
            (y + height) * scale + dy,
        );
        for row in 0..lines {
            for column in 0..columns {
                let expected = overlap(device, column, row) * 255.0;
                let pixel = image.pixel(column, row);
                assert!(
                    (f64::from(pixel[3]) - expected).abs() <= 1.0,
                    "{rule}: column {column} row {row} has alpha {}, the overlap says {expected}",
                    pixel[3]
                );
                // straight alpha: the colour is the fill's, to within the one level the premultiplied value can be off
                let wanted = premultiplied([red, green, blue, pixel[3]]);
                for (got, want) in premultiplied(pixel).into_iter().zip(wanted) {
                    assert!(
                        (got - want).abs() <= 1.0,
                        "{rule}: column {column} row {row}: {pixel:?}"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn a_fill_with_alpha_is_returned_as_straight_alpha() -> Outcome {
    // (red, green, blue, alpha) painted over the whole canvas; the pixels must say the same, within a level
    let rows = [
        (255, 0, 0, 255),
        (10, 20, 30, 255),
        (200, 100, 50, 128),
        (255, 255, 255, 128),
        (40, 220, 130, 77),
        (0, 0, 0, 200),
        (90, 160, 250, 64),
    ];
    let everything = Path::rect(-5.0, -5.0, 30.0, 30.0, 0.0)?;
    for (red, green, blue, alpha) in rows {
        let color = Color::parse(&format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}"))?;
        let image = render(&Drawing::from(everything.clone()).fill(color), 4, 4)?;
        let pixel = image.pixel(2, 1);
        assert_eq!(pixel[3], alpha, "{red},{green},{blue},{alpha}: {pixel:?}");
        // a colour left premultiplied would be about alpha / 255 times as large
        for (got, want) in [red, green, blue].into_iter().zip(&pixel) {
            assert!(
                (i32::from(*want) - got).abs() <= 2,
                "{red},{green},{blue},{alpha}: {pixel:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn straight_alpha_is_the_premultiplied_colour_over_alpha_rounded_to_nearest() -> Outcome {
    // (colour channel, alpha, the byte returned): the rasteriser holds the colour times alpha / 255 as a whole number
    // (not near a half, so every way of rounding it agrees: 172 x 59 / 255 = 39.8 is 40), and 40 x 255 / 59 = 172.9 is
    // 173 rounded to nearest, 172 truncated. Worked by hand for each row.
    let rows = [
        (172, 59, 173),  // 40 x 255 / 59 = 172.88
        (233, 38, 235),  // 35 x 255 / 38 = 234.87
        (67, 94, 68),    // 25 x 255 / 94 = 67.82
        (41, 55, 42),    // 9 x 255 / 55 = 41.73
        (7, 225, 7),     // 6 x 255 / 225 = 6.8
        (165, 189, 165), // 122 x 255 / 189 = 164.60
        (200, 128, 199), // 100 x 255 / 128 = 199.22, rounds down
    ];
    let everything = Path::rect(-5.0, -5.0, 30.0, 30.0, 0.0)?;
    for (channel, alpha, expected) in rows {
        let color = Color::parse(&format!("#{channel:02x}{channel:02x}{channel:02x}{alpha:02x}"))?;
        let image = render(&Drawing::from(everything.clone()).fill(color), 3, 3)?;
        assert_eq!(
            image.pixel(1, 1),
            [expected, expected, expected, alpha],
            "channel {channel} alpha {alpha}"
        );
    }
    Ok(())
}

#[test]
fn a_fill_that_is_not_painted_draws_nothing() -> Outcome {
    let image = render(&Drawing::from(Path::circle((8.0, 8.0), 6.0)?).fill(Color::NONE), 16, 16)?;
    assert!(image.bytes.iter().all(|&byte| byte == 0));
    Ok(())
}

#[test]
fn the_fill_rule_is_nonzero() -> Outcome {
    // a pentagram drawn in one stroke winds twice round its middle: nonzero fills it, even-odd would leave a hole
    let corners: Vec<(f64, f64)> = [0, 2, 4, 1, 3]
        .iter()
        .map(|&k| {
            let angle = 2.0 * PI * f64::from(k) / 5.0;
            (50.0 + 40.0 * sin(angle), 50.0 - 40.0 * cos(angle))
        })
        .collect();
    let image = render(&Drawing::from(Path::polyline(corners.clone(), true)?), 100, 100)?;
    assert_eq!(
        winding(&corners, (50.5, 50.5)),
        2,
        "the oracle: the middle is wound twice"
    );
    assert_eq!(image.alpha(50, 50), 255, "the middle of the pentagram");
    // the tips are wound once, and the gaps between them not at all
    assert_eq!(image.alpha(50, 15), 255, "a tip");
    assert_eq!(image.alpha(5, 5), 0, "outside");
    Ok(())
}

/// Every pixel at least a pixel away from the outline of the closed polygon through `corners` is opaque when its
/// centre is wound round and clear when it is not.
fn assert_polygon_pixels(image: &Image, corners: &[(f64, f64)]) {
    for row in 0..image.height {
        for column in 0..image.width {
            #[expect(clippy::cast_precision_loss, reason = "small pixel indices")]
            let centre = (column as f64 + 0.5, row as f64 + 0.5);
            if clearance(corners, centre) <= 1.0 {
                continue;
            }
            let expected = if winding(corners, centre) == 0 { 0 } else { 255 };
            assert_eq!(image.alpha(column, row), expected, "column {column} row {row}");
        }
    }
}

/// The corners of the star `Path::star` makes: tip first at the top, then clockwise, alternating outer and inner.
fn star_corners(center: (f64, f64), outer: f64, inner: f64, points: u32) -> Vec<(f64, f64)> {
    (0..2 * points)
        .map(|k| {
            let angle = PI * f64::from(k) / f64::from(points);
            let radius = if k % 2 == 0 { outer } else { inner };
            (center.0 + radius * sin(angle), center.1 - radius * cos(angle))
        })
        .collect()
}

// each case checks ten thousand pixels, so fewer cases than the default give the same time as the other files
proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn a_stars_pixels_are_wound_ones_and_its_coverage_is_its_area(
        center in (40.0f64..60.0, 40.0f64..60.0),
        outer in 20.0f64..38.0,
        share in 0.35f64..0.85,
        points in 3u32..9,
    ) {
        let inner = outer * share;
        let star = Path::star(center, outer, inner, i64::from(points)).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let image = render(&Drawing::from(star), 100, 100).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let corners = star_corners(center, outer, inner, points);
        assert_polygon_pixels(&image, &corners);
        let area = shoelace_area(&corners).abs();
        let covered = image.covered_area();
        prop_assert!((covered - area).abs() <= area * 0.005, "covered {covered}, area {area}");
    }

    #[test]
    fn a_circles_pixels_are_in_or_out_and_its_coverage_is_pi_r_squared_less_the_flattening(
        center in (30.0f64..70.0, 30.0f64..70.0),
        radius in 10.0f64..28.0,
    ) {
        let circle = Path::circle(center, radius).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let image = render(&Drawing::from(circle), 100, 100).map_err(|e| TestCaseError::fail(e.to_string()))?;
        for row in 0..100usize {
            for column in 0..100usize {
                #[expect(clippy::cast_precision_loss, reason = "small pixel indices")]
                let away = hypot(column as f64 + 0.5 - center.0, row as f64 + 0.5 - center.1);
                if away < radius - 1.0 {
                    prop_assert_eq!(image.alpha(column, row), 255);
                } else if away > radius + 1.0 {
                    prop_assert_eq!(image.alpha(column, row), 0);
                }
            }
        }
        // the rasteriser flattens a curve to straight edges at most a quarter of a pixel inside it, which can take a
        // quarter of a pixel off all the way round; the circle itself is the cubic arcs, 0.03% too large
        let area = PI * radius * radius;
        let covered = image.covered_area();
        let bound = 2.0 * PI * radius * 0.25;
        prop_assert!(covered >= area - bound && covered <= area * 1.001, "covered {covered}, area {area}");
    }
}
