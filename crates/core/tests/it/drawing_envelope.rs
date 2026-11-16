//! `Drawing::envelope` and `Drawing::arch` through the public API. The usage first: a drawing bent between two curves,
//! every shape sharing the frame of the whole drawing. Expected vertices are by hand from the definition (a point goes to
//! `lerp(T(u), B(u), v)`, `u` and `v` its shares of the frame).

use crate::common::fonts::fraunces;
use crate::common::{Outcome, format_refusal, has_vertex, lines, parameter_refusal, to_lines};
use bullterrier_vector::{Arch, Drawing, Envelope, Inputs, Param, Path, Rect, Values, by_index, text};

fn rect(x: f64, y: f64, width: f64, height: f64) -> Result<Path, bullterrier_vector::Error> {
    Path::rect(x, y, width, height, 0.0)
}

fn slope() -> Result<Path, bullterrier_vector::Error> {
    Path::line((0.0, 0.0), (100.0, 20.0))
}

#[test]
fn a_square_bent_under_a_slanted_top_moves_its_corners_by_hand() -> Outcome {
    // frame = its bounds (0, 0, 100, 100); the top runs to (100, 20), the bottom is the frame's own
    let bent = Drawing::from(rect(0.0, 0.0, 100.0, 100.0)?).envelope(&slope()?, Envelope::new());
    let shapes = bent.shapes(&Values::new())?;
    let found = lines(shapes[0].path())?;
    for corner in [(0.0, 0.0), (100.0, 20.0), (100.0, 100.0), (0.0, 100.0)] {
        assert!(has_vertex(&found, corner), "no vertex at {corner:?}");
    }
    Ok(())
}

#[test]
fn every_shape_of_the_drawing_shares_one_frame_and_an_explicit_frame_overrides_it() -> Outcome {
    // two bars, 0 to 10 and 50 to 60: the frame is (0, 0, 100, 60) for both
    let bars = Drawing::new([rect(0.0, 0.0, 100.0, 10.0)?, rect(0.0, 50.0, 100.0, 10.0)?]);
    let shapes = bars.arch(0.2).shapes(&Values::new())?;
    // the first bar's top edge is the frame's: its middle rises 4 * 0.2 * 100 * 0.25 = 20
    assert!(has_vertex(&lines(shapes[0].path())?, (50.0, -20.0)));
    // the second is at v = 50 / 60: (1 - 5/6) * (-20) + 5/6 * 60 = 46.667 at its top edge's middle
    let second = lines(shapes[1].path())?;
    assert!(to_lines((50.0, 140.0 / 3.0), &second) < 1e-6, "{second:?}");
    // a frame of the second bar alone makes its top edge the arch
    let alone = bars
        .arch(Arch::new(0.2).frame(Rect {
            x: 0.0,
            y: 50.0,
            width: 100.0,
            height: 10.0,
        }))
        .shapes(&Values::new())?;
    assert!(has_vertex(&lines(alone[1].path())?, (50.0, 30.0)));
    Ok(())
}

#[test]
fn each_shape_reads_its_own_amount() -> Outcome {
    let square = rect(0.0, 0.0, 100.0, 100.0)?;
    let both = Drawing::new([square.clone(), square]);
    let shapes = both
        .envelope(&slope()?, Envelope::new().amount(by_index(0.0, 1.0)))
        .shapes(&Values::new())?;
    let (flat, bent) = (lines(shapes[0].path())?, lines(shapes[1].path())?);
    assert!(has_vertex(&flat, (100.0, 0.0)), "{flat:?}");
    assert!(has_vertex(&bent, (100.0, 20.0)), "{bent:?}");
    Ok(())
}

#[test]
fn a_param_amount_is_read_when_drawn() -> Outcome {
    let amount = Param::new("amount", 0.0, 0.0, 1.0)?;
    let bent = Drawing::from(rect(0.0, 0.0, 100.0, 100.0)?).envelope(&slope()?, Envelope::new().amount(&amount));
    let corner = |values: &Values| -> Result<bool, Box<dyn std::error::Error>> {
        Ok(has_vertex(&lines(bent.shapes(values)?[0].path())?, (100.0, 10.0)))
    };
    assert!(!corner(&Values::new())?);
    assert!(corner(&Values::new().set("amount", 0.5))?);
    Ok(())
}

