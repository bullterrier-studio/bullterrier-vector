//! Gradient paints through the public API: what `Gradient::linear` and `radial` accept and refuse, how a fill or a
//! stroke carries one, how it replaces a colour and is replaced, and the document it is written to and read from.
//! What a gradient does to a picture is in `output_gradient`, `svg_gradient` and `mesh_gradient`.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Color, Drawing, Error, Gradient, GradientKind, Inputs, Paint, Path, Stroke, Values};

const RED: Color = Color::hex(0xff_00_00);
const BLUE: Color = Color::hex(0x00_00_ff);

fn square() -> Result<Path, Error> {
    Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)
}

fn two_stops() -> [(f64, Color); 2] {
    [(0.0, RED), (1.0, BLUE)]
}

fn fade() -> Result<Gradient, Error> {
    Gradient::linear((0.0, 0.0), (0.0, 100.0), two_stops())
}

fn only_style(drawing: &Drawing) -> Result<bullterrier_vector::Style, Box<dyn std::error::Error>> {
    Ok(drawing.shapes(&Values::new())?.remove(0).style().clone())
}

#[test]
fn a_linear_gradient_reads_back_its_line_and_stops() -> Outcome {
    let gradient = Gradient::linear((1.0, 2.0), (3.0, 4.0), [(0.0, RED), (0.25, BLUE), (1.0, RED)])?;
    assert_eq!(
        gradient.kind(),
        GradientKind::Linear {
            start: (1.0, 2.0).into(),
            end: (3.0, 4.0).into()
        }
    );
    assert_eq!(
        gradient.stops(),
        [
            (0.0, [255, 0, 0, 255]),
            (0.25, [0, 0, 255, 255]),
            (1.0, [255, 0, 0, 255])
        ]
    );
    Ok(())
}

#[test]
fn a_radial_gradient_reads_back_its_centre_and_radius() -> Outcome {
    let gradient = Gradient::radial((5.0, 6.0), 7.5, two_stops())?;
    assert_eq!(
        gradient.kind(),
        GradientKind::Radial {
            center: (5.0, 6.0).into(),
            radius: 7.5
        }
    );
    Ok(())
}

#[test]
fn stops_at_the_same_offset_make_a_hard_edge_and_are_accepted() -> Outcome {
    Gradient::linear(
        (0.0, 0.0),
        (1.0, 0.0),
        [(0.0, RED), (0.5, RED), (0.5, BLUE), (1.0, BLUE)],
    )?;
    Ok(())
}

#[test]
fn what_a_gradient_cannot_paint_is_refused_naming_the_argument() -> Outcome {
    let none = Color::NONE;
    let cases: Vec<(&str, Result<Gradient, Error>, &str, &str)> = vec![
        (
            "one stop",
            Gradient::linear((0.0, 0.0), (1.0, 0.0), [(0.0, RED)]),
            "stops",
            "at least 2",
        ),
        (
            "no stops",
            Gradient::linear((0.0, 0.0), (1.0, 0.0), []),
            "stops",
            "at least 2",
        ),
        (
            "offset below 0",
            Gradient::linear((0.0, 0.0), (1.0, 0.0), [(-0.1, RED), (1.0, BLUE)]),
            "stops",
            "-0.1",
        ),
        (
            "offset above 1",
            Gradient::linear((0.0, 0.0), (1.0, 0.0), [(0.0, RED), (1.5, BLUE)]),
            "stops",
            "1.5",
        ),
        (
            "offset NaN",
            Gradient::linear((0.0, 0.0), (1.0, 0.0), [(f64::NAN, RED), (1.0, BLUE)]),
            "stops",
            "NaN",
        ),
        (
            "offsets that decrease",
            Gradient::linear((0.0, 0.0), (1.0, 0.0), [(0.0, RED), (0.8, BLUE), (0.4, RED)]),
            "stops",
            "decrease",
        ),
        (
            "a stop of none",
            Gradient::linear((0.0, 0.0), (1.0, 0.0), [(0.0, RED), (1.0, none)]),
            "stops",
            "none",
        ),
        (
            "a line of no length",
            Gradient::linear((3.0, 3.0), (3.0, 3.0), two_stops()),
            "end",
            "differ",
        ),
        (
            "a start beyond range",
            Gradient::linear((1e101, 0.0), (0.0, 0.0), two_stops()),
            "start",
            "±1e100",
        ),
        (
            "an end not finite",
            Gradient::linear((0.0, 0.0), (f64::INFINITY, 0.0), two_stops()),
            "end",
            "inf",
        ),
        (
            "a radius of 0",
            Gradient::radial((0.0, 0.0), 0.0, two_stops()),
            "radius",
            "above 0",
        ),
        (
            "a negative radius",
            Gradient::radial((0.0, 0.0), -2.0, two_stops()),
            "radius",
            "-2",
        ),
        (
            "a radius not a number",
            Gradient::radial((0.0, 0.0), f64::NAN, two_stops()),
            "radius",
            "NaN",
        ),
        (
            "a centre beyond range",
            Gradient::radial((0.0, -1e101), 1.0, two_stops()),
            "center",
            "±1e100",
        ),
        (
            "radial with one stop",
            Gradient::radial((0.0, 0.0), 1.0, [(0.0, RED)]),
            "stops",
            "at least 2",
        ),
    ];
    for (case, outcome, parameter, fragment) in cases {
        let (name, message) = parameter_refusal(outcome)?;
        assert_eq!(name, parameter, "{case}");
        assert!(message.contains(fragment), "{case}: {message}");
    }
    Ok(())
}

