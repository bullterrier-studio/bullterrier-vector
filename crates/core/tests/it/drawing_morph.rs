//! `Drawing::morph` through the public API. The shapes pair by index, a shape without a partner shrinks to its center
//! or grows from the target's, each shape reads its own `fraction`. Expected values are worked out by hand on squares
//! (a unit square carried to a 2 x 2 one is 1.5 wide at 0.5); `Path::morph` is the oracle in one test only, which says so.

use crate::common::fonts::fraunces;
use crate::common::{Outcome, format_refusal, net_area, parameter_refusal, rings};
use bullterrier_vector::{
    Color, Drawing, Error, Inputs, Param, Path, Shape, Stagger, Values, by_index, by_order, drawing_schema, text, verbs,
};

/// Slack for the arithmetic of a lerp, which is exact on these numbers up to the last bits.
const ARITHMETIC: f64 = 1e-9;

fn rect(x: f64, y: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(x, y, width, height, 0.0)
}

fn drawn(drawing: &Drawing) -> Result<Vec<Shape>, Error> {
    drawing.shapes(&Values::new())
}

/// The bounds `(x, y, width, height)` of a shape's path.
fn bounds(shape: &Shape) -> Result<(f64, f64, f64, f64), Error> {
    let found = shape.path().bounds()?;
    Ok((found.x, found.y, found.width, found.height))
}

fn assert_bounds(shape: &Shape, expected: (f64, f64, f64, f64), what: &str) -> Outcome {
    let found = bounds(shape)?;
    let close = |a: f64, b: f64| (a - b).abs() <= ARITHMETIC;
    assert!(
        close(found.0, expected.0)
            && close(found.1, expected.1)
            && close(found.2, expected.2)
            && close(found.3, expected.3),
        "{what}: bounds {found:?}, expected {expected:?}"
    );
    Ok(())
}

fn red() -> Color {
    Color::hex(0xff_00_00)
}

fn blue() -> Color {
    Color::hex(0x00_00_ff)
}

fn green() -> Color {
    Color::hex(0x00_ff_00)
}

/// Unit squares at x = 0, 10, 20, filled red, blue, red.
fn small(count: u8) -> Result<Drawing, Error> {
    let fills = [red(), blue(), red()];
    Ok(Drawing::new(
        (0..count)
            .map(|at| Ok(rect(10.0 * f64::from(at), 0.0, 1.0, 1.0)?.fill(fills[usize::from(at) % 3])))
            .collect::<Result<Vec<_>, Error>>()?,
    ))
}

/// 2 x 2 squares at x = 0, 10, 20, all filled green.
fn big(count: u8) -> Result<Drawing, Error> {
    Ok(Drawing::new(
        (0..count)
            .map(|at| Ok(rect(10.0 * f64::from(at), 0.0, 2.0, 2.0)?.fill(green())))
            .collect::<Result<Vec<_>, Error>>()?,
    ))
}

// Pairing by index ---------------------------------------------------------------------------------------------------

#[test]
fn equal_counts_pair_by_index_and_keep_the_sources_style_and_attributes() -> Outcome {
    let (source, target) = (small(2)?, big(2)?);
    let (before, after) = (drawn(&source)?, drawn(&source.morph(&target, 0.5))?);
    assert_eq!(after.len(), 2);
    // the unit square at the origin to the 2 x 2 one: its far corner goes from 1 to 2, half way is 1.5
    assert_bounds(&after[0], (0.0, 0.0, 1.5, 1.5), "shape 0")?;
    assert_bounds(&after[1], (10.0, 0.0, 1.5, 1.5), "shape 1")?;
    for (kept, own) in after.iter().zip(&before) {
        assert_eq!(kept.style(), own.style());
        assert_eq!(kept.attributes(), own.attributes());
    }
    assert_ne!(after[0].style(), drawn(&target)?[0].style(), "not the target's style");
    Ok(())
}