#[test]
fn a_word_arched_rises_in_the_middle_and_keeps_its_style() -> Outcome {
    let word = text("Hif", &fraunces()?);
    let (before, after) = (word.bounds(&Values::new())?, word.arch(0.3).bounds(&Values::new())?);
    // the top edge of the frame is at the top of the word; its middle goes 0.3 widths up, the glyphs near the middle most
    assert!(after.y < before.y - 0.2 * before.width, "{before:?} {after:?}");
    let (a, b) = (word.shapes(&Values::new())?, word.arch(0.3).shapes(&Values::new())?);
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(&b) {
        assert_eq!(a.style(), b.style());
        assert_eq!(a.attributes(), b.attributes());
    }
    Ok(())
}

#[test]
fn the_steps_save_and_load_the_same_drawing() -> Outcome {
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    let square = Drawing::from(rect(0.0, 0.0, 100.0, 100.0)?);
    let drawings = [
        square.envelope(
            &slope()?,
            Envelope::new()
                .bottom(&Path::line((0.0, 100.0), (100.0, 130.0))?)
                .frame(frame)
                .amount(by_index(0.5, 1.0)),
        ),
        square.arch(Arch::new(by_index(0.1, 0.4)).frame(frame)),
        square.arch(0.2),
    ];
    for drawing in drawings {
        let json = drawing.to_json()?;
        let again = Drawing::from_json(&json, &Inputs::new())?;
        assert_eq!(again.to_json()?, json);
        assert_eq!(again.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    }
    let json = square.envelope(&slope()?, Envelope::new().frame(frame)).to_json()?;
    assert!(
        json.contains(r#""envelope":{"#) && json.contains(r#""frame":[0,0,100,100]"#),
        "{json}"
    );
    assert!(json.contains(r#""top":"M0 0 L100 20""#), "{json}");
    Ok(())
}

#[test]
fn a_document_frame_of_the_wrong_length_is_refused() -> Outcome {
    let document = |frame: &str| {
        format!(
            r#"{{"version":1,"root":[{{"path":"M0 0 L10 0 L10 10 Z"}},{{"arch":{{"bend":0.2,"frame":{frame}}}}}]}}"#
        )
    };
    assert!(Drawing::from_json(&document("[0,0,10,10]"), &Inputs::new()).is_ok());
    let (pointer, message) = format_refusal(Drawing::from_json(&document("[0,0,10]"), &Inputs::new()))?;
    assert!(
        pointer.ends_with("/frame") && message.contains("four"),
        "{pointer}: {message}"
    );
    Ok(())
}

#[test]
fn refusals_name_the_step_and_the_argument() -> Outcome {
    let square = Drawing::from(rect(0.0, 0.0, 100.0, 100.0)?);
    let (place, message) = parameter_refusal(
        square
            .envelope(
                &slope()?,
                Envelope::new().frame(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 5.0,
                }),
            )
            .shapes(&Values::new()),
    )?;
    assert!(
        place.contains("step 1 (envelope)") && place.ends_with("frame"),
        "{place}: {message}"
    );
    let error = square
        .envelope(&slope()?, Envelope::new().amount(f64::NAN))
        .error()
        .map(ToString::to_string)
        .ok_or("no error")?;
    assert!(error.contains("amount"), "{error}");
    let error = square
        .arch(f64::INFINITY)
        .error()
        .map(ToString::to_string)
        .ok_or("no error")?;
    assert!(error.contains("bend"), "{error}");
    // a curve with no length is a geometry error, placed at the step
    let nothing = square.envelope(&Path::from_svg_path("M5 5")?, Envelope::new());
    let error = nothing.shapes(&Values::new()).err().ok_or("drew")?.to_string();
    assert!(error.contains("step 1 (envelope)"), "{error}");
    // too many pieces: charged to MAX_SEGMENTS
    let long = Drawing::from(Path::line((0.0, 0.0), (1e9, 0.5))?);
    let (place, message) = parameter_refusal(
        long.arch(Arch::new(0.2).frame(Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }))
        .shapes(&Values::new()),
    )?;
    assert!(
        place.contains("step 1 (arch)") && message.contains("MAX_SEGMENTS"),
        "{place}: {message}"
    );
    Ok(())
}
