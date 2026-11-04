//! `text` layout through the public API: `width` wrapping the lines, `fit` making them exactly that wide. The expected
//! numbers are literals from `HarfBuzz` (uharfbuzz, an independent shaper) over `fraunces-subset.ttf` (`common/fonts.rs`
//! says how it was made), not computed by the library:
//!
//! ```text
//! uvx --from fonttools --with uharfbuzz python literals.py fraunces-subset.ttf "HiHiiHH ii" "Hi To" "iHi" ...
//! 'Hi To' [('H', 0, 1656), ('i', 1, 601), ('space', 2, 449), ('T', 3, 1249), ('o', 4, 1166)]
//! ```
//!
//! `H`, `i` and the space have advances of 1656, 601 and 449, and no kerning among them. The font has no glyph for U+00A0,
//! U+2007, U+202F, U+2003 or the tab: `HarfBuzz` draws the first two with the space glyph (449), U+202F with half of it (224),
//! U+2003 with the space glyph at 2000, and the tab with `.notdef` (1000); none of them draws an outline. Drawings are made at 2000 pixels to
//! the em, the font's own, so a pen position is a coordinate of the font file; the default leading is 2400, y pointing
//! down.

use crate::common::fonts::{fraunces, marks, narrowing};
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Align, Drawing, Error, Font, Inputs, Param, Path, Shape, Text, Values, text};
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

/// The outline of the one glyph `content` is, drawn `size` pixels to the em with its pen at (`x`, `y`).
fn glyph_at(content: &str, size: f64, x: f64, y: f64) -> Result<Path, Error> {
    let shapes = shapes_of(content, Text::new(&fraunces()?).size(size))?;
    assert_eq!(shapes.len(), 1, "{content:?}");
    shapes[0].path().translate(x, y)
}

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
    assert_wrapped(&rows)
}

#[test]
fn no_break_spaces_are_part_of_a_word_and_other_whitespace_still_breaks() -> Outcome {
    // (rule, text, width, glyphs); the advances of the characters the font lacks are in the header
    let rows: [(&str, &str, f64, Glyphs); 8] = [
        (
            "a no-break space keeps two words on one line when they fit",
            "H\u{a0}H",
            3761.0,
            &[('H', 0, 0.0), ('H', 0, 2105.0)],
        ),
        (
            "words joined by a no-break space move to the next line together",
            "H H\u{a0}H",
            3761.0,
            &[('H', 0, 0.0), ('H', 1, 0.0), ('H', 1, 2105.0)],
        ),
        (
            "a figure space (U+2007) is no break",
            "H\u{2007}H",
            3761.0,
            &[('H', 0, 0.0), ('H', 0, 2105.0)],
        ),
        (
            "a narrow no-break space (U+202F) is no break",
            "H\u{202f}H",
            3536.0,
            &[('H', 0, 0.0), ('H', 0, 1880.0)],
        ),
        ("a tab still breaks", "H\tH", 3000.0, &[('H', 0, 0.0), ('H', 1, 0.0)]),
        (
            "an em space (U+2003) still breaks",
            "H\u{2003}H",
            3000.0,
            &[('H', 0, 0.0), ('H', 1, 0.0)],
        ),
        (
            "a word joined by a no-break space and wider than the width stands alone and overflows",
            "H\u{a0}H H",
            3000.0,
            &[('H', 0, 0.0), ('H', 0, 2105.0), ('H', 1, 0.0)],
        ),
        (
            "a no-break space the text starts with is part of its first word",
            "\u{a0}H H",
            2105.0,
            &[('H', 0, 449.0), ('H', 1, 0.0)],
        ),
    ];
    assert_wrapped(&rows)
}

