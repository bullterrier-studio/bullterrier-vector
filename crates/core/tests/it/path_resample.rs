//! `Path::resample` through the public API: straight segments with corners a fixed distance apart along the path.
//! Expected paths are written by hand from the definition (corners at 0, s, 2s, ... along each sub-path, the end kept);
//! the properties compare the result with the path it came from.

use crate::common::{Corners, Outcome, build, commands, distance, log_uniform, parameter_refusal, step};
use bullterrier_vector::{Error, Path};
use proptest::prelude::*;

/// The corners of `path`'s sub-paths, each as its list of points, from the canonical path data.
fn corners(path: &Path) -> Result<Vec<Corners>, Box<dyn std::error::Error>> {
    let mut found: Vec<Corners> = Vec::new();
    for (letter, numbers) in commands(&path.to_svg_path())? {
        match (letter, numbers.as_slice()) {
            ('M', [x, y]) => found.push(vec![(*x, *y)]),
            ('L', [x, y]) => found.last_mut().ok_or("a line before a move")?.push((*x, *y)),
            ('Z', []) => {}
            _ => return Err(format!("{letter} {numbers:?} is not a straight segment").into()),
        }
    }
    Ok(found)
}

// By hand --------------------------------------------------------------------------------------------------------------

#[test]
fn corners_fall_every_spacing_along_a_line_and_the_end_is_kept() -> Outcome {
    let line = Path::line((0.0, 0.0), (10.0, 0.0))?;
    assert_eq!(line.resample(4.0)?.to_svg_path(), "M0 0 L4 0 L8 0 L10 0");
    // an end that falls on a corner is stored once
    assert_eq!(line.resample(5.0)?.to_svg_path(), "M0 0 L5 0 L10 0");
    // a spacing past the length leaves the two ends
    assert_eq!(line.resample(25.0)?.to_svg_path(), "M0 0 L10 0");
    Ok(())
}

#[test]
fn the_distance_runs_round_a_corner_so_a_gap_is_measured_along_the_path() -> Outcome {
    let bend = Path::from_svg_path("M0 0 L10 0 L10 10")?;
    // 12 along is 2 down the second leg; 8 is 8 along the first
    assert_eq!(bend.resample(4.0)?.to_svg_path(), "M0 0 L4 0 L8 0 L10 2 L10 6 L10 10");
    Ok(())
}

#[test]
fn a_closed_path_stays_closed_from_its_written_start_and_its_last_gap_wraps() -> Outcome {
    let square = Path::from_svg_path("M0 0 L10 0 L10 10 L0 10 Z")?;
    assert_eq!(square.resample(10.0)?.to_svg_path(), "M0 0 L10 0 L10 10 L0 10 Z");
    // the perimeter is 40: corners at 0, 15 and 30 along it, and the 10 left wrap to the start
    assert_eq!(square.resample(15.0)?.to_svg_path(), "M0 0 L10 5 L0 10 Z");
    // a spacing past the perimeter leaves the start alone
    assert_eq!(square.resample(100.0)?.to_svg_path(), "M0 0 Z");
    Ok(())
}

#[test]
fn each_sub_path_is_measured_from_its_own_start() -> Outcome {
    let two = Path::from_svg_path("M0 0 L10 0 M0 5 L6 5")?;
    assert_eq!(two.resample(4.0)?.to_svg_path(), "M0 0 L4 0 L8 0 L10 0 M0 5 L4 5 L6 5");
    Ok(())
}

#[test]
fn a_point_stays_a_point_and_an_empty_path_stays_empty() -> Outcome {
    let point = Path::from_svg_path("M3 4 M5 6 L9 6")?;
    assert_eq!(point.resample(2.0)?.to_svg_path(), "M3 4 M5 6 L7 6 L9 6");
    assert_eq!(Path::new().resample(2.0)?, Path::new());
    Ok(())
}

#[test]
fn a_circle_is_cut_into_equal_arcs_whose_corners_lie_on_it() -> Outcome {
    let circle = Path::circle((50.0, 50.0), 10.0)?;
    let spacing = circle.length() / 8.0;
    let found = corners(&circle.resample(spacing)?)?;
    assert_eq!(found.len(), 1);
    let ring = &found[0];
    assert_eq!(ring.len(), 8, "{ring:?}");
    // a regular octagon: every corner on the circle, every side 2 r sin(pi / 8)
    let side = 20.0 * libm::sin(std::f64::consts::PI / 8.0);
    for (index, corner) in ring.iter().enumerate() {
        let radius = distance(*corner, (50.0, 50.0));
        assert!((radius - 10.0).abs() < 0.02, "corner {index} at radius {radius}");
        let gap = distance(*corner, ring[(index + 1) % 8]);
        assert!((gap - side).abs() < 0.05, "side {index} is {gap}, not {side}");
    }
    Ok(())
}

// Refusals ---------------------------------------------------------------------------------------------------------

#[test]
fn a_spacing_that_is_not_above_zero_is_refused_by_name() -> Outcome {
    let line = Path::line((0.0, 0.0), (10.0, 0.0))?;
    for spacing in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.000_000_1e100] {
        let (name, _) = parameter_refusal(line.resample(spacing))?;
        assert_eq!(name, "spacing", "{spacing}");
    }
    assert!(line.resample(1e100).is_ok());
    Ok(())
}