#[test]
fn the_step_is_the_path_verb_on_each_shape() -> Outcome {
    // the one test that takes Path::morph as the oracle: a ring to a pentagon, which is no hand case
    let ring = Path::from_svg_path("M0 0 L100 0 L100 100 L0 100 Z M30 30 L30 70 L70 70 L70 30 Z")?;
    let pentagon = Path::from_svg_path("M150 10 L200 0 L210 60 L170 90 L140 50 Z")?;
    let shapes = drawn(&Drawing::from(ring.clone()).morph(pentagon.clone(), 0.3))?;
    assert_eq!(shapes.len(), 1);
    assert_eq!(shapes[0].path(), &ring.morph(&pentagon, 0.3)?);
    Ok(())
}

#[test]
fn the_target_is_not_united_its_shapes_pair_one_by_one() -> Outcome {
    // two overlapping squares as the target's two shapes: a boolean would unite them, a morph keeps them apart
    let target = Drawing::new([rect(0.0, 0.0, 2.0, 2.0)?, rect(1.0, 0.0, 2.0, 2.0)?]);
    let after = drawn(&small(2)?.morph(target, 1.0))?;
    assert_bounds(&after[0], (0.0, 0.0, 2.0, 2.0), "shape 0")?;
    assert_bounds(&after[1], (1.0, 0.0, 2.0, 2.0), "shape 1")?;
    Ok(())
}

// A shape without a partner -------------------------------------------------------------------------------------------

#[test]
fn a_source_shape_with_no_partner_shrinks_to_its_center_and_keeps_its_style() -> Outcome {
    let source = small(2)?;
    let (before, after) = (drawn(&source)?, drawn(&source.morph(big(1)?, 0.5))?);
    assert_eq!(after.len(), 2);
    assert_bounds(&after[0], (0.0, 0.0, 1.5, 1.5), "the paired shape")?;
    // the unit square at x = 10 is 0.5 x 0.5 half way to its center (10.5, 0.5), a quarter of its area
    assert_bounds(&after[1], (10.25, 0.25, 0.5, 0.5), "the lonely shape")?;
    assert_eq!(net_area(&rings(after[1].path())?), 0.25);
    assert_eq!(after[1].style(), before[1].style());
    assert_eq!(after[1].attributes(), before[1].attributes());
    // all the way it is a point, which fills nothing
    let gone = drawn(&source.morph(big(1)?, 1.0))?;
    assert_bounds(&gone[1], (10.5, 0.5, 0.0, 0.0), "at fraction 1")?;
    assert_eq!(net_area(&rings(gone[1].path())?), 0.0);
    Ok(())
}

#[test]
fn a_target_shape_with_no_partner_grows_in_place_with_the_targets_style() -> Outcome {
    let target = big(2)?;
    let after = drawn(&small(1)?.morph(&target, 0.5))?;
    assert_eq!(after.len(), 2);
    assert_bounds(&after[0], (0.0, 0.0, 1.5, 1.5), "the paired shape")?;
    // the 2 x 2 square at x = 10 grows from its center (11, 1): half way it is 1 x 1
    assert_bounds(&after[1], (10.5, 0.5, 1.0, 1.0), "the grown shape")?;
    let wanted = drawn(&target)?;
    assert_eq!(after[1].style(), wanted[1].style());
    assert_eq!(after[1].attributes(), wanted[1].attributes());
    assert_ne!(after[1].style(), after[0].style());
    // at 0 it is a point, at 1 the target's own square
    assert_bounds(
        &drawn(&small(1)?.morph(&target, 0.0))?[1],
        (11.0, 1.0, 0.0, 0.0),
        "at fraction 0",
    )?;
    assert_bounds(
        &drawn(&small(1)?.morph(&target, 1.0))?[1],
        (10.0, 0.0, 2.0, 2.0),
        "at fraction 1",
    )?;
    Ok(())
}

