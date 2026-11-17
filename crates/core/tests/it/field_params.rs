//! Params in a `Field` or a `Flow`, and a drawing where a path was asked for: a `hatch` step reads them with the
//! values it draws with. Oracles: a noise `phase` of one cycle on is the same flow; every line of a hatch along a
//! headline whose weight is a Param lies inside the letters at that weight (`Path::contains`); documents read back
//! to the same lines and the same text.

use crate::common::{Outcome, lines, parameter_refusal};
use bullterrier_vector::{Drawing, Error, Field, Flow, Font, Hatch, Inputs, Param, Path, Text, Values, text};

fn rect() -> Result<Drawing, Error> {
    Ok(Drawing::from(Path::rect(0.0, 0.0, 200.0, 120.0, 0.0)?))
}

fn svg(drawing: &Drawing, values: &Values) -> Result<String, Error> {
    Ok(drawing
        .shapes(values)?
        .iter()
        .map(|shape| shape.path().to_svg_path())
        .collect::<Vec<_>>()
        .join("|"))
}

fn phase() -> Result<Param, Error> {
    Param::new("phase", 0.0, 0.0, 4.0)
}

#[test]
fn a_noise_phase_param_moves_the_lines_and_a_whole_cycle_is_the_same_flow() -> Outcome {
    let phase = phase()?;
    let flow = Flow::noise(120.0, &phase, 3.0)?;
    let drawing = rect()?.hatch(&Hatch::new(8.0).flow(flow), None);
    let at = |value: f64| svg(&drawing, &Values::new().set("phase", value));
    assert_eq!(drawing.parameters()?.len(), 1);
    assert_eq!(at(1.0)?, at(0.0)?, "one cycle on is the same flow");
    assert_ne!(at(0.5)?, at(0.0)?, "half a cycle on is not");
    // the default is the Param's default: the flow outside a step reads it there
    assert_eq!(svg(&drawing, &Values::new())?, at(0.0)?);
    Ok(())
}

#[test]
fn a_param_in_a_field_or_a_mix_is_read_with_the_values_of_the_step() -> Outcome {
    let level = Param::new("level", 0.0, 0.0, 1.0)?;
    let share = Param::new("share", 0.0, 0.0, 1.0)?;
    let flow = Flow::angle(0.0)?.mix(&Flow::angle(90.0)?, &share)?;
    let tone = Field::constant(&level)?;
    let drawing = rect()?.hatch(&Hatch::between(6.0, 18.0, tone).flow(flow), None);
    let at = |level: f64, share: f64| svg(&drawing, &Values::new().set("level", level).set("share", share));
    // a tone of 0 and of 1 are the narrowest and the widest spacing: different line counts
    assert_ne!(at(0.0, 0.0)?, at(1.0, 0.0)?);
    // a mix of 0 and 90 degrees: horizontal lines at 0 (every point of a line at one height), vertical at 1
    for (share, horizontal) in [(0.0, true), (1.0, false)] {
        let drawn = drawing.shapes(&Values::new().set("level", 0.0).set("share", share))?;
        let found = lines(drawn[0].path())?;
        assert_ne!(found.len(), 0);
        for (points, _) in found {
            assert!(
                points.iter().all(|p| {
                    let (a, b) = if horizontal {
                        (p.1, points[0].1)
                    } else {
                        (p.0, points[0].0)
                    };
                    (a - b).abs() < 1e-6
                }),
                "share {share}: a line is not straight along its direction"
            );
        }
    }
    assert_eq!(drawing.parameters()?.len(), 2);
    Ok(())
}

#[test]
fn a_per_item_number_in_a_field_or_flow_is_refused() -> Outcome {
    let by = bullterrier_vector::by_index(0.0, 1.0);
    assert_eq!(parameter_refusal(Flow::angle(by.clone()))?.0, "degrees");
    assert_eq!(parameter_refusal(Flow::noise(100.0, by.clone(), 0.0))?.0, "phase");
    assert_eq!(parameter_refusal(Field::constant(by))?.0, "value");
    Ok(())
}

