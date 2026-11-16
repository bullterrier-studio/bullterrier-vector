//! `Path::twist` through the public API. A point at distance `r` from `about` turns by `angle * falloff(r)` degrees,
//! clockwise on the screen (`y` down) as `rotate` turns; the falloff is `1 - s^2 (3 - 2s)`, `s = min(r / radius, 1)`.
//! The oracle is that closed form written out here from the definition, checked against the output polyline by brute
//! force; the expected vertices are computed by hand.

use crate::common::{Outcome, has_vertex, lines, parameter_refusal, to_lines};
use bullterrier_vector::{Path, Point};
use proptest::prelude::*;

fn falloff(r: f64, radius: f64) -> f64 {
    let s = (r / radius).min(1.0);
    1.0 - s * s * (3.0 - 2.0 * s)
}

/// Where the closed form sends `p`.
fn exact(p: (f64, f64), angle: f64, radius: f64, about: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (p.0 - about.0, p.1 - about.1);
    let r = libm::hypot(dx, dy);
    if r >= radius {
        return p;
    }
    let turn = (angle * falloff(r, radius)).to_radians();
    let (sin, cos) = (libm::sin(turn), libm::cos(turn));
    (about.0 + dx * cos - dy * sin, about.1 + dx * sin + dy * cos)
}

/// Every exact image of a dense sampling of `path` is within a hundredth of the radius (and a hundredth of a pixel for
/// the flattening) of the twisted outline.
fn assert_twisted_matches_the_closed_form(path: &Path, angle: f64, radius: f64, about: (f64, f64)) -> Outcome {
    let twisted = lines(&path.twist(angle, Some(radius), Some(Point::new(about.0, about.1)))?)?;
    for index in 0..=600 {
        let original = path.point_at(f64::from(index) / 600.0, None)?;
        let image = exact((original.x, original.y), angle, radius, about);
        let off = to_lines(image, &twisted);
        assert!(
            off <= 0.01 * radius + 0.01,
            "sample {index}: the exact image {image:?} is {off} from the output"
        );
    }
    Ok(())
}

// By hand --------------------------------------------------------------------------------------------------------------

#[test]
fn a_point_near_the_centre_turns_by_the_whole_angle_clockwise_on_the_screen() -> Outcome {
    // r = 1 of a radius of 1e6: falloff 1 - 3e-12, so (1, 0) goes to (0, 1) (down the screen) to 1e-11, as rotate(90)
    let point = Path::from_svg_path("M1 0")?;
    let twisted = point.twist(90.0, Some(1e6), Some(Point::new(0.0, 0.0)))?;
    let rotated = point.rotate(90.0, Some(Point::new(0.0, 0.0)))?;
    let (a, b) = (lines(&twisted)?, lines(&rotated)?);
    assert!(
        (a[0].0[0].0 - b[0].0[0].0).abs() < 1e-9 && (a[0].0[0].1 - b[0].0[0].1).abs() < 1e-9,
        "{twisted:?} against {rotated:?}"
    );
    assert!(a[0].0[0].0.abs() < 1e-9 && (a[0].0[0].1 - 1.0).abs() < 1e-9);
    Ok(())
}

#[test]
fn a_point_at_half_the_radius_turns_by_half_the_angle_by_hand() -> Outcome {
    // s = 0.5, falloff 1 - 0.25 * 2 = 0.5: 90 degrees turn 45 degrees; (50, 0) about the origin, radius 100
    let twisted = Path::from_svg_path("M50 0")?.twist(90.0, Some(100.0), Some(Point::new(0.0, 0.0)))?;
    let (x, y) = lines(&twisted)?[0].0[0];
    let half = 50.0 * std::f64::consts::FRAC_1_SQRT_2;
    assert!((x - half).abs() < 1e-9 && (y - half).abs() < 1e-9, "({x}, {y})");
    Ok(())
}

