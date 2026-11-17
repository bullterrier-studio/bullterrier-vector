//! `Drawing::hatch` through the public API and its document form. The step is `Path::hatch` of each shape's path, so
//! the oracle is the `Path` call; a document written from the code reads back to a drawing that draws the same lines.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Drawing, Field, Flow, Hatch, Inputs, Path, Values, drawing_schema};

fn rect(width: f64, height: f64) -> Path {
    Path::rect(0.0, 0.0, width, height, 0.0).unwrap()
}

fn svg(drawing: &Drawing) -> Result<String, bullterrier_vector::Error> {
    Ok(drawing.shapes(&Values::new())?.iter().map(|shape| shape.path().to_svg_path()).collect::<Vec<_>>().join("|"))
}

fn flows() -> Vec<Flow> {
    let ring = Path::circle((100.0, 100.0), 80.0).unwrap();
    vec![
        Flow::angle(30.0).unwrap(),
        Flow::noise(120.0, 0.25, 5.0).unwrap(),
        Flow::contour(&ring).unwrap(),
        Flow::angle(0.0).unwrap().mix(&Flow::contour(&ring).unwrap(), 0.3).unwrap(),
    ]
}

#[test]
fn the_step_is_the_path_call_of_each_shape() -> Outcome {
    let path = rect(120.0, 90.0);
    for flow in flows() {
        let hatch = Hatch::new(9.0).flow(flow);
        let drawn = svg(&Drawing::from(path.clone()).hatch(&hatch, None))?;
        assert_eq!(drawn, path.hatch(&hatch, None)?.to_svg_path());
    }
    Ok(())
}

#[test]
fn the_step_avoids_the_shapes_of_another_drawing() -> Outcome {
    let hole = Path::circle((60.0, 45.0), 30.0)?;
    let hatch = Hatch::new(8.0).flow(Flow::angle(10.0)?);
    let drawn = svg(&Drawing::from(rect(120.0, 90.0)).hatch(&hatch, Some(&Drawing::from(hole.clone()))))?;
    assert_eq!(drawn, rect(120.0, 90.0).hatch(&hatch, Some(&hole))?.to_svg_path());
    assert_ne!(drawn, svg(&Drawing::from(rect(120.0, 90.0)).hatch(&hatch, None))?);
    Ok(())
}

#[test]
fn another_flow_or_tone_is_another_step() -> Outcome {
    // the result cache tells steps apart by their values: two flows must not share one answer
    let base = Drawing::from(rect(100.0, 100.0));
    let a = svg(&base.hatch(&Hatch::new(7.0).flow(Flow::noise(100.0, 0.0, 1.0)?), None))?;
    let b = svg(&base.hatch(&Hatch::new(7.0).flow(Flow::noise(100.0, 0.0, 2.0)?), None))?;
    assert_ne!(a, b);
    let tone = |at: f64| Field::ramp((0.0, 0.0), (at, 0.0));
    let c = svg(&base.hatch(&Hatch::between(5.0, 15.0, tone(100.0)?), None))?;
    let d = svg(&base.hatch(&Hatch::between(5.0, 15.0, tone(50.0)?), None))?;
    assert_ne!(c, d);
    Ok(())
}

#[test]
fn a_document_reads_back_to_the_same_lines() -> Outcome {
    let ring = Path::circle((100.0, 100.0), 80.0)?;
    let tone = Field::distance(&ring, true)?;
    for flow in flows() {
        let drawing = Drawing::from(rect(200.0, 200.0)).hatch(&Hatch::between(6.0, 12.0, tone.clone()).flow(flow), Some(&Drawing::from(ring.clone())));
        let text = drawing.to_json()?;
        let back = Drawing::from_json(&text, &Inputs::new())?;
        assert_eq!(svg(&back)?, svg(&drawing)?);
        assert_eq!(back.to_json()?, text, "written again, the same document");
    }
    let constant = Drawing::from(rect(50.0, 50.0)).hatch(&Hatch::between(4.0, 9.0, Field::constant(0.5)?), None);
    let text = constant.to_json()?;
    assert!(text.contains("\"tone\":0.5") && text.contains("\"spacing\":[4,9]"), "{text}");
    Ok(())
}

#[test]
fn a_document_with_a_bad_flow_or_field_is_refused_with_a_pointer() {
    let call = |args: &str| format!(r#"{{"version":1,"root":[{{"path":"M0 0 L50 0 L50 50 L0 50 Z"}},{{"hatch":{args}}}]}}"#);
    for (args, pointer) in [
        (r#"{"spacing":5,"flow":{"flow":"angle"}}"#, "/root/1/hatch/flow"),
        (r#"{"spacing":5,"flow":{"flow":"noise","wavelength":0}}"#, "/root/1/hatch/flow"),
        (r#"{"spacing":5,"flow":{"flow":"mix","parts":[{"flow":"angle","angle":1}]}}"#, "/root/1/hatch/flow/parts/0"),
        (r#"{"spacing":5,"tone":{"field":"ramp","from":[1,1],"to":[1,1]}}"#, "/root/1/hatch/tone"),
        (r#"{"spacing":5,"tone":{"field":"distance","path":"M0 0"}}"#, "/root/1/hatch/tone"),
        (r#"{"spacing":5,"tone":"dark"}"#, "/root/1/hatch/tone"),
        (r#"{"spacing":0}"#, "/root/1/hatch/spacing"),
    ] {
        let refusal = Drawing::from_json(&call(args), &Inputs::new());
        match refusal {
            Err(bullterrier_vector::Error::Format { pointer: found, .. }) => assert!(found.starts_with(pointer), "{args}: {found}"),
            other => panic!("{args}: {other:?}"),
        }
    }
}

#[test]
fn a_pair_of_spacings_needs_a_tone() {
    let drawing = Drawing::from(rect(50.0, 50.0));
    let refused = drawing.hatch(&Hatch::between(4.0, 9.0, Field::constant(0.5).unwrap()), None);
    assert!(refused.error().is_none());
    let text = r#"{"version":1,"root":[{"path":"M0 0 L50 0 L50 50 L0 50 Z"},{"hatch":{"spacing":[4,9]}}]}"#;
    assert!(Drawing::from_json(text, &Inputs::new()).is_err());
}

#[test]
fn a_bad_spacing_is_a_parameter_error_when_drawn() {
    let drawing = Drawing::from(rect(50.0, 50.0)).hatch(&Hatch::new(-1.0), None);
    assert!(parameter_refusal(drawing.shapes(&Values::new())).is_ok() || drawing.error().is_some());
}

#[test]
fn the_schema_names_the_flow_and_the_field() {
    let schema = drawing_schema();
    for needle in ["\"field\"", "\"flow\"", "\"hatch\""] {
        assert!(schema.contains(needle), "{needle}");
    }
}
