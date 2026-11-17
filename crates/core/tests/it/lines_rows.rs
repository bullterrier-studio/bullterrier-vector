//! `Path::rows` through the public API. Oracles: rows of a flat tone are straight lines at heights worked out by hand;
//! a ramp tone makes straight displaced rows, so the nearer rows' silhouettes are known exactly and every drawn point is
//! checked against them (none behind a nearer row); the region is checked with `Path::contains`.

use crate::common::{Outcome, lines, parameter_refusal};
use bullterrier_vector::{Field, Path, Rect, Rows};

fn rect(width: f64, height: f64) -> Result<Path, bullterrier_vector::Error> {
    Path::rect(0.0, 0.0, width, height, 0.0)
}

#[test]
fn a_flat_tone_gives_straight_rows_one_spacing_apart_from_half_a_spacing_in() -> Outcome {
    let rows = Rows::new(10.0, Field::constant(0.0)?, 8.0)?;
    let found = lines(&rect(100.0, 50.0)?.rows(&rows)?)?;
    let heights: Vec<f64> = found.iter().map(|(points, _)| points[0].1).collect();
    assert_eq!(heights, vec![5.0, 15.0, 25.0, 35.0, 45.0]);
    for (points, closed) in &found {
        assert!(!closed);
        assert!(points.iter().all(|p| (p.1 - points[0].1).abs() < 1e-12));
        assert!((points[0].0 - 0.0).abs() < 1e-9 && (points[points.len() - 1].0 - 100.0).abs() < 1e-9);
    }
    Ok(())
}

#[test]
fn a_tone_lifts_a_row_by_the_amplitude_times_the_tone() -> Outcome {
    let rows = Rows::new(10.0, Field::constant(0.5)?, 8.0)?;
    let found = lines(&rect(100.0, 20.0)?.rows(&rows)?)?;
    let heights: Vec<f64> = found.iter().map(|(points, _)| points[0].1).collect();
    assert_eq!(heights, vec![1.0, 11.0]);
    // a negative amplitude pushes down
    let down = lines(&rect(100.0, 20.0)?.rows(&Rows::new(10.0, Field::constant(0.5)?, -8.0)?)?)?;
    assert_eq!(down[0].0[0].1, 9.0);
    Ok(())
}

/// A bumpy 8 by 4 map over the box 200 by 100, from a small generator: tone that rises and falls along every row.
fn bumpy() -> Result<Field, bullterrier_vector::Error> {
    let mut seed = 12345_u32;
    let bytes: Vec<u8> = (0..32)
        .map(|_| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            u8::try_from((seed >> 16) % 256).unwrap_or(0)
        })
        .collect();
    Field::map(
        8,
        4,
        &bytes,
        Some(Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
        }),
        None,
    )
}

#[test]
fn no_drawn_point_lies_behind_a_nearer_rows_silhouette() -> Outcome {
    let (spacing, amplitude) = (10.0, 120.0);
    let tone = bumpy()?;
    let found = lines(&rect(200.0, 100.0)?.rows(&Rows::new(spacing, tone.clone(), amplitude)?)?)?;
    assert_ne!(found.len(), 0);
    // the oracle: every row rebuilt from the tone as a polyline with a point every half spacing, nearest row first
    let silhouettes: Vec<Vec<(f64, f64)>> = (0..10)
        .map(|k| {
            let base = 5.0 + spacing * f64::from(k);
            (0..=40)
                .map(|i| {
                    (
                        f64::from(i) * 5.0,
                        base - amplitude * tone.at((f64::from(i) * 5.0, base)),
                    )
                })
                .collect()
        })
        .collect();
    let height = |row: &[(f64, f64)], x: f64| {
        let at = (0..39).take_while(|i| f64::from(i + 1) * 5.0 <= x).count();
        let (a, b) = (row[at], row[at + 1]);
        a.1 + (b.1 - a.1) * (x - a.0) / (b.0 - a.0)
    };
    let mut drawn = 0;
    for (points, _) in &found {
        // which row is it: the one whose base the first point's tone lifts to its height
        let row = silhouettes
            .iter()
            .position(|row| points.iter().all(|p| (p.1 - height(row, p.0)).abs() < 1e-6))
            .ok_or("a drawn point is on no row")?;
        for p in points {
            drawn += 1;
            for nearer in &silhouettes[row + 1..] {
                let wall = height(nearer, p.0);
                assert!(
                    p.1 <= wall + 1e-6,
                    "({}, {}) lies behind a row in front, whose silhouette is {wall}",
                    p.0,
                    p.1
                );
            }
        }
    }
    let full = 10 * 41;
    assert!(
        drawn < full * 9 / 10,
        "{drawn} of {full}: hidden lines were not removed"
    );
    let front = found.last().ok_or("no rows")?;
    assert_eq!(front.0.len(), 41, "the nearest row is drawn whole");
    Ok(())
}

