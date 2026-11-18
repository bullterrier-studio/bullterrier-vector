//! A `Field` and a `Flow` in a document, written as the calls of formats.md: a field is a number or expression (the
//! same everywhere) or `{"Field.ramp": {"start", "end"}}`, `{"Field.distance": {"drawing", "normalized"}}` or
//! `{"Field.map": {"width", "height", "data", "box"}}`; a flow is `{"Flow.angle": {"angle"}}`,
//! `{"Flow.noise": {"wavelength", "phase", "seed"}}`, `{"Flow.contour": {"drawing"}}` or
//! `{"Flow.mix": {"flow", "other", "fraction"}}`. A `drawing` is a drawing like any other, drawn with the values of
//! the step that reads the field; a number may be an expression of Params (not a per-item number). What a call makes
//! is what is written, and reading it back builds the same value.

use super::doc::{Call, FieldDoc, FieldWhat, FlowDoc, FlowWhat};
use super::read::{Reader, at, number_at, refuse};
use super::value::Json;
use super::write::Writer;
use crate::field::{FieldKind, FlowKind};
use crate::param::is_name;
use crate::{Error, Field, Flow, Point, Reads, Rect};
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

/// A literal number of a call: a plain JSON number, never an expression.
fn literal(pointer: &str, args: &BTreeMap<String, Json>, name: &str, default: Option<f64>) -> Result<f64, Error> {
    match (args.get(name), default) {
        (Some(Json::Number(value)), _) => Ok(*value),
        (Some(Json::Object(_)), _) => Err(refuse(
            &at(pointer, name),
            "must be a number: this argument takes no Param",
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
            literal(&here, &BTreeMap::from([("0".to_owned(), items[0].clone())]), "0", None)?,
            literal(&here, &BTreeMap::from([("1".to_owned(), items[1].clone())]), "1", None)?,
        )),
        Some(_) => Err(refuse(&here, "must be two numbers [x, y]")),
        None => Err(refuse(pointer, format!("is missing `{name}`"))),
    }
}

impl Reader {
    /// A number of a call that may read Params, once for the whole call.
    fn once_arg(
        &mut self,
        pointer: &str,
        args: &BTreeMap<String, Json>,
        name: &str,
        default: Option<f64>,
    ) -> Result<super::doc::Ex, Error> {
        match (args.get(name), default) {
            (Some(json), _) => self.expression(&at(pointer, name), json, Reads::Once),
            (None, Some(default)) => Ok(super::doc::Ex::Number(default)),
            (None, None) => Err(refuse(pointer, format!("is missing `{name}`"))),
        }
    }

    /// The drawing of the argument `drawing`.
    fn drawing_arg(&mut self, pointer: &str, args: &BTreeMap<String, Json>) -> Result<Vec<Call>, Error> {
        match args.get("drawing") {
            Some(json) => self.drawing(&at(pointer, "drawing"), json),
            None => Err(refuse(pointer, "is missing `drawing`")),
        }
    }

    /// The field a document holds at `pointer`.
    pub(super) fn field(&mut self, pointer: &str, json: &Json) -> Result<FieldDoc, Error> {
        if matches!(json, Json::Number(_)) || matches!(json, Json::Object(members) if is_expression(members)) {
            let value = self.expression(pointer, json, Reads::Once)?;
            return Ok(FieldDoc {
                pointer: pointer.to_owned(),
                what: FieldWhat::Constant(value),
            });
        }
        let (word, args, here) = call(
            pointer,
            json,
            &["Field.ramp", "Field.distance", "Field.map"],
            "a number or a field",
        )?;
        let what = match word {
            "Field.ramp" => {
                only(&here, args, &["start", "end"])?;
                FieldWhat::Ramp {
                    start: point(&here, args, "start")?,
                    end: point(&here, args, "end")?,
                }
            }
            "Field.distance" => {
                only(&here, args, &["drawing", "normalized"])?;
                let normalized = match args.get("normalized") {
                    None => false,
                    Some(Json::Bool(flag)) => *flag,
                    Some(_) => return Err(refuse(&at(&here, "normalized"), "must be true or false")),
                };
                FieldWhat::Distance {
                    drawing: self.drawing_arg(&here, args)?,
                    normalized,
                }
            }
            _ => {
                only(&here, args, &["width", "height", "data", "box"])?;
                let size = |name: &str| -> Result<usize, Error> {
                    let value = literal(&here, args, name, None)?;
                    // a whole number from 1 that fits the limit of a map, which Field::map checks in full
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "a whole number from 1 to 2^53, checked here"
                    )]
                    if value.fract() == 0.0 && (1.0..=9_007_199_254_740_991.0).contains(&value) {
                        Ok(value as usize)
                    } else {
                        Err(refuse(
                            &at(&here, name),
                            format!("must be a whole number from 1, got {value}"),
                        ))
                    }
                };
                let data = match args.get("data") {
                    Some(Json::String(name)) if is_name(name) => (name.clone(), at(&here, "data")),
                    Some(_) => {
                        return Err(refuse(
                            &at(&here, "data"),
                            "must be the name of an input: letters, digits and _, starting with a letter",
                        ));
                    }
                    None => return Err(refuse(&here, "is missing `data`")),
                };
                let frame = match args.get("box") {
                    None => None,
                    Some(Json::Array(items)) if items.len() == 4 => {
                        let at_box = at(&here, "box");
                        let number = |index: usize| number_at(&at(&at_box, &index.to_string()), &items[index]);
                        Some(Rect {
                            x: number(0)?,
                            y: number(1)?,
                            width: number(2)?,
                            height: number(3)?,
                        })
                    }
                    Some(_) => return Err(refuse(&at(&here, "box"), "must be four numbers [x, y, width, height]")),
                };
                FieldWhat::Map {
                    name: data.0,
                    pointer: data.1,
                    width: size("width")?,
                    height: size("height")?,
                    frame,
                }
            }
        };
        Ok(FieldDoc { pointer: here, what })
    }

    /// The flow a document holds at `pointer`.
    pub(super) fn flow(&mut self, pointer: &str, json: &Json) -> Result<FlowDoc, Error> {
        let words = ["Flow.angle", "Flow.noise", "Flow.contour", "Flow.mix"];
        let (word, args, here) = call(pointer, json, &words, "a flow")?;
        let what = match word {
            "Flow.angle" => {
                only(&here, args, &["angle"])?;
                FlowWhat::Angle(self.once_arg(&here, args, "angle", None)?)
            }
            "Flow.noise" => {
                only(&here, args, &["wavelength", "phase", "seed"])?;
                FlowWhat::Noise {
                    wavelength: self.once_arg(&here, args, "wavelength", Some(400.0))?,
                    phase: self.once_arg(&here, args, "phase", Some(0.0))?,
                    seed: literal(&here, args, "seed", Some(0.0))?,
                }
            }
            "Flow.contour" => {
                only(&here, args, &["drawing"])?;
                FlowWhat::Contour(self.drawing_arg(&here, args)?)
            }
            _ => {
                only(&here, args, &["flow", "other", "fraction"])?;
                let member = |name: &str| {
                    args.get(name)
                        .ok_or_else(|| refuse(&here, format!("is missing `{name}`")))
                };
                let flow = Box::new(self.flow(&at(&here, "flow"), member("flow")?)?);
                let other = Box::new(self.flow(&at(&here, "other"), member("other")?)?);
                FlowWhat::Mix {
                    flow,
                    other,
                    fraction: self.once_arg(&here, args, "fraction", None)?,
                }
            }
        };
        Ok(FlowDoc { pointer: here, what })
    }
}