#[test]
fn an_empty_drawing_on_either_side_leaves_every_shape_without_a_partner() -> Outcome {
    let nothing = || Drawing::new(Vec::<Drawing>::new());
    let grown = drawn(&nothing().morph(big(2)?, 0.5))?;
    assert_eq!(grown.len(), 2);
    assert_bounds(&grown[1], (10.5, 0.5, 1.0, 1.0), "grown from nothing")?;
    assert_eq!(grown[1].style(), drawn(&big(2)?)?[1].style());
    let shrunk = drawn(&small(2)?.morph(nothing(), 0.5))?;
    assert_eq!(shrunk.len(), 2);
    assert_bounds(&shrunk[0], (0.25, 0.25, 0.5, 0.5), "shrunk to nothing")?;
    assert_eq!(shrunk[1].style(), drawn(&small(2)?)?[1].style());
    assert_eq!(drawn(&nothing().morph(nothing(), 0.5))?.len(), 0);
    Ok(())
}

// The ends and beyond -------------------------------------------------------------------------------------------------

#[test]
fn fraction_0_fills_what_the_drawing_fills_and_1_what_the_target_fills() -> Outcome {
    let (source, target) = (small(3)?, big(2)?);
    // areas: the unit squares 1 each; the 2 x 2 squares 4 each; the surplus source shape has nothing at 1
    let at = |fraction: f64| -> Result<Vec<f64>, Box<dyn std::error::Error>> {
        drawn(&source.morph(&target, fraction))?
            .iter()
            .map(|shape| Ok(net_area(&rings(shape.path())?)))
            .collect()
    };
    assert_eq!(at(0.0)?, vec![1.0, 1.0, 1.0]);
    assert_eq!(at(1.0)?, vec![4.0, 4.0, 0.0]);
    Ok(())
}

#[test]
fn a_fraction_beyond_0_and_1_carries_on_along_the_same_line() -> Outcome {
    let after = drawn(&small(1)?.morph(big(1)?, 1.5))?;
    // the far corner goes 1 + (2 - 1) * 1.5 = 2.5
    assert_bounds(&after[0], (0.0, 0.0, 2.5, 2.5), "overshoot")?;
    let before = drawn(&small(1)?.morph(big(1)?, -0.5))?;
    assert_bounds(&before[0], (0.0, 0.0, 0.5, 0.5), "undershoot")?;
    Ok(())
}

// Per-item fractions ------------------------------------------------------------------------------------------------

#[test]
fn each_shape_reads_its_own_fraction_by_index_over_the_whole_result() -> Outcome {
    // three shapes in the result (two of the drawing, a third that grows): by_index(0, 1) is 0, 0.5, 1
    let after = drawn(&small(2)?.morph(big(3)?, by_index(0.0, 1.0)))?;
    assert_eq!(after.len(), 3);
    assert_bounds(&after[0], (0.0, 0.0, 1.0, 1.0), "fraction 0")?;
    assert_bounds(&after[1], (10.0, 0.0, 1.5, 1.5), "fraction 0.5")?;
    assert_bounds(&after[2], (20.0, 0.0, 2.0, 2.0), "fraction 1, the shape that grows")?;
    Ok(())
}

#[test]
fn each_shape_reads_its_own_order_and_a_shape_that_grows_has_the_targets() -> Outcome {
    // the drawing's two shapes have order 0 and 1, the target's three 0, 0.5 and 1: by_order(0, 1) is 0, 1, 1
    let after = drawn(&small(2)?.morph(big(3)?, by_order(0.0, 1.0)))?;
    assert_bounds(&after[0], (0.0, 0.0, 1.0, 1.0), "order 0")?;
    assert_bounds(&after[1], (10.0, 0.0, 2.0, 2.0), "order 1")?;
    assert_bounds(
        &after[2],
        (20.0, 0.0, 2.0, 2.0),
        "the grown shape, order 1 in the target",
    )?;
    Ok(())
}

#[test]
fn a_param_fraction_is_read_when_drawn() -> Outcome {
    let progress = Param::new("progress", 0.0, -1.0, 2.0)?;
    let drawing = small(1)?.morph(big(1)?, &progress);
    for (value, width) in [(0.0, 1.0), (0.5, 1.5), (1.0, 2.0), (1.5, 2.5), (-0.5, 0.5)] {
        let after = drawn_with(&drawing, "progress", value)?;
        assert_bounds(&after[0], (0.0, 0.0, width, width), &format!("progress {value}"))?;
    }
    Ok(())
}

