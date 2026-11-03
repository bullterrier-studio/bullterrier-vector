//! The inputs of a document: a font named in `inputs` and supplied as bytes by whoever loads it. The checks
//! `input-declared`, `input-used` and `inputs-supplied` of formats.md, each with a document that passes and a document
//! that fails; the writer's `inputs` object (names, one entry per byte string, `sha256`); and the round trip of a text.
//!
//! The SHA-256 literals are `shasum -a 256 crates/core/tests/it/fonts/fraunces-subset.ttf` and `.../synthetic.ttf`.

use crate::common::fonts::{FRAUNCES, SYNTHETIC, fraunces, synthetic};
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Drawing, Error, Font, Inputs, Size, Text, Values, text};
use proptest::prelude::*;

const FRAUNCES_SHA: &str = "2c195cc254a3208dedaf708afac6634f8bda80c9346c06fa07f86a44462c805f";
const SYNTHETIC_SHA: &str = "036d2037d950dac9a8ce1455b79f941f5833a001d73cc0210ae6b6286d3cc659";

fn one_font() -> Inputs {
    Inputs::new().set("anybody", FRAUNCES)
}

/// A document whose root is one text in the input `font`, with `inputs` declared as given.
fn document(declared: &str, font: &str) -> String {
    format!(r#"{{"version":1,"inputs":{{{declared}}},"root":[{{"text":{{"content":"Hi","font":"{font}"}}}}]}}"#)
}

const ANYBODY: &str = r#""anybody":{"kind":"font"}"#;

/// The pointer and message of the `Error::Format` that loading `text` with `inputs` gave.
fn format_refusal(text: &str, inputs: &Inputs) -> Result<(String, String), Box<dyn std::error::Error>> {
    match Drawing::from_json(text, inputs) {
        Err(Error::Format { pointer, message }) => Ok((pointer, message)),
        Err(other) => Err(format!("expected a format error, got {other}").into()),
        Ok(_) => Err("expected a format error, but it loaded".into()),
    }
}

// Loading -----------------------------------------------------------------------------------------------------------

#[test]
fn a_document_loads_with_its_font_and_draws_what_the_code_draws() -> Outcome {
    let loaded = Drawing::from_json(&document(ANYBODY, "anybody"), &one_font())?;
    let built = text("Hi", &fraunces()?);
    assert_eq!(loaded.shapes(&Values::new())?, built.shapes(&Values::new())?);
    Ok(())
}

#[test]
fn a_font_with_its_digest_given_loads_when_the_bytes_match() -> Outcome {
    let declared = format!(r#""anybody":{{"kind":"font","sha256":"{FRAUNCES_SHA}"}}"#);
    Drawing::from_json(&document(&declared, "anybody"), &one_font())?;
    Ok(())
}

// input-declared, input-used ----------------------------------------------------------------------------------------

#[test]
fn input_declared_refuses_a_font_no_entry_declares_and_one_declared_as_a_map() -> Outcome {
    let rows = [
        (
            document(ANYBODY, "other"),
            "/root/0/text/font",
            "\"other\" is not an entry of `inputs`",
        ),
        (
            document(r#""anybody":{"kind":"map"}"#, "anybody"),
            "/root/0/text/font",
            "\"anybody\" is declared as a map, not a font",
        ),
    ];
    for (text, pointer, message) in rows {
        assert_eq!(
            format_refusal(&text, &one_font())?,
            (pointer.to_owned(), message.to_owned()),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn input_used_refuses_a_declared_input_no_call_reads() -> Outcome {
    let text = r#"{"version":1,"inputs":{"anybody":{"kind":"font"},"spare":{"kind":"font"}},"root":[{"text":{"content":"Hi","font":"anybody"}}]}"#;
    let inputs = Inputs::new().set("anybody", FRAUNCES).set("spare", SYNTHETIC);
    assert_eq!(
        format_refusal(text, &inputs)?,
        ("/inputs/spare".to_owned(), "is read by no call".to_owned())
    );
    Ok(())
}

#[test]
fn a_font_is_read_wherever_a_drawing_is_taken_or_shared() -> Outcome {
    let text = r#"{"version":1,"inputs":{"anybody":{"kind":"font"}},"drawings":{"d1":[{"text":{"content":"H","font":"anybody"}}]},"root":[{"stack":{"items":[[{"drawing":"d1"}],[{"drawing":"d1"},{"translate":{"dx":5,"dy":0}}]]}}]}"#;
    assert_eq!(Drawing::from_json(text, &one_font())?.shapes(&Values::new())?.len(), 2);
    Ok(())
}

// inputs-supplied ---------------------------------------------------------------------------------------------------

#[test]
fn inputs_supplied_refuses_what_the_caller_gives_wrongly_naming_the_input() -> Outcome {
    let two = r#"{"version":1,"inputs":{"a":{"kind":"font"},"b":{"kind":"font"}},"root":[{"stack":{"items":[[{"text":{"content":"H","font":"a"}}],[{"text":{"content":"H","font":"b"}}]]}}]}"#.to_owned();
    let wrong_digest = format!(r#""anybody":{{"kind":"font","sha256":"{SYNTHETIC_SHA}"}}"#);
    let rows: [(String, Inputs, &str, &str); 5] = [
        (
            document(ANYBODY, "anybody"),
            Inputs::new(),
            "inputs.anybody",
            "does not give it",
        ),
        (
            document(ANYBODY, "anybody"),
            one_font().set("extra", FRAUNCES),
            "inputs.extra",
            "declares no input of this name",
        ),
        (
            two,
            Inputs::new().set("a", FRAUNCES).set("b", FRAUNCES),
            "inputs.b",
            "has the same bytes as \"a\"",
        ),
        (
            document(&wrong_digest, "anybody"),
            one_font(),
            "inputs.anybody",
            &format!("has the SHA-256 {FRAUNCES_SHA}, but the document says {SYNTHETIC_SHA}"),
        ),
        (
            document(ANYBODY, "anybody"),
            Inputs::new().set("anybody", b"not a font".to_vec()),
            "inputs.anybody",
            "not a font",
        ),
    ];
    for (text, inputs, parameter, message) in rows {
        let (found, said) = parameter_refusal(Drawing::from_json(&text, &inputs))?;
        assert_eq!(found, parameter, "{text}");
        assert!(said.contains(message), "{said}");
    }
    Ok(())
}

// to_json -----------------------------------------------------------------------------------------------------------

#[test]
fn a_loaded_font_keeps_its_name_and_the_digest_is_written() -> Outcome {
    let loaded = Drawing::from_json(&document(ANYBODY, "anybody"), &one_font())?;
    assert_eq!(
        loaded.to_json()?,
        format!(
            r#"{{"inputs":{{"anybody":{{"kind":"font","sha256":"{FRAUNCES_SHA}"}}}},"root":[{{"text":{{"content":"Hi","font":"anybody"}}}}],"version":1}}"#
        )
    );
    Ok(())
}

#[test]
fn a_font_without_a_name_is_called_font1_font2_in_the_order_the_walk_meets_them() -> Outcome {
    let drawing = Drawing::new([text("H", &fraunces()?), text("A", &synthetic()?)]);
    let json = drawing.to_json()?;
    assert!(
        json.starts_with(&format!(
            r#"{{"inputs":{{"font1":{{"kind":"font","sha256":"{FRAUNCES_SHA}"}},"font2":{{"kind":"font","sha256":"{SYNTHETIC_SHA}"}}}},"#
        )),
        "{json}"
    );
    assert!(json.contains(r#"{"text":{"content":"H","font":"font1"}}"#), "{json}");
    assert!(json.contains(r#"{"text":{"content":"A","font":"font2"}}"#), "{json}");
    Ok(())
}

#[test]
fn equal_bytes_are_one_input_whatever_objects_the_code_made() -> Outcome {
    let drawing = Drawing::new([text("H", &fraunces()?), text("A", &fraunces()?.named("anybody")?)]);
    let json = drawing.to_json()?;
    assert_eq!(json.matches(r#""kind":"font""#).count(), 1, "{json}");
    assert!(json.contains(r#""anybody":{"kind":"font""#), "{json}");
    assert_eq!(json.matches(r#""font":"anybody""#).count(), 2, "{json}");
    Ok(())
}

#[test]
fn a_font_name_the_drawing_gives_is_skipped_for_the_automatic_ones() -> Outcome {
    let drawing = Drawing::new([text("H", &fraunces()?), text("A", &synthetic()?.named("font1")?)]);
    let json = drawing.to_json()?;
    assert!(
        json.contains(&format!(r#""font1":{{"kind":"font","sha256":"{SYNTHETIC_SHA}"}}"#)),
        "{json}"
    );
    assert!(
        json.contains(&format!(r#""font2":{{"kind":"font","sha256":"{FRAUNCES_SHA}"}}"#)),
        "{json}"
    );
    Ok(())
}

#[test]
fn one_byte_string_with_two_names_or_one_name_for_two_byte_strings_cannot_be_written() -> Outcome {
    let two_names = Drawing::new([
        text("H", &fraunces()?.named("one")?),
        text("A", &fraunces()?.named("two")?),
    ]);
    let (parameter, message) = parameter_refusal(two_names.to_json())?;
    assert_eq!(parameter, "to_json");
    assert!(message.contains("\"one\"") && message.contains("\"two\""), "{message}");
    let one_name = Drawing::new([
        text("H", &fraunces()?.named("same")?),
        text("A", &synthetic()?.named("same")?),
    ]);
    let (parameter, message) = parameter_refusal(one_name.to_json())?;
    assert_eq!(parameter, "to_json");
    assert!(message.contains("\"same\""), "{message}");
    Ok(())
}

#[test]
fn a_name_is_letters_digits_and_underscore_starting_with_a_letter() -> Outcome {
    for (name, accepted) in [
        ("anybody", true),
        ("font_2", true),
        ("", false),
        ("2font", false),
        ("a-b", false),
        ("a b", false),
    ] {
        assert_eq!(fraunces()?.named(name).is_ok(), accepted, "{name:?}");
    }
    Ok(())
}

// Round trip --------------------------------------------------------------------------------------------------------

proptest! {
    /// Writing a text and loading it with its font draws the same bytes, and writes the same text again.
    #[test]
    fn a_text_survives_to_json_and_from_json(
        content in "[HiToFeAVC .\\-lfé\n]{0,10}",
        size in 1.0_f64..3000.0,
        tracking in -300.0_f64..300.0,
        x in -500.0_f64..500.0,
    ) {
        let font: Font = fraunces()?;
        let drawing = text(content, Text::new(&font).size(size).tracking(tracking).anchor((x, 40.0))).with_size(640.0, 480.0);
        let json = drawing.to_json()?;
        let again = Drawing::from_json(&json, &Inputs::new().set("font1", FRAUNCES))?;
        prop_assert_eq!(again.to_json()?, json);
        let size = Size::px(320.0, 240.0);
        prop_assert_eq!(again.to_svg(size, &Values::new())?, drawing.to_svg(size, &Values::new())?);
    }
}
