//! `Path::wave` through the public API: points displaced sideways by a sine travelling towards `angle`. The oracle is
//! the closed form written out here from the definition (a point at distance `d` along the direction of travel moves
//! `amplitude * sin(2 pi (d / wavelength - phase))` along the perpendicular, `y` down), checked against the output
//! polyline by brute force; the expected vertices are computed by hand.

use crate::common::{Outcome, commands, parameter_refusal};
use bullterrier_vector::Path;
use proptest::prelude::*;
use std::f64::consts::TAU;

/// One sub-path of a result: its vertices and whether it is closed.
type Line = (Vec<(f64, f64)>, bool);

/// The sub-paths of a path made of straight lines only.
fn lines(path: &Path) -> Result<Vec<Line>, Box<dyn std::error::Error>> {
    let mut found: Vec<Line> = Vec::new();
    for (letter, numbers) in commands(&path.to_svg_path())? {
        match (letter, numbers.as_slice()) {
            ('M', [x, y]) => found.push((vec![(*x, *y)], false)),
            ('L', [x, y]) => found.last_mut().ok_or("a line before a move")?.0.push((*x, *y)),
            ('Z', []) => found.last_mut().ok_or("a close before a move")?.1 = true,
            _ => return Err(format!("{letter} {numbers:?} is not a straight segment").into()),
        }
    }
    Ok(found)
}

/// Where the closed form sends `(x, y)`.
fn exact(point: (f64, f64), amplitude: f64, wavelength: f64, angle: f64, phase: f64) -> (f64, f64) {
    let a = angle.to_radians();
    let d = point.0 * libm::cos(a) + point.1 * libm::sin(a);
    let s = amplitude * libm::sin(TAU * (d / wavelength - phase));
    (point.0 - s * libm::sin(a), point.1 + s * libm::cos(a))
}

/// The distance from `p` to the segment `a`-`b`.
fn to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length_squared = dx * dx + dy * dy;
    let share = if length_squared == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length_squared).clamp(0.0, 1.0)
    };
    libm::hypot(p.0 - (a.0 + dx * share), p.1 - (a.1 + dy * share))
}

/// The distance from `p` to the nearest edge of any sub-path.
fn to_lines(p: (f64, f64), found: &[Line]) -> f64 {
    let mut nearest = f64::INFINITY;
    for (points, closed) in found {
        for pair in points.windows(2) {
            nearest = nearest.min(to_segment(p, pair[0], pair[1]));
        }
        if *closed && points.len() > 1 {
            nearest = nearest.min(to_segment(p, points[points.len() - 1], points[0]));
        }
    }
    nearest
}

/// Whether `found` has a vertex within 1e-9 of `p`.
fn has_vertex(found: &[Line], p: (f64, f64)) -> bool {
    found
        .iter()
        .flat_map(|(points, _)| points)
        .any(|v| (v.0 - p.0).abs() < 1e-9 && (v.1 - p.1).abs() < 1e-9)
}

/// Every exact image of a dense sampling of `path` lies within a hundredth of the amplitude (and a hundredth of a
/// pixel for the flattening) of the waved outline. The step is a 24th of the wavelength, so a chord strays from the
/// sine by at most `amplitude * (2 pi / 24)^2 / 8` = 0.86% of the amplitude.
fn assert_waved_matches_the_closed_form(
    path: &Path,
    amplitude: f64,
    wavelength: f64,
    angle: f64,
    phase: f64,
) -> Outcome {
    let waved = lines(&path.wave(amplitude, wavelength, angle, phase)?)?;
    for index in 0..=400 {
        let original = path.point_at(f64::from(index) / 400.0, None)?;
        let image = exact((original.x, original.y), amplitude, wavelength, angle, phase);
        let off = to_lines(image, &waved);
        assert!(
            off <= 0.01 * amplitude.abs() + 0.01,
            "sample {index}: the exact image {image:?} is {off} from the output"
        );
    }
    Ok(())
}

// By hand --------------------------------------------------------------------------------------------------------------

#[test]
fn a_line_is_a_sine_by_hand() -> Outcome {
    // 120 long, one wavelength: pieces of 120 / 24 = 5, so 25 vertices; x = 30 is a quarter wave, y = 8 sin(pi/2)
    let line = Path::line((0.0, 0.0), (120.0, 0.0))?;
    let found = lines(&line.wave(8.0, 120.0, 0.0, 0.0)?)?;
    assert_eq!(found.len(), 1);
    let (points, closed) = &found[0];
    assert!(!closed);
    assert_eq!(points.len(), 25, "{points:?}");
    for (at, y) in [(0.0, 0.0), (30.0, 8.0), (60.0, 0.0), (90.0, -8.0), (120.0, 0.0)] {
        let vertex = points
            .iter()
            .find(|v| (v.0 - at).abs() < 1e-9)
            .ok_or("no vertex there")?;
        assert!((vertex.1 - y).abs() < 1e-9, "x {at}: y is {}, not {y}", vertex.1);
    }
    Ok(())
}

