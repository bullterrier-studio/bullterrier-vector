//! `Drawing::wobble` through the public API. The noise is tested on `Path::wobble`; here the step: that it is the
//! path's result per shape, reads its numbers per item and from Params, keeps style and attributes, saves and loads,
//! and refuses by naming the step.

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

fn line() -> Result<Path, bullterrier_vector::Error> {
    Path::line((0.0, 0.0), (200.0, 0.0))
}

#[test]
fn a_wobbled_drawing_is_the_wobbled_path() -> Outcome {
    let path = line()?;
    let found = drawn(
        &Drawing::from(path.clone()).wobble(6.0, 50.0, 0.25, 3.0),
        &Values::new(),
    )?;
    assert_eq!(found, vec![path.wobble(6.0, 50.0, 0.25, 3.0)?.to_svg_path()]);
    assert_ne!(found[0], path.to_svg_path());
    Ok(())
}

#[test]
fn each_shape_reads_its_own_amplitude_and_seed_and_keeps_its_style_and_attributes() -> Outcome {
    let lines = Drawing::new([line()?, line()?]);
    let found = drawn(
        &lines.wobble(by_index(2.0, 9.0), 50.0, 0.0, by_index(1.0, 2.0)),
        &Values::new(),
    )?;
    assert_eq!(found[0], line()?.wobble(2.0, 50.0, 0.0, 1.0)?.to_svg_path());
    assert_eq!(found[1], line()?.wobble(9.0, 50.0, 0.0, 2.0)?.to_svg_path());
    let word = text("Hi", &fraunces()?).fill(Color::hex(0xff_00_00));
    let (before, after) = (
        word.shapes(&Values::new())?,
        word.wobble(4.0, 60.0, 0.0, 0.0).shapes(&Values::new())?,
    );
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(a.style(), b.style());
        assert_eq!(a.attributes(), b.attributes());
    }
    Ok(())
}

#[test]
fn a_param_phase_is_read_when_drawn_and_a_whole_cycle_is_the_same_drawing() -> Outcome {
    let phase = Param::new("phase", 0.0, 0.0, 2.0)?;
    let wobbled = Drawing::from(line()?).wobble(6.0, 50.0, &phase, 0.0);
    let (start, quarter) = (
        drawn(&wobbled, &Values::new())?,
        drawn(&wobbled, &Values::new().set("phase", 0.25))?,
    );
    assert_ne!(start, quarter);
    assert_eq!(quarter, drawn(&wobbled, &Values::new().set("phase", 1.25))?);
    Ok(())
}

#[test]
fn the_step_saves_and_loads_the_same_drawing() -> Outcome {
    let drawing = Drawing::from(line()?).wobble(by_index(4.0, 8.0), 90.0, 0.25, 7.0);
    let json = drawing.to_json()?;
    assert!(json.contains(r#""wobble":{"#) && json.contains(r#""seed":7"#), "{json}");
    let again = Drawing::from_json(&json, &Inputs::new())?;
    assert_eq!(again.to_json()?, json);
    assert_eq!(again.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    Ok(())
}

#[test]
fn a_bad_wavelength_or_seed_names_the_step_and_a_tiny_wavelength_is_charged_to_max_segments() -> Outcome {
    let error = Drawing::from(line()?)
        .wobble(4.0, 0.0, 0.0, 0.0)
        .error()
        .map(ToString::to_string)
        .ok_or("no error")?;
    assert!(
        error.contains("step 1 (wobble)") && error.contains("wavelength"),
        "{error}"
    );
    // a seed that is not whole is refused when drawn
    let (place, _) = parameter_refusal(
        Drawing::from(line()?)
            .wobble(4.0, 80.0, 0.0, 0.5)
            .shapes(&Values::new()),
    )?;
    assert!(place.contains("step 1 (wobble)") && place.contains("seed"), "{place}");
    let (place, message) = parameter_refusal(
        Drawing::from(Path::line((0.0, 0.0), (1000.0, 0.0))?)
            .wobble(4.0, 1e-9, 0.0, 0.0)
            .shapes(&Values::new()),
    )?;
    assert!(
        place.contains("step 1 (wobble)") && message.contains("MAX_SEGMENTS"),
        "{place}: {message}"
    );
    Ok(())
}