#[test]
fn points_at_or_beyond_the_radius_do_not_move_and_the_centre_stays() -> Outcome {
    let path = Path::from_svg_path("M0 0 L100 0 L130 40")?;
    let twisted = lines(&path.twist(170.0, Some(100.0), Some(Point::new(0.0, 0.0)))?)?;
    assert!(has_vertex(&twisted, (0.0, 0.0)));
    assert!(has_vertex(&twisted, (100.0, 0.0)));
    assert!(has_vertex(&twisted, (130.0, 40.0)));
    Ok(())
}

#[test]
fn the_default_pivot_is_the_middle_of_the_bounds_and_the_radius_reaches_the_far_corner() -> Outcome {
    // a 100 x 60 rectangle: the middle (60, 50), the corners 58.31 away, so they do not move at all
    let rect = Path::rect(10.0, 20.0, 100.0, 60.0, 0.0)?;
    let twisted = lines(&rect.twist(90.0, None, None)?)?;
    for corner in [(10.0, 20.0), (110.0, 20.0), (110.0, 80.0), (10.0, 80.0)] {
        assert!(has_vertex(&twisted, corner), "{corner:?}");
    }
    // a vertex of the cut: the middle of the top edge, 30 from the middle, falloff 1 - s^2 (3 - 2s), s = 30 / 58.3095
    let radius = libm::hypot(50.0, 30.0);
    let s = 30.0 / radius;
    let turn = (90.0 * (1.0 - s * s * (3.0 - 2.0 * s))).to_radians();
    let image = (60.0 + 30.0 * libm::sin(turn), 50.0 - 30.0 * libm::cos(turn));
    assert!(to_lines(image, &twisted) <= 0.01 * radius + 0.01, "{image:?}");
    // the same with the pivot and the radius given: the identical output
    let given = rect.twist(90.0, Some(libm::hypot(50.0, 30.0)), Some(Point::new(60.0, 50.0)))?;
    assert_eq!(given, rect.twist(90.0, None, None)?);
    Ok(())
}

#[test]
fn an_angle_of_zero_is_the_outline_flattened() -> Outcome {
    let rect = Path::rect(10.0, 20.0, 100.0, 60.0, 0.0)?;
    let flat = lines(&rect.twist(0.0, None, None)?)?;
    assert_eq!(flat[0].0.len(), 4);
    for corner in [(10.0, 20.0), (110.0, 20.0), (110.0, 80.0), (10.0, 80.0)] {
        assert!(has_vertex(&flat, corner));
    }
    Ok(())
}

#[test]
fn a_closed_path_stays_closed_and_an_open_one_open() -> Outcome {
    let twisted = lines(&Path::rect(0.0, 0.0, 50.0, 50.0, 0.0)?.twist(90.0, None, None)?)?;
    assert!(twisted.iter().all(|line| line.1));
    let twisted = lines(&Path::from_svg_path("M0 0 L50 0 L50 50")?.twist(90.0, None, None)?)?;
    assert!(twisted.iter().all(|line| !line.1));
    Ok(())
}

#[test]
fn a_lone_point_and_an_empty_path_are_kept() -> Outcome {
    assert_eq!(
        Path::from_svg_path("M30 5")?.twist(90.0, None, None)?.to_svg_path(),
        "M30 5"
    );
    assert_eq!(Path::new().twist(90.0, None, None)?, Path::new());
    Ok(())
}

// The closed form and the invariants -------------------------------------------------------------------------------------

#[test]
fn a_circle_a_line_and_a_big_turn_match_the_closed_form() -> Outcome {
    assert_twisted_matches_the_closed_form(&Path::rect(0.0, 0.0, 100.0, 100.0, 0.0)?, 90.0, 71.0, (50.0, 50.0))?;
    assert_twisted_matches_the_closed_form(&Path::line((-80.0, 3.0), (90.0, 20.0))?, 180.0, 80.0, (0.0, 0.0))?;
    assert_twisted_matches_the_closed_form(&Path::circle((10.0, 10.0), 60.0)?, -270.0, 90.0, (20.0, 0.0))?;
    assert_twisted_matches_the_closed_form(&Path::line((-100.0, 0.0), (100.0, 0.0))?, 1080.0, 100.0, (0.0, 0.0))?;
    Ok(())
}

