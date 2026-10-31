//! The cache of evaluated steps, through the public API: a draw computes only the steps that depend on a Param whose
//! value changed, a drawing used twice is computed once, equal drawings built apart share their steps, and what a
//! draw did not use is dropped. Expected counts are worked out by hand from the chain, not from the library.

mod common;

use bullterrier_vector::{Deg, Drawing, Error, Param, Path, Select, Shape, Stats, Values};
use common::Outcome;
use common::ops::{chain, edits, extend, materialise, values_of};
use proptest::prelude::*;

fn square() -> Result<Drawing, Error> {
    Ok(Drawing::from(Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?))
}

fn slider(name: &str, default: f64) -> Result<Param, Error> {
    Param::new(name, default, -100.0, 100.0)
}

/// What the draw of `drawing` at `values` did.
fn stats_of(drawing: &Drawing, values: &Values) -> Result<Stats, Error> {
    drawing.shapes(values)?;
    Ok(drawing.stats())
}

fn counts(computed: usize, reused: usize, entries: usize) -> Stats {
    Stats {
        computed,
        reused,
        entries,
    }
}

/// Six calls, numbered as a person counts them: 1 the path, 2 reads `a`, 3 reads nothing, 4 reads `a`, 5 reads `b`,
/// 6 reads nothing. A step depends on every Param read at or before it.
fn six() -> Result<Drawing, Error> {
    let (a, b) = (slider("a", 0.0)?, slider("b", 0.0)?);
    Ok(square()?
        .translate(&a, 0.0)
        .translate(1.0, 1.0)
        .translate(0.0, &a)
        .translate(&b, 0.0)
        .opacity(0.5))
}

// Only the dependents are computed -----------------------------------------------------------------------------------

#[test]
fn a_changed_param_recomputes_the_steps_from_its_first_reader_on() -> Outcome {
    let chain = six()?;
    let rows = [
        // (case, values, computed, reused): the entries are one per step each time
        ("cold: all six", Values::new(), 6, 0),
        ("a changed: steps 2 to 6", Values::new().set("a", 1.0), 5, 1),
        (
            "b changed too: steps 5 and 6",
            Values::new().set("a", 1.0).set("b", 1.0),
            2,
            1,
        ),
        ("the same again: none", Values::new().set("a", 1.0).set("b", 1.0), 0, 1),
        ("b changed back: steps 5 and 6", Values::new().set("a", 1.0), 2, 1),
        ("a changed again: steps 2 to 6", Values::new().set("a", 2.0), 5, 1),
    ];
    for (case, values, computed, reused) in rows {
        assert_eq!(stats_of(&chain, &values)?, counts(computed, reused, 6), "{case}");
    }
    Ok(())
}

// A drawing used twice is computed once -----------------------------------------------------------------------------

#[test]
fn a_drawing_stacked_with_itself_twelve_times_computes_each_distinct_step_once() -> Outcome {
    // the reviewer's repro: without a cache this is 2^12 evaluations of 201 steps
    let mut drawing = square()?;
    for _ in 0..200 {
        drawing = drawing.translate(1.0, 0.0);
    }
    for _ in 0..12 {
        drawing = Drawing::new([drawing.clone(), drawing]);
    }
    let shapes = drawing.shapes(&Values::new())?;
    assert_eq!(shapes.len(), 4096);
    // distinct steps: the path, 200 translates and 12 stacks; each stack finds its second item already computed
    assert_eq!(drawing.stats(), counts(213, 12, 213));
    // drawn again, the last step is served as it is
    drawing.shapes(&Values::new())?;
    assert_eq!(drawing.stats(), counts(0, 1, 213));
    Ok(())
}

#[test]
fn equal_drawings_built_apart_share_their_steps() -> Outcome {
    let apart = || -> Result<Drawing, Error> { Ok(square()?.translate(&slider("a", 0.0)?, 0.0).rotate(Deg(30.0))) };
    let stacked = Drawing::new([apart()?, apart()?]);
    stacked.shapes(&Values::new())?;
    // the path, the translate and the rotate once, and the stack: the second item is the first again
    assert_eq!(stacked.stats(), counts(4, 1, 4));
    // and a value for the Param moves both: the path is served, the translate, the rotate and the stack are computed,
    // and the second item is served again
    let moved = stacked.shapes(&Values::new().set("a", 5.0))?;
    assert_eq!(stacked.stats(), counts(3, 2, 4));
    assert_eq!(moved[0].path(), moved[1].path());
    Ok(())
}

// What a draw drops -------------------------------------------------------------------------------------------------

