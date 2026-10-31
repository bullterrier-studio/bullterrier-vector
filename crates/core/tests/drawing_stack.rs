//! `Drawing::new` (a stack), `with_size`, the size a drawing has, the `order` a stack writes, and a `Shape` put back
//! as an item, through the public API. Expected values are written by hand.

mod common;

use bullterrier_vector::{Color, Drawing, Error, Param, Path, Shape, StrokeStyle, Values};
use common::{Outcome, parameter_refusal};

/// A square of side 10 whose left edge is at `x`.
fn square_at(x: f64) -> Result<Path, Error> {
    Path::rect(x, 0.0, 10.0, 10.0, 0.0)
}

fn drawn(drawing: &Drawing) -> Result<Vec<Shape>, Error> {
    drawing.shapes(&Values::new())
}

/// The left edges of the squares drawn, bottom first.
fn lefts(drawing: &Drawing) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    drawn(drawing)?
        .iter()
        .map(|shape| Ok(shape.path().bounds()?.x))
        .collect()
}

fn orders(drawing: &Drawing) -> Result<Vec<Option<f64>>, Error> {
    Ok(drawn(drawing)?.iter().map(|shape| shape.attributes().order()).collect())
}

fn held(drawing: &Drawing) -> Result<Error, Box<dyn std::error::Error>> {
    drawing
        .error()
        .cloned()
        .ok_or_else(|| "the drawing holds no error".into())
}

// Items -------------------------------------------------------------------------------------------------------------

#[test]
fn a_stack_draws_its_items_bottom_first() -> Outcome {
    let stack = Drawing::new([square_at(0.0)?, square_at(20.0)?, square_at(40.0)?]);
    assert_eq!(lefts(&stack)?, vec![0.0, 20.0, 40.0]);
    let indexes: Vec<_> = drawn(&stack)?.iter().map(Shape::index).collect();
    assert_eq!(indexes, vec![0, 1, 2]);
    // an item may itself have several shapes: they keep their order, and the index counts the whole list
    let nested = Drawing::new([stack.clone(), Drawing::from(square_at(60.0)?)]);
    assert_eq!(lefts(&nested)?, vec![0.0, 20.0, 40.0, 60.0]);
    let indexes: Vec<_> = drawn(&nested)?.iter().map(Shape::index).collect();
    assert_eq!(indexes, vec![0, 1, 2, 3]);
    Ok(())
}

#[test]
fn items_are_paths_shapes_or_drawings_and_a_drawing_can_be_used_twice() -> Outcome {
    let drawing = Drawing::from(square_at(0.0)?).translate(5.0, 0.0);
    let twice = Drawing::new([&drawing, &drawing]);
    assert_eq!(lefts(&twice)?, vec![5.0, 5.0]);
    let shapes = drawn(&drawing)?;
    let from_shapes = Drawing::new(shapes.clone());
    assert_eq!(lefts(&from_shapes)?, vec![5.0]);
    let mixed = Drawing::new([Drawing::from(shapes[0].clone()), Drawing::from(square_at(1.0)?)]);
    assert_eq!(lefts(&mixed)?, vec![5.0, 1.0]);
    Ok(())
}

#[test]
fn a_stack_of_one_item_and_no_size_is_that_item() -> Outcome {
    let p = Param::new("p", 3.0, 0.0, 10.0)?;
    let item = Drawing::from(square_at(0.0)?)
        .translate(&p, 0.0)
        .fill(Color::hex(0xff_4f_9a));
    let same = Drawing::new([item.clone()]);
    for values in [Values::new(), Values::new().set("p", 7.0)] {
        assert_eq!(same.shapes(&values)?, item.shapes(&values)?);
    }
    assert_eq!(same.parameters()?, item.parameters()?);
    assert_eq!(same.size(), item.size());
    // a path item becomes the one-shape drawing of the path, and a stack writes no order on it
    let path = square_at(0.0)?;
    assert_eq!(drawn(&Drawing::new([path.clone()]))?, drawn(&Drawing::from(path))?);
    assert_eq!(orders(&Drawing::new([Drawing::from(square_at(0.0)?)]))?, vec![None]);
    Ok(())
}

