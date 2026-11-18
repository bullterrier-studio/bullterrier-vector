//! The drawings that come with the library (`Drawing::preset`): the names, that each loads with no inputs and draws, that
//! it is the canonical text shipped (`to_json` of the loaded drawing equals the file), that each Param says what it
//! means and draws at its three poses, and that an unknown name is refused naming the choices. The oracle for the SVG
//! is a parse with roxmltree; the oracle for the text is the file itself.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Drawing, Size, Values};

/// (name, the shipped text): written here, apart from the library's own table, so a table that loses a file fails.
const SHIPPED: [(&str, &str); 8] = [
    ("bent_type", include_str!("../../presets/bent_type.json")),
    ("draw_on", include_str!("../../presets/draw_on.json")),
    ("flow_hatch", include_str!("../../presets/flow_hatch.json")),
    ("glyph_morph", include_str!("../../presets/glyph_morph.json")),
    ("glyph_scatter", include_str!("../../presets/glyph_scatter.json")),
    ("gradient_type", include_str!("../../presets/gradient_type.json")),
    ("ring", include_str!("../../presets/ring.json")),
    ("two_fonts", include_str!("../../presets/two_fonts.json")),
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
        assert!(
            drawing.to_json()? == text,
            "{name}: to_json differs from the shipped text"
        );
    }
    Ok(())
}

#[test]
fn every_preset_has_params_that_mean_something_and_move_the_picture_between_their_extremes() -> Outcome {
    for name in Drawing::preset_names() {
        let drawing = Drawing::preset(name)?;
        let params = drawing.parameters()?;
        assert!(!params.is_empty(), "{name} has no Param to play with");
        let mut moves_the_picture = false;
        for param in &params {
            assert!(
                !param.meaning_of().is_empty(),
                "{name}: {} has no meaning",
                param.name()
            );
            for value in [param.min(), param.default(), param.max()] {
                let values = Values::new().set(param.name(), value);
                assert!(paths(&drawing, &values)? > 0, "{name} at {}={value}", param.name());
            }
            let at = |value| drawing.to_svg(Size::px(200.0, 200.0), &Values::new().set(param.name(), value));
            moves_the_picture |= at(param.min())? != at(param.max())?;
        }
        // A phase loops, so it can look alike at both ends; some Param of the preset must change the picture.
        assert!(
            moves_the_picture,
            "{name}: no Param changes the picture between its minimum and maximum"
        );
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
fn every_preset_has_a_one_sentence_description() -> Outcome {
    for name in Drawing::preset_names() {
        let text = Drawing::preset_description(name)?;
        assert!(text.ends_with('.'), "{name}: {text:?} does not end with a full stop");
        assert!(!text.contains('\n'), "{name}: the description has a line break");
        assert!(text.trim().len() > 1, "{name}: the description is blank");
        assert!(
            !text[..text.len() - 1].contains(". "),
            "{name}: {text:?} is more than one sentence"
        );
    }
    Ok(())
}

#[test]
fn an_unknown_name_has_no_description_and_the_same_refusal_as_preset() -> Outcome {
    let refusal = |result: Result<&str, _>| parameter_refusal(result.map(str::to_owned));
    let (parameter, message) = refusal(Drawing::preset_description("nope"))?;
    assert_eq!(parameter, "name");
    assert_eq!(message, parameter_refusal(Drawing::preset("nope"))?.1);
    Ok(())
}

#[test]
fn at_least_twelve_presets_ship() {
    // The promise of the gallery batch: a starting point for each look.
    assert!(Drawing::preset_names().len() >= 12, "only {} presets", Drawing::preset_names().len());
}
