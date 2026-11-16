//! `Path::simplify` through the public API: a dense polyline fitted by the fewest smooth curves within a tolerance,
//! sharp corners kept. The usage first: a hand-drawn line of a hundred points becomes a handful of segments. Expected paths are
//! written by hand; the guarantees (every input point near the output, every output point near the input) are checked
//! against a brute-force distance written here, not against the library's own measuring.

use crate::common::{Corners, Outcome, commands, parameter_refusal};
use bullterrier_vector::{Error, Path};
use proptest::prelude::*;
use std::f64::consts::PI;
use std::fmt::Write;

/// The points of a circle of radius `radius` about `(cx, cy)`, `count` of them from angle 0, not repeating the first.
fn ring(cx: f64, cy: f64, radius: f64, count: usize) -> Corners {
    (0..count)
        .map(|i| {
            #[expect(clippy::cast_precision_loss, reason = "a few hundred points")]
            let angle = 2.0 * PI * i as f64 / count as f64;
            (cx + radius * libm::cos(angle), cy + radius * libm::sin(angle))
        })
        .collect()
}

/// A smooth open line: `count` points of a sine wave 300 wide.
fn wave(count: usize) -> Corners {
    (0..count)
        .map(|i| {
            #[expect(clippy::cast_precision_loss, reason = "a few hundred points")]
            let x = 300.0 * i as f64 / (count - 1) as f64;
            (x, 40.0 * libm::sin(x / 30.0))
        })
        .collect()
}

/// The segments of `path`: how many commands after a `M` or a `Z`.
fn segments(path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(commands(&path.to_svg_path())?
        .iter()
        .filter(|(letter, _)| matches!(letter, 'L' | 'Q' | 'C'))
        .count())
}

fn letters(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(commands(&path.to_svg_path())?
        .iter()
        .map(|(letter, _)| *letter)
        .collect())
}

/// The distance from `point` to the segment `from` to `to`.
fn to_segment(point: (f64, f64), from: (f64, f64), to: (f64, f64)) -> f64 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let square = dx * dx + dy * dy;
    let along = if square == 0.0 {
        0.0
    } else {
        (((point.0 - from.0) * dx + (point.1 - from.1) * dy) / square).clamp(0.0, 1.0)
    };
    let (x, y) = (from.0 + along * dx, from.1 + along * dy);
    ((point.0 - x) * (point.0 - x) + (point.1 - y) * (point.1 - y)).sqrt()
}

/// The distance from `point` to the polyline `points` (its closing edge too if `closed`).
fn to_polyline(point: (f64, f64), points: &[(f64, f64)], closed: bool) -> f64 {
    let mut nearest = f64::INFINITY;
    for pair in points.windows(2) {
        nearest = nearest.min(to_segment(point, pair[0], pair[1]));
    }
    if closed && let (Some(&first), Some(&last)) = (points.first(), points.last()) {
        nearest = nearest.min(to_segment(point, last, first));
    }
    if points.len() == 1 {
        nearest = to_segment(point, points[0], points[0]);
    }
    nearest
}

/// Points 0.1 apart along `path` (its sub-paths each from their start), read from its resampling.
fn samples(path: &Path) -> Result<Vec<(f64, f64)>, Box<dyn std::error::Error>> {
    let mut found = Vec::new();
    for (letter, numbers) in commands(&path.resample(0.1)?.to_svg_path())? {
        if let ('M' | 'L', [x, y]) = (letter, numbers.as_slice()) {
            found.push((*x, *y));
        }
    }
    Ok(found)
}

/// Checks the two halves of the promise for the polyline `points` simplified to `output` within `tolerance`: every
/// input point is within `tolerance` of the output and every output sample within `tolerance` of the input. The
/// samples are 0.1 apart and the walk is flat to 0.01, so each bound has 0.1 to spare.
fn assert_within(output: &Path, points: &[(f64, f64)], closed: bool, tolerance: f64) -> Outcome {
    let slack = 0.1;
    let found = samples(output)?;
    for (index, &point) in points.iter().enumerate() {
        let near = found
            .iter()
            .map(|&sample| to_segment(point, sample, sample))
            .fold(f64::INFINITY, f64::min);
        assert!(
            near <= tolerance + slack,
            "input point {index} {point:?} is {near} from the output (tolerance {tolerance})"
        );
    }
    for &sample in &found {
        let near = to_polyline(sample, points, closed);
        assert!(
            near <= tolerance + slack,
            "output point {sample:?} is {near} from the input (tolerance {tolerance})"
        );
    }
    Ok(())
}

