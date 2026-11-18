//! The drawings that come with the library (`Drawing::preset`): the names, that each loads with no inputs and draws, that
//! it is the canonical text shipped (`to_json` of the loaded drawing equals the file), that each Param says what it
//! means and draws at its three poses, and that an unknown name is refused naming the choices. The oracle for the SVG
//! is a parse with roxmltree; the oracle for the text is the file itself.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Drawing, Inputs, Size, Values};

/// (name, the shipped text): written here, apart from the library's own table, so a table that loses a file fails.
const SHIPPED: [(&str, &str); 3] = [
    ("bent_type", include_str!("../../presets/bent_type.json")),
    ("gradient_type", include_str!("../../presets/gradient_type.json")),
    ("ring", include_str!("../../presets/ring.json")),
];

/// The number of `path` elements in the SVG of `drawing` with these Param values.
fn paths(drawing: &Drawing, values: &Values) -> Result<usize, Box<dyn std::error::Error>> {
    let svg = drawing.to_svg(Size::px(200.0, 200.0), values)?;
    let document = roxmltree::Document::parse(&svg)?;
    Ok(document.descendants().filter(|node| node.has_tag_name("path")).count())
}

#[test]
fn the_names_are_sorted_unique_and_the_shipped_ones() {
    let names = Drawing::preset_names();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(names, sorted);
    assert_eq!(names, SHIPPED.map(|(name, _)| name));
}

#[test]
fn every_preset_loads_without_inputs_draws_shapes_and_is_the_shipped_text() -> Outcome {
    for (name, text) in SHIPPED {
        let drawing = Drawing::preset(name)?;
        assert!(paths(&drawing, &Values::new())? > 0, "{name} draws no shape");
        assert!(drawing.to_json()? == text, "{name}: to_json differs from the shipped text");
    }
    Ok(())
}

#[test]
fn every_param_has_a_meaning_and_draws_at_its_minimum_default_and_maximum() -> Outcome {
    for name in Drawing::preset_names() {
        let drawing = Drawing::preset(name)?;
        for param in drawing.parameters()? {
            assert!(!param.meaning_of().is_empty(), "{name}: {} has no meaning", param.name());
            for value in [param.min(), param.default(), param.max()] {
                let values = Values::new().set(param.name(), value);
                assert!(paths(&drawing, &values)? > 0, "{name} at {}={value}", param.name());
            }
        }
    }
    Ok(())
}

#[test]
fn an_unknown_name_is_refused_naming_every_preset() -> Outcome {
    let (parameter, message) = parameter_refusal(Drawing::preset("nope"))?;
    assert_eq!(parameter, "name");
    assert!(message.contains("\"nope\""), "{message}");
    assert!(message.contains(&Drawing::preset_names().join(", ")), "{message}");
    Ok(())
}

#[test]
fn a_text_that_needs_bytes_the_caller_has_not_supplied_does_not_load() -> Outcome {
    let undeclared = r#"{"version":1,"root":[{"text":{"content":"Hi","font":"anybody"}}]}"#;
    assert!(Drawing::from_json(undeclared, &Inputs::new()).is_err());
    Ok(())
}
