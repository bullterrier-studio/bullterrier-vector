//! `Path::wobble` through the public API: points displaced by smooth seeded noise. The noise itself is not copied here;
//! its promises are tested as properties of the displacement, read from lone points (a lone point is kept and moved
//! by itself, so a sub-path per point probes the field): bounded by the amplitude, smooth to the constants the
//! interpolant allows (derived below), periodic in the phase, the same for the same seed. The cuts are checked against
//! a finely resampled copy of the same path, which is the curve the chords stand for.

use crate::common::{Outcome, has_vertex, lines, parameter_refusal, to_lines};
use bullterrier_vector::Path;
use proptest::prelude::*;

// The smoothness constants. The noise is a lattice of hashed values in [-1, 1], one cell a wavelength (and a quarter
// cycle of phase) wide, blended by the quintic fade s(u) = 6u^5 - 15u^4 + 10u^3, whose slope is at most 15/8 and whose
// second derivative at most 10 sqrt(3) / 3 = 5.77 (at u = 0.2113). A value of the blend differs between two ends of a cell by at most 2.
//   slope along one axis:                 2 * 15/8 = 3.75 per cell
//   second derivative along one axis:     2 * 5.77 = 11.5 per cell^2
//   mixed second derivative (x and y):    4 * (15/8)^2 = 14.1 per cell^2
// A displacement has two fields, so its length is at most sqrt(2) times one field's change.
/// The most a displacement changes, in amplitudes, over a distance of one wavelength.
const SPACE: f64 = 3.75 * 2.0; // 2 sqrt(2) * 3.75 = 10.6 along a diagonal, 7.5 per axis step: 3.75 * (|dx|+|dy|) * sqrt 2
/// The most it changes over a whole cycle of phase (four cells of it).
const PHASE: f64 = 3.75 * 4.0 * 1.5;
/// Its second difference along a straight line over a step `ds`, in amplitudes times `ds^2 / wavelength^2`:
/// (11.5 + 14.1) * sqrt 2 = 36.2.
const SECOND: f64 = 36.3;

/// How far each point moves.
fn shifts(
    points: &[(f64, f64)],
    amplitude: f64,
    wavelength: f64,
    phase: f64,
    seed: f64,
) -> Result<Vec<(f64, f64)>, Box<dyn std::error::Error>> {
    let data = points
        .iter()
        .map(|p| format!("M{} {}", p.0, p.1))
        .collect::<Vec<_>>()
        .join(" ");
    let moved = lines(&Path::from_svg_path(&data)?.wobble(amplitude, wavelength, phase, seed)?)?;
    assert_eq!(moved.len(), points.len(), "a lone point stays one sub-path");
    Ok(moved
        .iter()
        .zip(points)
        .map(|((found, _), p)| (found[0].0 - p.0, found[0].1 - p.1))
        .collect())
}

fn grid(step: f64, count: u32) -> Vec<(f64, f64)> {
    (0..count)
        .flat_map(|i| (0..count).map(move |j| (f64::from(i) * step, f64::from(j) * step)))
        .collect()
}

fn length(shift: (f64, f64)) -> f64 {
    libm::hypot(shift.0, shift.1)
}

// Bounds, statistics ---------------------------------------------------------------------------------------------------

#[test]
fn each_component_stays_within_the_amplitude_and_reaches_it() -> Outcome {
    let mut biggest = 0.0_f64;
    for (phase, seed) in [(0.0, 0.0), (0.37, 5.0), (0.9, -3.0)] {
        for (dx, dy) in shifts(&grid(7.3, 60), 5.0, 80.0, phase, seed)? {
            assert!(dx.abs() <= 5.0 + 1e-9 && dy.abs() <= 5.0 + 1e-9, "{dx} {dy}");
            assert!(length((dx, dy)) <= 5.0 * 2.0_f64.sqrt() + 1e-9);
            biggest = biggest.max(dx.abs()).max(dy.abs());
        }
    }
    assert!(
        biggest > 0.8 * 5.0,
        "the field never gets near its amplitude: {biggest}"
    );
    Ok(())
}

#[test]
fn along_a_long_line_the_noise_has_a_zero_mean_and_a_size_of_the_amplitude() -> Outcome {
    let line: Vec<(f64, f64)> = (0..2000).map(|x| (f64::from(x), 0.0)).collect();
    let shifted = shifts(&line, 10.0, 80.0, 0.0, 3.0)?;
    for component in [0, 1] {
        let values: Vec<f64> = shifted.iter().map(|s| if component == 0 { s.0 } else { s.1 }).collect();
        let count = f64::from(2000_u16);
        let mean = values.iter().sum::<f64>() / count;
        let rms = (values.iter().map(|v| v * v).sum::<f64>() / count).sqrt();
        assert!(mean.abs() < 3.0, "component {component}: mean {mean}");
        assert!((2.0..=10.0).contains(&rms), "component {component}: rms {rms}");
    }
    Ok(())
}

