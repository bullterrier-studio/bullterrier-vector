//! Measuring a `Path` through the public API: its length, the point and the direction at a position, and the position
//! of a point. Every expected value is a closed form written here from the shape's definition (a perimeter, a law of
//! cosines, `2 pi r`, the integral of a parabola), never taken from the implementation.

mod common;

use bullterrier_vector::{Path, Point};
use common::{Outcome, distance, geometry_refusal, log_uniform, parameter_refusal};
use proptest::prelude::*;
use std::f64::consts::PI;

/// Whether `found` is within the relative `tolerance` of `expected`.
fn close(found: f64, expected: f64, tolerance: f64) -> bool {
    (found - expected).abs() <= tolerance * expected.abs()
}

// Length -----------------------------------------------------------------------------------------------------------

#[test]
fn a_line_is_as_long_as_its_distance() -> Outcome {
    // 3 right and 4 down is the 3-4-5 triangle
    assert!(close(Path::line((1.0, 2.0), (4.0, 6.0))?.length(), 5.0, 1e-12));
    Ok(())
}

#[test]
fn a_rectangle_is_as_long_as_its_perimeter() -> Outcome {
    assert!(close(Path::rect(3.0, -4.0, 30.0, 17.5, 0.0)?.length(), 95.0, 1e-12));
    Ok(())
}

#[test]
fn a_rounded_rectangle_is_its_straight_sides_plus_a_full_circle_of_corners() -> Outcome {
    // sides 60 x 40 with corners of radius 8: 2 (60 - 16) + 2 (40 - 16) straight, and 4 quarters of 2 pi 8
    let expected = 2.0 * 44.0 + 2.0 * 24.0 + 2.0 * PI * 8.0;
    assert!(close(Path::rect(0.0, 0.0, 60.0, 40.0, 8.0)?.length(), expected, 1e-3));
    Ok(())
}

#[test]
fn a_closed_path_counts_the_line_back_to_its_start_and_an_open_one_does_not() -> Outcome {
    let corners = [(0.0, 0.0), (5.0, 0.0), (5.0, 12.0)];
    // 5 + 12 for the two sides, and the hypotenuse 13 of the 5-12-13 triangle for the closing line
    assert!(close(Path::polyline(corners, false)?.length(), 17.0, 1e-12));
    assert!(close(Path::polyline(corners, true)?.length(), 30.0, 1e-12));
    Ok(())
}

#[test]
fn sub_paths_add_up() -> Outcome {
    let path = Path::line((0.0, 0.0), (3.0, 4.0))?
        .move_to((10.0, 10.0))?
        .line_to((10.0, 22.0))?
        .line_to((15.0, 22.0))?;
    assert!(close(path.length(), 5.0 + 12.0 + 5.0, 1e-12));
    Ok(())
}

#[test]
fn a_parabola_is_as_long_as_its_integral() -> Outcome {
    // an inscribed polyline is short by about flatness / (3 r) at a bend of radius r, and the bend here is 2.5 at its
    // tightest: the tolerance is 1e-3
    // the quadratic (0,0) (5,10) (10,0) is y = 2x - x^2/5; its length, the integral of sqrt(1 + y'^2) from 0 to 10, is
    // 5 sqrt 5 + 2.5 asinh 2
    let expected = 5.0 * 5.0_f64.sqrt() + 2.5 * libm::log(2.0 + 5.0_f64.sqrt());
    let parabola = Path::start((0.0, 0.0))?.quad_to((5.0, 10.0), (10.0, 0.0))?;
    assert!(close(parabola.length(), expected, 1e-3), "{}", parabola.length());
    Ok(())
}

#[test]
fn a_parabola_ten_million_pixels_wide_is_as_exact_as_a_small_one() -> Outcome {
    // the parabola above scaled by 1e6: still flat to a hundredth of a pixel, so short by about 1e-9 of its length
    let expected = 1e6 * (5.0 * 5.0_f64.sqrt() + 2.5 * libm::log(2.0 + 5.0_f64.sqrt()));
    let parabola = Path::start((0.0, 0.0))?.quad_to((5e6, 1e7), (1e7, 0.0))?;
    assert!(
        close(parabola.length(), expected, 1e-8),
        "{} against {expected}",
        parabola.length()
    );
    Ok(())
}

