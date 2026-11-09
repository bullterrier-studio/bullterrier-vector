//! A `Drawing` of one `Path` and the `Shape`s it draws, through the public API: what `shapes()` returns for a path
//! (the path itself, its index, no attributes, the default paint), and the colours a style is written in. Expected
//! values are written by hand.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Attributes, Blend, Cap, Color, Drawing, Error, Join, Path, Shape, Values};
use proptest::prelude::*;

/// The shapes `path` draws as a drawing, with no values.
fn draw(path: &Path) -> Result<Vec<Shape>, Error> {
    Drawing::from(path.clone()).shapes(&Values::new())
}

/// The only shape of a drawing of `path`.
fn only_shape(path: &Path) -> Result<Shape, Box<dyn std::error::Error>> {
    let mut shapes = draw(path)?;
    if shapes.len() != 1 {
        return Err(format!("expected one shape, got {}", shapes.len()).into());
    }
    Ok(shapes.remove(0))
}

// Colours -----------------------------------------------------------------------------------------------------------

#[test]
fn a_colour_is_read_from_hex_digits_in_either_case() -> Outcome {
    // (text, red, green, blue, alpha): six digits are opaque, eight give the alpha
    let rows = [
        ("#ff4f9a", [0xff, 0x4f, 0x9a, 0xff]),
        ("#FF4F9AFF", [0xff, 0x4f, 0x9a, 0xff]),
        ("#Ff4f9A", [0xff, 0x4f, 0x9a, 0xff]),
        ("#ff4f9a80", [0xff, 0x4f, 0x9a, 0x80]),
        ("#00000000", [0, 0, 0, 0]),
        ("#012345", [0x01, 0x23, 0x45, 0xff]),
        ("#89abcdef", [0x89, 0xab, 0xcd, 0xef]),
    ];
    for (text, bytes) in rows {
        let color = Color::parse(text).map_err(|error| format!("{text}: {error}"))?;
        assert_eq!(color.rgba(), Some(bytes), "{text}");
    }
    assert_eq!(Color::parse("none")?, Color::NONE);
    assert_eq!(Color::NONE.rgba(), None);
    Ok(())
}

#[test]
fn a_colour_that_is_not_hex_is_refused_naming_the_colour_and_the_text() -> Outcome {
    let rows = [
        "#fff",        // the short form is not accepted
        "red",         // names are not accepted
        "#gg0000",     // not hex digits
        "",            // nothing
        "ff4f9a",      // no #
        "#ff4f9",      // five digits
        "#ff4f9a8",    // seven digits
        "#ff4f9a8000", // ten digits
        "#ffff",       // four digits
        "#ff",         // two digits
        " #ff4f9a",    // space before
        "#ff4f9a ",    // space after
        "#+f4f9a",     // a sign is not a digit
        "#ff4f9é",     // not ASCII
        "None",        // only the lower-case word
        "#",
    ];
    for text in rows {
        let (name, message) = parameter_refusal(Color::parse(text))?;
        assert!(
            name == "color" && message.contains(&format!("{text:?}")),
            "{text:?}: named {name}, said {message}"
        );
    }
    Ok(())
}

#[test]
fn a_join_a_cap_and_a_blend_are_read_from_their_words() -> Outcome {
    assert_eq!(Join::parse("round")?, Join::Round);
    assert_eq!(Join::parse("miter")?, Join::Miter);
    assert_eq!(Join::parse("bevel")?, Join::Bevel);
    assert_eq!(Cap::parse("butt")?, Cap::Butt);
    assert_eq!(Cap::parse("round")?, Cap::Round);
    assert_eq!(Cap::parse("square")?, Cap::Square);
    assert_eq!(Blend::parse("normal")?, Blend::Normal);
    assert_eq!(Blend::parse("multiply")?, Blend::Multiply);
    // and written back
    assert_eq!(
        (Join::Miter.name(), Cap::Butt.name(), Blend::Multiply.name()),
        ("miter", "butt", "multiply")
    );
    Ok(())
}