// By hand --------------------------------------------------------------------------------------------------------------

#[test]
fn a_hand_drawn_line_of_a_hundred_points_becomes_a_handful_of_segments() -> Outcome {
    // the usage: a dense polyline in, `simplify(tolerance)`, a few curves out
    let drawn = wave(100);
    let line = Path::polyline(drawn.clone(), false)?;
    let smooth = line.simplify(0.5)?;
    assert!(segments(&smooth)? <= 10, "{}", smooth.to_svg_path());
    assert!(letters(&smooth)?.contains('C'), "{}", smooth.to_svg_path());
    assert_within(&smooth, &drawn, false, 0.5)?;
    Ok(())
}

#[test]
fn points_on_a_straight_line_are_one_line_segment() -> Outcome {
    let points: Corners = (0..100).map(|i| (f64::from(i), 2.0 * f64::from(i))).collect();
    let line = Path::polyline(points, false)?;
    assert_eq!(line.simplify(0.5)?.to_svg_path(), "M0 0 L99 198");
    // however small the tolerance: the points are exactly on the line
    assert_eq!(line.simplify(1e-9)?.to_svg_path(), "M0 0 L99 198");
    Ok(())
}

#[test]
fn a_square_with_many_points_on_each_edge_is_four_lines_and_four_corners() -> Outcome {
    let mut points = Vec::new();
    for (from, to) in [
        ((0.0, 0.0), (100.0, 0.0)),
        ((100.0, 0.0), (100.0, 100.0)),
        ((100.0, 100.0), (0.0, 100.0)),
        ((0.0, 100.0), (0.0, 0.0)),
    ] {
        for i in 0..25 {
            let share = f64::from(i) / 25.0;
            points.push((from.0 + (to.0 - from.0) * share, from.1 + (to.1 - from.1) * share));
        }
    }
    let square = Path::polyline(points, true)?;
    assert_eq!(square.simplify(0.5)?.to_svg_path(), "M0 0 L100 0 L100 100 L0 100 Z");
    Ok(())
}

#[test]
fn the_corners_of_a_half_disc_stay_and_its_arc_is_a_few_curves() -> Outcome {
    // from the top of the arc down to the right corner, back along the diameter to the left corner, up the arc
    let mut points = vec![(50.0, -50.0)];
    for i in 1..50 {
        let angle = -PI / 2.0 + PI / 2.0 * f64::from(i) / 50.0;
        points.push((50.0 + 50.0 * libm::cos(angle), 50.0 * libm::sin(angle)));
    }
    points.push((100.0, 0.0));
    for i in 1..40 {
        points.push((100.0 - 100.0 * f64::from(i) / 40.0, 0.0));
    }
    points.push((0.0, 0.0));
    for i in 1..50 {
        let angle = PI + PI / 2.0 * f64::from(i) / 50.0;
        points.push((50.0 + 50.0 * libm::cos(angle), 50.0 * libm::sin(angle)));
    }
    let disc = Path::polyline(points.clone(), true)?;
    let found = disc.simplify(0.5)?;
    let data = found.to_svg_path();
    // the written start is kept; the corners are where a curve ends and the diameter, one line, begins and ends
    assert!(data.starts_with("M50 -50 C"), "{data}");
    assert!(data.contains(" 100 0 L0 0 C"), "{data}");
    assert_eq!(data.matches('L').count(), 1, "{data}");
    assert!(data.ends_with(" 50 -50 Z"), "{data}");
    assert_within(&found, &points, true, 0.5)?;
    Ok(())
}

#[test]
fn a_circle_of_four_hundred_points_is_a_few_curves_closed_from_its_start() -> Outcome {
    let points = ring(150.0, 150.0, 100.0, 400);
    let circle = Path::polyline(points.clone(), true)?;
    let found = circle.simplify(0.5)?;
    let count = segments(&found)?;
    assert!((3..=12).contains(&count), "{count} segments: {}", found.to_svg_path());
    let data = found.to_svg_path();
    assert!(data.starts_with("M250 150 C") && data.ends_with(" Z"), "{data}");
    assert_within(&found, &points, true, 0.5)?;
    Ok(())
}

