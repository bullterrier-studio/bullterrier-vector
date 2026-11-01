//! Reading a parsed document into [`Doc`]: what the schema checks, value by value. The shape of each call comes from
//! its verb's declaration in the registry (`Kind`, `Reads`, `Domain`, defaults); nothing here lists a call.

use super::doc::{Call, Doc, Ex, Fixed, InputDecl, InputKind, ParamDecl, Value, VerbCall, What};
use super::value::{Json, pointer_token};
use super::{bare, expression_word};
use crate::expr::Op;
use crate::limits::{MAX_CALLS, MAX_PARAMS};
use crate::param::is_name;
use crate::shape::{Holds, SELECTABLE, Single};
use crate::{Arg, Attributes, Color, Domain, Error, Kind, Point, Reads, Role, Select, Unit, Verb, Wanted};

/// The version of the library, named in the errors that say a document is from a newer one.
const LIBRARY: &str = env!("CARGO_PKG_VERSION");

/// The version of the format this library reads.
const VERSION: f64 = 1.0;

fn refuse(pointer: &str, message: impl Into<String>) -> Error {
    Error::Format {
        pointer: pointer.to_owned(),
        message: message.into(),
    }
}

/// `pointer` followed by the reference token `token`.
fn at(pointer: &str, token: &str) -> String {
    format!("{pointer}/{}", pointer_token(token))
}

/// What the core said was wrong, without the name of the parameter it came with.
pub(super) fn reason(error: &Error) -> String {
    match error {
        Error::Parameter { message, .. } | Error::Geometry { message, .. } | Error::Format { message, .. } => {
            message.clone()
        }
    }
}

/// Reads `top` as a document.
pub(super) fn document(top: &Json) -> Result<Doc, Error> {
    let Json::Object(members) = top else {
        return Err(refuse("", format!("a document is an object, not {}", top.describe())));
    };
    match members.get("version") {
        Some(Json::Number(version)) if *version == VERSION => {}
        Some(other) => {
            let found = match other {
                Json::Number(number) => number.to_string(),
                other => other.describe().to_owned(),
            };
            return Err(refuse(
                "/version",
                format!("version {found} is not one this library reads; it reads version 1"),
            ));
        }
        None => return Err(refuse("", "has no `version`; this library reads version 1")),
    }
    if let Some(unknown) = members
        .keys()
        .find(|key| !["version", "params", "inputs", "drawings", "root"].contains(&key.as_str()))
    {
        return Err(refuse(
            &at("", unknown),
            format!(
                "`{unknown}` is not a key of a document (version, params, inputs, drawings, root); this is bullterrier-vector {LIBRARY}"
            ),
        ));
    }
    let mut reader = Reader { calls: 0 };
    let params = match members.get("params") {
        Some(value) => Reader::params("/params", value)?,
        None => Vec::new(),
    };
    let inputs = match members.get("inputs") {
        Some(value) => Reader::inputs("/inputs", value)?,
        None => Vec::new(),
    };
    let drawings = match members.get("drawings") {
        Some(value) => reader.shared("/drawings", value)?,
        None => Vec::new(),
    };
    let Some(root) = members.get("root") else {
        return Err(refuse("", "has no `root`"));
    };
    let root = reader.drawing("/root", root)?;
    Ok(Doc {
        params,
        inputs,
        drawings,
        root,
    })
}

struct Reader {
    /// The call objects met so far: sources, steps and expressions.
    calls: usize,
}

impl Reader {
    /// Counts one call object, refusing the document when it passes `MAX_CALLS`.
    fn count(&mut self) -> Result<(), Error> {
        self.calls += 1;
        if self.calls > MAX_CALLS {
            return Err(refuse(
                "",
                format!("the document has more than MAX_CALLS {MAX_CALLS} calls (sources, steps and expressions)"),
            ));
        }
        Ok(())
    }

    // The envelope ------------------------------------------------------------------------------------------------

    fn params(pointer: &str, json: &Json) -> Result<Vec<ParamDecl>, Error> {
        let Json::Array(entries) = json else {
            return Err(refuse(pointer, format!("must be an array, not {}", json.describe())));
        };
        if entries.is_empty() {
            return Err(refuse(
                pointer,
                "is left out when there are no Params, not written empty",
            ));
        }
        if entries.len() > MAX_PARAMS {
            return Err(refuse(
                &format!("{pointer}/{MAX_PARAMS}"),
                format!("is the first Param past MAX_PARAMS {MAX_PARAMS}, the most a document declares"),
            ));
        }
        entries
            .iter()
            .enumerate()
            .map(|(at, entry)| param(&format!("{pointer}/{at}"), entry))
            .collect()
    }

