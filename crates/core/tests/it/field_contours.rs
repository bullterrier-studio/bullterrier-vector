//! `Field::contours` through the public API. Oracles: the level set of a ramp is a straight line, of a circle's
//! distance an exact circle, a saddle's segments are worked out by hand, and the SVG of the result is parsed here.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Error, Field, Path, Rect};

fn frame(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect { x, y, width, height }
}

/// The sub-paths of path data written by the library (`M x y L x y ... Z`), as points and whether each is closed.
fn lines(path: &Path) -> Vec<(Vec<(f64, f64)>, bool)> {
    let text = path.to_svg_path();
    text.split('M')
        .skip(1)
        .map(|part| {
            let closed = part.contains('Z');
            let numbers: Vec<f64> = part
                .split(|c: char| c.is_ascii_alphabetic() || c == ' ' || c == ',')
                .filter(|piece| !piece.is_empty())
                .filter_map(|piece| piece.parse().ok())
                .collect();
            (numbers.chunks(2).map(|pair| (pair[0], pair[1])).collect(), closed)
        })
        .collect()
}

#[test]
fn the_level_of_a_ramp_is_one_straight_line_across_the_frame() -> Outcome {
    let ramp = Field::ramp((0.0, 0.0), (100.0, 0.0))?;
    let path = ramp.contours(&[0.37], frame(0.0, 0.0, 100.0, 50.0), 5.0)?;
    let found = lines(&path);
    assert_eq!(found.len(), 1);
    let (points, closed) = &found[0];
    assert!(!closed);
    assert!(points.iter().all(|p| (p.0 - 37.0).abs() < 1e-9), "{points:?}");
    let ys: Vec<f64> = points.iter().map(|p| p.1).collect();
    assert!(ys.iter().any(|y| y.abs() < 1e-9) && ys.iter().any(|y| (y - 50.0).abs() < 1e-9));
    Ok(())
}

#[test]
fn the_level_of_a_circles_distance_is_a_closed_circle() -> Outcome {
    let distance = Field::distance(&Path::circle((100.0, 100.0), 50.0)?, false)?;
    let path = distance.contours(&[-20.0, 15.0], frame(0.0, 0.0, 200.0, 200.0), 2.0)?;
    let found = lines(&path);
    assert_eq!(found.len(), 2);
    for ((points, closed), radius) in found.iter().zip([30.0, 65.0]) {
        assert!(closed, "a loop inside the frame is closed");
        for p in points {
            let r = libm::hypot(p.0 - 100.0, p.1 - 100.0);
            assert!((r - radius).abs() < 0.05, "{r} against {radius}");
        }
        assert!(points.len() > 40);
    }
    Ok(())
}

#[test]
fn a_loop_that_meets_the_frame_is_open_there() -> Outcome {
    let distance = Field::distance(&Path::circle((100.0, 100.0), 50.0)?, false)?;
    // the frame cuts the circle of radius 65: the line leaves it and enters it again
    let path = distance.contours(&[15.0], frame(60.0, 0.0, 140.0, 200.0), 2.0)?;
    let found = lines(&path);
    assert_eq!(found.len(), 1);
    assert!(!found[0].1);
    let (first, last) = (found[0].0[0], found[0].0[found[0].0.len() - 1]);
    assert!(
        (first.0 - 60.0).abs() < 1e-9 && (last.0 - 60.0).abs() < 1e-9,
        "{first:?} {last:?}"
    );
    Ok(())
}

