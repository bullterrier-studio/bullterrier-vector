//! `Path::bulge` through the public API: a radial remap about `about`, a point at distance `r` going to
//! `r (1 + amount * falloff(r))` along the same ray, the falloff `1 - s^2 (3 - 2s)`, `s = min(r / radius, 1)`. The
//! oracle is that closed form written out here from the definition, checked against the output polyline by brute
//! force; the expected vertices are computed by hand. `r -> r'` has the slope `1 + amount (1 - 9 s^2 + 8 s^3)`, whose
//! hand-computed extremes are 1 (at s = 0) and -11/16 (at s = 3/4): strictly increasing exactly for
//! `-1 < amount < 16/11`.

use crate::common::{Outcome, has_vertex, lines, parameter_refusal, radial_probe_edges, to_lines, worst_chord_error};
use bullterrier_vector::{Path, Point};
use proptest::prelude::*;

fn falloff(r: f64, radius: f64) -> f64 {
    let s = (r / radius).min(1.0);
    1.0 - s * s * (3.0 - 2.0 * s)
}

/// Where the closed form sends `p`.
fn exact(p: (f64, f64), amount: f64, radius: f64, about: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (p.0 - about.0, p.1 - about.1);
    let r = libm::hypot(dx, dy);
    if r >= radius {
        return p;
    }
    let factor = 1.0 + amount * falloff(r, radius);
    (about.0 + dx * factor, about.1 + dy * factor)
}

fn assert_bulged_matches_the_closed_form(path: &Path, amount: f64, radius: f64, about: (f64, f64)) -> Outcome {
    let bulged = lines(&path.bulge(amount, Some(radius), Some(Point::new(about.0, about.1)))?)?;
    for index in 0..=600 {
        let original = path.point_at(f64::from(index) / 600.0, None)?;
        let image = exact((original.x, original.y), amount, radius, about);
        let off = to_lines(image, &bulged);
        assert!(
            off <= 0.01 * radius + 0.01,
            "sample {index}: the exact image {image:?} is {off} from the output"
        );
    }
    Ok(())
}

// By hand --------------------------------------------------------------------------------------------------------------

#[test]
fn a_point_at_half_the_radius_goes_to_one_and_a_half_times_as_far_by_hand() -> Outcome {
    // s = 0.5, falloff 0.5, amount 1: r' = r (1 + 0.5) = 75 for r = 50 of a radius of 100; the direction (3, 4) / 5 stays
    let path = Path::from_svg_path("M30 40")?;
    let bulged = path.bulge(1.0, Some(100.0), Some(Point::new(0.0, 0.0)))?;
    let (x, y) = lines(&bulged)?[0].0[0];
    assert!((x - 45.0).abs() < 1e-9 && (y - 60.0).abs() < 1e-9, "({x}, {y})");
    Ok(())
}

#[test]
fn a_negative_amount_pinches_towards_the_center_by_hand() -> Outcome {
    // amount -0.5 at s = 0.5: r' = r (1 - 0.25) = 37.5 for r = 50
    let bulged = Path::from_svg_path("M50 0")?.bulge(-0.5, Some(100.0), Some(Point::new(0.0, 0.0)))?;
    let (x, y) = lines(&bulged)?[0].0[0];
    assert!((x - 37.5).abs() < 1e-9 && y.abs() < 1e-9, "({x}, {y})");
    Ok(())
}

#[test]
fn points_at_or_beyond_the_radius_do_not_move_to_the_bit_and_the_center_stays() -> Outcome {
    let path = Path::from_svg_path("M0 0 L100 0 L130.5 40.25")?;
    let bulged = lines(&path.bulge(1.2, Some(100.0), Some(Point::new(0.0, 0.0)))?)?;
    assert!(bulged[0].0.contains(&(0.0, 0.0)));
    assert!(bulged[0].0.contains(&(100.0, 0.0)));
    assert!(bulged[0].0.contains(&(130.5, 40.25)));
    Ok(())
}

#[test]
fn a_point_beyond_the_radius_is_returned_whole_though_adding_back_its_offset_would_round_it() -> Outcome {
    // 8.8 + (51.9 - 8.8) is 51.89999999999999, not 51.9: a point moved by a factor of 1 is not a point left alone
    let path = Path::from_svg_path("M51.9 20.3 L60.7 22.1")?;
    let bulged = lines(&path.bulge(1.2, Some(10.0), Some(Point::new(8.8, 3.1)))?)?;
    assert!(
        bulged[0].0.contains(&(51.9, 20.3)) && bulged[0].0.contains(&(60.7, 22.1)),
        "{:?}",
        bulged[0].0
    );
    Ok(())
}