#[test]
fn a_fill_and_a_stroke_carry_the_gradient_given() -> Outcome {
    let gradient = fade()?;
    let style = only_style(
        &Drawing::from(square()?)
            .fill(gradient.clone())
            .stroke(Stroke::new(gradient.clone()).width(3.0)),
    )?;
    assert_eq!(*style.fill(), Paint::from(gradient.clone()));
    let stroke = style.stroke().ok_or("no stroke")?;
    assert_eq!(*stroke.paint(), Paint::from(gradient));
    assert_eq!(stroke.width(), 3.0);
    Ok(())
}

#[test]
fn a_stroke_converts_from_a_gradient_and_from_a_colour_alike() -> Outcome {
    let by_gradient = only_style(&Drawing::from(square()?).stroke(fade()?))?;
    let by_stroke = only_style(&Drawing::from(square()?).stroke(Stroke::new(fade()?)))?;
    assert_eq!(by_gradient, by_stroke);
    Ok(())
}

#[test]
fn the_last_fill_wins_whether_a_gradient_or_a_colour() -> Outcome {
    let over_colour = only_style(&Drawing::from(square()?).fill(RED).fill(fade()?))?;
    assert_eq!(*over_colour.fill(), Paint::from(fade()?));
    let under_colour = only_style(&Drawing::from(square()?).fill(fade()?).fill(RED))?;
    assert_eq!(*under_colour.fill(), RED);
    // after a colour fill the style is what a plain fill gives, so nothing of the gradient is left
    assert_eq!(under_colour, only_style(&Drawing::from(square()?).fill(RED))?);
    Ok(())
}

#[test]
fn a_stroke_of_no_colour_is_refused_where_a_gradient_stroke_is_not() -> Outcome {
    let refused = Drawing::from(square()?).stroke(Color::NONE);
    assert!(refused.error().is_some());
    assert!(Drawing::from(square()?).stroke(fade()?).error().is_none());
    Ok(())
}

#[test]
fn a_shape_put_back_into_a_drawing_keeps_its_gradients() -> Outcome {
    let drawn = Drawing::from(square()?)
        .fill(fade()?)
        .stroke(Stroke::new(Gradient::radial((5.0, 5.0), 4.0, two_stops())?));
    let shape = drawn.shapes(&Values::new())?.remove(0);
    assert_eq!(
        drawn.shapes(&Values::new())?,
        Drawing::from(shape).shapes(&Values::new())?
    );
    Ok(())
}

// The document -----------------------------------------------------------------------------------------------------

