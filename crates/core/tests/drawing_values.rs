//! Which Params a `Drawing` reads and in what order, `with_values` fixing some for the steps before it, and the values
//! given when drawing, through the public API. Expected values are written by hand.

mod common;

use bullterrier_vector::{Deg, Drawing, Error, Param, Path, Rotate, Select, Unit, Values};
use common::{Outcome, parameter_refusal};

/// A square of side 10 whose left edge is at 0.
fn square() -> Result<Drawing, Error> {
    Ok(Drawing::from(Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?))
}

/// A Param `name` from 0 to 100, defaulting to `default`.
fn slider(name: &str, default: f64) -> Result<Param, Error> {
    Param::new(name, default, 0.0, 100.0)
}

/// The names of the Params the drawing reads, in order.
fn names(drawing: &Drawing) -> Result<Vec<String>, Error> {
    Ok(drawing
        .parameters()?
        .iter()
        .map(|param| param.name().to_owned())
        .collect())
}

/// The left edges of the shapes drawn with `values`.
fn lefts(drawing: &Drawing, values: &Values) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    drawing
        .shapes(values)?
        .iter()
        .map(|shape| Ok(shape.path().bounds()?.x))
        .collect()
}

fn held(drawing: &Drawing) -> Result<Error, Box<dyn std::error::Error>> {
    drawing
        .error()
        .cloned()
        .ok_or_else(|| "the drawing holds no error".into())
}

// The order of parameters() -----------------------------------------------------------------------------------------

#[test]
fn params_are_listed_in_the_order_the_walk_first_uses_them_each_once() -> Outcome {
    let (a, b, c) = (slider("a", 1.0)?, slider("b", 2.0)?, slider("c", 3.0)?);
    // steps in order, and a Param read twice is one
    let chain = square()?.translate(&b, &a).translate(&c, &b);
    assert_eq!(names(&chain)?, ["b", "a", "c"]);
    // the arguments of a step in the order the verb declares them (angle, then about), not the order they were given
    let turned = square()?.rotate(Rotate::new(Deg(&c)).about((&a, &b)));
    assert_eq!(names(&turned)?, ["c", "a", "b"]);
    // a stack's items in order, then the steps after it
    let stack = Drawing::new([square()?.translate(&b, 0.0), square()?.translate(&a, 0.0)]).rotate(Deg(&c));
    assert_eq!(names(&stack)?, ["b", "a", "c"]);
    // the Param itself, as declared
    assert_eq!(chain.parameters()?[1], a);
    Ok(())
}

#[test]
fn two_params_of_one_name_that_differ_are_refused_naming_the_field() -> Outcome {
    let base = slider("a", 1.0)?;
    let rows = [
        ("default", Param::new("a", 2.0, 0.0, 100.0)?),
        ("min", Param::new("a", 1.0, -5.0, 100.0)?),
        ("max", Param::new("a", 1.0, 0.0, 50.0)?),
        ("unit", slider("a", 1.0)?.unit(Unit::Px)?),
        ("meaning", slider("a", 1.0)?.meaning("how far")),
    ];
    for (field, other) in rows {
        let drawing = square()?.translate(&base, 0.0).translate(0.0, &other);
        let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
        assert_eq!(name, "a", "{field}");
        assert!(message.contains(field), "{field}: {message}");
        assert_eq!(drawing.parameters().err(), drawing.error().cloned(), "{field}");
    }
    // equal in every field they are one
    assert_eq!(
        names(&square()?.translate(&base, 0.0).translate(0.0, slider("a", 1.0)?))?,
        ["a"]
    );
    Ok(())
}

// with_values -------------------------------------------------------------------------------------------------------

#[test]
fn with_values_fixes_a_param_for_the_steps_before_it_and_removes_it_from_the_list() -> Outcome {
    let (a, b) = (slider("a", 1.0)?, slider("b", 2.0)?);
    let drawing = square()?
        .translate(&a, 0.0)
        .translate(&b, 0.0)
        .with_values(&Values::new().set("a", 5.0));
    assert_eq!(drawing.error(), None);
    assert_eq!(names(&drawing)?, ["b"]);
    assert_eq!(lefts(&drawing, &Values::new())?, vec![7.0]);
    assert_eq!(lefts(&drawing, &Values::new().set("b", 10.0))?, vec![15.0]);
    // the fixed value is not the caller's to give
    let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("a", 9.0)))?;
    assert!(name == "a" && message.contains("only b"), "{name}: {message}");
    Ok(())
}

