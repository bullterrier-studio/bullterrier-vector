//! `Drawing::trace_levels` and `Drawing::trace_labels` through the public API. Oracles: analytic maps (a ramp, a radial
//! cone, a soft disc) whose level sets are known, regions counted from the bytes, hand-built maps of bars and squares for
//! open, close and the area limits, and `Path::contains` / `Path::intersect` on the result.

use crate::common::region::{net_area, sampled};
use crate::common::{Outcome, clearance, parameter_refusal};
use bullterrier_vector::{Drawing, Error, Field, Path, Shape, Trace, Values};

fn field(width: usize, height: usize, bytes: &[u8]) -> Result<Field, Error> {
    Field::map(width, height, bytes, None, None)
}

fn levels(map: &Field, levels: &[f64], trace: &Trace) -> Result<Vec<Shape>, Error> {
    Drawing::trace_levels(map, levels, trace).shapes(&Values::new())
}

fn labels(map: &Field, trace: &Trace) -> Result<Vec<Shape>, Error> {
    Drawing::trace_labels(map, trace).shapes(&Values::new())
}

fn raw() -> Trace {
    Trace::new().tolerance(0.0)
}

/// The map as a frame-sized rectangle path.
fn frame(width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(0.0, 0.0, width, height, 0.0)
}

/// The area of `path` inside the map's frame.
fn inside(path: &Path, width: f64, height: f64) -> Result<f64, Box<dyn std::error::Error>> {
    net_area(&path.intersect(&frame(width, height)?, 0.001)?, 1)
}

/// A bytes map from a function of the pixel.
fn made(width: usize, height: usize, value: impl Fn(usize, usize) -> u8) -> Vec<u8> {
    (0..width * height).map(|i| value(i % width, i / width)).collect()
}

fn block(x: usize, y: usize, size: usize) -> impl Fn(usize, usize) -> bool {
    move |px, py| px >= x && px < x + size && py >= y && py < y + size
}

#[test]
fn the_level_of_a_ramp_is_the_region_past_a_straight_edge() -> Outcome {
    let bytes = made(256, 4, |x, _| u8::try_from(x).unwrap_or(255));
    let shapes = levels(&field(256, 4, &bytes)?, &[0.5], &raw())?;
    assert_eq!(shapes.len(), 1);
    // bytes 128.. are at or above 0.5; the edge falls half way between pixel 127 (0.498) and 128 (0.502): x = 128
    let area = inside(shapes[0].path(), 256.0, 4.0)?;
    assert!((area - 128.0 * 4.0).abs() < 1e-6, "{area}");
    Ok(())
}

#[test]
fn a_disc_cut_by_the_edge_has_no_edge_along_the_border() -> Outcome {
    // a soft disc of radius 30 centred on the left border of a 100 by 100 map
    let bytes = made(100, 100, |x, y| {
        let d = libm::hypot(x as f64 + 0.5, y as f64 + 0.5 - 50.0);
        (255.0 * (0.5 + (30.0 - d) / 4.0).clamp(0.0, 1.0)).round() as u8
    });
    let shapes = levels(&field(100, 100, &bytes)?, &[0.5], &raw())?;
    let path = shapes[0].path();
    let half_disc = std::f64::consts::PI * 900.0 / 2.0;
    let area = inside(path, 100.0, 100.0)?;
    assert!(
        (area - half_disc).abs() < 0.01 * half_disc,
        "{area} against {half_disc}"
    );
    let rings = sampled(path, 1)?;
    assert_eq!(rings.len(), 1);
    let along_border = |a: &(f64, f64), b: &(f64, f64)| {
        (a.0 == b.0 && (a.0 == 0.0 || a.0 == 100.0)) || (a.1 == b.1 && (a.1 == 0.0 || a.1 == 100.0))
    };
    for (index, from) in rings[0].iter().enumerate() {
        let to = &rings[0][(index + 1) % rings[0].len()];
        assert!(!along_border(from, to), "an edge along the border: {from:?} to {to:?}");
    }
    let bounds = path.bounds()?;
    assert!(
        bounds.x < 0.0,
        "the shape goes on past the border and closes outside: {bounds:?}"
    );
    // the same holds at the far side and the corners: a map that is all high is a rectangle past all four sides
    let full = levels(&field(8, 8, &[255; 64])?, &[0.5], &raw())?;
    let b = full[0].path().bounds()?;
    assert!(
        b.x < 0.0 && b.y < 0.0 && b.x + b.width > 8.0 && b.y + b.height > 8.0,
        "{b:?}"
    );
    Ok(())
}

