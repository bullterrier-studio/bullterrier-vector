//! Moving, turning and scaling a `Path`, and passing its points through a function, through the public API. Expected
//! paths are written by hand (dyadic numbers, so the arithmetic is exact); the properties compare a path with itself
//! moved and moved back, and the turn with the angle between a point and its image.

mod common;

use bullterrier_vector::{Error, Path, Point};
use common::{Outcome, clearance, commands, distance, log_uniform, parameter_refusal, shoelace_area, winding};
use proptest::prelude::*;
use std::f64::consts::PI;

/// The path written as canonical data, to compare with data written by hand.
fn data(path: Result<Path, Error>) -> Result<String, Box<dyn std::error::Error>> {
    Ok(path?.to_svg_path())
}

/// The points a polyline's data goes through, in order.
fn corners(path: &Path) -> Result<Vec<(f64, f64)>, Box<dyn std::error::Error>> {
    commands(&path.to_svg_path())?
        .into_iter()
        .filter(|(letter, _)| *letter != 'Z')
        .map(|(_, numbers)| match numbers[..] {
            [x, y] => Ok((x, y)),
            _ => Err("not a polyline command".into()),
        })
        .collect()
}

const SHAPE: &str = "M1 2 L10 4.25 Q11 5 12 6.5 C1 2 3 4 5 6 Z";

// Translate --------------------------------------------------------------------------------------------------------

#[test]
fn translating_adds_to_every_point_on_and_off_the_curve() -> Outcome {
    let shape = Path::from_svg_path(SHAPE)?;
    assert_eq!(
        data(shape.translate(3.5, -7.25))?,
        "M4.5 -5.25 L13.5 -3 Q14.5 -2.25 15.5 -0.75 C4.5 -5.25 6.5 -3.25 8.5 -1.25 Z"
    );
    assert_eq!(shape.translate(0.0, 0.0)?, shape);
    Ok(())
}

#[test]
fn translating_by_a_number_that_is_not_one_is_refused_naming_it() -> Outcome {
    let shape = Path::from_svg_path(SHAPE)?;
    let (name, message) = parameter_refusal(shape.translate(f64::NAN, 1.0))?;
    assert_eq!((name.as_str(), message.contains("NaN")), ("dx", true), "{message}");
    let (name, message) = parameter_refusal(shape.translate(1.0, f64::INFINITY))?;
    assert_eq!((name.as_str(), message.contains("inf")), ("dy", true), "{message}");
    // the same goes for an empty path, which would otherwise be left alone
    parameter_refusal(Path::new().translate(f64::NAN, 1.0))?;
    // a move that carries a point beyond what a number holds
    let far = Path::line((9e99, 0.0), (2.0, 0.0))?;
    let (name, message) = parameter_refusal(far.translate(9e99, 0.0))?;
    assert_eq!(
        (name.as_str(), message.contains("too large")),
        ("dx", true),
        "{message}"
    );
    Ok(())
}

proptest! {
    // Moving by (dx, dy) moves the bounds by (dx, dy) and keeps the length.
    #[test]
    fn translating_moves_the_bounds_and_keeps_the_length(
        sides in 3u32..=30,
        radius in log_uniform(-1.0, 4.0),
        shift in (-1000.0f64..1000.0, -1000.0f64..1000.0),
    ) {
        let shape = Path::star((7.0, -3.0), radius, radius * 0.4, i64::from(sides)).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let moved = shape.translate(shift.0, shift.1).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let (before, after) = (shape.bounds().map_err(|e| TestCaseError::fail(e.to_string()))?, moved.bounds().map_err(|e| TestCaseError::fail(e.to_string()))?);
        let slack = 1e-12 * (radius + 1000.0);
        prop_assert!((after.x - before.x - shift.0).abs() <= slack && (after.y - before.y - shift.1).abs() <= slack);
        prop_assert!((after.width - before.width).abs() <= slack && (after.height - before.height).abs() <= slack);
        prop_assert!((moved.length() - shape.length()).abs() <= 1e-9 * shape.length());
    }
}

// Rotate -----------------------------------------------------------------------------------------------------------

