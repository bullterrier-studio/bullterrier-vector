//! The `halftone` step of a `Drawing`, through the public API: what the shapes drawn carry, how it and `fill` replace
//! one another, what the step refuses, how a number reads a Param or the shape it is for, and the document it is
//! written to. What a halftone does to a picture is in `output_halftone`, `svg_halftone` and `mesh_halftone`.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{
    Color, Deg, Drawing, Error, Halftone, HalftoneStyle, Inputs, Param, Path, Values, by_index, verbs,
};

fn square() -> Result<Path, Error> {
    Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)
}

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

/// The halftone of every shape `drawing` draws with `values`.
fn halftones(drawing: &Drawing, values: &Values) -> Result<Vec<Option<HalftoneStyle>>, Box<dyn std::error::Error>> {
    Ok(drawing
        .shapes(values)?
        .iter()
        .map(|shape| shape.style().halftone().copied())
        .collect())
}

fn only_halftone(drawing: &Drawing) -> Result<HalftoneStyle, Box<dyn std::error::Error>> {
    match halftones(drawing, &Values::new())?.as_slice() {
        [Some(halftone)] => Ok(*halftone),
        other => Err(format!("expected one halftone, got {other:?}").into()),
    }
}

/// The colour, tone, pitch and angle of a halftone.
fn parts(halftone: &HalftoneStyle) -> (String, f64, f64, f64) {
    (
        halftone.color().to_string(),
        halftone.tone(),
        halftone.pitch(),
        halftone.angle(),
    )
}

#[test]
fn a_shape_has_no_halftone_until_the_step_gives_it_one() -> Outcome {
    assert_eq!(halftones(&Drawing::from(square()?), &Values::new())?, vec![None]);
    Ok(())
}

#[test]
fn what_a_halftone_leaves_out_is_the_default_of_api_md() -> Outcome {
    let drawing = Drawing::from(square()?).halftone(Halftone::new(color("#336699")?, 0.25));
    assert_eq!(
        parts(&only_halftone(&drawing)?),
        ("#336699ff".to_owned(), 0.25, 8.0, 45.0)
    );
    Ok(())
}

#[test]
fn every_argument_is_read_back() -> Outcome {
    let drawing =
        Drawing::from(square()?).halftone(Halftone::new(color("#ff000080")?, 0.5).pitch(6.0).angle(Deg(15.0)));
    assert_eq!(
        parts(&only_halftone(&drawing)?),
        ("#ff000080".to_owned(), 0.5, 6.0, 15.0)
    );
    Ok(())
}

#[test]
fn a_halftone_is_the_fill_and_the_last_of_the_two_wins() -> Outcome {
    let dots = Halftone::new(color("#000000")?, 0.5);
    let red = color("#ff0000")?;
    // a halftone paints no solid fill
    let screened = Drawing::from(square()?).halftone(dots.clone());
    assert_eq!(*screened.shapes(&Values::new())?[0].style().fill(), Color::NONE);
    // fill after halftone replaces it
    let filled = screened.fill(red);
    assert_eq!(halftones(&filled, &Values::new())?, vec![None]);
    assert_eq!(*filled.shapes(&Values::new())?[0].style().fill(), red);
    // halftone after fill replaces it, and so does a second halftone
    let again = filled.halftone(dots.clone());
    assert_eq!(*again.shapes(&Values::new())?[0].style().fill(), Color::NONE);
    let last = again.halftone(Halftone::new(red, 0.75));
    assert_eq!(parts(&only_halftone(&last)?), ("#ff0000ff".to_owned(), 0.75, 8.0, 45.0));
    Ok(())
}

#[test]
fn the_step_is_the_same_on_a_path_and_a_stroke_goes_with_it() -> Outcome {
    let dots = Halftone::new(color("#000000")?, 0.5);
    assert_eq!(
        square()?.halftone(dots.clone()).shapes(&Values::new())?,
        Drawing::from(square()?).halftone(dots.clone()).shapes(&Values::new())?
    );
    let both = Drawing::from(square()?).halftone(dots).stroke(color("#ff0000")?);
    let shapes = both.shapes(&Values::new())?;
    assert!(shapes[0].style().halftone().is_some() && shapes[0].style().stroke().is_some());
    Ok(())
}

#[test]
fn a_number_follows_a_param_or_the_shape_it_is_for() -> Outcome {
    let tone = Param::new("tone", 0.5, 0.0, 1.0)?;
    let drawing = Drawing::new([square()?, square()?, square()?])
        .halftone(Halftone::new(color("#000000")?, &tone).pitch(by_index(4.0, 8.0)));
    let seen: Vec<(f64, f64)> = halftones(&drawing, &Values::new())?
        .iter()
        .map(|halftone| halftone.map(|h| (h.tone(), h.pitch())).ok_or("no halftone"))
        .collect::<Result<_, _>>()?;
    assert_eq!(seen, vec![(0.5, 4.0), (0.5, 6.0), (0.5, 8.0)]);
    let lighter = halftones(&drawing, &Values::new().set("tone", 0.9))?;
    assert_eq!(lighter[0].map(|halftone| halftone.tone()), Some(0.9));
    Ok(())
}

