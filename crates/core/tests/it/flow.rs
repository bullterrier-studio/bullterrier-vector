//! `Flow` through the public API: directions without sign, in degrees from 0 up to 180. Oracles: the angle given,
//! the tangent of a circle, the doubled-angle mean computed here with plain `atan2`, the sum of two angles modulo 180
//! and the radius of a circle (`atan2` of the point from the centre).

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Error, Flow, Param, Path, Point, by_index};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

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
    let backward = Flow::contour(circle()?.reverse())?;
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
    assert!(Flow::contour(Path::new()).is_err());
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

/// Points spread over a few hundred pixels, none on a lattice line of the noise.
const SAMPLES: [(f64, f64); 5] = [(3.1, 7.7), (143.7, 101.3), (97.1, 242.9), (-62.3, 71.7), (333.3, -39.1)];

fn check(condition: bool, what: String) -> Result<(), TestCaseError> {
    if condition {
        Ok(())
    } else {
        Err(TestCaseError::fail(what))
    }
}

fn flow_case(result: Result<Flow, Error>) -> Result<Flow, TestCaseError> {
    result.map_err(|error| TestCaseError::fail(error.to_string()))
}

proptest! {
    /// A turn of an angle is the angle plus the turn, clockwise: the oracle is the sum modulo 180.
    #[test]
    fn turning_an_angle_adds_the_turn(start in -720.0_f64..720.0, angle in -720.0_f64..720.0) {
        let turned = flow_case(Flow::angle(start).and_then(|flow| flow.rotate(angle)))?;
        let want = (start + angle).rem_euclid(180.0);
        let got = turned.at((10.0, 20.0));
        check(apart(got, want) < 1e-9, format!("{start} turned {angle}: {got}, want {want}"))?;
    }

    /// Laws of a turn over a flow that varies: 0 and 180 change nothing, two turns are one turn by their sum.
    #[test]
    fn turns_by_0_and_180_change_nothing_and_two_turns_add(seed in 0_u32..1000, a in -400.0_f64..400.0, b in -400.0_f64..400.0) {
        let flow = flow_case(Flow::noise(90.0, 0.0, f64::from(seed)))?;
        let (zero, half) = (flow_case(flow.rotate(0.0))?, flow_case(flow.rotate(180.0))?);
        let (twice, once) = (flow_case(flow.rotate(a).and_then(|f| f.rotate(b)))?, flow_case(flow.rotate(a + b))?);
        for (x, y) in SAMPLES {
            let p = Point::new(x, y);
            check(apart(zero.at(p), flow.at(p)) < 1e-9, format!("0 at {p:?}"))?;
            check(apart(half.at(p), flow.at(p)) < 1e-9, format!("180 at {p:?}"))?;
            check(apart(twice.at(p), once.at(p)) < 1e-9, format!("{a} then {b} at {p:?}"))?;
            // a turn by `a` moves every direction by `a`: the noise read here is the oracle's start
            check(apart(once.at(p), flow.at(p) + a + b) < 1e-9, format!("{a} + {b} at {p:?}"))?;
        }
    }
}

#[test]
fn a_quarter_turn_of_a_circle_contour_runs_across_the_outline_along_the_radius() -> Outcome {
    let across = Flow::contour(&circle()?)?.rotate(90.0)?;
    for (x, y) in [
        (150.0, 100.0),
        (100.0, 150.0),
        (135.4, 64.6),
        (61.0, 70.0),
        (120.0, 104.0),
    ] {
        // the radius through the point, from the centre (100, 100)
        let radius = libm::atan2(y - 100.0, x - 100.0).to_degrees();
        assert!(
            apart(across.at((x, y)), radius) < 1.0,
            "at ({x}, {y}): {}",
            across.at((x, y))
        );
    }
    Ok(())
}

#[test]
fn a_turn_by_a_param_is_read_at_its_default_and_a_per_item_number_is_refused() -> Outcome {
    let turn = Param::new("turn", 25.0, -90.0, 90.0)?;
    assert!(apart(Flow::angle(10.0)?.rotate(&turn)?.at((0.0, 0.0)), 35.0) < 1e-9);
    // a mix whose parts cancel keeps its first direction, and turned that direction turns
    let cancelled = Flow::angle(0.0)?.mix(&Flow::angle(90.0)?, 0.5)?.rotate(30.0)?;
    assert!(apart(cancelled.at((0.0, 0.0)), 30.0) < 1e-9);
    let (named, message) = parameter_refusal(Flow::angle(10.0)?.rotate(by_index(0.0, 90.0)))?;
    assert_eq!(named, "angle");
    assert!(message.contains("per-item"), "{message}");
    assert_eq!(parameter_refusal(Flow::angle(10.0)?.rotate(f64::NAN))?.0, "angle");
    assert_eq!(parameter_refusal(Flow::angle(10.0)?.rotate(f64::INFINITY))?.0, "angle");
    Ok(())
}

#[test]
fn a_turn_prints_as_its_call() -> Outcome {
    assert_eq!(
        Flow::angle(30.0)?.rotate(90.0)?.to_string(),
        "Flow.angle(30).rotate(90)"
    );
    // equal calls are equal, and a turn is not the angle it reads the same as
    assert_eq!(Flow::angle(30.0)?.rotate(90.0)?, Flow::angle(30.0)?.rotate(90.0)?);
    assert_ne!(Flow::angle(30.0)?.rotate(90.0)?, Flow::angle(120.0)?);
    assert_ne!(Flow::angle(30.0)?.rotate(90.0)?, Flow::angle(30.0)?.rotate(91.0)?);
    Ok(())
}

#[test]
fn turns_and_mixes_nest_16_deep_together() -> Outcome {
    let mut flow = Flow::angle(0.0)?;
    for _ in 0..16 {
        flow = flow.rotate(5.0)?;
    }
    assert!(apart(flow.at((0.0, 0.0)), 80.0) < 1e-9);
    assert_eq!(parameter_refusal(flow.rotate(5.0))?.0, "flow");
    // a mix over a turn 16 deep is 17 deep: refused as any mix too deep is
    assert_eq!(parameter_refusal(flow.mix(&Flow::angle(1.0)?, 0.5))?.0, "other");
    let mut mixed = Flow::angle(0.0)?;
    for i in 1..=16 {
        mixed = mixed.mix(&Flow::angle(f64::from(i))?, 0.5)?;
    }
    assert_eq!(parameter_refusal(mixed.rotate(1.0))?.0, "flow");
    Ok(())
}
