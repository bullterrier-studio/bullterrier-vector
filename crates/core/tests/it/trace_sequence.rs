//! `Field::smooth_maps` through the public API: a sequence of maps from the caller made
//! steady before it is traced. Oracles: a static sequence (the single-map result, exactly), a weighted mean worked out by
//! hand from the Gaussian weights, and a measured flicker (the area that changes
//! between consecutive frames' shapes) against tracing each frame alone.

#![expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "map sizes and tones are small"
)]

use crate::common::region::net_area;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Drawing, Error, Field, Path, Shape, Trace, Values};

const W: usize = 24;
const H: usize = 16;

fn field(bytes: &[u8]) -> Result<Field, Error> {
    Field::map(W, H, bytes, None, None)
}

fn bytes_of(map: &Field) -> Result<Vec<u8>, Error> {
    map.to_map(W, H, None, 0.0, 1.0)
}

/// Two byte maps equal, or the first differing index (a mismatch must not print a whole map).
fn same(got: &[u8], want: &[u8], what: &str) {
    let first = got.iter().zip(want).position(|(a, b)| a != b);
    assert!(
        got.len() == want.len() && first.is_none(),
        "{what}: first difference at {first:?}, lengths {} and {}",
        got.len(),
        want.len()
    );
}

fn levels(map: &Field, at: &[f64], trace: &Trace) -> Result<Vec<Shape>, Error> {
    Drawing::trace_levels(map, at.iter().copied(), trace).shapes(&Values::new())
}

#[test]
fn a_static_sequence_smooths_to_itself_and_traces_the_same() -> Outcome {
    let bytes: Vec<u8> = (0..W * H).map(|i| ((i * 37 + (i / W) * 11) % 251) as u8).collect();
    let one = field(&bytes)?;
    let maps = vec![one.clone(); 5];
    let trace = Trace::new().smooth(1.0).tolerance(0.5);
    let want = levels(&one, &[0.3, 0.6], &trace)?;
    for sigma in [0.0, 0.7, 1.5] {
        for smooth in Field::smooth_maps(&maps, sigma)? {
            same(&bytes_of(&smooth)?, &bytes, &format!("sigma {sigma}"));
            let got = levels(&smooth, &[0.3, 0.6], &trace)?;
            let paths = |shapes: &[Shape]| shapes.iter().map(|s| s.path().to_svg_path()).collect::<Vec<_>>();
            assert_eq!(paths(&got), paths(&want), "sigma {sigma}");
        }
    }
    Ok(())
}

#[test]
fn a_frame_is_the_gaussian_weighted_mean_of_its_neighbours() -> Outcome {
    // seven 24 by 16 maps, each one flat byte; sigma 1 reaches 3 frames either way (taps within ceil(3 sigma))
    let flat = [10_u8, 20, 40, 80, 160, 30, 60];
    let maps = flat
        .iter()
        .map(|&b| field(&vec![b; W * H]))
        .collect::<Result<Vec<_>, _>>()?;
    // weights exp(-d^2 / 2) for d = 0..3
    let weight = [
        1.0,
        0.606_530_659_712_633_4,
        0.135_335_283_236_612_7,
        0.011_108_996_538_242_306,
    ];
    let mean = |at: usize| {
        let (mut sum, mut total) = (0.0, 0.0);
        for (i, &b) in flat.iter().enumerate() {
            let d = at.abs_diff(i);
            if d <= 3 {
                sum += weight[d] * f64::from(b);
                total += weight[d];
            }
        }
        (sum / total).round() as u8
    };
    let smooth = Field::smooth_maps(&maps, 1.0)?;
    assert_eq!(smooth.len(), 7);
    for (at, map) in smooth.iter().enumerate() {
        same(&bytes_of(map)?, &vec![mean(at); W * H], &format!("frame {at}"));
    }
    // the ends are renormalised over the frames that exist, not padded
    assert_eq!(mean(0), 16);
    Ok(())
}

#[test]
fn a_noisy_sequence_flickers_less_when_steadied() -> Outcome {
    // a disc, 48 by 48, drifting a pixel a frame, with a seeded speckle of noise on every frame
    let (width, height) = (48, 48);
    let mut seed = 12_345_u64;
    let mut noise = move || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((seed >> 33) % 240) as i32 - 120
    };
    let frame = |i: usize, noise: &mut dyn FnMut() -> i32| {
        let centre = (20.0 + i as f64 * 0.5, 24.0);
        let bytes: Vec<u8> = (0..width * height)
            .map(|p| {
                let (x, y) = ((p % width) as f64 + 0.5, (p / width) as f64 + 0.5);
                let squared = (x - centre.0) * (x - centre.0) + (y - centre.1) * (y - centre.1);
                let base = if squared < 196.0 { 190 } else { 40 };
                (base + noise()).clamp(0, 255) as u8
            })
            .collect();
        Field::map(width, height, &bytes, None, None)
    };
    let maps = (0..9).map(|i| frame(i, &mut noise)).collect::<Result<Vec<_>, _>>()?;
    let trace = Trace::new().tolerance(0.0);
    let flicker = |maps: &[Field]| -> Result<f64, Box<dyn std::error::Error>> {
        let shapes = maps
            .iter()
            .map(|m| levels(m, &[0.5], &trace).map(|s| s.into_iter().next()))
            .collect::<Result<Vec<_>, _>>()?;
        let mut total = 0.0;
        for pair in shapes.windows(2) {
            if let [Some(a), Some(b)] = pair {
                total += net_area(&a.path().exclude(b.path(), 0.001)?, 1)?.abs();
            }
        }
        Ok(total)
    };
    let alone = flicker(&maps)?;
    let steady = Field::smooth_maps(&maps, 1.5)?;
    let after = flicker(&steady)?;
    assert!(after < alone * 0.5, "{after} against {alone}");
    Ok(())
}

#[test]
fn refusals_name_the_argument() -> Outcome {
    let a = field(&vec![0; W * H])?;
    let small = Field::map(2, 2, &[0; 4], None, None)?;
    let shifted = Field::map(
        W,
        H,
        &vec![0; W * H],
        Some(bullterrier_vector::Rect {
            x: 1.0,
            y: 0.0,
            width: 24.0,
            height: 16.0,
        }),
        None,
    )?;
    let ramp = Field::distance(Path::rect(0.0, 0.0, 4.0, 4.0, 0.0)?, false)?;
    assert_eq!(
        parameter_refusal(Field::smooth_maps(&[a.clone(), small], 1.0))?.0,
        "maps"
    );
    assert_eq!(
        parameter_refusal(Field::smooth_maps(&[a.clone(), shifted], 1.0))?.0,
        "maps"
    );
    assert_eq!(parameter_refusal(Field::smooth_maps(&[ramp], 1.0))?.0, "maps");
    for sigma in [-1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            parameter_refusal(Field::smooth_maps(std::slice::from_ref(&a), sigma))?.0,
            "sigma"
        );
    }
    Ok(())
}
