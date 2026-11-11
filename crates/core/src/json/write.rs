//! `to_json`: a drawing as the canonical document of formats.md.

use super::value::Json;
use super::{bare, depth};
use crate::drawing::Graph;
use crate::expr::{Op, View};
use crate::limits::{MAX_CALLS, MAX_DEPTH, MAX_DOCUMENT_BYTES};
use crate::node::{Args, Given};
use crate::shape::{SELECTABLE, Single};
use crate::{Arg, Color, Drawing, Error, Expr, GradientKind, Kind, Literal, Paint, Select, Shape, Value, Values, Verb};
use std::cell::Cell;
use std::collections::BTreeMap;

impl Drawing {
    /// The drawing as JSON in the canonical form of formats.md: one text for one drawing, keys sorted, no whitespace,
    /// the shortest numbers that read back as the same value, arguments equal to their defaults left out, equal
    /// drawings merged and written once under a name when two or more calls build on them or take them.
    /// [`Drawing::from_json`] reads the text back to a drawing that draws the same shapes.
    ///
    /// # Errors
    /// The first error of the chain that built the drawing; [`Error::Parameter`] naming `to_json` if the document
    /// would pass `MAX_DOCUMENT_BYTES`, `MAX_CALLS` or `MAX_DEPTH`, which `from_json` would refuse; naming
    /// `expression` if a part of an expression that reads no Param has no value (a division by zero).
    pub fn to_json(&self) -> Result<String, Error> {
        let document = Writer::new(self)?.document()?;
        let nesting = depth::of(&document.json);
        if nesting > MAX_DEPTH {
            return Err(Error::parameter(
                "to_json",
                format!("the document nests {nesting} levels deep, but MAX_DEPTH {MAX_DEPTH} is the most accepted"),
            ));
        }
        if document.calls > MAX_CALLS {
            return Err(Error::parameter(
                "to_json",
                format!(
                    "the document has {} calls, but MAX_CALLS {MAX_CALLS} is the most accepted",
                    document.calls
                ),
            ));
        }
        let text = document.json.canonical();
        if text.len() > MAX_DOCUMENT_BYTES {
            return Err(Error::parameter(
                "to_json",
                format!(
                    "the document is {} bytes, but MAX_DOCUMENT_BYTES {MAX_DOCUMENT_BYTES} is the most accepted",
                    text.len()
                ),
            ));
        }
        Ok(text)
    }
}

/// The document as a tree, and how many call objects (sources, steps, expressions) it holds.
struct Document {
    json: Json,
    calls: usize,
}

/// Writes one drawing.
struct Writer<'a> {
    drawing: &'a Drawing,
    graph: Graph,
    /// The name each font is written under, by the SHA-256 of its bytes, and the fonts in the order the walk met them.
    fonts: Vec<(String, String)>,
    /// The name of each node that is written under `drawings`: `d1`, `d2`... in the order the walk reaches them.
    names: Vec<Option<String>>,
    calls: Cell<usize>,
}

impl<'a> Writer<'a> {
    fn new(drawing: &'a Drawing) -> Result<Self, Error> {
        let graph = drawing.graph()?;
        let mut count = 0;
        let names = graph
            .nodes
            .iter()
            .map(|node| {
                (node.uses >= 2).then(|| {
                    count += 1;
                    format!("d{count}")
                })
            })
            .collect();
        let fonts = font_names(&graph)?;
        Ok(Self {
            drawing,
            graph,
            fonts,
            names,
            calls: Cell::new(0),
        })
    }

