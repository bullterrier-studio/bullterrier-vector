//! The contract every registered verb keeps, checked over a table with one row per verb. The rows are enrolled exactly:
//! `verbs()` and the table name the same verbs, so a verb without a row, or a row without a verb, fails by name. Every
//! other check runs over the rows, so a verb added later is checked as soon as it has its row. Ranges, sizes and
//! hand results are written out here, not read from the library.
//!
//! 1. It draws the same shapes every time, and an equal drawing built apart draws them too.
//! 2. It accepts the boundaries of the ranges its arguments declare and refuses what is past them, at the call and
//!    when drawn, naming the step, the argument and the value.
//! 3. It makes the same paths and styles at any design size.
//! 4. A geometry verb equals the `Path` method of the same name, for any path, and gives a result written by hand.
//!
//! The table is in `rows.rs`.

use crate::common::{Outcome, build, log_uniform, parameter_refusal, step};
use bullterrier_vector::{Drawing, Error, Expr, Kind, Param, Path, Reads, Role, Shape, Values, verbs};
use proptest::prelude::*;
use std::collections::BTreeSet;

mod rows;
mod rows_style;

use rows::*;

fn drawn(drawing: &Drawing) -> Result<Vec<Shape>, Error> {
    drawing.shapes(&Values::new())
}

// Enrolment ---------------------------------------------------------------------------------------------------------

#[test]
fn the_rows_and_the_registered_verbs_are_the_same_set() {
    let registered: Vec<&str> = verbs().map(|verb| verb.name).collect();
    let rows = rows();
    let rowed: Vec<&str> = rows.iter().map(|row| row.verb).collect();
    let (registered_set, rowed_set): (BTreeSet<_>, BTreeSet<_>) =
        (registered.iter().copied().collect(), rowed.iter().copied().collect());
    assert_eq!(
        registered.len(),
        registered_set.len(),
        "a verb is registered twice: {registered:?}"
    );
    assert_eq!(rowed.len(), rowed_set.len(), "a verb has two rows: {rowed:?}");
    let without_row: Vec<_> = registered_set.difference(&rowed_set).collect();
    let without_verb: Vec<_> = rowed_set.difference(&registered_set).collect();
    assert!(
        without_row.is_empty(),
        "registered verbs without a row: {without_row:?}"
    );
    assert!(
        without_verb.is_empty(),
        "rows without a registered verb: {without_verb:?}"
    );
}

#[test]
fn every_number_argument_has_range_rows_and_every_row_names_an_argument_of_its_verb() -> Outcome {
    for row in rows() {
        let verb = verbs().find(|verb| verb.name == row.verb).ok_or("no such verb")?;
        let numbers: BTreeSet<&str> = verb
            .args
            .iter()
            .filter(|arg| arg.kind == Kind::Number)
            .map(|arg| arg.name)
            .collect();
        let ranged: BTreeSet<&str> = row.ranges.iter().map(|(argument, ..)| *argument).collect();
        let missing: Vec<_> = numbers.difference(&ranged).collect();
        assert!(
            missing.is_empty(),
            "{}: Number arguments without range rows: {missing:?}",
            row.verb
        );
        let declared: BTreeSet<&str> = verb.args.iter().map(|arg| arg.name).collect();
        let stray: Vec<_> = ranged.difference(&declared).collect();
        assert!(
            stray.is_empty(),
            "{}: range rows for arguments the verb lacks: {stray:?}",
            row.verb
        );
        for argument in &ranged {
            assert!(
                row.with.is_some_and(|with| with(argument, Expr::from(1.0)).is_ok()),
                "{}: no way to set {argument}",
                row.verb
            );
        }
        // a row that names an argument accepts something and refuses something, or it checks nothing
        for argument in &ranged {
            let outcomes: BTreeSet<bool> = row
                .ranges
                .iter()
                .filter(|(name, ..)| name == argument)
                .map(|(_, _, accepted)| *accepted)
                .collect();
            assert_eq!(
                outcomes.len(),
                2,
                "{}.{argument}: needs a value accepted and a value refused",
                row.verb
            );
        }
    }
    Ok(())
}

// 1. Determinism ----------------------------------------------------------------------------------------------------

#[test]
fn a_verb_draws_the_same_shapes_every_time_and_an_equal_drawing_built_apart_does_too() -> Outcome {
    for row in rows() {
        let (first, apart) = ((row.sample)()?, (row.sample)()?);
        assert_eq!(first.error(), None, "{}", row.verb);
        let shapes = drawn(&first)?;
        assert!(
            !shapes.is_empty(),
            "{}: a sample that draws nothing checks nothing",
            row.verb
        );
        assert_eq!(shapes, drawn(&first)?, "{}: drawn twice", row.verb);
        assert_eq!(shapes, drawn(&apart)?, "{}: built apart", row.verb);
        assert_eq!(first.parameters()?, apart.parameters()?, "{}: parameters", row.verb);
    }
    Ok(())
}

// 2. Values within range --------------------------------------------------------------------------------------------

