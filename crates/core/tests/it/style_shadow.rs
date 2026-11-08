//! The `shadow` step of a `Drawing`, through the public API: what the shapes drawn carry, what the step refuses, how a
//! number reads a Param or the shape it is for, and the document it is written to. Expected values are written by hand.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Color, Drawing, Error, Inputs, Param, Path, Shadow, ShadowStyle, Values, by_index, verbs};

fn square() -> Result<Path, Error> {
    Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)
}

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

/// The shadow of every shape `drawing` draws with `values`.
fn shadows(drawing: &Drawing, values: &Values) -> Result<Vec<Option<ShadowStyle>>, Box<dyn std::error::Error>> {
    Ok(drawing
        .shapes(values)?
        .iter()
        .map(|shape| shape.style().shadow().copied())
        .collect())
}

fn only_shadow(drawing: &Drawing) -> Result<ShadowStyle, Box<dyn std::error::Error>> {
    match shadows(drawing, &Values::new())?.as_slice() {
        [Some(shadow)] => Ok(*shadow),
        other => Err(format!("expected one shadow, got {other:?}").into()),
    }
}

#[test]
fn a_shape_casts_no_shadow_until_the_step_gives_it_one() -> Outcome {
    assert_eq!(shadows(&Drawing::from(square()?), &Values::new())?, vec![None]);
    Ok(())
}

#[test]
fn what_a_shadow_leaves_out_is_the_default_of_api_md() -> Outcome {
    let shadow = only_shadow(&Drawing::from(square()?).shadow(Shadow::new()))?;
    assert_eq!(
        (shadow.dx(), shadow.dy(), shadow.blur(), shadow.color().to_string()),
        (0.0, 8.0, 16.0, "#00000059".to_owned())
    );
    Ok(())
}

#[test]
fn every_argument_is_read_back_and_the_last_shadow_wins() -> Outcome {
    let first = Shadow::new().dx(1.0).dy(2.0).blur(3.0).color(color("#ff000080")?);
    let shadow = only_shadow(&Drawing::from(square()?).shadow(first.clone()))?;
    assert_eq!(
        (shadow.dx(), shadow.dy(), shadow.blur(), shadow.color().to_string()),
        (1.0, 2.0, 3.0, "#ff000080".to_owned())
    );
    // a second call replaces the first as a whole: what it leaves out is the default, not the first's value
    let last = only_shadow(&Drawing::from(square()?).shadow(first).shadow(Shadow::new().blur(5.0)))?;
    assert_eq!(
        (last.dx(), last.dy(), last.blur(), last.color().to_string()),
        (0.0, 8.0, 5.0, "#00000059".to_owned())
    );
    Ok(())
}

#[test]
fn the_step_is_the_same_on_a_path_and_it_survives_other_styles() -> Outcome {
    let shadow = Shadow::new().dy(4.0);
    assert_eq!(
        square()?.shadow(shadow.clone()).shapes(&Values::new())?,
        Drawing::from(square()?).shadow(shadow.clone()).shapes(&Values::new())?
    );
    let styled = Drawing::from(square()?)
        .shadow(shadow)
        .fill(color("#ff0000")?)
        .opacity(0.5);
    assert_eq!(only_shadow(&styled)?.dy(), 4.0);
    Ok(())
}

#[test]
fn a_number_follows_a_param_or_the_shape_it_is_for() -> Outcome {
    let lift = Param::new("lift", 3.0, 0.0, 20.0)?;
    let drawing =
        Drawing::new([square()?, square()?, square()?]).shadow(Shadow::new().dy(by_index(0.0, 10.0)).blur(&lift));
    let found = shadows(&drawing, &Values::new())?;
    let seen: Vec<(f64, f64)> = found
        .iter()
        .map(|shadow| shadow.map(|shadow| (shadow.dy(), shadow.blur())).ok_or("no shadow"))
        .collect::<Result<_, _>>()?;
    assert_eq!(seen, vec![(0.0, 3.0), (5.0, 3.0), (10.0, 3.0)]);
    let lifted = shadows(&drawing, &Values::new().set("lift", 12.0))?;
    assert_eq!(lifted[0].map(|shadow| shadow.blur()), Some(12.0));
    Ok(())
}