/// Whether `json` is a number or an expression rather than a field's call: what a `Field` argument that reads one number
/// per shape takes as that number.
pub(super) fn is_number(json: &Json) -> bool {
    match json {
        Json::Number(_) => true,
        Json::Object(members) => is_expression(members),
        _ => false,
    }
}

/// Whether an object is an expression (a call of the expression language) rather than a field's call.
fn is_expression(members: &BTreeMap<String, Json>) -> bool {
    members.len() == 1 && members.keys().all(|word| !word.starts_with("Field."))
}

fn pair(point: Point) -> Json {
    Json::Array(vec![Json::Number(point.x), Json::Number(point.y)])
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a map has at most 2^24 pixels, so its sides are exact in a number"
)]
impl Writer<'_> {
    /// `field` as a document writes it; `taken` yields the nodes of the drawings it reads.
    pub(super) fn field_json(&self, field: &Field, taken: &mut impl Iterator<Item = usize>) -> Result<Json, Error> {
        Ok(match field.kind() {
            FieldKind::Constant(value) => self.expression(value)?,
            FieldKind::Ramp { start, end } => Json::object([(
                "Field.ramp",
                Json::object([("start", pair(*start)), ("end", pair(*end))]),
            )]),
            FieldKind::Distance { normalized, .. } => {
                let mut args = vec![("drawing", self.taken_drawing(taken)?)];
                if *normalized {
                    args.push(("normalized", Json::Bool(true)));
                }
                Json::object([("Field.distance", Json::object(args))])
            }
            FieldKind::Map(map) => {
                let mut args = vec![
                    ("width", Json::Number(map.width as f64)),
                    ("height", Json::Number(map.height as f64)),
                    ("data", Json::text(self.map_name(&map.sha256_hex())?)),
                ];
                let default = Rect {
                    x: 0.0,
                    y: 0.0,
                    width: map.width as f64,
                    height: map.height as f64,
                };
                if map.frame != default {
                    let f = map.frame;
                    args.push((
                        "box",
                        Json::Array(vec![
                            Json::Number(f.x),
                            Json::Number(f.y),
                            Json::Number(f.width),
                            Json::Number(f.height),
                        ]),
                    ));
                }
                Json::object([("Field.map", Json::object(args))])
            }
        })
    }

    /// `flow` as a document writes it.
    pub(super) fn flow_json(&self, flow: &Flow, taken: &mut impl Iterator<Item = usize>) -> Result<Json, Error> {
        Ok(match flow.kind() {
            FlowKind::Angle { degrees } => {
                Json::object([("Flow.angle", Json::object([("angle", self.expression(degrees)?)]))])
            }
            FlowKind::Noise {
                wavelength,
                phase,
                seed,
            } => Json::object([(
                "Flow.noise",
                Json::object([
                    ("wavelength", self.expression(wavelength)?),
                    ("phase", self.expression(phase)?),
                    ("seed", Json::Number(*seed)),
                ]),
            )]),
            FlowKind::Contour { .. } => {
                Json::object([("Flow.contour", Json::object([("drawing", self.taken_drawing(taken)?)]))])
            }
            FlowKind::Mix { flow, other, fraction } => Json::object([(
                "Flow.mix",
                Json::object([
                    ("flow", self.flow_json(flow, taken)?),
                    ("other", self.flow_json(other, taken)?),
                    ("fraction", self.expression(fraction)?),
                ]),
            )]),
        })
    }

    fn taken_drawing(&self, taken: &mut impl Iterator<Item = usize>) -> Result<Json, Error> {
        let node = taken
            .next()
            .ok_or_else(|| Error::parameter("to_json", "a drawing the call takes is not in the graph"))?;
        Ok(Json::Array(self.chain(node, false)?))
    }
}