proptest! {
    /// Any rectangle twisted any way: the exact images stay near the output, corners are vertices, and every output
    /// vertex is as far from the pivot as the point it came from (a twist only turns).
    #[test]
    fn a_twisted_rectangle_matches_the_closed_form_and_keeps_every_distance(
        (x, y) in (-200.0..200.0_f64, -200.0..200.0_f64),
        (width, height) in (1.0..300.0_f64, 1.0..300.0_f64),
        angle in -540.0..540.0_f64,
        radius in 20.0..400.0_f64,
        about in (-100.0..100.0_f64, -100.0..100.0_f64),
    ) {
        let rect = Path::rect(x, y, width, height, 0.0).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let check = assert_twisted_matches_the_closed_form(&rect, angle, radius, about);
        prop_assert!(check.is_ok(), "{check:?}");
        let twisted = lines(&rect.twist(angle, Some(radius), Some(Point::new(about.0, about.1))).map_err(|e| TestCaseError::fail(e.to_string()))?)
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        for corner in [(x, y), (x + width, y), (x + width, y + height), (x, y + height)] {
            let image = exact(corner, angle, radius, about);
            prop_assert!(has_vertex(&twisted, image), "no vertex at {image:?}");
        }
    }
}

#[test]
fn every_vertex_keeps_its_distance_to_the_pivot() -> Outcome {
    // rays from the pivot: a ray of length L cut into k equal pieces has its vertices at distances L i / k, and a twist
    // only turns, so the output vertices of each ray are at exactly those distances
    let star = Path::from_svg_path("M0 0 L80 0 M0 0 L0 -60 M0 0 L30 40")?;
    let lengths = [80.0, 60.0, 50.0];
    let twisted = lines(&star.twist(200.0, Some(100.0), Some(Point::new(0.0, 0.0)))?)?;
    assert_eq!(twisted.len(), 3);
    for ((points, _), length) in twisted.iter().zip(lengths) {
        let pieces = f64::from(u32::try_from(points.len() - 1)?);
        for (index, v) in (0_u32..).zip(points) {
            let want = length * f64::from(index) / pieces;
            assert!(
                (libm::hypot(v.0, v.1) - want).abs() <= 1e-9 * length,
                "{index}: {v:?} against {want}"
            );
        }
    }
    Ok(())
}

// Refusals ---------------------------------------------------------------------------------------------------------------

#[test]
fn a_number_the_verb_cannot_use_is_refused_by_name() -> Outcome {
    let line = Path::line((0.0, 0.0), (10.0, 0.0))?;
    for radius in [0.0, -1.0, f64::NAN, f64::INFINITY, 1.000_000_1e100] {
        assert_eq!(
            parameter_refusal(line.twist(90.0, Some(radius), None))?.0,
            "radius",
            "{radius}"
        );
    }
    assert!(line.twist(90.0, Some(1e100), None).is_ok());
    for angle in [f64::NAN, f64::INFINITY] {
        assert_eq!(parameter_refusal(line.twist(angle, None, None))?.0, "angle");
    }
    assert_eq!(
        parameter_refusal(line.twist(90.0, None, Some(Point::new(f64::NAN, 0.0))))?.0,
        "about"
    );
    assert_eq!(
        parameter_refusal(line.twist(90.0, None, Some(Point::new(2e100, 0.0))))?.0,
        "about"
    );
    Ok(())
}

#[test]
fn a_radius_or_an_angle_that_would_make_more_than_ten_million_pieces_is_refused_at_once() -> Outcome {
    let line = Path::line((0.0, 0.0), (1000.0, 0.0))?;
    let (name, message) = parameter_refusal(line.twist(90.0, Some(1e-9), Some(Point::new(0.0, 0.0))))?;
    assert_eq!(name, "radius");
    assert!(message.contains("MAX_SEGMENTS"), "{message}");
    assert_eq!(parameter_refusal(line.twist(90.0, Some(5e-324), None))?.0, "radius");
    // the turn sets the step too: a thousand million turns cannot be cut
    assert_eq!(parameter_refusal(line.twist(1e300, Some(100.0), None))?.0, "angle");
    assert_eq!(parameter_refusal(line.twist(1e12, Some(100.0), None))?.0, "angle");
    Ok(())
}
