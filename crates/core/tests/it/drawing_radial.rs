//! `Drawing::twist` and `Drawing::bulge` through the public API. The maps are tested on `Path::twist` and
//! `Path::bulge`; here the steps: the usage first (a letter twisted about its center, a circle bulged and pinched, a
//! Param driving the angle), the pivot (the shape's `origin`, else the middle of its bounds, else `about`), the path's
//! result per shape, numbers per item and from Params, style and attributes kept, saving and loading, and refusals that
//! name the step.

use crate::common::fonts::fraunces;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Bulge, Color, Deg, Drawing, Inputs, Param, Path, Point, Text, Twist, Values, by_index, text};

fn drawn(drawing: &Drawing, values: &Values) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(drawing
        .shapes(values)?
        .iter()
        .map(|shape| shape.path().to_svg_path())
        .collect())
}

fn square() -> Result<Path, bullterrier_vector::Error> {
    Path::rect(0.0, 0.0, 100.0, 100.0, 0.0)
}

/// The `o` of `fraunces-subset.ttf` at 2000 pixels to the em: a shape whose origin, (583, -700), is not the middle of
/// its bounds (an `o` is x-height tall, not cap-height).
fn letter() -> Result<Drawing, Box<dyn std::error::Error>> {
    Ok(text("o", Text::new(&fraunces()?).size(2000.0)))
}

// Twist ------------------------------------------------------------------------------------------------------------------

#[test]
fn a_twisted_drawing_is_the_twisted_path_about_the_middle_of_its_bounds() -> Outcome {
    let path = square()?;
    let found = drawn(&Drawing::from(path.clone()).twist(Deg(90.0)), &Values::new())?;
    assert_eq!(found, vec![path.twist(90.0, None, None)?.to_svg_path()]);
    assert_ne!(found[0], path.to_svg_path());
    // the radius and the pivot given
    let found = drawn(
        &Drawing::from(path.clone()).twist(Twist::new(Deg(120.0)).radius(40.0).about((30.0, 20.0))),
        &Values::new(),
    )?;
    assert_eq!(
        found,
        vec![
            path.twist(120.0, Some(40.0), Some(Point::new(30.0, 20.0)))?
                .to_svg_path()
        ]
    );
    Ok(())
}

#[test]
fn a_twist_without_a_pivot_is_about_the_origin_of_the_shape_not_the_middle_of_its_bounds() -> Outcome {
    let shapes = letter()?.shapes(&Values::new())?;
    let path = shapes[0].path().clone();
    let b = path.bounds()?;
    assert!(
        (b.y + b.height / 2.0 + 700.0).abs() > 50.0,
        "the premise: the middle of the bounds is not the origin"
    );
    let found = drawn(&letter()?.twist(Deg(90.0)), &Values::new())?;
    assert_eq!(
        found,
        vec![path.twist(90.0, None, Some(Point::new(583.0, -700.0)))?.to_svg_path()]
    );
    assert_ne!(found, vec![path.twist(90.0, None, None)?.to_svg_path()]);
    // a pivot given beats the origin
    let given = drawn(
        &letter()?.twist(Twist::new(Deg(90.0)).about((0.0, 0.0))),
        &Values::new(),
    )?;
    assert_eq!(
        given,
        vec![path.twist(90.0, None, Some(Point::new(0.0, 0.0)))?.to_svg_path()]
    );
    Ok(())
}

#[test]
fn each_shape_twists_about_its_own_middle_with_its_own_angle_and_keeps_style_and_attributes() -> Outcome {
    let left = Path::rect(0.0, 0.0, 100.0, 100.0, 0.0)?;
    let right = Path::rect(300.0, 0.0, 100.0, 100.0, 0.0)?;
    let both = Drawing::new([left.clone(), right.clone()]);
    let found = drawn(&both.twist(Twist::new(Deg(by_index(30.0, 90.0)))), &Values::new())?;
    assert_eq!(found[0], left.twist(30.0, None, None)?.to_svg_path());
    assert_eq!(found[1], right.twist(90.0, None, None)?.to_svg_path());
    let word = text("Hi", &fraunces()?).fill(Color::hex(0xff_00_00));
    let (before, after) = (
        word.shapes(&Values::new())?,
        word.twist(Deg(40.0)).shapes(&Values::new())?,
    );
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(a.style(), b.style());
        assert_eq!(a.attributes(), b.attributes());
    }
    Ok(())
}

#[test]
fn a_param_angle_is_read_when_drawn() -> Outcome {
    let progress = Param::new("progress", 0.0, 0.0, 1.0)?;
    let twisted = Drawing::from(square()?).twist(Deg(&progress * 180.0));
    assert_eq!(
        drawn(&twisted, &Values::new())?,
        drawn(&Drawing::from(square()?).twist(Deg(0.0)), &Values::new())?
    );
    assert_eq!(
        drawn(&twisted, &Values::new().set("progress", 0.5))?,
        drawn(&Drawing::from(square()?).twist(Deg(90.0)), &Values::new())?
    );
    assert_ne!(
        drawn(&twisted, &Values::new().set("progress", 0.5))?,
        drawn(&twisted, &Values::new())?
    );
    Ok(())
}

