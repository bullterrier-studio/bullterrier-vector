//! Documents and drawings through JSON and back: the examples of docs/examples/json whose calls are all registered
//! load, and write the canonical text; a drawing written to JSON and read again draws the same shapes and writes the
//! same text.

mod common;

use bullterrier_vector::{
    Blend, Cap, Color, Deg, Drawing, Error, Expr, Inputs, Join, Param, Path, Rotate, ScaleBy, Select, Shape, Stroke,
    Unit, Values,
};
use common::Outcome;
use proptest::prelude::*;

const SERIES_BARS: &str = include_str!("../../../docs/examples/json/series_bars.json");

#[test]
fn the_series_bars_example_loads_and_writes_its_canonical_text() -> Outcome {
    let drawing = Drawing::from_json(SERIES_BARS, &Inputs::new())?;
    // the independent check of the bytes is tests/test_json.py, which builds the expected text with Python's `json`
    let canonical = drawing.to_json()?;
    assert_eq!(Drawing::from_json(&canonical, &Inputs::new())?.to_json()?, canonical);
    assert!(
        canonical.contains(r#""d1""#) && !canonical.contains("bars"),
        "the shared drawing is named d1"
    );
    Ok(())
}

#[test]
fn the_series_bars_example_draws_the_same_shapes_after_a_round_trip() -> Outcome {
    let loaded = Drawing::from_json(SERIES_BARS, &Inputs::new())?;
    let again = Drawing::from_json(&loaded.to_json()?, &Inputs::new())?;
    let shapes = loaded.shapes(&Values::new())?;
    assert_eq!(shapes, again.shapes(&Values::new())?);
    // twelve bars: the sixth in red, the others in ink
    assert_eq!(shapes.len(), 12);
    Ok(())
}

/// Every example of docs/examples/json.
const EXAMPLES: [(&str, &str); 9] = [
    (
        "cut_paper_one_frame",
        include_str!("../../../docs/examples/json/cut_paper_one_frame.json"),
    ),
    (
        "glyph_scatter",
        include_str!("../../../docs/examples/json/glyph_scatter.json"),
    ),
    ("halftone", include_str!("../../../docs/examples/json/halftone.json")),
    (
        "hatched_poster",
        include_str!("../../../docs/examples/json/hatched_poster.json"),
    ),
    (
        "kinetic_type",
        include_str!("../../../docs/examples/json/kinetic_type.json"),
    ),
    (
        "radial_repeat",
        include_str!("../../../docs/examples/json/radial_repeat.json"),
    ),
    ("ring", include_str!("../../../docs/examples/json/ring.json")),
    ("series_bars", SERIES_BARS),
    (
        "wobble_outline",
        include_str!("../../../docs/examples/json/wobble_outline.json"),
    ),
];

#[test]
fn every_example_whose_calls_are_registered_round_trips() -> Outcome {
    let mut loaded = Vec::new();
    for (name, text) in EXAMPLES {
        match Drawing::from_json(text, &Inputs::new()) {
            Ok(drawing) => {
                let written = drawing.to_json()?;
                let again = Drawing::from_json(&written, &Inputs::new())?;
                assert_eq!(again.to_json()?, written, "{name}");
                assert_eq!(drawing.shapes(&Values::new())?, again.shapes(&Values::new())?, "{name}");
                loaded.push(name);
            }
            // a call, or an argument of `text` (axes, width, fit), of a later batch: the example waits for it
            Err(Error::Format { message, .. })
                if message.contains("is not a call this library knows")
                    || message.contains("is not an argument of `text`") => {}
            Err(error) => return Err(format!("{name}: {error}").into()),
        }
    }
    assert!(loaded.contains(&"series_bars"), "{loaded:?}");
    Ok(())
}

// Random trees ------------------------------------------------------------------------------------------------------

/// A number in a random drawing: a literal, a Param (by index into `params()`), or arithmetic.
#[derive(Debug, Clone)]
enum Num {
    Lit(f64),
    Param(usize),
    Neg(Box<Num>),
    Binary(u8, Box<Num>, Box<Num>),
}

/// The Params random drawings read: a bounded px, a cyclic angle, a fraction and a plain number with a meaning.
fn params() -> Result<Vec<Param>, Error> {
    Ok(vec![
        Param::new("x", 5.0, 0.0, 100.0)?.unit(Unit::Px)?,
        Param::new("angle", 90.0, 0.0, 360.0)?
            .unit(Unit::Deg)?
            .meaning("a turn"),
        Param::new("t", 0.25, 0.0, 1.0)?.unit(Unit::Fraction)?,
        Param::new("k", 1.0, -10.0, 10.0)?.meaning("a \"gain\""),
    ])
}

fn literal() -> impl Strategy<Value = f64> {
    prop_oneof![
        (-20_i32..20).prop_map(f64::from),
        -500.0..500.0_f64,
        1e-3..1e3_f64,
        Just(0.1),
        Just(1e-7),
        Just(-0.0),
    ]
}

fn number() -> impl Strategy<Value = Num> {
    let leaf = prop_oneof![1 => literal().prop_map(Num::Lit), 2 => (0..4_usize).prop_map(Num::Param)];
    leaf.prop_recursive(3, 12, 2, |inner| {
        prop_oneof![
            inner.clone().prop_map(|number| Num::Neg(Box::new(number))),
            (0_u8..4, inner.clone(), inner).prop_map(|(op, left, right)| Num::Binary(
                op,
                Box::new(left),
                Box::new(right)
            )),
        ]
    })
}

fn expression(number: &Num, params: &[Param]) -> Expr {
    match number {
        Num::Lit(value) => Expr::from(*value),
        Num::Param(index) => Expr::from(&params[*index % params.len()]),
        Num::Neg(inner) => -expression(inner, params),
        Num::Binary(op, left, right) => {
            let (left, right) = (expression(left, params), expression(right, params));
            match op {
                0 => left + right,
                1 => left - right,
                2 => left * right,
                _ => left / right,
            }
        }
    }
}

/// A colour in either case, with or without an alpha.
fn colour() -> impl Strategy<Value = Color> {
    (
        0_u32..0x100_0000,
        prop_oneof![Just(""), Just("ff"), Just("FF"), Just("80"), Just("0a")],
        any::<bool>(),
    )
        .prop_filter_map("a colour", |(rgb, alpha, upper)| {
            let text = format!("#{rgb:06x}{alpha}");
            Color::parse(&if upper { text.to_uppercase() } else { text }).ok()
        })
}

/// One thing the pool of drawings grows by: a source, or a step on an earlier drawing (an index into the pool).
#[derive(Debug, Clone)]
enum Op {
    Rect(f64, f64, f64, f64, f64),
    Circle(f64, f64, f64),
    Star(f64, f64, f64, f64, i64),
    Line(f64, f64, f64, f64),
    Shape(usize),
    Stack(Vec<usize>, Option<(f64, f64)>),
    Translate(usize, Num, Num),
    Rotate(usize, Num, Option<(Num, Num)>),
    Scale(usize, Num, Option<Num>, Option<(f64, f64)>),
    Fill(usize, Option<Color>),
    Stroke(usize, Color, Option<f64>, Option<u8>, Option<u8>, Option<f64>),
    Opacity(usize, Num),
    Blend(usize, bool),
    Select(usize, bool, Vec<u32>),
    With(usize, f64),
}

fn coordinate() -> impl Strategy<Value = f64> {
    prop_oneof![-500.0..500.0_f64, (-50_i32..50).prop_map(f64::from)]
}

fn size() -> impl Strategy<Value = f64> {
    prop_oneof![1.0..300.0_f64, (1_i32..50).prop_map(f64::from)]
}

/// An op that makes a drawing from nothing but numbers.
fn path_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (coordinate(), coordinate(), size(), size(), 0.0..4.0_f64).prop_map(|(x, y, w, h, r)| Op::Rect(x, y, w, h, r)),
        (coordinate(), coordinate(), size()).prop_map(|(x, y, r)| Op::Circle(x, y, r)),
        (coordinate(), coordinate(), 20.0..90.0_f64, 5.0..19.0_f64, 3..9_i64)
            .prop_map(|(x, y, outer, inner, points)| Op::Star(x, y, outer, inner, points)),
        (coordinate(), coordinate(), coordinate(), coordinate()).prop_map(|(a, b, c, d)| Op::Line(a, b, c, d)),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    let pick = || 0..64_usize;
    prop_oneof![
        path_op(),
        pick().prop_map(Op::Shape),
        (prop::collection::vec(pick(), 0..4), prop::option::of((size(), size())))
            .prop_map(|(items, size)| Op::Stack(items, size)),
        (pick(), number(), number()).prop_map(|(at, dx, dy)| Op::Translate(at, dx, dy)),
        (pick(), number(), prop::option::of((number(), number())))
            .prop_map(|(at, angle, about)| Op::Rotate(at, angle, about)),
        (
            pick(),
            number(),
            prop::option::of(number()),
            prop::option::of((coordinate(), coordinate()))
        )
            .prop_map(|(at, x, y, about)| Op::Scale(at, x, y, about)),
        (pick(), prop::option::of(colour())).prop_map(|(at, color)| Op::Fill(at, color)),
        (
            pick(),
            colour(),
            prop::option::of(size()),
            prop::option::of(0_u8..3),
            prop::option::of(0_u8..3),
            prop::option::of(1.0..8.0_f64)
        )
            .prop_map(|(at, color, width, join, cap, limit)| Op::Stroke(at, color, width, join, cap, limit)),
        (pick(), number()).prop_map(|(at, alpha)| Op::Opacity(at, alpha)),
        (pick(), any::<bool>()).prop_map(|(at, multiply)| Op::Blend(at, multiply)),
        (pick(), any::<bool>(), prop::collection::vec(0_u32..4, 1..4))
            .prop_map(|(at, without, indexes)| Op::Select(at, without, indexes)),
        (pick(), 0.0..1.0_f64).prop_map(|(at, share)| Op::With(at, share)),
    ]
}