#[test]
fn a_polygon_far_from_round_is_kept_as_it_is() -> Outcome {
    // twelve points of a circle of radius 100: no turn is sharp, yet a smooth curve through them would bulge 3.4 off
    // the edges (the circle's sagitta), so every edge stays a line and the polygon is unchanged
    let points = ring(0.0, 0.0, 100.0, 12);
    let polygon = Path::polyline(points.clone(), true)?;
    let found = polygon.simplify(0.5)?;
    assert_eq!(found.to_svg_path(), polygon.to_svg_path());
    assert_within(&found, &points, true, 0.5)?;
    Ok(())
}

#[test]
fn a_looser_tolerance_cuts_a_polygon_whose_edges_are_within_it() -> Outcome {
    // the same twelve points with a tolerance past the sagitta: a few curves are allowed
    let points = ring(0.0, 0.0, 100.0, 12);
    let polygon = Path::polyline(points.clone(), true)?;
    let found = polygon.simplify(4.0)?;
    assert!(segments(&found)? < 12, "{}", found.to_svg_path());
    assert_within(&found, &points, true, 4.0)?;
    Ok(())
}

#[test]
fn a_path_never_gets_more_segments_than_it_had() -> Outcome {
    // four cubics already fit a circle: tighter and looser tolerances leave at most those
    let circle = Path::circle((0.0, 0.0), 100.0)?;
    for tolerance in [0.001, 0.01, 0.5, 5.0] {
        let found = circle.simplify(tolerance)?;
        assert!(segments(&found)? <= 4, "{tolerance}: {}", found.to_svg_path());
    }
    Ok(())
}

#[test]
fn a_larger_tolerance_never_leaves_more_segments() -> Outcome {
    let inputs = [
        (Path::polyline(ring(0.0, 0.0, 100.0, 400), true)?, "a circle"),
        (Path::polyline(wave(300), false)?, "a wave"),
    ];
    for (path, name) in inputs {
        let mut before = usize::MAX;
        for tolerance in [0.02, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0] {
            let count = segments(&path.simplify(tolerance)?)?;
            assert!(
                count <= before,
                "{name}: {count} segments at {tolerance}, {before} at a tighter one"
            );
            before = count;
        }
    }
    Ok(())
}

#[test]
fn simplifying_again_with_the_same_tolerance_never_adds_segments() -> Outcome {
    let inputs = [
        Path::polyline(ring(0.0, 0.0, 100.0, 400), true)?,
        Path::polyline(wave(300), false)?,
        Path::polyline(ring(0.0, 0.0, 100.0, 12), true)?,
    ];
    for path in inputs {
        for tolerance in [0.1, 0.5, 2.0] {
            let once = path.simplify(tolerance)?;
            let twice = once.simplify(tolerance)?;
            assert!(
                segments(&twice)? <= segments(&once)?,
                "{tolerance}: {} then {}",
                once.to_svg_path(),
                twice.to_svg_path()
            );
        }
    }
    Ok(())
}

#[test]
fn each_sub_path_is_simplified_on_its_own() -> Outcome {
    let two = Path::from_svg_path("M0 0 L5 0 L10 0 M0 5 L4 5 L8 5")?;
    assert_eq!(two.simplify(0.5)?.to_svg_path(), "M0 0 L10 0 M0 5 L8 5");
    Ok(())
}

#[test]
fn a_point_two_points_and_nothing_are_kept() -> Outcome {
    assert_eq!(Path::new().simplify(0.5)?, Path::new());
    let point = Path::from_svg_path("M5 5")?;
    assert_eq!(point.simplify(0.5)?, point);
    let line = Path::from_svg_path("M0 0 L10 3")?;
    assert_eq!(line.simplify(0.5)?, line);
    // the same point again and again has no length: it stays
    let still = Path::from_svg_path("M5 5 L5 5 L5 5")?;
    assert_eq!(still.simplify(0.5)?, still);
    // a closed outline of two points
    let flat = Path::from_svg_path("M0 0 L10 0 Z")?;
    assert_eq!(flat.simplify(0.5)?, flat);
    Ok(())
}

