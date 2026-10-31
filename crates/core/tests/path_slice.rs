//! Cutting a `Path` along its length and turning it round, through the public API. Expected paths are written by hand
//! from the shape's definition; the properties compare a path with itself seen another way (reversed, cut in two).

mod common;

use bullterrier_vector::{Path, Point};
use common::{Outcome, build, commands, distance, geometry_refusal, log_uniform, parameter_refusal, step};
use proptest::prelude::*;

/// The numbers `a` and `b` agree to within `tolerance`.
fn agree(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

/// Checks that `path` is written as `expected` commands (a letter and its numbers), the numbers to within `tolerance`.
fn assert_commands(path: &Path, expected: &[(char, &[f64])], tolerance: f64) -> Outcome {
    let data = path.to_svg_path();
    let found = commands(&data)?;
    let letters: String = found.iter().map(|(letter, _)| *letter).collect();
    let wanted: String = expected.iter().map(|(letter, _)| *letter).collect();
    assert_eq!(letters, wanted, "{data}");
    for ((letter, numbers), (_, wanted)) in found.iter().zip(expected) {
        assert_eq!(numbers.len(), wanted.len(), "{letter} in {data}");
        for (number, wanted) in numbers.iter().zip(*wanted) {
            assert!(
                agree(*number, *wanted, tolerance),
                "{letter}: {number} against {wanted} in {data}"
            );
        }
    }
    Ok(())
}

/// Where a path starts and where it ends.
type Ends = ((f64, f64), (f64, f64));

fn start_and_end(path: &Path) -> Result<Ends, Box<dyn std::error::Error>> {
    let (start, end) = (path.point_at(0.0, None)?, path.point_at(1.0, None)?);
    Ok(((start.x, start.y), (end.x, end.y)))
}

// Reverse ----------------------------------------------------------------------------------------------------------

#[test]
fn reversing_travels_the_same_outline_the_other_way() -> Outcome {
    let cases = [
        ("a line", "M1 2 L4 6", "M4 6 L1 2"),
        (
            "a quad and a cubic, handles in the other order",
            "M0 0 L10 0 Q15 5 10 10 C8 12 2 12 0 10",
            "M0 10 C2 12 8 12 10 10 Q15 5 10 0 L0 0",
        ),
        (
            "sub-paths in the other order",
            "M0 0 L1 1 M5 5 L6 7",
            "M6 7 L5 5 M1 1 L0 0",
        ),
        (
            "a closed outline keeps its start and goes back along the closing line first",
            "M0 0 L10 0 L10 5 L0 5 Z",
            "M0 0 L0 5 L10 5 L10 0 Z",
        ),
        (
            "a closed outline whose last segment is a curve home",
            "M0 0 C4 1 5 2 0 0 Z",
            "M0 0 C5 2 4 1 0 0 Z",
        ),
        (
            "a closed outline that starts with a curve",
            "M0 0 C3 -4 7 -4 10 0 L10 6 L0 6 Z",
            "M0 0 L0 6 L10 6 L10 0 C7 -4 3 -4 0 0 Z",
        ),
        ("a lone start", "M3 4", "M3 4"),
        ("a closed dot", "M3 4 Z", "M3 4 Z"),
        ("nothing", "", ""),
    ];
    for (what, data, expected) in cases {
        assert_eq!(Path::from_svg_path(data)?.reverse().to_svg_path(), expected, "{what}");
    }
    Ok(())
}

#[test]
fn a_reversed_circle_starts_at_the_top_and_goes_anticlockwise() -> Outcome {
    let circle = Path::circle((20.0, -30.0), 75.0)?;
    let reversed = circle.reverse();
    let top = reversed.point_at(0.0, None)?;
    assert!(agree(top.x, 20.0, 1e-9) && agree(top.y, -105.0, 1e-9), "{top:?}");
    let quarter = reversed.point_at(0.25, None)?;
    assert!(
        agree(quarter.x, -55.0, 1e-2) && agree(quarter.y, -30.0, 1e-2),
        "{quarter:?}"
    );
    Ok(())
}

/// A path of up to 12 steps within a square of half side `size`, the size spread over orders of magnitude.
fn sized_path() -> impl Strategy<Value = Path> {
    log_uniform(0.0, 4.0)
        .prop_flat_map(|size| {
            let corner = (-size..size, -size..size);
            (corner.clone(), prop::collection::vec(step(corner), 0..12))
        })
        .prop_filter_map("the steps build a path", |(start, steps)| build(start, &steps).ok())
}

#[test]
fn a_cut_exactly_at_a_corner_does_not_leave_a_segment_of_no_length() -> Outcome {
    let square = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?;
    // the first quarter of the length is the top edge to its far end, and the next quarter the right edge
    assert_eq!(square.slice(0.0, 0.25, None)?.to_svg_path(), "M0 0 L10 0");
    assert_eq!(square.slice(0.25, 0.5, None)?.to_svg_path(), "M10 0 L10 10");
    Ok(())
}

#[test]
fn a_path_of_several_points_with_no_length_is_cut_to_its_first() -> Outcome {
    let dots = Path::from_svg_path("M1 1 M2 2")?;
    assert_eq!(dots.slice(0.2, 0.7, None)?.to_svg_path(), "M1 1");
    Ok(())
}

proptest! {
    #[test]
    fn reversing_twice_is_the_same_path(path in sized_path()) {
        prop_assert_eq!(path.reverse().reverse(), path);
    }

    // A position t on the path is the position 1 - t on the reversed path.
    #[test]
    fn a_position_is_the_mirrored_position_on_the_reversed_path(path in sized_path(), at in 0.0f64..=1.0) {
        // a path with no length has one point, its first, and reversed that is another
        prop_assume!(path.length() > 0.0);
        let size = path.bounds().map_or(1.0, |bounds| bounds.width.max(bounds.height)).max(1.0);
        let forward = path.point_at(at, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let backward = path.reverse().point_at(1.0 - at, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let apart = distance((forward.x, forward.y), (backward.x, backward.y));
        prop_assert!(apart <= 1e-9 * size, "{:?} against {:?}", forward, backward);
    }

    #[test]
    fn a_shape_is_walked_back_by_the_reversed_shape(
        sides in 3u32..=40,
        radius in log_uniform(-2.0, 5.0),
        at in 0.0f64..=1.0,
    ) {
        for shape in [
            Path::polygon((3.0, 4.0), radius, i64::from(sides)),
            Path::star((3.0, 4.0), radius, radius * 0.45, i64::from(sides)),
            Path::circle((3.0, 4.0), radius),
            Path::rect(3.0, 4.0, radius, radius * 0.7, radius * 0.2),
        ] {
            let shape = shape.map_err(|e| TestCaseError::fail(e.to_string()))?;
            let forward = shape.point_at(at, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
            let backward = shape.reverse().point_at(1.0 - at, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
            let apart = distance((forward.x, forward.y), (backward.x, backward.y));
            prop_assert!(apart <= 1e-9 * (radius + 10.0), "{:?} against {:?}", forward, backward);
        }
    }
}

// Slice ------------------------------------------------------------------------------------------------------------

#[test]
fn a_piece_of_a_line_is_a_shorter_line() -> Outcome {
    let line = Path::line((1.0, 2.0), (4.0, 6.0))?;
    assert_commands(
        &line.slice(0.0, 0.5, None)?,
        &[('M', &[1.0, 2.0]), ('L', &[2.5, 4.0])],
        1e-9,
    )?;
    // 0.2 and 0.7 of the way along (3, 4): (1.6, 2.8) and (3.1, 4.8)
    assert_commands(
        &line.slice(0.2, 0.7, None)?,
        &[('M', &[1.6, 2.8]), ('L', &[3.1, 4.8])],
        1e-9,
    )?;
    Ok(())
}

#[test]
fn a_piece_of_a_rectangle_keeps_the_corners_it_passes() -> Outcome {
    // 95 round; 0.1 is 9.5 along the top (12.5, -4), 0.4 is 38: the corner (33, -4) and 8 down the right edge
    let rect = Path::rect(3.0, -4.0, 30.0, 17.5, 0.0)?;
    let piece = rect.slice(0.1, 0.4, None)?;
    assert_commands(
        &piece,
        &[('M', &[12.5, -4.0]), ('L', &[33.0, -4.0]), ('L', &[33.0, 4.0])],
        1e-9,
    )?;
    Ok(())
}

#[test]
fn a_piece_across_sub_paths_is_a_piece_of_each() -> Outcome {
    // 10 along y = 0, then 30 down x = 0 from (0, 5): 40 in all; 0.1 to 0.5 is 4 to 20
    let path = Path::line((0.0, 0.0), (10.0, 0.0))?
        .move_to((0.0, 5.0))?
        .line_to((0.0, 35.0))?;
    let piece = path.slice(0.1, 0.5, None)?;
    assert_commands(
        &piece,
        &[
            ('M', &[4.0, 0.0]),
            ('L', &[10.0, 0.0]),
            ('M', &[0.0, 5.0]),
            ('L', &[0.0, 15.0]),
        ],
        1e-9,
    )
}

#[test]
fn a_closed_sub_path_wholly_inside_the_piece_stays_closed() -> Outcome {
    // a line of 10, a square of 40 and a line of 10: 60 in all; 0.1 to 0.9 is 6 to 54
    let path = Path::line((0.0, 0.0), (10.0, 0.0))?
        .move_to((20.0, 20.0))?
        .line_to((30.0, 20.0))?
        .line_to((30.0, 30.0))?
        .line_to((20.0, 30.0))?
        .close()?
        .move_to((50.0, 0.0))?
        .line_to((50.0, 10.0))?;
    let piece = path.slice(0.1, 0.9, None)?;
    assert_commands(
        &piece,
        &[
            ('M', &[6.0, 0.0]),
            ('L', &[10.0, 0.0]),
            ('M', &[20.0, 20.0]),
            ('L', &[30.0, 20.0]),
            ('L', &[30.0, 30.0]),
            ('L', &[20.0, 30.0]),
            ('Z', &[]),
            ('M', &[50.0, 0.0]),
            ('L', &[50.0, 4.0]),
        ],
        1e-9,
    )
}

#[test]
fn the_whole_of_a_shape_is_the_shape_and_stays_closed() -> Outcome {
    for (what, shape) in [
        ("a rectangle", Path::rect(3.0, -4.0, 30.0, 17.5, 0.0)?),
        ("a rounded rectangle", Path::rect(3.0, -4.0, 30.0, 17.5, 6.0)?),
        ("a circle", Path::circle((20.0, -30.0), 75.0)?),
        ("a heptagon", Path::polygon((5.0, 6.0), 40.0, 7)?),
        ("a star", Path::star((5.0, 6.0), 40.0, 17.0, 5)?),
    ] {
        let whole = shape.slice(0.0, 1.0, None)?;
        assert_eq!(whole, shape, "{what}");
        assert!(whole.to_svg_path().ends_with('Z'), "{what} lost its closing");
    }
    Ok(())
}

#[test]
fn the_whole_of_an_open_path_is_the_path() -> Outcome {
    let path = Path::start((0.0, 0.0))?
        .quad_to((5.0, 10.0), (10.0, 0.0))?
        .curve_to((12.0, -3.0), (15.0, -3.0), (20.0, 2.0))?
        .move_to((30.0, 30.0))?
        .line_to((31.0, 33.0))?;
    assert_eq!(path.slice(0.0, 1.0, None)?, path);
    Ok(())
}

#[test]
fn a_closed_outline_not_written_from_its_top_is_cut_from_its_top() -> Outcome {
    // written from (50, 40); the walk starts at the topmost corner (30, 10)
    let outline = Path::polyline(
        [(50.0, 40.0), (90.0, 30.0), (60.0, 80.0), (10.0, 60.0), (30.0, 10.0)],
        true,
    )?;
    let whole = outline.slice(0.0, 1.0, None)?;
    assert_commands(
        &whole,
        &[
            ('M', &[30.0, 10.0]),
            ('L', &[50.0, 40.0]),
            ('L', &[90.0, 30.0]),
            ('L', &[60.0, 80.0]),
            ('L', &[10.0, 60.0]),
            ('Z', &[]),
        ],
        1e-9,
    )
}

#[test]
fn a_closed_outline_made_of_one_curve_is_cut_from_its_top_into_two_curves() -> Outcome {
    // a loop that starts at the origin; its topmost point is part-way round the curve, not at either end
    let loop_ = Path::from_svg_path("M0 0 C40 -60 80 60 0 0 Z")?;
    let mut top = f64::INFINITY;
    for step in 0..=4000 {
        top = top.min(loop_.point_at(f64::from(step) / 4000.0, None)?.y);
    }
    let whole = loop_.slice(0.0, 1.0, None)?;
    let letters: String = commands(&whole.to_svg_path())?
        .iter()
        .map(|(letter, _)| *letter)
        .collect();
    assert_eq!(letters, "MCCZ", "{}", whole.to_svg_path());
    let ((start_x, start_y), _) = start_and_end(&whole)?;
    assert!(
        agree(start_y, top, 0.02),
        "starts at y = {start_y}, the top is {top} (x = {start_x})"
    );
    assert!(
        agree(whole.length(), loop_.length(), 0.1),
        "{} against {}",
        whole.length(),
        loop_.length()
    );
    Ok(())
}

#[test]
fn rounding_at_the_end_of_a_sub_path_cannot_open_it_or_leave_a_stray_point() -> Outcome {
    // 0.3 + 0.4 is 0.7, but 0.7 - 0.3 is 0.39999999999999997: the square must still be reached whole, and closed
    let path = Path::line((0.0, 0.0), (0.3, 0.0))?
        .move_to((0.0, 0.0))?
        .line_to((0.1, 0.0))?
        .line_to((0.1, 0.1))?
        .line_to((0.0, 0.1))?
        .close()?;
    assert_eq!(path.slice(0.0, 1.0, None)?, path);
    // the end is a lone start, not a hair of the square
    assert_commands(&path.slice(1.0, 1.0, None)?, &[('M', &[0.0, 0.0])], 0.0)?;
    // positions within a few units in the last place of where the square starts: never a piece of no length
    let boundary: f64 = 0.3 / 0.7;
    for nudge in -8_i64..=8 {
        let start = f64::from_bits(boundary.to_bits().checked_add_signed(nudge).ok_or("no such position")?);
        let parts = path.slice(start, 1.0, None)?.split();
        if parts.len() > 1 {
            for part in &parts {
                assert!(part.length() > 0.0, "{nudge}: {}", part.to_svg_path());
            }
        }
    }
    Ok(())
}

#[test]
fn a_piece_that_is_not_the_whole_of_a_closed_outline_is_open() -> Outcome {
    let rect = Path::rect(3.0, -4.0, 30.0, 17.5, 0.0)?;
    for (start, end) in [(0.0, 0.5), (0.25, 1.0), (0.0, 0.999), (0.001, 1.0)] {
        let piece = rect.slice(start, end, None)?;
        assert!(
            !piece.to_svg_path().contains('Z'),
            "{start} to {end}: {}",
            piece.to_svg_path()
        );
    }
    Ok(())
}

#[test]
fn a_piece_that_starts_and_ends_at_one_position_is_a_lone_start_there() -> Outcome {
    let rect = Path::rect(3.0, -4.0, 30.0, 17.5, 0.0)?;
    // 0.3 is 28.5 along the top; 0.5 is 47.5, the bottom right corner
    for (at, expected) in [
        (0.3, (31.5, -4.0)),
        (0.0, (3.0, -4.0)),
        (1.0, (3.0, -4.0)),
        (0.5, (33.0, 13.5)),
    ] {
        let point = rect.slice(at, at, None)?;
        assert_commands(&point, &[('M', &[expected.0, expected.1])], 1e-9)?;
        assert!(point.length().abs() < f64::EPSILON);
        point.bounds()?;
    }
    let dot = Path::start((7.0, -2.0))?;
    assert_commands(&dot.slice(0.2, 0.9, None)?, &[('M', &[7.0, -2.0])], 0.0)
}

#[test]
fn a_piece_of_a_curve_is_a_curve_on_the_same_curve() -> Outcome {
    // the quadratic (0,0) (5,10) (10,0) is the parabola y = 2x - x^2/5
    let parabola = Path::start((0.0, 0.0))?.quad_to((5.0, 10.0), (10.0, 0.0))?;
    let piece = parabola.slice(0.25, 0.75, None)?;
    let letters: String = commands(&piece.to_svg_path())?
        .iter()
        .map(|(letter, _)| *letter)
        .collect();
    assert_eq!(letters, "MQ");
    for at in [0.0, 0.3, 0.5, 0.8, 1.0] {
        let point = piece.point_at(at, None)?;
        assert!(
            agree(point.y, 2.0 * point.x - point.x * point.x / 5.0, 1e-9),
            "at {at}: {point:?}"
        );
    }
    // half the length, to within the flattening
    assert!(
        agree(piece.length(), parabola.length() / 2.0, 0.05),
        "{} of {}",
        piece.length(),
        parabola.length()
    );
    Ok(())
}

#[test]
fn a_piece_of_a_curve_that_speeds_up_and_slows_down_has_its_share_of_the_length() -> Outcome {
    // x(t) = 100 (3 t^2 - 2 t^3) on a line: 100 pixels long, crawling at both ends
    let eased = Path::start((0.0, 0.0))?.curve_to((0.0, 0.0), (100.0, 0.0), (100.0, 0.0))?;
    for (start, end) in [(0.0, 0.25), (0.1, 0.3), (0.7, 1.0), (0.4, 0.45)] {
        let piece = eased.slice(start, end, None)?;
        let expected = 100.0 * (end - start);
        assert!(
            agree(piece.length(), expected, 0.05),
            "{start} to {end}: {} against {expected}",
            piece.length()
        );
        let ((from, _), (to, _)) = start_and_end(&piece)?;
        assert!(
            agree(from, 100.0 * start, 0.02) && agree(to, 100.0 * end, 0.02),
            "{start} to {end}: {from} to {to}"
        );
    }
    Ok(())
}

#[test]
fn a_piece_of_a_circle_is_arcs_that_end_on_the_circle() -> Outcome {
    let (centre, radius) = ((20.0, -30.0), 75.0);
    let circle = Path::circle(centre, radius)?;
    let piece = circle.slice(0.1, 0.35, None)?;
    // from the first arc into the second
    let letters: String = commands(&piece.to_svg_path())?
        .iter()
        .map(|(letter, _)| *letter)
        .collect();
    assert_eq!(letters, "MCC");
    let ((start_x, start_y), (end_x, end_y)) = start_and_end(&piece)?;
    for (end, angle) in [((start_x, start_y), 0.1), ((end_x, end_y), 0.35)] {
        let angle = 2.0 * std::f64::consts::PI * angle;
        let ideal = (
            centre.0 + radius * libm::sin(angle),
            centre.1 - radius * libm::cos(angle),
        );
        assert!(distance(end, ideal) <= 1e-3 * radius, "{end:?} against {ideal:?}");
    }
    Ok(())
}

#[test]
fn near_cuts_a_closed_outline_from_the_point_of_it_nearest() -> Outcome {
    // 95 round; near the middle of the right edge, 0.1 is 9.5 along: down 8.75, then left 0.75
    let rect = Path::rect(3.0, -4.0, 30.0, 17.5, 0.0)?;
    let near = Some(Point::new(40.0, 4.75));
    assert_commands(
        &rect.slice(0.0, 0.1, near)?,
        &[('M', &[33.0, 4.75]), ('L', &[33.0, 13.5]), ('L', &[32.25, 13.5])],
        1e-9,
    )?;
    // the whole of it is closed and starts there
    let whole = rect.slice(0.0, 1.0, near)?;
    assert_commands(
        &whole,
        &[
            ('M', &[33.0, 4.75]),
            ('L', &[33.0, 13.5]),
            ('L', &[3.0, 13.5]),
            ('L', &[3.0, -4.0]),
            ('L', &[33.0, -4.0]),
            ('Z', &[]),
        ],
        1e-9,
    )
}

#[test]
fn start_must_not_be_after_end() -> Outcome {
    let line = Path::line((1.0, 2.0), (4.0, 6.0))?;
    let (name, message) = parameter_refusal(line.slice(0.7, 0.2, None))?;
    assert_eq!(name, "start");
    assert!(message.contains("0.7") && message.contains("0.2"), "{message}");
    // nearly equal and equal positions are fine
    line.slice(0.2, 0.2, None)?;
    line.slice(0.2, 0.2 + f64::EPSILON, None)?;
    Ok(())
}

#[test]
fn positions_outside_0_to_1_are_refused_naming_which_one() -> Outcome {
    let line = Path::line((1.0, 2.0), (4.0, 6.0))?;
    let cases = [
        (-0.1, 0.5, "start", "-0.1"),
        (f64::NAN, 0.5, "start", "NaN"),
        (0.2, 1.5, "end", "1.5"),
        (0.2, f64::NAN, "end", "NaN"),
        (0.2, f64::INFINITY, "end", "inf"),
        (f64::NEG_INFINITY, 0.5, "start", "-inf"),
    ];
    for (start, end, expected, shown) in cases {
        let (name, message) = parameter_refusal(line.slice(start, end, None))?;
        assert_eq!(name, expected, "slice({start}, {end})");
        assert!(message.contains(shown), "{message}");
    }
    line.slice(0.0, 1.0, None)?;
    Ok(())
}

#[test]
fn slicing_refuses_an_empty_path_and_a_near_that_is_not_a_point() -> Outcome {
    let (verb, message) = geometry_refusal(Path::new().slice(0.2, 0.7, None))?;
    assert_eq!(verb, "slice");
    assert!(message.contains("empty"), "{message}");
    let line = Path::line((1.0, 2.0), (4.0, 6.0))?;
    let (name, message) = parameter_refusal(line.slice(0.2, 0.7, Some(Point::new(1.0, f64::NAN))))?;
    assert_eq!((name.as_str(), message.contains("NaN")), ("near", true), "{message}");
    Ok(())
}

proptest! {
    // The length of a piece is its share of the length, to within the flattening of the two curves it cuts.
    #[test]
    fn a_piece_has_its_share_of_the_length(
        path in sized_path(),
        ends in (0.0f64..=1.0, 0.0f64..=1.0),
    ) {
        let (start, end) = (ends.0.min(ends.1), ends.0.max(ends.1));
        let piece = path.slice(start, end, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let expected = (end - start) * path.length();
        prop_assert!(agree(piece.length(), expected, 0.1 + 1e-9 * path.length()), "{} against {}", piece.length(), expected);
    }

    // A piece starts where its start position is and ends where its end position is.
    #[test]
    fn a_piece_runs_from_its_start_position_to_its_end_position(
        path in sized_path(),
        start in 0.001f64..0.5,
        span in 0.001f64..0.499,
    ) {
        prop_assume!(path.length() > 0.0);
        let end = start + span;
        let piece = path.slice(start, end, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let size = path.bounds().map_or(1.0, |bounds| bounds.width.max(bounds.height)).max(1.0);
        let ((begin_x, begin_y), (finish_x, finish_y)) = start_and_end(&piece).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let from = path.point_at(start, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let to = path.point_at(end, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert!(distance((begin_x, begin_y), (from.x, from.y)) <= 1e-9 * size, "start {:?} against {:?}", (begin_x, begin_y), from);
        prop_assert!(distance((finish_x, finish_y), (to.x, to.y)) <= 1e-9 * size, "end {:?} against {:?}", (finish_x, finish_y), to);
    }

    // Two pieces side by side are as long as the piece made of both.
    #[test]
    fn pieces_side_by_side_add_up(path in sized_path(), cuts in (0.0f64..=1.0, 0.0f64..=1.0, 0.0f64..=1.0)) {
        let mut cuts = [cuts.0, cuts.1, cuts.2];
        cuts.sort_by(f64::total_cmp);
        let [start, middle, end] = cuts;
        let first = path.slice(start, middle, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let second = path.slice(middle, end, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let whole = path.slice(start, end, None).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert!(agree(first.length() + second.length(), whole.length(), 0.2 + 1e-9 * path.length()));
    }
}
