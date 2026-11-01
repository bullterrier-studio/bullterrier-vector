//! The JSON Schema of the format, generated from the declarations of the registered verbs ([`crate::verbs`]): each
//! call's name, arguments, kinds, defaults and domains, set into the fixed envelope of `schema_fixed.json` (the
//! document's keys, Params, inputs, expressions and the words the format spells the same for every call). The
//! generator has no list of calls: a verb added to the registry appears in the schema, and a new `Kind` or `Domain`
//! does not compile until it is given a schema here.

use super::bare;
use super::parse::parse;
use super::value::Json;
use crate::limits::MAX_PARAMS;
use crate::shape::{Holds, SELECTABLE};
use crate::{Arg, Domain, Kind, Literal, Reads, Role, Verb};

/// The part of the schema that does not depend on a call.
const FIXED: &str = include_str!("schema_fixed.json");

/// The largest coordinate a path holds, which a `Domain::Coordinate` argument stays within.
// constant: the bound `Domain::Coordinate` documents and `geometry::coordinate` enforces
const COORDINATE: f64 = 1e100;

/// The schema of the drawing format for the calls in `verbs`, as canonical JSON text (keys sorted, no whitespace).
pub(super) fn generate<'a>(verbs: impl Iterator<Item = &'a Verb>) -> String {
    let verbs: Vec<&Verb> = verbs.collect();
    let mut schema = parse(FIXED).unwrap_or(Json::Null);
    set(&mut schema, &["properties", "params", "maxItems"], number(max_params()));
    set(&mut schema, &["$defs", "source"], call(&verbs, Role::Source));
    set(&mut schema, &["$defs", "step"], call(&verbs, Role::Step));
    schema.canonical()
}

/// `MAX_PARAMS` as a JSON number.
#[expect(clippy::cast_precision_loss, reason = "the limit is far below 2^53")]
fn max_params() -> f64 {
    MAX_PARAMS as f64
}

/// Sets the member at `path` of the object `json`, whose parents must exist; a path that leads nowhere changes nothing.
fn set(json: &mut Json, path: &[&str], value: Json) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    let mut at = Some(json);
    for key in parents {
        at = match at {
            Some(Json::Object(members)) => members.get_mut(*key),
            _ => None,
        };
    }
    if let Some(Json::Object(members)) = at {
        members.insert((*last).to_owned(), value);
    }
}

fn number(value: f64) -> Json {
    Json::Number(value)
}

fn reference(name: &str) -> Json {
    Json::object([("$ref", Json::String(format!("#/$defs/{name}")))])
}

fn typed(name: &str) -> Json {
    Json::object([("type", Json::text(name))])
}

fn any_of(branches: Vec<Json>) -> Json {
    Json::object([("anyOf", Json::Array(branches))])
}

/// A schema made of `base` with the keyword `key` set to `value`.
fn with(mut base: Json, key: &str, value: Json) -> Json {
    if let Json::Object(members) = &mut base {
        members.insert(key.to_owned(), value);
    }
    base
}

/// An array of exactly two items, `first` and `second`.
fn pair(first: Json, second: Json) -> Json {
    Json::object([
        ("type", Json::text("array")),
        ("prefixItems", Json::Array(vec![first, second])),
        ("minItems", number(2.0)),
        ("items", Json::Bool(false)),
    ])
}

/// The schema of a call position: an object of one call, `role` giving which calls may stand there, then the schema
/// of each call by name. A `drawing` reference may start a drawing.
fn call(verbs: &[&Verb], role: Role) -> Json {
    let mut names = Vec::new();
    let mut properties = Vec::new();
    if role == Role::Source {
        names.push(Json::text("drawing"));
        properties.push(("drawing", reference("name")));
    }
    for verb in verbs.iter().filter(|verb| verb.role == role) {
        names.push(Json::text(verb.name));
        properties.push((verb.name, verb_schema(verb)));
    }
    Json::object([
        ("type", Json::text("object")),
        ("minProperties", number(1.0)),
        ("maxProperties", number(1.0)),
        ("propertyNames", Json::object([("enum", Json::Array(names))])),
        ("properties", Json::object(properties)),
    ])
}

/// The schema of the value of `{verb: value}`: the one argument itself for a bare call, else an object of arguments.
fn verb_schema(verb: &Verb) -> Json {
    let described = |schema| with(schema, "description", Json::text(verb.summary));
    if let [only] = verb.args
        && bare(verb)
    {
        return described(argument(only));
    }
    let required: Vec<Json> = verb
        .args
        .iter()
        .filter(|arg| arg.default.is_none())
        .map(|arg| Json::text(arg.name))
        .collect();
    let mut schema = Json::object([
        ("type", Json::text("object")),
        ("additionalProperties", Json::Bool(false)),
        (
            "properties",
            Json::object(verb.args.iter().map(|arg| (arg.name, argument(arg)))),
        ),
    ]);
    if !required.is_empty() {
        schema = with(schema, "required", Json::Array(required));
    }
    described(schema)
}