#[test]
fn entries_of_the_values_before_are_gone_after_a_draw() -> Outcome {
    let chain = six()?;
    for step in 0..20 {
        // each draw is a new `a`; the cache holds one entry per step, not one per value tried
        let values = Values::new().set("a", f64::from(step));
        assert_eq!(stats_of(&chain, &values)?.entries, 6, "draw {step}");
    }
    // gone: asking for an earlier value computes again
    stats_of(&chain, &Values::new().set("a", 3.0))?;
    assert_eq!(stats_of(&chain, &Values::new().set("a", 4.0))?.computed, 5);
    assert_eq!(stats_of(&chain, &Values::new().set("a", 3.0))?.computed, 5);
    Ok(())
}

#[test]
fn failed_draws_do_not_pile_up_entries() -> Outcome {
    // `a` is read by step 2 and `b` by step 3, which refuses 0: each failed draw computes step 2 at a new `a`
    let (a, b) = (slider("a", 0.0)?, slider("b", 1.0)?);
    let chain = square()?.translate(&a, 0.0).scale(&b);
    for step in 0..200 {
        let values = Values::new().set("a", f64::from(step)).set("b", 0.0);
        assert!(chain.shapes(&values).is_err(), "draw {step}");
        assert!(
            chain.stats().entries <= 3,
            "draw {step}: {} entries",
            chain.stats().entries
        );
    }
    // and the drawing still draws what a cold one does
    let good = Values::new().set("a", 7.0);
    assert_eq!(
        chain.shapes(&good)?,
        square()?
            .translate(&slider("a", 0.0)?, 0.0)
            .scale(&slider("b", 1.0)?)
            .shapes(&good)?
    );
    Ok(())
}

#[test]
fn steps_before_a_hit_stay_for_the_next_draw() -> Outcome {
    let chain = six()?;
    stats_of(&chain, &Values::new().set("a", 1.0).set("b", 1.0))?;
    // `b` changes: step 4 is served, steps 1 to 4 stay behind it, and steps 5 and 6 are new entries: six
    assert_eq!(
        stats_of(&chain, &Values::new().set("a", 1.0).set("b", 2.0))?,
        counts(2, 1, 6)
    );
    // so `b` changing again still finds step 4
    assert_eq!(
        stats_of(&chain, &Values::new().set("a", 1.0).set("b", 3.0))?,
        counts(2, 1, 6)
    );
    Ok(())
}

#[test]
fn the_drawings_a_served_step_was_made_of_stay_for_the_next_draw() -> Outcome {
    let a = slider("a", 0.0)?;
    // the path (one for both items), the left translate, the right rotate by `a`, the stack: four steps
    let left = square()?.translate(20.0, 0.0);
    let right = square()?.rotate(Deg(&a));
    let stacked = Drawing::new([left, right]);
    assert_eq!(stats_of(&stacked, &Values::new())?, counts(4, 1, 4));
    // served as a whole: nothing below the stack is looked at
    assert_eq!(stats_of(&stacked, &Values::new())?, counts(0, 1, 4));
    // `a` changes: the left translate and the path are still there, and served; the rotate and the stack are computed
    assert_eq!(stats_of(&stacked, &Values::new().set("a", 45.0))?, counts(2, 2, 4));
    Ok(())
}

// Every input of a step is in its key -------------------------------------------------------------------------------

/// Two drawings of one family (built on one base), with the values each is drawn at.
type Pair = (Drawing, Values, Drawing, Values);

/// Builds a pair on a base.
type Make = fn(&Drawing) -> Result<Pair, Error>;