// Seed, phase ------------------------------------------------------------------------------------------------------------

#[test]
fn the_same_seed_and_phase_give_the_same_bytes_and_another_seed_another_wobble() -> Outcome {
    let path = Path::circle((60.0, 60.0), 40.0)?;
    let base = path.wobble(4.0, 30.0, 0.3, 7.0)?;
    assert_eq!(path.wobble(4.0, 30.0, 0.3, 7.0)?.to_svg_path(), base.to_svg_path());
    assert_ne!(path.wobble(4.0, 30.0, 0.3, 8.0)?.to_svg_path(), base.to_svg_path());
    assert_ne!(path.wobble(4.0, 30.0, 0.3, -7.0)?.to_svg_path(), base.to_svg_path());
    // the two fields of one seed are not the same field
    let shifted = shifts(&grid(7.3, 20), 5.0, 80.0, 0.0, 1.0)?;
    assert!(
        shifted.iter().any(|s| (s.0 - s.1).abs() > 0.5),
        "dx and dy are one field"
    );
    Ok(())
}

#[test]
fn a_whole_cycle_of_phase_is_the_same_output_to_the_bit() -> Outcome {
    let path = Path::circle((60.0, 60.0), 40.0)?;
    for phase in [0.25, 0.5, 0.0] {
        let base = path.wobble(4.0, 30.0, phase, 2.0)?.to_svg_path();
        assert_eq!(path.wobble(4.0, 30.0, phase + 1.0, 2.0)?.to_svg_path(), base, "{phase}");
        assert_eq!(path.wobble(4.0, 30.0, phase - 1.0, 2.0)?.to_svg_path(), base, "{phase}");
    }
    Ok(())
}

#[test]
fn the_phase_moves_the_noise_smoothly_and_round_the_seam() -> Outcome {
    let points = grid(9.1, 20);
    let (amplitude, wavelength) = (6.0, 50.0);
    for phase in [0.0, 0.2499, 0.5, 0.9] {
        let (a, b) = (
            shifts(&points, amplitude, wavelength, phase, 4.0)?,
            shifts(&points, amplitude, wavelength, phase + 1e-3, 4.0)?,
        );
        for (a, b) in a.iter().zip(&b) {
            assert!(
                length((a.0 - b.0, a.1 - b.1)) <= PHASE * amplitude * 1e-3 + 1e-9,
                "{phase}: {a:?} {b:?}"
            );
        }
    }
    // across the seam: 1 - 1e-4 and 1e-4 are 2e-4 of a cycle apart
    let (a, b) = (
        shifts(&points, amplitude, wavelength, 1.0 - 1e-4, 4.0)?,
        shifts(&points, amplitude, wavelength, 1e-4, 4.0)?,
    );
    for (a, b) in a.iter().zip(&b) {
        assert!(length((a.0 - b.0, a.1 - b.1)) <= PHASE * amplitude * 2e-4 + 1e-9);
    }
    // and the phase does move it: a quarter cycle on is another field
    let moved = shifts(&points, amplitude, wavelength, 0.25, 4.0)?;
    let first = shifts(&points, amplitude, wavelength, 0.0, 4.0)?;
    assert!(
        first
            .iter()
            .zip(&moved)
            .any(|(a, b)| length((a.0 - b.0, a.1 - b.1)) > 1.0)
    );
    Ok(())
}

proptest! {
    /// Two points a distance `d` apart are moved by amounts that differ by at most `SPACE * amplitude * d / wavelength`.
    #[test]
    fn nearby_points_move_alike(
        (x, y) in (-500.0..500.0_f64, -500.0..500.0_f64),
        (dx, dy) in (-20.0..20.0_f64, -20.0..20.0_f64),
        amplitude in 0.0..20.0_f64,
        wavelength in 10.0..200.0_f64,
        phase in -2.0..2.0_f64,
        seed in -1000_i32..1000,
    ) {
        let found = shifts(&[(x, y), (x + dx, y + dy)], amplitude, wavelength, phase, f64::from(seed))
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        let apart = (found[0].0 - found[1].0, found[0].1 - found[1].1);
        prop_assert!(
            length(apart) <= SPACE * amplitude * libm::hypot(dx, dy) / wavelength + 1e-9,
            "{found:?}"
        );
    }

    /// Along a straight line the displacement has no spikes: its second difference is bounded.
    #[test]
    fn along_a_line_the_second_difference_is_bounded(
        (x, y) in (-300.0..300.0_f64, -300.0..300.0_f64),
        angle in 0.0..360.0_f64,
        amplitude in 0.0..30.0_f64,
        wavelength in 20.0..200.0_f64,
        phase in 0.0..1.0_f64,
        seed in -1000_i32..1000,
    ) {
        let (sin, cos) = (libm::sin(angle.to_radians()), libm::cos(angle.to_radians()));
        let points: Vec<(f64, f64)> = (0..80).map(|i| (x + f64::from(i) * cos, y + f64::from(i) * sin)).collect();
        let found = shifts(&points, amplitude, wavelength, phase, f64::from(seed))
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        for window in found.windows(3) {
            let second = (window[0].0 - 2.0 * window[1].0 + window[2].0, window[0].1 - 2.0 * window[1].1 + window[2].1);
            // the points themselves are a straight line, so they add nothing; rounding of the subtractions does
            prop_assert!(length(second) <= SECOND * amplitude / (wavelength * wavelength) + 1e-9, "{second:?}");
        }
    }
}