fn drawn_with(drawing: &Drawing, name: &str, value: f64) -> Result<Vec<Shape>, Error> {
    drawing.shapes(&Values::new().set(name, value))
}

#[test]
fn a_stagger_runs_each_shape_in_its_own_time() -> Outcome {
    // spread 0.5: the shape of order p runs while progress goes from p / 2 to p / 2 + 0.5; at 0.5 the three shapes
    // (order 0, 0.5, 1) are at 1, 0.5 and 0
    let progress = Param::new("progress", 0.0, 0.0, 1.0)?;
    let drawing = small(3)?.morph(big(3)?, Stagger::new(&progress));
    let after = drawn_with(&drawing, "progress", 0.5)?;
    assert_bounds(&after[0], (0.0, 0.0, 2.0, 2.0), "ahead")?;
    assert_bounds(&after[1], (10.0, 0.0, 1.5, 1.5), "half way")?;
    assert_bounds(&after[2], (20.0, 0.0, 1.0, 1.0), "not started")?;
    Ok(())
}

// Text ---------------------------------------------------------------------------------------------------------------

#[test]
fn two_letters_to_three_pair_by_glyph_and_the_third_grows_with_its_own_style() -> Outcome {
    let font = fraunces()?;
    let (ab, abc) = (text("ab", &font).fill(red()), text("abc", &font).fill(blue()));
    let (before, wanted) = (drawn(&ab)?, drawn(&abc)?);
    assert_eq!((before.len(), wanted.len()), (2, 3));
    let half = drawn(&ab.morph(&abc, 0.5))?;
    assert_eq!(half.len(), 3);
    for (found, own) in half.iter().zip(&before) {
        assert_eq!(found.style(), own.style());
        assert_eq!(
            found.attributes(),
            own.attributes(),
            "origin and glyph stay the source's"
        );
    }
    assert_eq!(half[2].style(), wanted[2].style());
    assert_eq!(half[2].attributes(), wanted[2].attributes());
    // at 1 every shape is the flattening of the target's glyph: its bounds lie within the flatness (0.01) inside
    for (found, own) in drawn(&ab.morph(&abc, 1.0))?.iter().zip(&wanted) {
        let (found, own) = (bounds(found)?, bounds(own)?);
        for (a, b) in [(found.0, own.0), (found.1, own.1), (found.2, own.2), (found.3, own.3)] {
            assert!((a - b).abs() < 0.03, "{found:?} against {own:?}");
        }
    }
    Ok(())
}

// Errors --------------------------------------------------------------------------------------------------------------

#[test]
fn a_fraction_the_library_refuses_names_the_step_and_the_argument() -> Outcome {
    for fraction in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let drawing = small(1)?.morph(big(1)?, fraction);
        let held = drawing.error().cloned().ok_or("no error held")?;
        let (name, _) = parameter_refusal(Err::<(), _>(held))?;
        assert_eq!(name, "step 2 (morph).fraction", "{fraction}");
    }
    // a Param that carries a value the step refuses when it is drawn, naming the shape
    let carrier = Param::new("carrier", 0.0, -1e308, 1e308)?;
    let drawing = small(2)?.morph(big(2)?, by_index(0.0, &carrier));
    assert_eq!(drawing.error(), None);
    let (name, message) = parameter_refusal(drawn_with(&drawing, "carrier", f64::NAN))?;
    assert_eq!(name, "step 1 (morph).fraction");
    assert!(message.starts_with("shape "), "{message}");
    // a finite fraction that sends a point past 1e100 is refused by the path verb, naming the same argument
    let (name, message) = parameter_refusal(small(1)?.morph(big(1)?, 1e300).shapes(&Values::new()))?;
    assert_eq!(name, "step 2 (morph).fraction");
    assert!(message.contains("shape 0"), "{message}");
    Ok(())
}