#[test]
fn quarter_turns_are_exact_and_run_clockwise_on_the_screen() -> Outcome {
    // right is 1 pixel along x; a quarter turn clockwise (y points down) puts it 1 pixel along y
    let line = Path::line((1.0, 0.0), (2.0, 0.0))?;
    let origin = Some(Point::new(0.0, 0.0));
    let cases = [
        (90.0, "M0 1 L0 2"),
        (180.0, "M-1 0 L-2 0"),
        (270.0, "M0 -1 L0 -2"),
        (360.0, "M1 0 L2 0"),
        (-90.0, "M0 -1 L0 -2"),
        (450.0, "M0 1 L0 2"),
        (0.0, "M1 0 L2 0"),
    ];
    for (angle, expected) in cases {
        assert_eq!(data(line.rotate(angle, origin))?, expected, "{angle} degrees");
    }
    Ok(())
}

#[test]
fn turning_about_a_point_keeps_that_point_where_it_is() -> Outcome {
    // 3 right of (2, 3), a quarter turn puts it 3 below
    let line = Path::line((5.0, 3.0), (8.0, 3.0))?;
    assert_eq!(data(line.rotate(90.0, Some(Point::new(2.0, 3.0))))?, "M2 6 L2 9");
    // the pivot is the start of this one and stays
    let from_pivot = Path::line((2.0, 3.0), (6.0, 3.0))?;
    assert_eq!(data(from_pivot.rotate(90.0, Some(Point::new(2.0, 3.0))))?, "M2 3 L2 7");
    Ok(())
}

#[test]
fn turning_without_a_pivot_turns_about_the_middle_of_the_bounds() -> Outcome {
    // a 10 x 10 square about (5, 5): each corner takes the place of the one before it
    let square = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?;
    let turned = square.rotate(90.0, None)?;
    assert_eq!(turned.to_svg_path(), "M10 0 L10 10 L0 10 L0 0 Z");
    assert_eq!(turned, square.rotate(90.0, Some(Point::new(5.0, 5.0)))?);
    // a wide one: the middle of its bounds is (7, 2)
    let wide = Path::line((3.0, 2.0), (11.0, 2.0))?;
    assert_eq!(data(wide.rotate(90.0, None))?, "M7 -2 L7 6");
    Ok(())
}

#[test]
fn an_eighth_of_a_turn_goes_by_the_sine_and_cosine() -> Outcome {
    let turned = Path::line((1.0, 0.0), (2.0, 0.0))?.rotate(45.0, Some(Point::new(0.0, 0.0)))?;
    let side = 2.0_f64.sqrt() / 2.0;
    let found = corners(&turned)?;
    assert!(distance(found[0], (side, side)) < 1e-15, "{found:?}");
    assert!(distance(found[1], (2.0 * side, 2.0 * side)) < 1e-15, "{found:?}");
    Ok(())
}

#[test]
fn turning_by_something_that_is_not_a_number_is_refused_naming_it() -> Outcome {
    let line = Path::line((5.0, 3.0), (8.0, 3.0))?;
    let (name, message) = parameter_refusal(line.rotate(f64::NAN, None))?;
    assert_eq!((name.as_str(), message.contains("NaN")), ("angle", true), "{message}");
    let (name, message) = parameter_refusal(line.rotate(f64::INFINITY, None))?;
    assert_eq!((name.as_str(), message.contains("inf")), ("angle", true), "{message}");
    let (name, message) = parameter_refusal(line.rotate(30.0, Some(Point::new(1.0, f64::NAN))))?;
    assert_eq!((name.as_str(), message.contains("NaN")), ("about", true), "{message}");
    parameter_refusal(Path::new().rotate(f64::NAN, None))?;
    parameter_refusal(Path::new().rotate(30.0, Some(Point::new(f64::NAN, 0.0))))?;
    // a turn that carries a point beyond the limit: the corner (9e99, 9e99) turned an eighth reaches 1.27e100 down
    let far = Path::line((9e99, 9e99), (0.0, 0.0))?;
    let (name, message) = parameter_refusal(far.rotate(45.0, Some(Point::new(0.0, 0.0))))?;
    assert_eq!(
        (name.as_str(), message.contains("too large")),
        ("angle", true),
        "{message}"
    );
    Ok(())
}

