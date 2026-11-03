//! `text` layout through the public API: `width` wrapping the lines, `fit` making them exactly that wide. The expected
//! numbers are literals from `HarfBuzz` (uharfbuzz, an independent shaper) over `fraunces-subset.ttf` (`common/fonts.rs`
//! says how it was made), not computed by the library:
//!
//! ```text
//! uvx --from fonttools --with uharfbuzz python literals.py fraunces-subset.ttf "HiHiiHH ii" "Hi To" "iHi" ...
//! 'Hi To' [('H', 0, 1656), ('i', 1, 601), ('space', 2, 449), ('T', 3, 1249), ('o', 4, 1166)]
//! ```
//!
//! `H`, `i` and the space have advances of 1656, 601 and 449, and no kerning among them. Drawings are made at 2000 pixels to
//! the em, the font's own, so a pen position is a coordinate of the font file; the default leading is 2400, y pointing
//! down.

use crate::common::fonts::fraunces;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Align, Error, Font, Param, Path, Shape, Text, Values, text};
use proptest::prelude::*;

/// 2000 pixels to the em: the font's own.
const EM: f64 = 2000.0;
/// 1.2 em.
const LEADING: f64 = 2400.0;

fn at_em(font: &Font) -> Text {
    Text::new(font).size(EM)
}

fn shapes_of(content: &str, options: Text) -> Result<Vec<Shape>, Error> {
    text(content, options).shapes(&Values::new())
}

/// The outline of the one glyph `content` is, as the library draws it at the origin.
fn alone(content: &str) -> Result<Path, Error> {
    let shapes = shapes_of(content, at_em(&fraunces()?))?;
    assert_eq!(shapes.len(), 1, "{content:?}");
    Ok(shapes[0].path().clone())
}

/// Glyphs as their character, line and the x of the pen (or of the middle of the glyph's advance).
type Glyphs = &'static [(char, u32, f64)];

/// The line of each shape.
fn lines_of(shapes: &[Shape]) -> Vec<Option<u32>> {
    shapes.iter().map(|shape| shape.attributes().line()).collect()
}

// width: wrapping -----------------------------------------------------------------------------------------------------

#[test]
fn lines_break_at_whitespace_as_late_as_they_fit() -> Outcome {
    // (rule, text, width, every glyph as its character, line and the pen's x). H 1656, i 601, space 449; T 1249 and o 1166
    // in "To" (kerned). A line is as wide as its glyphs and the spaces between them.
    let rows: [(&str, &str, f64, Glyphs); 14] = [
        (
            "exactly the width fits",
            "H H",
            3761.0,
            &[('H', 0, 0.0), ('H', 0, 2105.0)],
        ),
        ("one unit less wraps", "H H", 3760.0, &[('H', 0, 0.0), ('H', 1, 0.0)]),
        (
            "greedy: the third word starts the second line",
            "H H H",
            3761.0,
            &[('H', 0, 0.0), ('H', 0, 2105.0), ('H', 1, 0.0)],
        ),
        (
            "the spaces at a break are all dropped",
            "H   H",
            1656.0,
            &[('H', 0, 0.0), ('H', 1, 0.0)],
        ),
        (
            "the spaces inside a line stay",
            "H   H",
            4659.0,
            &[('H', 0, 0.0), ('H', 0, 3003.0)],
        ),
        (
            "the spaces the text starts with stay",
            "  H",
            5000.0,
            &[('H', 0, 898.0)],
        ),
        (
            "a newline ends a paragraph, whatever room is left",
            "H H\nH",
            9000.0,
            &[('H', 0, 0.0), ('H', 0, 2105.0), ('H', 1, 0.0)],
        ),
        (
            "every paragraph wraps by itself",
            "H H\nH H",
            1656.0,
            &[('H', 0, 0.0), ('H', 1, 0.0), ('H', 2, 0.0), ('H', 3, 0.0)],
        ),
        (
            "a word wider than the width stands alone and overflows",
            "HH H",
            2000.0,
            &[('H', 0, 0.0), ('H', 0, 1656.0), ('H', 1, 0.0)],
        ),
        (
            "an overlong word between two short ones takes its own line",
            "H HH H",
            3000.0,
            &[('H', 0, 0.0), ('H', 1, 0.0), ('H', 1, 1656.0), ('H', 2, 0.0)],
        ),
        (
            "an empty paragraph keeps its line",
            "H\n\nH",
            2000.0,
            &[('H', 0, 0.0), ('H', 2, 0.0)],
        ),
        (
            "a paragraph of spaces is an empty line",
            "H\n   \nH",
            2000.0,
            &[('H', 0, 0.0), ('H', 2, 0.0)],
        ),
        (
            "kerned letters: Hi To fits in 5121",
            "Hi To",
            5121.0,
            &[('H', 0, 0.0), ('i', 0, 1656.0), ('T', 0, 2706.0), ('o', 0, 3955.0)],
        ),
        (
            "kerned letters: Hi To wraps in 5120",
            "Hi To",
            5120.0,
            &[('H', 0, 0.0), ('i', 0, 1656.0), ('T', 1, 0.0), ('o', 1, 1249.0)],
        ),
    ];
    let font = fraunces()?;
    for (rule, content, width, expected) in rows {
        let shapes = shapes_of(content, at_em(&font).width(width))?;
        assert_eq!(shapes.len(), expected.len(), "{rule}");
        for (shape, &(character, line, x)) in shapes.iter().zip(expected) {
            let glyph = alone(&character.to_string())?;
            assert_eq!(
                shape.path(),
                &glyph.translate(x, f64::from(line) * LEADING)?,
                "{rule}: {character} of line {line} at {x}"
            );
            assert_eq!(shape.attributes().line(), Some(line), "{rule}");
        }
    }
    Ok(())
}

