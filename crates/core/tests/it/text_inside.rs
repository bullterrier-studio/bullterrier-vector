//! `text` set `inside` a shape, through the public API. The pen positions are computed by hand from `HarfBuzz` literals
//! over `fraunces-subset.ttf` (`text_layout.rs` says how they were made) and from the geometry of the shapes, not by the
//! library:
//!
//! `H` 1656, `i` 601, space 449 (no kerning among them); 2000 units to the em and a capital 1400 high, so at 2000 pixels
//! to the em the first baseline is 1400 below the top of the shape and the default leading is 2400. A word of `n` letters
//! `H` set with `m-1` spaces is `1656 n + 449 (m-1)` wide: `H H` is 3761. A glyph row is (character, line, pen x, baseline).

use crate::common::fonts::fraunces;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Align, Drawing, Error, Font, Inputs, Param, Path, Shape, Text, Values, text};
use proptest::prelude::*;

/// 2000 pixels to the em: the font's own.
const EM: f64 = 2000.0;

fn at_em(font: &Font) -> Text {
    Text::new(font).size(EM)
}

/// A glyph row: its character, its line, and where its pen stands on the baseline.
type Pen = (char, u32, f64, f64);

/// Glyph rows.
type Glyphs<'a> = &'a [Pen];

/// Where the pen of each shape stood: the shape's bounds against the bounds of the same glyph drawn at the origin.
fn pens(shapes: &[Shape]) -> Result<Vec<Pen>, Box<dyn std::error::Error>> {
    let font = fraunces()?;
    shapes
        .iter()
        .map(|shape| {
            let character = shape.attributes().char().ok_or("no char")?;
            let line = shape.attributes().line().ok_or("no line")?;
            let at_origin = text(character.to_string(), at_em(&font)).shapes(&Values::new())?;
            let (home, bounds) = (at_origin[0].path().bounds()?, shape.path().bounds()?);
            Ok((character, line, bounds.x - home.x, bounds.y - home.y))
        })
        .collect()
}

/// The glyphs `drawing` draws match `want`, to within `tolerance` (a curve is flattened to straight edges).
fn assert_pens(rule: &str, drawing: &Drawing, want: Glyphs<'_>, tolerance: f64) -> Outcome {
    let got = pens(&drawing.shapes(&Values::new())?)?;
    assert_eq!(got.len(), want.len(), "{rule}: {got:?}");
    for (got, want) in got.iter().zip(want) {
        assert!(
            got.0 == want.0
                && got.1 == want.1
                && (got.2 - want.2).abs() <= tolerance
                && (got.3 - want.3).abs() <= tolerance,
            "{rule}: got {got:?}, wanted {want:?}"
        );
    }
    Ok(())
}

fn svg(data: &str) -> Result<Path, Error> {
    Path::from_svg_path(data)
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(x, y, width, height, 0.0)
}

fn refusal_of(drawing: &Drawing) -> Result<(String, String), Box<dyn std::error::Error>> {
    match drawing.error() {
        Some(error) => parameter_refusal(Err::<(), _>(error.clone())),
        None => parameter_refusal(drawing.shapes(&Values::new())),
    }
}

// the lines of a text in a shape ----------------------------------------------------------------------------------------