#[test]
fn a_step_after_with_values_reads_the_free_param_again() -> Outcome {
    let (a, b) = (slider("a", 1.0)?, slider("b", 2.0)?);
    let drawing = square()?
        .translate(&a, 0.0)
        .with_values(&Values::new().set("a", 5.0))
        .translate(&b, 0.0)
        .translate(&a, 0.0);
    // `a` is read again by the last step, so it is free again, at its place in the order first used
    assert_eq!(names(&drawing)?, ["a", "b"]);
    assert_eq!(lefts(&drawing, &Values::new())?, vec![5.0 + 2.0 + 1.0]);
    assert_eq!(lefts(&drawing, &Values::new().set("a", 20.0))?, vec![5.0 + 2.0 + 20.0]);
    Ok(())
}

#[test]
fn a_fixed_param_still_counts_for_the_one_name_rule() -> Outcome {
    let drawing = square()?
        .translate(slider("a", 1.0)?, 0.0)
        .with_values(&Values::new().set("a", 5.0))
        .translate(0.0, slider("a", 2.0)?);
    let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
    assert!(name == "a" && message.contains("default"), "{name}: {message}");
    Ok(())
}

#[test]
fn with_values_reaches_the_drawings_a_step_takes_but_only_at_that_place() -> Outcome {
    let a = slider("a", 1.0)?;
    let item = square()?.translate(&a, 0.0);
    // the item twice: once fixed where it is used, once free
    let stack = Drawing::new([
        Drawing::new([item.clone(), square()?]).with_values(&Values::new().set("a", 30.0)),
        item,
    ]);
    assert_eq!(names(&stack)?, ["a"]);
    assert_eq!(lefts(&stack, &Values::new())?, vec![30.0, 0.0, 1.0]);
    assert_eq!(lefts(&stack, &Values::new().set("a", 8.0))?, vec![30.0, 0.0, 8.0]);
    // fixed on the stack, an item sees it however deep it is, and the outer list loses the Param
    let outer = Drawing::new([Drawing::new([square()?.translate(&a, 0.0), square()?]), square()?])
        .with_values(&Values::new().set("a", 40.0));
    assert_eq!(names(&outer)?, Vec::<String>::new());
    assert_eq!(lefts(&outer, &Values::new())?, vec![40.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn a_fixed_value_beats_the_one_given_when_drawing_where_the_param_is_still_free_elsewhere() -> Outcome {
    let a = slider("a", 1.0)?;
    let fixed = square()?.translate(&a, 0.0).with_values(&Values::new().set("a", 3.0));
    let stack = Drawing::new([fixed, square()?.translate(&a, 0.0)]);
    assert_eq!(lefts(&stack, &Values::new().set("a", 9.0))?, vec![3.0, 9.0]);
    Ok(())
}

#[test]
fn with_values_refuses_a_name_that_is_not_free_listing_the_free_params() -> Outcome {
    let (a, b) = (slider("a", 1.0)?, slider("b", 2.0)?);
    let two = square()?.translate(&a, &b);
    // (case, the drawing after the call, the argument named, a text the message holds)
    let rows = [
        (
            "unknown",
            two.with_values(&Values::new().set("zzz", 1.0)),
            "step 2 (with_values).zzz",
            "a, b",
        ),
        (
            "already fixed",
            two.with_values(&Values::new().set("a", 1.0))
                .with_values(&Values::new().set("a", 2.0)),
            "step 3 (with_values).a",
            "only b",
        ),
        (
            "no Params at all",
            square()?.with_values(&Values::new().set("a", 1.0)),
            "step 1 (with_values).a",
            "no Param",
        ),
    ];
    for (case, drawing, argument, text) in rows {
        let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
        assert!(name == argument && message.contains(text), "{case}: {name}: {message}");
    }
    Ok(())
}

#[test]
fn with_values_checks_the_range_of_a_bounded_param_when_called() -> Outcome {
    let a = slider("a", 1.0)?;
    let base = square()?.translate(&a, 0.0);
    for (value, text) in [
        (101.0, "101"),
        (-0.5, "-0.5"),
        (f64::NAN, "NaN"),
        (f64::INFINITY, "inf"),
    ] {
        let drawing = base.with_values(&Values::new().set("a", value));
        let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
        assert_eq!(name, "step 2 (with_values).a");
        assert!(message.contains(text), "{message}");
    }
    // the bounds are accepted, a list is checked entry by entry
    assert_eq!(base.with_values(&Values::new().set("a", 100.0)).error(), None);
    assert_eq!(base.with_values(&Values::new().set("a", 0.0)).error(), None);
    let listed = base.with_values(&Values::new().set("a", vec![1.0, 2.0, 200.0]));
    let (name, message) = parameter_refusal(Err::<(), _>(held(&listed)?))?;
    assert!(
        name == "step 2 (with_values).a" && message.contains("200"),
        "{name}: {message}"
    );
    Ok(())
}

#[test]
fn a_cyclic_param_takes_any_finite_value_wrapped_when_fixed_or_given() -> Outcome {
    let turn = Param::new("turn", 0.0, 0.0, 360.0)?.unit(Unit::Deg)?;
    let moved = square()?.translate(&turn, 0.0);
    let fixed_370 = moved.with_values(&Values::new().set("turn", 370.0));
    let fixed_10 = moved.with_values(&Values::new().set("turn", 10.0));
    assert_eq!(fixed_370.error(), None);
    assert_eq!(fixed_370.shapes(&Values::new())?, fixed_10.shapes(&Values::new())?);
    assert_eq!(lefts(&fixed_370, &Values::new())?, vec![10.0]);
    assert_eq!(lefts(&moved, &Values::new().set("turn", -10.0))?, vec![350.0]);
    assert_eq!(lefts(&moved, &Values::new().set("turn", 720.0))?, vec![0.0]);
    Ok(())
}

#[test]
fn a_list_can_be_fixed_and_goes_to_the_shapes_by_index() -> Outcome {
    let p = slider("p", 0.0)?;
    let two = Drawing::new([square()?, square()?]).translate(&p, 0.0);
    let fixed = two.with_values(&Values::new().set("p", vec![3.0, 4.0]));
    assert_eq!(lefts(&fixed, &Values::new())?, vec![3.0, 4.0]);
    let three = Drawing::new([square()?, square()?, square()?]).translate(&p, 0.0);
    let (name, message) = parameter_refusal(
        three
            .with_values(&Values::new().set("p", vec![3.0, 4.0]))
            .shapes(&Values::new()),
    )?;
    assert_eq!(name, "step 1 (translate).dx");
    assert!(
        message.contains('p') && message.contains('2') && message.contains("3 shapes"),
        "{message}"
    );
    Ok(())
}

// Values given when drawing -----------------------------------------------------------------------------------------

#[test]
fn a_value_for_a_param_the_drawing_does_not_read_is_refused_listing_the_ones_it_does() -> Outcome {
    let drawing = square()?.translate(slider("a", 1.0)?, slider("b", 2.0)?);
    let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("c", 1.0)))?;
    assert!(name == "c" && message.contains("a, b"), "{name}: {message}");
    let (name, message) = parameter_refusal(square()?.shapes(&Values::new().set("c", 1.0)))?;
    assert!(name == "c" && message.contains("no Params"), "{name}: {message}");
    Ok(())
}

#[test]
fn a_value_that_is_not_a_number_or_is_out_of_range_is_refused_naming_the_step_and_the_param() -> Outcome {
    let drawing = square()?.translate(slider("a", 1.0)?, 0.0);
    for (value, text) in [
        (f64::NAN, "NaN"),
        (f64::INFINITY, "inf"),
        (100.5, "100.5"),
        (-1.0, "-1"),
    ] {
        let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("a", value)))?;
        assert_eq!(name, "step 1 (translate).dx");
        assert!(message.contains('a') && message.contains(text), "{message}");
    }
    assert_eq!(lefts(&drawing, &Values::new().set("a", 100.0))?, vec![100.0]);
    assert_eq!(lefts(&drawing, &Values::new().set("a", 0.0))?, vec![0.0]);
    Ok(())
}