#[test]
fn another_word_is_refused_naming_the_argument_the_words_and_the_text() -> Outcome {
    // (the argument, the refusal, a word it lists)
    let rows = [
        ("join", parameter_refusal(Join::parse("Round")), "bevel"),
        ("cap", parameter_refusal(Cap::parse("")), "square"),
        ("mode", parameter_refusal(Blend::parse("screen")), "multiply"),
    ];
    for (argument, refusal, listed) in rows {
        let (name, message) = refusal?;
        assert!(
            name == argument && message.contains(listed),
            "{argument}: named {name}, said {message}"
        );
    }
    Ok(())
}

#[test]
fn a_colour_from_a_hex_number_is_opaque() -> Outcome {
    assert_eq!(Color::hex(0xff_4f_9a).rgba(), Some([0xff, 0x4f, 0x9a, 0xff]));
    assert_eq!(Color::hex(0x00_00_00).rgba(), Some([0, 0, 0, 0xff]));
    assert_eq!(Color::hex(0xff_ff_ff).rgba(), Some([0xff, 0xff, 0xff, 0xff]));
    assert_eq!(Color::hex(0x00_01_80), Color::parse("#000180")?);
    // the bits above the lowest 24 are not a colour: they are not read
    assert_eq!(Color::hex(0xab_ff_4f_9a), Color::hex(0xff_4f_9a));
    Ok(())
}

#[test]
fn a_colour_prints_as_eight_lower_case_digits_or_none() -> Outcome {
    assert_eq!(Color::hex(0xff_4f_9a).to_string(), "#ff4f9aff");
    assert_eq!(Color::parse("#FF4F9A80")?.to_string(), "#ff4f9a80");
    assert_eq!(Color::parse("#00000001")?.to_string(), "#00000001");
    assert_eq!(Color::NONE.to_string(), "none");
    Ok(())
}

proptest! {
    // Hex text written from four bytes reads back as those bytes in either case, and prints as the lower-case text.
    #[test]
    fn a_colour_written_from_its_bytes_reads_back_as_them(bytes in any::<[u8; 4]>()) {
        let [r, g, b, a] = bytes;
        let upper = format!("#{r:02X}{g:02X}{b:02X}{a:02X}");
        let lower = upper.to_lowercase();
        for text in [&upper, &lower] {
            let color = Color::parse(text).map_err(|error| TestCaseError::fail(error.to_string()))?;
            prop_assert_eq!(color.rgba(), Some(bytes));
            prop_assert_eq!(color.to_string(), lower.clone());
        }
        let opaque = format!("#{r:02x}{g:02x}{b:02x}");
        let color = Color::parse(&opaque).map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(color.rgba(), Some([r, g, b, 0xff]));
    }
}

// The shapes of a path ----------------------------------------------------------------------------------------------

#[test]
fn a_drawing_of_a_path_draws_one_shape_with_that_path_and_no_attributes() -> Outcome {
    let path = Path::from_svg_path("M1 2 L10 4.25 Q11 5 12 6.5 Z")?;
    let shape = only_shape(&path)?;
    assert_eq!(shape.path(), &path);
    assert_eq!(shape.index(), 0);
    assert_eq!(shape.attributes(), &Attributes::default());
    let attributes = shape.attributes();
    assert_eq!(
        (
            attributes.glyph(),
            attributes.char(),
            attributes.word(),
            attributes.line()
        ),
        (None, None, None, None)
    );
    assert_eq!(
        (
            attributes.part(),
            attributes.copy(),
            attributes.level(),
            attributes.order()
        ),
        (None, None, None, None)
    );
    assert_eq!(attributes.origin(), None);
    Ok(())
}

#[test]
fn a_closed_outline_is_filled_black_by_default() -> Outcome {
    let square = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?;
    let style = only_shape(&square)?.style().clone();
    assert_eq!(*style.fill(), Color::hex(0x00_00_00));
    assert_eq!(style.stroke(), None);
    assert_eq!(style.opacity(), 1.0);
    assert_eq!(style.blend(), Blend::Normal);
    Ok(())
}