#[test]
fn lines_are_set_in_the_stretches_of_the_shape() -> Outcome {
    let font = fraunces()?;
    // A step: 2000 wide above y 3000, 10000 wide below.
    let step = svg("M0 0H2000V3000H10000V9000H0Z")?;
    // A hole 3000 to 6000 wide from y 3000 to 17000, in a frame 10000 wide.
    let holed = svg("M0 0H10000V20000H0Z M3000 3000V17000H6000V3000Z")?;
    let tight = rect(500.0, 300.0, 3761.0, 20000.0)?;
    // (rule, shape, text, options, glyphs)
    let rows: Vec<(&str, &Path, &str, Text, Glyphs<'_>)> = vec![
        (
            "a rectangle fills as a width does, from a capital below its top",
            &tight,
            "H H H",
            at_em(&font),
            &[
                ('H', 0, 500.0, 1700.0),
                ('H', 0, 2605.0, 1700.0),
                ('H', 1, 500.0, 4100.0),
            ],
        ),
        (
            "a line whose baseline is in a narrow part but whose capitals are not still takes the narrow chord",
            &step,
            "H H H H H H",
            at_em(&font),
            &[
                ('H', 0, 0.0, 1400.0),
                ('H', 1, 0.0, 3800.0),
                ('H', 2, 0.0, 6200.0),
                ('H', 2, 2105.0, 6200.0),
                ('H', 2, 4210.0, 6200.0),
                ('H', 2, 6315.0, 6200.0),
            ],
        ),
        (
            "round a hole the widest stretch takes the line",
            &holed,
            "H H H H H H",
            at_em(&font),
            &[
                ('H', 0, 0.0, 1400.0),
                ('H', 0, 2105.0, 1400.0),
                ('H', 0, 4210.0, 1400.0),
                ('H', 0, 6315.0, 1400.0),
                ('H', 1, 6000.0, 3800.0),
                ('H', 1, 8105.0, 3800.0),
            ],
        ),
    ];
    for (rule, shape, content, options, want) in rows {
        assert_pens(rule, &text(content, options.inside(shape)), want, 1e-9)?;
    }
    Ok(())
}

#[test]
fn columns_empty_lines_and_leading_place_the_baselines() -> Outcome {
    let font = fraunces()?;
    // Two columns of one width, and of two.
    let level = svg("M0 0H3000V9000H0Z M5000 0H8000V9000H5000Z")?;
    let unequal = svg("M0 0H3000V9000H0Z M4000 0H9000V9000H4000Z")?;
    // 1000 wide above y 3000, 5000 below.
    let neck = svg("M0 0H1000V3000H5000V9000H0Z")?;
    let big = rect(0.0, 0.0, 10000.0, 20000.0)?;
    let rows: Vec<(&str, &Path, &str, Text, Glyphs<'_>)> = vec![
        (
            "equal columns: the leftmost",
            &level,
            "H",
            at_em(&font),
            &[('H', 0, 0.0, 1400.0)],
        ),
        (
            "equal columns: a word that fits there goes on, and the next line is the leftmost again",
            &level,
            "H H",
            at_em(&font),
            &[('H', 0, 0.0, 1400.0), ('H', 1, 0.0, 3800.0)],
        ),
        (
            "unequal columns: the widest",
            &unequal,
            "H H",
            at_em(&font),
            &[('H', 0, 4000.0, 1400.0), ('H', 0, 6105.0, 1400.0)],
        ),
        (
            "a word too wide for a baseline's chord waits for a baseline where it fits, and no line is spent on the others",
            &neck,
            "HH",
            at_em(&font),
            &[('H', 0, 0.0, 6200.0), ('H', 0, 1656.0, 6200.0)],
        ),
        (
            "an empty line takes a baseline",
            &big,
            "H\n\nH",
            at_em(&font),
            &[('H', 0, 0.0, 1400.0), ('H', 2, 0.0, 6200.0)],
        ),
        (
            "a line of spaces is an empty line",
            &big,
            "H\n   \nH",
            at_em(&font),
            &[('H', 0, 0.0, 1400.0), ('H', 2, 0.0, 6200.0)],
        ),
        (
            "a newline ends a line whatever room is left",
            &big,
            "H H\nH",
            at_em(&font),
            &[('H', 0, 0.0, 1400.0), ('H', 0, 2105.0, 1400.0), ('H', 1, 0.0, 3800.0)],
        ),
        (
            "leading is the step from one baseline to the next",
            &big,
            "H\nH",
            at_em(&font).leading(3000.0),
            &[('H', 0, 0.0, 1400.0), ('H', 1, 0.0, 4400.0)],
        ),
    ];
    for (rule, shape, content, options, want) in rows {
        assert_pens(rule, &text(content, options.inside(shape)), want, 1e-9)?;
    }
    Ok(())
}

