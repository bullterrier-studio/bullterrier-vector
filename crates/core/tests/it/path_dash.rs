//! `Path::dash` through the public API: the pieces a pattern of "on" and "off" lengths leaves. Expected paths are
//! written by hand from the definition (the pattern runs from the start of each sub-path; `phase` moves the dashes
//! forward). The properties compare the pieces with an independent count of the pattern's "on" length.

use crate::common::{Outcome, commands, distance, parameter_refusal};
use bullterrier_vector::{Error, Path, Point};
use proptest::prelude::*;

fn line(length: f64) -> Result<Path, Error> {
    Path::line((0.0, 0.0), (length, 0.0))
}

/// Whether `path` is the path data `expected`, the numbers equal to within rounding: a cut at a share of a length is
/// where it falls to the last digit, not always to the one written by hand.
fn is(path: &Path, expected: &str) -> Outcome {
    let (found, wanted) = (commands(&path.to_svg_path())?, commands(expected)?);
    let same = found.len() == wanted.len()
        && found.iter().zip(&wanted).all(|((a, xs), (b, ys))| {
            a == b && xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| (x - y).abs() <= 1e-9)
        });
    if same {
        Ok(())
    } else {
        Err(format!("{} is not {expected}", path.to_svg_path()).into())
    }
}

// By hand --------------------------------------------------------------------------------------------------------------

#[test]
fn a_line_is_cut_into_its_dashes_and_the_pattern_runs_to_the_end() -> Outcome {
    // on 4, off 6: dashes at 0, 10, 20; a dash that would start at the end is not there
    is(
        &line(30.0)?.dash(&[4.0, 6.0], 0.0, None)?,
        "M0 0 L4 0 M10 0 L14 0 M20 0 L24 0",
    )?;
    // a dash cut short by the end
    is(
        &line(22.0)?.dash(&[4.0, 6.0], 0.0, None)?,
        "M0 0 L4 0 M10 0 L14 0 M20 0 L22 0",
    )?;
    Ok(())
}

#[test]
fn the_phase_moves_the_dashes_forward_by_that_share_of_a_cycle() -> Outcome {
    let path = line(30.0)?;
    // a cycle is 10: a quarter is 2.5 further along, and the gap before the first dash is the end of the last one
    is(
        &path.dash(&[4.0, 6.0], 0.25, None)?,
        "M2.5 0 L6.5 0 M12.5 0 L16.5 0 M22.5 0 L26.5 0",
    )?;
    // three quarters: the first dash is cut by the start (it began 7.5 before it, 1.5 of it is left)
    is(
        &path.dash(&[4.0, 6.0], 0.75, None)?,
        "M0 0 L1.5 0 M7.5 0 L11.5 0 M17.5 0 L21.5 0 M27.5 0 L30 0",
    )?;
    Ok(())
}

#[test]
fn the_phase_wraps_so_a_whole_cycle_more_or_less_changes_nothing() -> Outcome {
    let path = line(30.0)?;
    let at_zero = path.dash(&[4.0, 6.0], 0.0, None)?;
    for phase in [1.0, 2.0, -1.0, -3.0] {
        assert_eq!(path.dash(&[4.0, 6.0], phase, None)?, at_zero, "{phase}");
    }
    assert_eq!(
        path.dash(&[4.0, 6.0], -0.25, None)?,
        path.dash(&[4.0, 6.0], 0.75, None)?
    );
    assert_eq!(path.dash(&[4.0, 6.0], 1.25, None)?, path.dash(&[4.0, 6.0], 0.25, None)?);
    Ok(())
}

#[test]
fn an_odd_pattern_repeats_itself_to_make_an_even_one() -> Outcome {
    let path = line(20.0)?;
    assert_eq!(path.dash(&[3.0], 0.0, None)?, path.dash(&[3.0, 3.0], 0.0, None)?);
    // [2, 1, 3] is [2, 1, 3, 2, 1, 3]: on 0-2, 3-6, 8-9, then again from 12: 12-14, 15-18
    is(
        &path.dash(&[2.0, 1.0, 3.0], 0.0, None)?,
        "M0 0 L2 0 M3 0 L6 0 M8 0 L9 0 M12 0 L14 0 M15 0 L18 0",
    )?;
    // the phase is in cycles of the whole even pattern (12 here): a quarter is 3: every dash is 3 further on
    is(
        &path.dash(&[2.0, 1.0, 3.0], 0.25, None)?,
        "M3 0 L5 0 M6 0 L9 0 M11 0 L12 0 M15 0 L17 0 M18 0 L20 0",
    )?;
    Ok(())
}