/// A polyline of 2 to 9 corners within a square of half side `size`, the size spread over orders of magnitude.
fn polyline() -> impl Strategy<Value = (f64, Vec<(f64, f64)>)> {
    log_uniform(-1.0, 5.0).prop_flat_map(|size| (Just(size), prop::collection::vec((-size..size, -size..size), 2..10)))
}

proptest! {
    // Every point turns by the angle about the pivot: the same distance from it, the angle between them the angle.
    #[test]
    fn rotating_turns_every_point_by_the_angle_about_the_pivot(
        (size, points) in polyline(),
        angle in -720.0f64..720.0,
        pivot in (-1.0f64..1.0, -1.0f64..1.0),
    ) {
        let pivot = (pivot.0 * size, pivot.1 * size);
        let path = Path::polyline(points, false).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let turned = path
            .rotate(angle, Some(Point::new(pivot.0, pivot.1)))
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        let (before, after) = (corners(&path).map_err(|e| TestCaseError::fail(e.to_string()))?, corners(&turned).map_err(|e| TestCaseError::fail(e.to_string()))?);
        prop_assert_eq!(before.len(), after.len());
        for (from, to) in before.iter().zip(&after) {
            let (ax, ay) = (from.0 - pivot.0, from.1 - pivot.1);
            let (bx, by) = (to.0 - pivot.0, to.1 - pivot.1);
            prop_assert!((distance(pivot, *from) - distance(pivot, *to)).abs() <= 1e-9 * size, "distance");
            // the signed angle from a to b, with y down and clockwise positive, is the angle turned
            let between = libm::atan2(ax * by - ay * bx, ax * bx + ay * by);
            let off_by = (between - angle.to_radians()).rem_euclid(2.0 * PI);
            let off_by = off_by.min(2.0 * PI - off_by);
            // a point on the pivot has no angle; otherwise the angle is exact to the precision of the point
            prop_assert!(distance(pivot, *from) < 1e-6 * size || off_by <= 1e-6, "angle off by {}", off_by);
        }
    }

    // Turning by an angle and back by the same about the same pivot returns each point to within 1e-9.
    #[test]
    fn turning_and_turning_back_returns_the_path(
        (size, points) in polyline(),
        angle in -720.0f64..720.0,
        pivot in (-1.0f64..1.0, -1.0f64..1.0),
    ) {
        let pivot = Some(Point::new(pivot.0 * size, pivot.1 * size));
        let path = Path::polyline(points, false).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let there = path.rotate(angle, pivot).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let back = there.rotate(-angle, pivot).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let (before, after) = (corners(&path).map_err(|e| TestCaseError::fail(e.to_string()))?, corners(&back).map_err(|e| TestCaseError::fail(e.to_string()))?);
        for (from, to) in before.iter().zip(&after) {
            prop_assert!(distance(*from, *to) <= 1e-9 * size, "{:?} against {:?}", from, to);
        }
    }
}

// Scale ------------------------------------------------------------------------------------------------------------

#[test]
fn scaling_stretches_every_point_away_from_the_pivot() -> Outcome {
    let square = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?;
    // about the middle of the bounds by default
    assert_eq!(data(square.scale(2.0, None))?, "M-5 -5 L15 -5 L15 15 L-5 15 Z");
    assert_eq!(
        data(square.scale(2.0, Some(Point::new(0.0, 0.0))))?,
        "M0 0 L20 0 L20 20 L0 20 Z"
    );
    // a pair scales across and down separately
    let tall = Path::rect(4.0, 4.0, 8.0, 16.0, 0.0)?;
    assert_eq!(
        data(tall.scale((2.0, 0.5), Some(Point::new(0.0, 0.0))))?,
        "M8 2 L24 2 L24 10 L8 10 Z"
    );
    // the pivot stays where it is
    let from_pivot = Path::line((2.0, 3.0), (6.0, 5.0))?;
    assert_eq!(data(from_pivot.scale(3.0, Some(Point::new(2.0, 3.0))))?, "M2 3 L14 9");
    // a curve's handles stretch with it
    let shape = Path::from_svg_path(SHAPE)?;
    assert_eq!(
        data(shape.scale(0.5, Some(Point::new(0.0, 0.0))))?,
        "M0.5 1 L5 2.125 Q5.5 2.5 6 3.25 C0.5 1 1.5 2 2.5 3 Z"
    );
    Ok(())
}