#[test]
fn points_move_along_their_ray_and_an_amount_of_zero_is_the_outline_flattened() -> Outcome {
    let star = Path::from_svg_path("M0 0 L80 0 M0 0 L30 40")?;
    let bulged = lines(&star.bulge(0.9, Some(100.0), Some(Point::new(0.0, 0.0)))?)?;
    for v in &bulged[1].0 {
        // the ray of (3, 4): the cross product with the direction is 0
        assert!((v.0 * 4.0 - v.1 * 3.0).abs() < 1e-9, "{v:?}");
    }
    let flat = lines(&star.bulge(0.0, Some(100.0), Some(Point::new(0.0, 0.0)))?)?;
    assert!(has_vertex(&flat, (80.0, 0.0)) && has_vertex(&flat, (30.0, 40.0)));
    for ((points, _), length) in flat.iter().zip([80.0, 50.0]) {
        let pieces = f64::from(u32::try_from(points.len() - 1)?);
        for (index, v) in (0_u32..).zip(points) {
            let want = length * f64::from(index) / pieces;
            assert!((libm::hypot(v.0, v.1) - want).abs() < 1e-9, "{index}: {v:?}");
        }
    }
    Ok(())
}

#[test]
fn a_ray_stays_in_order_up_to_the_strongest_amount_and_beyond_it_is_refused() -> Outcome {
    let ray = Path::line((0.0, 0.0), (120.0, 0.0))?;
    for amount in [1.45, -0.99, 0.5, 0.0] {
        let bulged = lines(&ray.bulge(amount, Some(100.0), Some(Point::new(0.0, 0.0)))?)?;
        for pair in bulged[0].0.windows(2) {
            assert!(pair[1].0 > pair[0].0, "amount {amount}: {pair:?}");
        }
    }
    for amount in [1.46, 16.0 / 11.0, -1.0, -1.5, f64::NAN, f64::INFINITY] {
        assert_eq!(
            parameter_refusal(ray.bulge(amount, Some(100.0), None))?.0,
            "amount",
            "{amount}"
        );
    }
    let (_, message) = parameter_refusal(ray.bulge(2.0, None, None))?;
    assert!(message.contains("16/11") && message.contains("-1"), "{message}");
    Ok(())
}

#[test]
fn the_default_pivot_is_the_middle_of_the_bounds_and_the_radius_reaches_the_far_corner() -> Outcome {
    let rect = Path::rect(10.0, 20.0, 100.0, 60.0, 0.0)?;
    let bulged = lines(&rect.bulge(0.8, None, None)?)?;
    for corner in [(10.0, 20.0), (110.0, 20.0), (110.0, 80.0), (10.0, 80.0)] {
        assert!(has_vertex(&bulged, corner), "{corner:?}");
    }
    let radius = libm::hypot(50.0, 30.0);
    let given = rect.bulge(0.8, Some(radius), Some(Point::new(60.0, 50.0)))?;
    assert_eq!(given, rect.bulge(0.8, None, None)?);
    // the middle of the top edge, 30 from the middle, goes out along the ray (0, -1)
    let image = exact((60.0, 20.0), 0.8, radius, (60.0, 50.0));
    assert!(image.1 < 20.0 - 1.0 && to_lines(image, &bulged) <= 0.01 * radius + 0.01);
    Ok(())
}

#[test]
fn a_closed_path_stays_closed_an_open_one_open_a_point_is_kept_and_an_empty_path_stays_empty() -> Outcome {
    assert!(
        lines(&Path::rect(0.0, 0.0, 50.0, 50.0, 0.0)?.bulge(0.5, None, None)?)?
            .iter()
            .all(|line| line.1)
    );
    assert!(
        lines(&Path::from_svg_path("M0 0 L50 0 L50 50")?.bulge(0.5, None, None)?)?
            .iter()
            .all(|line| !line.1)
    );
    assert_eq!(
        Path::from_svg_path("M30 5")?.bulge(0.5, None, None)?.to_svg_path(),
        "M30 5"
    );
    assert_eq!(Path::new().bulge(0.5, None, None)?, Path::new());
    Ok(())
}

// The closed form ----------------------------------------------------------------------------------------------------------

#[test]
fn a_circle_a_rectangle_and_a_line_match_the_closed_form() -> Outcome {
    assert_bulged_matches_the_closed_form(&Path::circle((50.0, 50.0), 40.0)?, 1.0, 60.0, (50.0, 50.0))?;
    assert_bulged_matches_the_closed_form(&Path::circle((50.0, 50.0), 40.0)?, -0.8, 60.0, (50.0, 50.0))?;
    assert_bulged_matches_the_closed_form(&Path::rect(0.0, 0.0, 100.0, 100.0, 0.0)?, 1.45, 71.0, (50.0, 50.0))?;
    assert_bulged_matches_the_closed_form(&Path::line((-80.0, 3.0), (90.0, 20.0))?, 1.2, 80.0, (10.0, 0.0))?;
    Ok(())
}