#[test]
fn align_places_each_line_against_its_stretch() -> Outcome {
    let font = fraunces()?;
    // the stretch is 1000 to 10000: its middle is 5500
    let shape = rect(1000.0, 0.0, 9000.0, 9000.0)?;
    let rows: [(&str, &str, Align, Glyphs<'_>); 5] = [
        ("left: at the start", "H", Align::Left, &[('H', 0, 1000.0, 1400.0)]),
        (
            "centre: half the width either side of the middle",
            "H",
            Align::Center,
            &[('H', 0, 4672.0, 1400.0)],
        ),
        (
            "right: ending at the end",
            "H",
            Align::Right,
            &[('H', 0, 8344.0, 1400.0)],
        ),
        (
            "centre, two words",
            "H H",
            Align::Center,
            &[('H', 0, 3619.5, 1400.0), ('H', 0, 5724.5, 1400.0)],
        ),
        (
            "right, a line of two words",
            "H H",
            Align::Right,
            &[('H', 0, 6239.0, 1400.0), ('H', 0, 8344.0, 1400.0)],
        ),
    ];
    for (rule, content, align, want) in rows {
        assert_pens(
            rule,
            &text(content, at_em(&font).align(align).inside(&shape)),
            want,
            1e-9,
        )?;
    }
    Ok(())
}

#[test]
fn a_circle_narrows_the_lines_toward_the_top_and_the_capital_line_counts() -> Outcome {
    // centre (5000, 5000), radius 5000. A line's chord is the narrower of the circle at its baseline and at the top of its
    // capitals, 1400 above: the one further from the centre. Baselines are 1400, 3800, 6200, 8600, ....
    let (centre, radius) = (5000.0_f64, 5000.0_f64);
    let half = |away: f64| (radius * radius - away * away).sqrt();
    // (baseline, how far from the centre the narrower of the two ends, words in the line). The top baseline touches the
    // circle at its top and has no width, so it is skipped. At 2105 per word and 449 less: four words are 7971 wide, five
    // 10076, three 5866.
    let lines: [(f64, f64, u32); 3] = [(3800.0, 2600.0, 4), (6200.0, 1200.0, 4), (8600.0, 3600.0, 3)];
    let mut want = Vec::new();
    for (number, (baseline, away, words)) in lines.into_iter().enumerate() {
        for word in 0..words {
            want.push((
                'H',
                u32::try_from(number)?,
                centre - half(away) + 2105.0 * f64::from(word),
                baseline,
            ));
        }
    }
    let circle = Path::circle((centre, centre), radius)?;
    let drawing = text(["H"; 11].join(" "), at_em(&fraunces()?).inside(&circle));
    // the circle is four cubic arcs, which stray from a true circle by up to 2.7e-4 of the radius (1.4), and across a
    // slanted edge that is a little more sideways
    assert_pens("circle", &drawing, &want, 2.5)
}

// the text and the shape disagree -------------------------------------------------------------------------------------

#[test]
fn a_text_the_shape_cannot_hold_is_refused_naming_the_word_and_the_line() -> Outcome {
    let font = fraunces()?;
    let narrow = rect(0.0, 0.0, 2000.0, 20000.0)?;
    let short = rect(0.0, 0.0, 3761.0, 3000.0)?;
    // (rule, shape, text, what the refusal names)
    let rows = [
        (
            "more lines than baselines",
            &short,
            "H H H",
            vec!["does not fit", "\"H\"", "line 1"],
        ),
        (
            "a word wider than every chord",
            &narrow,
            "H HH",
            vec!["does not fit", "\"HH\"", "line 1"],
        ),
    ];
    for (rule, shape, content, named) in rows {
        let (parameter, message) = parameter_refusal(text(content, at_em(&font).inside(shape)).shapes(&Values::new()))?;
        assert_eq!(parameter, "source (text).inside", "{rule}");
        for part in named {
            assert!(message.contains(part), "{rule}: {part} is not in {message}");
        }
    }
    Ok(())
}

#[test]
fn a_shape_with_no_outline_holds_no_text() -> Outcome {
    let drawing = text("H", at_em(&fraunces()?).inside(&Path::new()));
    let (parameter, message) = refusal_of(&drawing)?;
    assert_eq!(parameter, "source (text).inside");
    assert!(message.contains("no outline"), "{message}");
    Ok(())
}

#[test]
fn baselines_to_try_are_bounded() -> Outcome {
    // a leading of a thousandth of a pixel in a shape a billion tall has more baselines than any text may use
    let drawing = text(
        "HH",
        at_em(&fraunces()?).leading(0.001).inside(&rect(0.0, 0.0, 1000.0, 1e9)?),
    );
    let (parameter, message) = refusal_of(&drawing)?;
    assert_eq!(parameter, "source (text).inside");
    assert!(message.contains("MAX_GLYPHS"), "{message}");
    Ok(())
}

#[test]
fn inside_excludes_what_places_the_text_and_wants_a_leading_above_zero() -> Outcome {
    let font = fraunces()?;
    let shape = rect(0.0, 0.0, 9000.0, 9000.0)?;
    let lead = Param::new("lead", 0.0, -10.0, 10.0)?;
    // (rule, options, the argument the refusal is about, what it names, whether the drawing is refused as it is built)
    let rows: Vec<(&str, Text, &str, &str, bool)> = vec![
        ("a width", at_em(&font).width(3761.0), "inside", "width", true),
        (
            "a fit and a width",
            at_em(&font).width(3761.0).fit("size"),
            "inside",
            "width",
            true,
        ),
        ("a fit alone", at_em(&font).fit("size"), "inside", "fit", true),
        ("an anchor", at_em(&font).anchor((10.0, 20.0)), "inside", "anchor", true),
        ("a leading of zero", at_em(&font).leading(0.0), "leading", "0", true),
        (
            "a negative leading",
            at_em(&font).leading(-5.5),
            "leading",
            "-5.5",
            true,
        ),
        (
            "a leading Param that starts at zero, read when drawn at the reference pose",
            at_em(&font).leading(&lead),
            "leading",
            "0",
            false,
        ),
    ];
    for (rule, options, argument, named, early) in rows {
        let drawing = text("H", options.inside(&shape));
        assert_eq!(drawing.error().is_some(), early, "{rule}");
        let (parameter, message) = refusal_of(&drawing)?;
        assert_eq!(parameter, format!("source (text).{argument}"), "{rule}");
        assert!(message.contains(named), "{rule}: {message}");
    }
    let fine = text("H", at_em(&font).leading(1.0).inside(&shape));
    assert_eq!(fine.error(), None);
    Ok(())
}

// the lines do not move with a Param ----------------------------------------------------------------------------------

#[test]
fn the_lines_are_made_at_the_reference_pose() -> Outcome {
    // "H H H" in 3761: two lines at size 2000. At half the size the glyphs are half as big, but the lines stay.
    let size = Param::new("size", 2000.0, 100.0, 4000.0)?;
    let drawing = text(
        "H H H",
        Text::new(&fraunces()?)
            .size(&size)
            .inside(&rect(0.0, 0.0, 3761.0, 20000.0)?),
    );
    let lines = |size: f64| -> Result<Vec<Option<u32>>, Error> {
        Ok(drawing
            .shapes(&Values::new().set("size", size))?
            .iter()
            .map(|shape| shape.attributes().line())
            .collect())
    };
    assert_eq!(lines(2000.0)?, [Some(0), Some(0), Some(1)]);
    assert_eq!(lines(1000.0)?, [Some(0), Some(0), Some(1)]);
    Ok(())
}

// invariants ----------------------------------------------------------------------------------------------------------

/// The words a text is made of: letters of the test font with plain advances.
fn words() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        proptest::sample::select(vec!["H", "i", "I", "l", "o", "HH", "Hi", "il"]),
        1..14,
    )
    .prop_map(|words| words.join(" "))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// In a rectangle, `inside` gives the lines `width` gives, moved to the rectangle's corner and a capital down.
    #[test]
    fn a_rectangle_is_a_width(content in words(), width in 3400.0..12000.0_f64, x in 0.0..500.0_f64, y in 0.0..500.0_f64) {
        let font = fraunces().map_err(|e| TestCaseError::fail(e.to_string()))?;
        let fail = |e: Box<dyn std::error::Error>| TestCaseError::fail(e.to_string());
        let shape = rect(x, y, width, 1e6).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let inside = text(content.clone(), at_em(&font).inside(&shape)).shapes(&Values::new()).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let wrapped = text(content, at_em(&font).width(width)).shapes(&Values::new()).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let (inside, wrapped) = (pens(&inside).map_err(fail)?, pens(&wrapped).map_err(fail)?);
        prop_assert_eq!(inside.len(), wrapped.len());
        for (inside, wrapped) in inside.iter().zip(&wrapped) {
            prop_assert_eq!((inside.0, inside.1), (wrapped.0, wrapped.1));
            prop_assert!((inside.2 - (wrapped.2 + x)).abs() < 1e-6, "{:?} against {:?}", inside, wrapped);
            prop_assert!((inside.3 - (wrapped.3 + y + 1400.0)).abs() < 1e-6, "{:?} against {:?}", inside, wrapped);
        }
    }

    /// In a circle, a text either does not fit or has every glyph between the ends of its line: the ink of a glyph at
    /// the height of the middle of a capital, and the point the glyph turns about, are inside the circle (the region is
    /// convex, so what lies between two points of it is inside).
    #[test]
    fn every_glyph_is_inside_the_shape(
        content in words(),
        radius in 2500.0..9000.0_f64,
        size in prop_oneof![Just(2000.0), Just(1000.0), Just(500.0)],
        align in proptest::sample::select(vec![Align::Left, Align::Center, Align::Right]),
    ) {
        let fail = |e: Error| TestCaseError::fail(e.to_string());
        let font = fraunces().map_err(fail)?;
        let circle = Path::circle((radius, radius), radius).map_err(fail)?;
        let drawing = text(content, Text::new(&font).size(size).align(align).inside(&circle));
        match drawing.shapes(&Values::new()) {
            Ok(shapes) => {
                for shape in shapes {
                    let b = shape.path().bounds().map_err(fail)?;
                    let middle = shape.attributes().origin().ok_or_else(|| TestCaseError::fail("no origin"))?;
                    for point in [(b.x, middle.y), (b.x + b.width, middle.y), (middle.x, middle.y)] {
                        prop_assert!(circle.contains(point).map_err(fail)?, "{point:?} of {:?} is outside", shape.attributes().char());
                    }
                }
            }
            Err(Error::Parameter { message, .. }) => prop_assert!(message.contains("does not fit"), "{message}"),
            Err(other) => return Err(TestCaseError::fail(other.to_string())),
        }
    }
}