    fn document(self) -> Result<Document, Error> {
        let mut top = vec![("version", Json::Number(1.0))];
        let params: Vec<Json> = self
            .drawing
            .read()?
            .iter()
            .map(|param| {
                Json::object([
                    ("name", Json::text(param.name())),
                    ("default", Json::Number(param.default())),
                    ("min", Json::Number(param.min())),
                    ("max", Json::Number(param.max())),
                    ("unit", Json::text(param.unit_of().name())),
                    ("meaning", Json::text(param.meaning_of())),
                ])
            })
            .collect();
        if !params.is_empty() {
            top.push(("params", Json::Array(params)));
        }
        if !self.fonts.is_empty() {
            top.push((
                "inputs",
                Json::object(self.fonts.iter().map(|(name, sha256)| {
                    (
                        name.as_str(),
                        Json::object([("kind", Json::text("font")), ("sha256", Json::text(sha256))]),
                    )
                })),
            ));
        }
        let mut shared = Vec::new();
        for at in 0..self.graph.nodes.len() {
            if let Some(name) = self.names[at].clone() {
                shared.push((name, Json::Array(self.chain(at, true)?)));
            }
        }
        if !shared.is_empty() {
            top.push(("drawings", Json::Object(shared.into_iter().collect())));
        }
        let root = self.chain(self.graph.root, false)?;
        top.push(("root", Json::Array(root)));
        Ok(Document {
            json: Json::object(top),
            calls: self.calls.get(),
        })
    }

    /// The calls that make the drawing ending at `last`, from its source: written in place back to a node that is
    /// shared, which is a reference to it. `entry` writes the call of `last` itself even if it is shared: that is the
    /// entry under its name.
    fn chain(&self, last: usize, entry: bool) -> Result<Vec<Json>, Error> {
        let mut groups = Vec::new();
        let mut at = Some(last);
        while let Some(node) = at {
            if let Some(name) = &self.names[node]
                && !(entry && node == last)
            {
                self.calls.set(self.calls.get() + 1);
                groups.push(vec![Json::object([("drawing", Json::text(name))])]);
                break;
            }
            groups.push(self.calls_of(node)?);
            at = self.graph.nodes[node].previous;
        }
        groups.reverse();
        Ok(groups.into_iter().flatten().collect())
    }

    /// The call of `node`.
    fn calls_of(&self, node: usize) -> Result<Vec<Json>, Error> {
        let taken = self.graph.nodes[node].taken.clone();
        let verb = self.graph.nodes[node].verb();
        let mut taken = taken.into_iter();
        Ok(vec![self.call(verb, self.node_args(node), &mut taken)?])
    }

    fn node_args(&self, node: usize) -> &Args {
        self.graph.nodes[node].args()
    }

    /// `{"verb": {arguments}}`: every argument that is not at its default. `taken` yields the nodes of the drawings
    /// the arguments take, in order.
    fn call(&self, verb: &'static Verb, args: &Args, taken: &mut impl Iterator<Item = usize>) -> Result<Json, Error> {
        self.calls.set(self.calls.get() + 1);
        let mut members = Vec::new();
        for (arg, given) in args.declared() {
            if at_default(arg, given) {
                continue;
            }
            members.push((arg.name, self.value(arg, given, taken)?));
        }
        if bare(verb) {
            // the call's value is its one argument's: `{"path": "M0 0"}`
            let value = members
                .pop()
                .map_or_else(|| Json::Object(BTreeMap::default()), |(_, value)| value);
            return Ok(Json::object([(verb.name, value)]));
        }
        Ok(Json::object([(verb.name, Json::object(members))]))
    }