/// The schema of one argument's value, with its default if it has one.
fn argument(arg: &Arg) -> Json {
    let schema = match arg.kind {
        Kind::Number => value(arg),
        Kind::Point | Kind::Pair => pair(value(arg), value(arg)),
        Kind::Factor => any_of(vec![value(arg), pair(value(arg), value(arg))]),
        Kind::Color => match arg.domain {
            Domain::Painting => reference("color"),
            _ => any_of(vec![reference("color"), Json::object([("const", Json::text("none"))])]),
        },
        Kind::Text => match arg.domain {
            Domain::Choice(words) => {
                Json::object([("enum", Json::Array(words.iter().map(|word| Json::text(word)).collect()))])
            }
            _ => typed("string"),
        },
        Kind::Path => reference("path_data"),
        Kind::Shape => shape(),
        Kind::Drawings => Json::object([("type", Json::text("array")), ("items", reference("drawing"))]),
        Kind::Attributes => selection(),
        Kind::Font => reference("name"),
        Kind::Axes => tagged(value(arg)),
        Kind::Features => tagged(whole()),
        Kind::Values => Json::object([
            ("type", Json::text("object")),
            ("propertyNames", reference("param_name")),
            (
                "additionalProperties",
                any_of(vec![
                    typed("number"),
                    Json::object([("type", Json::text("array")), ("items", typed("number"))]),
                ]),
            ),
        ]),
    };
    match arg.default {
        Some(Literal::Number(default)) => with(schema, "default", number(default)),
        Some(Literal::Text(default)) => with(schema, "default", Json::text(default)),
        Some(Literal::Unset) | None => schema,
    }
}

/// An object whose keys are font tags and whose values are `value`.
fn tagged(value: Json) -> Json {
    Json::object([
        ("type", Json::text("object")),
        ("propertyNames", reference("tag")),
        ("additionalProperties", value),
    ])
}

/// One number of an argument: a literal within the domain, or an expression where the argument reads one per shape.
fn value(arg: &Arg) -> Json {
    let bounds = match arg.domain {
        Domain::Any | Domain::Finite | Domain::Choice(_) | Domain::Painting => Vec::new(),
        Domain::Coordinate => vec![("minimum", number(-COORDINATE)), ("maximum", number(COORDINATE))],
        Domain::NonZero => vec![("not", Json::object([("const", number(0.0))]))],
        Domain::Positive => vec![("exclusiveMinimum", number(0.0))],
        Domain::AtLeast(least) => vec![("minimum", number(least))],
        Domain::Fraction => vec![("minimum", number(0.0)), ("maximum", number(1.0))],
        Domain::PositiveUpTo(most) => vec![("exclusiveMinimum", number(0.0)), ("maximum", number(most))],
    };
    let literal = bounds
        .into_iter()
        .fold(typed("number"), |schema, (key, bound)| with(schema, key, bound));
    match arg.reads {
        Reads::Literal => literal,
        Reads::PerItem | Reads::Once => any_of(vec![literal, reference("expression")]),
    }
}

/// A whole number from 0 to the largest an attribute or a feature holds (`u32`).
fn whole() -> Json {
    Json::object([
        ("type", Json::text("integer")),
        ("minimum", number(0.0)),
        ("maximum", number(f64::from(u32::MAX))),
    ])
}

/// One character, counted in code points.
fn character() -> Json {
    Json::object([
        ("type", Json::text("string")),
        ("minLength", number(1.0)),
        ("maxLength", number(1.0)),
    ])
}

/// The schema of one attribute's value.
fn attribute(holds: Holds) -> Json {
    match holds {
        Holds::Whole => whole(),
        Holds::Real => typed("number"),
        Holds::Char => character(),
        Holds::Point => pair(typed("number"), typed("number")),
    }
}

/// The value of a `shape` source: a path and the attributes its generators wrote (`index` is the shape's own).
fn shape() -> Json {
    let attributes = SELECTABLE
        .iter()
        .filter(|(name, _)| *name != "index")
        .map(|(name, holds)| (*name, attribute(*holds)));
    Json::object([
        ("type", Json::text("object")),
        ("additionalProperties", Json::Bool(false)),
        (
            "properties",
            Json::object([
                ("path", reference("path_data")),
                (
                    "attributes",
                    Json::object([
                        ("type", Json::text("object")),
                        ("additionalProperties", Json::Bool(false)),
                        ("properties", Json::object(attributes)),
                    ]),
                ),
            ]),
        ),
        ("required", Json::Array(vec![Json::text("path")])),
    ])
}

/// The value of `only` and `without`: each attribute to the value a shape's must equal, or a list of values, any of
/// which will do; an empty list matches no shape.
fn selection() -> Json {
    let attributes = SELECTABLE.iter().map(|(name, holds)| {
        let one = attribute(*holds);
        let list = Json::object([("type", Json::text("array")), ("items", one.clone())]);
        (*name, any_of(vec![one, list]))
    });
    Json::object([
        ("type", Json::text("object")),
        ("additionalProperties", Json::Bool(false)),
        ("properties", Json::object(attributes)),
    ])
}