// Cutting, corners -------------------------------------------------------------------------------------------------------

#[test]
fn the_cut_edges_stay_within_a_hundredth_of_the_amplitude_of_the_wobbled_curve() -> Outcome {
    // the finely resampled line is the curve: each of its wobbled points must lie on the cut line
    let (amplitude, wavelength) = (12.0, 60.0);
    for (from, to) in [((0.0, 0.0), (400.0, 0.0)), ((13.0, 7.0), (250.0, 310.0))] {
        let line = Path::line(from, to)?;
        let cut = lines(&line.wobble(amplitude, wavelength, 0.3, 9.0)?)?;
        let fine = lines(
            &line
                .resample(wavelength / 400.0)?
                .wobble(amplitude, wavelength, 0.3, 9.0)?,
        )?;
        assert!(
            fine[0].0.len() > 5 * cut[0].0.len(),
            "{} {}",
            fine[0].0.len(),
            cut[0].0.len()
        );
        for point in &fine[0].0 {
            let off = to_lines(*point, &cut);
            assert!(off <= 0.01 * amplitude + 0.01, "{point:?} is {off} from the cut line");
        }
    }
    Ok(())
}

#[test]
fn the_corners_of_a_rectangle_are_vertices_of_its_wobbled_outline() -> Outcome {
    let rect = Path::rect(10.0, 20.0, 100.0, 60.0, 0.0)?;
    let wobbled = lines(&rect.wobble(5.0, 40.0, 0.3, 2.0)?)?;
    let corners = [(10.0, 20.0), (110.0, 20.0), (110.0, 80.0), (10.0, 80.0)];
    for (corner, shift) in corners.iter().zip(shifts(&corners, 5.0, 40.0, 0.3, 2.0)?) {
        let image = (corner.0 + shift.0, corner.1 + shift.1);
        assert!(
            has_vertex(&wobbled, image),
            "no vertex at the image {image:?} of {corner:?}"
        );
    }
    Ok(())
}

#[test]
fn closed_stays_closed_and_open_open_and_an_amplitude_of_zero_keeps_the_outline() -> Outcome {
    let wobbled = lines(&Path::rect(0.0, 0.0, 50.0, 50.0, 0.0)?.wobble(4.0, 40.0, 0.0, 0.0)?)?;
    assert!(wobbled.iter().all(|line| line.1));
    let wobbled = lines(&Path::from_svg_path("M0 0 L50 0 L50 50")?.wobble(4.0, 40.0, 0.0, 0.0)?)?;
    assert!(wobbled.iter().all(|line| !line.1));
    let flat = Path::rect(10.0, 20.0, 100.0, 60.0, 0.0)?.wobble(0.0, 70.0, 0.3, 4.0)?;
    let ring = &lines(&flat)?[0].0;
    // by hand: 100 x 60
    let doubled: f64 = ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
        .sum();
    assert!((doubled / 2.0 - 6000.0).abs() < 1e-6, "{doubled}");
    assert_eq!(Path::new().wobble(4.0, 80.0, 0.0, 0.0)?, Path::new());
    Ok(())
}

// Refusals ---------------------------------------------------------------------------------------------------------------

