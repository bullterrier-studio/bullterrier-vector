//! The fonts that come with the library (`Font::bundled`): the names, that each one loads with the axes and the
//! advances its font file has, that an unknown name is refused naming the choices, and that a saved drawing in a
//! bundled font loads again with no bytes supplied. The expected numbers are literals read with fontTools from the
//! upstream files (`Fraunces[SOFT,WONK,opsz,wght].ttf`, `BricolageGrotesque[opsz,wdth,wght].ttf` of the Google Fonts
//! repository): the `fvar` axes (tag, minimum, default, maximum, name) in file order, and the `hmtx` advance of `H`
//! (the advance at the file's default location).

use crate::common::fonts::FRAUNCES;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Drawing, Font, Inputs, Text, Values, text};

/// (name, units per em, `fvar` axes as (tag, min, default, max, name), advance of `H`).
type Row = (
    &'static str,
    f64,
    &'static [(&'static str, f64, f64, f64, &'static str)],
    f64,
);

const FONTS: [Row; 2] = [
    (
        "bricolage_grotesque",
        1000.0,
        &[
            ("opsz", 12.0, 96.0, 96.0, "Optical size"),
            ("wght", 200.0, 800.0, 800.0, "Weight"),
            ("wdth", 75.0, 100.0, 100.0, "Width"),
        ],
        637.0,
    ),
    (
        "fraunces",
        2000.0,
        &[
            ("opsz", 9.0, 9.0, 144.0, "Optical Size"),
            ("wght", 100.0, 900.0, 900.0, "Weight"),
            ("SOFT", 0.0, 0.0, 100.0, "Softness"),
            ("WONK", 0.0, 1.0, 1.0, "Wonky"),
        ],
        1805.0,
    ),
];

#[test]
fn the_bundled_fonts_are_listed_by_name() {
    assert_eq!(Font::bundled_names(), ["bricolage_grotesque", "fraunces"]);
}

#[test]
fn each_bundled_font_has_the_name_and_the_axes_of_its_font_file() -> Outcome {
    for (name, _, axes, _) in FONTS {
        let font = Font::bundled(name)?;
        assert_eq!(font.name(), Some(name));
        let found: Vec<_> = font
            .axes()
            .iter()
            .map(|axis| {
                (
                    axis.name().to_owned(),
                    axis.min(),
                    axis.default(),
                    axis.max(),
                    axis.meaning_of().to_owned(),
                )
            })
            .collect();
        let wanted: Vec<_> = axes
            .iter()
            .map(|&(tag, min, default, max, meaning)| (tag.to_owned(), min, default, max, meaning.to_owned()))
            .collect();
        assert_eq!(found, wanted, "{name}");
    }
    Ok(())
}

#[test]
fn each_bundled_font_shapes_a_string_with_the_advances_of_its_font_file() -> Outcome {
    for (name, em, _, advance_of_h) in FONTS {
        let font = Font::bundled(name)?;
        let shapes = text("HH Hi", Text::new(&font).size(em)).shapes(&Values::new())?;
        assert_eq!(shapes.len(), 4, "{name}");
        let x = |at: usize| shapes[at].attributes().origin().map(|point| point.x);
        assert!(
            (x(1).ok_or("origin")? - x(0).ok_or("origin")? - advance_of_h).abs() < 1e-9,
            "{name}"
        );
    }
    Ok(())
}

#[test]
fn a_name_that_is_not_bundled_is_refused_naming_it_and_the_choices() -> Outcome {
    for wrong in ["Fraunces", "roboto_flex", ""] {
        let (parameter, message) = parameter_refusal(Font::bundled(wrong))?;
        assert_eq!(parameter, "name", "{wrong:?}");
        assert!(message.contains(&format!("{wrong:?}")), "{message}");
        assert!(message.contains("bricolage_grotesque, fraunces"), "{message}");
    }
    Ok(())
}

#[test]
fn a_saved_drawing_in_a_bundled_font_loads_again_with_no_bytes_supplied() -> Outcome {
    for (name, ..) in FONTS {
        let drawing = text("Hi", Text::new(&Font::bundled(name)?));
        let json = drawing.to_json()?;
        let loaded = Drawing::from_json(&json, &Inputs::new())?;
        assert_eq!(loaded.to_json()?, json, "{name}");
        assert_eq!(
            loaded.shapes(&Values::new())?,
            drawing.shapes(&Values::new())?,
            "{name}"
        );
    }
    Ok(())
}

#[test]
fn a_font_that_is_not_bundled_is_still_to_be_supplied() -> Outcome {
    // the same document with the digest of a font that is not bundled, and with no digest at all
    let json = text("Hi", Text::new(&Font::bundled("fraunces")?)).to_json()?;
    let bundled = Drawing::from_json(&json, &Inputs::new())?.to_json()?;
    assert_eq!(bundled, json);
    let digest = json
        .split("\"sha256\":\"")
        .nth(1)
        .and_then(|rest| rest.get(..64))
        .ok_or("sha256")?;
    for changed in [
        json.replace(digest, &"0".repeat(64)),
        json.replace(&format!(",\"sha256\":\"{digest}\""), ""),
    ] {
        let (parameter, message) = parameter_refusal(Drawing::from_json(&changed, &Inputs::new()))?;
        assert_eq!(parameter, "inputs.fraunces");
        assert!(message.contains("does not give it"), "{message}");
    }
    // bytes given for it are read and must match the digest, as for any input
    let wrong = Inputs::new().set("fraunces", FRAUNCES);
    let (_, message) = parameter_refusal(Drawing::from_json(&json, &wrong))?;
    assert!(message.contains("but the document says"), "{message}");
    Ok(())
}
