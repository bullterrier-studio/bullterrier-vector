//! `only` and `without` through the public API: they keep or drop shapes by attribute, renumber the list, are
//! complements of each other, and refuse an attribute nobody has. Expected values are written by hand.

mod common;

use bullterrier_vector::{Drawing, Error, Path, Point, Select, Shape, Values};
use common::{Outcome, parameter_refusal};
use proptest::prelude::*;

/// A square of side 10 whose left edge is at `x`.
fn square_at(x: f64) -> Result<Path, Error> {
    Path::rect(x, 0.0, 10.0, 10.0, 0.0)
}

/// Squares with their left edges at 0, 20, 40, ... stacked bottom first.
fn row(count: u32) -> Result<Drawing, Error> {
    let squares = (0..count)
        .map(|n| square_at(f64::from(n) * 20.0))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Drawing::new(squares))
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

fn held(drawing: &Drawing) -> Result<Error, Box<dyn std::error::Error>> {
    drawing
        .error()
        .cloned()
        .ok_or_else(|| "the drawing holds no error".into())
}

// By index ----------------------------------------------------------------------------------------------------------

#[test]
fn only_keeps_the_shape_at_an_index_and_without_drops_it() -> Outcome {
    let four = row(4)?;
    assert_eq!(lefts(&four.only(Select::new().attribute("index", 2)))?, vec![40.0]);
    assert_eq!(
        lefts(&four.without(Select::new().attribute("index", 2)))?,
        vec![0.0, 20.0, 60.0]
    );
    Ok(())
}

#[test]
fn a_list_of_values_means_any_of_them() -> Outcome {
    let four = row(4)?;
    let ends = Select::new().attribute("index", [0, 3]);
    assert_eq!(lefts(&four.only(ends.clone()))?, vec![0.0, 60.0]);
    assert_eq!(lefts(&four.without(ends))?, vec![20.0, 40.0]);
    // a list of nothing is any of nothing
    let nothing: [u32; 0] = [];
    assert_eq!(lefts(&four.only(Select::new().attribute("index", nothing)))?, vec![]);
    Ok(())
}

#[test]
fn the_list_is_renumbered_after_the_step() -> Outcome {
    // `without(index=2)` leaves 0, 20, 60 at indexes 0, 1, 2: the former index 3 is now index 2
    let three = row(4)?.without(Select::new().attribute("index", 2));
    let indexes: Vec<_> = drawn(&three)?.iter().map(Shape::index).collect();
    assert_eq!(indexes, vec![0, 1, 2]);
    assert_eq!(lefts(&three.only(Select::new().attribute("index", 2)))?, vec![60.0]);
    Ok(())
}

#[test]
fn several_attributes_must_all_match() -> Outcome {
    let three = row(3)?;
    // the middle shape has index 1 and order 0.5; the last has index 2 and order 1
    let both = Select::new().attribute("index", [1, 2]).attribute("order", 1.0);
    assert_eq!(lefts(&three.only(both.clone()))?, vec![40.0]);
    assert_eq!(lefts(&three.without(both))?, vec![0.0, 20.0]);
    Ok(())
}

// By order ----------------------------------------------------------------------------------------------------------

#[test]
fn order_is_matched_exactly_and_survives_a_shape_put_back_in_a_stack() -> Outcome {
    let three = row(3)?;
    assert_eq!(lefts(&three.only(Select::new().attribute("order", 0.5)))?, vec![20.0]);
    assert_eq!(
        lefts(&three.only(Select::new().attribute("order", [0.0, 1.0])))?,
        vec![0.0, 40.0]
    );
    // the shapes drawn keep the order they carry; a new path beside them gets its own (0)
    let mut shapes: Vec<Drawing> = drawn(&three)?.into_iter().map(Drawing::from).collect();
    shapes.push(Drawing::from(square_at(100.0)?));
    let restacked = Drawing::new(shapes);
    assert_eq!(
        lefts(&restacked.only(Select::new().attribute("order", 0.0)))?,
        vec![0.0, 100.0]
    );
    Ok(())
}

// Refusals ----------------------------------------------------------------------------------------------------------