#[test]
fn a_curve_whose_handles_overshoot_along_its_own_chord_is_measured_out_and_back() -> Outcome {
    // x(t) = 45 t - 105 t^2 + 70 t^3 on a line: the curve runs out to x1, back to x2 and on to 10, so its length is
    // the total variation of x, found from the roots of x'(t) = 45 - 210 t + 210 t^2
    let x = |t: f64| 45.0 * t - 105.0 * t * t + 70.0 * t * t * t;
    let root = (210.0 - (210.0_f64 * 210.0 - 4.0 * 210.0 * 45.0).sqrt()) / (2.0 * 210.0);
    let other = (210.0 + (210.0_f64 * 210.0 - 4.0 * 210.0 * 45.0).sqrt()) / (2.0 * 210.0);
    let expected = x(root).abs() + (x(other) - x(root)).abs() + (10.0 - x(other)).abs();
    let curve = Path::start((0.0, 0.0))?.curve_to((15.0, 0.0), (-5.0, 0.0), (10.0, 0.0))?;
    assert!(
        close(curve.length(), expected, 1e-3),
        "{} against {expected}",
        curve.length()
    );
    Ok(())
}

#[test]
fn nothing_has_no_length_and_a_lone_start_none_either() -> Outcome {
    assert!(Path::new().length().abs() < f64::EPSILON);
    assert!(Path::start((3.0, 4.0))?.length().abs() < f64::EPSILON);
    assert!(Path::start((3.0, 4.0))?.line_to((3.0, 4.0))?.length().abs() < f64::EPSILON);
    Ok(())
}

proptest! {
    // A regular polygon of n corners on a circle of radius r has sides 2 r sin(pi / n).
    #[test]
    fn a_polygon_is_n_sides_long(sides in 3u32..=500, radius in log_uniform(-3.0, 6.0)) {
        let n = f64::from(sides);
        let expected = n * 2.0 * radius * libm::sin(PI / n);
        let path = Path::polygon((0.0, 0.0), radius, i64::from(sides)).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert!(close(path.length(), expected, 1e-10), "{} against {}", path.length(), expected);
    }

    // The side of a star joins a tip at R and a notch at r, an angle pi / n apart at the centre: the law of cosines.
    #[test]
    fn a_star_is_2n_sides_long(
        points in 2u32..=100,
        outer in log_uniform(-3.0, 6.0),
        inner in log_uniform(-3.0, 6.0),
    ) {
        let n = f64::from(points);
        let side = (outer * outer + inner * inner - 2.0 * outer * inner * libm::cos(PI / n)).sqrt();
        let path = Path::star((0.0, 0.0), outer, inner, i64::from(points)).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert!(close(path.length(), 2.0 * n * side, 1e-10), "{} against {}", path.length(), 2.0 * n * side);
    }

    // The four cubic arcs of a circle stay within 1e-3 of 2 pi r once the circle is big enough for the arcs to be
    // flat to 0.01 px with few pieces.
    #[test]
    fn a_circle_is_2_pi_r_long(radius in log_uniform(1.0, 6.0), centre in (-1000.0f64..1000.0, -1000.0f64..1000.0)) {
        let path = Path::circle(centre, radius).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert!(close(path.length(), 2.0 * PI * radius, 1e-3), "{} against {}", path.length(), 2.0 * PI * radius);
    }
}

// Points and directions at a position -------------------------------------------------------------------------------

/// The point a fraction `at` of the way along `path`, from the start.
fn point_at(path: &Path, at: f64) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let point = path.point_at(at, None)?;
    Ok((point.x, point.y))
}

/// The direction of travel a fraction `at` of the way along `path`, from the start.
fn tangent_at(path: &Path, at: f64) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let direction = path.tangent_at(at, None)?;
    Ok((direction.x, direction.y))
}

fn assert_near(found: (f64, f64), expected: (f64, f64), tolerance: f64, what: &str) {
    assert!(
        distance(found, expected) <= tolerance,
        "{what}: {found:?} is not within {tolerance} of {expected:?}"
    );
}

