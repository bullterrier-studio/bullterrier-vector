//! `Field::smooth_maps` and `Field::persist_holes` through the public API: a sequence of maps from the caller made
//! steady before it is traced. Oracles: a static sequence (the single-map result, exactly), a weighted mean worked out by
//! hand from the Gaussian weights, hand-built maps whose holes come and go, and a measured flicker (the area that changes
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

/// A bytes map: `base` everywhere, `dip` inside the `size` square at (`x`, `y`).
fn with_square(base: u8, dip: u8, x: usize, y: usize, size: usize) -> Vec<u8> {
    (0..W * H)
        .map(|i| {
            let (px, py) = (i % W, i / W);
            if px >= x && px < x + size && py >= y && py < y + size {
                dip
            } else {
                base
            }
        })
        .collect()
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

/// The area of the level-0.5 shape of `map` inside the frame.
fn lit_area(map: &Field, trace: &Trace) -> Result<f64, Box<dyn std::error::Error>> {
    let frame = Path::rect(0.0, 0.0, W as f64, H as f64, 0.0)?;
    let shapes = levels(map, &[0.5], trace)?;
    shapes
        .first()
        .map_or(Ok(0.0), |shape| net_area(&shape.path().intersect(&frame, 0.001)?, 1))
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
fn a_hole_in_one_frame_is_dropped_and_one_in_two_neighbours_is_kept() -> Outcome {
    let solid = with_square(200, 200, 0, 0, 0);
    let holed = with_square(200, 10, 8, 5, 3);
    let seq = |holes: &[usize]| -> Result<Vec<Field>, Error> {
        (0..5)
            .map(|i| field(if holes.contains(&i) { &holed } else { &solid }))
            .collect()
    };
    // the hole in frame 2 alone: gone from it
    let kept = Field::persist_holes(&seq(&[2])?, &[0.5], 1, 2)?;
    same(&bytes_of(&kept[2])?, &solid, "&kept[2]");
    // in frames 1 and 2: both keep it (each has one neighbour with it); frames 0 and 3 are solid
    let kept = Field::persist_holes(&seq(&[1, 2])?, &[0.5], 1, 2)?;
    same(&bytes_of(&kept[1])?, &holed, "&kept[1]");
    same(&bytes_of(&kept[2])?, &holed, "&kept[2]");
    same(&bytes_of(&kept[0])?, &solid, "&kept[0]");
    // asking for three of the window drops the pair's ends but keeps the middle of three
    let kept = Field::persist_holes(&seq(&[1, 2, 3])?, &[0.5], 1, 3)?;
    same(&bytes_of(&kept[2])?, &holed, "&kept[2]");
    same(&bytes_of(&kept[1])?, &solid, "&kept[1]");
    same(&bytes_of(&kept[3])?, &solid, "&kept[3]");
    Ok(())
}

#[test]
fn a_hole_that_moved_a_little_still_counts_and_one_elsewhere_does_not() -> Outcome {
    let solid = with_square(200, 200, 0, 0, 0);
    let a = with_square(200, 10, 8, 5, 3);
    let moved = with_square(200, 10, 9, 5, 3); // overlaps `a`
    let apart = with_square(200, 10, 18, 11, 3); // does not
    let kept = Field::persist_holes(&[field(&moved)?, field(&a)?, field(&solid)?], &[0.5], 1, 2)?;
    same(&bytes_of(&kept[0])?, &moved, "&kept[0]");
    same(&bytes_of(&kept[1])?, &a, "&kept[1]");
    let kept = Field::persist_holes(&[field(&apart)?, field(&a)?, field(&solid)?], &[0.5], 1, 2)?;
    same(&bytes_of(&kept[0])?, &solid, "&kept[0]");
    same(&bytes_of(&kept[1])?, &solid, "&kept[1]");
    Ok(())
}

#[test]
fn what_meets_the_frame_is_not_a_hole() -> Outcome {
    // a dark corner: the shape goes on past the frame, so it is no hole and is never filled
    let corner = with_square(200, 10, 0, 0, 4);
    let kept = Field::persist_holes(
        &[field(&corner)?, field(&with_square(200, 200, 0, 0, 0))?],
        &[0.5],
        1,
        2,
    )?;
    same(&bytes_of(&kept[0])?, &corner, "&kept[0]");
    Ok(())
}

#[test]
fn short_sequences_and_their_ends_keep_what_a_static_one_has() -> Outcome {
    let holed = field(&with_square(200, 10, 8, 5, 3))?;
    let alone = Field::persist_holes(std::slice::from_ref(&holed), &[0.5], 1, 3)?;
    same(&bytes_of(&alone[0])?, &bytes_of(&holed)?, "one map");
    // a static run: asking for more frames than the window holds (at the ends, 2 of 3) must not drop its hole
    let kept = Field::persist_holes(&vec![holed.clone(); 4], &[0.5], 1, 9)?;
    for map in &kept {
        same(&bytes_of(map)?, &bytes_of(&holed)?, "map");
    }
    assert_eq!(Field::persist_holes(&[], &[0.5], 1, 2)?.len(), 0);
    assert_eq!(Field::smooth_maps(&[], 1.0)?.len(), 0);
    Ok(())
}

#[test]
fn a_dropped_hole_takes_the_tone_round_it_and_stays_filled_through_the_trace_blur() -> Outcome {
    let solid = field(&with_square(200, 200, 0, 0, 0))?;
    let holed = field(&with_square(200, 10, 8, 5, 3))?;
    let steady = Field::persist_holes(&[solid.clone(), holed.clone(), solid], &[0.5], 1, 2)?;
    let blur = Trace::new().smooth(1.0).tolerance(0.0);
    let full = (W * H) as f64;
    assert!((lit_area(&steady[1], &blur)? - full).abs() < 1e-6);
    assert!(lit_area(&holed, &blur)? < full - 1.0);
    Ok(())
}

#[test]
fn nested_holes_at_two_levels_are_judged_each_at_its_own_level() -> Outcome {
    // an outer hole at 100 (below 0.6) holding an inner one at 10 (below 0.3); the neighbours have the outer only
    let outer_only = |x: usize, size: usize, dip: u8| with_square(200, dip, x, 4, size);
    let inner = |bytes: &mut Vec<u8>| {
        for y in 7..9 {
            for x in 9..11 {
                bytes[y * W + x] = 10;
            }
        }
    };
    let mut both = outer_only(6, 8, 100);
    inner(&mut both);
    let plain = outer_only(6, 8, 100);
    let maps = [field(&plain)?, field(&both)?, field(&plain)?];
    let kept = Field::persist_holes(&maps, &[0.3, 0.6], 1, 2)?;
    // the outer hole persists; the inner one takes its ring's tone, 100
    same(&bytes_of(&kept[1])?, &plain, "&kept[1]");
    // order of the levels does not matter
    let flipped = Field::persist_holes(&maps, &[0.6, 0.3], 1, 2)?;
    same(&bytes_of(&flipped[1])?, &plain, "&flipped[1]");
    // when nothing of the outer hole persists, all of it goes
    let solid = field(&with_square(200, 200, 0, 0, 0))?;
    let kept = Field::persist_holes(&[solid.clone(), field(&both)?, solid], &[0.3, 0.6], 1, 2)?;
    same(&bytes_of(&kept[1])?, &with_square(200, 200, 0, 0, 0), "&kept[1]");
    Ok(())
}

#[test]
fn a_noisy_sequence_flickers_less_when_steadied() -> Outcome {
    // a disc with a hole, 48 by 48, drifting a pixel a frame, with a seeded speckle of noise on every frame
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
    let steady = Field::persist_holes(&Field::smooth_maps(&maps, 1.5)?, &[0.5], 1, 2)?;
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
    assert_eq!(
        parameter_refusal(Field::persist_holes(std::slice::from_ref(&a), &[f64::NAN], 1, 2))?.0,
        "levels"
    );
    assert_eq!(
        parameter_refusal(Field::persist_holes(std::slice::from_ref(&a), &[0.5], 1, 0))?.0,
        "at_least"
    );
    Ok(())
}
