//! `split` and `merge` through the public API: a shape becomes its parts and shapes with an equal attribute become
//! one. Expected values are written by hand or read back from the paths drawn (a sub-path is an `M` of the path data,
//! bounds are the union of the members' bounds); nothing here reuses the library's own splitting.

use crate::common::fonts::fraunces;
use crate::common::{Outcome, format_refusal, parameter_refusal};
use bullterrier_vector::{Color, Drawing, Error, Inputs, Path, Point, Rect, Shape, Values, text};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

fn drawn(drawing: &Drawing) -> Result<Vec<Shape>, Error> {
    drawing.shapes(&Values::new())
}

/// How many sub-paths a shape's outline has: the `M`s of its path data.
fn subpaths(shape: &Shape) -> usize {
    shape.path().to_svg_path().matches('M').count()
}

fn blue() -> Color {
    Color::hex(0x2a_5b_ff)
}

/// The middle of a shape's bounds.
fn middle(shape: &Shape) -> Result<Point, Error> {
    let bounds = shape.path().bounds()?;
    Ok(Point::new(
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    ))
}

/// The smallest rectangle round all of `shapes`, from their own bounds.
fn union(shapes: &[&Shape]) -> Result<Rect, Error> {
    let boxes = shapes
        .iter()
        .map(|shape| shape.path().bounds())
        .collect::<Result<Vec<_>, _>>()?;
    let left = boxes.iter().map(|b| b.x).fold(f64::INFINITY, f64::min);
    let top = boxes.iter().map(|b| b.y).fold(f64::INFINITY, f64::min);
    let right = boxes.iter().map(|b| b.x + b.width).fold(f64::NEG_INFINITY, f64::max);
    let bottom = boxes.iter().map(|b| b.y + b.height).fold(f64::NEG_INFINITY, f64::max);
    Ok(Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

fn assert_same_rect(found: Rect, expected: Rect, what: &str) {
    let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
    assert!(
        close(found.x, expected.x)
            && close(found.y, expected.y)
            && close(found.width, expected.width)
            && close(found.height, expected.height),
        "{what}: {found:?} is not {expected:?}"
    );
}

/// "Hi Tel" in the test font, filled: the glyphs 0 to 4 are `H`, `i`, `T`, `e` and `l`, in the words "Hi" and "Tel".
fn words() -> Result<Drawing, Error> {
    Ok(text("Hi Tel", &fraunces()?).fill(blue()))
}

// split --------------------------------------------------------------------------------------------------------------

#[test]
fn split_gives_the_stem_and_the_dot_of_an_i_as_two_parts_with_the_attributes_of_the_glyph() -> Outcome {
    let whole = drawn(&text("i", &fraunces()?).fill(blue()))?;
    assert_eq!((whole.len(), subpaths(&whole[0])), (1, 2));
    let parts = drawn(&text("i", &fraunces()?).fill(blue()).split())?;
    assert_eq!(parts.len(), 2);
    for (number, part) in parts.iter().enumerate() {
        let number = u32::try_from(number)?;
        assert_eq!(subpaths(part), 1, "part {number}");
        assert_eq!(part.attributes().part(), Some(number));
        assert_eq!(part.index(), number as usize);
        // what the glyph had, bar its origin
        let (own, source) = (part.attributes(), whole[0].attributes());
        assert_eq!(
            (own.glyph(), own.char(), own.word(), own.line(), own.order()),
            (
                source.glyph(),
                source.char(),
                source.word(),
                source.line(),
                source.order()
            )
        );
        assert_eq!(part.style(), whole[0].style(), "part {number}: style");
        // its own middle, not the glyph's
        assert_eq!(own.origin(), Some(middle(part)?), "part {number}: origin");
    }
    assert_ne!(parts[0].attributes().origin(), whole[0].attributes().origin());
    // the two parts are the stem and the dot: one above the other, not overlapping, and together the glyph
    let (a, b) = (parts[0].path().bounds()?, parts[1].path().bounds()?);
    assert!(a.y + a.height <= b.y || b.y + b.height <= a.y, "{a:?} {b:?}");
    assert_same_rect(
        union(&[&parts[0], &parts[1]])?,
        whole[0].path().bounds()?,
        "the parts together",
    );
    Ok(())
}

#[test]
fn split_keeps_an_outline_with_its_hole_as_one_part() -> Outcome {
    let parts = drawn(&text("o", &fraunces()?).split())?;
    assert_eq!(parts.len(), 1);
    assert_eq!((subpaths(&parts[0]), parts[0].attributes().part()), (2, Some(0)));
    Ok(())
}

#[test]
fn split_renumbers_the_list_and_keeps_the_order_each_shape_had() -> Outcome {
    // two glyphs of two parts each
    let parts = drawn(&text("ii", &fraunces()?).split())?;
    let seen: Vec<_> = parts
        .iter()
        .map(|p| {
            (
                p.index(),
                p.attributes().glyph(),
                p.attributes().part(),
                p.attributes().order(),
            )
        })
        .collect();
    assert_eq!(
        seen,
        vec![
            (0, Some(0), Some(0), Some(0.0)),
            (1, Some(0), Some(1), Some(0.0)),
            (2, Some(1), Some(0), Some(1.0)),
            (3, Some(1), Some(1), Some(1.0)),
        ]
    );
    // a step after the split sees the new list: the third shape is the first part of the second glyph
    let third = drawn(
        &text("ii", &fraunces()?)
            .split()
            .only(bullterrier_vector::Select::new().attribute("index", 2)),
    )?;
    assert_eq!(third[0].attributes().glyph(), Some(1));
    Ok(())
}

#[test]
fn split_keeps_a_shape_with_nothing_in_it_in_the_list() -> Outcome {
    let two = Drawing::new([
        Drawing::from(Path::new()),
        Drawing::from(Path::rect(0.0, 0.0, 4.0, 4.0, 0.0)?),
    ]);
    let shapes = drawn(&two.split())?;
    assert_eq!(shapes.len(), 2);
    assert_eq!(subpaths(&shapes[0]), 0);
    assert_eq!((shapes[1].index(), shapes[1].attributes().part()), (1, Some(0)));
    Ok(())
}

#[test]
fn split_of_a_path_without_attributes_adds_part_and_the_middle_as_origin() -> Outcome {
    let two_squares = Path::from_svg_path("M0 0 L4 0 L4 4 Z M10 2 L16 2 L16 8 L10 8 Z")?;
    let shapes = drawn(&Drawing::from(two_squares).split())?;
    let seen: Vec<_> = shapes
        .iter()
        .map(|s| (s.attributes().part(), s.attributes().origin()))
        .collect();
    assert_eq!(
        seen,
        vec![
            (Some(0), Some(Point::new(2.0, 2.0))),
            (Some(1), Some(Point::new(13.0, 5.0))),
        ]
    );
    Ok(())
}

// merge --------------------------------------------------------------------------------------------------------------

/// `source` merged by `by`, or all of it.
fn merged(source: &Drawing, by: Option<&str>) -> Drawing {
    by.map_or_else(|| source.merge(), |by| source.merge_by(by))
}

/// `(by, the glyphs each shape it leaves is made of)`.
type MergeRow = (Option<&'static str>, &'static [&'static [usize]]);

#[test]
fn merge_gives_one_shape_per_group_in_order_of_first_appearance_with_the_first_ones_attributes() -> Outcome {
    let rows: [MergeRow; 6] = [
        (None, &[&[0, 1, 2, 3, 4]]),
        (Some("line"), &[&[0, 1, 2, 3, 4]]),
        // "Hi" and "Tel"
        (Some("word"), &[&[0, 1], &[2, 3, 4]]),
        // every glyph its own: nothing to join
        (Some("glyph"), &[&[0], &[1], &[2], &[3], &[4]]),
        (Some("index"), &[&[0], &[1], &[2], &[3], &[4]]),
        // H, i, T, e and l are five different characters
        (Some("char"), &[&[0], &[1], &[2], &[3], &[4]]),
    ];
    let source = words()?;
    let before = drawn(&source)?;
    for (by, expected) in rows {
        let shapes = drawn(&merged(&source, by))?;
        // the first glyph of each, and the sub-paths of all of them
        let seen: Vec<_> = shapes.iter().map(|s| (s.attributes().glyph(), subpaths(s))).collect();
        let wanted: Vec<_> = expected
            .iter()
            .map(|members| {
                (
                    u32::try_from(members[0]).ok(),
                    members.iter().map(|&m| subpaths(&before[m])).sum::<usize>(),
                )
            })
            .collect();
        assert_eq!(seen, wanted, "by {by:?}");
        for (shape, members) in shapes.iter().zip(expected) {
            assert_eq!(
                *shape.attributes(),
                *before[members[0]].attributes(),
                "by {by:?}: attributes of the first"
            );
            assert_eq!(
                shape.style(),
                before[members[0]].style(),
                "by {by:?}: style of the first"
            );
        }
        let indexes: Vec<_> = shapes.iter().map(Shape::index).collect();
        assert_eq!(indexes, (0..shapes.len()).collect::<Vec<_>>(), "by {by:?}: renumbered");
    }
    Ok(())
}

#[test]
fn merge_takes_the_attributes_of_the_first_member_and_sits_where_the_first_member_was() -> Outcome {
    // i H i: the two i are one shape, at the place of the first, and the H follows it
    let source = text("iHi", &fraunces()?);
    let before = drawn(&source)?;
    let after = drawn(&source.merge_by("char"))?;
    let seen: Vec<_> = after
        .iter()
        .map(|s| (s.attributes().char(), s.attributes().glyph(), subpaths(s)))
        .collect();
    let (i, h) = (subpaths(&before[0]), subpaths(&before[1]));
    assert_eq!(seen, vec![(Some('i'), Some(0), 2 * i), (Some('H'), Some(1), h)]);
    assert_eq!(after[0].attributes(), before[0].attributes());
    assert_eq!(after[0].attributes().origin(), before[0].attributes().origin());
    Ok(())
}

#[test]
fn merge_covers_what_its_members_covered() -> Outcome {
    let source = words()?;
    let before = drawn(&source)?;
    let after = drawn(&source.merge_by("word"))?;
    // "Hi" is the glyphs 0 and 1, "Tel" the glyphs 2 to 4
    let members: [&[usize]; 2] = [&[0, 1], &[2, 3, 4]];
    for (merged, members) in after.iter().zip(members) {
        let of: Vec<&Shape> = members.iter().map(|&m| &before[m]).collect();
        assert_same_rect(merged.path().bounds()?, union(&of)?, "bounds");
    }
    Ok(())
}

#[test]
fn merge_refuses_a_name_that_is_no_attribute_and_an_attribute_no_shape_carries() -> Outcome {
    let source = words()?;
    // (by, whether the call is refused, whether the draw is, what the message names)
    let rows = [
        ("nope", true, "nope"),
        ("", true, "\"\""),
        ("Glyph", true, "Glyph"),
        ("part", false, "part"),
        ("copy", false, "copy"),
        ("level", false, "level"),
    ];
    for (by, at_call, named) in rows {
        let drawing = source.merge_by(by);
        let (name, message) = if at_call {
            parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error at the call")?))?
        } else {
            assert_eq!(drawing.error(), None, "{by}: the call is accepted");
            parameter_refusal(drawn(&drawing))?
        };
        assert!(
            name == "step 2 (merge).by" && message.contains(named),
            "{by}: {name}: {message}"
        );
    }
    // after a split every shape carries `part`: accepted; an empty list asks nothing
    assert_eq!(drawn(&source.split().merge_by("part"))?.len(), 2);
    assert_eq!(drawn(&Drawing::new(Vec::<Drawing>::new()).merge_by("part"))?.len(), 0);
    // `origin` is carried by every glyph: all different, so nothing joins
    assert_eq!(drawn(&source.merge_by("origin"))?.len(), 5);
    Ok(())
}

#[test]
fn shapes_that_lack_the_attribute_while_others_carry_it_are_one_group() -> Outcome {
    // two squares, which carry no `part`, and the two parts of an i, which do
    let square = |left| Path::rect(left, 0.0, 4.0, 4.0, 0.0);
    let mixed = Drawing::new([
        Drawing::from(square(0.0)?),
        Drawing::from(square(9.0)?),
        text("i", &fraunces()?).split(),
    ]);
    let shapes = drawn(&mixed.merge_by("part"))?;
    let seen: Vec<_> = shapes.iter().map(|s| (s.attributes().part(), subpaths(s))).collect();
    assert_eq!(seen, vec![(None, 2), (Some(0), 1), (Some(1), 1)]);
    Ok(())
}

#[test]
fn merging_the_parts_of_each_glyph_gives_the_glyphs_back() -> Outcome {
    let source = words()?;
    let whole = drawn(&source)?;
    let again = drawn(&source.split().merge_by("glyph"))?;
    assert_eq!(again.len(), whole.len());
    for (again, whole) in again.iter().zip(&whole) {
        assert_eq!(subpaths(again), subpaths(whole));
        assert_same_rect(again.path().bounds()?, whole.path().bounds()?, "bounds");
        assert_eq!(again.attributes().glyph(), whole.attributes().glyph());
        assert_eq!(again.attributes().order(), whole.attributes().order());
    }
    Ok(())
}

proptest! {
    /// Squares side by side, as sub-paths of one path: split gives one shape each, merge joins them again.
    #[test]
    fn splitting_separate_squares_and_merging_them_again_gives_the_path_back(count in 1_u32..8, side in 1.0_f64..50.0) {
        let squares: Vec<String> = (0..count)
            .map(|n| {
                let left = 100.0 * f64::from(n);
                format!("M{left} 0 L{} 0 L{} {side} L{left} {side} Z", left + side, left + side)
            })
            .collect();
        let data = squares.join(" ");
        let path = Path::from_svg_path(&data).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let whole = Drawing::from(path.clone());
        let split = drawn(&whole.split()).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(split.len(), count as usize);
        let one = drawn(&whole.split().merge()).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(one.len(), 1);
        prop_assert_eq!(subpaths(&one[0]), count as usize);
        prop_assert_eq!(one[0].path().bounds().map_err(|e| TestCaseError::fail(e.to_string()))?, path.bounds().map_err(|e| TestCaseError::fail(e.to_string()))?);
    }
}

#[test]
fn split_and_merge_draw_the_same_shapes_every_time() -> Outcome {
    let build = || -> Result<Drawing, Error> { Ok(words()?.split().merge_by("word").split()) };
    assert_eq!(drawn(&build()?)?, drawn(&build()?)?);
    Ok(())
}

// JSON ---------------------------------------------------------------------------------------------------------------

#[test]
fn split_and_merge_are_written_with_their_arguments_and_read_back_to_the_same_shapes() -> Outcome {
    let two = Path::from_svg_path("M0 0 L4 0 L4 4 Z M10 2 L16 2 L16 8 L10 8 Z")?;
    // (the drawing, the call it writes)
    let rows = [
        (Drawing::from(two.clone()).split(), r#"{"split":{}}"#),
        (Drawing::from(two.clone()).merge(), r#"{"merge":{}}"#),
        (
            Drawing::from(two.clone()).split().merge_by("part"),
            r#"{"merge":{"by":"part"}}"#,
        ),
    ];
    for (drawing, call) in rows {
        let text = drawing.to_json()?;
        assert!(text.contains(call), "{call} not in {text}");
        let again = Drawing::from_json(&text, &Inputs::new())?;
        assert_eq!(again.to_json()?, text);
        assert_eq!(drawn(&again)?, drawn(&drawing)?, "{call}");
    }
    Ok(())
}

#[test]
fn a_document_that_merges_by_a_name_that_is_no_attribute_is_refused() -> Outcome {
    let two = Path::from_svg_path("M0 0 L4 0 L4 4 Z M10 2 L16 2 L16 8 L10 8 Z")?;
    let good = Drawing::from(two).split().merge_by("part").to_json()?;
    assert!(Drawing::from_json(&good, &Inputs::new()).is_ok());
    let (_, message) = format_refusal(Drawing::from_json(
        &good.replace(r#""by":"part""#, r#""by":"nope""#),
        &Inputs::new(),
    ))?;
    assert!(message.contains("nope"), "{message}");
    Ok(())
}