#[test]
fn a_saddle_is_decided_by_the_mean_of_its_corners() -> Outcome {
    // one cell whose corners read 1, 0 / 0, 1: a map of 2 by 2 pixels over the box of its centres
    let checker = Field::map(2, 2, &[255, 0, 0, 255], Some(frame(-0.5, -0.5, 2.0, 2.0)))?;
    let cell = frame(0.0, 0.0, 1.0, 1.0);
    // mean 0.5 is above 0.4: the high corners are joined, so each low corner is cut off alone
    let joined = lines(&checker.contours(&[0.4], cell, 1.0)?);
    assert_eq!(joined.len(), 2);
    let mut want_ends = vec![[(0.6, 0.0), (1.0, 0.4)], [(0.4, 1.0), (0.0, 0.6)]];
    for (points, _) in &joined {
        assert_eq!(points.len(), 2);
        let near = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9;
        let at = want_ends
            .iter()
            .position(|w| {
                (near(points[0], w[0]) && near(points[1], w[1])) || (near(points[0], w[1]) && near(points[1], w[0]))
            })
            .ok_or_else(|| format!("unexpected segment {points:?}"))?;
        want_ends.remove(at);
    }
    // mean 0.5 is below 0.6: the low corners are joined, so each high corner is cut off alone
    let apart = lines(&checker.contours(&[0.6], cell, 1.0)?);
    assert_eq!(apart.len(), 2);
    let mut want_ends = vec![[(0.4, 0.0), (0.0, 0.4)], [(1.0, 0.6), (0.6, 1.0)]];
    for (points, _) in &apart {
        let near = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9;
        let at = want_ends
            .iter()
            .position(|w| {
                (near(points[0], w[0]) && near(points[1], w[1])) || (near(points[0], w[1]) && near(points[1], w[0]))
            })
            .ok_or_else(|| format!("unexpected segment {points:?}"))?;
        want_ends.remove(at);
    }
    Ok(())
}

#[test]
fn a_level_the_field_never_reaches_gives_nothing() -> Outcome {
    let ramp = Field::ramp((0.0, 0.0), (100.0, 0.0))?;
    assert_eq!(
        ramp.contours(&[2.0, -1.0], frame(0.0, 0.0, 10.0, 10.0), 1.0)?
            .to_svg_path(),
        ""
    );
    Ok(())
}

#[test]
fn the_same_call_gives_the_same_path_and_a_far_frame_still_works() -> Outcome {
    let distance = Field::distance(&Path::circle((100.0, 100.0), 50.0)?, false)?;
    let a = distance.contours(&[-10.0, 0.0], frame(0.0, 0.0, 200.0, 200.0), 3.0)?;
    let b = distance.contours(&[-10.0, 0.0], frame(0.0, 0.0, 200.0, 200.0), 3.0)?;
    assert_eq!(a.to_svg_path(), b.to_svg_path());
    let far = 1e12;
    let moved = Field::distance(&Path::circle((far + 100.0, 100.0), 50.0)?, false)?;
    let found = lines(&moved.contours(&[0.0], frame(far, 0.0, 200.0, 200.0), 4.0)?);
    assert_eq!(found.len(), 1);
    assert!(
        found[0]
            .0
            .iter()
            .all(|p| (libm::hypot(p.0 - far - 100.0, p.1 - 100.0) - 50.0).abs() < 0.2)
    );
    Ok(())
}

#[test]
fn what_it_cannot_trace_is_refused_naming_the_argument() -> Outcome {
    let ramp = Field::ramp((0.0, 0.0), (100.0, 0.0))?;
    let ok = frame(0.0, 0.0, 10.0, 10.0);
    let many = vec![0.5; 257];
    let results: Vec<(Result<Path, Error>, &str)> = vec![
        (ramp.contours(&[f64::NAN], ok, 1.0), "levels"),
        (ramp.contours(&[f64::INFINITY], ok, 1.0), "levels"),
        (ramp.contours(&many, ok, 1.0), "levels"),
        (ramp.contours(&[0.5], ok, 0.0), "step"),
        (ramp.contours(&[0.5], ok, f64::NAN), "step"),
        (ramp.contours(&[0.5], ok, 1e-9), "step"),
        (ramp.contours(&[0.5], frame(0.0, 0.0, 0.0, 10.0), 1.0), "frame"),
        (ramp.contours(&[0.5], frame(f64::NAN, 0.0, 10.0, 10.0), 1.0), "frame"),
        (ramp.contours(&[0.5], frame(0.0, 0.0, 1e200, 10.0), 1.0), "frame"),
    ];
    for (outcome, parameter) in results {
        assert_eq!(parameter_refusal(outcome)?.0, parameter);
    }
    Ok(())
}
