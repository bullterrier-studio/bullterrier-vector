//! A `Field` and a `Flow` in a document, written as the calls of formats.md: a field is a number (the same
//! everywhere) or `{"Field.ramp": {"start", "end"}}` or `{"Field.distance": {"drawing", "normalized"}}`; a flow is
//! `{"Flow.angle": {"angle"}}`, `{"Flow.noise": {"wavelength", "phase", "seed"}}`, `{"Flow.contour": {"drawing"}}` or
//! `{"Flow.mix": {"flow", "other", "fraction"}}`. A `drawing` here is the one path the field reads, written as the
//! path alone, `[{"path": "M0 0 ..."}]`; a Param or a per-item number in place of a number is refused. What a call
//! makes is what is written, and reading it back builds the same value.

use super::read::{at, number_at, reason, refuse};
use super::value::Json;
use crate::field::{FieldKind, FlowKind};
use crate::{Error, Field, Flow, Path, Point};
use std::collections::BTreeMap;

/// The one call of `json`: its word and its arguments, the arguments an object that has only `allowed`.
fn call<'a>(
    pointer: &str,
    json: &'a Json,
    words: &[&str],
    what: &str,
) -> Result<(&'a str, &'a BTreeMap<String, Json>, String), Error> {
    let listed = || words.join(", ");
    let Json::Object(outer) = json else {
        return Err(refuse(pointer, format!("must be {what}, not {}", json.describe())));
    };
    let mut entries = outer.iter();
    let (Some((word, args)), None) = (entries.next(), entries.next()) else {
        return Err(refuse(
            pointer,
            format!("must be an object with one call, one of {}", listed()),
        ));
    };
    if !words.contains(&word.as_str()) {
        return Err(refuse(
            &at(pointer, word),
            format!("is not a call of {what}; it is one of {}", listed()),
        ));
    }
    let here = at(pointer, word);
    match args {
        Json::Object(args) => Ok((word, args, here)),
        other => Err(refuse(
            &here,
            format!("must be an object of arguments, not {}", other.describe()),
        )),
    }
}

fn only(pointer: &str, args: &BTreeMap<String, Json>, allowed: &[&str]) -> Result<(), Error> {
    match args.keys().find(|key| !allowed.contains(&key.as_str())) {
        Some(key) => Err(refuse(
            &at(pointer, key),
            format!("is not an argument of this call, which has {}", allowed.join(", ")),
        )),
        None => Ok(()),
    }
}

fn number(pointer: &str, args: &BTreeMap<String, Json>, name: &str, default: Option<f64>) -> Result<f64, Error> {
    match (args.get(name), default) {
        (Some(Json::Number(value)), _) => Ok(*value),
        (Some(Json::Object(_)), _) => Err(refuse(
            &at(pointer, name),
            "must be a number: a field or a flow reads literal numbers, never a Param or a per-item number",
        )),
        (Some(other), _) => number_at(&at(pointer, name), other),
        (None, Some(default)) => Ok(default),
        (None, None) => Err(refuse(pointer, format!("is missing `{name}`"))),
    }
}

fn point(pointer: &str, args: &BTreeMap<String, Json>, name: &str) -> Result<Point, Error> {
    let here = at(pointer, name);
    match args.get(name) {
        Some(Json::Array(items)) if items.len() == 2 => Ok(Point::new(
            number(&here, &BTreeMap::from([("0".to_owned(), items[0].clone())]), "0", None)?,
            number(&here, &BTreeMap::from([("1".to_owned(), items[1].clone())]), "1", None)?,
        )),
        Some(_) => Err(refuse(&here, "must be two numbers [x, y]")),
        None => Err(refuse(pointer, format!("is missing `{name}`"))),
    }
}

/// The path of an argument `drawing`: the path alone, `[{"path": "..."}]`.
fn drawing_path(pointer: &str, args: &BTreeMap<String, Json>) -> Result<Path, Error> {
    let here = at(pointer, "drawing");
    let lone = match args.get("drawing") {
        Some(Json::Array(calls)) => match calls.as_slice() {
            [Json::Object(call)] if call.len() == 1 => call.get("path").map(|data| (data, format!("{here}/0/path"))),
            _ => None,
        },
        Some(_) => None,
        None => return Err(refuse(pointer, "is missing `drawing`")),
    };
    let Some((Json::String(data), data_at)) = lone else {
        return Err(refuse(
            &here,
            "must be one path, written [{\"path\": \"M0 0 ...\"}]: a field reads a path, not a drawing, in this version",
        ));
    };
    Path::from_svg_path(data).map_err(|error| refuse(&data_at, reason(&error)))
}

