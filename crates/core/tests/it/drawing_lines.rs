//! The `rows` step through the public API and its document form. The step is `Path::rows` of each shape's path, so the
//! oracle is the `Path` call; a Param is read with the values of the draw; a document written from the code reads back
//! to a drawing that draws the same lines.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{
    Bands, Drawing, Error, Family, Field, Inputs, Param, Path, Rows, Stipple, Values, drawing_schema,
};

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

// bands -------------------------------------------------------------------------------------------------------------

fn every_family() -> Result<Vec<Bands>, Error> {
    let t = || Field::ramp((0.0, 0.0), (120.0, 90.0));
    Ok(vec![
        Bands::lines(9.0, t()?, (0.5, 4.0))?.angle(20.0)?,
        Bands::waves(9.0, t()?, (0.5, 4.0), 30.0, 3.0)?,
        Bands::rings(9.0, t()?, (0.5, 4.0))?.center((30.0, 20.0))?,
        Bands::spiral(9.0, t()?, (0.5, 4.0))?,
        Bands::rays(9.0, t()?, (0.5, 4.0))?.angle(10.0)?,
    ])
}

#[test]
fn the_bands_step_is_the_path_call_of_each_shape_for_every_family() -> Outcome {
    for bands in every_family()? {
        let drawn = svg(&Drawing::from(rect()?).bands(&bands), &Values::new())?;
        assert_eq!(drawn, rect()?.bands(&bands)?.to_svg_path());
        assert_ne!(drawn, "");
    }
    Ok(())
}

#[test]
fn a_param_in_a_bands_number_is_read_with_the_values_of_the_draw() -> Outcome {
    let thick = Param::new("thick", 1.0, 0.5, 6.0)?;
    let turn = Param::new("turn", 0.0, 0.0, 90.0)?;
    let drawing = Drawing::from(rect()?).bands(&Bands::lines(9.0, tone()?, (0.5, &thick))?.angle(&turn)?);
    assert_eq!(drawing.parameters()?.len(), 2);
    let at = |thick: f64, turn: f64| svg(&drawing, &Values::new().set("thick", thick).set("turn", turn));
    assert_eq!(at(1.0, 0.0)?, svg(&drawing, &Values::new())?);
    assert_ne!(at(5.0, 0.0)?, at(1.0, 0.0)?);
    assert_ne!(at(1.0, 45.0)?, at(1.0, 0.0)?);
    Ok(())
}

#[test]
fn a_bands_document_reads_back_to_the_same_outlines_and_the_same_text() -> Outcome {
    for bands in every_family()? {
        let drawing = Drawing::from(rect()?).bands(&bands);
        let text = drawing.to_json()?;
        let back = Drawing::from_json(&text, &Inputs::new())?;
        assert_eq!(svg(&back, &Values::new())?, svg(&drawing, &Values::new())?, "{text}");
        assert_eq!(back.to_json()?, text);
    }
    Ok(())
}