#[test]
fn a_vertical_line_waved_at_90_degrees_moves_along_x() -> Outcome {
    // travelling down (+y): the perpendicular is -x for angle 90, so the quarter wave at y = 30 moves x by -8
    let line = Path::line((0.0, 0.0), (0.0, 120.0))?;
    let found = lines(&line.wave(8.0, 120.0, 90.0, 0.0)?)?;
    let vertex = found[0]
        .0
        .iter()
        .find(|v| (v.1 - 30.0).abs() < 1e-9)
        .ok_or("no vertex at 30")?;
    assert!((vertex.0 + 8.0).abs() < 1e-9, "{vertex:?}");
    Ok(())
}

#[test]
fn a_horizontal_line_waved_at_90_degrees_is_shifted_along_x_whole() -> Outcome {
    // the distance along the direction of travel is 0 everywhere on the line: phase 0.25 puts it at sin = -1, and the
    // sideways direction is -x, so every vertex moves by +8 in x and not at all in y
    let line = Path::line((0.0, 0.0), (120.0, 0.0))?;
    let found = lines(&line.wave(8.0, 120.0, 90.0, 0.25)?)?;
    let points = &found[0].0;
    assert_eq!(points.len(), 25);
    for (index, point) in (0_i32..).zip(points) {
        let x = 5.0 * f64::from(index);
        assert!(
            (point.0 - (x + 8.0)).abs() < 1e-9 && point.1.abs() < 1e-9,
            "{index}: {point:?}"
        );
    }
    Ok(())
}

#[test]
fn an_edge_shorter_than_the_step_is_one_piece() -> Outcome {
    let line = Path::line((0.0, 0.0), (1.0, 0.0))?;
    assert_eq!(lines(&line.wave(8.0, 120.0, 0.0, 0.0)?)?[0].0.len(), 2);
    Ok(())
}

#[test]
fn the_corners_of_a_rectangle_are_vertices_of_its_waved_outline() -> Outcome {
    let rect = Path::rect(10.0, 20.0, 100.0, 60.0, 0.0)?;
    let waved = lines(&rect.wave(8.0, 70.0, 30.0, 0.3)?)?;
    for corner in [(10.0, 20.0), (110.0, 20.0), (110.0, 80.0), (10.0, 80.0)] {
        let image = exact(corner, 8.0, 70.0, 30.0, 0.3);
        assert!(
            has_vertex(&waved, image),
            "no vertex at the image {image:?} of {corner:?}"
        );
    }
    Ok(())
}

#[test]
fn a_closed_path_stays_closed_and_an_open_one_open() -> Outcome {
    let waved = lines(&Path::rect(0.0, 0.0, 50.0, 50.0, 0.0)?.wave(8.0, 40.0, 0.0, 0.0)?)?;
    assert!(waved.iter().all(|line| line.1));
    let waved = lines(&Path::from_svg_path("M0 0 L50 0 L50 50")?.wave(8.0, 40.0, 0.0, 0.0)?)?;
    assert!(waved.iter().all(|line| !line.1));
    Ok(())
}

#[test]
fn a_lone_point_is_kept_and_moved_and_an_empty_path_stays_empty() -> Outcome {
    // x = 30 at a quarter wave is 8 down
    assert_eq!(
        Path::from_svg_path("M30 0")?.wave(8.0, 120.0, 0.0, 0.0)?.to_svg_path(),
        "M30 8"
    );
    assert_eq!(Path::new().wave(8.0, 120.0, 0.0, 0.0)?, Path::new());
    Ok(())
}

// The closed form --------------------------------------------------------------------------------------------------------

#[test]
fn a_circle_and_a_slanted_line_match_the_closed_form() -> Outcome {
    assert_waved_matches_the_closed_form(&Path::circle((60.0, 60.0), 40.0)?, 8.0, 50.0, 30.0, 0.2)?;
    assert_waved_matches_the_closed_form(&Path::line((3.0, 4.0), (200.0, 90.0))?, 8.0, 37.0, 200.0, 0.7)?;
    assert_waved_matches_the_closed_form(&Path::line((0.0, 0.0), (300.0, 0.0))?, -12.0, 60.0, 0.0, 0.0)?;
    Ok(())
}

proptest! {
    /// A rectangle anywhere, waved any way: the exact images stay near the output and its corners are vertices.
    #[test]
    fn a_waved_rectangle_matches_the_closed_form(
        (x, y) in (-200.0..200.0_f64, -200.0..200.0_f64),
        (width, height) in (1.0..300.0_f64, 1.0..300.0_f64),
        amplitude in -20.0..20.0_f64,
        wavelength in 10.0..200.0_f64,
        angle in -360.0..360.0_f64,
        phase in -2.0..2.0_f64,
    ) {
        let rect = Path::rect(x, y, width, height, 0.0).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let check = assert_waved_matches_the_closed_form(&rect, amplitude, wavelength, angle, phase);
        prop_assert!(check.is_ok(), "{check:?}");
        let waved = lines(&rect.wave(amplitude, wavelength, angle, phase).map_err(|e| TestCaseError::fail(e.to_string()))?)
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        for corner in [(x, y), (x + width, y), (x + width, y + height), (x, y + height)] {
            let image = exact(corner, amplitude, wavelength, angle, phase);
            prop_assert!(has_vertex(&waved, image), "no vertex at {image:?}");
        }
    }
}