#[test]
fn a_negative_factor_mirrors() -> Outcome {
    let line = Path::line((6.0, 1.0), (9.0, 3.0))?;
    let axis = Some(Point::new(5.0, 0.0));
    assert_eq!(data(line.scale(-1.0, axis))?, "M4 -1 L1 -3");
    assert_eq!(data(line.scale((-1.0, 1.0), axis))?, "M4 1 L1 3");
    assert_eq!(data(line.scale((1.0, -1.0), axis))?, "M6 -1 L9 -3");
    Ok(())
}

#[test]
fn mirroring_flips_the_winding_and_reversing_flips_it_back() -> Outcome {
    let polygon = Path::polygon((5.0, 6.0), 40.0, 7)?;
    assert!(shoelace_area(&corners(&polygon)?) > 0.0, "clockwise as built");
    let mirrored = polygon.scale((-1.0, 1.0), None)?;
    assert!(shoelace_area(&corners(&mirrored)?) < 0.0, "mirrored once");
    assert!(shoelace_area(&corners(&mirrored.reverse())?) > 0.0, "and reversed");
    // turned half way round (a mirror in both directions) keeps the winding
    let turned = polygon.scale(-1.0, None)?;
    assert!(shoelace_area(&corners(&turned)?) > 0.0, "mirrored twice");
    Ok(())
}

#[test]
fn what_is_inside_a_mirrored_outline_is_the_mirror_of_what_was_inside() -> Outcome {
    // an L, mirrored about the vertical line x = 15
    let ell = Path::polyline(
        [
            (0.0, 0.0),
            (30.0, 0.0),
            (30.0, 10.0),
            (10.0, 10.0),
            (10.0, 30.0),
            (0.0, 30.0),
        ],
        true,
    )?;
    let mirrored = ell.scale((-1.0, 1.0), Some(Point::new(15.0, 15.0)))?;
    // the points are at x and 30 - x; the L covers the top bar and the left column
    let cases = [
        ((25.0, 5.0), true),
        ((25.0, 20.0), false),
        ((5.0, 25.0), true),
        ((5.0, 5.0), true),
        ((20.0, 20.0), false),
        ((35.0, 35.0), false),
    ];
    for ((x, y), inside) in cases {
        assert_eq!(ell.contains((x, y))?, inside, "the L at ({x}, {y})");
        assert_eq!(
            mirrored.contains((30.0 - x, y))?,
            inside,
            "the mirror at ({}, {y})",
            30.0 - x
        );
    }
    // and the mirror is not the L: its column is on the right
    assert!(mirrored.contains((25.0, 20.0))? && !ell.contains((25.0, 20.0))?);
    Ok(())
}

#[test]
fn scaling_by_zero_or_something_that_is_not_a_number_is_refused_naming_it() -> Outcome {
    let line = Path::line((5.0, 3.0), (8.0, 3.0))?;
    let cases: [(Result<Path, Error>, &str); 9] = [
        (line.scale(0.0, None), "got 0"),
        (line.scale(-0.0, None), "got -0"),
        (line.scale((2.0, 0.0), None), "got 0"),
        (line.scale((0.0, 3.0), None), "got 0"),
        (line.scale(f64::NAN, None), "NaN"),
        (line.scale((1.0, f64::NAN), None), "NaN"),
        (line.scale(f64::INFINITY, None), "inf"),
        (Path::new().scale(0.0, None), "got 0"),
        (Path::new().scale((f64::NEG_INFINITY, 1.0), None), "-inf"),
    ];
    for (outcome, shown) in cases {
        let (name, message) = parameter_refusal(outcome)?;
        assert_eq!(name, "by");
        assert!(message.contains(shown), "{message}");
    }
    let (name, message) = parameter_refusal(line.scale(2.0, Some(Point::new(f64::NAN, 1.0))))?;
    assert_eq!((name.as_str(), message.contains("NaN")), ("about", true), "{message}");
    parameter_refusal(Path::new().scale(2.0, Some(Point::new(1.0, f64::INFINITY))))?;
    // small, negative and mixed factors are fine
    for by in [1e-300, -2.0, 1.0] {
        line.scale(by, None)?;
    }
    line.scale((-1.0, 1e-3), None)?;
    // a factor that carries a point beyond what a number holds
    let (name, message) = parameter_refusal(Path::line((1.0, 1.0), (1e90, 0.0))?.scale(1e20, None))?;
    assert_eq!(
        (name.as_str(), message.contains("too large")),
        ("by", true),
        "{message}"
    );
    Ok(())
}

