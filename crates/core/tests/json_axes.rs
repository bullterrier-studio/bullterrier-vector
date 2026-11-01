//! `axes` and `features` of `text` in a document: the round trip of a drawing with Param axes, what the reader refuses
//! by itself (a tag, a value that is not a whole number), and the loader's `font-checks` (an axis or feature the
//! supplied font lacks, an axis value outside its range), each with a document that passes and one that fails.

mod common;

use bullterrier_vector::{Drawing, Error, Inputs, Text, Values, text};
use common::fonts::{FRAUNCES, fraunces};
use common::{Outcome, parameter_refusal};

fn one_font() -> Inputs {
    Inputs::new().set("anybody", FRAUNCES)
}

const WEIGHT: &str = r#"{"name":"wght","default":400,"min":300,"max":700,"unit":"","meaning":"Weight"}"#;

/// A document with the font `anybody` and one text whose extra arguments are `arguments`; `params` are declared if given.
fn document(params: &str, arguments: &str) -> String {
    let params = if params.is_empty() {
        String::new()
    } else {
        format!(r#""params":[{params}],"#)
    };
    format!(
        r#"{{"version":1,{params}"inputs":{{"anybody":{{"kind":"font"}}}},"root":[{{"text":{{"content":"fi","font":"anybody",{arguments}}}}}]}}"#
    )
}

fn format_refusal(text: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    match Drawing::from_json(text, &one_font()) {
        Err(Error::Format { pointer, message }) => Ok((pointer, message)),
        Err(other) => Err(format!("expected a format error, got {other}").into()),
        Ok(_) => Err("expected a format error, but it loaded".into()),
    }
}

// The round trip ----------------------------------------------------------------------------------------------------

#[test]
fn a_drawing_with_a_param_axis_and_features_writes_a_document_that_reads_back() -> Outcome {
    let font = fraunces()?.named("anybody")?;
    let weight = font.axes().remove(0);
    let drawing = text(
        "fi",
        Text::new(&font)
            .size(100.0)
            .axes([("wght", &weight + 100.0)])
            .features([("liga", 0), ("kern", 1)]),
    );
    let written = drawing.to_json()?;
    assert!(
        written.contains(r#""axes":{"wght":{"add":[100,{"param":"wght"}]}}"#)
            && written.contains(r#""features":{"kern":1,"liga":0}"#),
        "{written}"
    );
    // the Param is the font's own, in the document with its fields
    assert!(
        written.contains(r#""meaning":"Weight""#) && written.contains(r#""min":300"#),
        "{written}"
    );
    let loaded = Drawing::from_json(&written, &one_font())?;
    assert_eq!(loaded.to_json()?, written);
    assert_eq!(loaded.parameters()?, vec![weight]);
    for values in [Values::new(), Values::new().set("wght", 300.0)] {
        assert_eq!(loaded.shapes(&values)?, drawing.shapes(&values)?);
    }
    // 700 = 600 + 100: two shapes (liga off), the second as heavy as the font goes
    assert_eq!(loaded.shapes(&Values::new().set("wght", 600.0))?.len(), 2);
    Ok(())
}

#[test]
fn an_empty_axes_or_features_object_is_the_default_and_is_not_written() -> Outcome {
    let plain = Drawing::from_json(&document("", r#""size":100"#), &one_font())?;
    let empty = Drawing::from_json(&document("", r#""size":100,"axes":{},"features":{}"#), &one_font())?;
    assert_eq!(empty.to_json()?, plain.to_json()?);
    assert_eq!(empty.shapes(&Values::new())?, plain.shapes(&Values::new())?);
    Ok(())
}

#[test]
fn a_document_draws_what_the_code_draws() -> Outcome {
    let font = fraunces()?.named("anybody")?;
    let loaded = Drawing::from_json(
        &document("", r#""axes":{"wght":700},"features":{"liga":0}"#),
        &one_font(),
    )?;
    let built = text("fi", Text::new(&font).axes([("wght", 700.0)]).features([("liga", 0)]));
    assert_eq!(loaded.shapes(&Values::new())?, built.shapes(&Values::new())?);
    Ok(())
}

#[test]
fn a_param_read_only_by_an_axis_is_declared_and_used() -> Outcome {
    // param-declared, param-used and param-order see the axes: declared and read, it loads
    let drawing = Drawing::from_json(&document(WEIGHT, r#""axes":{"wght":{"param":"wght"}}"#), &one_font())?;
    assert_eq!(drawing.parameters()?.len(), 1);
    // read by an axis but not declared
    assert_eq!(
        format_refusal(&document("", r#""axes":{"wght":{"param":"wght"}}"#))?,
        (
            "/root/0/text/axes/wght/param".to_owned(),
            "\"wght\" is not an entry of `params`".to_owned()
        )
    );
    // declared, read by nothing
    assert_eq!(
        format_refusal(&document(WEIGHT, r#""axes":{"wght":400}"#))?,
        ("/params/0".to_owned(), "is read by no call".to_owned())
    );
    Ok(())
}

// What the reader refuses -------------------------------------------------------------------------------------------

#[test]
fn the_reader_refuses_a_tag_and_a_feature_value_that_cannot_be_right() -> Outcome {
    // (the arguments, the pointer of the refusal, a fragment of the message)
    let rows = [
        (r#""axes":{"wgt":400}"#, "/root/0/text/axes/wgt", "four printable ASCII"),
        (r#""axes":{"":400}"#, "/root/0/text/axes/", "four printable ASCII"),
        (r#""axes":[400]"#, "/root/0/text/axes", "an object of tags"),
        (r#""axes":{"wght":"bold"}"#, "/root/0/text/axes/wght", "a number"),
        (r#""axes":{"wght":null}"#, "/root/0/text/axes/wght", "a number"),
        (
            r#""features":{"liga":1.5}"#,
            "/root/0/text/features/liga",
            "whole number",
        ),
        (
            r#""features":{"liga":-1}"#,
            "/root/0/text/features/liga",
            "whole number",
        ),
        (
            r#""features":{"liga":4294967296}"#,
            "/root/0/text/features/liga",
            "whole number",
        ),
        (
            r#""features":{"liga":{"param":"x"}}"#,
            "/root/0/text/features/liga",
            "a number",
        ),
        (r#""features":{"liga":true}"#, "/root/0/text/features/liga", "a number"),
        (
            r#""features":{"lig":1}"#,
            "/root/0/text/features/lig",
            "four printable ASCII",
        ),
        (r#""features":1"#, "/root/0/text/features", "an object of tags"),
    ];
    for (arguments, pointer, fragment) in rows {
        let (found, message) = format_refusal(&document("", arguments))?;
        assert_eq!(found, pointer, "{arguments}: {message}");
        assert!(message.contains(fragment), "{arguments}: {message}");
    }
    // the largest whole number is accepted
    Drawing::from_json(&document("", r#""features":{"liga":4294967295}"#), &one_font())?;
    Ok(())
}

// font-checks -------------------------------------------------------------------------------------------------------

#[test]
fn font_checks_refuse_a_font_that_lacks_what_the_document_names_and_accept_one_that_has_it() -> Outcome {
    // (the arguments, the message must name it: none if the document loads)
    let rows = [
        (r#""axes":{"wght":300}"#, None),
        (r#""axes":{"wght":700}"#, None),
        (r#""axes":{"wght":400.5}"#, None),
        (r#""features":{"liga":0,"kern":1}"#, None),
        (
            r#""axes":{"wdth":100}"#,
            Some(vec![
                "\"wdth\"",
                "wght (300 to 700, default 400)",
                "/root/0/text/axes/wdth",
            ]),
        ),
        (
            r#""axes":{"wght":701}"#,
            Some(vec!["\"wght\"", "300 to 700", "701", "/root/0/text/axes/wght"]),
        ),
        (r#""axes":{"wght":299}"#, Some(vec!["299", "300 to 700"])),
        (r#""axes":{"wght":1e300}"#, Some(vec!["1e300"])),
        (
            r#""features":{"ss01":1}"#,
            Some(vec!["\"ss01\"", "kern, liga", "/root/0/text/features/ss01"]),
        ),
    ];
    for (arguments, expected) in rows {
        let loaded = Drawing::from_json(&document("", arguments), &one_font());
        match (expected, loaded) {
            (None, Ok(drawing)) => {
                drawing.shapes(&Values::new())?;
            }
            (Some(named), loaded) => {
                let (parameter, message) = parameter_refusal(loaded)?;
                assert_eq!(parameter, "inputs.anybody", "{arguments}");
                for part in named {
                    assert!(message.contains(part), "{arguments}: {part} not in {message}");
                }
            }
            (None, Err(error)) => return Err(format!("{arguments}: {error}").into()),
        }
    }
    Ok(())
}

#[test]
fn a_font_with_no_axes_is_refused_an_axis_and_the_message_says_so() -> Outcome {
    let inputs = Inputs::new().set("anybody", common::fonts::SYNTHETIC);
    let (parameter, message) = parameter_refusal(Drawing::from_json(&document("", r#""axes":{"wght":400}"#), &inputs))?;
    assert_eq!(parameter, "inputs.anybody");
    assert!(message.contains("the font has no axes"), "{message}");
    // and a static font's own feature loads
    let inputs = Inputs::new().set("anybody", common::fonts::MARKS);
    Drawing::from_json(&document("", r#""features":{"mark":0}"#), &inputs)?;
    Ok(())
}

#[test]
fn a_value_that_reads_a_param_is_checked_when_drawn_not_when_loaded() -> Outcome {
    // the Param's own range is wider than the font's: loads, then the draw is refused
    let wide = r#"{"name":"w","default":400,"min":0,"max":1000,"unit":"","meaning":""}"#;
    let drawing = Drawing::from_json(&document(wide, r#""axes":{"wght":{"param":"w"}}"#), &one_font())?;
    drawing.shapes(&Values::new().set("w", 700.0))?;
    let (parameter, message) = parameter_refusal(drawing.shapes(&Values::new().set("w", 900.0)))?;
    assert_eq!(parameter, "source (text).axes");
    assert!(message.contains("900") && message.contains("300 to 700"), "{message}");
    Ok(())
}