#[test]
fn a_dash_goes_round_a_corner_and_each_sub_path_starts_the_pattern_again() -> Outcome {
    let bend = Path::from_svg_path("M0 0 L10 0 L10 10")?;
    is(&bend.dash(&[14.0, 2.0], 0.0, None)?, "M0 0 L10 0 L10 4 M10 6 L10 10")?;
    let two = Path::from_svg_path("M0 0 L10 0 M0 5 L10 5")?;
    is(&two.dash(&[4.0, 10.0], 0.0, None)?, "M0 0 L4 0 M0 5 L4 5")?;
    Ok(())
}

#[test]
fn a_closed_path_is_measured_from_its_top_or_the_point_nearest_near() -> Outcome {
    let square = Path::from_svg_path("M0 0 L10 0 L10 10 L0 10 Z")?;
    // 40 round: on 15 from the top-left corner, off 5, on 15, off 5
    is(
        &square.dash(&[15.0, 5.0], 0.0, None)?,
        "M0 0 L10 0 L10 5 M10 10 L0 10 L0 5",
    )?;
    let from_corner = square.dash(&[15.0, 5.0], 0.0, Some(Point::new(10.0, 10.0)))?;
    is(&from_corner, "M10 10 L0 10 L0 5 M0 0 L10 0 L10 5")?;
    Ok(())
}

#[test]
fn a_closed_path_wholly_inside_one_dash_stays_closed_and_an_open_one_stays_open() -> Outcome {
    let square = Path::from_svg_path("M0 0 L10 0 L10 10 L0 10 Z")?;
    assert_eq!(
        square.dash(&[100.0, 1.0], 0.0, None)?.to_svg_path(),
        square.slice(0.0, 1.0, None)?.to_svg_path()
    );
    assert!(square.dash(&[100.0, 1.0], 0.0, None)?.to_svg_path().ends_with('Z'));
    // the first and last dash of a closed path are not joined, as the gap that would join them is not there
    is(&square.dash(&[30.0, 10.0], 0.5, None)?, "M0 0 L10 0 M10 10 L0 10 L0 0")?;
    assert!(!line(5.0)?.dash(&[100.0, 1.0], 0.0, None)?.to_svg_path().ends_with('Z'));
    Ok(())
}

#[test]
fn curves_stay_curves_cut_where_the_pattern_falls() -> Outcome {
    let arc = Path::from_svg_path("M0 0 C0 10 10 10 10 0")?;
    let length = arc.length();
    let pieces = arc.dash(&[length / 2.0, length / 4.0], 0.0, None)?;
    let data = pieces.to_svg_path();
    assert!(data.contains('C') && !data.contains('L'), "{data}");
    Ok(())
}

#[test]
fn a_point_stays_a_point_an_empty_path_stays_empty_and_a_path_wholly_in_a_gap_is_empty() -> Outcome {
    let point = Path::from_svg_path("M3 4 M5 6 L9 6")?;
    is(&point.dash(&[2.0, 100.0], 0.0, None)?, "M3 4 M5 6 L7 6")?;
    assert_eq!(Path::new().dash(&[2.0, 1.0], 0.0, None)?, Path::new());
    // 3 long, a gap from 1 to 101: a phase of a half moves the first dash 50.5 on
    assert_eq!(line(3.0)?.dash(&[1.0, 100.0], 0.5, None)?, Path::new());
    Ok(())
}

// Refusals ---------------------------------------------------------------------------------------------------------

#[test]
fn a_pattern_that_is_empty_or_has_a_length_not_above_zero_is_refused_by_name() -> Outcome {
    let path = line(10.0)?;
    let (name, _) = parameter_refusal(path.dash(&[], 0.0, None))?;
    assert_eq!(name, "pattern");
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY, 1.000_000_1e100] {
        let (name, _) = parameter_refusal(path.dash(&[2.0, bad], 0.0, None))?;
        assert_eq!(name, "pattern", "{bad}");
    }
    assert!(path.dash(&[1e100], 0.0, None).is_ok());
    // from 1 to 1000 lengths
    assert!(path.dash(&[1.0; 1000], 0.0, None).is_ok());
    let (name, _) = parameter_refusal(path.dash(&[1.0; 1001], 0.0, None))?;
    assert_eq!(name, "pattern");
    Ok(())
}

#[test]
fn a_phase_or_near_that_is_not_a_number_is_refused_by_name() -> Outcome {
    let path = line(10.0)?;
    for phase in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(parameter_refusal(path.dash(&[1.0], phase, None))?.0, "phase");
    }
    let near = Some(Point::new(f64::NAN, 0.0));
    assert_eq!(parameter_refusal(path.dash(&[1.0], 0.0, near))?.0, "near");
    Ok(())
}

