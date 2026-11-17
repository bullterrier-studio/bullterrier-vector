//! `Path::bands` through the public API: lines whose width carries a tone along straight lines, waves, rings, a spiral
//! or rays. Oracles: the width at a sample is the documented mapping of the tone (read from the polygon's vertices and
//! from `Path::contains`, never from the library's own arithmetic), and the carriers are worked out by hand.

use crate::common::{Outcome, lines, parameter_refusal};
use bullterrier_vector::{Bands, Field, Path, Point};

fn rect(width: f64, height: f64) -> Result<Path, bullterrier_vector::Error> {
    Path::rect(0.0, 0.0, width, height, 0.0)
}

fn near(got: f64, want: f64, tolerance: f64, what: &str) {
    assert!((got - want).abs() <= tolerance, "{what}: {got} against {want}");
}

#[test]
fn along_straight_lines_the_width_at_a_vertex_is_the_mapping_of_the_tone() -> Outcome {
    // tone 0 at x = 0 to 1 at x = 100; width 0.5 at tone 0 and 4.5 at tone 1; lines 10 apart, half a spacing in
    let tone = Field::ramp((0.0, 0.0), (100.0, 0.0))?;
    let bands = Bands::lines(10.0, tone, (0.5, 4.5))?;
    let found = lines(&rect(100.0, 30.0)?.bands(&bands)?)?;
    assert_eq!(found.len(), 3, "one closed outline for each of the three lines");
    for (index, (points, closed)) in found.iter().enumerate() {
        assert!(closed);
        let centre = 5.0 + 10.0 * f64::from(u8::try_from(index)?);
        // every x has two vertices, one on each side of the centre line, as far apart as the mapping says
        let mut by_x: Vec<(f64, Vec<f64>)> = Vec::new();
        for p in points {
            match by_x.iter_mut().find(|(x, _)| (x - p.0).abs() < 1e-9) {
                Some((_, ys)) => ys.push(p.1),
                None => by_x.push((p.0, vec![p.1])),
            }
        }
        assert!(by_x.len() > 20);
        for (x, ys) in by_x {
            assert_eq!(ys.len(), 2, "x = {x}");
            let (low, high) = (ys[0].min(ys[1]), ys[0].max(ys[1]));
            near(high - low, 0.5 + 4.0 * x / 100.0, 1e-9, "width");
            near(f64::midpoint(low, high), centre, 1e-9, "centre");
        }
    }
    Ok(())
}

#[test]
fn rings_are_closed_outlines_with_the_hole_inside() -> Outcome {
    let region = Path::circle((100.0, 100.0), 45.0)?;
    let bands = Bands::rings(10.0, Field::constant(0.5)?, (1.0, 3.0))?; // constant tone 0.5: width 2
    let drawn = region.bands(&bands)?;
    // rings at radii 5, 15, 25, 35 are whole; 45 is cut by the region
    for (radius, whole) in [(5.0, true), (15.0, true), (25.0, true), (35.0, true)] {
        let _ = whole;
        for angle in [0.3_f64, 1.7, 3.1, 5.0] {
            let (sin, cos) = (libm::sin(angle), libm::cos(angle));
            let on = |r: f64| drawn.contains((100.0 + r * cos, 100.0 + r * sin));
            assert!(on(radius)?, "the ring at {radius} covers its own radius at {angle}");
            assert!(
                on(radius + 0.9)? && on(radius - 0.9)?,
                "and a pixel either side of the centre line, inside the width 2"
            );
            assert!(
                !on(radius + 1.3)? && !on(radius - 1.3)?,
                "and not past the width 2 at {radius}, {angle}"
            );
        }
    }
    Ok(())
}

#[test]
fn the_spiral_arms_are_one_spacing_apart_along_a_ray() -> Outcome {
    let region = Path::circle((100.0, 100.0), 90.0)?;
    let bands = Bands::spiral(12.0, Field::constant(0.5)?, (2.0, 2.0))?;
    let drawn = region.bands(&bands)?;
    // walk out along the +x ray: the centres of the runs inside the bands are 12 apart
    let mut centres = Vec::new();
    let mut start: Option<f64> = None;
    let mut r = 0.5;
    while r < 85.0 {
        let inside = drawn.contains((100.0 + r, 100.0))?;
        match (inside, start) {
            (true, None) => start = Some(r),
            (false, Some(from)) => {
                centres.push(f64::midpoint(from, r));
                start = None;
            }
            _ => {}
        }
        r += 0.02;
    }
    // the first run holds the centre of the spiral, where the arm starts: it has no pair
    centres.remove(0);
    assert!(centres.len() >= 6, "{centres:?}");
    for pair in centres.windows(2) {
        near(pair[1] - pair[0], 12.0, 0.15, "arm spacing");
    }
    Ok(())
}