#[test]
fn an_empty_path_and_a_lone_point_stay_as_they_are() -> Outcome {
    let nothing = Path::new();
    assert_eq!(nothing.translate(3.0, 4.0)?, nothing);
    assert_eq!(nothing.rotate(30.0, None)?, nothing);
    assert_eq!(nothing.scale(2.0, None)?, nothing);
    assert_eq!(
        nothing.map_points(|_| Err(Error::Geometry {
            verb: "unused".into(),
            message: "never called".into()
        }))?,
        nothing
    );
    let dot = Path::start((3.0, 4.0))?;
    assert_eq!(dot.rotate(30.0, None)?, dot);
    assert_eq!(dot.scale(2.0, None)?, dot);
    Ok(())
}

proptest! {
    // Scaling by k scales the length by |k|, whichever way it mirrors.
    #[test]
    fn scaling_by_a_factor_scales_the_length_by_its_size(
        sides in 3u32..=40,
        radius in log_uniform(-1.0, 4.0),
        size in log_uniform(-1.0, 1.5),
        negative in any::<bool>(),
    ) {
        let factor = if negative { -size } else { size };
        let shape = Path::star((7.0, -3.0), radius, radius * 0.4, i64::from(sides)).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let scaled = shape.scale(factor, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert!((scaled.length() - size * shape.length()).abs() <= 1e-9 * size * shape.length(), "{} against {}", scaled.length(), size * shape.length());
    }

    // Scaling by k and by 1/k about the same pivot returns the points.
    #[test]
    fn scaling_and_scaling_back_returns_the_path(
        (size, points) in polyline(),
        factor in (log_uniform(-1.0, 1.0), any::<bool>(), log_uniform(-1.0, 1.0), any::<bool>()),
        pivot in (-1.0f64..1.0, -1.0f64..1.0),
    ) {
        let (x, y) = (if factor.1 { -factor.0 } else { factor.0 }, if factor.3 { -factor.2 } else { factor.2 });
        let pivot = Some(Point::new(pivot.0 * size, pivot.1 * size));
        let path = Path::polyline(points, false).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let there = path.scale((x, y), pivot).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let back = there.scale((1.0 / x, 1.0 / y), pivot).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let (before, after) = (corners(&path).map_err(|e| TestCaseError::fail(e.to_string()))?, corners(&back).map_err(|e| TestCaseError::fail(e.to_string()))?);
        for (from, to) in before.iter().zip(&after) {
            prop_assert!(distance(*from, *to) <= 1e-9 * size * (1.0 + 10.0 / x.abs().min(y.abs())), "{:?} against {:?}", from, to);
        }
    }

    // A mirrored outline contains the mirror of what the outline contained, away from the edges.
    #[test]
    fn a_mirrored_outline_contains_the_mirrored_points(
        size in log_uniform(-1.0, 4.0),
        corners in prop::collection::vec((-1.0f64..1.0, -1.0f64..1.0), 3..10),
        point in (-1.2f64..1.2, -1.2f64..1.2),
        axis in -1.0f64..1.0,
    ) {
        let corners: Vec<(f64, f64)> = corners.into_iter().map(|(x, y)| (x * size, y * size)).collect();
        let point = (point.0 * size, point.1 * size);
        let axis = axis * size;
        prop_assume!(clearance(&corners, point) > 1e-6 * size);
        let outline = Path::polyline(corners.clone(), true).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let mirrored = outline.scale((-1.0, 1.0), Some(Point::new(axis, 0.0))).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let expected = winding(&corners, point) != 0;
        prop_assert_eq!(outline.contains(point).map_err(|e| TestCaseError::fail(e.to_string()))?, expected);
        prop_assert_eq!(mirrored.contains((2.0 * axis - point.0, point.1)).map_err(|e| TestCaseError::fail(e.to_string()))?, expected);
    }
}

// Rotating and scaling keep the structure ---------------------------------------------------------------------------

#[test]
fn a_turned_path_keeps_its_curves_and_its_closing() -> Outcome {
    // (x, y) becomes (-y, x) a quarter turn clockwise about the origin
    let shape = Path::from_svg_path(SHAPE)?;
    assert_eq!(
        data(shape.rotate(90.0, Some(Point::new(0.0, 0.0))))?,
        "M-2 1 L-4.25 10 Q-5 11 -6.5 12 C-2 1 -4 3 -6 5 Z"
    );
    Ok(())
}

// Map points -------------------------------------------------------------------------------------------------------

#[test]
fn every_point_goes_through_the_function_on_and_off_the_curve() -> Outcome {
    let shape = Path::from_svg_path(SHAPE)?;
    let mapped = shape.map_points(|point| Ok(Point::new(point.x * 2.0 + 1.0, point.y - 0.5)))?;
    assert_eq!(
        mapped.to_svg_path(),
        "M3 1.5 L21 3.75 Q23 4.5 25 6 C3 1.5 7 3.5 11 5.5 Z"
    );
    // each point once, in the order they are written
    let two = Path::from_svg_path("M1 2 L3 4 Q5 6 7 8 M9 10 C11 12 13 14 15 16")?;
    let mut seen = Vec::new();
    two.map_points(|point| {
        seen.push((point.x, point.y));
        Ok(point)
    })?;
    assert_eq!(
        seen,
        [
            (1.0, 2.0),
            (3.0, 4.0),
            (5.0, 6.0),
            (7.0, 8.0),
            (9.0, 10.0),
            (11.0, 12.0),
            (13.0, 14.0),
            (15.0, 16.0)
        ]
    );
    Ok(())
}

#[test]
fn mapping_every_point_to_itself_gives_the_same_path() -> Outcome {
    for data in [
        SHAPE,
        "M0 0 L10 0 L10 10 Z M20 20 Q25 30 30 20 M5 5",
        "M3 4",
        "M0 0 C4 1 5 2 0 0 Z",
        "",
    ] {
        let path = Path::from_svg_path(data)?;
        assert_eq!(path.map_points(Ok)?, path, "{data}");
    }
    Ok(())
}

#[test]
fn what_the_function_returns_as_an_error_comes_out_unchanged() -> Outcome {
    let shape = Path::from_svg_path(SHAPE)?;
    for failure in [
        Error::Geometry {
            verb: "wobble".into(),
            message: "the noise fell through".into(),
        },
        Error::Parameter {
            parameter: "amplitude".into(),
            message: "must be above 0, got -3".into(),
        },
    ] {
        let mut calls = 0;
        let outcome = shape.map_points(|point| {
            calls += 1;
            if calls == 3 { Err(failure.clone()) } else { Ok(point) }
        });
        assert_eq!(outcome, Err(failure));
    }
    Ok(())
}

#[test]
fn a_function_that_returns_what_is_not_a_point_is_refused() -> Outcome {
    let shape = Path::from_svg_path(SHAPE)?;
    let (name, message) = parameter_refusal(shape.map_points(|point| Ok(Point::new(point.x, f64::NAN))))?;
    assert_eq!((name.as_str(), message.contains("NaN")), ("fn", true), "{message}");
    let (name, message) = parameter_refusal(shape.map_points(|point| Ok(Point::new(f64::INFINITY, point.y))))?;
    assert_eq!((name.as_str(), message.contains("inf")), ("fn", true), "{message}");
    Ok(())
}

#[test]
fn mapping_a_closed_outline_to_one_point_leaves_a_closed_dot() -> Outcome {
    // the lines that would run back to the start are no longer kept, as when closing a path by hand
    let square = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?;
    let dot = square.map_points(|_| Ok(Point::new(5.0, 5.0)))?;
    assert_eq!(dot.to_svg_path(), "M5 5 Z");
    assert_eq!(Path::from_svg_path(&dot.to_svg_path())?, dot);
    Ok(())
}
