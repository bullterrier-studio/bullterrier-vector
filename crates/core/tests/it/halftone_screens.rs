//! The dot shapes and the screens of `halftone` through the public API, read back from the SVG it writes. The oracles
//! are worked out here, not read from the library: a dot's area by the shoelace formula against `(1 - tone)` of the
//! area its cell has by hand (a square of the pitch; a wedge of a ring is the mean radius times the angle times the
//! pitch), the corners of a diamond on the axes of the rows, the radii of the rings, the angles of the rays.

use crate::common::shoelace_area;
use crate::common::svg_oracle::{dots_of, svg_agrees};
use crate::common::{Outcome, Ring, parameter_refusal};
use bullterrier_vector::{Color, Deg, Drawing, Error, Field, Halftone, Inputs, Mark, Path, Rect, Screen, Size, Values};
use std::f64::consts::{PI, TAU};

const BLACK: Color = Color::hex(0x00_00_00);

fn rect(side: f64) -> Result<Path, Error> {
    Path::rect(0.0, 0.0, side, side, 0.0)
}

/// The dots of `halftone` over a square of `side`, whole (the SVG clips them, it does not cut them).
fn dots(side: f64, halftone: Halftone) -> Result<Vec<Ring>, Box<dyn std::error::Error>> {
    let svg = Drawing::from(rect(side)?)
        .halftone(halftone)
        .to_svg(Size::px(side, side), &Values::new())?;
    dots_of(&svg)
}

fn mean(dot: &Ring) -> (f64, f64) {
    #[expect(clippy::cast_precision_loss, reason = "a few hundred corners")]
    let count = dot.len() as f64;
    (
        dot.iter().map(|p| p.0).sum::<f64>() / count,
        dot.iter().map(|p| p.1).sum::<f64>() / count,
    )
}

/// The centre of mass of the polygon (not the mean of its corners).
fn centroid(dot: &Ring) -> (f64, f64) {
    let (mut area, mut x, mut y) = (0.0, 0.0, 0.0);
    for (at, &(x0, y0)) in dot.iter().enumerate() {
        let (x1, y1) = dot[(at + 1) % dot.len()];
        let cross = x0 * y1 - x1 * y0;
        area += cross;
        x += (x0 + x1) * cross;
        y += (y0 + y1) * cross;
    }
    (x / (3.0 * area), y / (3.0 * area))
}

fn near(got: f64, want: f64, tolerance: f64, what: &str) {
    assert!((got - want).abs() <= tolerance, "{what}: {got} against {want}");
}

fn black(tone: f64) -> Halftone {
    Halftone::new(BLACK, tone)
}

/// An asymmetric closed outline in its own units: a right triangle 4 long and 2 high.
fn triangle(clockwise: bool) -> Result<Path, Error> {
    Path::from_svg_path(if clockwise {
        "M0 0 L4 0 L0 2 Z"
    } else {
        "M0 0 L0 2 L4 0 Z"
    })
}

// What a dot covers ---------------------------------------------------------------------------------------------------