    fn value(&self, arg: &Arg, given: &Given, taken: &mut impl Iterator<Item = usize>) -> Result<Json, Error> {
        Ok(match given {
            Given::Number(expr) => self.expression(expr)?,
            Given::Pair(first, second) if arg.kind == Kind::Factor && first.is_written_like(second) => {
                self.expression(first)?
            }
            Given::Pair(first, second) => Json::Array(vec![self.expression(first)?, self.expression(second)?]),
            Given::Text(text) => Json::text(text),
            Given::Color(color) => Json::String(color_text(*color)),
            Given::Paint(paint) => paint_json(paint),
            Given::Path(path) => Json::String(path.to_svg_path()),
            Given::Shape(shape) => shape_json(shape),
            Given::Drawings(items) => {
                let mut arrays = Vec::new();
                for _ in items {
                    let Some(node) = taken.next() else { break };
                    arrays.push(Json::Array(self.chain(node, false)?));
                }
                Json::Array(arrays)
            }
            Given::Drawing(_) => {
                let node = taken
                    .next()
                    .ok_or_else(|| Error::parameter("to_json", "a drawing the call takes is not in the graph"))?;
                Json::Array(self.chain(node, false)?)
            }
            Given::Select(select) => select_json(select),
            Given::Values(values) => values_json(values),
            Given::Font(font) => {
                let digest = font.sha256();
                let (name, _) = self
                    .fonts
                    .iter()
                    .find(|(_, sha256)| *sha256 == digest)
                    .ok_or_else(|| Error::parameter("to_json", "a font of the drawing is not among its inputs"))?;
                Json::text(name)
            }
            Given::Axes(axes) => Json::Object(
                axes.iter()
                    .map(|(tag, expr)| Ok((tag.clone(), self.expression(expr)?)))
                    .collect::<Result<_, Error>>()?,
            ),
            Given::Features(features) => Json::Object(
                features
                    .iter()
                    .map(|(tag, value)| (tag.clone(), Json::Number(f64::from(*value))))
                    .collect(),
            ),
            Given::Unset => Json::Null,
        })
    }

    /// An expression with every part that is a plain number folded to its value, a per-item number kept; [`Expr::view`] gives the operands of
    /// `add` and `mul` with a literal one first.
    fn expression(&self, expr: &Expr) -> Result<Json, Error> {
        if expr.is_plain() {
            return Ok(Json::Number(expr.value(&Values::new())?));
        }
        self.calls.set(self.calls.get() + 1);
        Ok(match expr.view() {
            View::Const(number) => Json::Number(number),
            View::Param(param) => Json::object([("param", Json::text(param.name()))]),
            View::TooDeep => return Err(expr.too_deep()),
            View::Neg(inner) => Json::object([("neg", self.expression(inner)?)]),
            View::Item(call) => {
                let fields = call
                    .fields()
                    .iter()
                    .zip(call.operands())
                    .map(|(field, operand)| Ok((field.name, self.expression(operand)?)))
                    .collect::<Result<Vec<_>, Error>>()?;
                Json::object([(call.word(), Json::object(fields))])
            }
            View::Binary(op, left, right) => {
                let (left, right) = (self.expression(left)?, self.expression(right)?);
                if op == Op::Div && matches!(right, Json::Number(divisor) if divisor == 0.0) {
                    return Err(Error::parameter(
                        "expression",
                        "divides by a literal 0, which a document cannot hold",
                    ));
                }
                Json::object([(op.word(), Json::Array(vec![left, right]))])
            }
        })
    }
}

/// The inputs of a drawing: each font once, with the name it is written under and the SHA-256 of its bytes, in the order
/// the walk of formats.md meets them. A font keeps the name it was given, else it is called `font1`, `font2`... in that
/// order, skipping names the drawing gives; fonts with equal bytes are one input.
///
/// # Errors
/// [`Error::Parameter`] naming `to_json` if one font is given two names, or two fonts one name.
fn font_names(graph: &Graph) -> Result<Vec<(String, String)>, Error> {
    // the fonts by digest in walk order, each with the name it was given
    let mut found: Vec<(String, Option<String>)> = Vec::new();
    for node in &graph.nodes {
        for (_, given) in node.args().declared() {
            let Given::Font(font) = given else { continue };
            let digest = font.sha256();
            match found.iter_mut().find(|(known, _)| *known == digest) {
                None => found.push((digest, font.name().map(str::to_owned))),
                Some((_, name)) => match (name.as_deref(), font.name()) {
                    (Some(first), Some(second)) if first != second => {
                        return Err(Error::parameter(
                            "to_json",
                            format!("the font {first:?} is also given as {second:?}: one input is given one name"),
                        ));
                    }
                    (None, Some(second)) => *name = Some(second.to_owned()),
                    _ => {}
                },
            }
        }
    }
    let given: Vec<String> = found.iter().filter_map(|(_, name)| name.clone()).collect();
    for (at, name) in given.iter().enumerate() {
        if given[..at].contains(name) {
            return Err(Error::parameter(
                "to_json",
                format!("two different fonts are both called {name:?}: one name is given one input"),
            ));
        }
    }
    let mut number = 0;
    let mut automatic = || loop {
        number += 1;
        let name = format!("font{number}");
        if !given.contains(&name) {
            return name;
        }
    };
    Ok(found
        .into_iter()
        .map(|(digest, name)| (name.unwrap_or_else(&mut automatic), digest))
        .collect())
}

