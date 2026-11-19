//! `Path::stipple` through the public API. Oracles: the dots in a slab of a tone ramp are counted against the density
//! mapping worked out by hand (cells in the slab times the density at its tone), a lone dot's bounds are its diameter, and
//! the region is checked with `Path::contains`.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Field, Path, Rect, Stipple};

fn rect(width: f64, height: f64) -> Result<Path, bullterrier_vector::Error> {
    Path::rect(0.0, 0.0, width, height, 0.0)
}

/// The centres of the dots: a dot is a circle that starts at its top, so its start is `(x, y - radius)`.
fn centres(path: &Path, radius: f64) -> Vec<(f64, f64)> {
    path.to_svg_path()
        .split('M')
        .skip(1)
        .map(|part| {
            let numbers: Vec<f64> = part
                .split(|c: char| c.is_ascii_alphabetic() || c == ' ' || c == ',')
                .filter_map(|piece| piece.parse().ok())
                .collect();
            (numbers[0], numbers[1] + radius)
        })
        .collect()
}

#[test]
fn a_lone_dot_is_a_circle_of_the_radius() -> Outcome {
    let drawn = rect(4.0, 4.0)?.stipple(&Stipple::new(4.0, Field::constant(0.0)?, (0.7, 0.7))?)?;
    let bounds = drawn.bounds()?;
    assert!(
        (bounds.width - 1.4).abs() < 1e-9 && (bounds.height - 1.4).abs() < 1e-9,
        "{bounds:?}"
    );
    assert!(bounds.x >= 0.0 && bounds.y >= 0.0 && bounds.x + bounds.width <= 4.0 + 1e-9);
    Ok(())
}

#[test]
fn the_density_follows_the_tone() -> Outcome {
    // a ramp from 0 at x = 0 to 1 at x = 400; density 1 at tone 0, 0 at tone 1 (the default): dark is dense
    let tone = Field::ramp((0.0, 0.0), (400.0, 0.0))?;
    let drawn = rect(400.0, 100.0)?.stipple(&Stipple::new(4.0, tone, (0.8, 0.8))?)?;
    let dots = centres(&drawn, 0.8);
    for (slab, mid) in [(0.0, 0.125), (100.0, 0.375), (200.0, 0.625), (300.0, 0.875)] {
        let count = dots.iter().filter(|d| d.0 >= slab && d.0 < slab + 100.0).count();
        let want = 625.0 * (1.0 - mid);
        assert!(
            (f64::from(u32::try_from(count)?) - want).abs() <= 0.08 * 625.0,
            "slab at {slab}: {count} dots, expected {want}"
        );
    }
    Ok(())
}

#[test]
fn a_density_pair_maps_tone_0_and_tone_1() -> Outcome {
    let tone = Field::ramp((0.0, 0.0), (400.0, 0.0))?;
    let stipple = Stipple::new(4.0, tone, (0.8, 0.8))?.density(0.2, 0.8);
    let dots = centres(&rect(400.0, 100.0)?.stipple(&stipple)?, 0.8);
    let first = f64::from(u32::try_from(dots.iter().filter(|d| d.0 < 100.0).count())?);
    let last = f64::from(u32::try_from(dots.iter().filter(|d| d.0 >= 300.0).count())?);
    // density 0.2 + 0.6 * 0.125 = 0.275 and 0.2 + 0.6 * 0.875 = 0.725 of 625 cells
    assert!((first - 0.275 * 625.0).abs() < 0.06 * 625.0, "{first}");
    assert!((last - 0.725 * 625.0).abs() < 0.06 * 625.0, "{last}");
    Ok(())
}

#[test]
fn dots_stay_inside_the_region_and_a_seed_moves_them() -> Outcome {
    let region = Path::circle((100.0, 100.0), 60.0)?;
    let drawn = region.stipple(&Stipple::new(5.0, Field::constant(0.0)?, (1.0, 1.0))?)?;
    let dots = centres(&drawn, 1.0);
    assert!(dots.len() > 300);
    for d in &dots {
        assert!(region.contains((d.0, d.1))?, "{d:?}");
    }
    let other = region.stipple(&Stipple::new(5.0, Field::constant(0.0)?, (1.0, 1.0))?.seed(7.0))?;
    assert_ne!(other.to_svg_path(), drawn.to_svg_path());
    assert_eq!(
        drawn.to_svg_path(),
        region
            .stipple(&Stipple::new(5.0, Field::constant(0.0)?, (1.0, 1.0))?)?
            .to_svg_path()
    );
    Ok(())
}