    fn inputs(pointer: &str, json: &Json) -> Result<Vec<InputDecl>, Error> {
        let entries = named_entries(pointer, json)?;
        let mut inputs = Vec::new();
        for (name, entry) in entries {
            let here = at(pointer, name);
            let Json::Object(fields) = entry else {
                return Err(refuse(&here, format!("must be an object, not {}", entry.describe())));
            };
            if let Some(unknown) = fields.keys().find(|key| *key != "kind" && *key != "sha256") {
                return Err(refuse(&at(&here, unknown), "is not a field of an input (kind, sha256)"));
            }
            let kind = match fields.get("kind") {
                Some(Json::String(kind)) if kind == "font" => InputKind::Font,
                Some(Json::String(kind)) if kind == "map" => InputKind::Map,
                Some(_) => return Err(refuse(&at(&here, "kind"), "must be \"font\" or \"map\"")),
                None => return Err(refuse(&here, "is missing `kind`")),
            };
            let mut digest = None;
            if let Some(sha) = fields.get("sha256") {
                let lowercase_hex = |text: &String| {
                    text.len() == 64
                        && text
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                };
                if !matches!(sha, Json::String(text) if lowercase_hex(text)) {
                    return Err(refuse(&at(&here, "sha256"), "must be 64 lowercase hexadecimal digits"));
                }
                if let Json::String(text) = sha {
                    digest = Some(text.clone());
                }
            }
            inputs.push(InputDecl {
                pointer: here,
                name: name.clone(),
                kind,
                sha256: digest,
            });
        }
        Ok(inputs)
    }

    fn shared(&mut self, pointer: &str, json: &Json) -> Result<Vec<(String, Vec<Call>)>, Error> {
        let entries = named_entries(pointer, json)?;
        entries
            .into_iter()
            .map(|(name, entry)| Ok((name.clone(), self.drawing(&at(pointer, name), entry)?)))
            .collect()
    }

    // Drawings and calls ------------------------------------------------------------------------------------------

    /// A drawing: a non-empty array of calls, a source and then steps.
    fn drawing(&mut self, pointer: &str, json: &Json) -> Result<Vec<Call>, Error> {
        let Json::Array(calls) = json else {
            return Err(refuse(
                pointer,
                format!("a drawing is an array of calls, not {}", json.describe()),
            ));
        };
        if calls.is_empty() {
            return Err(refuse(pointer, "a drawing has at least a source"));
        }
        calls
            .iter()
            .enumerate()
            .map(|(position, call)| self.call(&format!("{pointer}/{position}"), call, position == 0))
            .collect()
    }

    fn call(&mut self, pointer: &str, json: &Json, first: bool) -> Result<Call, Error> {
        self.count()?;
        let Json::Object(members) = json else {
            return Err(refuse(
                pointer,
                format!("a call is an object with one key, not {}", json.describe()),
            ));
        };
        let mut entries = members.iter();
        let (Some((name, body)), None) = (entries.next(), entries.next()) else {
            return Err(refuse(
                pointer,
                format!("a call is an object with one key, its name; this has {}", members.len()),
            ));
        };
        let here = at(pointer, name);
        if name == "drawing" {
            if !first {
                return Err(refuse(
                    &here,
                    "`drawing` is a source: it starts a drawing, it is not a step",
                ));
            }
            let Json::String(shared) = body else {
                return Err(refuse(&here, "must be the name of a shared drawing"));
            };
            if !is_name(shared) {
                return Err(refuse(&here, "must be letters, digits and _, starting with a letter"));
            }
            return Ok(Call {
                pointer: pointer.to_owned(),
                what: What::Reference {
                    name: shared.clone(),
                    pointer: here,
                },
            });
        }
        let Some(node) = crate::registry::node(name) else {
            return Err(refuse(
                &here,
                format!("`{name}` is not a call this library knows; this is bullterrier-vector {LIBRARY}"),
            ));
        };
        let verb = node.verb();
        match (first, verb.role) {
            (true, Role::Step) => {
                return Err(refuse(
                    &here,
                    format!("a drawing starts with a source; `{name}` is a step"),
                ));
            }
            (false, Role::Source) => {
                return Err(refuse(
                    &here,
                    format!("`{name}` is a source: it starts a drawing, it is not a step"),
                ));
            }
            _ => {}
        }
        let args = self.arguments(verb, &here, body)?;
        Ok(Call {
            pointer: pointer.to_owned(),
            what: What::Verb(VerbCall { node, verb, args }),
        })
    }

