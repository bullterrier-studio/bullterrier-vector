//! `Path::hatch` through the public API: evenly spaced streamlines inside a region. The oracles are measured on the
//! output: the gap between parallel lines against the spacing asked, the floor between any two lines by brute force,
//! containment by `Path::contains`, the radius of a closed loop.

use crate::common::{Line, Outcome, lines, parameter_refusal, to_lines};
use bullterrier_vector::{Error, Field, Flow, Hatch, Path};

fn rect(width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(0.0, 0.0, width, height, 0.0)
}

fn ring(outer: f64, inner: f64) -> Result<Path, Error> {
    let outer = Path::circle((200.0, 200.0), outer)?;
    let hole = Path::circle((200.0, 200.0), inner)?.reverse();
    Path::from_svg_path(&format!("{} {}", outer.to_svg_path(), hole.to_svg_path()))
}

fn made(path: &Path, hatch: &Hatch, avoid: Option<&Path>) -> Result<Vec<Line>, Box<dyn std::error::Error>> {
    lines(&path.hatch(hatch, avoid)?)
}

#[test]
fn parallel_lines_are_one_spacing_apart_and_span_the_region() -> Outcome {
    let result = made(&rect(400.0, 300.0)?, &Hatch::new(10.0).flow(Flow::angle(0.0)?), None)?;
    let mut heights: Vec<f64> = result
        .iter()
        .map(|(points, _)| {
            assert!(
                points.iter().all(|p| (p.1 - points[0].1).abs() < 1e-9),
                "a line is straight"
            );
            points[0].1
        })
        .collect();
    heights.sort_by(f64::total_cmp);
    assert!(heights.len() >= 29, "{} lines", heights.len());
    for pair in heights.windows(2) {
        assert!((pair[1] - pair[0] - 10.0).abs() < 1e-6, "gap {}", pair[1] - pair[0]);
    }
    for (points, closed) in &result {
        assert!(!closed);
        let (first, last) = (points[0].0, points[points.len() - 1].0);
        assert!(
            first.min(last) < 3.0 && first.max(last) > 397.0,
            "from {first} to {last}"
        );
    }
    Ok(())
}

#[test]
fn spacing_follows_the_tone() -> Outcome {
    let tone = Field::ramp((0.0, 0.0), (400.0, 0.0))?;
    let hatch = Hatch::between(6.0, 18.0, tone).flow(Flow::angle(90.0)?);
    let result = made(&rect(400.0, 300.0)?, &hatch, None)?;
    let mut xs: Vec<f64> = result.iter().map(|(points, _)| points[0].0).collect();
    xs.sort_by(f64::total_cmp);
    let spacing = |x: f64| 6.0 + 12.0 * (x / 400.0);
    for pair in xs.windows(2) {
        let gap = pair[1] - pair[0];
        assert!(
            gap >= spacing(pair[0]) - 1e-6 && gap <= spacing(pair[1]) + 1e-6,
            "gap {gap} at {}",
            pair[0]
        );
    }
    assert!(xs[1] - xs[0] < 8.0 && xs[xs.len() - 1] - xs[xs.len() - 2] > 16.0);
    Ok(())
}

#[test]
fn no_two_lines_come_closer_than_half_the_spacing_and_none_leave_a_gap() -> Outcome {
    let spacing = 12.0;
    let flow = Flow::noise(250.0, 0.0, 3.0)?;
    let result = made(&rect(300.0, 300.0)?, &Hatch::new(spacing).flow(flow), None)?;
    assert!(result.len() > 10);
    let mut nearest = f64::MAX;
    for (a, (line_a, _)) in result.iter().enumerate() {
        for (line_b, _) in &result[a + 1..] {
            for p in line_a {
                for q in line_b {
                    nearest = nearest.min(libm::hypot(p.0 - q.0, p.1 - q.1));
                }
            }
        }
    }
    assert!(nearest >= 0.5 * spacing * 0.98, "two lines {nearest} apart");
    let mut worst = 0.0_f64;
    for i in 3..30 {
        for j in 3..30 {
            worst = worst.max(to_lines((f64::from(i) * 10.0, f64::from(j) * 10.0), &result));
        }
    }
    assert!(worst <= spacing, "a point {worst} from every line");
    Ok(())
}

