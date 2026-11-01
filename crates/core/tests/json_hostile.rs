//! `Drawing::from_json` on hostile input: arbitrary bytes, arbitrary JSON, and JSON shaped like documents return a
//! drawing or an `Error::Format`, never panic; and a document that is accepted writes and reads back to the same text.

mod common;

use bullterrier_vector::{Drawing, Error, Inputs, Values};
use proptest::prelude::*;

/// What `from_json` must do with any text: refuse it with a format error at a JSON Pointer, or give a drawing that
/// writes a document which reads back to a drawing that writes the same text and draws the same.
fn checked(text: &str) -> Result<(), TestCaseError> {
    checked_with(text, &Inputs::new())
}

/// As `checked`, with `inputs` supplied; an input the caller gave wrongly is a parameter error naming `inputs.` and the
/// input.
fn checked_with(text: &str, inputs: &Inputs) -> Result<(), TestCaseError> {
    match Drawing::from_json(text, inputs) {
        Err(Error::Format { pointer, .. }) => {
            prop_assert!(
                pointer.is_empty() || pointer.starts_with('/'),
                "not a JSON Pointer: {pointer:?}"
            );
        }
        Err(Error::Parameter { parameter, .. }) if parameter.starts_with("inputs.") => {}
        Err(other) => return Err(TestCaseError::fail(format!("not a format error: {other}"))),
        Ok(drawing) => {
            let written = drawing
                .to_json()
                .map_err(|error| TestCaseError::fail(format!("to_json: {error}\n{text}")))?;
            let again = Drawing::from_json(&written, inputs)
                .map_err(|error| TestCaseError::fail(format!("the canonical text is refused: {error}\n{written}")))?;
            prop_assert_eq!(
                again
                    .to_json()
                    .map_err(|error| TestCaseError::fail(error.to_string()))?,
                written
            );
            prop_assert_eq!(drawing.parameters(), again.parameters());
            // drawing may be refused (a value for a Param, a selection of nothing): both must agree
            prop_assert_eq!(
                drawing.shapes(&Values::new()).is_ok(),
                again.shapes(&Values::new()).is_ok()
            );
        }
    }
    Ok(())
}

