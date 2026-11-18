//! `marks` through the public API. Oracles: the corners, arms and ends worked out by hand from the rules of the
//! documentation (a bracket arm is `bracket` times the smaller side; a crosshair reaches `cross` either way; a leader
//! runs `cross + label_size` up and right), path data written out, and the numbers of the bundled font read with
//! fontTools (`H` inked from x 93 to 507 and y 0 to 730 of 1000 units, so the middle of a capital is 365 up).

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{BoxMark, Drawing, Font, Inputs, Marks, PointMark, Select, Shape, Values, marks};

fn mono() -> Result<Font, bullterrier_vector::Error> {
    Font::bundled("jetbrains_mono")
}

fn drawn(options: Marks) -> Result<Vec<Shape>, Box<dyn std::error::Error>> {
    Ok(marks(options).shapes(&Values::new())?)
}

fn role(shape: &Shape) -> &str {
    shape.attributes().role().unwrap_or("")
}

fn data(shapes: &[Shape], wanted: &str) -> Vec<String> {
    shapes
        .iter()
        .filter(|shape| role(shape) == wanted)
        .map(|shape| shape.path().to_svg_path())
        .collect()
}

#[test]
fn a_box_has_a_bracket_of_arms_a_quarter_of_its_smaller_side_in_each_corner() -> Outcome {
    let shapes = drawn(Marks::new(&mono()?).boxes([BoxMark::new(40.0, 60.0, 120.0, 80.0)]))?;
    // arm: 0.25 * min(120, 80) = 20
    assert_eq!(
        data(&shapes, "bracket"),
        [
            "M60 60 L40 60 L40 80",
            "M140 60 L160 60 L160 80",
            "M140 140 L160 140 L160 120",
            "M60 140 L40 140 L40 120"
        ]
    );
    assert_eq!(shapes.len(), 4, "no label, no leader");
    assert!(shapes.iter().all(|shape| shape.attributes().mark() == Some(0)));
    Ok(())
}

#[test]
fn the_arm_follows_the_bracket_argument() -> Outcome {
    let shapes = drawn(
        Marks::new(&mono()?)
            .boxes([BoxMark::new(0.0, 0.0, 100.0, 40.0)])
            .bracket(0.5),
    )?;
    assert_eq!(data(&shapes, "bracket")[0], "M20 0 L0 0 L0 20");
    Ok(())
}

#[test]
fn a_point_has_a_crosshair_reaching_the_cross_either_way() -> Outcome {
    let shapes = drawn(Marks::new(&mono()?).points([PointMark::new(200.0, 90.0)]).cross(10.0))?;
    assert_eq!(data(&shapes, "cross"), ["M190 90 L210 90", "M200 80 L200 100"]);
    assert_eq!(shapes.len(), 2);
    Ok(())
}

#[test]
fn a_label_stands_at_the_end_of_a_leader_with_a_capital_centred_on_its_height() -> Outcome {
    let shapes = drawn(
        Marks::new(&mono()?)
            .points([PointMark::new(200.0, 90.0).label("H")])
            .cross(10.0)
            .label_size(12.0),
    )?;
    // the leader runs 10 + 12 = 22 up and right: from (200, 90) to (222, 68)
    assert_eq!(data(&shapes, "leader"), ["M200 90 L222 68"]);
    let label = shapes.iter().find(|shape| role(shape) == "label").ok_or("a label")?;
    let bounds = label.path().bounds()?;
    // the pen stands a quarter em (3) right of the end; H is inked 93 to 507 units of 12 / 1000
    let near = |left: f64, right: f64| (left - right).abs() < 1e-6;
    assert!(near(bounds.x, 225.0 + 0.093 * 12.0), "{bounds:?}");
    assert!(near(bounds.x + bounds.width, 225.0 + 0.507 * 12.0), "{bounds:?}");
    // 730 units of 12 high, centred on y = 68
    assert!(near(bounds.y + bounds.height / 2.0, 68.0), "{bounds:?}");
    assert!(near(bounds.height, 0.730 * 12.0), "{bounds:?}");
    assert_eq!(label.attributes().char(), Some('H'));
    Ok(())
}

#[test]
fn a_boxs_label_lies_right_of_its_own_box_and_a_points_beyond_its_crosshair() -> Outcome {
    let box_shapes = drawn(
        Marks::new(&mono()?)
            .boxes([BoxMark::new(40.0, 60.0, 120.0, 80.0).label("EAR L")])
            .cross(6.0)
            .label_size(14.0),
    )?;
    assert_eq!(data(&box_shapes, "leader"), ["M160 60 L180 40"]);
    for glyph in box_shapes.iter().filter(|shape| role(shape) == "label") {
        assert!(glyph.path().bounds()?.x > 160.0, "right of the box");
    }
    let point_shapes = drawn(
        Marks::new(&mono()?)
            .points([PointMark::new(0.0, 0.0).label("NOSE")])
            .cross(30.0)
            .label_size(8.0),
    )?;
    for glyph in point_shapes.iter().filter(|shape| role(shape) == "label") {
        assert!(glyph.path().bounds()?.x > 30.0, "beyond the arm of the cross");
    }
    Ok(())
}