#[test]
fn a_bad_bands_document_is_refused_with_a_pointer() -> Outcome {
    let call =
        |args: &str| format!(r#"{{"version":1,"root":[{{"path":"M0 0 L50 0 L50 50 L0 50 Z"}},{{"bands":{args}}}]}}"#);
    for (args, pointer) in [
        (r#"{"spacing":5,"tone":0.5}"#, "/root/1/bands"),
        (
            r#"{"spacing":5,"tone":0.5,"width":[1,2],"family":"zigzag"}"#,
            "/root/1/bands/family",
        ),
        (
            r#"{"spacing":5,"tone":0.5,"width":[1,2],"family":"waves"}"#,
            "/root/1/bands/wavelength",
        ),
        (r#"{"spacing":5,"tone":0.5,"width":[-1,2]}"#, "/root/1/bands/width/0"),
        (
            r#"{"spacing":5,"tone":0.5,"width":[1,2],"center":[1]}"#,
            "/root/1/bands/center",
        ),
        (
            r#"{"spacing":5,"tone":0.5,"width":[1,2],"wavelength":0,"family":"waves","amplitude":1}"#,
            "/root/1/bands/wavelength",
        ),
    ] {
        match Drawing::from_json(&call(args), &Inputs::new()) {
            Err(Error::Format { pointer: got, .. }) => assert_eq!(got, pointer, "{args}"),
            other => return Err(format!("{args}: expected a format error, got {other:?}").into()),
        }
    }
    Ok(())
}

// stipple -----------------------------------------------------------------------------------------------------------

#[test]
fn the_stipple_step_is_the_path_call_and_a_param_is_read_with_the_values_of_the_draw() -> Outcome {
    let stipple = Stipple::new(6.0, tone()?, (1.0, 1.0))?.density(1.0, 0.1).seed(3.0);
    let drawn = svg(&Drawing::from(rect()?).stipple(&stipple), &Values::new())?;
    assert_eq!(drawn, rect()?.stipple(&stipple)?.to_svg_path());
    let dot = Param::new("dot", 1.0, 0.5, 3.0)?;
    let seed = Param::new("seed", 0.0, 0.0, 9.0)?;
    let drawing = Drawing::from(rect()?).stipple(&Stipple::new(6.0, tone()?, (&dot, &dot))?.seed(&seed));
    let at = |dot: f64, seed: f64| svg(&drawing, &Values::new().set("dot", dot).set("seed", seed));
    assert_ne!(at(2.0, 0.0)?, at(1.0, 0.0)?);
    assert_ne!(at(1.0, 4.0)?, at(1.0, 0.0)?);
    // a document reads back to the same dots and the same text
    let text = drawing.to_json()?;
    let back = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(svg(&back, &Values::new())?, svg(&drawing, &Values::new())?);
    assert_eq!(back.to_json()?, text);
    let call =
        |args: &str| format!(r#"{{"version":1,"root":[{{"path":"M0 0 L50 0 L50 50 L0 50 Z"}},{{"stipple":{args}}}]}}"#);
    for (args, pointer) in [
        (r#"{"spacing":5,"tone":0.5}"#, "/root/1/stipple"),
        (
            r#"{"spacing":5,"tone":0.5,"radius":1,"density":[0,2]}"#,
            "/root/1/stipple/density/1",
        ),
        (r#"{"spacing":5,"tone":0.5,"radius":0}"#, "/root/1/stipple/radius"),
        (r#"{"spacing":5,"tone":0.5,"radius":[1,0]}"#, "/root/1/stipple/radius/1"),
        (r#"{"spacing":5,"tone":0.5,"radius":[1,2,3]}"#, "/root/1/stipple/radius"),
    ] {
        match Drawing::from_json(&call(args), &Inputs::new()) {
            Err(Error::Format { pointer: got, .. }) => assert_eq!(got, pointer, "{args}"),
            other => return Err(format!("{args}: expected a format error, got {other:?}").into()),
        }
    }
    Ok(())
}

// a width of 0 and the arguments a family does not read ---------------------------------------------------------------

#[test]
fn a_width_of_nothing_fades_a_band_away_in_the_path_the_step_and_a_document() -> Outcome {
    // tone 0 at the left, 1 at the right: the outline is a point at the left and 4 wide at the right
    let ramp = Field::ramp((0.0, 0.0), (120.0, 0.0))?;
    let bands = Bands::lines(30.0, ramp, (0.0, 4.0))?;
    let by_path = rect()?.bands(&bands)?;
    assert_eq!(
        svg(&Drawing::from(rect()?).bands(&bands), &Values::new())?,
        by_path.to_svg_path()
    );
    assert!(
        by_path.contains((110.0, 15.0))? && !by_path.contains((2.0, 17.0))?,
        "thick at the right, nothing at the left"
    );
    let text = Drawing::from(rect()?).bands(&bands).to_json()?;
    let back = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(svg(&back, &Values::new())?, by_path.to_svg_path());
    Ok(())
}

#[test]
fn an_argument_the_family_does_not_read_is_refused_naming_it_and_the_family() -> Outcome {
    let flat = || Field::constant(0.5);
    let refused = |bands: Result<Bands, Error>| -> Result<(String, String), Box<dyn std::error::Error>> {
        parameter_refusal(bands)
    };
    for (what, bands, argument) in [
        (
            "lines",
            Bands::lines(5.0, flat()?, (1.0, 2.0))?.center((1.0, 1.0)),
            "center",
        ),
        (
            "waves",
            Bands::waves(5.0, flat()?, (1.0, 2.0), 10.0, 1.0)?.center((1.0, 1.0)),
            "center",
        ),
        ("rings", Bands::rings(5.0, flat()?, (1.0, 2.0))?.angle(10.0), "angle"),
    ] {
        let (name, message) = refused(bands)?;
        assert_eq!(name, argument);
        assert!(message.contains(what), "{message}");
    }
    assert!(Family::parse("lines")?.refuse_unread("wavelength").is_err());
    assert!(Family::parse("rays")?.refuse_unread("amplitude").is_err());
    assert!(
        Family::parse("waves")?.refuse_unread("wavelength").is_ok()
            && Family::parse("spiral")?.refuse_unread("angle").is_ok()
    );
    let call =
        |args: &str| format!(r#"{{"version":1,"root":[{{"path":"M0 0 L50 0 L50 50 L0 50 Z"}},{{"bands":{args}}}]}}"#);
    for (args, pointer) in [
        (
            r#"{"spacing":5,"tone":0.5,"width":[1,2],"wavelength":9}"#,
            "/root/1/bands/wavelength",
        ),
        (
            r#"{"spacing":5,"tone":0.5,"width":[1,2],"center":[1,1]}"#,
            "/root/1/bands/center",
        ),
        (
            r#"{"spacing":5,"tone":0.5,"width":[1,2],"family":"rings","angle":0}"#,
            "/root/1/bands/angle",
        ),
    ] {
        match Drawing::from_json(&call(args), &Inputs::new()) {
            Err(Error::Format { pointer: got, message }) => {
                assert_eq!(got, pointer, "{args}");
                assert!(message.contains("family"), "{message}");
            }
            other => return Err(format!("{args}: expected a format error, got {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn a_radius_pair_is_written_as_a_pair_and_one_size_as_a_number() -> Outcome {
    let one = Drawing::from(rect()?).stipple(&Stipple::new(6.0, tone()?, (1.5, 1.5))?);
    let pair = Drawing::from(rect()?).stipple(&Stipple::new(6.0, tone()?, (0.5, 2.5))?);
    assert!(one.to_json()?.contains(r#""radius":1.5"#), "{}", one.to_json()?);
    assert!(pair.to_json()?.contains(r#""radius":[0.5,2.5]"#), "{}", pair.to_json()?);
    for drawing in [one, pair] {
        let back = Drawing::from_json(&drawing.to_json()?, &Inputs::new())?;
        assert_eq!(svg(&back, &Values::new())?, svg(&drawing, &Values::new())?);
    }
    // the step is the path call with the pair
    let stipple = Stipple::new(6.0, tone()?, (0.5, 2.5))?;
    assert_eq!(
        svg(&Drawing::from(rect()?).stipple(&stipple), &Values::new())?,
        rect()?.stipple(&stipple)?.to_svg_path()
    );
    Ok(())
}
