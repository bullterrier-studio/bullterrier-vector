//! `Drawing::slice` through the public API: each shape keeps the piece of its outline between two positions, and the
//! positions may differ per shape, so a text draws on letter by letter. Expected values are written by hand (a line of
//! known length, the documented stagger window); nothing here reuses the library's own slicing.

use crate::common::fonts::{FRAUNCES, fraunces};
use crate::common::{Outcome, format_refusal, parameter_refusal};
use bullterrier_vector::{
    Color, Drawing, Error, Font, Inputs, Param, Path, Rect, Shape, Stagger, Stroke, Text, Values, by_index, text,
};

/// A horizontal line 80 long from x = 10, at height `y`.
fn line(y: f64) -> Result<Path, Error> {
    Path::line((10.0, y), (90.0, y))
}

fn drawn(drawing: &Drawing) -> Result<Vec<Shape>, Error> {
    drawing.shapes(&Values::new())
}

fn bounds(shape: &Shape) -> Result<Rect, Error> {
    shape.path().bounds()
}

fn near(found: f64, expected: f64, what: &str) {
    assert!((found - expected).abs() < 1e-6, "{what}: {found} is not {expected}");
}

// The piece between two positions ------------------------------------------------------------------------------------

#[test]
fn slice_keeps_the_piece_of_a_line_between_the_two_positions() -> Outcome {
    // (start, end, left edge, right edge) of the piece of a line from x = 10 to x = 90
    let rows = [
        (0.0, 0.5, 10.0, 50.0),
        (0.25, 0.75, 30.0, 70.0),
        (0.5, 1.0, 50.0, 90.0),
        (0.0, 1.0, 10.0, 90.0),
        (0.125, 0.125, 20.0, 20.0),
    ];
    for (start, end, left, right) in rows {
        let shapes = drawn(&Drawing::from(line(0.0)?).slice(start, end))?;
        let found = bounds(&shapes[0])?;
        near(found.x, left, &format!("slice({start}, {end}) left"));
        near(found.x + found.width, right, &format!("slice({start}, {end}) right"));
        near(found.height, 0.0, &format!("slice({start}, {end}) height"));
    }
    Ok(())
}

#[test]
fn slice_of_the_whole_of_a_line_is_the_line() -> Outcome {
    let whole = drawn(&Drawing::from(line(0.0)?))?;
    let sliced = drawn(&Drawing::from(line(0.0)?).slice(0.0, 1.0))?;
    assert_eq!(sliced[0].path().to_svg_path(), whole[0].path().to_svg_path());
    Ok(())
}

#[test]
fn slice_keeps_the_style_and_attributes_of_the_shape() -> Outcome {
    let drawing = Drawing::from(line(0.0)?).stroke(Stroke::new(Color::hex(0x2a_5b_ff)));
    let (before, after) = (drawn(&drawing)?, drawn(&drawing.slice(0.0, 0.5))?);
    assert_eq!(after[0].style(), before[0].style());
    assert_eq!(after[0].attributes(), before[0].attributes());
    Ok(())
}

#[test]
fn near_chooses_where_a_closed_outline_is_measured_from() -> Outcome {
    let square = Path::from_svg_path("M0 0 L10 0 L10 10 L0 10 Z")?;
    let quarter = |drawing: Drawing| -> Result<Rect, Error> { bounds(&drawn(&drawing)?[0]) };
    // from the top, the first quarter lies along the top edge or the left edge, starting at a top corner
    let from_top = quarter(Drawing::from(square.clone()).slice(0.0, 0.25))?;
    assert!(from_top.y.abs() < 1e-9, "{from_top:?}");
    // from the corner nearest (10, 10): the first quarter is a side that ends at that corner
    let from_corner = quarter(Drawing::from(square).slice_near(0.0, 0.25, (10.0, 10.0)))?;
    assert!(
        from_corner.x + from_corner.width > 10.0 - 1e-9 && from_corner.y + from_corner.height > 10.0 - 1e-9,
        "{from_corner:?}"
    );
    near(
        from_corner.width.max(from_corner.height),
        10.0,
        "a quarter of the outline is one side",
    );
    near(from_corner.width.min(from_corner.height), 0.0, "and straight");
    Ok(())
}

#[test]
fn a_shape_with_nothing_in_it_stays_and_the_list_keeps_its_numbering() -> Outcome {
    let drawing = Drawing::new([Drawing::from(Path::new()), Drawing::from(line(0.0)?)]).slice(0.0, 0.5);
    let shapes = drawn(&drawing)?;
    assert_eq!(shapes.len(), 2);
    assert_eq!(shapes[0].path().to_svg_path(), "");
    near(bounds(&shapes[1])?.width, 40.0, "the other shape");
    Ok(())
}