#[test]
fn what_the_step_refuses_is_refused_at_the_call_naming_the_argument() -> Outcome {
    let base = Drawing::from(square()?);
    let black = color("#000000")?;
    // (case, drawing, the argument named)
    let rows = [
        (
            "a tone below 0",
            base.halftone(Halftone::new(black, -0.1)),
            "step 1 (halftone).tone",
        ),
        (
            "a tone above 1",
            base.halftone(Halftone::new(black, 1.1)),
            "step 1 (halftone).tone",
        ),
        (
            "a tone of NaN",
            base.halftone(Halftone::new(black, f64::NAN)),
            "step 1 (halftone).tone",
        ),
        (
            "a pitch of 0",
            base.halftone(Halftone::new(black, 0.5).pitch(0.0)),
            "step 1 (halftone).pitch",
        ),
        (
            "a negative pitch",
            base.halftone(Halftone::new(black, 0.5).pitch(-2.0)),
            "step 1 (halftone).pitch",
        ),
        (
            "an infinite pitch",
            base.halftone(Halftone::new(black, 0.5).pitch(f64::INFINITY)),
            "step 1 (halftone).pitch",
        ),
        (
            "an angle of NaN",
            base.halftone(Halftone::new(black, 0.5).angle(Deg(f64::NAN))),
            "step 1 (halftone).angle",
        ),
        (
            "a colour of none",
            base.halftone(Halftone::new(Color::NONE, 0.5)),
            "step 1 (halftone).color",
        ),
    ];
    for (case, drawing, argument) in rows {
        let error = drawing.error().cloned().ok_or("the drawing holds no error")?;
        let (name, message) = parameter_refusal(Err::<(), _>(error))?;
        assert_eq!(name, argument, "{case}: {message}");
    }
    // the edges of what is accepted
    for accepted in [
        Halftone::new(black, 0.0),
        Halftone::new(black, 1.0),
        Halftone::new(black, 0.5).pitch(1e-9),
        Halftone::new(black, 0.5).angle(Deg(-720.0)),
        Halftone::new(color("#00000000")?, 0.5),
    ] {
        assert_eq!(base.halftone(accepted).error(), None);
    }
    Ok(())
}

#[test]
fn a_tone_that_reads_a_param_is_refused_when_drawn() -> Outcome {
    let tone = Param::new("tone", 0.5, -1.0, 2.0)?;
    let drawing = Drawing::from(square()?).halftone(Halftone::new(color("#000000")?, &tone));
    assert_eq!(drawing.error(), None);
    let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("tone", 1.5)))?;
    assert!(
        name == "step 1 (halftone).tone" && message.starts_with("shape 0: "),
        "{name}: {message}"
    );
    Ok(())
}

#[test]
fn the_registry_declares_the_arguments_with_their_defaults_and_ranges() -> Outcome {
    let verb = verbs().find(|verb| verb.name == "halftone").ok_or("no halftone verb")?;
    let declared: Vec<(&str, String)> = verb
        .args
        .iter()
        .map(|arg| (arg.name, format!("{:?} {:?}", arg.default, arg.domain)))
        .collect();
    assert_eq!(
        declared,
        vec![
            ("color", "None Painting".to_owned()),
            ("tone", "None Fraction".to_owned()),
            ("pitch", "Some(Number(8.0)) Positive".to_owned()),
            ("angle", "Some(Number(45.0)) Finite".to_owned()),
        ]
    );
    Ok(())
}

// Documents ---------------------------------------------------------------------------------------------------------

#[test]
fn a_halftone_is_written_with_what_differs_from_the_defaults_and_read_back() -> Outcome {
    let drawing = Drawing::from(square()?).halftone(Halftone::new(color("#336699")?, 0.4).pitch(5.0));
    let text = drawing.to_json()?;
    assert!(
        text.contains(r##"{"halftone":{"color":"#336699","pitch":5,"tone":0.4}}"##),
        "{text}"
    );
    let back = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(back.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    Ok(())
}

#[test]
fn a_shape_with_a_halftone_goes_back_into_a_drawing_with_it() -> Outcome {
    let drawing = Drawing::from(square()?)
        .halftone(Halftone::new(color("#ff0000")?, 0.3).pitch(3.0).angle(Deg(10.0)))
        .stroke(color("#0000ff")?);
    let shapes = drawing.shapes(&Values::new())?;
    let again = Drawing::new(shapes.clone()).shapes(&Values::new())?;
    assert_eq!(again, shapes);
    let through_json = Drawing::from_json(&Drawing::new(shapes.clone()).to_json()?, &Inputs::new())?;
    assert_eq!(through_json.shapes(&Values::new())?, shapes);
    Ok(())
}