#[test]
fn a_list_goes_to_the_shape_with_that_index_at_the_step_that_reads_it() -> Outcome {
    let p = slider("p", 0.0)?;
    let four = Drawing::new([square()?, square()?, square()?, square()?]);
    // squares sit at 0; the translate after `without(index=1)` sees three shapes numbered 0, 1, 2
    let three = four.without(Select::new().attribute("index", 1)).translate(&p, 0.0);
    assert_eq!(
        lefts(&three, &Values::new().set("p", vec![10.0, 20.0, 30.0]))?,
        vec![10.0, 20.0, 30.0]
    );
    // a list for the four it started with no longer fits
    let (name, message) = parameter_refusal(three.shapes(&Values::new().set("p", vec![1.0, 2.0, 3.0, 4.0])))?;
    assert_eq!(name, "step 2 (translate).dx");
    assert!(message.contains('4') && message.contains("3 shapes"), "{message}");
    // read before the filter, the list is for four shapes
    let early = four.translate(&p, 0.0).without(Select::new().attribute("index", 1));
    assert_eq!(
        lefts(&early, &Values::new().set("p", vec![10.0, 20.0, 30.0, 40.0]))?,
        vec![10.0, 30.0, 40.0]
    );
    Ok(())
}

#[test]
fn a_drawing_with_an_error_keeps_it_through_with_values() -> Outcome {
    let bad = square()?.translate(f64::NAN, 0.0);
    let fixed = bad.with_values(&Values::new().set("a", 1.0));
    assert_eq!(fixed.error(), bad.error());
    Ok(())
}