#[test]
fn nested_levels_of_a_cone_are_discs_of_the_hand_computed_radii() -> Outcome {
    let n = 400;
    let bytes = made(n, n, |x, y| {
        let d = libm::hypot(x as f64 + 0.5 - 200.0, y as f64 + 0.5 - 200.0);
        (255.0 * (1.0 - d / 200.0).clamp(0.0, 1.0)).round() as u8
    });
    let map = field(n, n, &bytes)?;
    let wanted = [0.25, 0.5, 0.75, 1e12, -1e12];
    let shapes = levels(&map, &wanted, &raw())?;
    // 1e12 is above every tone and is left out; -1e12 is below every tone (and the padding) and fills the frame
    let found: Vec<f64> = shapes.iter().filter_map(|s| s.attributes().level()).collect();
    assert_eq!(found, vec![0.25, 0.5, 0.75, -1e12]);
    for (shape, level) in shapes.iter().zip([0.25, 0.5, 0.75]) {
        let radius = 200.0 * (1.0 - level);
        let area = net_area(shape.path(), 1)?.abs();
        let disc = std::f64::consts::PI * radius * radius;
        assert!(
            (area - disc).abs() < 0.025 * disc,
            "level {level}: {area} against {disc}"
        );
    }
    let everything = inside(shapes[3].path(), 400.0, 400.0)?;
    assert!((everything - 160_000.0).abs() < 1e-6);
    // each level lies inside the one below it
    for pair in shapes.windows(2).take(2) {
        let leftover = net_area(&pair[1].path().subtract(pair[0].path(), 0.001)?, 1)?;
        assert!(leftover.abs() < 1.0, "{leftover}");
    }
    Ok(())
}

#[test]
fn fitted_curves_stay_within_the_tolerance_of_the_polyline() -> Outcome {
    let bytes = made(200, 200, |x, y| {
        let d = libm::hypot(x as f64 + 0.5 - 100.0, y as f64 + 0.5 - 100.0);
        (255.0 * (1.0 - d / 100.0).clamp(0.0, 1.0)).round() as u8
    });
    let map = field(200, 200, &bytes)?;
    let polyline = &levels(&map, &[0.4], &raw())?[0];
    let fitted = &levels(&map, &[0.4], &Trace::new().tolerance(0.3))?[0];
    let line = &sampled(polyline.path(), 1)?[0];
    let curve = &sampled(fitted.path(), 24)?[0];
    assert!(fitted.path().to_svg_path().contains('C'), "curves, not lines");
    assert!(fitted.path().to_svg_path().len() < polyline.path().to_svg_path().len() / 3);
    for p in curve {
        assert!(
            clearance(line, *p) <= 0.3 + 0.02,
            "a curve point {p:?} is {} from the polyline",
            clearance(line, *p)
        );
    }
    for p in line {
        assert!(
            clearance(curve, *p) <= 0.3 + 0.02,
            "a polyline point {p:?} is {} from the curve",
            clearance(curve, *p)
        );
    }
    Ok(())
}

/// A 60 by 30 binary map with a 20 by 20 square, a bar three pixels wide and two 8-pixel squares four pixels apart.
fn parts() -> Result<(Field, [(f64, f64); 4]), Error> {
    let square = block(2, 5, 20);
    let bar = |x: usize, y: usize| (25..28).contains(&x) && (2..28).contains(&y);
    let (a, b) = (block(35, 10, 8), block(47, 10, 8));
    let bytes = made(60, 30, |x, y| {
        if square(x, y) || bar(x, y) || a(x, y) || b(x, y) {
            255
        } else {
            0
        }
    });
    // the middles of the square, the bar and the gap between the small squares
    Ok((
        field(60, 30, &bytes)?,
        [(12.0, 15.0), (26.5, 15.0), (45.0, 14.0), (39.0, 14.0)],
    ))
}

