//! Own binary: its two million-shape tests peak at 2 to 3 GB, and a process keeps what it allocated.
//!
//! The limits that keep a small drawing from asking for unbounded work, through the public API: the shapes a draw
//! makes, how deep drawings nest, and a long chain of steps. Expected values are written by hand from the rule in
//! formats.md "Limits and security": a step is charged for the shapes it makes when it is computed, a source one and a
//! stack its whole list, so a drawing used twice in a stack is charged for its stack each time, and for itself once:
//! equal sources are one step, computed once, so the sources of these tests differ.

#[path = "it/common/mod.rs"]
mod common;

use bullterrier_vector::{Color, Drawing, Error, Param, Path, Values};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Held by the tests that draw a million shapes (2 GB each), so that two never hold theirs at once.
static MILLION_SHAPES: Mutex<()> = Mutex::new(());

fn one_million_at_a_time() -> MutexGuard<'static, ()> {
    MILLION_SHAPES.lock().unwrap_or_else(PoisonError::into_inner)
}
use common::{Outcome, parameter_refusal};

/// A drawing of one shape that is an empty path, which keeps a million of them cheap.
fn nothing() -> Drawing {
    Drawing::from(Path::new())
}

/// A drawing of one shape that is a point, and no other call gives the same: a source of its own.
#[expect(clippy::cast_precision_loss, reason = "the counts are far below 2^53")]
fn alone(number: usize) -> Result<Drawing, Error> {
    Ok(Drawing::from(Path::start((number as f64, 0.0))?))
}

/// A stack of `count` shapes of different sources: `count` shapes made by sources and `count` by the stack. The
/// sources are numbered from `first`, so that two stacks do not share them.
fn flat(first: usize, count: usize) -> Result<Drawing, Error> {
    Ok(Drawing::new(
        (first..first + count).map(alone).collect::<Result<Vec<_>, _>>()?,
    ))
}

fn drawn(drawing: &Drawing) -> Result<usize, Error> {
    Ok(drawing.shapes(&Values::new())?.len())
}

// MAX_SHAPES --------------------------------------------------------------------------------------------------------

#[test]
fn a_draw_is_charged_for_the_shapes_it_makes_and_refused_past_a_million() -> Outcome {
    let _alone = one_million_at_a_time();
    // (case, the drawing, the shapes made, whether the draw is accepted)
    let nested = |inner: usize, extra: usize| -> Result<Drawing, Error> {
        let mut items = vec![flat(0, inner)?];
        items.extend((inner..inner + extra).map(alone).collect::<Result<Vec<_>, _>>()?);
        Ok(Drawing::new(items))
    };
    let rows = [
        // 500,000 sources and a stack of 500,000
        ("flat, exactly at the limit", flat(0, 500_000)?, 1_000_000, true),
        ("flat, two past", flat(0, 500_001)?, 1_000_002, false),
        // 2 x 333,332 for the inner stack, 2 sources, and a stack of 333,334
        ("nested, exactly at the limit", nested(333_332, 2)?, 1_000_000, true),
        ("nested, one past", nested(333_333, 1)?, 1_000_001, false),
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
fn a_shape_put_back_into_a_drawing_is_charged_like_a_path() -> Outcome {
    let _alone = one_million_at_a_time();
    // 500,001 different shapes: one source, a fill each, and the stack of them (500,002 shapes made)
    let painted = Drawing::new((0..=500_000_u32).map(|number| nothing().fill(Color::hex(number))));
    let shapes = painted.shapes(&Values::new())?;
    // each shape source is one shape, and the stack of them is its whole list: 500,000 of each is the limit
    let of_shapes = |count: usize| Drawing::new(shapes.iter().take(count).cloned().map(Drawing::from));
    assert_eq!(drawn(&of_shapes(500_000))?, 500_000);
    let (name, message) = parameter_refusal(of_shapes(500_001).shapes(&Values::new()))?;
    assert!(
        name.contains("(stack)") && message.contains("MAX_SHAPES"),
        "{name}: {message}"
    );
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
    let _alone = one_million_at_a_time();
    let inner = flat(1, 500_001)?;
    let outer = Drawing::new([alone(0)?, inner]).translate(1.0, 0.0);
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

#[test]
fn a_chain_of_a_hundred_thousand_steps_prints_without_overflowing_the_stack() {
    let mut drawing = nothing();
    for _ in 0..100_000 {
        drawing = drawing.translate(1.0, 0.0);
    }
    let printed = format!("{drawing:?}");
    assert!(printed.len() < 200 && printed.contains("100001"), "{printed}");
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
    let drawing = nothing().translate(&shared, 0.0);
    let (name, message) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error held")?))?;
    assert!(
        name.contains("translate") && message.contains("MAX_EXPR_NODES"),
        "{name}: {message}"
    );
    Ok(())
}

// MAX_PARAMS --------------------------------------------------------------------------------------------------------

/// A drawing whose steps read `count` Params of different names, one each: `p0`, `p1`...
fn reading_params(count: usize) -> Result<Drawing, Error> {
    let mut drawing = nothing();
    for number in 0..count {
        let param = Param::new(&format!("p{number}"), 0.0, 0.0, 10.0)?;
        drawing = drawing.translate(&param, 0.0);
    }
    Ok(drawing)
}

#[test]
fn a_drawing_reads_a_thousand_params_and_not_a_thousand_and_one() -> Outcome {
    let accepted = reading_params(1_000)?;
    assert_eq!(accepted.error(), None);
    assert_eq!(accepted.parameters()?.len(), 1_000);
    let refused = reading_params(1_001)?;
    let (name, message) = parameter_refusal(Err::<(), _>(refused.error().cloned().ok_or("no error held")?))?;
    assert_eq!(name, "step 1001 (translate)");
    assert!(message.contains("MAX_PARAMS 1000"), "{message}");
    // the drawing keeps the error through a step and returns it when drawn
    assert_eq!(refused.translate(1.0, 0.0).error(), refused.error());
    assert_eq!(refused.shapes(&Values::new()).err(), refused.error().cloned());
    Ok(())
}

#[test]
fn a_param_read_again_does_not_count_twice() -> Outcome {
    let mut drawing = reading_params(1_000)?;
    for number in 0..1_000 {
        drawing = drawing.translate(&Param::new(&format!("p{number}"), 0.0, 0.0, 10.0)?, 0.0);
    }
    assert_eq!(drawing.error(), None);
    assert_eq!(drawing.parameters()?.len(), 1_000);
    Ok(())
}