// Per shape ---------------------------------------------------------------------------------------------------------

#[test]
fn each_shape_takes_its_own_end_from_a_per_item_number() -> Outcome {
    let lines = Drawing::new([line(0.0)?, line(10.0)?, line(20.0)?]);
    let widths = |drawing: &Drawing| -> Result<Vec<f64>, Error> {
        drawn(drawing)?.iter().map(|shape| Ok(bounds(shape)?.width)).collect()
    };
    let found = widths(&lines.slice(0.0, by_index(0.0, 1.0)))?;
    for (found, expected) in found.iter().zip([0.0, 40.0, 80.0]) {
        near(*found, expected, "by_index(0, 1)");
    }
    // and a start per shape: 0, 1/2, 1 of an end at 1 leaves 80, 40, 0
    let found = widths(&lines.slice(by_index(0.0, 1.0), 1.0))?;
    for (found, expected) in found.iter().zip([80.0, 40.0, 0.0]) {
        near(*found, expected, "by_index(0, 1) as start");
    }
    Ok(())
}

/// The fraction of its outline a letter at `order` has drawn when the progress is `progress`: the letter runs while
/// the progress goes from `order * spread` to `order * spread + 1 - spread` (api.md, `stagger`).
fn drawn_fraction(order: f64, progress: f64, spread: f64) -> f64 {
    ((progress - order * spread) / (1.0 - spread)).clamp(0.0, 1.0)
}

/// The `order` of the five letters of a text: 0, 1/4, ..., 1.
const ORDERS: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

#[test]
fn a_text_draws_on_letter_by_letter_with_a_staggered_end() -> Outcome {
    let progress = Param::new("progress", 0.0, 0.0, 1.0)?;
    let word = text("HIFIT", &fraunces()?);
    let draw_on = word.slice(0.0, Stagger::new(&progress).spread(0.6));
    let whole = drawn(&word)?;
    let count = whole.len();
    assert_eq!(count, 5);
    for at in [0.0, 0.2, 0.5, 0.75, 1.0] {
        let shapes = draw_on.shapes(&Values::new().set("progress", at))?;
        assert_eq!(shapes.len(), count);
        let mut lengths = Vec::new();
        for (index, ((shape, full), order)) in shapes.iter().zip(&whole).zip(ORDERS).enumerate() {
            let expected = drawn_fraction(order, at, 0.6) * full.path().length();
            // curve lengths are measured numerically: agree to a thousandth
            assert!(
                (shape.path().length() - expected).abs() <= 1e-3 * full.path().length(),
                "letter {index} at progress {at}: {} is not {expected}",
                shape.path().length()
            );
            lengths.push(shape.path().length() / full.path().length());
        }
        // the letters before are at least as far along as the letters after
        assert!(
            lengths.windows(2).all(|pair| pair[0] >= pair[1] - 1e-3),
            "{lengths:?} at {at}"
        );
    }
    // nothing at 0, and every letter whole at 1 (its outline is closed again, so the same shape and size)
    let finished = draw_on.shapes(&Values::new().set("progress", 1.0))?;
    for (letter, original) in finished.iter().zip(&whole) {
        assert_eq!(bounds(letter)?, bounds(original)?);
        assert!((letter.path().length() - original.path().length()).abs() <= 1e-3 * original.path().length());
    }
    Ok(())
}

// Refusals ----------------------------------------------------------------------------------------------------------