#[test]
fn opening_removes_what_is_narrower_than_twice_the_radius_and_keeps_the_rest() -> Outcome {
    let (map, [square, bar, gap, small]) = parts()?;
    let at = |trace: &Trace| -> Result<Vec<bool>, Box<dyn std::error::Error>> {
        let shape = &levels(&map, &[0.5], trace)?[0];
        [square, bar, gap, small]
            .iter()
            .map(|p| Ok(shape.path().contains(*p)?))
            .collect()
    };
    assert_eq!(at(&raw())?, vec![true, true, false, true]);
    // a radius of 1 keeps a bar 3 wide (it is wider than 2), a radius of 2 removes it (3 < 4); an 8 square needs 4
    assert_eq!(at(&raw().open(1.0))?, vec![true, true, false, true]);
    assert_eq!(at(&raw().open(2.0))?, vec![true, false, false, true]);
    assert_eq!(at(&raw().open(4.5))?, vec![true, false, false, false]);
    let opened = &levels(&map, &[0.5], &raw().open(5.0))?[0];
    let area = net_area(opened.path(), 32)?;
    // the 20 square keeps its sides and loses its corners to a quarter disc of radius 5 each: 400 - 4 * (25 - 25 pi / 4)
    let expected = 400.0 - 4.0 * (25.0 - 25.0 * std::f64::consts::PI / 4.0);
    assert!((area - expected).abs() < 3.0, "{area} against {expected}");
    Ok(())
}

#[test]
fn closing_fills_what_is_narrower_than_twice_the_radius() -> Outcome {
    let (map, [_, _, gap, _]) = parts()?;
    let kept = |trace: &Trace| -> Result<bool, Box<dyn std::error::Error>> {
        Ok(levels(&map, &[0.5], trace)?[0].path().contains(gap)?)
    };
    assert!(!kept(&raw())?);
    // the squares are four pixels apart: a radius of 1.5 leaves the gap, 2.5 fills it
    assert!(!kept(&raw().close(1.5))?);
    assert!(kept(&raw().close(2.5))?);
    Ok(())
}

#[test]
fn small_holes_are_filled_and_small_parts_dropped_by_area() -> Outcome {
    // a 20 square with a 2 by 2 hole and a 6 by 6 hole, and a 3 by 3 speck and a 5 by 5 speck beside it
    let (hole_a, hole_b, speck_a, speck_b) = (block(4, 4, 2), block(12, 12, 6), block(24, 2, 3), block(24, 12, 5));
    let body = block(2, 2, 20);
    let bytes = made(32, 24, |x, y| {
        let high = (body(x, y) && !hole_a(x, y) && !hole_b(x, y)) || speck_a(x, y) || speck_b(x, y);
        if high { 255 } else { 0 }
    });
    let map = field(32, 24, &bytes)?;
    let parts = |trace: &Trace| -> Result<usize, Box<dyn std::error::Error>> {
        Ok(levels(&map, &[0.5], trace)?[0].path().split().len())
    };
    assert_eq!(parts(&raw())?, 3, "the body, two specks");
    assert_eq!(
        parts(&raw().min_area(10.0))?,
        2,
        "the 3 by 3 speck (area 8.5: tracing cuts its corners) goes, the 5 by 5 stays"
    );
    assert_eq!(parts(&raw().min_area(100.0))?, 1, "both specks go");
    let shape = |trace: &Trace| -> Result<Path, Box<dyn std::error::Error>> {
        Ok(levels(&map, &[0.5], trace)?[0].path().clone())
    };
    let point_in = |path: &Path, p: (f64, f64)| path.contains(p);
    let plain = shape(&raw())?;
    assert!(!point_in(&plain, (5.0, 5.0))? && !point_in(&plain, (15.0, 15.0))?);
    let filled = shape(&raw().min_hole(10.0))?;
    assert!(point_in(&filled, (5.0, 5.0))?, "the 4-pixel hole is filled");
    assert!(!point_in(&filled, (15.0, 15.0))?, "the 36-pixel hole stays");
    Ok(())
}

