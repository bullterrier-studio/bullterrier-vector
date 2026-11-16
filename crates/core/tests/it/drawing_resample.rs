//! `Drawing::resample` through the public API. The usage first: dots spaced evenly along a line are the corners of the
//! line resampled, one shape each once the drawing is split into its corners by the caller. Expected values are by hand.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Color, Drawing, Inputs, Param, Path, Values, by_index, text};

use crate::common::fonts::fraunces;

fn drawn(drawing: &Drawing, values: &Values) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(drawing
        .shapes(values)?
        .iter()
        .map(|shape| shape.path().to_svg_path())
        .collect())
}

#[test]
fn a_line_resampled_to_twelve_pixels_has_a_corner_every_twelve() -> Outcome {
    let line = Drawing::from(Path::line((0.0, 0.0), (30.0, 0.0))?);
    let found = drawn(&line.resample(12.0), &Values::new())?;
    assert_eq!(found, ["M0 0 L12 0 L24 0 L30 0"]);
    Ok(())
}

#[test]
fn each_shape_reads_its_own_spacing_and_keeps_its_style_and_attributes() -> Outcome {
    let lines = Drawing::new([
        Path::line((0.0, 0.0), (20.0, 0.0))?,
        Path::line((0.0, 5.0), (20.0, 5.0))?,
    ]);
    let found = drawn(&lines.resample(by_index(10.0, 20.0)), &Values::new())?;
    assert_eq!(found, ["M0 0 L10 0 L20 0", "M0 5 L20 5"]);
    let word = text("Hi", &fraunces()?).fill(Color::hex(0xff_00_00));
    let (before, after) = (word.shapes(&Values::new())?, word.resample(5.0).shapes(&Values::new())?);
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(a.style(), b.style());
        assert_eq!(a.attributes(), b.attributes());
    }
    Ok(())
}

#[test]
fn a_param_spacing_is_read_when_drawn() -> Outcome {
    let spacing = Param::new("spacing", 10.0, 1.0, 100.0)?;
    let line = Drawing::from(Path::line((0.0, 0.0), (20.0, 0.0))?).resample(&spacing);
    assert_eq!(drawn(&line, &Values::new())?, ["M0 0 L10 0 L20 0"]);
    assert_eq!(drawn(&line, &Values::new().set("spacing", 20.0))?, ["M0 0 L20 0"]);
    Ok(())
}

#[test]
fn the_step_saves_and_loads_the_same_drawing() -> Outcome {
    let drawing = Drawing::from(Path::line((0.0, 0.0), (30.0, 0.0))?).resample(by_index(5.0, 10.0));
    let json = drawing.to_json()?;
    assert!(
        json.contains(r#"{"resample":{"spacing":{"by_index":{"first":5,"last":10}}}}"#),
        "{json}"
    );
    let again = Drawing::from_json(&json, &Inputs::new())?;
    assert_eq!(again.to_json()?, json);
    assert_eq!(again.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    Ok(())
}

#[test]
fn a_spacing_not_above_zero_names_the_step_and_a_tiny_one_is_charged_to_max_segments() -> Outcome {
    let line = Drawing::from(Path::line((0.0, 0.0), (1.0e6, 0.0))?);
    let (place, _) = parameter_refusal(line.resample(0.0).shapes(&Values::new())).or_else(
        |_| -> Result<_, Box<dyn std::error::Error>> {
            // refused at the call: the drawing holds the error
            let error = line.resample(0.0).error().map(ToString::to_string).ok_or("no error")?;
            Ok((error, String::new()))
        },
    )?;
    assert!(place.contains("spacing"), "{place}");
    let (place, message) = parameter_refusal(line.resample(1.0e-3).shapes(&Values::new()))?;
    assert!(
        place.contains("step 1 (resample)") && message.contains("MAX_SEGMENTS"),
        "{place}: {message}"
    );
    Ok(())
}