// the document --------------------------------------------------------------------------------------------------------

#[test]
fn a_shape_is_written_as_path_data_and_read_back() -> Outcome {
    let font = fraunces()?.named("anybody")?;
    let inputs = Inputs::new().set("anybody", crate::common::fonts::FRAUNCES);
    let drawing = text("H H H", at_em(&font).inside(&svg("M500 300H4261V20300H500Z")?));
    let json = drawing.to_json()?;
    assert!(json.contains(r#""inside":"M500 300"#), "{json}");
    let loaded = Drawing::from_json(&json, &inputs)?;
    assert_eq!(loaded.to_json()?, json);
    let (a, b) = (loaded.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    assert_eq!(a.len(), 3);
    assert!(a.iter().zip(&b).all(|(a, b)| a.path() == b.path()), "{json}");
    let plain = text("H H H", at_em(&font)).to_json()?;
    assert!(!plain.contains("inside"), "{plain}");
    Ok(())
}

#[test]
fn a_document_that_gives_inside_with_what_it_excludes_is_refused_when_loaded() -> Outcome {
    let inputs = Inputs::new().set("anybody", crate::common::fonts::FRAUNCES);
    let document = |arguments: &str| {
        format!(
            r#"{{"version":1,"inputs":{{"anybody":{{"kind":"font"}}}},"root":[{{"text":{{"content":"H H","font":"anybody",{arguments}}}}}]}}"#
        )
    };
    for (rule, arguments, named) in [
        ("a width", r#""inside":"M0 0H9V9H0Z","width":100"#, "inside"),
        ("an anchor", r#""inside":"M0 0H9V9H0Z","anchor":[1,2]"#, "inside"),
        ("a leading of zero", r#""inside":"M0 0H9V9H0Z","leading":0"#, "leading"),
        ("path data that is not path data", r#""inside":"round""#, "inside"),
    ] {
        let message = match Drawing::from_json(&document(arguments), &inputs) {
            Err(Error::Parameter { parameter, message }) => format!("{parameter}: {message}"),
            Err(Error::Format { pointer, message }) => format!("{pointer}: {message}"),
            Err(other) => return Err(format!("{rule}: {other}").into()),
            Ok(_) => return Err(format!("{rule}: loaded").into()),
        };
        assert!(message.contains(named), "{rule}: {message}");
    }
    Ok(())
}
