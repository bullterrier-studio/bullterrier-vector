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

use crate::common::fonts::fraunces;
use crate::common::{Outcome, build, log_uniform, parameter_refusal, step};
use bullterrier_vector::{
    Blend, Color, Deg, Drawing, Error, Expr, Kind, Param, Path, Point, Role, Rotate, Scale, ScaleBy, Select, Shape,
    Stroke, Text, Values, text, verbs,
};
use proptest::prelude::*;
use std::collections::BTreeSet;

/// A drawing made of a verb with one argument set from an expression, or the reason the row has no such argument.
type With = fn(&str, Expr) -> Result<Drawing, Error>;

/// The cases of a geometry verb on a path: the drawing that applies the verb to the path with the numbers given, and
/// what the `Path` method of the same name returns for them.
type Cases = fn(&Path, f64, f64) -> Vec<(&'static str, Drawing, Result<Path, Error>)>;

struct Row {
    /// The verb, as `verbs()` names it.
    verb: &'static str,
    /// A drawing that uses the verb, built through the typed API.
    sample: fn() -> Result<Drawing, Error>,
    /// The drawing that uses the verb with one number argument set, on a drawing of a square at the origin, so that
    /// the verb is the one step after the source.
    with: Option<With>,
    /// `(argument, value, accepted)`: the boundaries of each argument's range, both sides.
    ranges: &'static [(&'static str, f64, bool)],
    /// For a geometry verb: the verb against the `Path` method, and results written by hand.
    geometry: Option<Geometry>,
}

/// A path, the verb applied to a drawing of it, and the path data that must result.
type ByHand = (&'static str, fn(Drawing) -> Drawing, &'static str);

struct Geometry {
    cases: Cases,
    by_hand: &'static [ByHand],
}

const SQUARE: &str = "M0 0 L10 0 L10 10 L0 10 Z";

fn square() -> Result<Drawing, Error> {
    Ok(Drawing::from(Path::from_svg_path(SQUARE)?))
}

fn unknown(verb: &str, argument: &str) -> Error {
    Error::Parameter {
        parameter: argument.to_owned(),
        message: format!("the row of {verb} has no such argument"),
    }
}

fn blue() -> Color {
    Color::hex(0x2a_5b_ff)
}