#[test]
fn a_turn_past_forty_five_degrees_is_a_corner_and_a_smaller_one_is_not() -> Outcome {
    // 40 degrees: the polyline is smooth, one curve fits (the points are within 0.5 of its chord is false: 30 off)
    let (c40, s40) = (libm::cos(40.0 * PI / 180.0), libm::sin(40.0 * PI / 180.0));
    let gentle = Path::polyline([(0.0, 0.0), (100.0, 0.0), (100.0 + 100.0 * c40, 100.0 * s40)], false)?;
    // within the tolerance of nothing but themselves, the three points keep both edges, as lines or one curve
    let count = segments(&gentle.simplify(0.5)?)?;
    assert!(count <= 2, "{count}");
    // 50 degrees: a corner, kept exactly: two lines
    let (c50, s50) = (libm::cos(50.0 * PI / 180.0), libm::sin(50.0 * PI / 180.0));
    let sharp_points = [(0.0, 0.0), (100.0, 0.0), (100.0 + 100.0 * c50, 100.0 * s50)];
    let sharp = Path::polyline(sharp_points, false)?;
    assert_eq!(sharp.simplify(0.5)?, sharp);
    // a corner in the middle of dense points stays a point of the output
    let mut dense: Corners = (0..=50).map(|i| (f64::from(i) * 2.0, 0.0)).collect();
    dense.extend((1..=50).map(|i| (100.0, f64::from(i) * 2.0)));
    let found = Path::polyline(dense, false)?.simplify(0.5)?;
    assert_eq!(found.to_svg_path(), "M0 0 L100 0 L100 100");
    Ok(())
}

#[test]
fn a_turn_of_exactly_forty_five_degrees_is_not_a_corner() -> Outcome {
    // edges (100, 0) then (100, 100): a turn of exactly 45 degrees, not past it. Of these edge lengths the unit
    // vectors' dot product rounds to exactly the cosine of 45 degrees, so only a strict comparison leaves the middle
    // point free to go: it is 44.7 from the chord, within a tolerance of 50.
    let turn = Path::polyline([(0.0, 0.0), (100.0, 0.0), (200.0, 100.0)], false)?;
    assert_eq!(turn.simplify(50.0)?.to_svg_path(), "M0 0 L200 100");
    Ok(())
}

#[test]
fn the_ends_of_an_open_path_are_exact() -> Outcome {
    let points = wave(200);
    let found = Path::polyline(points.clone(), false)?.simplify(0.5)?;
    let data = found.to_svg_path();
    let (first, last) = (points[0], points[199]);
    assert!(data.starts_with(&format!("M{} {}", first.0, first.1)), "{data}");
    let numbers = commands(&data)?;
    let end = numbers
        .last()
        .and_then(|(_, n)| n.get(n.len().saturating_sub(2)..))
        .ok_or("no end")?;
    assert_eq!((end[0], end[1]), last, "{data}");
    Ok(())
}

#[test]
fn a_curved_path_is_simplified_from_its_flattening() -> Outcome {
    // a circle of many small cubics (a polyline of 400 points written as quadratics is not the same input)
    let mut path = Path::start((100.0, 0.0))?;
    for i in 1..=40 {
        let angle = 2.0 * PI * f64::from(i) / 40.0;
        let before = 2.0 * PI * f64::from(i - 1) / 40.0;
        // a cubic arc of a tenth of a turn: control points a third of the way along the tangent
        let k = 4.0 / 3.0 * libm::tan((angle - before) / 4.0) * 100.0;
        path = path.curve_to(
            (
                100.0 * libm::cos(before) - k * libm::sin(before),
                100.0 * libm::sin(before) + k * libm::cos(before),
            ),
            (
                100.0 * libm::cos(angle) + k * libm::sin(angle),
                100.0 * libm::sin(angle) - k * libm::cos(angle),
            ),
            (100.0 * libm::cos(angle), 100.0 * libm::sin(angle)),
        )?;
    }
    let path = path.close()?;
    assert_eq!(segments(&path)?, 40);
    let found = path.simplify(0.5)?;
    assert!(segments(&found)? <= 8, "{}", found.to_svg_path());
    Ok(())
}