#[test]
fn lines_stay_in_the_region_and_close_where_the_flow_circles() -> Outcome {
    let region = ring(150.0, 50.0)?;
    let flow = Flow::contour(&region)?;
    let result = made(&region, &Hatch::new(10.0).flow(flow), None)?;
    let closed: Vec<&Line> = result.iter().filter(|(_, closed)| *closed).collect();
    assert!(closed.len() >= 5, "{} closed of {}", closed.len(), result.len());
    for (points, _) in &result {
        for p in points {
            assert!(region.contains(*p)?, "{p:?} left the region");
        }
    }
    for (points, _) in closed {
        let radii: Vec<f64> = points.iter().map(|p| libm::hypot(p.0 - 200.0, p.1 - 200.0)).collect();
        let (low, high) = (
            radii.iter().copied().fold(f64::MAX, f64::min),
            radii.iter().copied().fold(0.0, f64::max),
        );
        assert!(high - low < 1.5, "a loop that is not round: {low} to {high}");
    }
    Ok(())
}

#[test]
fn lines_keep_out_of_what_they_avoid() -> Outcome {
    let hole = Path::circle((200.0, 150.0), 60.0)?;
    let result = made(
        &rect(400.0, 300.0)?,
        &Hatch::new(10.0).flow(Flow::angle(30.0)?),
        Some(&hole),
    )?;
    assert!(result.len() > 20);
    for (points, _) in &result {
        for p in points {
            assert!(!hole.contains(*p)?, "{p:?} is inside what is avoided");
        }
    }
    Ok(())
}

#[test]
fn refusals_and_the_empty_path() -> Outcome {
    let flow = Flow::angle(0.0)?;
    for spacing in [0.0, -3.0, f64::NAN] {
        assert!(parameter_refusal(rect(10.0, 10.0)?.hatch(&Hatch::new(spacing).flow(flow.clone()), None)).is_ok());
    }
    let tone = Field::constant(0.5)?;
    assert!(parameter_refusal(rect(10.0, 10.0)?.hatch(&Hatch::between(0.0, 5.0, tone), None)).is_ok());
    // far too many lines: refused before they are made, not after a long time
    let huge = rect(1e6, 1e6)?.hatch(&Hatch::new(1.0).flow(flow), None);
    assert!(parameter_refusal(huge).is_ok());
    assert_eq!(Path::new().hatch(&Hatch::new(5.0), None)?.to_svg_path(), "");
    Ok(())
}

#[test]
fn a_path_of_two_points_encloses_nothing() -> Outcome {
    for points in [
        vec![(0.0, 0.0), (100.0, 100.0)],
        vec![(0.0, 0.0), (100.0, 100.0), (0.0, 0.0)],
    ] {
        let line = Path::polyline(points, false)?;
        assert_eq!(line.hatch(&Hatch::new(50.0), None)?.to_svg_path(), "");
        let field = Field::distance(&line, false)?;
        assert!(field.at(bullterrier_vector::Point::new(10.0, 50.0)) > 0.0);
    }
    Ok(())
}

#[test]
fn a_very_wide_pair_of_spacings_is_refused_not_slow() -> Outcome {
    let tone = Field::ramp((0.0, 0.0), (400.0, 0.0))?;
    for wide in [1e6, 1e3] {
        let result = rect(400.0, 400.0)?.hatch(&Hatch::between(1.0, wide, tone.clone()), None);
        assert!(parameter_refusal(result).is_ok());
    }
    // the same ratio over a region the grid holds in a few cells is cheap, so it stays allowed
    assert!(rect(20.0, 20.0)?.hatch(&Hatch::between(1.0, 1e6, tone), None).is_ok());
    Ok(())
}