#[test]
fn a_line_gives_its_points_by_fraction_of_its_length() -> Outcome {
    let line = Path::line((1.0, 2.0), (4.0, 6.0))?;
    for (at, expected) in [
        (0.0, (1.0, 2.0)),
        (0.3, (1.9, 3.2)),
        (0.5, (2.5, 4.0)),
        (1.0, (4.0, 6.0)),
    ] {
        assert_near(point_at(&line, at)?, expected, 1e-12, &format!("at {at}"));
    }
    Ok(())
}

#[test]
fn a_rectangle_is_walked_from_the_left_of_its_top_edge_clockwise() -> Outcome {
    // perimeter 2 (30 + 17.5) = 95
    let rect = Path::rect(3.0, -4.0, 30.0, 17.5, 0.0)?;
    let cases = [
        (0.0, (3.0, -4.0), "start"),
        (15.0 / 95.0, (18.0, -4.0), "middle of the top edge"),
        ((30.0 + 8.75) / 95.0, (33.0, 4.75), "middle of the right edge"),
        (
            (47.5 + 10.0) / 95.0,
            (23.0, 13.5),
            "10 along the bottom edge, leftwards",
        ),
        ((77.5 + 5.0) / 95.0, (3.0, 8.5), "5 up the left edge"),
        (1.0, (3.0, -4.0), "end of a closed outline is its start"),
    ];
    for (at, expected, what) in cases {
        assert_near(point_at(&rect, at)?, expected, 1e-9, what);
    }
    Ok(())
}

#[test]
fn a_circle_is_walked_from_the_top_clockwise_and_stays_on_the_circle() -> Outcome {
    let (centre, radius) = ((20.0, -30.0), 75.0);
    let circle = Path::circle(centre, radius)?;
    for at in [0.0, 0.07, 0.25, 0.4, 0.5, 0.61, 0.75, 0.93] {
        let angle = 2.0 * PI * at;
        let ideal = (
            centre.0 + radius * libm::sin(angle),
            centre.1 - radius * libm::cos(angle),
        );
        // the cubic strays 2.7e-4 r from the circle, and the walk puts the point a hair off its arc length
        assert_near(point_at(&circle, at)?, ideal, 1e-3 * radius, &format!("at {at}"));
    }
    Ok(())
}

#[test]
fn a_curve_that_speeds_up_and_slows_down_is_still_walked_by_arc_length() -> Outcome {
    // x(t) = 100 (3 t^2 - 2 t^3) on a line: the same 100 pixels as a line, but the curve crawls at both ends; a
    // quarter of the length along is x = 25, where the curve itself is only at t = 0.3 with x = 21.6
    let eased = Path::start((0.0, 0.0))?.curve_to((0.0, 0.0), (100.0, 0.0), (100.0, 0.0))?;
    assert!(close(eased.length(), 100.0, 1e-9), "{}", eased.length());
    for (at, x) in [(0.1, 10.0), (0.25, 25.0), (0.5, 50.0), (0.8, 80.0)] {
        let found = point_at(&eased, at)?.0;
        assert!((found - x).abs() <= 0.02, "at {at}: x is {found}, expected {x}");
    }
    Ok(())
}

#[test]
fn sub_paths_are_walked_in_order_and_a_boundary_belongs_to_the_later_one() -> Outcome {
    // 10 along y = 0, then 30 down x = 0 from (0, 5): 40 in all
    let path = Path::line((0.0, 0.0), (10.0, 0.0))?
        .move_to((0.0, 5.0))?
        .line_to((0.0, 35.0))?;
    assert_near(point_at(&path, 0.2)?, (8.0, 0.0), 1e-9, "in the first");
    assert_near(point_at(&path, 0.5)?, (0.0, 15.0), 1e-9, "in the second");
    assert_near(point_at(&path, 0.25)?, (0.0, 5.0), 1e-9, "where the first ends");
    assert_near(point_at(&path, 1.0)?, (0.0, 35.0), 1e-9, "end");
    Ok(())
}

