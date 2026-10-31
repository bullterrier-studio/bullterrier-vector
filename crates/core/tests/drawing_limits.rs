//! The limits that keep a small drawing from asking for unbounded work, through the public API: the shapes a draw
//! makes, how deep drawings nest, and a long chain of steps. Expected values are written by hand from the rule in
//! formats.md "Limits and security": a step is charged for the shapes it makes, a source one and a stack its whole
//! list, so a drawing used twice in a stack is charged twice.

mod common;

use bullterrier_vector::{Drawing, Error, Param, Path, Values};
use common::{Outcome, parameter_refusal};
use std::time::Instant;

/// A drawing of one shape that is an empty path, which keeps a million of them cheap.
fn nothing() -> Drawing {
    Drawing::from(Path::new())
}

/// A stack of `count` empty shapes: `count` shapes made by sources and `count` by the stack.
fn flat(count: usize) -> Drawing {
    Drawing::new((0..count).map(|_| Path::new()))
}

fn drawn(drawing: &Drawing) -> Result<usize, Error> {
    Ok(drawing.shapes(&Values::new())?.len())
}

// MAX_SHAPES --------------------------------------------------------------------------------------------------------

#[test]
fn a_draw_is_charged_for_the_shapes_it_makes_and_refused_past_a_million() -> Outcome {
    // (case, the drawing, the shapes made, whether the draw is accepted)
    let nested = |inner: usize, extra: usize| {
        let mut items = vec![flat(inner)];
        items.extend((0..extra).map(|_| nothing()));
        Drawing::new(items)
    };
    let rows = [
        // 500,000 sources and a stack of 500,000
        ("flat, exactly at the limit", flat(500_000), 1_000_000, true),
        ("flat, two past", flat(500_001), 1_000_002, false),
        // 2 x 333,332 for the inner stack, 2 sources, and a stack of 333,334
        ("nested, exactly at the limit", nested(333_332, 2), 1_000_000, true),
        ("nested, one past", nested(333_333, 1), 1_000_001, false),
    ];
    for (case, drawing, made, accepted) in rows {
        assert_eq!(drawing.error(), None, "{case}");
        if accepted {
            assert!(drawn(&drawing)? > 0, "{case}: {made} shapes made");
        } else {
            let (name, message) = parameter_refusal(drawing.shapes(&Values::new()))?;
            assert!(
                name.contains("(stack)") && message.contains("MAX_SHAPES") && message.contains("1000000"),
                "{case}: {name}: {message}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_stack_of_a_drawing_with_itself_is_charged_each_time_and_stops() -> Outcome {
    let doubled = |times: usize| {
        let mut drawing = nothing();
        for _ in 0..times {
            drawing = Drawing::new([drawing.clone(), drawing]);
        }
        drawing
    };
    // 2^15 shapes, made 16 x 2^15 times over by the sources and the stacks: well under the limit
    assert_eq!(drawn(&doubled(15))?, 32_768);
    // 2^20 shapes in the list: the draw is refused, and the refusal names where
    let (name, message) = parameter_refusal(doubled(20).shapes(&Values::new()))?;
    assert!(name.contains("(stack)"), "{name}");
    assert!(
        message.contains("MAX_SHAPES") && message.contains("1000000"),
        "{message}"
    );
    Ok(())
}

#[test]
fn the_refusal_names_the_stack_inside_the_item_it_is_in() -> Outcome {
    let inner = flat(500_001);
    let outer = Drawing::new([nothing(), inner]).translate(1.0, 0.0);
    let (name, _) = parameter_refusal(outer.shapes(&Values::new()))?;
    assert_eq!(name, "source (stack) / item 2 / source (stack)");
    Ok(())
}

// MAX_DEPTH ---------------------------------------------------------------------------------------------------------

/// A drawing nested `depth` drawings deep: a path is 1, a stack of it and another path 2, and so on.
fn nest(depth: usize) -> Drawing {
    let mut drawing = nothing();
    for _ in 1..depth {
        drawing = Drawing::new([drawing, nothing()]);
    }
    drawing
}

#[test]
fn drawings_nested_128_deep_are_accepted_and_129_are_not() -> Outcome {
    let accepted = nest(128);
    assert_eq!(accepted.error(), None);
    assert_eq!(drawn(&accepted)?, 128);
    let refused = nest(129);
    let (name, message) = parameter_refusal(Err::<(), _>(refused.error().cloned().ok_or("no error held")?))?;
    assert_eq!(name, "source (stack).items");
    assert!(
        message.contains("129") && message.contains("MAX_DEPTH") && message.contains("128"),
        "{message}"
    );
    // the drawing keeps the error through a step and returns it when drawn
    assert_eq!(refused.translate(1.0, 0.0).error(), refused.error());
    assert_eq!(refused.shapes(&Values::new()).err(), refused.error().cloned());
    Ok(())
}

#[test]
fn a_frame_adds_a_level_unless_the_drawing_is_a_stack_it_can_resize() {
    // a stack is given the size, with no nesting
    assert_eq!(nest(128).with_size(10.0, 10.0).error(), None);
    // any other drawing is stacked alone in the frame: one level more
    assert_eq!(nest(127).translate(1.0, 0.0).with_size(10.0, 10.0).error(), None);
    assert_ne!(nest(128).translate(1.0, 0.0).with_size(10.0, 10.0).error(), None);
}

#[test]
fn a_chain_of_a_hundred_thousand_steps_draws_and_drops_without_overflowing_the_stack() -> Outcome {
    let mut drawing = Drawing::from(Path::line((0.0, 0.0), (1.0, 0.0))?);
    for _ in 0..100_000 {
        drawing = drawing.translate(1.0, 0.0);
    }
    assert_eq!(drawing.error(), None);
    let shapes = drawing.shapes(&Values::new())?;
    assert_eq!(shapes[0].path().to_svg_path(), "M100000 0 L100001 0");
    assert_eq!(drawing.parameters()?, vec![]);
    Ok(())
}

// MAX_EXPR_NODES ----------------------------------------------------------------------------------------------------

#[test]
fn an_expression_that_shares_itself_is_refused_at_the_call_without_being_worked_out() -> Outcome {
    // each doubling shares the expression and reads it twice: 2^29 - 1 nodes, 19 s of evaluation
    let x = Param::new("x", 1.0, 0.0, 10.0)?;
    let mut shared = bullterrier_vector::Expr::from(&x);
    for _ in 0..28 {
        shared = &shared + &shared;
    }
    let started = Instant::now();
    let drawing = nothing().translate(&shared, 0.0);
    let (name, message) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error held")?))?;
    assert!(
        name.contains("translate") && message.contains("MAX_EXPR_NODES"),
        "{name}: {message}"
    );
    assert!(
        started.elapsed().as_secs_f64() < 1.0,
        "refused after {:?}",
        started.elapsed()
    );
    Ok(())
}