#[test]
fn rays_are_as_many_as_fit_round_the_farthest_corner() -> Outcome {
    let region = Path::circle((50.0, 50.0), 40.0)?;
    let bands = Bands::rays(10.0, Field::constant(0.5)?, (1.0, 1.0))?.centre((50.0, 50.0));
    let drawn = region.bands(&bands)?;
    // the bounding box corner is 40 sqrt 2 = 56.57 from the centre: 2 pi 56.57 / 10 = 35.5, so 36 rays
    let mut runs = 0;
    let mut before = false;
    for k in 0..7200 {
        // start in the gap between two rays, so that no run wraps past the start
        let angle = (f64::from(k) / 7200.0 + 0.5 / 36.0) * std::f64::consts::TAU;
        let inside = drawn.contains((50.0 + 30.0 * libm::cos(angle), 50.0 + 30.0 * libm::sin(angle)))?;
        if inside && !before {
            runs += 1;
        }
        before = inside;
    }
    assert_eq!(runs, 36);
    Ok(())
}

#[test]
fn waves_follow_a_sine_across_the_lines() -> Outcome {
    let bands = Bands::waves(20.0, Field::constant(0.5)?, (2.0, 2.0), 40.0, 3.0)?;
    let drawn = rect(120.0, 40.0)?.bands(&bands)?;
    for k in 2..58 {
        let x = f64::from(k) * 2.0;
        let wave = 3.0 * libm::sin(std::f64::consts::TAU * x / 40.0);
        assert!(drawn.contains((x, 10.0 + wave))?, "on the wave at {x}");
        assert!(drawn.contains((x, 30.0 + wave))?);
        assert!(!drawn.contains((x, 10.0 + wave + 2.5))? && !drawn.contains((x, 10.0 + wave - 2.5))?);
        assert!(!drawn.contains((x, 20.0 + wave))?, "and nothing between the lines");
    }
    Ok(())
}

#[test]
fn bands_stay_inside_the_region_up_to_half_their_width() -> Outcome {
    let region = Path::circle((60.0, 60.0), 50.0)?;
    for bands in [
        Bands::lines(7.0, Field::constant(1.0)?, (1.0, 4.0))?.angle(30.0),
        Bands::spiral(7.0, Field::constant(1.0)?, (1.0, 4.0))?,
        Bands::rays(7.0, Field::constant(1.0)?, (1.0, 4.0))?,
    ] {
        for (points, _) in lines(&region.bands(&bands)?)? {
            for p in points {
                assert!(libm::hypot(p.0 - 60.0, p.1 - 60.0) <= 50.0 + 2.0 + 0.05, "{p:?}");
            }
        }
    }
    Ok(())
}

#[test]
fn what_it_cannot_make_is_refused_naming_the_argument() -> Outcome {
    let flat = || Field::constant(0.5);
    let region = rect(50.0, 50.0)?;
    let call = |bands: Result<Bands, bullterrier_vector::Error>| bands.and_then(|bands| region.bands(&bands));
    for spacing in [0.0, -2.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            parameter_refusal(call(Bands::lines(spacing, flat()?, (1.0, 2.0))))?.0,
            "spacing"
        );
    }
    for width in [(-1.0, 2.0), (1.0, f64::NAN), (f64::INFINITY, 1.0), (1.0, 1e101)] {
        assert_eq!(
            parameter_refusal(call(Bands::lines(5.0, flat()?, width)))?.0,
            "width",
            "{width:?}"
        );
    }
    assert_eq!(
        parameter_refusal(call(Bands::waves(5.0, flat()?, (1.0, 1.0), 0.0, 1.0)))?.0,
        "wavelength"
    );
    assert_eq!(
        parameter_refusal(call(Bands::waves(5.0, flat()?, (1.0, 1.0), 10.0, f64::NAN)))?.0,
        "amplitude"
    );
    assert_eq!(
        parameter_refusal(call(Bands::lines(5.0, flat()?, (1.0, 1.0)).map(|b| b.angle(f64::NAN))))?.0,
        "angle"
    );
    assert_eq!(
        parameter_refusal(call(
            Bands::rings(5.0, flat()?, (1.0, 1.0)).map(|b| b.centre(Point::new(f64::NAN, 0.0)))
        ))?
        .0,
        "centre"
    );
    // too many: a tiny spacing over a big region, rays from a centre far away
    let big = rect(1e5, 1e5)?;
    assert_eq!(
        parameter_refusal(big.bands(&Bands::lines(0.01, flat()?, (0.0, 0.01))?))?.0,
        "spacing"
    );
    assert_eq!(
        parameter_refusal(big.bands(&Bands::rays(1.0, flat()?, (0.0, 0.01))?))?.0,
        "spacing"
    );
    assert_eq!(
        Path::new()
            .bands(&Bands::lines(5.0, flat()?, (1.0, 1.0))?)?
            .to_svg_path(),
        ""
    );
    Ok(())
}

#[test]
fn the_same_call_gives_the_same_bands() -> Outcome {
    let bands = Bands::spiral(6.0, Field::ramp((0.0, 0.0), (90.0, 50.0))?, (0.5, 3.0))?;
    let region = rect(90.0, 60.0)?;
    assert_eq!(region.bands(&bands)?.to_svg_path(), region.bands(&bands)?.to_svg_path());
    Ok(())
}