#[test]
fn a_stack_of_nothing_draws_nothing() -> Outcome {
    let none: [Path; 0] = [];
    let empty = Drawing::new(none);
    assert_eq!(empty.error(), None);
    assert_eq!(drawn(&empty)?, vec![]);
    Ok(())
}

// Order -------------------------------------------------------------------------------------------------------------

#[test]
fn a_stack_gives_its_shapes_an_order_from_zero_to_one() -> Outcome {
    let four = Drawing::new(
        (0..4)
            .map(|n| square_at(f64::from(n) * 20.0))
            .collect::<Result<Vec<_>, _>>()?,
    );
    let three = Drawing::new([square_at(0.0)?, square_at(20.0)?, square_at(40.0)?]);
    let two = Drawing::new([square_at(0.0)?, square_at(20.0)?]);
    assert_eq!(orders(&two)?, vec![Some(0.0), Some(1.0)]);
    assert_eq!(orders(&three)?, vec![Some(0.0), Some(0.5), Some(1.0)]);
    assert_eq!(
        orders(&four)?,
        vec![Some(0.0), Some(1.0 / 3.0), Some(2.0 / 3.0), Some(1.0)]
    );
    // one shape, but in a frame: it is stacked, and has order 0
    let framed = Drawing::new([square_at(0.0)?]).with_size(50.0, 50.0);
    assert_eq!(orders(&framed)?, vec![Some(0.0)]);
    Ok(())
}

#[test]
fn a_stack_leaves_the_order_a_shape_already_has_and_spreads_over_the_others() -> Outcome {
    let drawn_stack = drawn(&Drawing::new([square_at(0.0)?, square_at(20.0)?]))?;
    // two shapes that carry an order (0 and 1), and three that carry none
    let restacked = Drawing::new([
        Drawing::from(square_at(100.0)?),
        Drawing::from(drawn_stack[1].clone()),
        Drawing::from(square_at(120.0)?),
        Drawing::from(drawn_stack[0].clone()),
        Drawing::from(square_at(140.0)?),
    ]);
    assert_eq!(
        orders(&restacked)?,
        vec![Some(0.0), Some(1.0), Some(0.5), Some(0.0), Some(1.0)]
    );
    // a lone shape without one has order 0 beside shapes with theirs
    let beside = Drawing::new([Drawing::from(drawn_stack[1].clone()), Drawing::from(square_at(0.0)?)]);
    assert_eq!(orders(&beside)?, vec![Some(1.0), Some(0.0)]);
    Ok(())
}

// A shape put back --------------------------------------------------------------------------------------------------

#[test]
fn a_restacked_shape_draws_and_styles_as_it_did() -> Outcome {
    let pink = Color::hex(0xff_4f_9a);
    // (case, a drawing whose style is set, and whose single shape is put back)
    let rows = [
        ("default fill", Drawing::from(square_at(0.0)?)),
        ("default stroke", Drawing::from(Path::line((0.0, 0.0), (10.0, 5.0))?)),
        ("filled", Drawing::from(square_at(0.0)?).fill(pink)),
        ("fill none", Drawing::from(square_at(0.0)?).fill(Color::NONE)),
        (
            "fill none on a line",
            Drawing::from(Path::line((0.0, 0.0), (10.0, 5.0))?).fill(Color::NONE),
        ),
        ("stroked", Drawing::from(square_at(0.0)?).stroke(pink).opacity(0.5)),
    ];
    for (case, original) in rows {
        let first = drawn(&original)?;
        let again = drawn(&Drawing::from(first[0].clone()))?;
        assert_eq!(again[0].style(), first[0].style(), "{case}");
        assert_eq!(again[0].path(), first[0].path(), "{case}");
        // a stack of it draws the same too
        let stacked = drawn(&Drawing::new([first[0].clone(), first[0].clone()]))?;
        assert_eq!(stacked[1].style(), first[0].style(), "{case}");
    }
    Ok(())
}