#[test]
fn an_open_line_is_stroked_black_two_pixels_wide_by_default() -> Outcome {
    let line = Path::line((0.0, 0.0), (10.0, 5.0))?;
    let style = only_shape(&line)?.style().clone();
    assert_eq!(*style.fill(), Color::NONE);
    let stroke = style.stroke().ok_or("an open line has no stroke")?;
    assert_eq!(*stroke.paint(), Color::hex(0x00_00_00));
    assert_eq!(stroke.width(), 2.0);
    assert_eq!((stroke.join(), stroke.cap()), (Join::Round, Cap::Round));
    assert_eq!(stroke.miter_limit(), 4.0);
    assert_eq!((style.opacity(), style.blend()), (1.0, Blend::Normal));
    Ok(())
}

#[test]
fn a_path_with_any_open_sub_path_is_stroked_not_filled() -> Outcome {
    let rect = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?;
    let closed_then_open = rect.move_to((20.0, 20.0))?.line_to((30.0, 30.0))?;
    let open_then_closed = Path::line((0.0, 0.0), (5.0, 5.0))?
        .move_to((20.0, 20.0))?
        .line_to((30.0, 20.0))?;
    let open_then_closed = open_then_closed.line_to((30.0, 30.0))?.close()?;
    // a segment after a close starts a new sub-path, which is open
    let reopened = Path::start((0.0, 0.0))?
        .line_to((5.0, 0.0))?
        .line_to((5.0, 5.0))?
        .close()?
        .line_to((9.0, 9.0))?;
    // a lone point is an open sub-path
    let point = Path::start((1.0, 2.0))?;
    for (case, path) in [
        ("closed then open", closed_then_open),
        ("open then closed", open_then_closed),
        ("reopened after close", reopened),
        ("a lone point", point),
    ] {
        let style = only_shape(&path)?.style().clone();
        assert_eq!(*style.fill(), Color::NONE, "{case}");
        assert!(style.stroke().is_some(), "{case}");
    }
    Ok(())
}

#[test]
fn a_path_whose_sub_paths_are_all_closed_is_filled() -> Outcome {
    let two_rings = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?
        .move_to((20.0, 20.0))?
        .line_to((25.0, 20.0))?;
    let two_rings = two_rings.line_to((25.0, 25.0))?.close()?;
    let circle = Path::circle((5.0, 5.0), 3.0)?;
    for (case, path) in [("two closed sub-paths", two_rings), ("a circle", circle)] {
        let style = only_shape(&path)?.style().clone();
        assert_eq!(*style.fill(), Color::hex(0x00_00_00), "{case}");
        assert_eq!(style.stroke(), None, "{case}");
    }
    Ok(())
}

#[test]
fn an_empty_path_is_a_filled_shape_that_draws_nothing() -> Outcome {
    let shape = only_shape(&Path::new())?;
    assert_eq!(shape.path(), &Path::new());
    assert_eq!(*shape.style().fill(), Color::hex(0x00_00_00));
    assert_eq!(shape.style().stroke(), None);
    assert_eq!(shape.index(), 0);
    Ok(())
}

#[test]
fn drawing_twice_or_from_a_clone_gives_equal_shapes() -> Outcome {
    let drawing = Drawing::from(Path::star((50.0, 50.0), 40.0, 16.0, 5)?);
    let values = Values::new();
    let first = drawing.shapes(&values)?;
    assert_eq!(first, drawing.shapes(&values)?);
    assert_eq!(first, drawing.clone().shapes(&values)?);
    assert_eq!(
        first,
        Drawing::from(Path::star((50.0, 50.0), 40.0, 16.0, 5)?).shapes(&values)?
    );
    // the shape is a value: a clone is equal, a different path is not
    assert_eq!(first[0].clone(), first[0]);
    assert_ne!(first, draw(&Path::star((50.0, 50.0), 41.0, 16.0, 5)?)?);
    Ok(())
}

#[test]
fn a_value_for_a_param_the_drawing_does_not_read_is_refused_naming_it() -> Outcome {
    let drawing = Drawing::from(Path::circle((0.0, 0.0), 1.0)?);
    let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("weight", 820.0)))?;
    assert!(name == "weight" && message.contains("no Param"), "{name}: {message}");
    Ok(())
}