/// Checks that each row's text, wrapped to its width, has every glyph as its character, line and pen x.
fn assert_wrapped(rows: &[(&str, &str, f64, Glyphs)]) -> Outcome {
    let font = fraunces()?;
    for (rule, content, width, expected) in rows.iter().copied() {
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
fn a_width_param_moves_the_breaks_as_drawn() -> Outcome {
    // the width is what the lines are made to, so it is read as drawn, and fixed by `with_values` too: "H H" is 3761 wide
    let width = Param::new("width", 3760.0, 100.0, 90000.0)?;
    let drawing = text("H H", at_em(&fraunces()?).width(&width));
    for (value, lines) in [
        (3760.0, [Some(0), Some(1)]),
        (3761.0, [Some(0), Some(0)]),
        (90000.0, [Some(0), Some(0)]),
    ] {
        let values = Values::new().set("width", value);
        assert_eq!(lines_of(&drawing.shapes(&values)?), lines, "width {value}");
        assert_eq!(
            lines_of(&drawing.with_values(&values).shapes(&Values::new())?),
            lines,
            "width {value} fixed"
        );
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
    let rows: [(&str, &str, Align, Glyphs); 4] = [
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
        (
            "a trailing no-break space is part of the width",
            "H\u{a0}",
            Align::Right,
            &[('H', 0, -2105.0)],
        ),
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
    // at the defaults "H H" is 3761 wide: 3760 wraps it, and 3761 does not. A size Param moved afterwards moves the glyphs but
    // not the breaks, whether it is set when drawn or fixed by `with_values`.
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

// fit = "size" --------------------------------------------------------------------------------------------------------

/// Glyphs as their character, the size of their line, and where the pen stands.
type Sized = &'static [(char, f64, f64, f64)];

/// A row of the fit table: its rule, text, tracking, width, leading and the glyphs expected.
type FitRow = (&'static str, &'static str, f64, f64, Option<f64>, Sized);

#[test]
fn fit_size_sets_each_line_to_the_size_that_makes_it_exactly_the_width_wide() -> Outcome {
    // Each glyph as its character, size and pen x and y. A line of w units at 2000 to the em is
    // W wide at 2000 * W / w; a line's default leading, from the line above to it, is 1.2 times its own size.
    let rows: [FitRow; 7] = [
        ("one glyph", "H", 0.0, 3312.0, None, &[('H', 4000.0, 0.0, 0.0)]),
        (
            "a word",
            "HH",
            0.0,
            3312.0,
            None,
            &[('H', 2000.0, 0.0, 0.0), ('H', 2000.0, 1656.0, 0.0)],
        ),
        (
            "a short line then a long one: the second stands 1.2 of its own size below",
            "H\nHH",
            0.0,
            3312.0,
            None,
            &[
                ('H', 4000.0, 0.0, 0.0),
                ('H', 2000.0, 0.0, 2400.0),
                ('H', 2000.0, 1656.0, 2400.0),
            ],
        ),
        (
            "a long line then a short one: the second stands 1.2 of its own size below",
            "HH\nH",
            0.0,
            3312.0,
            None,
            &[
                ('H', 2000.0, 0.0, 0.0),
                ('H', 2000.0, 1656.0, 0.0),
                ('H', 4000.0, 0.0, 4800.0),
            ],
        ),
        (
            "a leading of the text is the same between every two lines",
            "H\nHH\nH",
            0.0,
            3312.0,
            Some(3000.0),
            &[
                ('H', 4000.0, 0.0, 0.0),
                ('H', 2000.0, 0.0, 3000.0),
                ('H', 2000.0, 1656.0, 3000.0),
                ('H', 4000.0, 0.0, 6000.0),
            ],
        ),
        (
            "wrapped lines are each fitted: the breaks are made at the size of the text",
            "H H",
            0.0,
            828.0,
            None,
            &[('H', 1000.0, 0.0, 0.0), ('H', 1000.0, 0.0, 1200.0)],
        ),
        (
            "an empty line keeps the size of the text for its spacing",
            "H\n\nH",
            0.0,
            3312.0,
            None,
            &[('H', 4000.0, 0.0, 0.0), ('H', 4000.0, 0.0, 7200.0)],
        ),
    ];
    let font = fraunces()?;
    for (rule, content, tracking, width, leading, expected) in rows {
        let mut options = at_em(&font).tracking(tracking).width(width).fit("size");
        if let Some(leading) = leading {
            options = options.leading(leading);
        }
        let shapes = shapes_of(content, options)?;
        assert_eq!(shapes.len(), expected.len(), "{rule}");
        for (shape, &(character, size, x, y)) in shapes.iter().zip(expected) {
            assert_eq!(
                shape.path(),
                &glyph_at(&character.to_string(), size, x, y)?,
                "{rule}: {character} at size {size}, pen ({x}, {y})"
            );
        }
    }
    Ok(())
}

#[test]
fn fit_size_scales_the_tracking_with_the_line() -> Outcome {
    // 1000 to the em, tracking 100: "H H" is 1880.5 + 2 * 100 = 2080.5 wide, so 4161 wide takes size 2000 and, with the
    // tracking at 200, the second H stands at 1656 + 200 + 449 + 200
    let shapes = shapes_of(
        "H H",
        Text::new(&fraunces()?)
            .size(1000.0)
            .tracking(100.0)
            .width(4161.0)
            .fit("size"),
    )?;
    assert_eq!(shapes.len(), 2);
    assert_eq!(shapes[0].path(), &glyph_at("H", 2000.0, 0.0, 0.0)?);
    assert_eq!(shapes[1].path(), &glyph_at("H", 2000.0, 2505.0, 0.0)?);
    Ok(())
}

#[test]
fn the_size_of_a_fitted_text_is_planned_at_the_reference_pose() -> Outcome {
    // the size Param is read at its default, 2000, whatever it is drawn at: it neither moves the fitted size nor the breaks
    let font = fraunces()?;
    let size = Param::new("size", EM, 500.0, 4000.0)?;
    let drawing = text("H\nHH", Text::new(&font).size(&size).width(3312.0).fit("size"));
    let plain = drawing.shapes(&Values::new())?;
    for values in [Values::new().set("size", 700.0), Values::new().set("size", 4000.0)] {
        let moved = drawing.shapes(&values)?;
        assert_eq!(moved.len(), plain.len());
        assert!(
            moved.iter().zip(&plain).all(|(a, b)| a.path() == b.path()),
            "the fitted text moved"
        );
    }
    let fixed = drawing
        .with_values(&Values::new().set("size", 700.0))
        .shapes(&Values::new())?;
    assert!(
        fixed.iter().zip(&plain).all(|(a, b)| a.path() == b.path()),
        "with_values moved the fitted text"
    );
    Ok(())
}

#[test]
fn a_fit_without_a_width_or_that_is_not_a_fit_is_refused_naming_it() -> Outcome {
    let font = fraunces()?;
    let rows = [
        ("fit needs a width", Text::new(&font).fit("size"), "width"),
        (
            "a word that is no fit",
            Text::new(&font).width(100.0).fit("bold"),
            "\"bold\"",
        ),
        (
            "a fit in other case",
            Text::new(&font).width(100.0).fit("Size"),
            "\"Size\"",
        ),
    ];
    for (rule, options, shown) in rows {
        let drawing = text("H", options);
        let (parameter, message) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error")?))?;
        assert_eq!(parameter, "source (text).fit", "{rule}");
        assert!(message.contains(shown), "{rule}: {message}");
    }
    assert_eq!(text("H", Text::new(&font).width(100.0).fit("size")).error(), None);
    Ok(())
}

#[test]
fn a_line_with_no_width_cannot_be_fitted() -> Outcome {
    // marks.ttf: the acute (U+0301) has no advance
    let drawing = text("\u{301}", Text::new(&marks()?).width(100.0).fit("size"));
    let (parameter, message) = parameter_refusal(drawing.shapes(&Values::new()))?;
    assert_eq!(parameter, "source (text).fit");
    assert!(message.contains("line 0") && message.contains("100"), "{message}");
    Ok(())
}

// fit = an axis -------------------------------------------------------------------------------------------------------

/// The x of the middle of the advance of each glyph of the first line: an advance is the step from one to the next.
fn centres(shapes: &[Shape]) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    shapes
        .iter()
        .map(|shape| Ok(shape.attributes().origin().ok_or("no origin")?.x))
        .collect()
}

/// The advance of the first glyph of `HHH`-like text drawn as `shapes`.
fn step(shapes: &[Shape]) -> Result<f64, Box<dyn std::error::Error>> {
    let centres = centres(shapes)?;
    Ok(centres[1] - centres[0])
}

/// A row of the axis table: its rule, font, axis, the font's units to the em, text, width and the advance expected.
type AxisRow<'a> = (&'a str, &'a Font, &'a str, f64, &'a str, f64, f64);

#[test]
fn fit_on_an_axis_makes_the_line_exactly_the_width_wide() -> Outcome {
    // fraunces-subset.ttf, HarfBuzz: the advance of H at wght 300, 400, 500, 600, 700 is 1608, 1656, 1680, 1704, 1752
    // (it rises with the weight, in steps of a unit, so a width of three H that one weight makes is made at that weight);
    // narrowing.ttf, nrrw 0, 500, 1000: the advance of A is 1000, 800, 600 (it falls with the axis)
    let (weight, narrow) = (fraunces()?, narrowing()?);
    let rows: [AxisRow<'_>; 8] = [
        (
            "the minimum is reached",
            &weight,
            "wght",
            2000.0,
            "HHH",
            3.0 * 1608.0,
            1608.0,
        ),
        ("the default", &weight, "wght", 2000.0, "HHH", 3.0 * 1656.0, 1656.0),
        (
            "a weight in between",
            &weight,
            "wght",
            2000.0,
            "HHH",
            3.0 * 1680.0,
            1680.0,
        ),
        ("another", &weight, "wght", 2000.0, "HHH", 3.0 * 1704.0, 1704.0),
        (
            "the maximum is reached",
            &weight,
            "wght",
            2000.0,
            "HHH",
            3.0 * 1752.0,
            1752.0,
        ),
        (
            "a falling axis, at its start",
            &narrow,
            "nrrw",
            1000.0,
            "AA",
            2000.0,
            1000.0,
        ),
        ("a falling axis, between", &narrow, "nrrw", 1000.0, "AA", 1600.0, 800.0),
        (
            "a falling axis, at its end",
            &narrow,
            "nrrw",
            1000.0,
            "AA",
            1200.0,
            600.0,
        ),
    ];
    for (rule, font, axis, size, content, width, advance) in rows {
        let shapes = shapes_of(content, Text::new(font).size(size).width(width).fit(axis))?;
        assert_eq!(shapes.len(), content.len(), "{rule}");
        let step = step(&shapes)?;
        assert!(
            (step - advance).abs() < 1e-9,
            "{rule}: the advance is {step}, not {advance}"
        );
    }
    Ok(())
}

#[test]
fn a_width_the_axis_cannot_reach_is_refused_naming_the_line_and_the_nearest_it_reaches() -> Outcome {
    // (rule, text, width, font, axis and its size, the line named, the nearest width the axis reaches, the axis value there)
    let (weight, narrow) = (fraunces()?, narrowing()?);
    let rows = [
        (
            "below the minimum",
            "HHH",
            4823.0,
            &weight,
            "wght",
            2000.0,
            "line 0",
            "4824",
            "300",
        ),
        (
            "above the maximum",
            "HHH",
            5257.0,
            &weight,
            "wght",
            2000.0,
            "line 0",
            "5256",
            "700",
        ),
        (
            "the second line",
            "HHH\nH",
            4824.0,
            &weight,
            "wght",
            2000.0,
            "line 1",
            "1752",
            "700",
        ),
        (
            "a falling axis, above its start",
            "AA",
            2001.0,
            &narrow,
            "nrrw",
            1000.0,
            "line 0",
            "2000",
            "0",
        ),
        (
            "a falling axis, below its end",
            "AA",
            1199.0,
            &narrow,
            "nrrw",
            1000.0,
            "line 0",
            "1200",
            "1000",
        ),
    ];
    for (rule, content, width, font, axis, size, line, nearest, at) in rows {
        let drawing = text(content, Text::new(font).size(size).width(width).fit(axis));
        let (parameter, message) = parameter_refusal(drawing.shapes(&Values::new()))?;
        assert_eq!(parameter, "source (text).fit", "{rule}");
        for shown in [line, nearest, at, &format!("{axis:?}")] {
            assert!(message.contains(shown), "{rule}: {shown} is not in {message}");
        }
    }
    Ok(())
}

#[test]
fn an_axis_the_font_lacks_or_the_text_sets_already_is_no_fit() -> Outcome {
    let font = fraunces()?;
    let weight = font.axes().remove(0);
    let rows = [
        (
            "an axis the font lacks",
            Text::new(&font).width(100.0).fit("opsz"),
            "opsz",
        ),
        (
            "an axis the text gives a value",
            Text::new(&font).width(100.0).fit("wght").axes([("wght", &weight)]),
            "axes",
        ),
    ];
    for (rule, options, shown) in rows {
        let drawing = text("H", options);
        let (parameter, message) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error")?))?;
        assert_eq!(parameter, "source (text).fit", "{rule}");
        assert!(message.contains(shown), "{rule}: {message}");
    }
    assert_eq!(text("H", Text::new(&font).width(100.0).fit("wght")).error(), None);
    Ok(())
}

#[test]
fn an_empty_line_is_not_fitted_to_an_axis() -> Outcome {
    let shapes = shapes_of("HHH\n\nHHH", Text::new(&fraunces()?).size(EM).width(5040.0).fit("wght"))?;
    assert_eq!(shapes.len(), 6);
    assert_eq!(lines_of(&shapes[3..]), [Some(2); 3]);
    assert!((step(&shapes[3..])? - 1680.0).abs() < 1e-9);
    Ok(())
}

#[test]
fn the_weight_is_planned_at_the_reference_pose() -> Outcome {
    // three H of 1680 each fill 5040 at wght 500. With the size Param at 1000 instead of its default 2000 the same weight
    // is kept, so the advance halves; fitting at the live size would find no weight to make 5040 of 2520 to 2628
    let font = fraunces()?;
    let size = Param::new("size", EM, 500.0, 4000.0)?;
    let drawing = text("HHH", Text::new(&font).size(&size).width(5040.0).fit("wght"));
    for (live, advance) in [(2000.0, 1680.0), (1000.0, 840.0)] {
        let shapes = drawing.shapes(&Values::new().set("size", live))?;
        assert!((step(&shapes)? - advance).abs() < 1e-9, "size {live}");
    }
    let fixed = drawing
        .with_values(&Values::new().set("size", 1000.0))
        .shapes(&Values::new())?;
    assert!((step(&fixed)? - 840.0).abs() < 1e-9, "with_values");
    Ok(())
}

proptest! {
    /// A wider width never gets a narrower line, and the line is within a unit a glyph of the width: the shaper rounds
    /// each advance to a unit. The line is twice the distance from the middle of the first glyph to the middle of the
    /// second (the first's advance, and the second's). "Hi" is 1608 + 568 wide at wght 300 and 1752 + 667 at 700.
    #[test]
    fn a_fitted_line_is_as_wide_as_asked_and_grows_with_the_width(a in 2176.0_f64..=2419.0, b in 2176.0_f64..=2419.0) {
        let font = fraunces()?;
        let total = |width: f64| -> Result<f64, TestCaseError> {
            let shapes = shapes_of("Hi", at_em(&font).width(width).fit("wght"))?;
            let centres = centres(&shapes).map_err(|error| TestCaseError::fail(error.to_string()))?;
            Ok(2.0 * (centres[1] - centres[0]))
        };
        let (narrow, wide) = (a.min(b), a.max(b));
        let (at_narrow, at_wide) = (total(narrow)?, total(wide)?);
        prop_assert!((at_narrow - narrow).abs() <= 2.0, "{} for {}", at_narrow, narrow);
        prop_assert!((at_wide - wide).abs() <= 2.0, "{} for {}", at_wide, wide);
        prop_assert!(at_narrow <= at_wide, "{} for {} against {} for {}", at_narrow, narrow, at_wide, wide);
    }
}

// In a document -------------------------------------------------------------------------------------------------------

/// A document with the font `anybody` and one text of `fi` whose extra arguments are `arguments`.
fn document(arguments: &str) -> String {
    format!(
        r#"{{"version":1,"inputs":{{"anybody":{{"kind":"font"}}}},"root":[{{"text":{{"content":"H H","font":"anybody",{arguments}}}}}]}}"#
    )
}

#[test]
fn width_and_fit_are_written_and_read_back_and_a_text_without_them_writes_neither() -> Outcome {
    let font = fraunces()?.named("anybody")?;
    let inputs = Inputs::new().set("anybody", crate::common::fonts::FRAUNCES);
    for (options, written) in [
        (at_em(&font).width(3760.0), vec![r#""width":3760"#]),
        (
            at_em(&font).width(2000.0).fit("size"),
            vec![r#""width":2000"#, r#""fit":"size""#],
        ),
        (
            at_em(&font).width(1700.0).fit("wght"),
            vec![r#""width":1700"#, r#""fit":"wght""#],
        ),
        (at_em(&font), vec![]),
    ] {
        let drawing = text("H H", options);
        let json = drawing.to_json()?;
        for part in &written {
            assert!(json.contains(part), "{part} is not in {json}");
        }
        if written.is_empty() {
            assert!(!json.contains("width") && !json.contains("fit"), "{json}");
        }
        let loaded = Drawing::from_json(&json, &inputs)?;
        assert_eq!(loaded.to_json()?, json);
        let (a, b) = (loaded.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
        assert_eq!(a.len(), b.len(), "{json}");
        assert!(a.iter().zip(&b).all(|(a, b)| a.path() == b.path()), "{json}");
    }
    Ok(())
}

#[test]
fn a_document_with_a_fit_that_cannot_be_made_is_refused_when_loaded() -> Outcome {
    let inputs = Inputs::new().set("anybody", crate::common::fonts::FRAUNCES);
    // (rule, arguments, what the refusal names)
    let rows = [
        ("fit without a width", r#""fit":"size""#, "fit"),
        (
            "an axis given too",
            r#""width":100,"fit":"wght","axes":{"wght":400}"#,
            "fit",
        ),
        ("a width of zero", r#""width":0"#, "width"),
        ("a width that is a word", r#""width":"wide""#, "width"),
    ];
    for (rule, arguments, named) in rows {
        let refused = Drawing::from_json(&document(arguments), &inputs);
        let message = match refused {
            Err(Error::Parameter { parameter, message }) => format!("{parameter}: {message}"),
            Err(Error::Format { pointer, message }) => format!("{pointer}: {message}"),
            Err(other) => return Err(format!("{rule}: {other}").into()),
            Ok(_) => return Err(format!("{rule}: loaded").into()),
        };
        assert!(message.contains(named), "{rule}: {message}");
    }
    Ok(())
}
