//! `Path::hatch` through the public API: evenly spaced streamlines inside a region. The oracles are measured on the
//! output: the gap between parallel lines against the spacing asked, the floor between any two lines by brute force,
//! containment by `Path::contains`, the radius of a closed loop.

use crate::common::{Line, lines, parameter_refusal, to_lines};
use bullterrier_vector::{Field, Flow, Hatch, Path};

fn rect(width: f64, height: f64) -> Path {
    Path::rect(0.0, 0.0, width, height, 0.0).unwrap()
}

fn ring(outer: f64, inner: f64) -> Path {
    let outer = Path::circle((200.0, 200.0), outer).unwrap();
    let hole = Path::circle((200.0, 200.0), inner).unwrap().reverse();
    Path::from_svg_path(&format!("{} {}", outer.to_svg_path(), hole.to_svg_path())).unwrap()
}

fn made(path: &Path, hatch: &Hatch, avoid: Option<&Path>) -> Vec<Line> {
    lines(&path.hatch(hatch, avoid).unwrap()).unwrap()
}

#[test]
fn parallel_lines_are_one_spacing_apart_and_span_the_region() {
    let result = made(&rect(400.0, 300.0), &Hatch::new(10.0).flow(Flow::angle(0.0).unwrap()), None);
    let mut heights: Vec<f64> = result
        .iter()
        .map(|(points, _)| {
            assert!(points.iter().all(|p| (p.1 - points[0].1).abs() < 1e-9), "a line is straight");
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
        assert!(first.min(last) < 3.0 && first.max(last) > 397.0, "from {first} to {last}");
    }
}

#[test]
fn spacing_follows_the_tone() {
    let tone = Field::ramp((0.0, 0.0), (400.0, 0.0)).unwrap();
    let hatch = Hatch::between(6.0, 18.0, tone).flow(Flow::angle(90.0).unwrap());
    let result = made(&rect(400.0, 300.0), &hatch, None);
    let mut xs: Vec<f64> = result.iter().map(|(points, _)| points[0].0).collect();
    xs.sort_by(f64::total_cmp);
    let spacing = |x: f64| 6.0 + 12.0 * (x / 400.0);
    for pair in xs.windows(2) {
        let gap = pair[1] - pair[0];
        assert!(gap >= spacing(pair[0]) - 1e-6 && gap <= spacing(pair[1]) + 1e-6, "gap {gap} at {}", pair[0]);
    }
    assert!(xs[1] - xs[0] < 8.0 && xs[xs.len() - 1] - xs[xs.len() - 2] > 16.0);
}

#[test]
fn no_two_lines_come_closer_than_half_the_spacing_and_none_leave_a_gap() {
    let spacing = 12.0;
    let flow = Flow::noise(250.0, 0.0, 3.0).unwrap();
    let result = made(&rect(300.0, 300.0), &Hatch::new(spacing).flow(flow), None);
    assert!(result.len() > 10);
    let mut nearest = f64::MAX;
    for (a, (line_a, _)) in result.iter().enumerate() {
        for (line_b, _) in &result[a + 1..] {
            for p in line_a {
                for q in line_b {
                    nearest = nearest.min((p.0 - q.0).hypot(p.1 - q.1));
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
}

#[test]
fn lines_stay_in_the_region_and_close_where_the_flow_circles() {
    let region = ring(150.0, 50.0);
    let flow = Flow::contour(&region).unwrap();
    let result = made(&region, &Hatch::new(10.0).flow(flow), None);
    let closed: Vec<&Line> = result.iter().filter(|(_, closed)| *closed).collect();
    assert!(closed.len() >= 5, "{} closed of {}", closed.len(), result.len());
    for (points, _) in &result {
        for p in points {
            assert!(region.contains(*p).unwrap(), "{p:?} left the region");
        }
    }
    for (points, _) in closed {
        let radii: Vec<f64> = points.iter().map(|p| (p.0 - 200.0).hypot(p.1 - 200.0)).collect();
        let (low, high) = (radii.iter().copied().fold(f64::MAX, f64::min), radii.iter().copied().fold(0.0, f64::max));
        assert!(high - low < 1.5, "a loop that is not round: {low} to {high}");
    }
}

#[test]
fn lines_keep_out_of_what_they_avoid() {
    let hole = Path::circle((200.0, 150.0), 60.0).unwrap();
    let result = made(&rect(400.0, 300.0), &Hatch::new(10.0).flow(Flow::angle(30.0).unwrap()), Some(&hole));
    assert!(result.len() > 20);
    for (points, _) in &result {
        for p in points {
            assert!(!hole.contains(*p).unwrap(), "{p:?} is inside what is avoided");
        }
    }
}

#[test]
fn refusals_and_the_empty_path() {
    let flow = Flow::angle(0.0).unwrap();
    for spacing in [0.0, -3.0, f64::NAN] {
        assert!(parameter_refusal(rect(10.0, 10.0).hatch(&Hatch::new(spacing).flow(flow.clone()), None)).is_ok());
    }
    let tone = Field::constant(0.5).unwrap();
    assert!(parameter_refusal(rect(10.0, 10.0).hatch(&Hatch::between(0.0, 5.0, tone), None)).is_ok());
    // far too many lines: refused before they are made, not after a long time
    let huge = rect(1e6, 1e6).hatch(&Hatch::new(1.0).flow(flow), None);
    assert!(parameter_refusal(huge).is_ok());
    assert_eq!(Path::new().hatch(&Hatch::new(5.0), None).unwrap().to_svg_path(), "");
}