#[test]
fn a_verb_accepts_the_edges_of_its_ranges_and_refuses_past_them_at_the_call_and_when_drawn() -> Outcome {
    // a Param wide enough to carry any finite number
    let carrier = Param::new("carrier", 0.0, -1e308, 1e308)?;
    for row in rows() {
        let Some(with) = row.with else { continue };
        for (argument, value, accepted) in row.ranges {
            let case = format!("{}.{argument} = {value}", row.verb);
            let at_call = with(argument, Expr::from(*value))?;
            // a literal argument takes no Param: there is no drawing that reads one, so nothing is checked when drawn
            let literal = verbs().any(|verb| {
                verb.name == row.verb
                    && verb
                        .args
                        .iter()
                        .any(|arg| arg.name == *argument && arg.reads == Reads::Literal)
            });
            if literal {
                assert!(
                    with(argument, Expr::from(&carrier)).is_err(),
                    "{case}: a literal took a Param"
                );
            }
            let when_drawn = if literal {
                None
            } else {
                Some(with(argument, Expr::from(&carrier))?)
            };
            let given = Values::new().set("carrier", *value);
            if *accepted {
                assert_eq!(at_call.error(), None, "{case}: at the call");
                drawn(&at_call)?;
                if let Some(when_drawn) = &when_drawn {
                    when_drawn
                        .shapes(&given)
                        .map_err(|error| format!("{case}: when drawn: {error}"))?;
                }
            } else {
                // a source is the place `source (text)`, the one step of a chain after a square `step 1 (rotate)`
                let place = if verbs().any(|verb| verb.name == row.verb && verb.role == Role::Source) {
                    format!("source ({})", row.verb)
                } else {
                    format!("step 1 ({})", row.verb)
                };
                let expected = format!("{place}.{argument}");
                let text = format!("{value}");
                let held = at_call
                    .error()
                    .cloned()
                    .ok_or_else(|| format!("{case}: no error at the call"))?;
                let (name, message) = parameter_refusal(Err::<(), _>(held))?;
                assert!(
                    name == expected && message.contains(&text),
                    "{case}: at the call: {name}: {message}"
                );
                if let Some(when_drawn) = &when_drawn {
                    assert_eq!(when_drawn.error(), None, "{case}: a Param is not checked at the call");
                    let (name, message) = parameter_refusal(when_drawn.shapes(&given))?;
                    assert!(
                        name == expected && message.contains(&text),
                        "{case}: when drawn: {name}: {message}"
                    );
                }
            }
        }
    }
    Ok(())
}

// 3. Design size ----------------------------------------------------------------------------------------------------

#[test]
fn a_verb_makes_the_same_paths_and_styles_at_any_design_size() -> Outcome {
    for row in rows() {
        let sample = (row.sample)()?;
        let bare = drawn(&sample)?;
        for (width, height) in [(1.0, 1.0), (640.0, 480.0), (100_000.0, 3.0)] {
            let framed = Drawing::new([sample.clone()]).with_size(width, height);
            assert_eq!(framed.error(), None, "{}: {width} x {height}", row.verb);
            let shapes = drawn(&framed)?;
            assert_eq!(shapes.len(), bare.len(), "{}: {width} x {height}", row.verb);
            for (framed, bare) in shapes.iter().zip(&bare) {
                assert_eq!(framed.path(), bare.path(), "{}: {width} x {height}", row.verb);
                assert_eq!(framed.style(), bare.style(), "{}: {width} x {height}", row.verb);
            }
        }
    }
    Ok(())
}

// 4. Agreement with Path --------------------------------------------------------------------------------------------

#[test]
fn a_geometry_verb_gives_the_result_written_by_hand() -> Outcome {
    for row in rows() {
        let Some(geometry) = row.geometry else { continue };
        for (start, apply, expected) in geometry.by_hand {
            let drawing = apply(Drawing::from(Path::from_svg_path(start)?))?;
            assert_eq!(drawing.error(), None, "{}", row.verb);
            assert_eq!(drawn(&drawing)?[0].path().to_svg_path(), *expected, "{}", row.verb);
        }
    }
    Ok(())
}

proptest! {
    // The step is the Path method applied to the shape's path, for shapes and numbers of every size.
    #[test]
    fn a_geometry_verb_equals_the_path_method(
        start in (-1000.0f64..1000.0, -1000.0f64..1000.0),
        steps in proptest::collection::vec(step((-1000.0f64..1000.0, -1000.0f64..1000.0)), 0..8),
        numbers in (log_uniform(-3.0, 6.0), any::<bool>(), log_uniform(-3.0, 6.0), any::<bool>()),
    ) {
        let fail = |e: Error| TestCaseError::fail(e.to_string());
        let path = build(start, &steps).map_err(fail)?;
        let a = if numbers.1 { numbers.0 } else { -numbers.0 };
        let b = if numbers.3 { numbers.2 } else { -numbers.2 };
        for row in rows() {
            let Some(geometry) = row.geometry else { continue };
            for (case, drawing, expected) in (geometry.cases)(&path, a, b).map_err(fail)? {
                let drawn = drawing.shapes(&Values::new());
                match (drawn, expected) {
                    (Ok(shapes), Ok(expected)) => prop_assert_eq!(shapes[0].path(), &expected, "{}: {}", row.verb, case),
                    (Err(_), Err(_)) => {}
                    (drawn, expected) => prop_assert!(false, "{}: {}: drawing {:?}, path {:?}", row.verb, case, drawn.map(|s| s.len()), expected.map(|_| ())),
                }
            }
        }
    }
}
