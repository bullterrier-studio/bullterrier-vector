//! A `Field` and a `Flow` in a document. A field is a number (the same everywhere) or an object with `field`:
//! `"ramp"` (`from`, `to`) or `"distance"` (`path`, `normalized`). A flow is an object with `flow`: `"angle"`
//! (`angle`), `"noise"` (`wavelength`, `phase`, `seed`), `"contour"` (`path`) or `"mix"` (`parts`: flows, each with a
//! `weight`). What a call makes is what is written: reading it back builds the same value.

use super::read::{at, number_at, reason, refuse, string};
use super::value::Json;
use crate::field::{FieldKind, FlowKind};
use crate::{Error, Field, Flow, Path, Point};
use std::collections::BTreeMap;

fn members<'a>(pointer: &str, json: &'a Json, allowed: &[&str], what: &str) -> Result<&'a BTreeMap<String, Json>, Error> {
    let Json::Object(members) = json else {
        return Err(refuse(pointer, format!("must be {what}, not {}", json.describe())));
    };
    match members.keys().find(|key| !allowed.contains(&key.as_str())) {
        Some(key) => Err(refuse(
            &at(pointer, key),
            format!("is not a member of {what}, which has {}", allowed.join(", ")),
        )),
        None => Ok(members),
    }
}

fn number(pointer: &str, members: &BTreeMap<String, Json>, name: &str, default: Option<f64>) -> Result<f64, Error> {
    match (members.get(name), default) {
        (Some(json), _) => number_at(&at(pointer, name), json),
        (None, Some(default)) => Ok(default),
        (None, None) => Err(refuse(pointer, format!("is missing `{name}`"))),
    }
}

fn point(pointer: &str, members: &BTreeMap<String, Json>, name: &str) -> Result<Point, Error> {
    let here = at(pointer, name);
    match members.get(name) {
        Some(Json::Array(items)) if items.len() == 2 => Ok(Point::new(
            number_at(&format!("{here}/0"), &items[0])?,
            number_at(&format!("{here}/1"), &items[1])?,
        )),
        Some(_) => Err(refuse(&here, "must be two numbers [x, y]")),
        None => Err(refuse(pointer, format!("is missing `{name}`"))),
    }
}

fn path(pointer: &str, members: &BTreeMap<String, Json>) -> Result<Path, Error> {
    let here = at(pointer, "path");
    let data = members
        .get("path")
        .ok_or_else(|| refuse(pointer, "is missing `path`"))
        .and_then(|json| string(&here, json))?;
    Path::from_svg_path(data).map_err(|error| refuse(&here, reason(&error)))
}

fn built<T>(pointer: &str, made: Result<T, Error>) -> Result<T, Error> {
    made.map_err(|error| refuse(pointer, reason(&error)))
}

/// The field a document holds at `pointer`.
pub(super) fn field(pointer: &str, json: &Json) -> Result<Field, Error> {
    if let Json::Number(value) = json {
        return built(pointer, Field::constant(*value));
    }
    let members = members(pointer, json, &["field", "from", "to", "path", "normalized"], "a number or a field object")?;
    match members.get("field") {
        Some(Json::String(kind)) if kind == "ramp" => {
            members_only(pointer, members, &["field", "from", "to"])?;
            built(pointer, Field::ramp(point(pointer, members, "from")?, point(pointer, members, "to")?))
        }
        Some(Json::String(kind)) if kind == "distance" => {
            members_only(pointer, members, &["field", "path", "normalized"])?;
            let normalized = match members.get("normalized") {
                None => false,
                Some(Json::Bool(flag)) => *flag,
                Some(_) => return Err(refuse(&at(pointer, "normalized"), "must be true or false")),
            };
            built(pointer, Field::distance(&path(pointer, members)?, normalized))
        }
        Some(_) => Err(refuse(&at(pointer, "field"), "must be \"ramp\" or \"distance\"")),
        None => Err(refuse(pointer, "is missing `field`: \"ramp\" or \"distance\"")),
    }
}

/// Refuses a member that this kind does not have.
fn members_only(pointer: &str, members: &BTreeMap<String, Json>, allowed: &[&str]) -> Result<(), Error> {
    match members.keys().find(|key| !allowed.contains(&key.as_str())) {
        Some(key) => Err(refuse(&at(pointer, key), format!("is not a member of this kind, which has {}", allowed.join(", ")))),
        None => Ok(()),
    }
}

/// The flow a document holds at `pointer`; `part` is whether it is a part of a mix, which has a `weight`.
pub(super) fn flow(pointer: &str, json: &Json) -> Result<Flow, Error> {
    flow_of(pointer, json, false).map(|(flow, _)| flow)
}