#[test]
fn a_spacing_that_would_make_more_than_ten_million_corners_is_refused_before_any_are_made() -> Outcome {
    let line = Path::line((0.0, 0.0), (1.0e6, 0.0))?;
    let (name, message) = parameter_refusal(line.resample(1.0e-3))?;
    assert_eq!(name, "spacing");
    assert!(message.contains("MAX_SEGMENTS"), "{message}");
    // exactly at the limit is allowed by the count: 1e6 / 0.1 = 1e7 gaps, 10,000,001 corners is past it, so use 0.2
    assert!(line.resample(0.2).is_ok());
    // the counts add up over sub-paths
    let many = Path::from_svg_path("M0 0 L1000000 0 M0 1 L1000000 1")?;
    assert!(parameter_refusal(many.resample(0.15)).is_ok());
    Ok(())
}

// Properties -------------------------------------------------------------------------------------------------------

/// A path of up to 12 steps within a square of half side `size`, the size spread over orders of magnitude.
fn sized_path() -> impl Strategy<Value = Path> {
    log_uniform(0.0, 4.0)
        .prop_flat_map(|size| {
            let corner = (-size..size, -size..size);
            (corner.clone(), prop::collection::vec(step(corner), 0..12))
        })
        .prop_filter_map("the steps build a path", |(start, steps)| build(start, &steps).ok())
}

#[expect(clippy::needless_pass_by_value, reason = "used as the function of map_err")]
fn fail(error: impl ToString) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

proptest! {
    // Every corner lies on the original path (to within the 0.01 the walk flattens curves to), the ends are kept,
    // no gap is longer than the spacing (a curve is measured on its flattening, so by the same slack) and the result is no longer than the path.
    #[test]
    fn resampling_keeps_the_ends_stays_on_the_path_and_never_lengthens_it(
        path in sized_path(),
        share in 0.02f64..=1.5,
    ) {
        let length = path.length();
        prop_assume!(length > 0.0);
        let spacing = length * share;
        let sampled = path.resample(spacing).map_err(fail)?;
        let size = path.bounds().map_or(1.0, |bounds| bounds.width.max(bounds.height)).max(1.0);
        let slack = 0.02 + 1e-9 * size;
        for ring in corners(&sampled).map_err(fail)? {
            // a lone point is not on the measured walk, which has no edge there
            for corner in ring.iter().filter(|_| ring.len() > 1) {
                // the nearest place on each separate part (a position on a boundary between sub-paths is the later one's)
                let mut nearest = f64::INFINITY;
                for part in path.split() {
                    let at = part.locate((corner.0, corner.1), None).map_err(fail)?;
                    let found = part.point_at(at, None).map_err(fail)?;
                    nearest = nearest.min(distance(*corner, (found.x, found.y)));
                }
                prop_assert!(nearest <= slack, "{:?} is {} off the path", corner, nearest);
            }
            for pair in ring.windows(2) {
                prop_assert!(distance(pair[0], pair[1]) <= spacing * (1.0 + 1e-9) + slack, "a gap of {} past {}", distance(pair[0], pair[1]), spacing);
            }
        }
        prop_assert!(sampled.length() <= length + slack, "{} against {}", sampled.length(), length);
        // a closed path is walked from its top, not its written start: ends are for open paths
        if !path.to_svg_path().contains('Z') {
            let (start, end) = (path.point_at(0.0, None).map_err(fail)?, path.point_at(1.0, None).map_err(fail)?);
            let (first, last) = (sampled.point_at(0.0, None).map_err(fail)?, sampled.point_at(1.0, None).map_err(fail)?);
            prop_assert!(distance((first.x, first.y), (start.x, start.y)) <= slack);
            prop_assert!(distance((last.x, last.y), (end.x, end.y)) <= slack);
        }
    }

    // On straight sub-paths, the gaps along the path are the spacing, but the last; the number of corners is the
    // length over the spacing, rounded up, plus the end.
    #[test]
    fn a_polyline_has_as_many_corners_as_spacings_fit_and_the_end(
        points in prop::collection::vec((-100.0f64..100.0, -100.0f64..100.0), 2..8),
        spacing in 1.0f64..60.0,
    ) {
        let path = Path::polyline(points, false).map_err(fail)?;
        let length = path.length();
        prop_assume!(length > 0.0);
        let sampled = path.resample(spacing).map_err(fail)?;
        let found = corners(&sampled).map_err(fail)?;
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a small positive count")]
        let wanted = (length / spacing).ceil() as usize + 1;
        // a corner that falls within rounding of the end counts once or twice
        prop_assert!(found[0].len().abs_diff(wanted) <= 1, "{} corners, {} wanted", found[0].len(), wanted);
    }

    // The result converges on the path as the spacing shrinks: it is short of the length by at most two spacings at
    // each place the path turns.
    #[test]
    fn a_finer_spacing_loses_less_of_the_length(
        points in prop::collection::vec((-100.0f64..100.0, -100.0f64..100.0), 2..8),
        spacing in 0.05f64..20.0,
    ) {
        let path = Path::polyline(points, false).map_err(fail)?;
        #[expect(clippy::cast_precision_loss, reason = "a handful of segments")]
        let turns = path.to_svg_path().matches('L').count() as f64;
        let sampled = path.resample(spacing).map_err(fail)?;
        prop_assert!(path.length() - sampled.length() <= 2.0 * spacing * turns + 1e-9, "{} against {}", sampled.length(), path.length());
    }
}

#[test]
fn the_error_type_is_the_one_every_verb_returns() {
    // the signature: a plain number in, a path or an error out
    let _: fn(&Path, f64) -> Result<Path, Error> = Path::resample;
}

#[test]
fn an_end_within_rounding_of_a_corner_is_stored_once() -> Outcome {
    let line = Path::line((0.0, 0.0), (0.9, 0.0))?;
    assert_eq!(line.resample(0.3)?.to_svg_path(), "M0 0 L0.3 0 L0.6 0 L0.9 0");
    Ok(())
}