    /// The arguments of a call to `verb`, whose value is `body` at `pointer`, in the verb's declaration order.
    fn arguments(
        &mut self,
        verb: &'static Verb,
        pointer: &str,
        body: &Json,
    ) -> Result<Vec<(&'static str, Value)>, Error> {
        if let [only] = verb.args
            && bare(verb)
        {
            return Ok(vec![(only.name, self.value(only, pointer, body)?)]);
        }
        let Json::Object(members) = body else {
            return Err(refuse(
                pointer,
                format!(
                    "the arguments of `{}` are an object, not {}",
                    verb.name,
                    body.describe()
                ),
            ));
        };
        if let Some(unknown) = members
            .keys()
            .find(|key| verb.args.iter().all(|arg| arg.name != key.as_str()))
        {
            let known: Vec<&str> = verb.args.iter().map(|arg| arg.name).collect();
            return Err(refuse(
                &at(pointer, unknown),
                format!(
                    "`{unknown}` is not an argument of `{}`{}",
                    verb.name,
                    if known.is_empty() {
                        String::from(", which takes none")
                    } else {
                        format!("; its arguments are {}", known.join(", "))
                    }
                ),
            ));
        }
        let mut args = Vec::new();
        for arg in verb.args {
            match members.get(arg.name) {
                Some(json) => args.push((arg.name, self.value(arg, &at(pointer, arg.name), json)?)),
                None if arg.default.is_none() => {
                    return Err(refuse(pointer, format!("is missing `{}`", arg.name)));
                }
                None => {}
            }
        }
        Ok(args)
    }

    // Values ------------------------------------------------------------------------------------------------------

    fn value(&mut self, arg: &'static Arg, pointer: &str, json: &Json) -> Result<Value, Error> {
        match arg.kind {
            Kind::Number => Ok(Value::Number(self.number(arg, pointer, json)?)),
            Kind::Point | Kind::Pair => {
                let [first, second] = self.two(arg, pointer, json)?;
                Ok(Value::Pair(first, second))
            }
            Kind::Factor => match json {
                Json::Array(_) => {
                    let [first, second] = self.two(arg, pointer, json)?;
                    Ok(Value::Pair(first, second))
                }
                single => {
                    let both = self.number(arg, pointer, single)?;
                    Ok(Value::Pair(both.clone(), both))
                }
            },
            Kind::Color => {
                let text = string(pointer, json)?;
                Color::parse(text)
                    .map(Value::Color)
                    .map_err(|error| refuse(pointer, reason(&error)))
            }
            Kind::Text => {
                let text = string(pointer, json)?;
                if let Domain::Choice(words) = arg.domain
                    && !words.contains(&text)
                {
                    let listed: Vec<String> = words.iter().map(|word| format!("{word:?}")).collect();
                    return Err(refuse(
                        pointer,
                        format!("must be one of {}, got {text:?}", listed.join(", ")),
                    ));
                }
                Ok(Value::Text(text.to_owned()))
            }
            Kind::Path => Ok(Value::Path {
                data: string(pointer, json)?.to_owned(),
                pointer: pointer.to_owned(),
            }),
            Kind::Shape => Self::shape(pointer, json),
            Kind::Drawings => {
                let Json::Array(items) = json else {
                    return Err(refuse(
                        pointer,
                        format!("must be an array of drawings, not {}", json.describe()),
                    ));
                };
                items
                    .iter()
                    .enumerate()
                    .map(|(at, item)| self.drawing(&format!("{pointer}/{at}"), item))
                    .collect::<Result<_, _>>()
                    .map(Value::Drawings)
            }
            Kind::Attributes => select(pointer, json).map(Value::Select),
            Kind::Values => values(pointer, json).map(Value::Values),
            Kind::Font => match json {
                Json::String(name) if is_name(name) => Ok(Value::Font {
                    name: name.clone(),
                    pointer: pointer.to_owned(),
                }),
                _ => Err(refuse(
                    pointer,
                    "must be the name of an input: letters, digits and _, starting with a letter",
                )),
            },
        }
    }