#[test]
fn the_twist_step_saves_and_loads_the_same_drawing() -> Outcome {
    let drawing =
        Drawing::from(square()?).twist(Twist::new(Deg(by_index(30.0, 60.0))).radius(70.0).about((10.0, 20.0)));
    let json = drawing.to_json()?;
    assert!(
        json.contains(r#""twist":{"#) && json.contains(r#""radius":70"#) && json.contains(r#""about":[10,20]"#),
        "{json}"
    );
    let again = Drawing::from_json(&json, &Inputs::new())?;
    assert_eq!(again.to_json()?, json);
    assert_eq!(again.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    // what is left at its default is not written, and the default angle is 90
    let loaded = Drawing::from_json(
        r#"{"version":1,"root":[{"path":"M0 0 L100 0 L100 100 L0 100 Z"},{"twist":{}}]}"#,
        &Inputs::new(),
    )?;
    assert_eq!(
        drawn(&loaded, &Values::new())?,
        drawn(&Drawing::from(square()?).twist(Deg(90.0)), &Values::new())?
    );
    Ok(())
}

#[test]
fn a_twist_radius_not_above_zero_names_the_step_and_a_tiny_one_is_charged_to_max_segments() -> Outcome {
    let error = Drawing::from(square()?)
        .twist(Twist::new(Deg(90.0)).radius(0.0))
        .error()
        .map(ToString::to_string)
        .ok_or("no error")?;
    assert!(error.contains("radius"), "{error}");
    let (place, message) = parameter_refusal(
        Drawing::from(square()?)
            .twist(Twist::new(Deg(90.0)).radius(1e-9))
            .shapes(&Values::new()),
    )?;
    assert!(
        place.contains("step 1 (twist)") && message.contains("MAX_SEGMENTS"),
        "{place}: {message}"
    );
    Ok(())
}

// Bulge ------------------------------------------------------------------------------------------------------------------

#[test]
fn a_bulged_and_a_pinched_circle_are_the_bulged_path_and_differ_the_way_round() -> Outcome {
    let circle = Path::circle((50.0, 50.0), 40.0)?;
    let drawing = Drawing::from(circle.clone());
    let out = drawn(&drawing.bulge(0.6), &Values::new())?;
    assert_eq!(out, vec![circle.bulge(0.6, None, None)?.to_svg_path()]);
    // the middle of a circle's rim is at 40 from its center, the radius reaches 70.7: bulging moves the rim out
    let width = |stretch: f64| -> Result<f64, Box<dyn std::error::Error>> {
        Ok(circle.bulge(stretch, None, None)?.bounds()?.width)
    };
    assert!(width(0.6)? > 80.0 + 1.0 && width(-0.6)? < 80.0 - 1.0);
    let given = drawn(
        &drawing.bulge(Bulge::new(0.6).radius(60.0).about((50.0, 50.0))),
        &Values::new(),
    )?;
    assert_eq!(
        given,
        vec![
            circle
                .bulge(0.6, Some(60.0), Some(Point::new(50.0, 50.0)))?
                .to_svg_path()
        ]
    );
    Ok(())
}

#[test]
fn a_bulge_without_a_pivot_is_about_the_origin_of_the_shape() -> Outcome {
    let path = letter()?.shapes(&Values::new())?[0].path().clone();
    let found = drawn(&letter()?.bulge(0.5), &Values::new())?;
    assert_eq!(
        found,
        vec![path.bulge(0.5, None, Some(Point::new(583.0, -700.0)))?.to_svg_path()]
    );
    assert_ne!(found, vec![path.bulge(0.5, None, None)?.to_svg_path()]);
    Ok(())
}

#[test]
fn each_shape_reads_its_own_amount_and_a_param_is_read_when_drawn() -> Outcome {
    let two = Drawing::new([square()?, square()?]);
    let found = drawn(&two.bulge(by_index(0.2, 1.0)), &Values::new())?;
    assert_eq!(found[0], square()?.bulge(0.2, None, None)?.to_svg_path());
    assert_eq!(found[1], square()?.bulge(1.0, None, None)?.to_svg_path());
    let stretch = Param::new("stretch", 0.0, -0.9, 1.4)?;
    let bulged = Drawing::from(square()?).bulge(&stretch);
    assert_eq!(
        drawn(&bulged, &Values::new().set("stretch", -0.5))?,
        vec![square()?.bulge(-0.5, None, None)?.to_svg_path()]
    );
    Ok(())
}

#[test]
fn the_bulge_step_saves_and_loads_the_same_drawing() -> Outcome {
    let drawing = Drawing::from(square()?).bulge(Bulge::new(by_index(0.2, 0.9)).radius(70.0).about((10.0, 20.0)));
    let json = drawing.to_json()?;
    assert!(
        json.contains(r#""bulge":{"#) && json.contains(r#""radius":70"#),
        "{json}"
    );
    let again = Drawing::from_json(&json, &Inputs::new())?;
    assert_eq!(again.to_json()?, json);
    assert_eq!(again.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    Ok(())
}

#[test]
fn an_amount_that_would_fold_the_shape_is_refused_at_draw_time_naming_the_step() -> Outcome {
    for stretch in [-1.0, 16.0 / 11.0, 5.0] {
        let (place, message) = parameter_refusal(Drawing::from(square()?).bulge(stretch).shapes(&Values::new()))?;
        assert!(
            place.contains("step 1 (bulge)")
                && place.contains("stretch")
                && message.contains("above -1 and below 16/11 (1.4545), so that the shape does not fold over"),
            "{place}: {message}"
        );
    }
    Ok(())
}