/// A JSON value, rendered from parts so that it can be wrong in ways a serializer would not write.
#[derive(Debug, Clone)]
enum Val {
    Null,
    Bool(bool),
    /// The text of a number.
    Number(&'static str),
    /// The text between the quotes, already escaped.
    Text(&'static str),
    Array(Vec<Val>),
    /// Members in order, keys possibly repeated.
    Object(Vec<(&'static str, Val)>),
}

impl Val {
    fn render(&self, into: &mut String) {
        match self {
            Self::Null => into.push_str("null"),
            Self::Bool(value) => into.push_str(if *value { "true" } else { "false" }),
            Self::Number(text) => into.push_str(text),
            Self::Text(text) => {
                into.push('"');
                into.push_str(text);
                into.push('"');
            }
            Self::Array(items) => {
                into.push('[');
                for (at, item) in items.iter().enumerate() {
                    if at > 0 {
                        into.push(',');
                    }
                    item.render(into);
                }
                into.push(']');
            }
            Self::Object(members) => {
                into.push('{');
                for (at, (key, value)) in members.iter().enumerate() {
                    if at > 0 {
                        into.push(',');
                    }
                    into.push('"');
                    into.push_str(key);
                    into.push_str("\":");
                    value.render(into);
                }
                into.push('}');
            }
        }
    }
}

const KEYS: [&str; 52] = [
    "version",
    "params",
    "inputs",
    "drawings",
    "root",
    "name",
    "default",
    "min",
    "max",
    "unit",
    "meaning",
    "path",
    "shape",
    "stack",
    "items",
    "size",
    "translate",
    "dx",
    "dy",
    "rotate",
    "angle",
    "about",
    "scale",
    "by",
    "fill",
    "color",
    "stroke",
    "width",
    "join",
    "cap",
    "miter_limit",
    "opacity",
    "alpha",
    "blend",
    "mode",
    "only",
    "without",
    "with_values",
    "drawing",
    "param",
    "add",
    "sub",
    "mul",
    "div",
    "neg",
    "index",
    "x",
    "d1",
    "d2",
    "kind",
    "sha256",
    "attributes",
];

const NUMBERS: [&str; 14] = [
    "0",
    "-0",
    "1",
    "2",
    "3.5",
    "1e2",
    "-1",
    "0.5",
    "1e999",
    "100000",
    "360",
    "1.5e-7",
    "0.25",
    "9007199254740993",
];

const TEXTS: [&str; 18] = [
    "",
    "x",
    "#ff0000",
    "#FF000080",
    "none",
    "M0 0 L10 0 L10 10 Z",
    "M0 0 X",
    "m0 0 l5 5 z",
    "round",
    "miter",
    "multiply",
    "normal",
    "d1",
    "d2",
    "deg",
    "px",
    "\\ud800",
    "\\u00e9",
];

fn leaf() -> impl Strategy<Value = Val> {
    prop_oneof![
        Just(Val::Null),
        any::<bool>().prop_map(Val::Bool),
        prop::sample::select(&NUMBERS[..]).prop_map(Val::Number),
        prop::sample::select(&TEXTS[..]).prop_map(Val::Text),
    ]
}

fn value() -> impl Strategy<Value = Val> {
    leaf().prop_recursive(6, 60, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..5).prop_map(Val::Array),
            prop::collection::vec((prop::sample::select(&KEYS[..]), inner), 0..5).prop_map(Val::Object),
        ]
    })
}

/// A value shaped like a document: its keys, with the version right more often than not.
fn document() -> impl Strategy<Value = Val> {
    (
        prop::option::weighted(0.9, Just(Val::Number("1"))),
        prop::option::of(value()),
        prop::option::of(value()),
        prop::option::of(value()),
        prop::option::weighted(0.9, value()),
    )
        .prop_map(|(version, params, inputs, drawings, root)| {
            let mut members = Vec::new();
            for (key, part) in [
                ("version", version),
                ("params", params),
                ("inputs", inputs),
                ("drawings", drawings),
                ("root", root),
            ] {
                members.extend(part.map(|part| (key, part)));
            }
            Val::Object(members)
        })
}

/// A call-like object: a key of the vocabulary and arguments of the vocabulary, in a root.
fn calls() -> impl Strategy<Value = Val> {
    let call = (prop::sample::select(&KEYS[..]), value()).prop_map(|(key, body)| Val::Object(vec![(key, body)]));
    prop::collection::vec(call, 0..6).prop_map(|calls| {
        let mut root = vec![Val::Object(vec![("path", Val::Text("M0 0 L10 0 L10 10 Z"))])];
        root.extend(calls);
        Val::Object(vec![("version", Val::Number("1")), ("root", Val::Array(root))])
    })
}

/// A number, or an expression of the Param `p`, as an argument of a plausible call.
fn number() -> impl Strategy<Value = Val> {
    let param = || Val::Object(vec![("param", Val::Text("p"))]);
    prop_oneof![
        3 => prop::sample::select(&NUMBERS[..]).prop_map(Val::Number),
        1 => Just(param()),
        1 => prop::sample::select(&NUMBERS[..]).prop_map(move |n| Val::Object(vec![("mul", Val::Array(vec![Val::Number(n), param()]))])),
        1 => Just(Val::Object(vec![("neg", param())])),
        1 => Just(Val::Object(vec![("div", Val::Array(vec![param(), Val::Number("0")]))])),
    ]
}

/// A call of the registered verbs with arguments that are right more often than not.
fn plausible_call() -> impl Strategy<Value = Val> {
    let word = |choices: &'static [&'static str]| prop::sample::select(choices).prop_map(Val::Text);
    let point = || (number(), number()).prop_map(|(x, y)| Val::Array(vec![x, y]));
    let one = |verb: &'static str, body: Val| Val::Object(vec![(verb, body)]);
    prop_oneof![
        (number(), number()).prop_map(move |(dx, dy)| one("translate", Val::Object(vec![("dx", dx), ("dy", dy)]))),
        (number(), prop::option::of(point())).prop_map(move |(angle, about)| {
            let mut body = vec![("angle", angle)];
            body.extend(about.map(|about| ("about", about)));
            one("rotate", Val::Object(body))
        }),
        (prop_oneof![number(), point()], prop::option::of(point())).prop_map(move |(by, about)| {
            let mut body = vec![("by", by)];
            body.extend(about.map(|about| ("about", about)));
            one("scale", Val::Object(body))
        }),
        word(&["#ff0000", "#FF000080", "none", "red"])
            .prop_map(move |color| one("fill", Val::Object(vec![("color", color)]))),
        (
            word(&["#00ff00", "none"]),
            prop::option::of(number()),
            prop::option::of(word(&["round", "miter", "bevel", "sharp"]))
        )
            .prop_map(move |(color, width, join)| {
                let mut body = vec![("color", color)];
                body.extend(width.map(|width| ("width", width)));
                body.extend(join.map(|join| ("join", join)));
                one("stroke", Val::Object(body))
            }),
        number().prop_map(move |alpha| one("opacity", Val::Object(vec![("alpha", alpha)]))),
        word(&["normal", "multiply", "screen"]).prop_map(move |mode| one("blend", Val::Object(vec![("mode", mode)]))),
        (
            prop::sample::select(&["only", "without"][..]),
            prop_oneof![
                number(),
                prop::collection::vec(prop::sample::select(&NUMBERS[..]).prop_map(Val::Number), 0..3)
                    .prop_map(Val::Array),
            ]
        )
            .prop_map(|(verb, index)| Val::Object(vec![(verb, Val::Object(vec![("index", index)]))])),
        prop::sample::select(&NUMBERS[..])
            .prop_map(move |n| one("with_values", Val::Object(vec![("p", Val::Number(n))]))),
    ]
}

