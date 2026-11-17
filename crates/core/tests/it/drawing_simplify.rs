//! `Drawing::simplify` through the public API. The usage first: a dense polyline drawn, then `simplify(tolerance)`, a
//! few segments. Expected values are by hand.

use crate::common::fonts::fraunces;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Color, Drawing, Inputs, Param, Path, Values, by_index, text};

fn drawn(drawing: &Drawing, values: &Values) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(drawing
        .shapes(values)?
        .iter()
        .map(|shape| shape.path().to_svg_path())
        .collect())
}

/// A line of `count` points, `step` apart along x.
fn dense(count: u32, step: f64) -> Result<Path, bullterrier_vector::Error> {
    Path::polyline((0..count).map(|i| (f64::from(i) * step, 0.0)), false)
}

#[test]
fn a_dense_polyline_simplified_is_one_line() -> Outcome {
    let line = Drawing::from(dense(50, 2.0)?);
    assert_eq!(drawn(&line.simplify(0.5), &Values::new())?, ["M0 0 L98 0"]);
    Ok(())
}

#[test]
fn each_shape_reads_its_own_tolerance_and_keeps_its_style_and_attributes() -> Outcome {
    // a bump 1 high: a line within a tolerance of 2, a curve through its top at one of 0.25
    let bump = Path::polyline([(0.0, 0.0), (10.0, 1.0), (20.0, 0.0)], false)?;
    let shapes = Drawing::new([bump.clone(), bump.clone()]);
    let found = drawn(&shapes.simplify(by_index(2.0, 0.25)), &Values::new())?;
    assert_eq!(found[0], "M0 0 L20 0");
    assert!(
        found[1].starts_with("M0 0 C") && found[1].ends_with(" 20 0"),
        "{}",
        found[1]
    );
    let word = text("Hi", &fraunces()?).fill(Color::hex(0xff_00_00));
    let (before, after) = (word.shapes(&Values::new())?, word.simplify(0.5).shapes(&Values::new())?);
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(a.style(), b.style());
        // origin and every other attribute are untouched
        assert_eq!(a.attributes(), b.attributes());
    }
    Ok(())
}

#[test]
fn a_param_tolerance_is_read_when_drawn() -> Outcome {
    let tolerance = Param::new("slack", 0.25, 0.1, 5.0)?;
    let bump = Path::polyline([(0.0, 0.0), (10.0, 1.0), (20.0, 0.0)], false)?;
    let drawing = Drawing::from(bump).simplify(&tolerance);
    let curve = drawn(&drawing, &Values::new())?;
    assert!(curve[0].starts_with("M0 0 C"), "{curve:?}");
    assert_eq!(drawn(&drawing, &Values::new().set("slack", 2.0))?, ["M0 0 L20 0"]);
    Ok(())
}

#[test]
fn the_step_saves_and_loads_the_same_drawing() -> Outcome {
    let drawing = Drawing::from(dense(10, 1.0)?).simplify(by_index(0.5, 2.0));
    let json = drawing.to_json()?;
    assert!(
        json.contains(r#"{"simplify":{"tolerance":{"by_index":{"first":0.5,"last":2}}}}"#),
        "{json}"
    );
    let again = Drawing::from_json(&json, &Inputs::new())?;
    assert_eq!(again.to_json()?, json);
    assert_eq!(again.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    Ok(())
}

#[test]
fn a_tolerance_left_out_of_a_document_is_half_a_pixel() -> Outcome {
    let line = Drawing::from(dense(10, 1.0)?);
    let json = line
        .simplify(0.5)
        .to_json()?
        .replace(r#"{"simplify":{"tolerance":0.5}}"#, r#"{"simplify":{}}"#);
    assert!(json.contains(r#"{"simplify":{}}"#), "{json}");
    let loaded = Drawing::from_json(&json, &Inputs::new())?;
    assert_eq!(drawn(&loaded, &Values::new())?, ["M0 0 L9 0"]);
    Ok(())
}

#[test]
fn a_tolerance_not_above_zero_names_the_step() -> Outcome {
    let line = Drawing::from(dense(10, 1.0)?);
    let error = line.simplify(0.0).error().map(ToString::to_string).ok_or("no error")?;
    assert!(error.contains("tolerance"), "{error}");
    let tolerance = Param::new("slack", 1.0, -5.0, 5.0)?;
    let (place, _) = parameter_refusal(line.simplify(&tolerance).shapes(&Values::new().set("slack", -1.0)))?;
    assert!(place.contains("step 1 (simplify)"), "{place}");
    Ok(())
}
