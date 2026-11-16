//! `Drawing::wave` through the public API. The usage first: a line waved, a sine in the line's points. Expected values
//! are by hand: a point at distance `d` along the direction of travel moves `amplitude * sin(2 pi (d / wavelength -
//! phase))` sideways (angle 0: down the screen).

use crate::common::fonts::fraunces;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Color, Deg, Drawing, Inputs, Param, Path, Values, by_index, text};

fn drawn(drawing: &Drawing, values: &Values) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(drawing
        .shapes(values)?
        .iter()
        .map(|shape| shape.path().to_svg_path())
        .collect())
}

fn line() -> Result<Drawing, bullterrier_vector::Error> {
    Ok(Drawing::from(Path::line((0.0, 0.0), (120.0, 0.0))?))
}

#[test]
fn a_waved_line_is_a_sine_in_its_points() -> Outcome {
    // 120 long, a wavelength of 120: the quarter wave at x = 30 is 8 down, the three quarters at x = 90 is 8 up
    let found = drawn(&line()?.wave(8.0, 120.0, Deg(0.0), 0.0), &Values::new())?;
    assert_eq!(found.len(), 1);
    assert!(found[0].starts_with("M0 0 L5 "), "{}", found[0]);
    assert!(found[0].contains(" L30 8 "), "{}", found[0]);
    assert!(found[0].contains(" L90 -8 "), "{}", found[0]);
    assert!(
        found[0].ends_with(" L120 0") || found[0].contains(" L120 -0"),
        "{}",
        found[0]
    );
    Ok(())
}

#[test]
fn an_angle_turns_the_direction_of_travel() -> Outcome {
    // travelling down, the quarter wave at y = 30 moves a point of the vertical line 8 to the left
    let vertical = Drawing::from(Path::line((0.0, 0.0), (0.0, 120.0))?);
    let found = drawn(&vertical.wave(8.0, 120.0, Deg(90.0), 0.0), &Values::new())?;
    assert!(found[0].contains(" L-8 30 "), "{}", found[0]);
    Ok(())
}

#[test]
fn each_shape_reads_its_own_amplitude_and_keeps_its_style_and_attributes() -> Outcome {
    let lines = Drawing::new([
        Path::line((0.0, 0.0), (120.0, 0.0))?,
        Path::line((0.0, 0.0), (120.0, 0.0))?,
    ]);
    let found = drawn(&lines.wave(by_index(8.0, 16.0), 120.0, Deg(0.0), 0.0), &Values::new())?;
    assert!(found[0].contains(" L30 8 "), "{}", found[0]);
    assert!(found[1].contains(" L30 16 "), "{}", found[1]);
    let word = text("Hi", &fraunces()?).fill(Color::hex(0xff_00_00));
    let (before, after) = (
        word.shapes(&Values::new())?,
        word.wave(4.0, 60.0, Deg(0.0), 0.0).shapes(&Values::new())?,
    );
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(a.style(), b.style());
        assert_eq!(a.attributes(), b.attributes());
    }
    Ok(())
}

#[test]
fn a_param_phase_is_read_when_drawn() -> Outcome {
    let phase = Param::new("phase", 0.0, 0.0, 2.0)?;
    let waved = line()?.wave(8.0, 120.0, Deg(0.0), &phase);
    // a quarter cycle on, the start is at sin(-pi/2) = -1: 8 up
    assert!(drawn(&waved, &Values::new())?[0].starts_with("M0 0 "));
    assert!(drawn(&waved, &Values::new().set("phase", 0.25))?[0].starts_with("M0 -8 "));
    // a whole cycle on is the same drawing to the bit
    assert_eq!(
        drawn(&waved, &Values::new().set("phase", 0.25))?,
        drawn(&waved, &Values::new().set("phase", 1.25))?
    );
    Ok(())
}

#[test]
fn the_step_saves_and_loads_the_same_drawing() -> Outcome {
    let drawing = line()?.wave(by_index(4.0, 8.0), 90.0, Deg(30.0), 0.25);
    let json = drawing.to_json()?;
    assert!(json.contains(r#""wave":{"#) && json.contains(r#""angle":30"#), "{json}");
    let again = Drawing::from_json(&json, &Inputs::new())?;
    assert_eq!(again.to_json()?, json);
    assert_eq!(again.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    Ok(())
}

#[test]
fn a_wavelength_not_above_zero_names_the_step_and_a_tiny_one_is_charged_to_max_segments() -> Outcome {
    let error = line()?
        .wave(8.0, 0.0, Deg(0.0), 0.0)
        .error()
        .map(ToString::to_string)
        .ok_or("no error")?;
    assert!(error.contains("wavelength"), "{error}");
    let (place, message) = parameter_refusal(line()?.wave(8.0, 1e-9, Deg(0.0), 0.0).shapes(&Values::new()))?;
    assert!(
        place.contains("step 1 (wave)") && message.contains("MAX_SEGMENTS"),
        "{place}: {message}"
    );
    Ok(())
}