#[test]
fn labels_make_one_shape_each_that_together_fill_the_map() -> Outcome {
    // left half label 2, right half split: top label 7, bottom label 0
    let bytes = made(40, 20, |x, y| {
        if x < 20 {
            2
        } else if y < 10 {
            7
        } else {
            0
        }
    });
    let shapes = labels(&field(40, 20, &bytes)?, &raw())?;
    let found: Vec<f64> = shapes.iter().filter_map(|s| s.attributes().level()).collect();
    assert_eq!(found, vec![0.0, 2.0, 7.0]);
    let areas: Vec<f64> = shapes
        .iter()
        .map(|s| inside(s.path(), 40.0, 20.0))
        .collect::<Result<_, _>>()?;
    for (area, expected) in areas.iter().zip([200.0, 400.0, 200.0]) {
        assert!((area - expected).abs() < 0.02 * expected, "{area} against {expected}");
    }
    assert!((areas.iter().sum::<f64>() - 800.0).abs() < 0.01 * 800.0);
    // a label at the border goes on past it
    assert!(shapes[1].path().bounds()?.x < 0.0);
    Ok(())
}

#[test]
fn too_many_labels_and_hostile_numbers_are_refused_naming_the_argument() -> Outcome {
    let named = |outcome: Result<Vec<Shape>, Error>| parameter_refusal(outcome).map(|(name, _)| name);
    let many: Vec<u8> = (0..64).collect();
    let map = field(8, 8, &many)?;
    assert!(named(labels(&map, &Trace::new().max_colors(10)))?.contains("max_colors"));
    assert!(labels(&map, &Trace::new().max_colors(64)).is_ok());
    let tone = field(4, 4, &[100; 16])?;
    for (trace, argument) in [
        (Trace::new().smooth(f64::NAN), "smooth"),
        (Trace::new().tolerance(-1.0), "tolerance"),
        (Trace::new().open(f64::INFINITY), "open"),
        (Trace::new().close(-0.5), "close"),
        (Trace::new().min_area(f64::NAN), "min_area"),
        (Trace::new().min_hole(-2.0), "min_hole"),
        (Trace::new().open(1e101), "open"),
    ] {
        assert!(named(levels(&tone, &[0.5], &trace))?.contains(argument), "{argument}");
    }
    assert!(named(levels(&tone, &[f64::NAN], &Trace::new()))?.contains("levels"));
    assert!(named(levels(&tone, &[], &Trace::new()))?.contains("levels"));
    assert!(named(levels(&tone, &vec![0.5; 257], &Trace::new()))?.contains("levels"));
    assert!(named(levels(&Field::constant(0.5)?, &[0.5], &Trace::new()))?.contains("map"));
    assert!(named(Field::map(0, 5, &[], None, None).map(|_| Vec::new()))?.contains("width"));
    Ok(())
}

#[test]
fn a_huge_map_with_many_levels_or_a_wide_radius_is_refused_before_the_work() -> Outcome {
    let side = 1900;
    let map = field(side, side, &vec![128; side * side])?;
    let many: Vec<f64> = (0..200).map(|i| f64::from(i) / 200.0).collect();
    let started = std::time::Instant::now();
    let (argument, message) = parameter_refusal(levels(&map, &many, &Trace::new()))?;
    assert!(
        argument.contains("levels") && message.contains("steps"),
        "{argument}: {message}"
    );
    let (argument, _) = parameter_refusal(levels(&map, &[0.5], &Trace::new().open(1e6)))?;
    assert!(argument.contains("open"));
    let (argument, _) = parameter_refusal(levels(&map, &[0.5], &Trace::new().smooth(5000.0)))?;
    assert!(argument.contains("smooth"));
    assert!(started.elapsed().as_secs() < 10, "refusals are quick");
    Ok(())
}