/// `(start, end, near, the argument a refusal names, accepted)`.
type Refusal = (f64, f64, Option<(f64, f64)>, &'static str, bool);

#[test]
fn slice_refuses_positions_outside_0_to_1_start_after_end_and_a_near_that_is_not_a_number() -> Outcome {
    // (start, end, near, argument named, accepted)
    let rows: [Refusal; 13] = [
        (0.0, 1.0, None, "", true),
        (0.0, 0.0, None, "", true),
        (1.0, 1.0, None, "", true),
        (0.5, 0.5, None, "", true),
        (0.0, 1.0, Some((3.0, 4.0)), "", true),
        (1.1, 1.0, None, "start", false),
        (-0.1, 0.5, None, "start", false),
        (0.0, 1.1, None, "end", false),
        (0.0, -0.1, None, "end", false),
        (0.8, 0.2, None, "start", false),
        (f64::NAN, 1.0, None, "start", false),
        (0.0, f64::NAN, None, "end", false),
        (0.0, 1.0, Some((f64::INFINITY, 0.0)), "near", false),
    ];
    for (start, end, point, argument, accepted) in rows {
        let drawing = match point {
            Some(point) => Drawing::from(line(0.0)?).slice_near(start, end, point),
            None => Drawing::from(line(0.0)?).slice(start, end),
        };
        let case = format!("slice({start}, {end}, near {point:?})");
        if accepted {
            assert_eq!(drawing.error(), None, "{case}");
            drawn(&drawing)?;
        } else {
            let (name, message) = parameter_refusal(drawn(&drawing))?;
            assert!(
                name.contains("slice") && (name.contains(argument) || message.contains(argument)),
                "{case}: {name}: {message}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_per_item_number_past_the_range_is_refused_when_drawn_naming_the_shape() -> Outcome {
    // the third of three shapes gets 2
    let drawing = Drawing::new([line(0.0)?, line(10.0)?, line(20.0)?]).slice(0.0, by_index(0.0, 2.0));
    assert_eq!(drawing.error(), None);
    let (name, message) = parameter_refusal(drawn(&drawing))?;
    assert!(
        name.contains("slice") && name.contains("end") && message.contains('2'),
        "{name}: {message}"
    );
    Ok(())
}

// Documents, determinism, the cache ------------------------------------------------------------------------------------

#[test]
fn slice_round_trips_through_json_and_draws_the_same() -> Outcome {
    let progress = Param::new("progress", 0.3, 0.0, 1.0)?;
    let square = Path::from_svg_path("M0 0 L10 0 L10 10 L0 10 Z")?;
    let named = Font::new(FRAUNCES)?.named("fraunces")?;
    let fonts = Inputs::new().set("fraunces", FRAUNCES);
    let drawings = [
        (Drawing::from(line(0.0)?).slice(0.25, 0.75), Inputs::new()),
        (
            Drawing::new([line(0.0)?, line(10.0)?]).slice(0.0, by_index(0.5, 1.0)),
            Inputs::new(),
        ),
        (
            text("HIFIT", Text::new(&named)).slice(0.0, Stagger::new(&progress).spread(0.6)),
            fonts,
        ),
        (Drawing::from(square).slice_near(0.1, 0.9, (10.0, 10.0)), Inputs::new()),
    ];
    for (drawing, inputs) in drawings {
        let document = drawing.to_json()?;
        assert!(document.contains(r#""slice""#), "{document}");
        let again = Drawing::from_json(&document, &inputs)?;
        assert_eq!(again.to_json()?, document);
        assert_eq!(drawn(&again)?, drawn(&drawing)?);
    }
    Ok(())
}

#[test]
fn a_document_with_a_slice_past_the_range_is_refused_naming_the_argument() -> Outcome {
    let good = Drawing::from(line(0.0)?).slice(0.25, 0.75).to_json()?;
    assert!(Drawing::from_json(&good, &Inputs::new()).is_ok());
    let (pointer, message) = format_refusal(Drawing::from_json(&good.replace("0.75", "1.5"), &Inputs::new()))?;
    assert!(
        pointer.contains("end") || message.contains("end"),
        "{pointer}: {message}"
    );
    Ok(())
}

#[test]
fn slice_draws_the_same_bytes_every_time() -> Outcome {
    let drawing = text("HIFIT", &fraunces()?).slice(0.0, Stagger::new(0.5).spread(0.6));
    assert_eq!(drawn(&drawing)?, drawn(&drawing)?);
    assert_eq!(drawing.to_json()?, drawing.to_json()?);
    Ok(())
}

#[test]
fn equal_slices_built_apart_share_a_step_and_different_ones_do_not() -> Outcome {
    let slice = |end: f64| -> Result<Drawing, Error> { Ok(Drawing::from(line(0.0)?).slice(0.0, end)) };
    let same = Drawing::new([slice(0.5)?, slice(0.5)?]);
    same.shapes(&Values::new())?;
    // the path, the slice and the stack, once each
    assert_eq!((same.stats().computed, same.stats().reused), (3, 1));
    let different = Drawing::new([slice(0.5)?, slice(0.6)?]);
    let shapes = different.shapes(&Values::new())?;
    // the path is shared; each slice is its own step
    assert_eq!((different.stats().computed, different.stats().reused), (4, 1));
    near(bounds(&shapes[0])?.width, 40.0, "end 0.5");
    near(bounds(&shapes[1])?.width, 48.0, "end 0.6");
    // and the start counts too
    let other_start = Drawing::new([slice(0.5)?, Drawing::from(line(0.0)?).slice(0.1, 0.5)]);
    other_start.shapes(&Values::new())?;
    assert_eq!(other_start.stats().computed, 4);
    Ok(())
}