#[test]
fn every_shape_names_its_role_and_its_mark_and_none_is_closed() -> Outcome {
    let shapes = drawn(
        Marks::new(&mono()?)
            .boxes([BoxMark::new(0.0, 0.0, 50.0, 50.0).label("A")])
            .points([PointMark::new(80.0, 80.0).label("B"), PointMark::new(10.0, 90.0)]),
    )?;
    let found: Vec<(&str, u32)> = shapes
        .iter()
        .map(|shape| (role(shape), shape.attributes().mark().unwrap_or(99)))
        .collect();
    assert_eq!(
        found,
        [
            ("bracket", 0),
            ("bracket", 0),
            ("bracket", 0),
            ("bracket", 0),
            ("leader", 0),
            ("label", 0),
            ("cross", 1),
            ("cross", 1),
            ("leader", 1),
            ("label", 1),
            ("cross", 2),
            ("cross", 2)
        ]
    );
    for shape in shapes.iter().filter(|shape| role(shape) != "label") {
        assert!(!shape.path().to_svg_path().contains('Z'), "open lines for a plotter");
    }
    // styling by role: only the labels
    let labels = marks(Marks::new(&mono()?).points([PointMark::new(1.0, 1.0).label("A")]))
        .only(Select::new().attribute("role", "label"))
        .shapes(&Values::new())?;
    assert_eq!(labels.len(), 1);
    Ok(())
}

#[test]
fn nothing_to_mark_is_an_empty_drawing_and_an_empty_label_is_no_label() -> Outcome {
    assert_eq!(drawn(Marks::new(&mono()?))?.len(), 0);
    let shapes = drawn(Marks::new(&mono()?).points([PointMark::new(1.0, 1.0).label("")]))?;
    assert_eq!(shapes.len(), 2);
    Ok(())
}

#[test]
fn a_very_long_label_is_one_shape_per_glyph() -> Outcome {
    let label = "W".repeat(2_000);
    let shapes = drawn(Marks::new(&mono()?).points([PointMark::new(1.0, 1.0).label(label)]))?;
    assert_eq!(shapes.iter().filter(|shape| role(shape) == "label").count(), 2_000);
    Ok(())
}

#[test]
fn what_the_rules_forbid_is_refused_naming_it() -> Outcome {
    let font = mono()?;
    let nan = f64::NAN;
    for (case, options, named) in [
        (
            "a bracket of 0.6",
            Marks::new(&font).boxes([BoxMark::new(0.0, 0.0, 9.0, 9.0)]).bracket(0.6),
            "bracket",
        ),
        ("a bracket of 0", Marks::new(&font).bracket(0.0), "bracket"),
        ("a cross of 0", Marks::new(&font).cross(0.0), "cross"),
        ("a label size of -1", Marks::new(&font).label_size(-1.0), "label_size"),
        (
            "a box of no width",
            Marks::new(&font).boxes([BoxMark::new(0.0, 0.0, 0.0, 9.0)]),
            "width",
        ),
        (
            "a NaN corner",
            Marks::new(&font).boxes([BoxMark::new(nan, 0.0, 9.0, 9.0)]),
            "x",
        ),
        (
            "an infinite point",
            Marks::new(&font).points([PointMark::new(0.0, f64::INFINITY)]),
            "y",
        ),
        (
            "a tab in a label",
            Marks::new(&font).points([PointMark::new(0.0, 0.0).label("a\tb")]),
            "label",
        ),
        (
            "a line break in a label",
            Marks::new(&font).points([PointMark::new(0.0, 0.0).label("a\nb")]),
            "label",
        ),
    ] {
        let (parameter, message) = parameter_refusal(marks(options).shapes(&Values::new()))?;
        assert!(
            parameter.contains(named) || message.contains(named),
            "{case}: {parameter}: {message}"
        );
    }
    Ok(())
}

#[test]
fn more_marks_than_the_limit_are_refused_before_anything_is_drawn() -> Outcome {
    let points = (0..10_001).map(|at| PointMark::new(f64::from(at), 0.0));
    let (_, message) = parameter_refusal(marks(Marks::new(&mono()?).points(points)).shapes(&Values::new()))?;
    assert!(message.contains("MAX_MARKS"), "{message}");
    Ok(())
}

#[test]
fn a_corner_so_far_out_that_a_bracket_leaves_the_coordinates_is_refused() -> Outcome {
    let drawing = marks(Marks::new(&mono()?).boxes([BoxMark::new(1e100, 0.0, 1e100, 1.0)]));
    parameter_refusal(drawing.shapes(&Values::new()))?;
    Ok(())
}

#[test]
fn marks_save_to_json_and_load_the_same_drawing() -> Outcome {
    let drawing = marks(
        Marks::new(&mono()?)
            .boxes([BoxMark::new(40.0, 60.0, 120.0, 80.0).label("EAR L")])
            .points([PointMark::new(200.0, 90.0)])
            .bracket(0.3)
            .cross(9.0)
            .label_size(10.0),
    );
    let json = drawing.to_json()?;
    let loaded = Drawing::from_json(&json, &Inputs::new())?;
    assert_eq!(loaded.to_json()?, json);
    assert_eq!(drawing.shapes(&Values::new())?, loaded.shapes(&Values::new())?);
    Ok(())
}