#[test]
fn a_path_with_no_length_has_one_point_and_a_sub_path_with_none_is_passed_over() -> Outcome {
    for at in [0.0, 0.4, 1.0] {
        assert_near(
            point_at(&Path::start((7.0, -2.0))?, at)?,
            (7.0, -2.0),
            0.0,
            "lone start",
        );
        let stationary = Path::start((7.0, -2.0))?.line_to((7.0, -2.0))?;
        assert_near(point_at(&stationary, at)?, (7.0, -2.0), 0.0, "line to itself");
    }
    let after_a_dot = Path::start((1.0, 1.0))?.move_to((5.0, 5.0))?.line_to((8.0, 9.0))?;
    assert_near(point_at(&after_a_dot, 0.0)?, (5.0, 5.0), 0.0, "start skips the dot");
    assert_near(point_at(&after_a_dot, 0.5)?, (6.5, 7.0), 1e-9, "middle");
    Ok(())
}

#[test]
fn a_closed_outline_is_walked_from_its_topmost_corner_in_its_own_direction() -> Outcome {
    // written from A (50, 40) on, the topmost corner is E (30, 10); the walk goes E, A, B, C, D
    let corners = [(50.0, 40.0), (90.0, 30.0), (60.0, 80.0), (10.0, 60.0), (30.0, 10.0)];
    let outline = Path::polyline(corners, true)?;
    let side = |from: usize, to: usize| distance(corners[from], corners[to]);
    let perimeter = side(0, 1) + side(1, 2) + side(2, 3) + side(3, 4) + side(4, 0);
    assert_near(point_at(&outline, 0.0)?, (30.0, 10.0), 1e-9, "start");
    assert_near(
        point_at(&outline, side(4, 0) / perimeter)?,
        (50.0, 40.0),
        1e-9,
        "after the first side",
    );
    let to_c = side(4, 0) + side(0, 1) + side(1, 2);
    assert_near(
        point_at(&outline, to_c / perimeter)?,
        (60.0, 80.0),
        1e-9,
        "after three sides",
    );
    Ok(())
}

#[test]
fn of_two_equally_high_corners_the_walk_starts_at_the_left_one() -> Outcome {
    // (50, 5) is written first, but (10, 5) is as high and further left
    let corners = [(50.0, 5.0), (90.0, 40.0), (60.0, 70.0), (10.0, 5.0)];
    let outline = Path::polyline(corners, true)?;
    let perimeter = distance(corners[0], corners[1])
        + distance(corners[1], corners[2])
        + distance(corners[2], corners[3])
        + distance(corners[3], corners[0]);
    assert_near(point_at(&outline, 0.0)?, (10.0, 5.0), 1e-9, "start");
    assert_near(
        point_at(&outline, 20.0 / perimeter)?,
        (30.0, 5.0),
        1e-9,
        "20 along the level side",
    );
    Ok(())
}

#[test]
fn an_open_path_keeps_the_start_it_was_written_with() -> Outcome {
    // the last corner is the highest, but an open path is not rotated
    let open = Path::polyline([(50.0, 40.0), (90.0, 30.0), (30.0, 10.0)], false)?;
    assert_near(point_at(&open, 0.0)?, (50.0, 40.0), 0.0, "start");
    assert_near(point_at(&open, 1.0)?, (30.0, 10.0), 0.0, "end");
    Ok(())
}

#[test]
fn a_position_outside_0_to_1_is_refused_naming_it_and_the_edges_are_accepted() -> Outcome {
    let path = Path::line((1.0, 2.0), (4.0, 6.0))?;
    for (at, shown) in [
        (-0.1, "-0.1"),
        (1.0 + f64::EPSILON, "1.0000000000000002"),
        (f64::NAN, "NaN"),
        (f64::INFINITY, "inf"),
        (f64::NEG_INFINITY, "-inf"),
    ] {
        let (name, message) = parameter_refusal(path.point_at(at, None))?;
        assert_eq!(name, "at", "point_at({at})");
        assert!(message.contains(shown), "{message}");
        let (name, message) = parameter_refusal(path.tangent_at(at, None))?;
        assert_eq!(name, "at", "tangent_at({at})");
        assert!(message.contains(shown), "{message}");
    }
    for at in [0.0, 1.0, 0.5] {
        path.point_at(at, None)?;
        path.tangent_at(at, None)?;
    }
    Ok(())
}