/// A document of a path, plausible calls, perhaps a Param `p` and a shared drawing, in a stack or not.
fn plausible() -> impl Strategy<Value = Val> {
    let path = || Val::Object(vec![("path", Val::Text("M0 0 L10 0 L10 10 Z"))]);
    let param = Val::Object(vec![
        ("name", Val::Text("p")),
        ("default", Val::Number("0")),
        ("min", Val::Number("-1")),
        ("max", Val::Number("1")),
        ("unit", Val::Text("")),
        ("meaning", Val::Text("")),
    ]);
    (
        prop::collection::vec(plausible_call(), 0..5),
        prop::collection::vec(plausible_call(), 0..3),
        prop::option::of(Just(Val::Number("1"))),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(move |(calls, shared, size, stacked, with_param)| {
            let chain = |steps: Vec<Val>| {
                let mut chain = vec![path()];
                chain.extend(steps);
                Val::Array(chain)
            };
            let mut members = vec![("version", Val::Number("1"))];
            if with_param {
                members.push(("params", Val::Array(vec![param.clone()])));
            }
            members.push(("drawings", Val::Object(vec![("d1", chain(shared))])));
            let using = Val::Array(vec![Val::Object(vec![("drawing", Val::Text("d1"))])]);
            let root = if stacked {
                let mut body = vec![("items", Val::Array(vec![using, chain(calls)]))];
                body.extend(size.map(|_| ("size", Val::Array(vec![Val::Number("100000"), Val::Number("2")]))));
                Val::Array(vec![Val::Object(vec![("stack", Val::Object(body))])])
            } else {
                chain(calls)
            };
            members.push(("root", root));
            Val::Object(members)
        })
}

/// A document of one `text` call whose arguments, and the declaration of its inputs, are right more often than not.
fn text_documents() -> impl Strategy<Value = (Val, bool)> {
    let word = |choices: &'static [&'static str]| prop::sample::select(choices).prop_map(Val::Text);
    let point = || (number(), number()).prop_map(|(x, y)| Val::Array(vec![x, y]));
    let argument = prop_oneof![
        word(&["Hi", "", "fi\\nAV", "\\u00e9", "\\ud800"]).prop_map(|content| ("content", content)),
        word(&["f", "g", "", "x y"]).prop_map(|font| ("font", font)),
        number().prop_map(|size| ("size", size)),
        number().prop_map(|tracking| ("tracking", tracking)),
        number().prop_map(|leading| ("leading", leading)),
        word(&["left", "center", "right", "middle"]).prop_map(|align| ("align", align)),
        point().prop_map(|anchor| ("anchor", anchor)),
    ];
    let input = || {
        (
            prop::sample::select(&["font", "map", "other"][..]),
            prop::option::of(prop::sample::select(
                &[
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "short",
                ][..],
            )),
        )
            .prop_map(|(kind, sha256)| {
                let mut members = vec![("kind", Val::Text(kind))];
                members.extend(sha256.map(|sha256| ("sha256", Val::Text(sha256))));
                Val::Object(members)
            })
    };
    (
        prop::collection::vec(argument, 0..7),
        prop::option::of(input()),
        prop::option::of(input()),
        any::<bool>(),
    )
        .prop_map(|(arguments, f, g, supplies_both)| {
            let mut members = vec![("version", Val::Number("1"))];
            let declared: Vec<_> = [("f", f), ("g", g)]
                .into_iter()
                .filter_map(|(name, input)| input.map(|input| (name, input)))
                .collect();
            if !declared.is_empty() {
                members.push(("inputs", Val::Object(declared)));
            }
            // each argument once, the last one written winning: a repeated key is refused before anything else is read
            let mut body = std::collections::BTreeMap::from([("font", Val::Text("f"))]);
            body.extend(arguments);
            members.push((
                "root",
                Val::Array(vec![Val::Object(vec![(
                    "text",
                    Val::Object(body.into_iter().collect()),
                )])]),
            ));
            (Val::Object(members), supplies_both)
        })
}