#[test]
fn every_shape_covers_one_minus_tone_of_its_own_cell_on_a_grid() -> Outcome {
    let star = Path::from_svg_path("M0 -5 L1.5 -1.5 L5 -1 L2 1 L3 5 L0 2.5 L-3 5 L-2 1 L-5 -1 L-1.5 -1.5 Z")?;
    let marks = [
        Mark::Dot,
        Mark::Square,
        Mark::Ellipse,
        Mark::Diamond,
        Mark::Euclidean,
        Mark::path(star),
    ];
    for mark in marks {
        for tone in [0.9, 0.7, 0.5, 0.3, 0.1] {
            for angle in [0.0, 30.0] {
                let found = dots(
                    200.0,
                    black(tone)
                        .pitch(10.0)
                        .angle(Deg(angle))
                        .shape(mark.clone())
                        .aspect(2.5),
                )?;
                assert!(found.len() > 100, "{}: {} dots", mark.name(), found.len());
                for dot in &found {
                    near(
                        shoelace_area(dot) / 100.0,
                        1.0 - tone,
                        1e-7,
                        &format!("{} at tone {tone}, angle {angle}", mark.name()),
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn an_ellipse_is_aspect_times_as_long_as_wide_along_the_rows() -> Outcome {
    for (angle, long_is_wide) in [(0.0, true), (90.0, false)] {
        let found = dots(
            200.0,
            black(0.6)
                .pitch(10.0)
                .angle(Deg(angle))
                .shape(Mark::Ellipse)
                .aspect(3.0),
        )?;
        assert!(found.len() > 100);
        for dot in &found {
            let (xs, ys): (Vec<f64>, Vec<f64>) = dot.iter().copied().unzip();
            let span = |values: &[f64]| {
                values.iter().copied().fold(f64::MIN, f64::max) - values.iter().copied().fold(f64::MAX, f64::min)
            };
            let ratio = if long_is_wide {
                span(&xs) / span(&ys)
            } else {
                span(&ys) / span(&xs)
            };
            near(ratio, 3.0, 1e-9, "the long axis against the short one");
        }
    }
    Ok(())
}

#[test]
fn a_diamond_has_its_corners_on_the_axes_of_the_rows_and_stays_in_its_cell() -> Outcome {
    for angle in [0.0_f64, 30.0, 75.0] {
        let (sin, cos) = (libm::sin(angle.to_radians()), libm::cos(angle.to_radians()));
        // in the frame of the rows: along, across
        let frame = |at: (f64, f64), centre: (f64, f64)| {
            let (dx, dy) = (at.0 - centre.0, at.1 - centre.1);
            (dx * cos + dy * sin, -dx * sin + dy * cos)
        };
        let found = dots(200.0, black(0.7).pitch(10.0).angle(Deg(angle)).shape(Mark::Diamond))?;
        let half = (0.3_f64 / 2.0).sqrt() * 10.0;
        for dot in &found {
            let centre = mean(dot);
            for &corner in dot {
                let (along, across) = frame(corner, centre);
                near(along.abs().min(across.abs()), 0.0, 1e-9, "a corner is on an axis");
                near(along.abs().max(across.abs()), half, 1e-9, "at half the diagonal");
            }
        }
        // past half the cover the corners reach the cell's sides and no further
        for dot in &dots(200.0, black(0.3).pitch(10.0).angle(Deg(angle)).shape(Mark::Diamond))? {
            let centre = centroid(dot);
            for &corner in dot {
                let (along, across) = frame(corner, centre);
                assert!(
                    along.abs() <= 5.0 + 1e-9 && across.abs() <= 5.0 + 1e-9,
                    "{along} {across}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn a_square_turns_with_the_rows_and_its_side_is_the_root_of_the_cover() -> Outcome {
    let found = dots(200.0, black(0.75).pitch(10.0).angle(Deg(30.0)).shape(Mark::Square))?;
    assert!(found.len() > 100);
    for dot in &found {
        assert_eq!(dot.len(), 4);
        for at in 0..4 {
            let (a, b) = (dot[at], dot[(at + 1) % 4]);
            near(
                libm::hypot(b.0 - a.0, b.1 - a.1),
                5.0,
                1e-9,
                "a side of sqrt(0.25) pitches",
            );
            // a side runs along 30 degrees, 120, 210 or 300
            let heading = libm::atan2(b.1 - a.1, b.0 - a.0).to_degrees().rem_euclid(90.0);
            near(heading, 30.0, 1e-7, "the heading of a side");
        }
    }
    Ok(())
}

#[test]
fn euclidean_is_round_at_low_cover_a_diamond_at_half_and_a_round_hole_at_high_cover() -> Outcome {
    let radius = |dot: &Ring, centre: (f64, f64)| -> Vec<f64> {
        dot.iter()
            .map(|p| libm::hypot(p.0 - centre.0, p.1 - centre.1))
            .collect()
    };
    for dot in &dots(200.0, black(0.9).pitch(10.0).angle(Deg(0.0)).shape(Mark::Euclidean))? {
        let centre = centroid(dot);
        let radii = radius(dot, centre);
        let (low, high) = (
            radii.iter().copied().fold(f64::MAX, f64::min),
            radii.iter().copied().fold(f64::MIN, f64::max),
        );
        assert!(high / low < 1.05, "round: {low} to {high}");
        near(
            low,
            (0.1_f64 / PI).sqrt() * 10.0,
            0.05,
            "the radius of a round dot of that cover",
        );
    }
    for dot in &dots(200.0, black(0.5).pitch(10.0).angle(Deg(0.0)).shape(Mark::Euclidean))? {
        let centre = centroid(dot);
        for &(x, y) in dot {
            near(
                (x - centre.0).abs() + (y - centre.1).abs(),
                5.0,
                0.01,
                "a corner of the diamond |x| + |y| = half a pitch",
            );
        }
    }
    // 90 % cover: the dot fills the middle of every side of its cell and leaves the cell's corners bare, round
    for dot in dots(200.0, black(0.1).pitch(10.0).angle(Deg(0.0)).shape(Mark::Euclidean))?
        .iter()
        .take(20)
    {
        let polygon = Path::polyline(dot.clone(), true)?;
        let centre = centroid(dot);
        assert!(polygon.contains((centre.0 + 4.99, centre.1))?);
        assert!(polygon.contains((centre.0, centre.1 - 4.99))?);
        assert!(!polygon.contains((centre.0 + 4.99, centre.1 + 4.99))?);
        assert!(!polygon.contains((centre.0 - 4.99, centre.1 + 4.99))?);
    }
    Ok(())
}

#[test]
fn a_caller_outline_is_centred_on_its_centroid_scaled_to_the_area_and_turned_with_the_rows() -> Outcome {
    for clockwise in [true, false] {
        for angle in [0.0, 90.0] {
            let found = dots(
                200.0,
                black(0.8)
                    .pitch(10.0)
                    .angle(Deg(angle))
                    .shape(Mark::path(triangle(clockwise)?)),
            )?;
            assert!(found.len() > 100);
            for dot in &found {
                // the lattice point of this dot is the nearest one: its centroid sits on it
                let centre = centroid(dot);
                let (sin, cos) = (libm::sin(angle.to_radians()), libm::cos(angle.to_radians()));
                let (along, across) = (centre.0 * cos + centre.1 * sin, -centre.0 * sin + centre.1 * cos);
                near(along / 10.0, (along / 10.0).round(), 1e-9, "on a column");
                near(across / 10.0, (across / 10.0).round(), 1e-9, "on a row");
                assert!(
                    shoelace_area(dot) > 0.0,
                    "clockwise on the screen, whichever way the caller drew it"
                );
                near(shoelace_area(dot), 20.0, 1e-9, "a fifth of the cell");
                // the long side (4 against 2) runs along the rows
                let (xs, ys): (Vec<f64>, Vec<f64>) = dot.iter().copied().unzip();
                let span = |values: &[f64]| {
                    values.iter().copied().fold(f64::MIN, f64::max) - values.iter().copied().fold(f64::MAX, f64::min)
                };
                let ratio = if angle == 0.0 {
                    span(&xs) / span(&ys)
                } else {
                    span(&ys) / span(&xs)
                };
                near(ratio, 2.0, 1e-9, "4 long to 2 high");
            }
        }
    }
    Ok(())
}

// The tone ------------------------------------------------------------------------------------------------------------

#[test]
fn a_ramp_tone_gives_each_dot_the_tone_at_its_centre() -> Outcome {
    let ramp = Field::ramp((0.0, 0.0), (300.0, 0.0))?;
    let found = dots(
        300.0,
        Halftone::field(BLACK, ramp)
            .pitch(10.0)
            .angle(Deg(0.0))
            .shape(Mark::Square),
    )?;
    assert!(found.len() > 500, "{} dots", found.len());
    for dot in &found {
        let x = mean(dot).0;
        near(
            shoelace_area(dot),
            (1.0 - x / 300.0) * 100.0,
            1e-7,
            &format!("at x = {x}"),
        );
    }
    Ok(())
}

#[test]
fn a_map_tone_is_read_at_the_centre_of_each_dot_on_every_screen() -> Outcome {
    let bytes: Vec<u8> = (0..16_u8)
        .flat_map(|row| (0..16_u8).map(move |column| 15 * column + row))
        .collect();
    let map = Field::map(
        16,
        16,
        &bytes,
        Some(Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
        }),
        Some("photo"),
    )?;
    for screen in [Screen::Grid, Screen::Wave, Screen::Rings, Screen::Spiral, Screen::Rays] {
        let found = dots(
            200.0,
            Halftone::field(BLACK, map.clone())
                .pitch(8.0)
                .shape(Mark::Square)
                .screen(screen),
        )?;
        assert!(found.len() > 100, "{}: {} dots", screen.name(), found.len());
        // the dot is a square of the area (1 - tone) of its own cell; the cell is unknown here, so the dot is read
        // against the field at its centre: a darker place never has the smaller dot, among dots of one pitch cell
        let mut pairs: Vec<(f64, f64)> = found
            .iter()
            .filter(|dot| {
                let (x, y) = mean(dot);
                (20.0..180.0).contains(&x) && (20.0..180.0).contains(&y)
            })
            .map(|dot| {
                let (x, y) = mean(dot);
                (map.at((x, y)), shoelace_area(dot))
            })
            .collect();
        pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (dark, light) = (pairs[..pairs.len() / 4].to_vec(), pairs[3 * pairs.len() / 4..].to_vec());
        #[expect(clippy::cast_precision_loss, reason = "a few hundred dots")]
        let mean_of = |rows: &[(f64, f64)]| rows.iter().map(|r| r.1).sum::<f64>() / rows.len() as f64;
        assert!(mean_of(&dark) > 1.5 * mean_of(&light), "{}", screen.name());
    }
    // on a grid the invariant is exact
    for dot in &dots(
        200.0,
        Halftone::field(BLACK, map.clone()).pitch(8.0).shape(Mark::Square),
    )? {
        let (x, y) = mean(dot);
        near(
            shoelace_area(dot),
            (1.0 - map.at((x, y))) * 64.0,
            1e-7,
            "a map at a grid dot",
        );
    }
    Ok(())
}

#[test]
fn a_constant_field_and_the_number_it_holds_draw_the_same_dots() -> Outcome {
    for screen in [Screen::Grid, Screen::Rings, Screen::Rays] {
        let number = dots(120.0, black(0.4).shape(Mark::Euclidean).screen(screen))?;
        let field = dots(
            120.0,
            Halftone::field(BLACK, Field::constant(0.4)?)
                .shape(Mark::Euclidean)
                .screen(screen),
        )?;
        assert_eq!(number, field, "{}", screen.name());
    }
    Ok(())
}

#[test]
fn the_style_of_a_drawn_shape_says_what_the_tone_is() -> Outcome {
    let by_number = Drawing::from(rect(50.0)?).halftone(black(0.3).screen(Screen::Rings).centre((10.0, 20.0)));
    let shapes = by_number.shapes(&Values::new())?;
    let style = shapes[0].style().halftone().ok_or("no halftone")?;
    assert_eq!(style.tone().level(), Some(0.3));
    assert!(style.tone().field().is_none());
    assert_eq!(style.screen(), Screen::Rings);
    assert_eq!(style.mark(), &Mark::Dot);
    assert_eq!(style.centre().map(|c| (c.x, c.y)), Some((10.0, 20.0)));
    assert_eq!((style.aspect(), style.amplitude(), style.wavelength()), (1.0, 1.0, 6.0));
    let ramp = Field::ramp((0.0, 0.0), (0.0, 50.0))?;
    let by_field = Drawing::from(rect(50.0)?).halftone(Halftone::field(BLACK, ramp));
    let shapes = by_field.shapes(&Values::new())?;
    let style = shapes[0].style().halftone().ok_or("no halftone")?;
    assert_eq!(style.tone().level(), None);
    near(style.tone().at((0.0, 25.0)), 0.5, 1e-12, "the field behind the tone");
    // the shape goes back into a drawing with its tone and is the same shape
    assert_eq!(Drawing::new(shapes.clone()).shapes(&Values::new())?, shapes);
    Ok(())
}

// The screens ---------------------------------------------------------------------------------------------------------

/// How far a point is from `centre`, and its angle about it in turns (0 up to 1, clockwise on the screen).
fn polar(at: (f64, f64), centre: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (at.0 - centre.0, at.1 - centre.1);
    (libm::hypot(dx, dy), libm::atan2(dy, dx).rem_euclid(TAU) / TAU)
}

#[test]
fn rings_put_cells_at_half_pitch_radii_from_the_centre_and_a_dot_in_each() -> Outcome {
    let found = dots(400.0, black(0.5).pitch(10.0).screen(Screen::Rings))?;
    let mut by_ring = std::collections::BTreeMap::<i64, Vec<&Ring>>::new();
    for dot in &found {
        let (radius, _) = polar(mean(dot), (200.0, 200.0));
        let ring = radius / 10.0 - 0.5;
        near(ring, ring.round(), 1e-9, "a dot at (k + 1/2) pitches from the centre");
        #[expect(clippy::cast_possible_truncation, reason = "a small ring number")]
        by_ring.entry(ring.round() as i64).or_default().push(dot);
    }
    for k in 0..=17_i64 {
        let ring = by_ring.get(&k).ok_or("a missing ring")?;
        #[expect(clippy::cast_precision_loss, reason = "a small ring number")]
        let middle = k as f64 + 0.5;
        let cells = 2.0 * (PI * middle).round();
        #[expect(clippy::cast_precision_loss, reason = "a few hundred dots")]
        let count = ring.len() as f64;
        near(count, cells, 0.0, &format!("the cells of ring {k}"));
        // the cell is the wedge of the ring: the mean radius times the angle times the pitch
        for dot in ring {
            near(
                shoelace_area(dot),
                0.5 * (middle * 10.0) * (TAU / cells) * 10.0,
                1e-7,
                "half of a wedge",
            );
        }
    }
    Ok(())
}

#[test]
fn rings_are_about_the_centre_the_caller_gives() -> Outcome {
    for dot in &dots(
        300.0,
        black(0.5).pitch(10.0).screen(Screen::Rings).centre((100.0, 150.0)),
    )? {
        let (radius, _) = polar(mean(dot), (100.0, 150.0));
        near(
            radius / 10.0 - 0.5,
            (radius / 10.0 - 0.5).round(),
            1e-9,
            "about (100, 150)",
        );
    }
    Ok(())
}

#[test]
fn rays_are_as_many_as_a_pitch_apart_round_the_farthest_corner_and_merge_towards_the_centre() -> Outcome {
    let found = dots(400.0, black(0.5).pitch(10.0).screen(Screen::Rays))?;
    let farthest = 200.0 * 2.0_f64.sqrt();
    let rays = (TAU * farthest / 10.0).ceil();
    let mut by_row = std::collections::BTreeMap::<i64, Vec<(f64, f64)>>::new();
    for dot in &found {
        let (radius, turns) = polar(mean(dot), (200.0, 200.0));
        let row = radius / 10.0 - 0.5;
        near(
            row,
            row.round(),
            1e-9,
            "a cell a pitch long about (m + 1/2) pitches out",
        );
        #[expect(clippy::cast_possible_truncation, reason = "a small row number")]
        by_row
            .entry(row.round() as i64)
            .or_default()
            .push((turns, shoelace_area(dot)));
    }
    // far out, where a ray is as wide as a pitch, the rays are all there
    #[expect(clippy::cast_precision_loss, reason = "a few hundred dots")]
    let far_row = by_row[&15].len() as f64;
    near(far_row, rays, 0.0, "rays at 155 px");
    for (row, cells) in &by_row {
        if *row > 17 {
            continue;
        }
        #[expect(clippy::cast_precision_loss, reason = "a few hundred dots")]
        let (count, radius) = (cells.len() as f64, (*row as f64 + 0.5) * 10.0);
        let width = TAU * radius / count;
        assert!(
            (5.0 - 1e-9..=10.0 + 1e-9).contains(&width),
            "row {row}: cells {width} px wide"
        );
        for &(_, area) in cells {
            near(area, 0.5 * 10.0 * width, 1e-7, "half of a wedge");
        }
    }
    Ok(())
}

#[test]
fn a_spiral_is_one_row_a_pitch_further_out_each_turn() -> Outcome {
    for angle in [0.0, 40.0] {
        let found = dots(400.0, black(0.5).pitch(10.0).angle(Deg(angle)).screen(Screen::Spiral))?;
        assert!(found.len() > 500);
        for dot in &found {
            let (radius, turns) = polar(mean(dot), (200.0, 200.0));
            let turned = (turns - angle / 360.0).rem_euclid(1.0);
            let arm = radius / 10.0 - turned - 0.5;
            near(arm, arm.round(), 1e-9, "radius = (arm + 1/2 + turns) pitches");
            // the arm of k + 1 cells to the turn of 2 round(pi (k + 1)): the wedge, half of it
            let cells = 2.0 * (PI * (arm.round() + 1.0)).round();
            near(
                shoelace_area(dot),
                0.5 * radius * (TAU / cells) * 10.0,
                1e-7,
                "half of a wedge of the arm",
            );
        }
    }
    Ok(())
}

#[test]
fn a_wave_swings_the_rows_by_amplitude_pitches_over_a_wavelength_of_pitches() -> Outcome {
    let (angle, pitch) = (20.0_f64, 10.0);
    let (sin, cos) = (libm::sin(angle.to_radians()), libm::cos(angle.to_radians()));
    let found = dots(
        300.0,
        black(0.5)
            .pitch(pitch)
            .angle(Deg(angle))
            .screen(Screen::Wave)
            .amplitude(1.5)
            .wavelength(8.0),
    )?;
    assert!(found.len() > 500);
    for dot in &found {
        let (x, y) = mean(dot);
        let (along, across) = (x * cos + y * sin, -x * sin + y * cos);
        near(
            along / pitch,
            (along / pitch).round(),
            1e-9,
            "a dot every pitch along the row",
        );
        let row = (across - 1.5 * pitch * libm::sin(TAU * along / (8.0 * pitch))) / pitch;
        near(row, row.round(), 1e-9, "on a row displaced by the sine");
        near(
            shoelace_area(dot),
            50.0,
            1e-7,
            "a dot covers half of its cell, which keeps the area of a square",
        );
    }
    Ok(())
}

#[test]
fn the_default_screen_is_the_lattice_it_always_was() -> Outcome {
    let found = dots(100.0, black(0.5).pitch(10.0).angle(Deg(0.0)))?;
    for dot in &found {
        let (x, y) = mean(dot);
        near(x / 10.0, (x / 10.0).round(), 1e-9, "on a column");
        near(y / 10.0, (y / 10.0).round(), 1e-9, "on a row");
    }
    Ok(())
}

// Refusals ------------------------------------------------------------------------------------------------------------

#[test]
fn what_the_screens_cannot_draw_is_refused_naming_the_argument() -> Outcome {
    let base = Drawing::from(rect(100.0)?);
    let size = Size::px(100.0, 100.0);
    let cases = [
        ("aspect below 1", black(0.5).shape(Mark::Ellipse).aspect(0.5), "aspect"),
        (
            "a NaN aspect",
            black(0.5).shape(Mark::Ellipse).aspect(f64::NAN),
            "aspect",
        ),
        ("a centre on a grid", black(0.5).centre((1.0, 1.0)), "centre"),
        (
            "a centre on a wave",
            black(0.5).screen(Screen::Wave).centre((1.0, 1.0)),
            "centre",
        ),
        (
            "a wavelength of 0",
            black(0.5).screen(Screen::Wave).wavelength(0.0),
            "wavelength",
        ),
        (
            "an infinite amplitude",
            black(0.5).screen(Screen::Wave).amplitude(f64::INFINITY),
            "amplitude",
        ),
        (
            "an infinite centre",
            black(0.5).screen(Screen::Rings).centre((f64::INFINITY, 0.0)),
            "centre",
        ),
    ];
    for (case, halftone, argument) in cases {
        let drawing = base.halftone(halftone);
        let error = drawing
            .error()
            .cloned()
            .or_else(|| drawing.to_svg(size, &Values::new()).err());
        let (name, _) = parameter_refusal(Err::<(), _>(error.ok_or(format!("{case}: no error"))?))?;
        assert!(name.contains(argument), "{case}: {name}");
    }
    Ok(())
}

#[test]
fn a_caller_outline_that_cannot_be_a_dot_is_refused_naming_the_mark() -> Outcome {
    let base = Drawing::from(rect(100.0)?);
    let cases = [
        ("a line", "M0 0 L5 5"),
        ("an open outline", "M0 0 L4 0 L4 4 L0 4"),
        ("no area", "M0 0 L4 0 L8 0 Z"),
        ("two outlines", "M0 0 L4 0 L0 2 Z M10 0 L14 0 L10 2 Z"),
        ("a point", "M3 3 Z"),
        ("nothing", ""),
    ];
    for (case, data) in cases {
        let drawing = base.halftone(black(0.5).shape(Mark::path(Path::from_svg_path(data)?)));
        let error = drawing.error().cloned().ok_or(format!("{case}: no error"))?;
        let (name, _) = parameter_refusal(Err::<(), _>(error))?;
        assert!(name.contains("mark"), "{case}: {name}");
    }
    Ok(())
}

#[test]
fn a_pitch_too_small_for_the_shape_is_refused_before_any_dot_is_made_on_every_screen() -> Outcome {
    for screen in [Screen::Grid, Screen::Wave, Screen::Rings, Screen::Spiral, Screen::Rays] {
        for mark in [Mark::Dot, Mark::Euclidean] {
            let drawing = Drawing::from(rect(1000.0)?).halftone(black(0.5).pitch(0.001).shape(mark).screen(screen));
            let (name, message) = parameter_refusal(drawing.to_svg(Size::px(100.0, 100.0), &Values::new()))?;
            assert!(
                name.contains("pitch") && message.contains("MAX_SEGMENTS"),
                "{name}: {message}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_huge_aspect_is_refused_by_the_work_it_would_take() -> Outcome {
    let drawing = Drawing::from(rect(100.0)?).halftone(black(0.5).shape(Mark::Ellipse).aspect(1e100));
    let (name, _) = parameter_refusal(drawing.to_svg(Size::px(100.0, 100.0), &Values::new()))?;
    assert!(name.contains("pitch"), "{name}");
    Ok(())
}

// Documents -----------------------------------------------------------------------------------------------------------

#[test]
fn a_document_with_every_argument_reads_back_and_writes_the_same_text() -> Outcome {
    let drawings = [
        Drawing::from(rect(50.0)?).halftone(
            black(0.4)
                .pitch(5.0)
                .shape(Mark::Ellipse)
                .aspect(2.0)
                .screen(Screen::Rings)
                .centre((10.0, 20.0)),
        ),
        Drawing::from(rect(50.0)?).halftone(black(0.4).screen(Screen::Wave).amplitude(2.0).wavelength(9.0)),
        Drawing::from(rect(50.0)?)
            .halftone(Halftone::field(BLACK, Field::ramp((0.0, 0.0), (0.0, 50.0))?).shape(Mark::Euclidean)),
        Drawing::from(rect(50.0)?).halftone(black(0.4).shape(Mark::path(triangle(true)?)).screen(Screen::Spiral)),
    ];
    for drawing in drawings {
        let text = drawing.to_json()?;
        let back = Drawing::from_json(&text, &Inputs::new())?;
        assert_eq!(back.to_json()?, text);
        assert_eq!(back.shapes(&Values::new())?, drawing.shapes(&Values::new())?, "{text}");
    }
    let text = Drawing::from(rect(50.0)?)
        .halftone(black(0.4).shape(Mark::path(triangle(true)?)))
        .to_json()?;
    assert!(
        text.contains(r#""mark":"M0 0 L4 0 L0 2 Z""#) && text.contains(r#""shape":"path""#),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_document_naming_an_unknown_shape_or_screen_lists_the_valid_ones() -> Outcome {
    for (argument, text, listed) in [
        (
            "shape",
            r##"{"halftone":{"color":"#000000","tone":0.5,"shape":"star"}}"##,
            r#""dot", "square""#,
        ),
        (
            "screen",
            r##"{"halftone":{"color":"#000000","tone":0.5,"screen":"zigzag"}}"##,
            r#""grid", "wave""#,
        ),
    ] {
        let document = format!(r#"{{"version":1,"root":[{{"path":"M0 0 L9 0 L9 9 L0 9 Z"}},{text}]}}"#);
        let error = Drawing::from_json(&document, &Inputs::new()).err().ok_or("accepted")?;
        let message = error.to_string();
        assert!(message.contains(argument) && message.contains(listed), "{message}");
    }
    Ok(())
}

// Pixels --------------------------------------------------------------------------------------------------------------

#[test]
fn the_svg_and_the_pixels_agree_on_every_shape_and_screen() -> Outcome {
    let rows = [
        (Mark::Euclidean, Screen::Rings),
        (Mark::Ellipse, Screen::Wave),
        (Mark::Diamond, Screen::Spiral),
        (Mark::Square, Screen::Rays),
        (Mark::path(triangle(true)?), Screen::Grid),
    ];
    for (mark, screen) in rows {
        let name = format!("{} on {}", mark.name(), screen.name());
        let paper = Drawing::from(rect(120.0)?).fill(Color::hex(0xff_ff_ff));
        // the round screens are about a point off the disc: the sub-pixel dots at a centre are where two renderers differ
        let screened = black(0.7).pitch(7.0).shape(mark).aspect(1.4).screen(screen);
        let screened = if matches!(screen, Screen::Grid | Screen::Wave) {
            screened
        } else {
            screened.centre((10.0, 110.0))
        };
        let dotted = Drawing::from(Path::circle((60.0, 60.0), 52.0)?).halftone(screened);
        let drawing = Drawing::new([paper, dotted]).with_size(120.0, 120.0);
        let agreement = svg_agrees(&drawing, 120, 120)?;
        // a different dot anywhere would differ by 255; the two renderers cover the sharp corner of a turned square or a
        // sub-pixel diamond differently by up to 10 levels
        assert!(
            agreement.interior <= 12 && agreement.edge_over_40 * 100 <= agreement.edge_pixels,
            "{name}: {agreement:?}"
        );
    }
    Ok(())
}

#[test]
fn the_reference_document_reads_its_tone_from_a_map_and_draws_dots_that_follow_it() -> Outcome {
    // docs/examples/json/halftone.json: a portrait map for a tone; here a ramp, dark at the left and bright at the right
    let text = include_str!("../../../../docs/examples/json/halftone.json");
    let bytes: Vec<u8> = (0..675_u32)
        .flat_map(|_| (0..540_u32).map(|column| u8::try_from(column * 255 / 539).unwrap_or(255)))
        .collect();
    let drawing = Drawing::from_json(text, &Inputs::new().set("portrait", bytes))?;
    let svg = drawing.to_svg(Size::px(1080.0, 1350.0), &Values::new())?;
    let found = dots_of(&svg)?;
    assert!(found.len() > 2000, "{} dots", found.len());
    // darker (lower tone) dots are bigger: the mean area of the dots in the left fifth against the right fifth
    let mean_area = |from: f64, to: f64| {
        let chosen: Vec<f64> = found
            .iter()
            .filter(|dot| (from..to).contains(&mean(dot).0))
            .map(|dot| shoelace_area(dot))
            .collect();
        #[expect(clippy::cast_precision_loss, reason = "a few thousand dots")]
        let count = chosen.len() as f64;
        chosen.iter().sum::<f64>() / count
    };
    assert!(mean_area(150.0, 330.0) > 3.0 * mean_area(750.0, 930.0));
    Ok(())
}
