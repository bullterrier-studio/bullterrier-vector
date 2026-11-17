//! The `rows` step through the public API and its document form. The step is `Path::rows` of each shape's path, so the
//! oracle is the `Path` call; a Param is read with the values of the draw; a document written from the code reads back
//! to a drawing that draws the same lines.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Drawing, Error, Field, Inputs, Param, Path, Rows, Values, drawing_schema};

fn rect() -> Result<Path, Error> {
    Path::rect(0.0, 0.0, 120.0, 90.0, 0.0)
}

fn svg(drawing: &Drawing, values: &Values) -> Result<String, Error> {
    Ok(drawing
        .shapes(values)?
        .iter()
        .map(|shape| shape.path().to_svg_path())
        .collect::<Vec<_>>()
        .join("|"))
}

fn tone() -> Result<Field, Error> {
    Field::ramp((0.0, 0.0), (120.0, 90.0))
}

#[test]
fn the_rows_step_is_the_path_call_of_each_shape() -> Outcome {
    let rows = Rows::new(9.0, tone()?, 14.0)?;
    let drawn = svg(&Drawing::from(rect()?).rows(&rows), &Values::new())?;
    assert_eq!(drawn, rect()?.rows(&rows)?.to_svg_path());
    Ok(())
}

#[test]
fn a_param_in_the_amplitude_or_the_spacing_is_read_with_the_values_of_the_draw() -> Outcome {
    let lift = Param::new("lift", 0.0, 0.0, 40.0)?;
    let gap = Param::new("gap", 10.0, 4.0, 20.0)?;
    let drawing = Drawing::from(rect()?).rows(&Rows::new(&gap, tone()?, &lift)?);
    assert_eq!(drawing.parameters()?.len(), 2);
    let at = |lift: f64, gap: f64| svg(&drawing, &Values::new().set("lift", lift).set("gap", gap));
    let flat = svg(
        &Drawing::from(rect()?).rows(&Rows::new(10.0, tone()?, 0.0)?),
        &Values::new(),
    )?;
    assert_eq!(
        svg(&drawing, &Values::new())?,
        flat,
        "the defaults are the Params' defaults"
    );
    assert_ne!(at(30.0, 10.0)?, flat);
    assert_ne!(at(0.0, 5.0)?, flat);
    Ok(())
}

#[test]
fn a_per_item_number_is_refused_by_name() -> Outcome {
    let refusal = parameter_refusal(Rows::new(bullterrier_vector::by_index(1.0, 2.0), tone()?, 1.0))?;
    assert_eq!(refusal.0, "spacing");
    Ok(())
}

#[test]
fn a_document_reads_back_to_the_same_rows_and_the_same_text() -> Outcome {
    let lift = Param::new("lift", 12.0, 0.0, 40.0)?;
    let drawing = Drawing::from(rect()?).rows(&Rows::new(8.0, tone()?, &lift)?);
    let text = drawing.to_json()?;
    assert!(text.contains("\"rows\""), "{text}");
    let back = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(svg(&back, &Values::new())?, svg(&drawing, &Values::new())?);
    assert_eq!(back.to_json()?, text);
    Ok(())
}

#[test]
fn the_schema_describes_rows_and_a_bad_document_is_refused_with_a_pointer() -> Outcome {
    assert!(drawing_schema().contains("\"rows\""));
    let call =
        |args: &str| format!(r#"{{"version":1,"root":[{{"path":"M0 0 L50 0 L50 50 L0 50 Z"}},{{"rows":{args}}}]}}"#);
    for (args, pointer) in [
        (r#"{"spacing":0,"tone":0.5,"amplitude":1}"#, "/root/1/rows/spacing"),
        (r#"{"spacing":5,"amplitude":1}"#, "/root/1/rows"),
        (r#"{"spacing":5,"tone":"dark","amplitude":1}"#, "/root/1/rows/tone"),
        (r#"{"spacing":5,"tone":0.5}"#, "/root/1/rows"),
        (
            r#"{"spacing":5,"tone":0.5,"amplitude":{"random":{"low":0,"high":9}}}"#,
            "/root/1/rows/amplitude/random",
        ),
    ] {
        match Drawing::from_json(&call(args), &Inputs::new()) {
            Err(Error::Format { pointer: got, .. }) => assert_eq!(got, pointer, "{args}"),
            other => return Err(format!("{args}: expected a format error, got {other:?}").into()),
        }
    }
    Ok(())
}