#[test]
fn a_target_that_holds_an_error_leaves_the_step_with_it_named_by_item() -> Outcome {
    let broken = Drawing::from(rect(0.0, 0.0, 1.0, 1.0)?).translate(f64::NAN, 0.0);
    assert!(broken.error().is_some());
    let drawing = small(1)?.morph(broken, 0.5);
    let (name, _) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error held")?))?;
    assert!(name.contains("step 2 (morph) / item 1"), "{name}");
    Ok(())
}

#[test]
fn both_arguments_are_required() {
    let morph = verbs().find(|verb| verb.name == "morph");
    let required: Vec<_> = morph
        .iter()
        .flat_map(|verb| verb.args)
        .map(|arg| (arg.name, arg.default))
        .collect();
    assert_eq!(required, vec![("target", None), ("fraction", None)]);
}

#[test]
fn a_step_is_charged_for_the_edges_it_flattens_before_it_makes_them() -> Outcome {
    // a circle of radius 1e6 flattens to about 22,000 edges within 0.01 of the curve; 600 of them pass 10,000,000
    let circles = (0..600)
        .map(|number| Ok(Path::circle((f64::from(number), 0.0), 1.0e6)?.to_svg_path()))
        .collect::<Result<Vec<_>, Error>>()?;
    let heavy = Path::from_svg_path(&circles.join(" "))?;
    let drawing = Drawing::from(heavy).morph(rect(0.0, 0.0, 4.0, 4.0)?, 0.5);
    let (name, message) = parameter_refusal(drawn(&drawing))?;
    assert!(
        name.contains("step 1 (morph)") && message.contains("MAX_SEGMENTS"),
        "{name}: {message}"
    );
    Ok(())
}

// The cache and the depth ----------------------------------------------------------------------------------------------

#[test]
fn two_targets_on_one_drawing_are_two_results_even_though_they_share_a_cache() -> Outcome {
    let base = small(1)?;
    let near = base.morph(rect(0.0, 0.0, 2.0, 2.0)?, 0.5);
    let far = base.morph(rect(0.0, 0.0, 4.0, 4.0)?, 0.5);
    for (drawing, width) in [(&near, 1.5), (&far, 2.5), (&near, 1.5)] {
        assert_bounds(
            &drawn(drawing)?[0],
            (0.0, 0.0, width, width),
            "a result from the shared cache",
        )?;
    }
    Ok(())
}

#[test]
fn a_param_the_target_reads_changes_the_result_and_the_same_values_draw_nothing_again() -> Outcome {
    let shift = Param::new("shift", 2.0, 0.0, 20.0)?;
    let target = Drawing::from(rect(0.0, 0.0, 2.0, 2.0)?).translate(&shift, 0.0);
    let drawing = small(1)?.morph(target, 0.5);
    assert_eq!(drawing.parameters()?, vec![shift.clone()]);
    // the unit square's corners go half way to those of the 2 x 2 square at `shift`: the left edge at shift / 2, the
    // right edge at (shift + 3) / 2
    for (value, left) in [(2.0, 1.0), (6.0, 3.0), (2.0, 1.0)] {
        let after = drawn_with(&drawing, "shift", value)?;
        assert_bounds(&after[0], (left, 0.0, 1.5, 1.5), &format!("shift {value}"))?;
    }
    drawn_with(&drawing, "shift", 2.0)?;
    assert_eq!(drawing.stats().computed, 0, "the same values draw nothing again");
    drawn_with(&drawing, "shift", 7.0)?;
    assert!(drawing.stats().computed > 0, "a new value is computed");
    Ok(())
}

#[test]
fn a_target_nested_past_the_deepest_nesting_is_refused_where_it_is_built() -> Outcome {
    // a lone drawing is 1 deep, each morph over a target one deeper: 15 steps make 16, the most accepted
    let square = rect(0.0, 0.0, 2.0, 2.0)?;
    let ink = || Drawing::from(square.clone());
    let mut drawing = Drawing::from(rect(0.0, 0.0, 1.0, 1.0)?);
    for _ in 0..15 {
        drawing = ink().morph(drawing, 0.5);
        assert_eq!(drawing.error(), None);
    }
    assert_eq!(drawn(&drawing)?.len(), 1);
    let (name, message) = parameter_refusal(Err::<(), _>(
        ink().morph(drawing, 0.5).error().cloned().ok_or("no error")?,
    ))?;
    assert_eq!(name, "step 1 (morph).target");
    assert!(message.contains("MAX_DEPTH"), "{message}");
    Ok(())
}