#[expect(clippy::too_many_lines, reason = "a table: one row per verb")]
fn rows() -> Vec<Row> {
    vec![
        Row {
            verb: "path",
            sample: || square(),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "shape",
            sample: || {
                let shapes = square()?.fill(blue()).shapes(&Values::new())?;
                Ok(Drawing::from(shapes[0].clone()))
            },
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "stack",
            sample: || Ok(Drawing::new([square()?, square()?.translate(20.0, 0.0)])),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "text",
            sample: || Ok(text("Hi", &fraunces()?)),
            with: Some(|argument, value| {
                let options = Text::new(&fraunces()?);
                match argument {
                    "size" => Ok(text("Hi", options.size(value))),
                    "tracking" => Ok(text("Hi", options.tracking(value))),
                    "leading" => Ok(text("Hi", options.leading(value))),
                    _ => Err(unknown("text", argument)),
                }
            }),
            ranges: &[
                ("size", 1.0, true),
                ("size", 100_000.0, true),
                ("size", 0.0, false),
                ("size", -1.0, false),
                ("size", f64::NAN, false),
                ("size", f64::INFINITY, false),
                ("tracking", 0.0, true),
                ("tracking", -250.0, true),
                ("tracking", 1000.0, true),
                ("tracking", f64::NAN, false),
                ("tracking", f64::NEG_INFINITY, false),
                ("leading", 0.0, true),
                ("leading", -600.0, true),
                ("leading", 5000.0, true),
                ("leading", f64::NAN, false),
                ("leading", f64::INFINITY, false),
            ],
            geometry: None,
        },
        Row {
            verb: "translate",
            sample: || Ok(square()?.translate(3.0, 4.0)),
            with: Some(|argument, value| match argument {
                "dx" => Ok(square()?.translate(value, 0.0)),
                "dy" => Ok(square()?.translate(0.0, value)),
                _ => Err(unknown("translate", argument)),
            }),
            ranges: &[
                ("dx", 0.0, true),
                ("dx", 1e100, true),
                ("dx", -1e100, true),
                ("dx", 2e100, false),
                ("dx", -2e100, false),
                ("dx", f64::NAN, false),
                ("dx", f64::INFINITY, false),
                ("dy", 0.0, true),
                ("dy", 1e100, true),
                ("dy", -1e100, true),
                ("dy", 2e100, false),
                ("dy", -2e100, false),
                ("dy", f64::NAN, false),
                ("dy", f64::NEG_INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    vec![(
                        "by two numbers",
                        Drawing::from(path.clone()).translate(a, b),
                        path.translate(a, b),
                    )]
                },
                by_hand: &[("M1 2 L10 4.25 Z", |d| d.translate(3.5, -7.25), "M4.5 -5.25 L13.5 -3 Z")],
            }),
        },
        Row {
            verb: "rotate",
            sample: || Ok(square()?.rotate(Deg(30.0))),
            with: Some(|argument, value| match argument {
                "angle" => Ok(square()?.rotate(Deg(value))),
                _ => Err(unknown("rotate", argument)),
            }),
            ranges: &[
                ("angle", 0.0, true),
                ("angle", -720.5, true),
                ("angle", 1e300, true),
                ("angle", f64::NAN, false),
                ("angle", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let about = Point::new(b, -a);
                    vec![
                        (
                            "about the middle",
                            Drawing::from(path.clone()).rotate(Deg(a)),
                            path.rotate(a, None),
                        ),
                        (
                            "about a point",
                            Drawing::from(path.clone()).rotate(Rotate::new(Deg(a)).about(about)),
                            path.rotate(a, Some(about)),
                        ),
                    ]
                },
                // a quarter turn clockwise on the screen about the origin: (10, 0) goes to (0, 10)
                by_hand: &[(
                    SQUARE,
                    |d| d.rotate(Rotate::new(Deg(90.0)).about((0.0, 0.0))),
                    "M0 0 L0 10 L-10 10 L-10 0 Z",
                )],
            }),
        },
        Row {
            verb: "scale",
            sample: || Ok(square()?.scale(2.0)),
            with: Some(|argument, value| match argument {
                "by" => Ok(square()?.scale(value)),
                _ => Err(unknown("scale", argument)),
            }),
            ranges: &[
                ("by", 2.0, true),
                ("by", -2.0, true),
                ("by", 1e-300, true),
                ("by", 0.0, false),
                ("by", f64::NAN, false),
                ("by", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let about = Point::new(b, -a);
                    vec![
                        (
                            "by a number",
                            Drawing::from(path.clone()).scale(a),
                            path.scale(Scale::from(a), None),
                        ),
                        (
                            "by a pair about a point",
                            Drawing::from(path.clone()).scale(ScaleBy::new((a, -b.abs() - 1.0)).about(about)),
                            path.scale((a, -b.abs() - 1.0), Some(about)),
                        ),
                    ]
                },
                by_hand: &[(
                    "M1 2 L10 4 Z",
                    |d| d.scale(ScaleBy::new((2.0, -0.5)).about((0.0, 0.0))),
                    "M2 -1 L20 -2 Z",
                )],
            }),
        },
        Row {
            verb: "fill",
            sample: || Ok(square()?.fill(blue())),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "stroke",
            sample: || Ok(square()?.stroke(Stroke::new(blue()).width(3.0))),
            with: Some(|argument, value| match argument {
                "width" => Ok(square()?.stroke(Stroke::new(blue()).width(value))),
                "miter_limit" => Ok(square()?.stroke(Stroke::new(blue()).miter_limit(value))),
                _ => Err(unknown("stroke", argument)),
            }),
            ranges: &[
                ("width", 2.0, true),
                ("width", 1e-9, true),
                ("width", 0.0, false),
                ("width", -1.0, false),
                ("width", f64::NAN, false),
                ("miter_limit", 1.0, true),
                ("miter_limit", 1e6, true),
                ("miter_limit", 0.99, false),
                ("miter_limit", f64::NAN, false),
                ("miter_limit", f64::INFINITY, false),
            ],
            geometry: None,
        },
        Row {
            verb: "opacity",
            sample: || Ok(square()?.opacity(0.5)),
            with: Some(|argument, value| match argument {
                "alpha" => Ok(square()?.opacity(value)),
                _ => Err(unknown("opacity", argument)),
            }),
            ranges: &[
                ("alpha", 0.0, true),
                ("alpha", 0.5, true),
                ("alpha", 1.0, true),
                ("alpha", 1.000_000_1, false),
                ("alpha", -1e-9, false),
                ("alpha", f64::NAN, false),
                ("alpha", f64::INFINITY, false),
            ],
            geometry: None,
        },
        Row {
            verb: "blend",
            sample: || Ok(square()?.blend(Blend::Multiply)),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "only",
            sample: || {
                Ok(Drawing::new([square()?, square()?.translate(20.0, 0.0)]).only(Select::new().attribute("index", 1)))
            },
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "without",
            sample: || {
                Ok(Drawing::new([square()?, square()?.translate(20.0, 0.0)])
                    .without(Select::new().attribute("order", 0.0)))
            },
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "with_values",
            sample: || {
                let p = Param::new("p", 1.0, 0.0, 10.0)?;
                Ok(square()?.translate(&p, 0.0).with_values(&Values::new().set("p", 5.0)))
            },
            with: None,
            ranges: &[],
            geometry: None,
        },
    ]
}

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
            let when_drawn = with(argument, Expr::from(&carrier))?;
            let given = Values::new().set("carrier", *value);
            if *accepted {
                assert_eq!(at_call.error(), None, "{case}: at the call");
                drawn(&at_call)?;
                when_drawn
                    .shapes(&given)
                    .map_err(|error| format!("{case}: when drawn: {error}"))?;
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
                assert_eq!(when_drawn.error(), None, "{case}: a Param is not checked at the call");
                let (name, message) = parameter_refusal(when_drawn.shapes(&given))?;
                assert!(
                    name == expected && message.contains(&text),
                    "{case}: when drawn: {name}: {message}"
                );
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
            let drawing = apply(Drawing::from(Path::from_svg_path(start)?));
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
            for (case, drawing, expected) in (geometry.cases)(&path, a, b) {
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