/// The drawings the ops build, each op on an earlier drawing (indexes wrap); an op that is not valid (a scale of 0, an
/// alpha of 2) adds nothing.
fn build_pool(ops: &[Op], params: &[Param]) -> Vec<Drawing> {
    let mut pool: Vec<Drawing> = Vec::new();
    for op in ops {
        let earlier = |at: usize| pool.get(at % pool.len().max(1)).cloned();
        let Some(next) = grow(op, params, earlier) else {
            continue;
        };
        if next.error().is_some() {
            continue;
        }
        pool.push(next);
    }
    pool
}

/// The drawing `op` makes, if it can be made.
fn grow(op: &Op, params: &[Param], earlier: impl Fn(usize) -> Option<Drawing>) -> Option<Drawing> {
    Some(match op {
        Op::Rect(left, top, width, height, radius) => {
            Drawing::from(Path::rect(*left, *top, *width, *height, *radius).ok()?)
        }
        Op::Circle(x, y, r) => Drawing::from(Path::circle((*x, *y), *r).ok()?),
        Op::Star(x, y, outer, inner, points) => Drawing::from(Path::star((*x, *y), *outer, *inner, *points).ok()?),
        Op::Line(a, b, c, d) => Drawing::from(Path::line((*a, *b), (*c, *d)).ok()?),
        Op::Shape(at) => {
            let shape = earlier(*at)?.shapes(&Values::new()).ok()?.pop()?;
            Drawing::from(shape)
        }
        Op::Stack(items, size) => {
            let items: Vec<Drawing> = items.iter().map(|at| earlier(*at)).collect::<Option<_>>()?;
            let stack = Drawing::new(items);
            size.map_or(stack.clone(), |(w, h)| stack.with_size(w, h))
        }
        Op::Translate(at, dx, dy) => earlier(*at)?.translate(expression(dx, params), expression(dy, params)),
        Op::Rotate(at, angle, about) => {
            let turn = Rotate::new(Deg(expression(angle, params)));
            let turn = match about {
                Some((x, y)) => turn.about((expression(x, params), expression(y, params))),
                None => turn,
            };
            earlier(*at)?.rotate(turn)
        }
        Op::Scale(at, x, y, about) => {
            let by = match y {
                Some(y) => ScaleBy::new((expression(x, params), expression(y, params))),
                None => ScaleBy::new(expression(x, params)),
            };
            let by = match about {
                Some(point) => by.about(*point),
                None => by,
            };
            earlier(*at)?.scale(by)
        }
        Op::Fill(at, color) => earlier(*at)?.fill(color.unwrap_or(Color::NONE)),
        Op::Stroke(at, color, width, join, cap, limit) => {
            let mut stroke = Stroke::new(*color);
            if let Some(width) = width {
                stroke = stroke.width(*width);
            }
            if let Some(join) = join {
                stroke = stroke.join([Join::Round, Join::Miter, Join::Bevel][usize::from(*join)]);
            }
            if let Some(cap) = cap {
                stroke = stroke.cap([Cap::Butt, Cap::Round, Cap::Square][usize::from(*cap)]);
            }
            if let Some(limit) = limit {
                stroke = stroke.miter_limit(*limit);
            }
            earlier(*at)?.stroke(stroke)
        }
        Op::Opacity(at, alpha) => earlier(*at)?.opacity(expression(alpha, params)),
        Op::Blend(at, multiply) => earlier(*at)?.blend(if *multiply { Blend::Multiply } else { Blend::Normal }),
        Op::Select(at, without, indexes) => {
            let select = Select::new().attribute("index", indexes.clone());
            let before = earlier(*at)?;
            if *without {
                before.without(select)
            } else {
                before.only(select)
            }
        }
        Op::With(at, share) => {
            let before = earlier(*at)?;
            let free = before.parameters().ok()?;
            let param = free.first()?;
            let value = param.min() + share * (param.max() - param.min());
            before.with_values(&Values::new().set(param.name(), value))
        }
    })
}

