//! `Path::arc` through the public API: an open circular arc, angles in degrees clockwise on the screen from +x. Endpoints
//! are computed by hand; the length is the circle's `radius * sweep`.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::Path;
use std::f64::consts::PI;

type Ends = ((f64, f64), (f64, f64));

fn ends(path: &Path) -> Result<Ends, Box<dyn std::error::Error>> {
    let (start, end) = (path.point_at(0.0, None)?, path.point_at(1.0, None)?);
    Ok(((start.x, start.y), (end.x, end.y)))
}

fn near(found: (f64, f64), expected: (f64, f64)) -> bool {
    (found.0 - expected.0).abs() < 1e-9 && (found.1 - expected.1).abs() < 1e-9
}

#[test]
fn a_quarter_turn_runs_from_the_right_of_the_center_to_below_it() -> Outcome {
    // 0 degrees is +x, 90 is down the screen (y down)
    let arc = Path::arc((10.0, 20.0), 5.0, 0.0, 90.0)?;
    let (start, end) = ends(&arc)?;
    assert!(near(start, (15.0, 20.0)), "{start:?}");
    assert!(near(end, (10.0, 25.0)), "{end:?}");
    let data = arc.to_svg_path();
    assert!(data.starts_with("M15 20 C") && data.ends_with(" 10 25"), "{data}");
    assert_eq!(data.matches('C').count(), 1, "one cubic for a quarter turn: {data}");
    Ok(())
}

#[test]
fn a_negative_sweep_runs_counter_clockwise_on_the_screen() -> Outcome {
    let arc = Path::arc((10.0, 20.0), 5.0, 0.0, -90.0)?;
    let (start, end) = ends(&arc)?;
    assert!(near(start, (15.0, 20.0)), "{start:?}");
    assert!(near(end, (10.0, 15.0)), "{end:?}");
    // and it passes through the point at -45 degrees, up the screen, not the one at +45 below it (half the arc's
    // length is its middle; a cubic quarter arc is within 3e-4 of the radius of the circle)
    let middle = arc.point_at(0.5, None)?;
    let half_diagonal = 5.0 * std::f64::consts::FRAC_1_SQRT_2;
    assert!(
        (middle.x - (10.0 + half_diagonal)).abs() < 5e-3 && (middle.y - (20.0 - half_diagonal)).abs() < 5e-3,
        "{middle:?}"
    );
    // a start that is not 0: 180 to 270 is from the left of the center up to its top
    let (start, end) = ends(&Path::arc((0.0, 0.0), 2.0, 180.0, 270.0)?)?;
    assert!(near(start, (-2.0, 0.0)) && near(end, (0.0, -2.0)), "{start:?} {end:?}");
    Ok(())
}

#[test]
fn the_length_is_the_radius_times_the_sweep_in_radians() -> Outcome {
    for (start, end) in [
        (0.0, 90.0),
        (0.0, 180.0),
        (30.0, 300.0),
        (0.0, 360.0),
        (100.0, -20.0),
        (0.0, 45.0),
        (-170.0, 170.0),
    ] {
        let arc = Path::arc((3.0, -4.0), 7.0, start, end)?;
        let expected = 7.0 * (end - start).abs() / 180.0 * PI;
        assert!(
            (arc.length() - expected).abs() <= 1e-3 * expected,
            "{start} to {end}: {} against {expected}",
            arc.length()
        );
    }
    Ok(())
}

#[test]
fn every_point_is_on_the_circle() -> Outcome {
    let arc = Path::arc((3.0, -4.0), 7.0, 20.0, 330.0)?;
    for index in 0..=200 {
        let point = arc.point_at(f64::from(index) / 200.0, None)?;
        let off = (libm::hypot(point.x - 3.0, point.y + 4.0) - 7.0).abs();
        assert!(off <= 7.0 * 3e-4, "{index}: {off}");
    }
    Ok(())
}

#[test]
fn the_bounds_of_a_quarter_arc_are_its_quarter() -> Outcome {
    let bounds = Path::arc((10.0, 20.0), 5.0, 0.0, 90.0)?.bounds()?;
    for (found, expected) in [
        (bounds.x, 10.0),
        (bounds.y, 20.0),
        (bounds.width, 5.0),
        (bounds.height, 5.0),
    ] {
        assert!((found - expected).abs() < 2e-3, "{bounds:?}");
    }
    Ok(())
}

#[test]
fn one_cubic_per_quarter_turn_at_most() -> Outcome {
    for (sweep, cubics) in [
        (10.0, 1),
        (90.0, 1),
        (91.0, 2),
        (180.0, 2),
        (270.0, 3),
        (360.0, 4),
        (-360.0, 4),
        (-200.0, 3),
    ] {
        let data = Path::arc((0.0, 0.0), 1.0, 0.0, sweep)?.to_svg_path();
        assert_eq!(data.matches('C').count(), cubics, "{sweep}: {data}");
    }
    Ok(())
}

#[test]
fn a_full_turn_is_open_and_ends_where_it_starts() -> Outcome {
    let arc = Path::arc((0.0, 0.0), 10.0, 0.0, 360.0)?;
    let data = arc.to_svg_path();
    assert!(!data.contains('Z'), "{data}");
    let (start, end) = ends(&arc)?;
    assert!(near(start, end), "{start:?} {end:?}");
    Ok(())
}

#[test]
fn a_bad_radius_sweep_or_center_is_refused_naming_it() -> Outcome {
    let name = |outcome: Result<Path, bullterrier_vector::Error>| parameter_refusal(outcome).map(|found| found.0);
    for radius in [0.0, -1.0, f64::NAN, f64::INFINITY, 1.1e100] {
        assert_eq!(name(Path::arc((0.0, 0.0), radius, 0.0, 90.0))?, "radius");
    }
    assert_eq!(name(Path::arc((0.0, 0.0), 1.0, f64::NAN, 90.0))?, "start");
    assert_eq!(name(Path::arc((0.0, 0.0), 1.0, 0.0, f64::INFINITY))?, "end");
    assert_eq!(name(Path::arc((0.0, 0.0), 1.0, 0.0, 360.5))?, "end");
    assert_eq!(name(Path::arc((0.0, 0.0), 1.0, 10.0, -351.0))?, "end");
    assert_eq!(name(Path::arc((0.0, 0.0), 1.0, 45.0, 45.0))?, "end");
    assert_eq!(name(Path::arc((1.1e100, 0.0), 1.0, 0.0, 90.0))?, "center");
    assert_eq!(name(Path::arc((f64::NAN, 0.0), 1.0, 0.0, 90.0))?, "center");
    assert_eq!(name(Path::arc((1e100, 0.0), 1e100, 0.0, 90.0))?, "radius");
    Ok(())
}