#[test]
fn what_the_step_refuses_is_refused_at_the_call_naming_the_argument() -> Outcome {
    let base = Drawing::from(square()?);
    // (case, drawing, the argument named)
    let rows = [
        (
            "a negative blur",
            base.shadow(Shadow::new().blur(-0.5)),
            "step 1 (shadow).blur",
        ),
        (
            "a blur of NaN",
            base.shadow(Shadow::new().blur(f64::NAN)),
            "step 1 (shadow).blur",
        ),
        (
            "an infinite blur",
            base.shadow(Shadow::new().blur(f64::INFINITY)),
            "step 1 (shadow).blur",
        ),
        (
            "dx of NaN",
            base.shadow(Shadow::new().dx(f64::NAN)),
            "step 1 (shadow).dx",
        ),
        (
            "an infinite dy",
            base.shadow(Shadow::new().dy(f64::NEG_INFINITY)),
            "step 1 (shadow).dy",
        ),
        (
            "a colour of none",
            base.shadow(Shadow::new().color(Color::NONE)),
            "step 1 (shadow).color",
        ),
    ];
    for (case, drawing, argument) in rows {
        let error = drawing.error().cloned().ok_or("the drawing holds no error")?;
        let (name, message) = parameter_refusal(Err::<(), _>(error))?;
        assert_eq!(name, argument, "{case}: {message}");
    }
    // the edges of what is accepted
    for accepted in [
        Shadow::new().blur(0.0),
        Shadow::new().blur(1e300),
        Shadow::new().dx(-1e300).dy(1e300),
        Shadow::new().color(color("#00000000")?),
    ] {
        assert_eq!(base.shadow(accepted).error(), None);
    }
    Ok(())
}

#[test]
fn a_blur_that_reads_a_param_is_refused_when_drawn() -> Outcome {
    let blur = Param::new("blur", 4.0, -5.0, 5.0)?;
    let drawing = Drawing::from(square()?).shadow(Shadow::new().blur(&blur));
    assert_eq!(drawing.error(), None);
    let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("blur", -1.0)))?;
    assert!(
        name == "step 1 (shadow).blur" && message.starts_with("shape 0: "),
        "{name}: {message}"
    );
    Ok(())
}

#[test]
fn the_registry_declares_the_arguments_with_their_defaults_and_ranges() -> Outcome {
    let verb = verbs().find(|verb| verb.name == "shadow").ok_or("no shadow verb")?;
    let declared: Vec<(&str, String)> = verb
        .args
        .iter()
        .map(|arg| (arg.name, format!("{:?} {:?}", arg.default, arg.domain)))
        .collect();
    assert_eq!(
        declared,
        vec![
            ("dx", "Some(Number(0.0)) Finite".to_owned()),
            ("dy", "Some(Number(8.0)) Finite".to_owned()),
            ("blur", "Some(Number(16.0)) AtLeast(0.0)".to_owned()),
            ("color", "Some(Unset) Painting".to_owned()),
        ]
    );
    Ok(())
}

// Documents ---------------------------------------------------------------------------------------------------------

#[test]
fn a_shadow_is_written_with_what_differs_from_the_defaults_and_read_back() -> Outcome {
    let drawing = Drawing::from(square()?).shadow(Shadow::new().dy(10.0).blur(18.0).color(color("#0000004d")?));
    let text = drawing.to_json()?;
    assert!(
        text.contains(r##"{"shadow":{"blur":18,"color":"#0000004d","dy":10}}"##),
        "{text}"
    );
    let back = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(back.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    // all defaults: nothing but the step
    let plain = Drawing::from(square()?).shadow(Shadow::new()).to_json()?;
    assert!(plain.contains(r#"{"shadow":{}}"#), "{plain}");
    Ok(())
}

#[test]
fn a_shape_with_a_shadow_goes_back_into_a_drawing_with_it() -> Outcome {
    let drawing = Drawing::from(square()?)
        .fill(color("#ff0000")?)
        .shadow(Shadow::new().dx(2.0).blur(7.0));
    let shapes = drawing.shapes(&Values::new())?;
    let again = Drawing::new(shapes.clone()).shapes(&Values::new())?;
    assert_eq!(again, shapes);
    let through_json = Drawing::from_json(&Drawing::new(shapes.clone()).to_json()?, &Inputs::new())?;
    assert_eq!(through_json.shapes(&Values::new())?, shapes);
    Ok(())
}