#[test]
fn an_empty_path_has_no_point_and_no_direction() -> Outcome {
    let (verb, message) = geometry_refusal(Path::new().point_at(0.5, None))?;
    assert_eq!(verb, "point_at");
    assert!(message.contains("empty"), "{message}");
    let (verb, message) = geometry_refusal(Path::new().tangent_at(0.5, None))?;
    assert_eq!(verb, "tangent_at");
    assert!(message.contains("empty"), "{message}");
    // the position is checked before the path
    parameter_refusal(Path::new().point_at(2.0, None))?;
    Ok(())
}

#[test]
fn a_line_points_the_way_it_runs_all_along() -> Outcome {
    let line = Path::line((1.0, 2.0), (4.0, 6.0))?;
    for at in [0.0, 0.4, 1.0] {
        assert_near(tangent_at(&line, at)?, (0.6, 0.8), 1e-12, &format!("at {at}"));
    }
    Ok(())
}

#[test]
fn at_a_corner_the_direction_is_the_one_leaving_it() -> Outcome {
    // 24 x 8: the corners are at 24, 32 and 56 of 64, all exact fractions
    let rect = Path::rect(3.0, -4.0, 24.0, 8.0, 0.0)?;
    let cases = [
        (0.0, (1.0, 0.0), "start, along the top"),
        (0.25, (1.0, 0.0), "in the top edge"),
        (0.375, (0.0, 1.0), "the top right corner, leaving down"),
        (0.5, (-1.0, 0.0), "the bottom right corner, leaving left"),
        (0.875, (0.0, -1.0), "the bottom left corner, leaving up"),
        (0.9, (0.0, -1.0), "in the left edge"),
    ];
    for (at, expected, what) in cases {
        assert_near(tangent_at(&rect, at)?, expected, 1e-12, what);
    }
    // the end of a closed outline looks the same way as its start
    assert_eq!(tangent_at(&rect, 1.0)?, tangent_at(&rect, 0.0)?);
    // the same holds for the corner of an open path
    let bend = Path::polyline([(0.0, 0.0), (8.0, 0.0), (8.0, 8.0)], false)?;
    assert_near(tangent_at(&bend, 0.5)?, (0.0, 1.0), 1e-12, "the bend");
    assert_near(tangent_at(&bend, 0.4)?, (1.0, 0.0), 1e-12, "before the bend");
    Ok(())
}

#[test]
fn the_end_of_an_open_path_points_the_way_it_arrives() -> Outcome {
    let bend = Path::polyline([(0.0, 0.0), (8.0, 0.0), (8.0, 8.0)], false)?;
    assert_near(tangent_at(&bend, 1.0)?, (0.0, 1.0), 1e-12, "end");
    Ok(())
}

#[test]
fn a_circle_points_along_its_circle_clockwise() -> Outcome {
    let circle = Path::circle((20.0, -30.0), 50.0)?;
    for at in [0.1, 0.37, 0.5, 0.83] {
        let angle = 2.0 * PI * at;
        let found = tangent_at(&circle, at)?;
        // the direction of a flat edge is off the curve's by at most half the angle one edge turns, about 0.013
        let off_by = libm::atan2(
            found.0 * libm::sin(angle) - found.1 * libm::cos(angle),
            found.0 * libm::cos(angle) + found.1 * libm::sin(angle),
        );
        assert!(
            off_by.abs() < 0.03,
            "at {at}: {found:?} turns {off_by} from the circle's"
        );
        assert!(
            close(distance((0.0, 0.0), found), 1.0, 1e-12),
            "{found:?} is not a unit vector"
        );
    }
    Ok(())
}

#[test]
fn segments_of_no_length_have_no_direction_to_give() -> Outcome {
    let path = Path::start((0.0, 0.0))?.line_to((0.0, 0.0))?.line_to((3.0, 4.0))?;
    assert_near(tangent_at(&path, 0.0)?, (0.6, 0.8), 1e-12, "past the point");
    Ok(())
}