proptest! {
    /// A rectangle anywhere, bulged any way: the exact images stay near the output and its corners are vertices.
    #[test]
    fn a_bulged_rectangle_matches_the_closed_form(
        (x, y) in (-200.0..200.0_f64, -200.0..200.0_f64),
        (width, height) in (1.0..300.0_f64, 1.0..300.0_f64),
        amount in -0.99..1.45_f64,
        radius in 20.0..400.0_f64,
        about in (-100.0..100.0_f64, -100.0..100.0_f64),
    ) {
        let rect = Path::rect(x, y, width, height, 0.0).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let check = assert_bulged_matches_the_closed_form(&rect, amount, radius, about);
        prop_assert!(check.is_ok(), "{check:?}");
        let bulged = lines(&rect.bulge(amount, Some(radius), Some(Point::new(about.0, about.1))).map_err(|e| TestCaseError::fail(e.to_string()))?)
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        for corner in [(x, y), (x + width, y), (x + width, y + height), (x, y + height)] {
            let image = exact(corner, amount, radius, about);
            prop_assert!(has_vertex(&bulged, image), "no vertex at {image:?}");
        }
    }
}

/// The pieces are chosen so that a chord strays from the image by at most `CHORD_ERROR = 0.005` of the radius (the bound
/// is worked in `radial.rs`); the extremes of the amount are where it is nearest, so a cut with half the pieces (four
/// times the error) must fail here, which the hundredth of the tests above does not show.
#[test]
fn a_chord_strays_from_the_exact_image_by_at_most_half_a_hundredth_of_the_radius_at_the_extremes() -> Outcome {
    for value in [1.45_f64, -0.99, 0.5, -0.2] {
        let mut worst = 0.0_f64;
        for (from, to) in radial_probe_edges() {
            let path = Path::line(from, to)?;
            let output = lines(&path.bulge(value, Some(100.0), Some(Point::new(0.0, 0.0)))?)?;
            let image = |p: (f64, f64)| exact(p, value, 100.0, (0.0, 0.0));
            worst = worst.max(worst_chord_error(from, to, &output[0].0, &image)?);
        }
        assert!(
            worst <= 0.005 * 100.0,
            "amount {value}: the chord is {worst} from the image"
        );
    }
    Ok(())
}

// Refusals ---------------------------------------------------------------------------------------------------------------

#[test]
fn a_radius_or_a_point_the_verb_cannot_use_is_refused_by_name() -> Outcome {
    let line = Path::line((0.0, 0.0), (10.0, 0.0))?;
    for radius in [0.0, -1.0, f64::NAN, f64::INFINITY, 1.000_000_1e100] {
        assert_eq!(
            parameter_refusal(line.bulge(0.5, Some(radius), None))?.0,
            "radius",
            "{radius}"
        );
    }
    assert!(line.bulge(0.5, Some(1e100), None).is_ok());
    assert_eq!(
        parameter_refusal(line.bulge(0.5, None, Some(Point::new(f64::NAN, 0.0))))?.0,
        "about"
    );
    assert_eq!(
        parameter_refusal(line.bulge(0.5, None, Some(Point::new(0.0, 2e100))))?.0,
        "about"
    );
    Ok(())
}

#[test]
fn a_radius_that_would_make_more_than_ten_million_pieces_is_refused_at_once() -> Outcome {
    let line = Path::line((0.0, 0.0), (1000.0, 0.0))?;
    let (name, message) = parameter_refusal(line.bulge(0.5, Some(1e-9), Some(Point::new(0.0, 0.0))))?;
    assert_eq!(name, "radius");
    assert!(message.contains("MAX_SEGMENTS"), "{message}");
    assert_eq!(parameter_refusal(line.bulge(0.5, Some(5e-324), None))?.0, "radius");
    Ok(())
}

#[test]
fn a_big_bulge_is_within_a_twentieth_of_a_pixel_of_the_image() -> Outcome {
    let (radius, about) = (1000.0, (0.0, 0.0));
    let line = Path::line((-1200.0, 300.0), (1200.0, 300.0))?;
    let bulged = lines(&line.bulge(1.2, Some(radius), Some(Point::new(0.0, 0.0)))?)?;
    let worst = (0..=24_000)
        .map(|index| {
            let image = exact((-1200.0 + f64::from(index) * 0.1, 300.0), 1.2, radius, about);
            to_lines(image, &bulged)
        })
        .fold(0.0_f64, f64::max);
    assert!(worst <= 0.05, "the image is {worst} pixels from the output");
    Ok(())
}