fn flow_of(pointer: &str, json: &Json, part: bool) -> Result<(Flow, f64), Error> {
    let all = ["flow", "angle", "wavelength", "phase", "seed", "path", "parts", "weight"];
    let members = members(pointer, json, &all, "a flow object")?;
    let weight = match (part, members.get("weight")) {
        (true, _) => number(pointer, members, "weight", None)?,
        (false, None) => 1.0,
        (false, Some(_)) => return Err(refuse(&at(pointer, "weight"), "belongs to a part of a mix")),
    };
    let flow = match members.get("flow") {
        Some(Json::String(kind)) if kind == "angle" => {
            members_only(pointer, members, &with_weight(&["flow", "angle"], part))?;
            built(pointer, Flow::angle(number(pointer, members, "angle", None)?))?
        }
        Some(Json::String(kind)) if kind == "noise" => {
            members_only(pointer, members, &with_weight(&["flow", "wavelength", "phase", "seed"], part))?;
            built(
                pointer,
                Flow::noise(
                    number(pointer, members, "wavelength", None)?,
                    number(pointer, members, "phase", Some(0.0))?,
                    number(pointer, members, "seed", Some(0.0))?,
                ),
            )?
        }
        Some(Json::String(kind)) if kind == "contour" => {
            members_only(pointer, members, &with_weight(&["flow", "path"], part))?;
            built(pointer, Flow::contour(&path(pointer, members)?))?
        }
        Some(Json::String(kind)) if kind == "mix" && !part => {
            members_only(pointer, members, &["flow", "parts"])?;
            let here = at(pointer, "parts");
            let Some(Json::Array(items)) = members.get("parts") else {
                return Err(refuse(&here, "must be an array of flows, each with a `weight`"));
            };
            let mut parts = Vec::new();
            for (index, item) in items.iter().enumerate() {
                let (one, weight) = flow_of(&format!("{here}/{index}"), item, true)?;
                parts.push((weight, one));
            }
            built(&here, Flow::weighted(&parts))?
        }
        Some(_) => return Err(refuse(&at(pointer, "flow"), "must be \"angle\", \"noise\", \"contour\" or \"mix\"")),
        None => return Err(refuse(pointer, "is missing `flow`: \"angle\", \"noise\", \"contour\" or \"mix\"")),
    };
    if part && !(weight.is_finite() && weight > 0.0) {
        return Err(refuse(&at(pointer, "weight"), format!("must be above 0, got {weight}")));
    }
    Ok((flow, weight))
}

fn with_weight<'a>(names: &[&'a str], part: bool) -> Vec<&'a str> {
    let mut all = names.to_vec();
    if part {
        all.push("weight");
    }
    all
}

fn pair(point: Point) -> Json {
    Json::Array(vec![Json::Number(point.x), Json::Number(point.y)])
}

/// `field` as a document writes it.
pub(super) fn field_json(field: &Field) -> Json {
    match field.kind() {
        FieldKind::Constant(value) => Json::Number(*value),
        FieldKind::Ramp { start, end } => {
            Json::object([("field", Json::text("ramp")), ("from", pair(*start)), ("to", pair(*end))])
        }
        FieldKind::Distance { path, normalized } => {
            let mut members = vec![("field", Json::text("distance")), ("path", Json::String(path.to_svg_path()))];
            if *normalized {
                members.push(("normalized", Json::Bool(true)));
            }
            Json::object(members)
        }
    }
}

/// `flow` as a document writes it: a lone part plainly, several as a mix.
pub(super) fn flow_json(flow: &Flow) -> Json {
    let parts = flow.parts();
    if let [only] = parts
        && only.weight == 1.0
    {
        return part_json(&only.kind, None);
    }
    let items = parts.iter().map(|part| part_json(&part.kind, Some(part.weight))).collect();
    Json::object([("flow", Json::text("mix")), ("parts", Json::Array(items))])
}

fn part_json(kind: &FlowKind, weight: Option<f64>) -> Json {
    let mut members = match kind {
        FlowKind::Angle { degrees } => vec![("flow", Json::text("angle")), ("angle", Json::Number(*degrees))],
        FlowKind::Noise { wavelength, phase, seed } => vec![
            ("flow", Json::text("noise")),
            ("wavelength", Json::Number(*wavelength)),
            ("phase", Json::Number(*phase)),
            ("seed", Json::Number(*seed)),
        ],
        FlowKind::Contour { path } => vec![("flow", Json::text("contour")), ("path", Json::String(path.to_svg_path()))],
    };
    if let Some(weight) = weight {
        members.push(("weight", Json::Number(weight)));
    }
    Json::object(members)
}