#[test]
fn a_restacked_shape_takes_a_new_style_like_any_other() -> Outcome {
    let shape = drawn(&Drawing::from(square_at(0.0)?))?.remove(0);
    let style = drawn(&Drawing::from(shape).stroke(Color::hex(0x2a_5b_ff)))?
        .remove(0)
        .style()
        .clone();
    // it was drawn black-filled; a stroke added to it adds to that
    assert_eq!(style.fill(), Color::hex(0));
    assert_eq!(style.stroke().map(StrokeStyle::color), Some(Color::hex(0x2a_5b_ff)));
    Ok(())
}

#[test]
fn a_shape_with_an_empty_path_stays_in_the_list() -> Outcome {
    let stack = Drawing::new([Path::new(), square_at(0.0)?, Path::new()]);
    let shapes = drawn(&stack)?;
    assert_eq!(shapes.len(), 3);
    assert_eq!(shapes[0].path(), &Path::new());
    assert_eq!(shapes[1].path(), &square_at(0.0)?);
    let indexes: Vec<_> = shapes.iter().map(Shape::index).collect();
    assert_eq!(indexes, vec![0, 1, 2]);
    assert_eq!(orders(&stack)?, vec![Some(0.0), Some(0.5), Some(1.0)]);
    Ok(())
}

#[test]
fn a_restacked_shape_is_renumbered_to_its_place_in_the_new_list() -> Outcome {
    let shapes = drawn(&Drawing::new([square_at(0.0)?, square_at(20.0)?]))?;
    assert_eq!(shapes[1].index(), 1);
    let alone = drawn(&Drawing::from(shapes[1].clone()))?;
    assert_eq!(alone[0].index(), 0);
    Ok(())
}

// Steps on a stack --------------------------------------------------------------------------------------------------

#[test]
fn a_step_on_a_stack_reaches_every_shape_and_a_list_gives_each_its_entry() -> Outcome {
    let shift = Param::new("shift", 0.0, -100.0, 100.0)?;
    let stack = Drawing::new([square_at(0.0)?, square_at(20.0)?, square_at(40.0)?]).translate(&shift, 0.0);
    assert_eq!(lefts(&stack)?, vec![0.0, 20.0, 40.0]);
    assert_eq!(
        stack
            .shapes(&Values::new().set("shift", 5.0))
            .map(|shapes| shapes.len())?,
        3
    );
    let lists = Values::new().set("shift", vec![1.0, 2.0, 3.0]);
    let moved: Vec<f64> = stack
        .shapes(&lists)?
        .iter()
        .map(|s| s.path().bounds().map(|b| b.x))
        .collect::<Result<_, _>>()?;
    assert_eq!(moved, vec![1.0, 22.0, 43.0]);
    // a list of the wrong length names the Param, the step and both counts
    let (name, message) = parameter_refusal(stack.shapes(&Values::new().set("shift", vec![1.0, 2.0])))?;
    assert_eq!(name, "step 1 (translate).dx");
    for text in ["shift", "2", "3"] {
        assert!(message.contains(text), "{text:?} not in {message}");
    }
    Ok(())
}

// Size --------------------------------------------------------------------------------------------------------------