/// Whether `given` is what the call would take if the argument were left out.
fn at_default(arg: &Arg, given: &Given) -> bool {
    match (arg.default, given) {
        (Some(Literal::Number(default)), Given::Number(expr)) => expr.constant() == Some(default),
        (Some(Literal::Text(default)), Given::Text(text)) => text == default,
        (_, Given::Unset) => true,
        _ => false,
    }
}

/// `color` as the document writes it: lower case, `#rrggbb` when opaque.
fn color_text(color: Color) -> String {
    let text = color.to_string();
    match text.strip_suffix("ff") {
        Some(opaque) if text.len() == 9 => opaque.to_owned(),
        _ => text,
    }
}

/// `paint` as the document writes it: a colour text, or the gradient object.
fn paint_json(paint: &Paint) -> Json {
    let Paint::Gradient(gradient) = paint else {
        return match paint {
            Paint::Color(color) => Json::String(color_text(*color)),
            Paint::Gradient(_) => unreachable!("matched above"),
        };
    };
    let pair = |at: crate::Point| Json::Array(vec![Json::Number(at.x), Json::Number(at.y)]);
    let stops = gradient
        .stops()
        .iter()
        .map(|(offset, rgba)| {
            Json::Array(vec![
                Json::Number(*offset),
                Json::String(color_text(Color::from_bytes(*rgba))),
            ])
        })
        .collect();
    let mut members = vec![("stops", Json::Array(stops))];
    match gradient.kind() {
        GradientKind::Linear { start, end } => members.extend([
            ("gradient", Json::text("linear")),
            ("from", pair(start)),
            ("to", pair(end)),
        ]),
        GradientKind::Radial { center, radius } => members.extend([
            ("gradient", Json::text("radial")),
            ("center", pair(center)),
            ("radius", Json::Number(radius)),
        ]),
    }
    Json::object(members)
}

/// A `shape` source's value: its path and the attributes its generators wrote.
fn shape_json(shape: &Shape) -> Json {
    let attributes: Vec<(&str, Json)> = SELECTABLE
        .iter()
        .filter(|(name, _)| *name != "index")
        .filter_map(|(name, _)| shape.attributes().get(name).map(|single| (*name, single_json(&single))))
        .collect();
    let mut members = vec![("path", Json::String(shape.path().to_svg_path()))];
    if !attributes.is_empty() {
        members.push(("attributes", Json::object(attributes)));
    }
    Json::object(members)
}

fn single_json(single: &Single) -> Json {
    match single {
        Single::Number(number) => Json::Number(*number),
        Single::Char(character) => Json::String(character.to_string()),
        Single::Point(point) => Json::Array(vec![Json::Number(point.x), Json::Number(point.y)]),
        Single::Text(text) => Json::String(text.clone()),
    }
}

/// A selection: each attribute to its value, or to a list of values when it asks for more or fewer than one.
fn select_json(select: &Select) -> Json {
    Json::object(select.asked().map(|(name, wanted)| {
        let value = match wanted {
            [one] => single_json(one),
            many => Json::Array(many.iter().map(single_json).collect()),
        };
        (name, value)
    }))
}

/// The values of a `with_values`: each Param to a number or a list of numbers.
fn values_json(values: &Values) -> Json {
    Json::object(values.iter().map(|(name, value)| {
        let json = match value {
            Value::Number(number) => Json::Number(*number),
            Value::List(numbers) => Json::Array(numbers.iter().map(|number| Json::Number(*number)).collect()),
        };
        (name, json)
    }))
}
