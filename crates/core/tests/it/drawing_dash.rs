//! `Drawing::dash` through the public API. The usage first: marching ants are a dashed outline whose `phase` is a Param
//! that goes from 0 to 1. Expected values are by hand.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Color, Drawing, Expr, Inputs, Param, Path, Point, Values, by_index};

fn drawn(drawing: &Drawing, values: &Values) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(drawing
        .shapes(values)?
        .iter()
        .map(|shape| shape.path().to_svg_path())
        .collect())
}

#[test]
fn marching_ants_move_forward_with_the_phase_and_a_whole_cycle_is_where_they_began() -> Outcome {
    let phase = Param::new("march", 0.0, 0.0, 1.0)?;
    let ants = Drawing::from(Path::line((0.0, 0.0), (64.0, 0.0))?)
        .dash([4.0, 12.0], &phase)
        .stroke(Color::hex(0x15_16_1a));
    let at = |value: f64| drawn(&ants, &Values::new().set("march", value));
    assert_eq!(at(0.0)?, ["M0 0 L4 0 M16 0 L20 0 M32 0 L36 0 M48 0 L52 0"]);
    // a quarter of the 16-pixel cycle: every dash 4 further on
    assert_eq!(at(0.25)?, ["M4 0 L8 0 M20 0 L24 0 M36 0 L40 0 M52 0 L56 0"]);
    assert_eq!(at(1.0)?, at(0.0)?);
    Ok(())
}

#[test]
fn each_shape_reads_its_own_pattern_and_phase_and_keeps_its_style_and_attributes() -> Outcome {
    let lines = Drawing::new([
        Path::line((0.0, 0.0), (16.0, 0.0))?,
        Path::line((0.0, 5.0), (16.0, 5.0))?,
    ])
    .fill(Color::hex(0xff_00_00));
    let found = drawn(&lines.dash([by_index(4.0, 8.0), by_index(4.0, 8.0)], 0.0), &Values::new())?;
    assert_eq!(found, ["M0 0 L4 0 M8 0 L12 0", "M0 5 L8 5"]);
    let (before, after) = (lines.shapes(&Values::new())?, lines.dash([4.0, 4.0], 0.0).shapes(&Values::new())?);
    assert_eq!(before[0].style(), after[0].style());
    assert_eq!(before[0].attributes(), after[0].attributes());
    Ok(())
}

#[test]
fn a_pattern_length_can_be_a_param_and_near_picks_the_start_of_a_closed_outline() -> Outcome {
    let on = Param::new("on", 4.0, 1.0, 16.0)?;
    let line = Drawing::from(Path::line((0.0, 0.0), (16.0, 0.0))?).dash([Expr::from(&on), Expr::from(4.0)], 0.0);
    assert_eq!(drawn(&line, &Values::new())?, ["M0 0 L4 0 M8 0 L12 0"]);
    assert_eq!(drawn(&line, &Values::new().set("on", 2.0))?, ["M0 0 L2 0 M6 0 L8 0 M12 0 L14 0"]);
    let square = Drawing::from(Path::from_svg_path("M0 0 L10 0 L10 10 L0 10 Z")?);
    let from_corner = square.dash_near([15.0, 5.0], 0.0, Point::new(10.0, 10.0));
    assert_eq!(drawn(&from_corner, &Values::new())?, ["M10 10 L0 10 L0 5 M0 0 L10 0 L10 5"]);
    Ok(())
}

#[test]
fn the_step_saves_and_loads_the_same_drawing() -> Outcome {
    let phase = Param::new("march", 0.0, 0.0, 1.0)?;
    let drawing = Drawing::from(Path::line((0.0, 0.0), (64.0, 0.0))?).dash([4.0, 12.0], &phase);
    let json = drawing.to_json()?;
    assert!(json.contains(r#"{"dash":{"pattern":[4,12],"phase":{"param":"march"}}}"#), "{json}");
    let again = Drawing::from_json(&json, &Inputs::new())?;
    assert_eq!(again.to_json()?, json);
    let values = Values::new().set("march", 0.25);
    assert_eq!(again.shapes(&values)?, drawing.shapes(&values)?);
    Ok(())
}

#[test]
fn a_pattern_that_cannot_dash_names_the_step_and_a_tiny_one_is_charged_to_max_segments() -> Outcome {
    let line = Drawing::from(Path::line((0.0, 0.0), (1.0e6, 0.0))?);
    for bad in [line.dash(Vec::<f64>::new(), 0.0), line.dash([4.0, 0.0], 0.0), line.dash([-1.0], 0.0)] {
        let error = bad.error().map(ToString::to_string).ok_or("no error")?;
        assert!(error.contains("pattern"), "{error}");
    }
    let (place, message) = parameter_refusal(line.dash([1.0e-3, 1.0e-3], 0.0).shapes(&Values::new()))?;
    assert!(
        place.contains("step 1 (dash)") && message.contains("MAX_SEGMENTS"),
        "{place}: {message}"
    );
    Ok(())
}