/// A headline whose weight is a Param, set in a bundled variable font.
fn headline() -> Result<(Drawing, Param), Error> {
    let font = Font::bundled("bricolage_grotesque")?;
    let weight = Param::new("weight", 800.0, 200.0, 800.0)?;
    let word = text(
        "HI",
        Text::new(&font)
            .size(180.0)
            .anchor((20.0, 160.0))
            .axes([("wght", &weight)]),
    )
    .resolve(0.01);
    Ok((word, weight))
}

#[test]
fn a_hatch_along_a_headline_follows_its_weight_param() -> Outcome {
    let (word, _) = headline()?;
    let hatch = Hatch::between(3.0, 8.0, Field::distance(&word, true)?).flow(Flow::contour(&word)?);
    let hatched = word.hatch(&hatch, None);
    assert_eq!(
        hatched.parameters()?.len(),
        1,
        "the weight is read once, by the text, the field and the flow"
    );
    let mut counts = Vec::new();
    for weight in [800.0, 250.0] {
        let values = Values::new().set("weight", weight);
        // the oracle: the same hatch built from the letters at this weight as plain paths, shape by shape
        let letters = word.shapes(&values)?;
        let whole = Path::from_svg_path(
            &letters
                .iter()
                .map(|shape| shape.path().to_svg_path())
                .collect::<Vec<_>>()
                .join(" "),
        )?;
        let at_weight = Hatch::between(3.0, 8.0, Field::distance(&whole, true)?).flow(Flow::contour(&whole)?);
        let drawn = hatched.shapes(&values)?;
        assert_eq!(drawn.len(), letters.len());
        let mut points = 0;
        for (shape, letter) in drawn.iter().zip(&letters) {
            let expected = letter.path().hatch(&at_weight, None)?;
            assert_eq!(shape.path().to_svg_path(), expected.to_svg_path(), "at weight {weight}");
            points += lines(shape.path())?.iter().map(|(line, _)| line.len()).sum::<usize>();
        }
        counts.push(points);
    }
    // heavier letters hold more line
    assert!(counts[0] > counts[1], "{counts:?}");
    Ok(())
}

#[test]
fn a_document_with_params_and_drawings_in_its_fields_reads_back_to_the_same_text_and_lines() -> Outcome {
    let (word, _) = headline()?;
    let phase = phase()?;
    let flow = Flow::contour(&word)?.mix(&Flow::noise(300.0, &phase, 4.0)?, 0.3)?;
    let drawing = word.hatch(
        &Hatch::between(3.0, 8.0, Field::distance(&word, true)?).flow(flow),
        None,
    );
    let text = drawing.to_json()?;
    assert!(
        text.contains("\"Flow.contour\"") && text.contains("\"Field.distance\""),
        "{text}"
    );
    assert!(text.contains("\"phase\":{\"param\":\"phase\"}"), "{text}");
    let back = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(back.to_json()?, text, "written again, the same document");
    for (weight, phase) in [(800.0, 0.0), (300.0, 0.7)] {
        let values = Values::new().set("weight", weight).set("phase", phase);
        assert_eq!(svg(&back, &values)?, svg(&drawing, &values)?);
    }
    Ok(())
}

#[test]
fn a_path_still_goes_where_a_drawing_is_asked_for_and_writes_as_one_path() -> Outcome {
    let ring = Path::circle((100.0, 100.0), 60.0)?;
    let drawing = rect()?.hatch(&Hatch::new(8.0).flow(Flow::contour(&ring)?), None);
    let text = drawing.to_json()?;
    assert!(text.contains("\"Flow.contour\":{\"drawing\":[{\"path\":"), "{text}");
    Ok(())
}

#[test]
fn a_param_a_document_does_not_declare_is_refused_where_the_field_reads_it() -> Outcome {
    let text = r#"{"version":1,"root":[{"path":"M0 0 L50 0 L50 50 L0 50 Z"},
        {"hatch":{"spacing":5,"flow":{"Flow.noise":{"phase":{"param":"ghost"}}}}}]}"#;
    let error = Drawing::from_json(text, &Inputs::new()).err().ok_or("loaded")?;
    assert!(
        error.to_string().contains("/root/1/hatch/flow/Flow.noise/phase"),
        "{error}"
    );
    Ok(())
}