    /// The two numbers of an argument written `[a, b]`.
    fn two(&mut self, arg: &'static Arg, pointer: &str, json: &Json) -> Result<[Ex; 2], Error> {
        match json {
            Json::Array(items) if items.len() == 2 => {
                let first = self.number(arg, &format!("{pointer}/0"), &items[0])?;
                let second = self.number(arg, &format!("{pointer}/1"), &items[1])?;
                Ok([first, second])
            }
            Json::Array(items) => Err(refuse(pointer, format!("must be two numbers, not {}", items.len()))),
            other => Err(refuse(
                pointer,
                format!("must be two numbers [a, b], not {}", other.describe()),
            )),
        }
    }

    /// A number or an expression for `arg`: an expression only where the argument reads one per shape, and a literal
    /// within the argument's domain.
    fn number(&mut self, arg: &'static Arg, pointer: &str, json: &Json) -> Result<Ex, Error> {
        let literal = arg.reads == Reads::Literal;
        let number = self.expression(pointer, json, literal)?;
        if let Ex::Number(value) = number {
            arg.domain
                .number(arg.name, value)
                .map_err(|error| refuse(pointer, reason(&error)))?;
        }
        Ok(number)
    }

    /// A JSON number, or an expression (an object of one key).
    fn expression(&mut self, pointer: &str, json: &Json, literal: bool) -> Result<Ex, Error> {
        let members = match json {
            Json::Number(number) => return Ok(Ex::Number(*number)),
            Json::Object(members) if !literal => members,
            Json::Object(_) => {
                return Err(refuse(
                    pointer,
                    "must be a number here: this argument takes no expression",
                ));
            }
            other => {
                return Err(refuse(
                    pointer,
                    format!(
                        "must be a number{}, not {}",
                        if literal { "" } else { " or an expression" },
                        other.describe()
                    ),
                ));
            }
        };
        self.count()?;
        let mut entries = members.iter();
        let (Some((word, body)), None) = (entries.next(), entries.next()) else {
            return Err(refuse(
                pointer,
                format!(
                    "an expression is an object with one key, its name; this has {}",
                    members.len()
                ),
            ));
        };
        let here = at(pointer, word);
        let expression = match word.as_str() {
            "param" => match body {
                Json::String(name) if is_name(name) => Ex::Param {
                    name: name.clone(),
                    pointer: here.clone(),
                },
                _ => {
                    return Err(refuse(
                        &here,
                        "must be the name of a Param: letters, digits and _, starting with a letter",
                    ));
                }
            },
            "neg" => Ex::Neg(Box::new(self.expression(&here, body, false)?)),
            other => {
                let Some(op) = expression_word(other) else {
                    return Err(refuse(
                        &here,
                        format!(
                            "`{other}` is not an expression this library knows; this is bullterrier-vector {LIBRARY}"
                        ),
                    ));
                };
                let Json::Array(operands) = body else {
                    return Err(refuse(
                        &here,
                        format!("must be an array of two operands, not {}", body.describe()),
                    ));
                };
                let [left, right] = operands.as_slice() else {
                    return Err(refuse(&here, format!("must have two operands, not {}", operands.len())));
                };
                let left = self.expression(&format!("{here}/0"), left, false)?;
                let right = self.expression(&format!("{here}/1"), right, false)?;
                if op == Op::Div && matches!(right, Ex::Number(divisor) if divisor == 0.0) {
                    return Err(refuse(&format!("{here}/1"), "a literal divisor of 0"));
                }
                Ex::Binary(op, Box::new(left), Box::new(right))
            }
        };
        if expression.reads_param() {
            Ok(expression)
        } else {
            Err(refuse(
                pointer,
                "an expression that reads no Param is a plain number: write its value",
            ))
        }
    }