#[test]
fn tracking_counts_between_the_glyphs_of_a_line_and_not_before_the_first() -> Outcome {
    // tracking 100 is 200 pixels at 2000 to the em, between every two glyphs: "H H" is 1656 + 200 + 449 + 200 + 1656 = 4161
    // wide, and a line that starts at the third word is as wide as the first
    let font = fraunces()?;
    let rows: [(f64, &[(u32, f64)]); 2] = [
        (4161.0, &[(0, 0.0), (0, 2505.0), (1, 0.0), (1, 2505.0)]),
        (4160.0, &[(0, 0.0), (1, 0.0), (2, 0.0), (3, 0.0)]),
    ];
    for (width, expected) in rows {
        let shapes = shapes_of("H H H H", at_em(&font).tracking(100.0).width(width))?;
        assert_eq!(shapes.len(), expected.len(), "width {width}");
        for (shape, &(line, x)) in shapes.iter().zip(expected) {
            assert_eq!(
                shape.path(),
                &alone("H")?.translate(x, f64::from(line) * LEADING)?,
                "width {width}: line {line} at {x}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_width_not_above_zero_is_refused_naming_the_step_and_the_value() -> Outcome {
    let font = fraunces()?;
    for width in [0.0, -3.5, f64::NAN, f64::INFINITY] {
        let drawing = text("H", Text::new(&font).width(width));
        let (parameter, message) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error")?))?;
        assert_eq!(parameter, "source (text).width");
        assert!(message.contains(&width.to_string()), "{message}");
    }
    Ok(())
}

#[test]
fn each_wrapped_line_is_aligned_by_its_own_width() -> Outcome {
    // "Hi" is 2257 wide, "To" 2415; a line's trailing spaces are no part of its width
    let rows: [(&str, &str, Align, Glyphs); 3] = [
        (
            "right",
            "Hi To",
            Align::Right,
            &[
                ('H', 0, -2257.0),
                ('i', 0, -601.0),
                ('T', 1, -2415.0),
                ('o', 1, -1166.0),
            ],
        ),
        (
            "centre",
            "Hi To",
            Align::Center,
            &[('H', 0, -1128.5), ('i', 0, 527.5), ('T', 1, -1207.5), ('o', 1, 41.5)],
        ),
        ("trailing spaces", "H  ", Align::Right, &[('H', 0, -1656.0)]),
    ];
    let font = fraunces()?;
    for (rule, content, align, expected) in rows {
        let width = if content == "Hi To" { 3000.0 } else { 5000.0 };
        let shapes = shapes_of(content, at_em(&font).width(width).align(align))?;
        assert_eq!(shapes.len(), expected.len(), "{rule}");
        for (shape, &(character, line, x)) in shapes.iter().zip(expected) {
            let glyph = alone(&character.to_string())?;
            assert_eq!(
                shape.path(),
                &glyph.translate(x, f64::from(line) * LEADING)?,
                "{rule}: {character} of line {line}"
            );
        }
    }
    Ok(())
}

#[test]
fn the_lines_are_planned_at_the_reference_pose_and_never_change_with_the_values() -> Outcome {
    // at the defaults "H H" is 3761 wide: 3760 wraps it, and 3761 does not. A Param moved afterwards moves the glyphs but not
    // the breaks, whether it is set when drawn or fixed by `with_values`.
    let font = fraunces()?;
    let size = Param::new("size", EM, 500.0, 4000.0)?;
    let wrapped = text("H H", Text::new(&font).size(&size).width(3760.0));
    let whole = text("H H", Text::new(&font).size(&size).width(3761.0));
    let small = Values::new().set("size", 1000.0);
    let large = Values::new().set("size", 4000.0);
    assert_eq!(
        lines_of(&wrapped.shapes(&small)?),
        [Some(0), Some(1)],
        "narrower glyphs would fit, the lines stay"
    );
    assert_eq!(
        lines_of(&wrapped.with_values(&small).shapes(&Values::new())?),
        [Some(0), Some(1)],
        "with_values"
    );
    assert_eq!(
        lines_of(&whole.shapes(&large)?),
        [Some(0), Some(0)],
        "wider glyphs overflow, the lines stay"
    );
    let width = Param::new("width", 3760.0, 100.0, 90000.0)?;
    let by_param = text("H H", at_em(&font).width(&width));
    assert_eq!(
        lines_of(&by_param.shapes(&Values::new().set("width", 90000.0))?),
        [Some(0), Some(1)],
        "a width Param is read at its default"
    );
    Ok(())
}

/// The advance of a letter of the property below, from the literals in the header.
fn advance(letter: char) -> f64 {
    match letter {
        'H' => 1656.0,
        'i' => 601.0,
        _ => 449.0,
    }
}

/// Greedy wrapping of `words` joined by `separators` (the spaces between word `k` and `k + 1`) to `width`, worked out
/// from the literals: every glyph as its letter, line and the middle of its advance.
fn expected(words: &[String], separators: &[u32], width: f64) -> Vec<(char, u32, f64)> {
    let mut glyphs = Vec::new();
    let (mut line, mut pen) = (0_u32, 0.0);
    for (at, word) in words.iter().enumerate() {
        let wide: f64 = word.chars().map(advance).sum();
        let gap = if at == 0 {
            0.0
        } else {
            advance(' ') * f64::from(separators[at - 1])
        };
        if at > 0 && pen + gap + wide > width {
            line += 1;
            pen = 0.0;
        } else {
            pen += gap;
        }
        for letter in word.chars() {
            glyphs.push((letter, line, pen + advance(letter) / 2.0));
            pen += advance(letter);
        }
    }
    glyphs
}

proptest! {
    /// Wrapping loses no letter, keeps their order, and breaks exactly where greedy filling of the line says: a line is
    /// as full as the width allows and starts at the left edge.
    #[test]
    fn a_wrapped_text_is_what_greedy_filling_gives(
        words in prop::collection::vec("[Hi]{1,4}", 1..9),
        separators in prop::collection::vec(1_u32..4, 8),
        width in 500.0_f64..15000.0,
    ) {
        let mut content = words[0].clone();
        for (word, spaces) in words.iter().skip(1).zip(&separators) {
            content.push_str(&" ".repeat(*spaces as usize));
            content.push_str(word);
        }
        let shapes = shapes_of(&content, at_em(&fraunces()?).width(width))?;
        let drawn: Vec<(char, u32, f64)> = shapes
            .iter()
            .map(|shape| {
                let attributes = shape.attributes();
                (
                    attributes.char().unwrap_or(' '),
                    attributes.line().unwrap_or(u32::MAX),
                    attributes.origin().map_or(f64::NAN, |origin| origin.x),
                )
            })
            .collect();
        prop_assert_eq!(drawn, expected(&words, &separators, width), "{:?} to {}", content, width);
    }
}
