//! The `knockout` step of a `Drawing`, through the public API: what the shapes drawn carry, how the step composes with
//! the others, the registry's declaration and the document it is written to. What a knockout does to a picture is in
//! `output_knockout`, `svg_knockout` and `mesh_knockout`.

use crate::common::Outcome;
use bullterrier_vector::{Color, Drawing, Error, Inputs, Path, Shadow, Values, verbs};

fn square() -> Result<Path, Error> {
    Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)
}

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

/// Whether each shape `drawing` draws knocks out.
fn knocks(drawing: &Drawing) -> Result<Vec<bool>, Box<dyn std::error::Error>> {
    Ok(drawing
        .shapes(&Values::new())?
        .iter()
        .map(|shape| shape.style().knockout())
        .collect())
}

#[test]
fn a_shape_paints_until_the_step_makes_it_knock_out() -> Outcome {
    let drawing = Drawing::new([square()?, square()?]);
    assert_eq!(knocks(&drawing)?, vec![false, false]);
    assert_eq!(knocks(&drawing.knockout())?, vec![true, true]);
    Ok(())
}

#[test]
fn a_second_call_changes_nothing_and_the_step_is_the_same_on_a_path() -> Outcome {
    let once = Drawing::from(square()?).knockout();
    assert_eq!(once.knockout().shapes(&Values::new())?, once.shapes(&Values::new())?);
    assert_eq!(
        square()?.knockout().shapes(&Values::new())?,
        once.shapes(&Values::new())?
    );
    Ok(())
}

#[test]
fn the_other_styles_keep_their_say_before_and_after_it() -> Outcome {
    let drawing = Drawing::from(square()?)
        .fill(color("#ff0000")?)
        .knockout()
        .opacity(0.5)
        .shadow(Shadow::new().dy(4.0));
    let shapes = drawing.shapes(&Values::new())?;
    let style = shapes[0].style();
    assert!(style.knockout());
    assert_eq!(*style.fill(), Color::parse("#ff0000")?);
    assert_eq!(style.opacity(), 0.5);
    assert_eq!(style.shadow().map(bullterrier_vector::ShadowStyle::dy), Some(4.0));
    Ok(())
}

#[test]
fn the_registry_declares_a_step_with_no_arguments() -> Outcome {
    let verb = verbs().find(|verb| verb.name == "knockout").ok_or("no knockout verb")?;
    assert_eq!(verb.args.len(), 0, "a step with no arguments");
    Ok(())
}

#[test]
fn a_knockout_is_written_as_a_step_with_nothing_in_it_and_read_back() -> Outcome {
    let drawing = Drawing::from(square()?).knockout();
    let text = drawing.to_json()?;
    assert!(text.contains(r#"{"knockout":{}}"#), "{text}");
    let back = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(back.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    Ok(())
}

#[test]
fn a_shape_that_knocks_out_goes_back_into_a_drawing_still_knocking_out() -> Outcome {
    let drawing = Drawing::new([square()?, square()?.translate(5.0, 0.0)?])
        .fill(color("#ff0000")?)
        .knockout();
    let shapes = drawing.shapes(&Values::new())?;
    let again = Drawing::new(shapes.clone()).shapes(&Values::new())?;
    assert_eq!(again, shapes);
    let through_json = Drawing::from_json(&Drawing::new(shapes.clone()).to_json()?, &Inputs::new())?;
    assert_eq!(through_json.shapes(&Values::new())?, shapes);
    Ok(())
}
