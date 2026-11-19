//! `Drawing::trace_levels` and `Drawing::trace_labels` through the public API. Oracles: analytic maps (a ramp, a radial
//! cone, a soft disc) whose level sets are known, regions counted from the bytes, hand-built maps of bars and squares for
//! open, close and the area limits, and `Path::contains` / `Path::intersect` on the result.

#![expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::float_cmp,
    reason = "map sizes and tones are small, a corner of a cell map is an exact multiple, and a proptest body has no Result to return"
)]

use crate::common::region::{net_area, sampled};
use crate::common::{Outcome, clearance, parameter_refusal};
use bullterrier_vector::{Drawing, Error, Field, Path, Rect, Shape, Trace, Values};

fn field(width: usize, height: usize, bytes: &[u8]) -> Result<Field, Error> {
    Field::map(width, height, bytes, None, None)
}

fn levels(map: &Field, levels: &[f64], trace: &Trace) -> Result<Vec<Shape>, Error> {
    Drawing::trace_levels(map, levels.iter().copied(), trace).shapes(&Values::new())
}

fn labels(map: &Field, trace: &Trace) -> Result<Vec<Shape>, Error> {
    Drawing::trace_labels(map, 32, trace).shapes(&Values::new())
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
    let found: Vec<u32> = shapes.iter().filter_map(|s| s.attributes().label()).collect();
    // the commonest label (2) is the base and comes first, the others in order of the byte
    assert_eq!(found, vec![2, 0, 7]);
    let areas: Vec<f64> = shapes
        .iter()
        .map(|s| inside(s.path(), 40.0, 20.0))
        .collect::<Result<_, _>>()?;
    for (area, expected) in areas.iter().zip([400.0, 200.0, 200.0]) {
        assert!((area - expected).abs() < 0.02 * expected, "{area} against {expected}");
    }
    assert!((areas.iter().sum::<f64>() - 800.0).abs() < 0.01 * 800.0);
    // a label at the border goes on past it
    assert!(shapes[0].path().bounds()?.x < 0.0);
    Ok(())
}

/// Four labels in blobs and waves, 64 by 64: places where three labels meet, thin parts and small islands.
fn blobs(kind: usize) -> Vec<u8> {
    made(64, 64, |x, y| {
        let (x, y) = (x as f64, y as f64);
        let v = match kind {
            0 => libm::sin(x / 5.0) + libm::cos(y / 7.0) + libm::sin(x * y / 300.0),
            1 => (y + 6.0 * libm::sin(x / 9.0)) / 11.0,
            _ => libm::sin(libm::hypot(x - 20.0, y - 30.0) / 6.0) + libm::cos(libm::hypot(x - 45.0, y - 15.0) / 4.0),
        };
        (v * 1.7).floor().rem_euclid(4.0) as u8 * 3
    })
}