fn built<T>(pointer: &str, made: Result<T, Error>) -> Result<T, Error> {
    made.map_err(|error| refuse(pointer, reason(&error)))
}

/// The field a document holds at `pointer`.
pub(super) fn field(pointer: &str, json: &Json) -> Result<Field, Error> {
    if let Json::Number(value) = json {
        return built(pointer, Field::constant(*value));
    }
    let (word, args, here) = call(pointer, json, &["Field.ramp", "Field.distance"], "a number or a field")?;
    if word == "Field.ramp" {
        only(&here, args, &["start", "end"])?;
        return built(
            &here,
            Field::ramp(point(&here, args, "start")?, point(&here, args, "end")?),
        );
    }
    only(&here, args, &["drawing", "normalized"])?;
    let normalized = match args.get("normalized") {
        None => false,
        Some(Json::Bool(flag)) => *flag,
        Some(_) => return Err(refuse(&at(&here, "normalized"), "must be true or false")),
    };
    built(&here, Field::distance(&drawing_path(&here, args)?, normalized))
}

/// The flow a document holds at `pointer`.
pub(super) fn flow(pointer: &str, json: &Json) -> Result<Flow, Error> {
    let words = ["Flow.angle", "Flow.noise", "Flow.contour", "Flow.mix"];
    let (word, args, here) = call(pointer, json, &words, "a flow")?;
    match word {
        "Flow.angle" => {
            only(&here, args, &["angle"])?;
            built(&here, Flow::angle(number(&here, args, "angle", None)?))
        }
        "Flow.noise" => {
            only(&here, args, &["wavelength", "phase", "seed"])?;
            built(
                &here,
                Flow::noise(
                    number(&here, args, "wavelength", Some(400.0))?,
                    number(&here, args, "phase", Some(0.0))?,
                    number(&here, args, "seed", Some(0.0))?,
                ),
            )
        }
        "Flow.contour" => {
            only(&here, args, &["drawing"])?;
            built(&here, Flow::contour(&drawing_path(&here, args)?))
        }
        _ => {
            only(&here, args, &["flow", "other", "fraction"])?;
            let member = |name: &str| {
                args.get(name)
                    .ok_or_else(|| refuse(&here, format!("is missing `{name}`")))
            };
            let first = flow(&at(&here, "flow"), member("flow")?)?;
            let second = flow(&at(&here, "other"), member("other")?)?;
            built(&here, first.mix(&second, number(&here, args, "fraction", None)?))
        }
    }
}

fn pair(point: Point) -> Json {
    Json::Array(vec![Json::Number(point.x), Json::Number(point.y)])
}

fn path_call(path: &Path) -> Json {
    Json::Array(vec![Json::object([("path", Json::String(path.to_svg_path()))])])
}

/// `field` as a document writes it.
pub(super) fn field_json(field: &Field) -> Json {
    match field.kind() {
        FieldKind::Constant(value) => Json::Number(*value),
        FieldKind::Ramp { start, end } => Json::object([(
            "Field.ramp",
            Json::object([("start", pair(*start)), ("end", pair(*end))]),
        )]),
        FieldKind::Map(_) => Json::Null,
        FieldKind::Distance { path, normalized } => {
            let mut args = vec![("drawing", path_call(path))];
            if *normalized {
                args.push(("normalized", Json::Bool(true)));
            }
            Json::object([("Field.distance", Json::object(args))])
        }
    }
}

/// `flow` as a document writes it.
pub(super) fn flow_json(flow: &Flow) -> Json {
    match flow.kind() {
        FlowKind::Angle { degrees } => Json::object([("Flow.angle", Json::object([("angle", Json::Number(degrees))]))]),
        FlowKind::Noise {
            wavelength,
            phase,
            seed,
        } => Json::object([(
            "Flow.noise",
            Json::object([
                ("wavelength", Json::Number(wavelength)),
                ("phase", Json::Number(phase)),
                ("seed", Json::Number(seed)),
            ]),
        )]),
        FlowKind::Contour { path } => Json::object([("Flow.contour", Json::object([("drawing", path_call(&path))]))]),
        FlowKind::Mix { flow, other, fraction } => Json::object([(
            "Flow.mix",
            Json::object([
                ("flow", flow_json(&flow)),
                ("other", flow_json(&other)),
                ("fraction", Json::Number(fraction)),
            ]),
        )]),
    }
}