#[test]
fn a_zigzag_of_ten_thousand_points_ends_and_stays_within_the_tolerance() -> Outcome {
    // points 3 apart, 0.8 up and down: no turn is sharp, and the wiggle is past the tolerance, so the fit has to split
    // down to the points
    let points: Corners = (0..10_000)
        .map(|i| (3.0 * f64::from(i), if i % 2 == 0 { 0.8 } else { -0.8 }))
        .collect();
    let zigzag = Path::polyline(points.clone(), false)?;
    let found = zigzag.simplify(0.5)?;
    assert!(segments(&found)? <= 9_999);
    // the same, with a wiggle inside the tolerance: one line
    let small: Corners = (0..10_000)
        .map(|i| (3.0 * f64::from(i), if i % 2 == 0 { 0.2 } else { -0.2 }))
        .collect();
    assert_eq!(segments(&Path::polyline(small, false)?.simplify(0.5)?)?, 1);
    Ok(())
}

#[test]
fn coordinates_near_the_limit_are_simplified_or_refused_never_overflowed() -> Outcome {
    let big = Path::from_svg_path("M-1e100 -1e100 L0 0 L1e100 1e100")?;
    let found = big.simplify(0.5)?;
    assert_eq!(segments(&found)?, 1);
    let arc = Path::polyline(ring(0.0, 0.0, 1e100, 64), true)?;
    // either a result whose coordinates are all within the limit (the writer would not print them otherwise) or a refusal
    match arc.simplify(1e98) {
        Ok(path) => assert!(path.to_svg_path().len() > 2),
        Err(Error::Parameter { .. }) => {}
        Err(other) => return Err(other.into()),
    }
    Ok(())
}

#[test]
fn a_tolerance_not_above_zero_or_not_finite_or_past_1e100_is_refused_naming_it() -> Outcome {
    let line = Path::from_svg_path("M0 0 L5 0 L10 0")?;
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.000_000_1e100] {
        let (parameter, _) = parameter_refusal(line.simplify(tolerance))?;
        assert_eq!(parameter, "tolerance", "{tolerance}");
    }
    assert!(line.simplify(1e100).is_ok());
    assert!(line.simplify(1e-300).is_ok());
    Ok(())
}

#[test]
fn more_flattened_edges_than_max_segments_are_refused_before_they_are_all_made() -> Outcome {
    // each of these cubics flattens to 2^16 edges: 160 of them are past ten million
    let mut data = String::from("M0 0");
    for i in 0..160 {
        let x = f64::from(i) * 1e9;
        write!(data, " C{} 1e9 {} -1e9 {} 0", x + 3e8, x + 6e8, x + 1e9)?;
    }
    let huge = Path::from_svg_path(&data)?;
    let (parameter, message) = parameter_refusal(huge.simplify(0.5))?;
    assert!(
        parameter.contains("tolerance") && message.contains("MAX_SEGMENTS"),
        "{parameter}: {message}"
    );
    Ok(())
}

// Properties -------------------------------------------------------------------------------------------------------

/// A smooth open polyline: steps of 3 to 40 pixels, each turning at most 0.7 radians (40 degrees) from the last.
fn smooth_polyline() -> impl Strategy<Value = Corners> {
    prop::collection::vec((-0.7f64..0.7, 3.0f64..40.0), 2..40).prop_map(|steps| {
        let (mut at, mut heading) = ((0.0, 0.0), 0.0f64);
        let mut points = vec![at];
        for (turn, length) in steps {
            heading += turn;
            at = (at.0 + length * libm::cos(heading), at.1 + length * libm::sin(heading));
            points.push(at);
        }
        points
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn every_input_point_is_near_the_output_and_every_output_point_near_the_input(
        points in smooth_polyline(),
        tolerance in 0.1f64..5.0,
    ) {
        let path = Path::polyline(points.clone(), false).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let found = path.simplify(tolerance).map_err(|e| TestCaseError::fail(e.to_string()))?;
        assert_within(&found, &points, false, tolerance).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let count = segments(&found).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert!(count < points.len(), "{} segments from {} points", count, points.len());
        // the ends are exact
        let data = found.to_svg_path();
        prop_assert!(data.starts_with(&format!("M{} {}", points[0].0, points[0].1)), "{}", data);
    }
}