#[test]
fn the_direction_at_a_boundary_between_sub_paths_is_the_later_ones() -> Outcome {
    let path = Path::line((0.0, 0.0), (10.0, 0.0))?
        .move_to((0.0, 5.0))?
        .line_to((0.0, 35.0))?;
    assert_near(tangent_at(&path, 0.2)?, (1.0, 0.0), 1e-12, "in the first");
    assert_near(tangent_at(&path, 0.25)?, (0.0, 1.0), 1e-12, "where the first ends");
    assert_near(tangent_at(&path, 1.0)?, (0.0, 1.0), 1e-12, "end");
    Ok(())
}

#[test]
fn a_path_with_no_length_has_no_direction() -> Outcome {
    for path in [
        Path::start((7.0, -2.0))?,
        Path::start((7.0, -2.0))?.line_to((7.0, -2.0))?,
        Path::start((7.0, -2.0))?.line_to((7.0, -2.0))?.close()?,
    ] {
        let (verb, message) = geometry_refusal(path.tangent_at(0.5, None))?;
        assert_eq!(verb, "tangent_at");
        assert!(message.contains("no length"), "{message}");
    }
    Ok(())
}

// Where a point lies -----------------------------------------------------------------------------------------------

fn locate(path: &Path, point: (f64, f64)) -> Result<f64, Box<dyn std::error::Error>> {
    Ok(path.locate(point, None)?)
}

fn assert_position(found: f64, expected: f64, what: &str) {
    assert!(
        (found - expected).abs() <= 1e-9,
        "{what}: found {found}, expected {expected}"
    );
}

#[test]
fn a_point_is_located_at_the_nearest_point_of_a_line() -> Outcome {
    let line = Path::line((1.0, 2.0), (4.0, 6.0))?;
    assert_position(locate(&line, (2.5, 4.0))?, 0.5, "on the line");
    // 3 right of the start: its share along (3, 4) is 9 / 25
    assert_position(locate(&line, (4.0, 2.0))?, 0.36, "beside the line");
    assert_position(locate(&line, (-10.0, -10.0))?, 0.0, "before the start");
    assert_position(locate(&line, (100.0, 100.0))?, 1.0, "past the end");
    Ok(())
}

#[test]
fn a_point_is_located_on_the_edge_of_a_rectangle_nearest_to_it() -> Outcome {
    let rect = Path::rect(3.0, -4.0, 30.0, 17.5, 0.0)?;
    assert_position(locate(&rect, (18.0, -10.0))?, 15.0 / 95.0, "above the top edge");
    assert_position(locate(&rect, (40.0, 4.75))?, 38.75 / 95.0, "right of the right edge");
    // inside, 4 below the top edge and 7 right of the left edge
    assert_position(locate(&rect, (10.0, 0.0))?, 7.0 / 95.0, "inside, nearer the top");
    Ok(())
}

#[test]
fn the_start_of_a_closed_outline_is_at_0_and_never_at_1() -> Outcome {
    let rect = Path::rect(3.0, -4.0, 30.0, 17.5, 0.0)?;
    assert_position(locate(&rect, (3.0, -4.0))?, 0.0, "on the start");
    assert_position(locate(&rect, (-5.0, -10.0))?, 0.0, "off the start corner");
    // 0.1 below the start, on the line that closes the outline
    assert_position(
        locate(&rect, (3.0, -3.9))?,
        (95.0 - 0.1) / 95.0,
        "just before the start",
    );
    Ok(())
}

#[test]
fn of_equally_near_places_the_smallest_position_wins() -> Outcome {
    // the middle of a square is 5 from all four sides; the top edge comes first, 5 of 40 along
    let square = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?;
    assert_position(locate(&square, (5.0, 5.0))?, 0.125, "four sides");
    // the middle between two lines of 10 each is 5 from both
    let lines = Path::line((0.0, 0.0), (10.0, 0.0))?
        .move_to((0.0, 10.0))?
        .line_to((10.0, 10.0))?;
    assert_position(locate(&lines, (5.0, 5.0))?, 0.25, "two sub-paths");
    // nearer the second: 3 along it, after the 10 of the first, of 20
    assert_position(locate(&lines, (3.0, 9.0))?, 0.65, "in the later sub-path");
    Ok(())
}

