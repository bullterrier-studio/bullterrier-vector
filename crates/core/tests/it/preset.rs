//! The drawings that come with the library (`Drawing::preset`): the names, that each loads with no inputs and draws, that
//! it is the canonical text shipped (`to_json` of the loaded drawing equals the file), that each Param says what it
//! means, draws its subject at its default and maximum and changes the picture, and that an unknown name is refused
//! naming the choices. The oracle for the picture is its own pixels against the frame's corner, and the SVG compared
//! as text; the oracle for the text is the file itself.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Drawing, Size, Unit, Values};

/// A drawing has a subject if at least this share of its pixels differs from the frame's corner.
const MIN_INK: f64 = 0.01;

/// (name, the shipped text): written here, apart from the library's own table, so a table that loses a file fails.
const SHIPPED: [(&str, &str); 13] = [
    ("bent_type", include_str!("../../presets/bent_type.json")),
    ("brush_taper", include_str!("../../presets/brush_taper.json")),
    ("draw_on", include_str!("../../presets/draw_on.json")),
    ("flow_hatch", include_str!("../../presets/flow_hatch.json")),
    ("glyph_scatter", include_str!("../../presets/glyph_scatter.json")),
    ("gradient_type", include_str!("../../presets/gradient_type.json")),
    ("halftone_screen", include_str!("../../presets/halftone_screen.json")),
    ("marching_dashes", include_str!("../../presets/marching_dashes.json")),
    ("ring", include_str!("../../presets/ring.json")),
    ("stipple", include_str!("../../presets/stipple.json")),
    ("text_on_path", include_str!("../../presets/text_on_path.json")),
    ("two_fonts", include_str!("../../presets/two_fonts.json")),
    ("waves", include_str!("../../presets/waves.json")),
];

/// The share of pixels, at a quarter of the design frame, that differ from the pixel at its top left corner.
fn ink(drawing: &Drawing, values: &Values) -> Result<f64, Box<dyn std::error::Error>> {
    let (width, height) = drawing.size().ok_or("a preset has a frame")?;
    let (width, height) = ((width / 4.0).round(), (height / 4.0).round());
    let pixels = drawing.to_rgba(Size::px(width, height), values)?;
    let differing = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| **pixel != pixels[..4])
        .count();
    Ok(f64::from(u32::try_from(differing)?) / (width * height))
}

/// The SVG of `drawing` with these Param values: the picture, as text.
fn picture(drawing: &Drawing, values: &Values) -> Result<String, Box<dyn std::error::Error>> {
    Ok(drawing.to_svg(Size::px(200.0, 200.0), values)?)
}

/// One sentence: ends with a full stop, on one line, and no other sentence ending (`!`, `?`, or a full stop and a
/// space before a capital).
fn is_one_sentence(text: &str) -> bool {
    let Some(body) = text.strip_suffix('.') else {
        return false;
    };
    !text.contains('\n')
        && !body.trim().is_empty()
        && !body.contains(['!', '?'])
        && !body
            .split(". ")
            .skip(1)
            .any(|rest| rest.starts_with(char::is_uppercase))
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
fn every_preset_loads_without_inputs_draws_its_subject_and_is_the_shipped_text() -> Outcome {
    for (name, text) in SHIPPED {
        let drawing = Drawing::preset(name)?;
        assert!(ink(&drawing, &Values::new())? >= MIN_INK, "{name} draws no subject");
        assert!(
            drawing.to_json()? == text,
            "{name}: to_json differs from the shipped text"
        );
    }
    Ok(())
}

#[test]
fn every_preset_has_params_that_mean_something_draw_the_subject_and_move_the_picture() -> Outcome {
    for name in Drawing::preset_names() {
        let drawing = Drawing::preset(name)?;
        let params = drawing.parameters()?;
        assert!(!params.is_empty(), "{name} has no Param to play with");
        for param in &params {
            assert!(
                !param.meaning_of().is_empty(),
                "{name}: {} has no meaning",
                param.name()
            );
            // Not at the minimum: a reveal (`draw_on`'s progress) is bare there, by design.
            for value in [param.default(), param.max()] {
                let values = Values::new().set(param.name(), value);
                assert!(ink(&drawing, &values)? >= MIN_INK, "{name} at {}={value}", param.name());
            }
            // A cyclic Param ends where it starts, so it is compared with a quarter turn; any other, with its far end.
            let far = match param.unit_of() {
                Unit::Deg | Unit::Cycle => param.min() + (param.max() - param.min()) / 4.0,
                Unit::None | Unit::Px | Unit::Fraction => param.max(),
            };
            let at = |value| picture(&drawing, &Values::new().set(param.name(), value));
            assert!(
                at(param.min())? != at(far)?,
                "{name}: {} does not change the picture",
                param.name()
            );
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
fn every_preset_has_a_one_sentence_description() -> Outcome {
    for name in Drawing::preset_names() {
        let text = Drawing::preset_description(name)?;
        assert!(
            is_one_sentence(text),
            "{name}: {text:?} is not one sentence ending in a full stop"
        );
    }
    Ok(())
}

#[test]
fn the_one_sentence_rule_tells_known_good_from_known_bad() {
    let rows = [
        ("A ring.", true),
        ("Type set at approx. 200 px.", true),
        ("A ring, with a slider; a wide one.", true),
        ("A ring! Its width is a slider.", false),
        ("A ring? Yes, a wide one.", false),
        ("A ring. Its width is a slider.", false),
        ("A ring!", false),
        ("A ring", false),
        ("A ring.\n", false),
        ("A ring\nand a disc.", false),
        (" .", false),
    ];
    for (text, one_sentence) in rows {
        assert_eq!(is_one_sentence(text), one_sentence, "{text:?}");
    }
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
    assert!(
        Drawing::preset_names().len() >= 12,
        "only {} presets",
        Drawing::preset_names().len()
    );
}