/// Rows that differ in exactly one input of a step; each is built from a base of its own, so building one again gives
/// a drawing with a cache of its own to compare with.
fn pairs() -> Vec<(&'static str, Make)> {
    vec![
        ("an argument of the step", |base| {
            Ok((
                base.translate(1.0, 0.0),
                Values::new(),
                base.translate(2.0, 0.0),
                Values::new(),
            ))
        }),
        ("the step before", |base| {
            Ok((
                base.translate(1.0, 0.0).rotate(Deg(30.0)),
                Values::new(),
                base.translate(2.0, 0.0).rotate(Deg(30.0)),
                Values::new(),
            ))
        }),
        ("a drawing the step takes", |base| {
            Ok((
                Drawing::new([base.clone(), base.translate(1.0, 0.0)]),
                Values::new(),
                Drawing::new([base.clone(), base.translate(2.0, 0.0)]),
                Values::new(),
            ))
        }),
        ("the values of a with_values", |base| {
            let x = slider("x", 0.0)?;
            let read = base.translate(&x, 0.0);
            Ok((
                read.with_values(&Values::new().set("x", 1.0)),
                Values::new(),
                read.with_values(&Values::new().set("x", 2.0)),
                Values::new(),
            ))
        }),
        ("the default of a Param", |base| {
            Ok((
                base.translate(&slider("x", 1.0)?, 0.0),
                Values::new(),
                base.translate(&slider("x", 2.0)?, 0.0),
                Values::new(),
            ))
        }),
        ("a selection", |base| {
            let two = Drawing::new([base.clone(), base.translate(20.0, 0.0)]);
            Ok((
                two.only(Select::new().attribute("index", 0_u32)),
                Values::new(),
                two.only(Select::new().attribute("index", 1_u32)),
                Values::new(),
            ))
        }),
        ("the unit of a Param", |base| {
            // 400 wraps to 40 in a cyclic Param and is refused in a bounded one
            let cyclic = Param::new("t", 0.0, 0.0, 360.0)?.unit(bullterrier_vector::Unit::Deg)?;
            let bounded = Param::new("t", 0.0, 0.0, 360.0)?;
            let at = Values::new().set("t", 400.0);
            Ok((base.rotate(Deg(&cyclic)), at.clone(), base.rotate(Deg(&bounded)), at))
        }),
        ("a Param given a value", |base| {
            let x = slider("x", 0.0)?;
            Ok((
                base.translate(&x, 0.0),
                Values::new().set("x", 1.0),
                base.translate(&x, 0.0),
                Values::new().set("x", 2.0),
            ))
        }),
        ("a list given to a Param", |base| {
            let x = slider("x", 0.0)?;
            let two = Drawing::new([base.clone(), base.translate(0.0, 20.0)]).translate(&x, 0.0);
            Ok((
                two.clone(),
                Values::new().set("x", vec![1.0, 2.0]),
                two,
                Values::new().set("x", vec![1.0, 3.0]),
            ))
        }),
        ("a number against a list of it", |base| {
            let x = slider("x", 0.0)?;
            let two = Drawing::new([base.clone(), base.translate(0.0, 20.0)]).translate(&x, 0.0);
            Ok((
                two.clone(),
                Values::new().set("x", 1.0),
                two,
                Values::new().set("x", vec![1.0, 2.0]),
            ))
        }),
    ]
}

fn outcome(drawing: &Drawing, values: &Values) -> Result<Vec<Shape>, String> {
    drawing.shapes(values).map_err(|error| error.to_string())
}

#[test]
fn drawings_that_differ_in_one_input_never_share_a_result() -> Outcome {
    for (case, make) in pairs() {
        let (left, left_at, right, right_at) = make(&square()?)?;
        // a twin of the right drawing, built on a base of its own: its cache holds nothing of the left
        let (_, _, twin, twin_at) = make(&square()?)?;
        let first = outcome(&left, &left_at);
        let second = outcome(&right, &right_at);
        assert_ne!(first, second, "{case}: the row must tell the two apart");
        assert_eq!(second, outcome(&twin, &twin_at), "{case}: drawn after its sibling");
    }
    Ok(())
}

// Cached equals fresh -----------------------------------------------------------------------------------------------

/// A failed property case.
fn failed(error: &Error) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

proptest! {
    #[test]
    fn a_long_lived_drawing_draws_what_a_cold_one_does_at_every_edit(ops in chain(), edits in edits()) {
        let long = materialise(&ops).map_err(|error| failed(&error))?;
        let free = long.parameters().map_err(|error| failed(&error))?;
        for edit in &edits {
            let values = values_of(edit, &free);
            let cold = materialise(&ops).map_err(|error| failed(&error))?;
            prop_assert_eq!(outcome(&long, &values), outcome(&cold, &values));
        }
    }

    #[test]
    fn drawings_that_share_a_start_draw_what_a_cold_one_does_when_drawn_by_turns(
        start in chain(),
        left in chain(),
        right in chain(),
        edits in edits(),
    ) {
        // two drawings derived from one, so in one cache, drawn by turns at the same edits
        let family = materialise(&start).map_err(|error| failed(&error))?;
        let siblings = [
            (extend(family.clone(), &left).map_err(|error| failed(&error))?, [&start[..], &left[..]].concat()),
            (extend(family, &right).map_err(|error| failed(&error))?, [&start[..], &right[..]].concat()),
        ];
        for edit in &edits {
            for (drawing, ops) in &siblings {
                let free = drawing.parameters().map_err(|error| failed(&error))?;
                let values = values_of(edit, &free);
                let cold = materialise(ops).map_err(|error| failed(&error))?;
                prop_assert_eq!(outcome(drawing, &values), outcome(&cold, &values));
            }
        }
    }
}