    /// The value of a `shape` source: its path and the attributes its generators wrote.
    fn shape(pointer: &str, json: &Json) -> Result<Value, Error> {
        let Json::Object(members) = json else {
            return Err(refuse(pointer, format!("must be an object, not {}", json.describe())));
        };
        if let Some(unknown) = members.keys().find(|key| *key != "path" && *key != "attributes") {
            return Err(refuse(
                &at(pointer, unknown),
                "is not a field of a shape (path, attributes)",
            ));
        }
        let Some(path) = members.get("path") else {
            return Err(refuse(pointer, "is missing `path`"));
        };
        let path_pointer = at(pointer, "path");
        let data = string(&path_pointer, path)?.to_owned();
        let attributes = match members.get("attributes") {
            Some(json) => attributes(&at(pointer, "attributes"), json)?,
            None => Attributes::default(),
        };
        Ok(Value::Shape {
            data,
            pointer: path_pointer,
            attributes,
        })
    }
}

// Entries ---------------------------------------------------------------------------------------------------------

/// The members of the object `json` (`inputs`, `drawings`), each named by a valid name; not empty.
fn named_entries<'a>(pointer: &str, json: &'a Json) -> Result<Vec<(&'a String, &'a Json)>, Error> {
    let Json::Object(entries) = json else {
        return Err(refuse(pointer, format!("must be an object, not {}", json.describe())));
    };
    if entries.is_empty() {
        return Err(refuse(pointer, "is left out when it is empty, not written empty"));
    }
    if let Some(name) = entries.keys().find(|name| !is_name(name)) {
        return Err(refuse(
            &at(pointer, name),
            "a name is letters, digits and _, starting with a letter",
        ));
    }
    Ok(entries.iter().collect())
}

/// The fields of a Param's description.
const PARAM_FIELDS: [&str; 6] = ["name", "default", "min", "max", "unit", "meaning"];

/// An entry of `params`.
fn param(pointer: &str, json: &Json) -> Result<ParamDecl, Error> {
    let Json::Object(fields) = json else {
        return Err(refuse(pointer, format!("must be an object, not {}", json.describe())));
    };
    if let Some(unknown) = fields.keys().find(|key| !PARAM_FIELDS.contains(&key.as_str())) {
        return Err(refuse(
            &at(pointer, unknown),
            "is not a field of a Param (name, default, min, max, unit, meaning)",
        ));
    }
    let field = |name: &str| {
        fields
            .get(name)
            .ok_or_else(|| refuse(pointer, format!("is missing `{name}`")))
    };
    let number = |name: &str| match field(name)? {
        Json::Number(number) => Ok(*number),
        other => Err(refuse(
            &at(pointer, name),
            format!("must be a number, not {}", other.describe()),
        )),
    };
    let name = string(&at(pointer, "name"), field("name")?)?;
    if !is_name(name) {
        return Err(refuse(
            &at(pointer, "name"),
            format!("must be letters, digits and _, starting with a letter, got {name:?}"),
        ));
    }
    let unit = Unit::parse(string(&at(pointer, "unit"), field("unit")?)?)
        .map_err(|error| refuse(&at(pointer, "unit"), reason(&error)))?;
    Ok(ParamDecl {
        pointer: pointer.to_owned(),
        name: name.to_owned(),
        default: number("default")?,
        min: number("min")?,
        max: number("max")?,
        unit,
        meaning: string(&at(pointer, "meaning"), field("meaning")?)?.to_owned(),
    })
}

fn string<'a>(pointer: &str, json: &'a Json) -> Result<&'a str, Error> {
    match json {
        Json::String(text) => Ok(text),
        other => Err(refuse(pointer, format!("must be a string, not {}", other.describe()))),
    }
}

fn number_at(pointer: &str, json: &Json) -> Result<f64, Error> {
    match json {
        Json::Number(number) => Ok(*number),
        other => Err(refuse(pointer, format!("must be a number, not {}", other.describe()))),
    }
}

// Attributes and selections ---------------------------------------------------------------------------------------