#[test]
fn a_size_is_given_by_with_size_and_replaced_by_another_without_nesting() -> Outcome {
    let far = Param::new("far", 0.0, -1e200, 1e200)?;
    let broken = Drawing::from(square_at(0.0)?).translate(0.0, &far);
    let stack = Drawing::new([broken, Drawing::from(square_at(20.0)?)]);
    assert_eq!(stack.size(), None);
    let sized = stack.with_size(400.0, 300.0);
    assert_eq!(sized.size(), Some((400.0, 300.0)));
    let resized = sized.with_size(800.0, 600.0);
    assert_eq!(resized.size(), Some((800.0, 600.0)));
    assert_eq!(lefts(&resized)?, lefts(&stack)?);
    // a stack inside a stack would put its place into the error twice
    let values = Values::new().set("far", 1e150);
    let (name, _) = parameter_refusal(resized.shapes(&values))?;
    assert_eq!(name, "source (stack) / item 1 / step 1 (translate).dy");
    // any other drawing is stacked, alone, in the frame
    let framed = Drawing::from(square_at(0.0)?).with_size(100.0, 50.0);
    assert_eq!(framed.size(), Some((100.0, 50.0)));
    assert_eq!(lefts(&framed)?, vec![0.0]);
    Ok(())
}

#[test]
fn a_size_the_frame_cannot_have_is_refused_at_the_call() -> Outcome {
    let base = Drawing::new([square_at(0.0)?, square_at(20.0)?]);
    for (case, drawing) in [
        ("zero width", base.with_size(0.0, 10.0)),
        ("zero height", base.with_size(10.0, 0.0)),
        ("negative", base.with_size(-1.0, 10.0)),
        ("NaN", base.with_size(f64::NAN, 10.0)),
        ("infinite", base.with_size(10.0, f64::INFINITY)),
        ("too large", base.with_size(100_000.5, 10.0)),
        (
            "a single shape, too large",
            Drawing::from(square_at(0.0)?).with_size(10.0, 100_001.0),
        ),
    ] {
        let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
        assert_eq!(name, "source (stack).size", "{case}: {message}");
    }
    for (width, height) in [(100_000.0, 100_000.0), (f64::MIN_POSITIVE, 1.0)] {
        assert_eq!(base.with_size(width, height).error(), None);
    }
    Ok(())
}

#[test]
fn a_drawing_has_the_size_its_items_have_and_steps_keep_it() -> Outcome {
    let squares = [square_at(0.0)?, square_at(20.0)?];
    let sized = |width: f64, height: f64| Drawing::new(squares.clone()).with_size(width, height);
    let a = sized(400.0, 300.0);
    let unsized_drawing = Drawing::from(square_at(5.0)?);
    // items that agree, or have none
    assert_eq!(Drawing::new([a.clone(), a.clone()]).size(), Some((400.0, 300.0)));
    assert_eq!(
        Drawing::new([a.clone(), unsized_drawing.clone()]).size(),
        Some((400.0, 300.0))
    );
    assert_eq!(
        Drawing::new([unsized_drawing.clone(), a.clone()]).size(),
        Some((400.0, 300.0))
    );
    assert_eq!(
        Drawing::new([unsized_drawing.clone(), unsized_drawing.clone()]).size(),
        None
    );
    // steps keep the size, and so does a stack of one shared drawing
    assert_eq!(a.translate(1.0, 1.0).fill(Color::NONE).size(), Some((400.0, 300.0)));
    assert_eq!(
        Drawing::new([a.translate(1.0, 1.0), a.clone()]).size(),
        Some((400.0, 300.0))
    );
    // a drawing with a step after a stack that has a size
    assert_eq!(
        Drawing::new([a.rotate(bullterrier_vector::Deg(10.0))]).size(),
        Some((400.0, 300.0))
    );
    // a stack's own size beats its items'
    assert_eq!(
        Drawing::new([a.clone(), sized(800.0, 600.0)])
            .with_size(10.0, 20.0)
            .size(),
        Some((10.0, 20.0))
    );
    Ok(())
}