#[test]
fn what_it_cannot_make_is_refused_naming_the_argument() -> Outcome {
    let flat = || Field::constant(0.0);
    let region = rect(50.0, 50.0)?;
    let call = |s: Result<Stipple, bullterrier_vector::Error>| s.and_then(|s| region.stipple(&s));
    for spacing in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            parameter_refusal(call(Stipple::new(spacing, flat()?, (1.0, 1.0))))?.0,
            "spacing"
        );
    }
    for radius in [0.0, -1.0, f64::NAN, 1e101] {
        assert_eq!(
            parameter_refusal(call(Stipple::new(5.0, flat()?, (radius, radius))))?.0,
            "radius"
        );
    }
    for density in [(-0.1, 1.0), (0.0, 1.5), (f64::NAN, 0.0)] {
        let stipple = Stipple::new(5.0, flat()?, (1.0, 1.0)).map(|s| s.density(density.0, density.1));
        assert_eq!(parameter_refusal(call(stipple))?.0, "density", "{density:?}");
    }
    let seeded = Stipple::new(5.0, flat()?, (1.0, 1.0)).map(|s| s.seed(0.5));
    assert_eq!(parameter_refusal(call(seeded))?.0, "seed");
    let huge = Rect {
        x: 0.0,
        y: 0.0,
        width: 1e6,
        height: 1e6,
    };
    let big = Path::rect(huge.x, huge.y, huge.width, huge.height, 0.0)?;
    assert_eq!(
        parameter_refusal(big.stipple(&Stipple::new(1.0, flat()?, (0.4, 0.4))?))?.0,
        "spacing"
    );
    assert_eq!(
        Path::new()
            .stipple(&Stipple::new(5.0, flat()?, (1.0, 1.0))?)?
            .to_svg_path(),
        ""
    );
    Ok(())
}

/// Each dot's centre and radius from its own bounds: a dot is a circle, so its box is its diameter about its centre.
fn dots(path: &Path) -> Result<Vec<(f64, f64, f64)>, bullterrier_vector::Error> {
    path.subpaths()
        .iter()
        .map(|dot| {
            let b = dot.bounds()?;
            Ok((b.x + b.width / 2.0, b.y + b.height / 2.0, b.width / 2.0))
        })
        .collect()
}

#[test]
fn a_radius_pair_follows_the_tone_read_at_each_dots_centre() -> Outcome {
    // a steep ramp, 0 at x = 0 to 1 at x = 40, held beyond: a dot stands up to 1.6 px from its cell's middle, which is
    // up to 0.04 of tone and 0.06 px of radius away, far past the tolerance
    let tone = Field::ramp((0.0, 0.0), (40.0, 0.0))?;
    let (small, large) = (0.5, 2.0);
    let drawn = rect(60.0, 40.0)?.stipple(&Stipple::new(4.0, tone, (small, large))?.density(1.0, 1.0))?;
    let found = dots(&drawn)?;
    assert!(found.len() > 100, "{}", found.len());
    let mut sizes = Vec::new();
    for (x, y, radius) in found {
        let share = (x / 40.0).clamp(0.0, 1.0);
        let want = small + (large - small) * share;
        assert!(
            (radius - want).abs() < 1e-6,
            "dot at ({x}, {y}): radius {radius}, want {want}"
        );
        sizes.push(radius);
    }
    // both ends of the pair are reached: the dots past x = 40 are all the large size
    assert!(sizes.iter().any(|r| (r - large).abs() < 1e-9));
    assert!(sizes.iter().any(|r| *r < 0.6));
    Ok(())
}

#[test]
fn an_equal_radius_pair_is_the_one_number() -> Outcome {
    let tone = Field::ramp((0.0, 0.0), (40.0, 0.0))?;
    let drawn = rect(60.0, 40.0)?.stipple(&Stipple::new(4.0, tone, (1.2, 1.2))?)?;
    for (_, _, radius) in dots(&drawn)? {
        assert!((radius - 1.2).abs() < 1e-9, "{radius}");
    }
    Ok(())
}

#[test]
fn either_radius_out_of_range_is_refused_naming_radius() -> Outcome {
    let region = rect(50.0, 50.0)?;
    for pair in [(1.0, 0.0), (0.0, 1.0), (1.0, -2.0), (f64::NAN, 1.0), (1.0, 1e101)] {
        let stipple = Stipple::new(5.0, Field::constant(0.5)?, pair)?;
        assert_eq!(parameter_refusal(region.stipple(&stipple))?.0, "radius", "{pair:?}");
    }
    Ok(())
}