/// The attributes a `shape` carries: each of the nine a generator writes, and nothing else.
fn attributes(pointer: &str, json: &Json) -> Result<Attributes, Error> {
    let Json::Object(members) = json else {
        return Err(refuse(pointer, format!("must be an object, not {}", json.describe())));
    };
    let mut written = Attributes::default();
    for (name, json) in members {
        let here = at(pointer, name);
        let whole = || match json {
            Json::Number(number) => {
                whole_u32(*number).ok_or_else(|| refuse(&here, format!("must be a whole number from 0, got {number}")))
            }
            other => Err(refuse(
                &here,
                format!("must be a whole number, not {}", other.describe()),
            )),
        };
        match name.as_str() {
            "glyph" => written.glyph = Some(whole()?),
            "word" => written.word = Some(whole()?),
            "line" => written.line = Some(whole()?),
            "part" => written.part = Some(whole()?),
            "copy" => written.copy = Some(whole()?),
            "char" => written.char = Some(one_character(&here, json)?),
            "level" => written.level = Some(number_at(&here, json)?),
            "order" => written.order = Some(number_at(&here, json)?),
            "origin" => written.origin = Some(point(&here, json)?),
            _ => {
                return Err(refuse(
                    &here,
                    "is not an attribute a shape carries (glyph, char, word, line, part, copy, level, order, origin)",
                ));
            }
        }
    }
    Ok(written)
}

/// `number` as a `u32` if it is one.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the number is checked to be whole and within u32"
)]
fn whole_u32(number: f64) -> Option<u32> {
    (number.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&number)).then_some(number as u32)
}

fn one_character(pointer: &str, json: &Json) -> Result<char, Error> {
    let text = string(pointer, json)?;
    let mut characters = text.chars();
    match (characters.next(), characters.next()) {
        (Some(only), None) => Ok(only),
        _ => Err(refuse(pointer, format!("must be one character, got {text:?}"))),
    }
}

fn point(pointer: &str, json: &Json) -> Result<Point, Error> {
    match json {
        Json::Array(items) if items.len() == 2 => Ok(Point::new(
            number_at(&format!("{pointer}/0"), &items[0])?,
            number_at(&format!("{pointer}/1"), &items[1])?,
        )),
        other => Err(refuse(
            pointer,
            format!("must be a point [x, y], not {}", other.describe()),
        )),
    }
}

/// The attributes `only` and `without` ask about, and what a shape's must be.
fn select(pointer: &str, json: &Json) -> Result<Select, Error> {
    let Json::Object(members) = json else {
        return Err(refuse(pointer, format!("must be an object, not {}", json.describe())));
    };
    let mut asked = Select::new();
    for (name, json) in members {
        let here = at(pointer, name);
        let Some((_, holds)) = SELECTABLE.iter().find(|(known, _)| known == name) else {
            let known: Vec<&str> = SELECTABLE.iter().map(|(known, _)| *known).collect();
            return Err(refuse(
                &here,
                format!("is not an attribute, the attributes are {}", known.join(", ")),
            ));
        };
        asked = asked.attribute(name, Wanted::of(singles(&here, *holds, json)?));
    }
    Ok(asked)
}

/// One value an attribute holds, or a list of them.
fn singles(pointer: &str, holds: Holds, json: &Json) -> Result<Vec<Single>, Error> {
    let one = |pointer: &str, json: &Json| -> Result<Single, Error> {
        Ok(match holds {
            Holds::Whole | Holds::Real => Single::Number(number_at(pointer, json)?),
            Holds::Char => Single::Char(one_character(pointer, json)?),
            Holds::Point => Single::Point(point(pointer, json)?),
        })
    };
    match json {
        // a point is itself an array of numbers: a list of points is an array of arrays, or empty
        Json::Array(items)
            if holds != Holds::Point || items.is_empty() || items.iter().any(|item| matches!(item, Json::Array(_))) =>
        {
            items
                .iter()
                .enumerate()
                .map(|(at, item)| one(&format!("{pointer}/{at}"), item))
                .collect()
        }
        single => Ok(vec![one(pointer, single)?]),
    }
}

/// The Params a `with_values` fixes: each to a number, or to a list of numbers.
fn values(pointer: &str, json: &Json) -> Result<Vec<Fixed>, Error> {
    let Json::Object(members) = json else {
        return Err(refuse(pointer, format!("must be an object, not {}", json.describe())));
    };
    members
        .iter()
        .map(|(name, json)| {
            let here = at(pointer, name);
            let value = match json {
                Json::Array(items) => crate::Value::List(
                    items
                        .iter()
                        .enumerate()
                        .map(|(at, item)| number_at(&format!("{here}/{at}"), item))
                        .collect::<Result<_, _>>()?,
                ),
                other => crate::Value::Number(number_at(&here, other)?),
            };
            Ok(Fixed {
                name: name.clone(),
                pointer: here,
                value,
            })
        })
        .collect()
}