#[test]
fn items_of_different_sizes_are_refused_naming_both() -> Outcome {
    let one = Drawing::from(square_at(0.0)?).with_size(400.0, 300.0);
    let other = Drawing::from(square_at(0.0)?).with_size(800.0, 600.0);
    let stack = Drawing::new([one.clone(), Drawing::from(square_at(1.0)?), other.clone()]);
    let (name, message) = parameter_refusal(Err::<(), _>(held(&stack)?))?;
    assert_eq!(name, "source (stack).items");
    for text in ["400 x 300", "800 x 600"] {
        assert!(message.contains(text), "{text:?} not in {message}");
    }
    // it is a drawing that cannot be drawn, and nothing after it changes that
    assert_eq!(stack.translate(1.0, 1.0).error(), stack.error());
    assert_eq!(stack.shapes(&Values::new()).err(), stack.error().cloned());
    // a size of its own says which frame is meant
    assert_eq!(Drawing::new([one, other]).with_size(400.0, 300.0).error(), None);
    Ok(())
}

// Errors inside items -----------------------------------------------------------------------------------------------

#[test]
fn an_error_inside_an_item_names_the_stack_the_item_and_the_step() -> Outcome {
    let far = Param::new("far", 0.0, -1e200, 1e200)?;
    let good = Drawing::from(square_at(0.0)?);
    let bad_when_drawn = Drawing::from(square_at(0.0)?).translate(&far, 0.0);
    let stack = Drawing::new([good.clone(), bad_when_drawn.clone()]);
    let (name, message) = parameter_refusal(stack.shapes(&Values::new().set("far", 1e150)))?;
    assert_eq!(name, "source (stack) / item 2 / step 1 (translate).dx");
    assert!(message.starts_with("shape 0: "), "{message}");
    // a stack inside a stack, and a step after the stack does not change the place
    let outer = Drawing::new([good.clone(), stack, good]).translate(1.0, 1.0);
    let (name, _) = parameter_refusal(outer.shapes(&Values::new().set("far", 1e150)))?;
    assert_eq!(
        name,
        "source (stack) / item 2 / source (stack) / item 2 / step 1 (translate).dx"
    );
    Ok(())
}

#[test]
fn an_item_that_holds_an_error_leaves_the_stack_with_it_placed_inside_the_item() -> Outcome {
    let good = Drawing::from(square_at(0.0)?);
    let bad = Drawing::from(square_at(0.0)?)
        .translate(1.0, 1.0)
        .translate(f64::NAN, 0.0);
    let stack = Drawing::new([good.clone(), bad]);
    let (name, message) = parameter_refusal(Err::<(), _>(held(&stack)?))?;
    assert_eq!(name, "source (stack) / item 2 / step 2 (translate).dx");
    assert!(message.contains("NaN"), "{message}");
    assert_eq!(stack.parameters().err(), stack.error().cloned());
    Ok(())
}

#[test]
fn params_of_the_items_are_read_in_order_and_one_name_is_one_param() -> Outcome {
    let a = Param::new("a", 0.0, -10.0, 10.0)?;
    let b = Param::new("b", 0.0, -10.0, 10.0)?;
    let stack = Drawing::new([
        Drawing::from(square_at(0.0)?).translate(&b, &a),
        Drawing::from(square_at(20.0)?).translate(&a, 0.0),
    ])
    .translate(0.0, &b);
    assert_eq!(stack.parameters()?, vec![b.clone(), a.clone()]);
    // a value for a Param an item reads is accepted
    let moved = stack.shapes(&Values::new().set("b", 4.0))?;
    assert_eq!(moved.len(), 2);
    // two Params of one name that differ, in different items
    let other_a = Param::new("a", 0.0, -20.0, 20.0)?;
    let clash = Drawing::new([
        Drawing::from(square_at(0.0)?).translate(&a, 0.0),
        Drawing::from(square_at(20.0)?).translate(&other_a, 0.0),
    ]);
    let (name, message) = parameter_refusal(Err::<(), _>(held(&clash)?))?;
    assert!(name == "a" && message.contains("min"), "{name}: {message}");
    Ok(())
}