#[test]
fn labels_tile_the_frame_whatever_the_tidying() -> Outcome {
    let tidyings = [
        raw(),
        Trace::new(),
        Trace::new().smooth(2.0),
        Trace::new().close(2.0),
        Trace::new().open(2.0),
        Trace::new()
            .smooth(1.0)
            .open(1.5)
            .close(1.5)
            .min_area(6.0)
            .min_hole(6.0),
    ];
    for kind in 0..3 {
        let map = field(64, 64, &blobs(kind))?;
        for (n, trace) in tidyings.iter().enumerate() {
            let shapes = labels(&map, trace)?;
            let covered: f64 = shapes
                .iter()
                .map(|s| inside(s.path(), 64.0, 64.0))
                .sum::<Result<f64, _>>()?;
            assert!(
                (covered - 4096.0).abs() < 0.05,
                "map {kind}, tidying {n}: labels cover {covered} of 4096"
            );
            for (i, a) in shapes.iter().enumerate() {
                for b in &shapes[i + 1..] {
                    let common = a.path().intersect(b.path(), 0.001)?;
                    let both = net_area(&common, 1)?.abs();
                    assert!(
                        both < 0.05,
                        "map {kind}, tidying {n}: labels {:?} and {:?} overlap by {both} at {:?}: {}",
                        a.attributes().label(),
                        b.attributes().label(),
                        common.bounds()?,
                        common.to_svg_path()
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn untidied_labels_still_match_the_map_cell_by_cell() -> Outcome {
    let bytes = blobs(0);
    let shapes = labels(&field(64, 64, &bytes)?, &raw())?;
    for (index, &byte) in bytes.iter().enumerate() {
        let centre = ((index % 64) as f64 + 0.5, (index / 64) as f64 + 0.5);
        for shape in &shapes {
            let label = shape.attributes().label().ok_or("no label")?;
            assert_eq!(shape.path().contains(centre)?, label == u32::from(byte), "cell {index}");
        }
    }
    Ok(())
}

#[test]
fn a_big_map_traces_and_its_refusals_name_what_is_too_big() -> Outcome {
    let side = 3000;
    let ramp = made(side, side, |x, _| (x * 255 / side) as u8);
    let map = field(side, side, &ramp)?;
    assert_eq!(
        levels(&map, &[0.5], &Trace::new().smooth(2.0).open(2.0).close(2.0))?.len(),
        1
    );
    let halves = made(side, side, |x, _| u8::from(x > side / 2));
    let shapes = labels(&field(side, side, &halves)?, &Trace::new().close(2.0))?;
    assert_eq!(shapes.len(), 2);
    // the padding is what would make the lattice too large: the refusal names the number that sets it
    for (trace, name) in [
        (Trace::new().open(1e6), "open"),
        (Trace::new().close(1e6), "close"),
        (Trace::new().tolerance(1e6), "tolerance"),
    ] {
        let (argument, message) = parameter_refusal(levels(&map, &[0.5], &trace))?;
        assert!(
            argument.contains(name) && message.contains("nodes"),
            "{name}: {argument}: {message}"
        );
    }
    Ok(())
}

#[test]
fn cutting_many_labels_apart_past_the_budget_is_refused_naming_max_colors() -> Outcome {
    // a noisy map of 64 labels: every pixel edge is an outline edge, and each label is cut by all those after it
    let bytes = made(300, 300, |x, y| {
        u8::try_from((x * 31 + y * 17 + (x * y) % 13) % 64).unwrap_or(0)
    });
    let (argument, message) =
        parameter_refusal(Drawing::trace_labels(&field(300, 300, &bytes)?, 64, &raw()).shapes(&Values::new()))?;
    assert!(
        argument.contains("max_colors") && message.contains("segments"),
        "{argument}: {message}"
    );
    Ok(())
}

#[test]
fn a_level_can_be_a_param_in_code_as_in_a_document() -> Outcome {
    use bullterrier_vector::{Expr, Param};
    let level = Param::new("level", 0.25, 0.0, 1.0)?;
    let bytes = made(256, 4, |x, _| u8::try_from(x).unwrap_or(255));
    let map = field(256, 4, &bytes)?;
    let drawing = Drawing::trace_levels(&map, [Expr::from(&level), Expr::from(0.75)], &raw());
    assert_eq!(drawing.error(), None);
    for (value, found) in [(0.25, 0.25), (0.5, 0.5)] {
        let shapes = drawing.shapes(&Values::new().set("level", value))?;
        assert_eq!(shapes[0].attributes().level(), Some(found));
        assert_eq!(shapes.len(), 2);
    }
    Ok(())
}

#[test]
fn too_many_labels_and_hostile_numbers_are_refused_naming_the_argument() -> Outcome {
    let named = |outcome: Result<Vec<Shape>, Error>| parameter_refusal(outcome).map(|(name, _)| name);
    let many: Vec<u8> = (0..64).collect();
    let map = field(8, 8, &many)?;
    assert!(named(Drawing::trace_labels(&map, 10, &Trace::new()).shapes(&Values::new()))?.contains("max_colors"));
    assert!(
        Drawing::trace_labels(&map, 64, &Trace::new())
            .shapes(&Values::new())
            .is_ok()
    );
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
    // a field that is not a map has no box of its own to be read over
    assert!(named(levels(&Field::constant(0.5)?, &[0.5], &Trace::new()))?.contains("box"));
    assert!(named(Field::map(0, 5, &[], None, None).map(|_| Vec::new()))?.contains("width"));
    Ok(())
}

#[test]
fn a_huge_map_with_many_levels_or_a_wide_radius_is_refused_before_the_work() -> Outcome {
    let side = 1900;
    let map = field(side, side, &vec![128; side * side])?;
    let many: Vec<f64> = (0..200).map(|i| f64::from(i) / 200.0).collect();
    let (argument, message) = parameter_refusal(levels(&map, &many, &Trace::new()))?;
    assert!(
        argument.contains("levels") && message.contains("steps"),
        "{argument}: {message}"
    );
    let (argument, _) = parameter_refusal(levels(&map, &[0.5], &Trace::new().open(1e6)))?;
    assert!(argument.contains("open"));
    let (argument, _) = parameter_refusal(levels(&map, &[0.5], &Trace::new().smooth(5000.0)))?;
    assert!(argument.contains("smooth"));
    Ok(())
}

// Documents ---------------------------------------------------------------------------------------------------------

#[test]
fn the_three_calls_are_documents_that_draw_what_the_code_draws_and_write_back_the_same() -> Outcome {
    use bullterrier_vector::Inputs;
    let bytes: Vec<u8> = (0..64).map(|i| u8::try_from(i * 4 % 256).unwrap_or(0)).collect();
    let map = Field::map(8, 8, &bytes, None, Some("photo"))?;
    let drawings = [
        Drawing::trace_levels(&map, [0.25, 0.75], &Trace::new().smooth(0.5).open(0.25).min_area(2.0)),
        Drawing::trace_labels(&map, 64, &Trace::new().tolerance(0.0)),
        Drawing::trace_cells(&map, 1, 64),
    ];
    for drawing in drawings {
        let text = drawing.to_json()?;
        let back = Drawing::from_json(&text, &Inputs::new().set("photo", &bytes[..]))?;
        assert_eq!(back.shapes(&Values::new())?, drawing.shapes(&Values::new())?, "{text}");
        assert_eq!(back.to_json()?, text);
    }
    Ok(())
}

// Any field ------------------------------------------------------------------------------------------------------------

fn framed(field: &Field, levels: &[f64], frame: Rect, cell: f64) -> Result<Vec<Shape>, Error> {
    Drawing::trace_levels(field, levels.iter().copied(), &raw().frame(frame).cell(cell)).shapes(&Values::new())
}

const BOX: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 200.0,
    height: 200.0,
};

#[test]
fn a_field_that_is_no_map_is_traced_over_its_box_at_its_level() -> Outcome {
    // minus the distance to a circle of radius 60 is at least 15 inside the circle of radius 45, the oracle
    let inside = Field::distance(&Path::circle((100.0, 100.0), 60.0)?, false)?.mul(&Field::constant(-1.0)?)?;
    let shapes = framed(&inside, &[15.0, 500.0], BOX, 2.0)?;
    assert_eq!(shapes.len(), 1, "a level the field never reaches gives no shape");
    assert_eq!(shapes[0].attributes().level(), Some(15.0), "the level as given");
    let path = shapes[0].path();
    for step in 0..36 {
        let angle = (f64::from(step) * 10.0).to_radians();
        let (sin, cos) = (libm::sin(angle), libm::cos(angle));
        assert!(
            path.contains((100.0 + 43.0 * cos, 100.0 + 43.0 * sin))?,
            "inside at {step}"
        );
        assert!(
            !path.contains((100.0 + 47.0 * cos, 100.0 + 47.0 * sin))?,
            "outside at {step}"
        );
    }
    Ok(())
}

#[test]
fn a_map_read_over_its_own_box_and_pixels_is_the_map_left_alone() -> Outcome {
    let bytes: Vec<u8> = (0..64_u32).filter_map(|i| u8::try_from((i * 37) % 251).ok()).collect();
    let map = Field::map(
        8,
        8,
        &bytes,
        Some(Rect {
            x: 10.0,
            y: 20.0,
            width: 80.0,
            height: 40.0,
        }),
        None,
    )?;
    let svg = |shapes: Vec<Shape>| shapes.iter().map(|s| s.path().to_svg_path()).collect::<Vec<_>>();
    let alone = svg(Drawing::trace_levels(&map, [0.3, 0.6], &Trace::new()).shapes(&Values::new())?);
    let frame = Rect {
        x: 10.0,
        y: 20.0,
        width: 80.0,
        height: 40.0,
    };
    let given = svg(Drawing::trace_levels(&map, [0.3, 0.6], &Trace::new().frame(frame)).shapes(&Values::new())?);
    assert_eq!(alone, given);
    // read through `at` on a finer grid, it is a field like any other
    assert_eq!(framed(&map, &[0.3, 0.6], frame, 2.5)?.len(), 2);
    Ok(())
}

#[test]
fn a_field_that_is_no_map_needs_a_box_and_a_cell() -> Outcome {
    let ramp = Field::ramp((0.0, 0.0), (100.0, 0.0))?;
    let named = |trace: &Trace| -> Result<String, Box<dyn std::error::Error>> {
        Ok(parameter_refusal(Drawing::trace_levels(&ramp, [0.5], trace).shapes(&Values::new()))?.0)
    };
    assert!(named(&raw().cell(2.0))?.ends_with("box"));
    assert!(named(&raw().frame(BOX))?.ends_with("cell"));
    assert!(
        named(&raw().frame(BOX).cell(0.01))?.ends_with("cell"),
        "a grid past 2^24 cells"
    );
    // labels are read at a map's own pixels: a frame or a cell is refused where the call is made
    let map = field(2, 2, &[0, 1, 1, 0])?;
    for (trace, name) in [(raw().frame(BOX), "box"), (raw().cell(2.0), "cell")] {
        let drawing = Drawing::trace_labels(&map, 8, &trace);
        assert!(
            drawing.error().is_some_and(|error| error.to_string().contains(name)),
            "{name}"
        );
    }
    assert_eq!(framed(&ramp, &[0.5], BOX, 2.0)?.len(), 1);
    Ok(())
}

#[test]
fn a_param_of_the_field_is_read_with_the_values_of_the_draw() -> Outcome {
    let wavelength = bullterrier_vector::Param::new("wavelength", 40.0, 10.0, 80.0)?;
    let svg = |field: &Field, values: &Values| -> Result<String, Error> {
        let drawing = Drawing::trace_levels(field, [0.0], &raw().frame(BOX).cell(4.0));
        Ok(drawing.shapes(values)?.iter().map(|s| s.path().to_svg_path()).collect())
    };
    let read = Field::noise(&wavelength, 0.0, 5.0, 2)?;
    for value in [40.0, 25.0] {
        let given = Field::noise(value, 0.0, 5.0, 2)?;
        assert_eq!(
            svg(&read, &Values::new().set("wavelength", value))?,
            svg(&given, &Values::new())?
        );
    }
    Ok(())
}