#[test]
fn a_number_the_verb_cannot_use_is_refused_by_name() -> Outcome {
    let line = Path::line((0.0, 0.0), (10.0, 0.0))?;
    for wavelength in [0.0, -1.0, f64::NAN, f64::INFINITY, 1.000_000_1e100] {
        assert_eq!(
            parameter_refusal(line.wobble(4.0, wavelength, 0.0, 0.0))?.0,
            "wavelength",
            "{wavelength}"
        );
    }
    assert!(line.wobble(4.0, 1e100, 0.0, 0.0).is_ok());
    for amplitude in [f64::NAN, f64::INFINITY] {
        assert_eq!(
            parameter_refusal(line.wobble(amplitude, 80.0, 0.0, 0.0))?.0,
            "amplitude"
        );
    }
    for phase in [f64::NAN, f64::INFINITY] {
        assert_eq!(parameter_refusal(line.wobble(4.0, 80.0, phase, 0.0))?.0, "phase");
    }
    // a seed is a whole number within +-(2^53 - 1), as for `random`
    for seed in [0.5, f64::NAN, f64::INFINITY, 9_007_199_254_740_992.0] {
        assert_eq!(
            parameter_refusal(line.wobble(4.0, 80.0, 0.0, seed))?.0,
            "seed",
            "{seed}"
        );
    }
    assert!(line.wobble(4.0, 80.0, 0.0, 9_007_199_254_740_991.0).is_ok());
    assert!(line.wobble(4.0, 80.0, 0.0, -9_007_199_254_740_991.0).is_ok());
    // a displacement beyond the coordinate limit
    assert_eq!(parameter_refusal(line.wobble(1e101, 80.0, 0.25, 0.0))?.0, "amplitude");
    Ok(())
}

#[test]
fn a_wavelength_that_would_make_more_than_ten_million_pieces_is_refused_at_once() -> Outcome {
    let line = Path::line((0.0, 0.0), (1000.0, 0.0))?;
    let (name, message) = parameter_refusal(line.wobble(4.0, 1e-9, 0.0, 0.0))?;
    assert_eq!(name, "wavelength");
    assert!(message.contains("MAX_SEGMENTS"), "{message}");
    assert_eq!(parameter_refusal(line.wobble(4.0, 5e-324, 0.0, 0.0))?.0, "wavelength");
    Ok(())
}

#[test]
fn a_big_wobble_is_within_a_twentieth_of_a_pixel_of_the_curve() -> Outcome {
    let (amplitude, wavelength) = (100.0, 400.0);
    let line = Path::line((0.0, 0.0), (2000.0, 600.0))?;
    let cut = lines(&line.wobble(amplitude, wavelength, 0.3, 9.0)?)?;
    let fine = lines(&line.resample(1.0)?.wobble(amplitude, wavelength, 0.3, 9.0)?)?;
    let worst = fine[0]
        .0
        .iter()
        .map(|point| to_lines(*point, &cut))
        .fold(0.0_f64, f64::max);
    assert!(worst <= 0.05, "the curve is {worst} pixels from the output");
    Ok(())
}

#[test]
fn a_big_wobble_is_cut_by_the_sound_chord_bound_of_the_noise() -> Outcome {
    // by hand: the second derivative of one noise field along a line is at most 2 * 5.77 + 4 * (15/8)^2 = 25.6
    // amplitudes per cell squared, a displacement (two fields) at most sqrt 2 times that, so a chord over a step h
    // strays by at most 36.2 / 8 * (h / wavelength)^2 = 0.79% of the amplitude at a 24th of the wavelength. At
    // amplitude 100 that is 0.79 px, over the twentieth-of-a-pixel budget: the step is shortened by sqrt(0.05 / 0.79).
    let (amplitude, wavelength, length) = (100.0, 80.0, 1000.0);
    let cut = lines(&Path::line((0.0, 0.0), (length, 0.0))?.wobble(amplitude, wavelength, 0.0, 0.0)?)?;
    let step = wavelength / 24.0 * (0.05 / (0.0079 * amplitude)).sqrt();
    let points = (length / step).ceil() + 1.0;
    let found = f64::from(u32::try_from(cut[0].0.len())?);
    assert!((found - points).abs() < 0.5, "{found} points, expected {points}");
    Ok(())
}

#[test]
fn a_chord_of_a_small_wobble_strays_under_four_thousandths_of_the_amplitude() -> Outcome {
    // 6 px is under the pixel budget (0.79% of it is 0.047 px), so the cut is the 24th of the wavelength alone. Measured over
    // 40 seeds: 0.22% of the amplitude at 24 pieces to the wavelength, 0.88% at 12, so this fails a cut half as fine.
    let (amplitude, wavelength) = (6.0, 80.0);
    for seed in 0..10 {
        let line = Path::line((3.0, 5.0), (1003.0, 300.0))?;
        let cut = lines(&line.wobble(amplitude, wavelength, 0.3, f64::from(seed))?)?;
        let fine = lines(
            &line
                .resample(wavelength / 400.0)?
                .wobble(amplitude, wavelength, 0.3, f64::from(seed))?,
        )?;
        for point in &fine[0].0 {
            let off = to_lines(*point, &cut);
            assert!(
                off <= 0.004 * amplitude,
                "seed {seed}: {point:?} is {off} from the cut line"
            );
        }
    }
    Ok(())
}