// Amplitude, phase -----------------------------------------------------------------------------------------------------

#[test]
fn an_amplitude_of_zero_leaves_the_outline_whole() -> Outcome {
    let rect = Path::rect(10.0, 20.0, 100.0, 60.0, 0.0)?;
    let flat = rect.wave(0.0, 70.0, 30.0, 0.3)?;
    // the cut points are interpolated, so the bounds agree to rounding
    let (flat_bounds, bounds) = (flat.bounds()?, rect.bounds()?);
    for (a, b) in [
        (flat_bounds.x, bounds.x),
        (flat_bounds.y, bounds.y),
        (flat_bounds.width, bounds.width),
        (flat_bounds.height, bounds.height),
    ] {
        assert!((a - b).abs() < 1e-9, "{flat_bounds:?} against {bounds:?}");
    }
    let ring = &lines(&flat)?[0].0;
    // by hand: 100 x 60
    let doubled: f64 = ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
        .sum();
    assert!((doubled / 2.0 - 6000.0).abs() < 1e-6, "{doubled}");
    Ok(())
}

#[test]
fn a_phase_a_whole_cycle_on_is_the_same_output_to_the_bit() -> Outcome {
    let path = Path::circle((60.0, 60.0), 40.0)?;
    let base = path.wave(8.0, 50.0, 30.0, 0.25)?;
    assert_eq!(path.wave(8.0, 50.0, 30.0, 1.25)?, base);
    assert_eq!(path.wave(8.0, 50.0, 30.0, -0.75)?, base);
    assert_eq!(path.wave(8.0, 50.0, 30.0, 17.25)?, base);
    Ok(())
}

#[test]
fn half_a_cycle_of_phase_is_the_negated_amplitude() -> Outcome {
    let path = Path::circle((60.0, 60.0), 40.0)?;
    let shifted = lines(&path.wave(8.0, 50.0, 30.0, 0.5)?)?;
    let negated = lines(&path.wave(-8.0, 50.0, 30.0, 0.0)?)?;
    assert_eq!(shifted[0].0.len(), negated[0].0.len());
    for (a, b) in shifted[0].0.iter().zip(&negated[0].0) {
        assert!((a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9, "{a:?} {b:?}");
    }
    Ok(())
}

// Refusals ---------------------------------------------------------------------------------------------------------------

#[test]
fn a_number_the_verb_cannot_use_is_refused_by_name() -> Outcome {
    let line = Path::line((0.0, 0.0), (10.0, 0.0))?;
    for wavelength in [0.0, -1.0, f64::NAN, f64::INFINITY, 1.000_000_1e100] {
        assert_eq!(
            parameter_refusal(line.wave(8.0, wavelength, 0.0, 0.0))?.0,
            "wavelength",
            "{wavelength}"
        );
    }
    assert!(line.wave(8.0, 1e100, 0.0, 0.0).is_ok());
    assert_eq!(parameter_refusal(line.wave(f64::NAN, 120.0, 0.0, 0.0))?.0, "amplitude");
    assert_eq!(
        parameter_refusal(line.wave(f64::INFINITY, 120.0, 0.0, 0.0))?.0,
        "amplitude"
    );
    assert_eq!(parameter_refusal(line.wave(8.0, 120.0, f64::NAN, 0.0))?.0, "angle");
    assert_eq!(parameter_refusal(line.wave(8.0, 120.0, f64::INFINITY, 0.0))?.0, "angle");
    assert_eq!(parameter_refusal(line.wave(8.0, 120.0, 0.0, f64::NAN))?.0, "phase");
    assert_eq!(parameter_refusal(line.wave(8.0, 120.0, 0.0, f64::INFINITY))?.0, "phase");
    // a result beyond the coordinate limit
    assert_eq!(parameter_refusal(line.wave(2e100, 120.0, 0.0, 0.25))?.0, "amplitude");
    Ok(())
}

#[test]
fn a_wavelength_that_would_make_more_than_ten_million_pieces_is_refused_at_once() -> Outcome {
    let line = Path::line((0.0, 0.0), (1000.0, 0.0))?;
    let (name, message) = parameter_refusal(line.wave(8.0, 1e-9, 0.0, 0.0))?;
    assert_eq!(name, "wavelength");
    assert!(message.contains("MAX_SEGMENTS"), "{message}");
    // a smallest wavelength too: the step underflows and the count is still refused, not hung on
    assert_eq!(parameter_refusal(line.wave(8.0, 5e-324, 0.0, 0.0))?.0, "wavelength");
    // the counts of all the sub-paths add up
    let many = Path::from_svg_path("M0 0 L1000000 0 M0 1 L1000000 1")?;
    assert_eq!(parameter_refusal(many.wave(8.0, 4.0, 0.0, 0.0))?.0, "wavelength");
    Ok(())
}