/// Values for every free Param of `drawing`: a third of the way into each range.
fn values_for(drawing: &Drawing) -> Result<Values, Error> {
    Ok(drawing.parameters()?.iter().fold(Values::new(), |values, param| {
        values.set(param.name(), param.min() + 0.37 * (param.max() - param.min()))
    }))
}

/// Whether two draws agree: the same shapes, or both refused (for a value the drawing cannot take).
fn same_draw(a: &Result<Vec<Shape>, Error>, b: &Result<Vec<Shape>, Error>) -> bool {
    match (a, b) {
        (Ok(a), Ok(b)) => a == b,
        (Err(_), Err(_)) => true,
        _ => false,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// A drawing built from random calls writes a document that reads back to a drawing with the same Params that
    /// draws the same shapes and writes the same text.
    #[test]
    fn a_random_tree_round_trips_to_the_same_text_params_and_shapes(
        first in path_op(),
        more in prop::collection::vec(op(), 0..9),
        roots in prop::collection::vec(0..64_usize, 1..4),
    ) {
        let params = params().map_err(|error| TestCaseError::fail(error.to_string()))?;
        let ops: Vec<Op> = std::iter::once(first).chain(more).collect();
        let pool = build_pool(&ops, &params);
        prop_assume!(!pool.is_empty());
        let root = Drawing::new(roots.iter().map(|at| pool[at % pool.len()].clone()));
        prop_assume!(root.error().is_none());
        let text = match root.to_json() {
            Ok(text) => text,
            // a divisor that is a literal 0 has no spelling in a document
            Err(Error::Parameter { parameter, .. }) if parameter == "expression" => return Err(TestCaseError::reject("a zero divisor")),
            Err(other) => return Err(TestCaseError::fail(format!("to_json: {other}"))),
        };
        let loaded = Drawing::from_json(&text, &Inputs::new()).map_err(|error| TestCaseError::fail(format!("{error}\n{text}")))?;
        prop_assert_eq!(loaded.to_json().map_err(|error| TestCaseError::fail(error.to_string()))?, text);
        prop_assert_eq!(loaded.parameters(), root.parameters());
        prop_assert!(same_draw(&root.shapes(&Values::new()), &loaded.shapes(&Values::new())));
        let values = values_for(&root).map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert!(same_draw(&root.shapes(&values), &loaded.shapes(&values)));
    }
}

#[test]
fn a_negative_zero_in_a_document_is_written_as_zero() -> Outcome {
    let text = r#"{"version":1,"root":[{"shape":{"path":"M0 0","attributes":{"level":-0,"origin":[-0,-0.5]}}},{"translate":{"dx":-0,"dy":-0}}]}"#;
    assert_eq!(
        Drawing::from_json(text, &Inputs::new())?.to_json()?,
        r#"{"root":[{"shape":{"attributes":{"level":0,"origin":[0,-0.5]},"path":"M0 0"}},{"translate":{"dx":0,"dy":0}}],"version":1}"#
    );
    Ok(())
}