fn rendered(value: &Val) -> String {
    let mut text = String::new();
    value.render(&mut text);
    text
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Arbitrary bytes, as far as they are text.
    #[test]
    fn arbitrary_bytes_give_a_drawing_or_a_format_error(bytes in prop::collection::vec(any::<u8>(), 0..400)) {
        checked(&String::from_utf8_lossy(&bytes))?;
    }

    /// Arbitrary text, and text made of the pieces of JSON.
    #[test]
    fn arbitrary_text_gives_a_drawing_or_a_format_error(text in "\\PC{0,300}") {
        checked(&text)?;
    }

    #[test]
    fn pieces_of_json_give_a_drawing_or_a_format_error(
        pieces in prop::collection::vec(
            prop::sample::select(&["{", "}", "[", "]", ",", ":", "\"", "\\", "u", "-", "e", "0", "1", ".", "null", "true",
                "\"version\"", "\"root\"", "\"path\"", " ", "\n", "\\ud800", "1e999"][..]),
            0..80,
        )
    ) {
        checked(&pieces.concat())?;
    }

    /// Arbitrary JSON values, written with keys of the format's vocabulary.
    #[test]
    fn arbitrary_json_gives_a_drawing_or_a_format_error(value in value()) {
        checked(&rendered(&value))?;
    }

    /// JSON shaped like documents, which gets past the envelope more often.
    #[test]
    fn documents_with_random_parts_give_a_drawing_or_a_format_error(value in document()) {
        checked(&rendered(&value))?;
    }

    /// A root of a path and random calls: the calls of the format, with random arguments.
    #[test]
    fn random_calls_give_a_drawing_or_a_format_error(value in calls()) {
        checked(&rendered(&value))?;
    }

    /// A `text` call and its inputs, supplied rightly or not.
    #[test]
    fn text_documents_give_a_drawing_a_format_error_or_a_refused_input((value, both) in text_documents()) {
        let mut inputs = Inputs::new().set("f", common::fonts::FRAUNCES);
        if both {
            inputs = inputs.set("g", common::fonts::FRAUNCES);
        }
        checked_with(&rendered(&value), &inputs)?;
    }

    /// Plausible documents, which reach the checks that relate values across the document.
    #[test]
    fn plausible_documents_give_a_drawing_or_a_format_error(value in plausible()) {
        checked(&rendered(&value))?;
    }
}

#[test]
fn text_nested_far_past_the_limit_is_refused_not_recursed_into() {
    for levels in [129, 1000, 100_000] {
        for (open, close) in [("[", "]"), ("{\"a\":", "}")] {
            let text = format!("{}1{}", open.repeat(levels), close.repeat(levels));
            assert!(
                matches!(Drawing::from_json(&text, &Inputs::new()), Err(Error::Format { .. })),
                "{levels} levels of {open}"
            );
        }
    }
}