#[test]
fn a_row_is_cut_where_the_one_in_front_crosses_it() -> Outcome {
    // a 2 by 2 map over the box 100 by 20: tone 0 along y = 5, and along y = 15 rising from 0 at x = 25 to 1 at x = 75
    let tone = Field::map(
        2,
        2,
        &[0, 0, 0, 255],
        Some(Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 20.0,
        }),
        None,
    )?;
    let found = lines(&rect(100.0, 20.0)?.rows(&Rows::new(10.0, tone, 20.0)?)?)?;
    // the back row stays at 5; the front row, 15 - 20 tone, meets it at tone 1/2, x = 50, and passes above it after
    let back = &found[0].0;
    assert!(
        (back[0].0 - 0.0).abs() < 1e-9 && (back[back.len() - 1].0 - 50.0).abs() < 1e-6,
        "{back:?}"
    );
    assert!(back.iter().all(|p| (p.1 - 5.0).abs() < 1e-9));
    assert_eq!(found.len(), 2);
    Ok(())
}

#[test]
fn rows_stay_inside_the_region() -> Outcome {
    let region = Path::circle((100.0, 100.0), 60.0)?;
    let rows = Rows::new(8.0, Field::ramp((0.0, 0.0), (200.0, 0.0))?, 10.0)?;
    for (points, _) in lines(&region.rows(&rows)?)? {
        for p in points {
            // the row's base is inside: lift it back by the displacement the ramp gives there
            let base = (p.0, p.1 + 10.0 * (p.0 / 200.0).clamp(0.0, 1.0));
            assert!(
                libm::hypot(base.0 - 100.0, base.1 - 100.0) <= 60.0 + 0.05,
                "{p:?} is outside"
            );
        }
    }
    Ok(())
}

#[test]
fn what_it_cannot_make_is_refused_naming_the_argument() -> Outcome {
    let flat = || Field::constant(0.0);
    for spacing in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let refusal =
            parameter_refusal(Rows::new(spacing, flat()?, 1.0).and_then(|rows| rect(10.0, 10.0)?.rows(&rows)))?;
        assert_eq!(refusal.0, "spacing", "{spacing}");
    }
    for amplitude in [f64::NAN, f64::INFINITY, 1e101] {
        let refusal =
            parameter_refusal(Rows::new(5.0, flat()?, amplitude).and_then(|rows| rect(10.0, 10.0)?.rows(&rows)))?;
        assert_eq!(refusal.0, "amplitude", "{amplitude}");
    }
    // a tiny spacing over a big region: refused before any row is made
    let refusal = parameter_refusal(rect(1e6, 1e6)?.rows(&Rows::new(1e-3, flat()?, 1.0)?))?;
    assert_eq!(refusal.0, "spacing");
    // an empty path has no rows
    assert_eq!(Path::new().rows(&Rows::new(5.0, flat()?, 1.0)?)?.to_svg_path(), "");
    Ok(())
}

#[test]
fn the_same_call_gives_the_same_rows() -> Outcome {
    let rows = Rows::new(7.0, Field::ramp((0.0, 0.0), (90.0, 50.0))?, 12.0)?;
    let region = rect(90.0, 60.0)?;
    assert_eq!(region.rows(&rows)?.to_svg_path(), region.rows(&rows)?.to_svg_path());
    Ok(())
}