// JSON ---------------------------------------------------------------------------------------------------------------

#[test]
fn a_document_with_a_morph_loads_and_draws_it() -> Outcome {
    let text = r#"{"version":1,"root":[{"path":"M0 0 L1 0 L1 1 L0 1 Z"},{"morph":{"target":[{"path":"M0 0 L2 0 L2 2 L0 2 Z"}],"fraction":0.5}}]}"#;
    let drawing = Drawing::from_json(text, &Inputs::new())?;
    assert_bounds(&drawn(&drawing)?[0], (0.0, 0.0, 1.5, 1.5), "the loaded morph")?;
    Ok(())
}

#[test]
fn the_step_writes_target_as_calls_and_fraction_as_a_number_and_reads_back_the_same() -> Outcome {
    let progress = Param::new("progress", 0.25, -1.0, 2.0)?;
    let drawing = small(2)?
        .morph(big(3)?, by_index(&progress, 1.0))
        .morph(rect(0.0, 0.0, 5.0, 5.0)?, 0.75);
    let text = drawing.to_json()?;
    for expected in [
        r#"{"morph":{"fraction":{"by_index":{"first":{"param":"progress"},"last":1}},"target":["#,
        r#"{"morph":{"fraction":0.75,"target":[{"path":"M0 0 L5 0 L5 5 L0 5 Z"}]}}"#,
    ] {
        assert!(text.contains(expected), "{expected} not in {text}");
    }
    let again = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(again.to_json()?, text);
    assert_eq!(again.parameters()?, drawing.parameters()?);
    for value in [0.25, 1.5] {
        assert_eq!(
            drawn_with(&again, "progress", value)?,
            drawn_with(&drawing, "progress", value)?,
            "shapes at {value}"
        );
    }
    Ok(())
}

#[test]
fn a_document_that_gives_target_or_fraction_wrongly_is_refused_at_the_value() -> Outcome {
    let call =
        |call: &str| format!(r#"{{"version":1,"root":[{{"path":"M0 0 L4 0 L4 4 L0 4 Z"}},{{"morph":{call}}}]}}"#);
    for (what, call_text, pointer, message) in [
        (
            "fraction missing",
            r#"{"target":[{"path":"M0 0"}]}"#,
            "/root/1/morph",
            "fraction",
        ),
        ("target missing", r#"{"fraction":0.5}"#, "/root/1/morph", "target"),
        (
            "target not an array",
            r#"{"target":"M0 0","fraction":0.5}"#,
            "/root/1/morph/target",
            "array",
        ),
        (
            "fraction a word",
            r#"{"target":[{"path":"M0 0"}],"fraction":"half"}"#,
            "/root/1/morph/fraction",
            "number",
        ),
        (
            "an argument it lacks",
            r#"{"target":[{"path":"M0 0"}],"fraction":0.5,"tolerance":1}"#,
            "/root/1/morph/tolerance",
            "not an argument",
        ),
    ] {
        let (at, reason) = format_refusal(Drawing::from_json(&call(call_text), &Inputs::new()))?;
        assert_eq!(at, pointer, "{what}: {reason}");
        assert!(reason.contains(message), "{what}: {reason}");
    }
    Ok(())
}

#[test]
fn the_schema_takes_target_as_a_drawing_and_requires_both_arguments() {
    let schema = drawing_schema();
    assert!(schema.contains(r#""morph":{"#), "{schema}");
    assert!(schema.contains(r##""target":{"$ref":"#/$defs/drawing"}"##), "{schema}");
    assert!(schema.contains(r#""required":["target","fraction"]"#), "{schema}");
}