#[test]
fn a_shape_far_from_the_origin_is_refused_when_the_spacing_is_below_what_a_coordinate_resolves() -> Outcome {
    let far = Path::rect(1e17, 1e17, 1000.0, 1000.0, 0.0)?;
    assert!(parameter_refusal(far.hatch(&Hatch::new(1.0), None)).is_ok());
    // a spacing the coordinates do resolve works there
    assert_ne!(far.hatch(&Hatch::new(400.0), None)?.to_svg_path(), "");
    Ok(())
}

#[test]
fn lines_stay_where_the_tone_is_below_the_level() -> Outcome {
    let tone = Field::ramp((0.0, 0.0), (100.0, 0.0))?;
    let flow = Flow::angle(45.0)?;
    let all = made(&rect(100.0, 100.0)?, &Hatch::new(4.0).flow(flow.clone()), None)?;
    let dark = made(
        &rect(100.0, 100.0)?,
        &Hatch::between(4.0, 4.0, tone.clone()).flow(flow).below(0.3),
        None,
    )?;
    assert!(!dark.is_empty() && dark.len() < all.len());
    let mut farthest = 0.0_f64;
    for (points, _) in &dark {
        for p in points {
            assert!(
                tone.at(*p) <= 0.3 + 1e-12,
                "a line point at {p:?} reads {}",
                tone.at(*p)
            );
            farthest = farthest.max(p.0);
        }
    }
    // and they reach the level: a step is a quarter of the spacing, so within a spacing of 30
    assert!(
        farthest > 29.0 - 1.0,
        "lines stop at {farthest}, short of the level at 30"
    );
    Ok(())
}

#[test]
fn a_level_above_the_tone_everywhere_changes_nothing() -> Outcome {
    let tone = Field::constant(0.4)?;
    let plain = Hatch::between(5.0, 5.0, tone.clone());
    let above = plain.clone().below(0.5);
    let shape = rect(80.0, 60.0)?;
    assert_eq!(
        shape.hatch(&plain, None)?.to_svg_path(),
        shape.hatch(&above, None)?.to_svg_path()
    );
    // and a level below the tone everywhere draws nothing
    let none = shape.hatch(&plain.below(0.3), None)?;
    assert_eq!(none.to_svg_path(), "");
    Ok(())
}

#[test]
fn a_level_needs_a_tone_and_a_finite_number() -> Outcome {
    let shape = rect(80.0, 60.0)?;
    let tone = Field::constant(0.4)?;
    assert_eq!(
        parameter_refusal(shape.hatch(&Hatch::new(5.0).below(0.5), None))?.0,
        "below"
    );
    assert_eq!(
        parameter_refusal(shape.hatch(&Hatch::between(5.0, 5.0, tone).below(f64::NAN), None))?.0,
        "below"
    );
    Ok(())
}

#[test]
fn a_line_across_a_small_shape_is_kept_where_a_stub_against_other_lines_is_not() -> Outcome {
    // spacing 5 over a square 10 wide: each line is a chord of the shape, under three spacings long and kept
    let result = made(&rect(10.0, 10.0)?, &Hatch::new(5.0).flow(Flow::angle(0.0)?), None)?;
    assert_eq!(result.len(), 2, "{result:?}");
    Ok(())
}

#[test]
fn parallel_lines_start_half_a_spacing_below_the_shapes_own_top() -> Outcome {
    // a box from y = 3: the first line starts half a spacing below its top, so the lines are at 8, 18, .. not 5, 15
    let region = Path::rect(0.0, 3.0, 400.0, 300.0, 0.0)?;
    let result = made(&region, &Hatch::new(10.0).flow(Flow::angle(0.0)?), None)?;
    assert!(result.len() >= 29, "{} lines", result.len());
    for (points, _) in &result {
        let height = points[0].1;
        assert!(
            ((height - 8.0) / 10.0 - ((height - 8.0) / 10.0).round()).abs() < 1e-6,
            "a line at {height}"
        );
    }
    Ok(())
}