#[test]
fn a_path_with_no_length_is_located_at_0() -> Outcome {
    assert_position(locate(&Path::start((7.0, -2.0))?, (100.0, 3.0))?, 0.0, "lone start");
    Ok(())
}

#[test]
fn locating_is_refused_on_an_empty_path_and_for_points_that_are_not_numbers() -> Outcome {
    let (verb, message) = geometry_refusal(Path::new().locate((1.0, 2.0), None))?;
    assert_eq!(verb, "locate");
    assert!(message.contains("empty"), "{message}");
    let line = Path::line((1.0, 2.0), (4.0, 6.0))?;
    let (name, message) = parameter_refusal(line.locate((f64::NAN, 2.0), None))?;
    assert_eq!((name.as_str(), message.contains("NaN")), ("point", true), "{message}");
    let (name, message) = parameter_refusal(line.locate((1.0, f64::INFINITY), None))?;
    assert_eq!((name.as_str(), message.contains("inf")), ("point", true), "{message}");
    // a far point is located as well as a near one, up to the limit
    line.locate((1e99, -1e99), None)?;
    Ok(())
}

proptest! {
    // The point at a position is located at that position, on outlines that do not cross themselves.
    #[test]
    fn a_point_of_the_outline_is_located_where_it_was_taken(
        sides in 3u32..=60,
        radius in log_uniform(-2.0, 5.0),
        at in 0.001f64..0.999,
        star in any::<bool>(),
    ) {
        let outline = if star {
            Path::star((5.0, -7.0), radius, radius * 0.4, i64::from(sides))
        } else {
            Path::polygon((5.0, -7.0), radius, i64::from(sides))
        }
        .map_err(|e| TestCaseError::fail(e.to_string()))?;
        let point = outline.point_at(at, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let found = outline.locate(point, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert!((found - at).abs() <= 1e-9, "{} against {}", found, at);
    }
}

// Starting from the point nearest another ----------------------------------------------------------------------------

#[test]
fn near_starts_a_closed_outline_at_the_point_of_it_nearest() -> Outcome {
    let rect = Path::rect(3.0, -4.0, 30.0, 17.5, 0.0)?;
    let near = Some(Point::new(40.0, 4.75));
    // the middle of the right edge; then down 8.75, left 30, up 17.5, to the start corner
    assert_near(point_pair(&rect, 0.0, near)?, (33.0, 4.75), 1e-9, "start");
    assert_near(point_pair(&rect, 1.0, near)?, (33.0, 4.75), 1e-9, "end");
    assert_near(
        point_pair(&rect, 8.75 / 95.0, near)?,
        (33.0, 13.5),
        1e-9,
        "the corner after",
    );
    let direction = rect.tangent_at(0.0, near)?;
    assert_near((direction.x, direction.y), (0.0, 1.0), 1e-12, "leaving down");
    assert_position(rect.locate((33.0, 4.75), near)?, 0.0, "locate the new start");
    assert_position(rect.locate((3.0, -4.0), near)?, 56.25 / 95.0, "locate the old start");
    assert!(close(rect.length(), 95.0, 1e-12));
    Ok(())
}

fn point_pair(path: &Path, at: f64, near: Option<Point>) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let point = path.point_at(at, near)?;
    Ok((point.x, point.y))
}

#[test]
fn near_leaves_an_open_sub_path_where_it_starts() -> Outcome {
    let line = Path::line((1.0, 2.0), (4.0, 6.0))?;
    assert_near(
        point_pair(&line, 0.0, Some(Point::new(100.0, 100.0)))?,
        (1.0, 2.0),
        0.0,
        "start",
    );
    assert_near(
        point_pair(&line, 1.0, Some(Point::new(100.0, 100.0)))?,
        (4.0, 6.0),
        0.0,
        "end",
    );
    Ok(())
}

#[test]
fn near_starts_every_closed_sub_path_at_its_own_nearest_point() -> Outcome {
    let two_squares = Path::start((0.0, 0.0))?
        .line_to((10.0, 0.0))?
        .line_to((10.0, 10.0))?
        .line_to((0.0, 10.0))?
        .close()?
        .move_to((20.0, 0.0))?
        .line_to((30.0, 0.0))?
        .line_to((30.0, 10.0))?
        .line_to((20.0, 10.0))?
        .close()?;
    let near = Some(Point::new(35.0, 5.0));
    assert_near(
        point_pair(&two_squares, 0.0, near)?,
        (10.0, 5.0),
        1e-9,
        "the first starts on its right edge",
    );
    assert_near(
        point_pair(&two_squares, 0.5, near)?,
        (30.0, 5.0),
        1e-9,
        "the second starts on its right edge",
    );
    Ok(())
}

#[test]
fn near_equally_close_to_several_edges_starts_on_the_first_as_written() -> Outcome {
    let square = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?;
    assert_near(
        point_pair(&square, 0.0, Some(Point::new(5.0, 5.0)))?,
        (5.0, 0.0),
        1e-9,
        "the top edge",
    );
    Ok(())
}

#[test]
fn near_a_corner_starts_at_the_corner_and_leaves_along_the_next_edge() -> Outcome {
    let square = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?;
    let near = Some(Point::new(13.0, 14.0));
    assert_near(point_pair(&square, 0.0, near)?, (10.0, 10.0), 1e-9, "start");
    assert_near(point_pair(&square, 0.25, near)?, (0.0, 10.0), 1e-9, "a side on");
    let direction = square.tangent_at(0.0, near)?;
    assert_near(
        (direction.x, direction.y),
        (-1.0, 0.0),
        1e-12,
        "leaving along the bottom",
    );
    // near the start corner changes nothing; the closing line ends there
    assert_near(
        point_pair(&square, 0.0, Some(Point::new(-3.0, -3.0)))?,
        (0.0, 0.0),
        1e-9,
        "start corner",
    );
    assert_near(
        point_pair(&square, 0.125, Some(Point::new(-3.0, -3.0)))?,
        (5.0, 0.0),
        1e-9,
        "start corner, on",
    );
    Ok(())
}

#[test]
fn near_must_be_a_finite_point() -> Outcome {
    let line = Path::line((1.0, 2.0), (4.0, 6.0))?;
    let bad = Some(Point::new(f64::NAN, 1.0));
    for (verb, outcome) in [
        ("point_at", line.point_at(0.5, bad).map(|_| ())),
        ("tangent_at", line.tangent_at(0.5, bad).map(|_| ())),
        ("locate", line.locate((1.0, 2.0), bad).map(|_| ())),
    ] {
        let (name, message) = parameter_refusal(outcome)?;
        assert_eq!(
            (name.as_str(), message.contains("NaN")),
            ("near", true),
            "{verb}: {message}"
        );
    }
    Ok(())
}

proptest! {
    // The start `near` picks is as near to the point as any sampled place on the outline, up to the spacing of the
    // samples.
    #[test]
    fn the_start_chosen_by_near_is_the_nearest_place_of_the_outline(
        sides in 3u32..=30,
        radius in log_uniform(-1.0, 4.0),
        near_in_radii in (-2.0f64..2.0, -2.0f64..2.0),
    ) {
        let outline = Path::polygon((0.0, 0.0), radius, i64::from(sides)).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let near = Point::new(near_in_radii.0 * radius, near_in_radii.1 * radius);
        let start = outline.point_at(0.0, Some(near)).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let samples = 2000;
        let mut nearest_sampled = f64::INFINITY;
        for step in 0..samples {
            let place = outline
                .point_at(f64::from(step) / f64::from(samples), None)
                .map_err(|e| TestCaseError::fail(e.to_string()))?;
            nearest_sampled = nearest_sampled.min(distance((place.x, place.y), (near.x, near.y)));
        }
        let found = distance((start.x, start.y), (near.x, near.y));
        let spacing = outline.length() / f64::from(samples);
        prop_assert!(found <= nearest_sampled + 1e-9 * radius, "{} against {}", found, nearest_sampled);
        prop_assert!(found >= nearest_sampled - spacing, "{} against {}", found, nearest_sampled);
    }
}