#[test]
fn a_selection_the_call_cannot_make_sense_of_is_refused_naming_the_step_and_the_attribute() -> Outcome {
    let two = row(2)?;
    // (case, the drawing after the call, the argument named, a text the message holds)
    let rows = [
        (
            "unknown name",
            two.only(Select::new().attribute("colour", 1)),
            "step 1 (only).colour",
            "index",
        ),
        (
            "unknown name in without",
            two.without(Select::new().attribute("colour", 1)),
            "step 1 (without).colour",
            "index",
        ),
        (
            "a letter for a number",
            two.only(Select::new().attribute("index", 'a')),
            "step 1 (only).index",
            "whole number",
        ),
        (
            "a fraction for a whole number",
            two.only(Select::new().attribute("index", 1.5)),
            "step 1 (only).index",
            "1.5",
        ),
        (
            "a negative index",
            two.only(Select::new().attribute("index", -1.0)),
            "step 1 (only).index",
            "-1",
        ),
        (
            "NaN",
            two.without(Select::new().attribute("order", f64::NAN)),
            "step 1 (without).order",
            "NaN",
        ),
        (
            "a number for a character",
            two.only(Select::new().attribute("char", 65)),
            "step 1 (only).char",
            "character",
        ),
        (
            "a number for a point",
            two.only(Select::new().attribute("origin", 1.0)),
            "step 1 (only).origin",
            "point",
        ),
        (
            "nothing named",
            two.only(Select::new()),
            "step 1 (only).attributes",
            "at least one",
        ),
    ];
    for (case, drawing, argument, text) in rows {
        let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
        assert!(name == argument && message.contains(text), "{case}: {name}: {message}");
        assert_eq!(drawing.shapes(&Values::new()).err(), drawing.error().cloned(), "{case}");
    }
    // every kind of value is accepted for its attribute
    for accepted in [
        Select::new().attribute("char", 'O'),
        Select::new().attribute("origin", Point::new(1.0, 2.0)),
        Select::new().attribute("level", 0.5),
        Select::new().attribute("glyph", 0),
    ] {
        assert_eq!(two.only(accepted).error(), None);
    }
    Ok(())
}

#[test]
fn a_whole_number_attribute_is_accepted_up_to_u32_max_and_refused_past_it() -> Outcome {
    let two = row(2)?;
    assert_eq!(
        two.only(Select::new().attribute("index", 4_294_967_295.0)).error(),
        None
    );
    let past = two.only(Select::new().attribute("index", 4_294_967_296.0));
    let (name, message) = parameter_refusal(Err::<(), _>(held(&past)?))?;
    assert!(
        name == "step 1 (only).index" && message.contains("whole number") && message.contains("4294967296"),
        "{name}: {message}"
    );
    Ok(())
}

#[test]
fn an_attribute_no_shape_carries_is_refused_when_drawn_naming_it() -> Outcome {
    let two = row(2)?;
    // a drawing of paths and a stack carries index and order, and nothing a generator writes
    for (case, drawing, argument) in [
        (
            "only",
            two.only(Select::new().attribute("char", 'O')),
            "step 1 (only).char",
        ),
        (
            "without",
            two.without(Select::new().attribute("copy", 1)),
            "step 1 (without).copy",
        ),
    ] {
        assert_eq!(drawing.error(), None, "{case}: the call cannot know");
        let (name, message) = parameter_refusal(drawing.shapes(&Values::new()))?;
        assert!(
            name == argument && message.contains("no shape carries"),
            "{case}: {name}: {message}"
        );
    }
    // an empty list carries nothing, and asks nothing
    let none = two.only(Select::new().attribute("index", 9));
    assert_eq!(lefts(&none.only(Select::new().attribute("char", 'O')))?, vec![]);
    Ok(())
}

// Complements -------------------------------------------------------------------------------------------------------

proptest! {
    // `only` and `without` of one selection split the list: together they are every shape, with none in both.
    #[test]
    fn only_and_without_split_the_list(count in 0u32..=12, wanted in proptest::collection::btree_set(0u32..=14, 0..=6)) {
        let fail = |e: Error| TestCaseError::fail(e.to_string());
        let all = row(count).map_err(fail)?;
        let select = Select::new().attribute("index", wanted.iter().copied().collect::<Vec<u32>>());
        let kept = lefts(&all.only(select.clone())).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let dropped = lefts(&all.without(select)).map_err(|e| TestCaseError::fail(e.to_string()))?;
        // the oracle: shape n is at 20 n
        let at = |n: u32| f64::from(n) * 20.0;
        let expected_kept: Vec<f64> = (0..count).filter(|n| wanted.contains(n)).map(at).collect();
        let expected_dropped: Vec<f64> = (0..count).filter(|n| !wanted.contains(n)).map(at).collect();
        prop_assert_eq!(&kept, &expected_kept);
        prop_assert_eq!(&dropped, &expected_dropped);
    }
}
