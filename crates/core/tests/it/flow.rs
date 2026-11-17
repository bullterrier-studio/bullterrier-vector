//! `Flow` through the public API: directions without sign, in degrees from 0 up to 180. Oracles: the angle given,
//! the tangent of a circle, and the doubled-angle mean computed here with plain `atan2`.

use crate::common::Outcome;
use bullterrier_vector::{Error, Flow, Path, Point};

/// The distance between two directions without sign, in degrees (at most 90).
fn apart(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(180.0);
    d.min(180.0 - d)
}

fn circle() -> Result<Path, Error> {
    Path::circle((100.0, 100.0), 50.0)
}

#[test]
fn an_angle_is_the_same_everywhere_and_has_no_sign() -> Outcome {
    for (given, want) in [(30.0, 30.0), (-30.0, 150.0), (210.0, 30.0), (0.0, 0.0)] {
        let flow = Flow::angle(given)?;
        for q in [(0.0, 0.0), (500.0, -20.0)] {
            let got = flow.at(Point::new(q.0, q.1));
            assert!(apart(got, want) < 1e-9 && (0.0..180.0).contains(&got), "{given}: {got}");
        }
    }
    Ok(())
}

#[test]
fn a_contour_follows_the_tangent_of_the_nearest_outline() -> Outcome {
    let flow = Flow::contour(&circle()?)?;
    assert!(apart(flow.at(Point::new(150.0, 100.0)), 90.0) < 1.0);
    assert!(apart(flow.at(Point::new(100.0, 150.0)), 0.0) < 1.0);
    // inside and outside agree: the nearest outline point is the same
    assert!(apart(flow.at(Point::new(130.0, 100.0)), 90.0) < 1.0);
    assert!(apart(flow.at(Point::new(400.0, 100.0)), 90.0) < 1.0);
    Ok(())
}

#[test]
fn a_direction_and_its_opposite_give_the_same_flow_when_mixed() -> Outcome {
    // the same outline run the other way has opposite tangents: a naive vector blend would cancel to nothing
    let forward = Flow::contour(&circle()?)?;
    let backward = Flow::contour(&circle()?.reverse())?;
    let mixed = forward.mix(&backward, 0.5)?;
    for q in [
        (143.7, 101.3),
        (97.1, 142.9),
        (62.3, 71.7),
        (133.3, 139.1),
        (10.2, 11.9),
    ] {
        let p = Point::new(q.0, q.1);
        assert!(apart(mixed.at(p), forward.at(p)) < 1e-6, "at {q:?}");
    }
    Ok(())
}

#[test]
fn a_mix_is_the_mean_in_doubled_angle_space() -> Outcome {
    // 10 and 170 degrees are 20 degrees either side of 0: the mean is 0, where the mean of the numbers is 90
    let mean = Flow::angle(10.0)?.mix(&Flow::angle(170.0)?, 0.5)?;
    assert!(apart(mean.at(Point::new(0.0, 0.0)), 0.0) < 1e-6);
    // 0 and 60 at a quarter: the doubled vectors 0.75 (1, 0) and 0.25 (cos 120, sin 120)
    let (x, y) = (0.75 + 0.25 * (-0.5), 0.25 * (3.0_f64.sqrt() / 2.0));
    let want = libm::atan2(y, x).to_degrees() / 2.0;
    let blend = Flow::angle(0.0)?.mix(&Flow::angle(60.0)?, 0.25)?;
    assert!(apart(blend.at(Point::new(5.0, 5.0)), want) < 1e-6);
    // fraction 0 and 1 are the two flows
    let zero = Flow::angle(20.0)?.mix(&Flow::angle(70.0)?, 0.0)?;
    let one = Flow::angle(20.0)?.mix(&Flow::angle(70.0)?, 1.0)?;
    assert!(apart(zero.at(Point::new(0.0, 0.0)), 20.0) < 1e-9 && apart(one.at(Point::new(0.0, 0.0)), 70.0) < 1e-9);
    Ok(())
}

#[test]
fn parts_that_cancel_exactly_keep_the_first_direction() -> Outcome {
    let flow = Flow::angle(0.0)?.mix(&Flow::angle(90.0)?, 0.5)?;
    assert!(apart(flow.at(Point::new(0.0, 0.0)), 0.0) < 1e-9);
    Ok(())
}

#[test]
fn noise_is_seeded_smooth_and_periodic_in_phase() -> Outcome {
    let a = Flow::noise(100.0, 0.0, 7.0)?;
    let again = Flow::noise(100.0, 0.0, 7.0)?;
    let other = Flow::noise(100.0, 0.0, 8.0)?;
    let cycle = Flow::noise(100.0, 1.0, 7.0)?;
    let mut differs = false;
    for i in 0..40 {
        let p = Point::new(f64::from(i) * 13.0, f64::from(i) * 7.0);
        assert_eq!(a.at(p).to_bits(), again.at(p).to_bits());
        assert_eq!(a.at(p).to_bits(), cycle.at(p).to_bits());
        assert!((0.0..180.0).contains(&a.at(p)));
        differs |= a.at(p).to_bits() != other.at(p).to_bits();
        // a step of a pixel changes the direction a little
        assert!(apart(a.at(p), a.at(Point::new(p.x + 0.5, p.y))) < 10.0);
    }
    assert!(differs);
    Ok(())
}

#[test]
fn refusals() -> Outcome {
    assert!(Flow::noise(0.0, 0.0, 0.0).is_err());
    assert!(Flow::angle(f64::NAN).is_err());
    assert!(Flow::angle(0.0)?.mix(&Flow::angle(1.0)?, 1.5).is_err());
    assert!(Flow::contour(&Path::new()).is_err());
    // a mix holds a bounded number of parts
    let mut flow = Flow::angle(0.0)?;
    let mut refused = false;
    for i in 1..200 {
        match flow.mix(&Flow::angle(f64::from(i))?, 0.5) {
            Ok(next) => flow = next,
            Err(_) => refused = true,
        }
    }
    assert!(refused);
    Ok(())
}