#[test]
fn a_pattern_that_would_make_more_than_ten_million_pieces_is_refused_before_any_are_made() -> Outcome {
    let long = line(1.0e6)?;
    let (name, message) = parameter_refusal(long.dash(&[1.0e-3, 1.0e-3], 0.0, None))?;
    assert_eq!(name, "pattern");
    assert!(message.contains("MAX_SEGMENTS"), "{message}");
    // a tiny length beside a large one is fine: one piece a cycle
    assert!(long.dash(&[1.0e-300, 1.0e5], 0.0, None).is_ok());
    // the counts add up over sub-paths
    let many = Path::from_svg_path("M0 0 L1000000 0 M0 1 L1000000 1")?;
    assert!(parameter_refusal(many.dash(&[0.1, 0.1], 0.0, None)).is_ok());
    Ok(())
}

// Properties -------------------------------------------------------------------------------------------------------

#[expect(clippy::needless_pass_by_value, reason = "used as the function of map_err")]
fn fail(error: impl ToString) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

/// How much of `[0, upto]` of a pattern that starts with a dash at 0 is "on": whole cycles, then the part of one,
/// counted entry by entry. Written apart from the library, which walks the path instead.
fn on_within(lengths: &[f64], upto: f64) -> f64 {
    let even: Vec<f64> = if lengths.len() % 2 == 1 {
        lengths.iter().chain(lengths).copied().collect()
    } else {
        lengths.to_vec()
    };
    let cycle: f64 = even.iter().sum();
    let on_in_cycle: f64 = even.iter().step_by(2).sum();
    let whole = (upto / cycle).floor();
    let mut left = upto - whole * cycle;
    let mut on = whole * on_in_cycle;
    for (index, length) in even.iter().enumerate() {
        let used = left.min(*length);
        if index % 2 == 0 {
            on += used;
        }
        left -= used;
    }
    on
}

fn pattern() -> impl Strategy<Value = Vec<f64>> {
    prop::collection::vec(0.5f64..40.0, 1..6)
}

proptest! {
    // The pieces of an open polyline add up to the "on" length of the pattern over its length, no piece is longer
    // than the path, and every piece lies on the path.
    #[test]
    fn the_pieces_add_up_to_the_on_length_of_the_pattern_and_lie_on_the_path(
        points in prop::collection::vec((-100.0f64..100.0, -100.0f64..100.0), 2..8),
        lengths in pattern(),
        eighths in -16i32..16,
    ) {
        let path = Path::polyline(points, false).map_err(fail)?;
        let length = path.length();
        prop_assume!(length > 1.0);
        let phase = f64::from(eighths) / 8.0;
        let dashed = path.dash(&lengths, phase, None).map_err(fail)?;
        let cycle: f64 = lengths.iter().sum::<f64>() * if lengths.len() % 2 == 1 { 2.0 } else { 1.0 };
        let shift = (-phase).rem_euclid(1.0) * cycle;
        // the pattern seen from `shift` into it, over the length of the path
        let wanted = on_within(&lengths, shift + length) - on_within(&lengths, shift);
        let found: f64 = dashed.split().iter().map(Path::length).sum();
        prop_assert!((found - wanted).abs() <= 1e-9 * length + 1e-9, "{} against {}", found, wanted);
        prop_assert!(found <= length + 1e-9);
        for piece in dashed.split() {
            for at in [0.0, 0.5, 1.0] {
                let spot = piece.point_at(at, None).map_err(fail)?;
                let place = path.locate((spot.x, spot.y), None).map_err(fail)?;
                let on = path.point_at(place, None).map_err(fail)?;
                prop_assert!(distance((spot.x, spot.y), (on.x, on.y)) <= 1e-6, "{:?} off the path", spot);
            }
        }
    }

    // A pattern is the same pattern a whole cycle on.
    #[test]
    fn a_whole_cycle_of_phase_changes_nothing(
        points in prop::collection::vec((-100.0f64..100.0, -100.0f64..100.0), 2..6),
        lengths in pattern(),
        eighths in 0i32..8,
        cycles in 1i32..4,
    ) {
        let path = Path::polyline(points, false).map_err(fail)?;
        let phase = f64::from(eighths) / 8.0;
        let again = phase + f64::from(cycles);
        prop_assert_eq!(
            path.dash(&lengths, phase, None).map_err(fail)?,
            path.dash(&lengths, again, None).map_err(fail)?
        );
    }
}