#[test]
fn a_gradient_is_written_as_an_object_and_a_colour_as_a_text_and_both_read_back() -> Outcome {
    let drawing = Drawing::from(square()?)
        .fill(Gradient::linear(
            (0.0, 0.0),
            (0.0, 400.0),
            [(0.0, Color::parse("#ff0066")?), (1.0, Color::parse("#ffcc0080")?)],
        )?)
        .stroke(
            Stroke::new(Gradient::radial(
                (200.0, 200.0),
                150.0,
                [(0.0, RED), (0.5, BLUE), (1.0, Color::parse("#00000000")?)],
            )?)
            .width(6.0),
        );
    let text = drawing.to_json()?;
    for expected in [
        r##""color":{"from":[0,0],"gradient":"linear","stops":[[0,"#ff0066"],[1,"#ffcc0080"]],"to":[0,400]}"##,
        r##""center":[200,200],"gradient":"radial","radius":150,"stops":[[0,"#ff0000"],[0.5,"#0000ff"],[1,"#00000000"]]"##,
    ] {
        assert!(text.contains(expected), "{expected} not in {text}");
    }
    let again = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(again.to_json()?, text);
    assert_eq!(again.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    Ok(())
}

fn document(fill: &str) -> String {
    format!(r#"{{"version":1,"root":[{{"path":"M0 0 L10 0 L10 10 Z"}},{{"fill":{fill}}}]}}"#)
}

#[test]
fn a_document_that_writes_a_gradient_wrongly_is_refused_at_the_member() -> Outcome {
    let cases = [
        (
            "an unknown kind",
            r##"{"color":{"gradient":"conic","stops":[[0,"#ff0000"],[1,"#0000ff"]]}}"##,
            "/root/1/fill/color/gradient",
        ),
        (
            "no kind",
            r##"{"color":{"stops":[[0,"#ff0000"],[1,"#0000ff"]],"from":[0,0],"to":[1,0]}}"##,
            "/root/1/fill/color",
        ),
        (
            "no stops",
            r#"{"color":{"gradient":"linear","from":[0,0],"to":[1,0]}}"#,
            "/root/1/fill/color",
        ),
        (
            "no end",
            r##"{"color":{"gradient":"linear","stops":[[0,"#ff0000"],[1,"#0000ff"]],"from":[0,0]}}"##,
            "/root/1/fill/color",
        ),
        (
            "a radial member on a linear",
            r##"{"color":{"gradient":"linear","stops":[[0,"#ff0000"],[1,"#0000ff"]],"from":[0,0],"to":[1,0],"radius":3}}"##,
            "/root/1/fill/color/radius",
        ),
        (
            "an offset out of range",
            r##"{"color":{"gradient":"linear","stops":[[0,"#ff0000"],[2,"#0000ff"]],"from":[0,0],"to":[1,0]}}"##,
            "/root/1/fill/color",
        ),
        (
            "a stop of none",
            r##"{"color":{"gradient":"linear","stops":[[0,"#ff0000"],[1,"none"]],"from":[0,0],"to":[1,0]}}"##,
            "/root/1/fill/color",
        ),
        (
            "a stop colour by name",
            r##"{"color":{"gradient":"linear","stops":[[0,"red"],[1,"#0000ff"]],"from":[0,0],"to":[1,0]}}"##,
            "/root/1/fill/color/stops/0/1",
        ),
        (
            "a point of three numbers",
            r##"{"color":{"gradient":"linear","stops":[[0,"#ff0000"],[1,"#0000ff"]],"from":[0,0,0],"to":[1,0]}}"##,
            "/root/1/fill/color/from",
        ),
        (
            "an expression where a literal belongs",
            r##"{"color":{"gradient":"linear","stops":[[0,"#ff0000"],[1,"#0000ff"]],"from":[{"param":"x"},0],"to":[1,0]}}"##,
            "/root/1/fill/color/from/0",
        ),
        ("a number as the paint", r#"{"color":3}"#, "/root/1/fill/color"),
    ];
    for (case, fill, pointer) in cases {
        match Drawing::from_json(&document(fill), &Inputs::new()) {
            Err(Error::Format { pointer: found, .. }) => assert_eq!(found, pointer, "{case}"),
            other => return Err(format!("{case}: expected a format error, got {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn a_stroke_of_none_in_a_document_is_still_refused() {
    let text = r#"{"version":1,"root":[{"path":"M0 0 L10 0 L10 10 Z"},{"stroke":{"color":"none"}}]}"#;
    assert!(Drawing::from_json(text, &Inputs::new()).is_err());
}
