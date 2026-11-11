//! `Drawing::from_svg`: an SVG file read into a drawing that keeps its ids, classes and groups as attributes.

mod affine;
mod shapes;
mod style;

use self::affine::Affine;
use self::shapes::Failure;
use self::style::Painting;
use crate::limits::{MAX_DOCUMENT_BYTES, MAX_SVG_DEPTH, MAX_SVG_ELEMENTS, MAX_SVG_NODES};
use crate::{Drawing, Error, Path, Shape, Stroke};
use roxmltree::{Document, Node, ParsingOptions};
use std::sync::Arc;

/// The namespace every element read is in.
const NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The elements that draw one shape.
const SHAPES: [&str; 7] = ["path", "rect", "circle", "ellipse", "line", "polyline", "polygon"];

/// The elements that hold no drawing of their own and are skipped with what is inside them.
const NOT_CONTENT: [&str; 3] = ["title", "desc", "metadata"];

/// What an element inherits from the elements round it.
#[derive(Clone)]
struct Inherited {
    ctm: Affine,
    painting: Painting,
    /// The ids of the groups round it, joined by `/`.
    group: Option<String>,
}

/// The error of a document with more than `MAX_SVG_ELEMENTS` elements, or more nodes than `MAX_SVG_NODES`.
fn too_many_elements() -> Error {
    Error::parameter(
        "svg",
        format!(
            "holds more than MAX_SVG_ELEMENTS {MAX_SVG_ELEMENTS} elements (the text and comments between them count \
             toward MAX_SVG_NODES {MAX_SVG_NODES})"
        ),
    )
}

/// How an element reads in an error: its name, and its id if it has one.
fn describe(node: Node<'_, '_>) -> String {
    match node.attribute("id") {
        Some(id) => format!("<{} id=\"{id}\">", node.tag_name().name()),
        None => format!("<{}>", node.tag_name().name()),
    }
}

fn refuse(node: Node<'_, '_>, message: impl std::fmt::Display) -> Error {
    Error::parameter("svg", format!("{} {message}", describe(node)))
}

/// What to do instead, for an element that is not read.
fn instead(name: &str) -> &'static str {
    match name {
        "use" => "write the shape out where it is used",
        "text" => "convert the text to outlines",
        "style" => "CSS stylesheets are not read: set fill and stroke on each element",
        "clipPath" | "mask" => "clip paths and masks are not read",
        "linearGradient" | "radialGradient" | "pattern" => "gradients and patterns are not read: paint with one colour",
        "svg" => "nested svg elements are not read",
        _ => "only svg, g and the shapes are read",
    }
}

impl Drawing {
    /// The drawing an SVG document describes: one shape for each `path`, `rect`, `circle`, `ellipse`, `line`,
    /// `polyline` and `polygon`, in document order, with the `id`, `class` and `group` attributes of the elements, so
    /// that a part of a logo is picked by name:
    ///
    /// ```text
    /// let logo = Drawing::from_svg(&text)?;
    /// let ring = logo.only(Select::new().attribute("id", "ring"));
    /// ```
    ///
    /// A shape has `id` and `class` as written on its element, and `group`, the ids of the `g` elements round it from
    /// the outside in, joined by `/` (`logo/mark`); a group without an id adds nothing, and an attribute the element
    /// lacks is absent. `only(group="logo")` matches exactly: a shape in `logo/mark` is not in it. Every shape also
    /// has its `index`.
    ///
    /// The `viewBox` is the drawing's size (without one, `width` and `height` in pixels), shapes are moved so that its
    /// corner is the origin, and `transform`s (`matrix`, `translate`, `scale`, `rotate`, `skewX`, `skewY`) are worked
    /// into the points. `fill` (black unless set), `fill-opacity`, `stroke` (none unless set), `stroke-opacity`,
    /// `stroke-width`, `stroke-linejoin`, `stroke-linecap`, `stroke-miterlimit`, as presentation attributes or in
    /// `style`, are handed down groups; `opacity` is read on shapes. Colours are `#rgb`, `#rrggbb`, `#rrggbbaa` or
    /// `none`. Lengths are plain numbers or pixels. `title`, `desc`, `metadata`, comments, an empty `defs`, elements
    /// of other namespaces (an editor's own) and attributes that change nothing drawn are skipped.
    ///
    /// What is not read is an error that names the element or the attribute and what to do instead, never dropped:
    /// `fill-rule="evenodd"` (the renderer fills nonzero only: split the shape into paths that need no evenodd),
    /// `use`, `symbol`, `defs` holding anything, gradients, patterns, `clip-path`, `mask`, `filter`, markers, dashes,
    /// `paint-order`, `vector-effect`, `transform-origin`, a `<!DOCTYPE>` (remove the line), `text`, `image`, `style` elements, `<?xml-stylesheet?>`, a `transform` in `style`, nested `svg`, `opacity` on a `g`, lengths in other units, a
    /// stroke under a transform that stretches one way (the stroke cannot follow it), unequal `rx` and `ry`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `svg`: the text is not XML, not an SVG document, longer than `MAX_DOCUMENT_BYTES`
    /// (16 MiB), holds more than `MAX_SVG_ELEMENTS` (20,000) elements or nests them past `MAX_SVG_DEPTH` (128), has
    /// something that is not read (above), or a value a shape refuses.
    pub fn from_svg(text: &str) -> Result<Self, Error> {
        if text.len() > MAX_DOCUMENT_BYTES {
            return Err(Error::parameter(
                "svg",
                format!("is {} bytes, past MAX_DOCUMENT_BYTES {MAX_DOCUMENT_BYTES}", text.len()),
            ));
        }
        let options = ParsingOptions {
            nodes_limit: MAX_SVG_NODES,
            ..ParsingOptions::default()
        };
        let document = Document::parse_with_options(text, options).map_err(|error| match error {
            roxmltree::Error::NodesLimitReached => too_many_elements(),
            roxmltree::Error::DtdDetected => Error::parameter(
                "svg",
                "starts with a <!DOCTYPE>, which is not read (its entities could expand without bound); remove the <!DOCTYPE> line",
            ),
            other => Error::parameter("svg", format!("is not XML: {other}")),
        })?;
        let root = svg_root(&document)?;
        let Frame {
            left,
            top,
            width,
            height,
        } = frame(root)?;
        let mut items = Vec::new();
        let mut count = 0;
        let mut pending = vec![(
            root,
            1,
            Inherited {
                ctm: Affine::translate(-left, -top),
                painting: Painting::default(),
                group: None,
            },
        )];
        while let Some((node, depth, parent)) = pending.pop() {
            count += 1;
            if count > MAX_SVG_ELEMENTS {
                return Err(too_many_elements());
            }
            if depth > MAX_SVG_DEPTH {
                return Err(refuse(node, format!("is nested past MAX_SVG_DEPTH {MAX_SVG_DEPTH}")));
            }
            let name = node.tag_name().name();
            if node.tag_name().namespace() != Some(NAMESPACE) || NOT_CONTENT.contains(&name) {
                continue;
            }
            if name == "defs" {
                if let Some(content) = node
                    .children()
                    .find(|child| child.is_element() && child.tag_name().namespace() == Some(NAMESPACE))
                {
                    return Err(refuse(
                        node,
                        format!(
                            "holds {}: definitions are used by <use>, gradients and clip paths, which are not read",
                            describe(content)
                        ),
                    ));
                }
                continue;
            }
            let is_group = name == "g" || (name == "svg" && depth == 1);
            if !is_group && !SHAPES.contains(&name) {
                return Err(refuse(node, format!("is not read: {}", instead(name))));
            }
            let (inherited, opacity) = inherit(node, &parent)?;
            if is_group {
                if opacity.total_cmp(&1.0).is_ne() {
                    return Err(refuse(
                        node,
                        "has opacity: a group's opacity fades it as one picture, which is not read; set opacity on the shapes",
                    ));
                }
                let mut inside = inherited;
                if name == "g" {
                    inside.group = match (parent.group, node.attribute("id").filter(|id| !id.is_empty())) {
                        (Some(outer), Some(id)) => Some(format!("{outer}/{id}")),
                        (None, Some(id)) => Some(id.to_owned()),
                        (outer, None) => outer,
                    };
                }
                for child in node.children().filter(Node::is_element).rev() {
                    pending.push((child, depth + 1, inside.clone()));
                }
            } else {
                items.push(drawn(node, &inherited, opacity)?);
            }
        }
        let drawing = Self::new(items).with_size(width, height);
        match drawing.error() {
            Some(error) => Err(Error::parameter("svg", format!("cannot be drawn: {}", error.reason()))),
            None => Ok(drawing),
        }
    }
}

/// The `svg` element of `document`.
///
/// # Errors
/// The document starts with a stylesheet instruction, or its root is not `svg` in the SVG namespace.
fn svg_root<'a, 'i>(document: &'a Document<'i>) -> Result<Node<'a, 'i>, Error> {
    if let Some(instruction) = document
        .root()
        .children()
        .find(|node| node.pi().is_some_and(|pi| pi.target == "xml-stylesheet"))
    {
        return Err(Error::parameter(
            "svg",
            format!(
                "starts with <?xml-stylesheet?> at byte {}: CSS stylesheets are not read; set fill and stroke on each element",
                instruction.range().start
            ),
        ));
    }
    let root = document.root_element();
    if root.tag_name().name() != "svg" || root.tag_name().namespace() != Some(NAMESPACE) {
        return Err(Error::parameter(
            "svg",
            format!(
                "starts with <{}>, not <svg xmlns=\"{NAMESPACE}\">",
                root.tag_name().name()
            ),
        ));
    }
    Ok(root)
}

/// The corner and the size of the frame of a drawing.
struct Frame {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

/// The corner and the size of the frame the `viewBox` of `root` gives, else the `width` and `height` in pixels.
fn frame(root: Node<'_, '_>) -> Result<Frame, Error> {
    let numbers = |name: &str, text: &str| -> Result<Vec<f64>, Error> {
        text.split(|c: char| c.is_whitespace() || c == ',')
            .filter(|word| !word.is_empty())
            .map(|word| {
                word.trim_end_matches("px")
                    .parse::<f64>()
                    .ok()
                    .filter(|number| number.is_finite())
                    .ok_or_else(|| refuse(root, format!("has {name}=\"{text}\": it is read as numbers of pixels")))
            })
            .collect()
    };
    if let Some(text) = root.attribute("viewBox") {
        return match numbers("viewBox", text)?.as_slice() {
            &[left, top, width, height] => Ok(Frame {
                left,
                top,
                width,
                height,
            }),
            _ => Err(refuse(
                root,
                format!("has viewBox=\"{text}\": it is min-x min-y width height"),
            )),
        };
    }
    let one = |name: &str| match root
        .attribute(name)
        .map(|text| numbers(name, text))
        .transpose()?
        .as_deref()
    {
        Some(&[value]) => Ok(value),
        _ => Err(refuse(root, "needs a viewBox, or a width and a height in pixels")),
    };
    Ok(Frame {
        left: 0.0,
        top: 0.0,
        width: one("width")?,
        height: one("height")?,
    })
}

/// What `node` inherits and sets, and its own opacity.
fn inherit(node: Node<'_, '_>, parent: &Inherited) -> Result<(Inherited, f64), Error> {
    let declared = style::declared(
        node.attributes()
            .filter(|attribute| attribute.namespace().is_none())
            .map(|attribute| (attribute.name(), attribute.value())),
        node.attribute("style"),
    )
    .map_err(|why| refuse(node, why))?;
    if let Some(why) = style::refusal(&declared) {
        return Err(refuse(node, why));
    }
    let painting = parent
        .painting
        .clone()
        .with(&declared)
        .map_err(|why| refuse(node, why))?;
    let opacity = style::opacity(&declared).map_err(|why| refuse(node, why))?;
    let own = match node.attribute("transform") {
        Some(text) => {
            Affine::parse(text).map_err(|why| refuse(node, format!("has transform=\"{text}\", which {why}")))?
        }
        None => Affine::IDENTITY,
    };
    Ok((
        Inherited {
            ctm: parent.ctm.after(own),
            painting,
            group: parent.group.clone(),
        },
        opacity,
    ))
}

/// The drawing of the shape element `node`.
fn drawn(node: Node<'_, '_>, here: &Inherited, opacity: f64) -> Result<Drawing, Error> {
    let path = shapes::outline(node).map_err(|failure| match failure {
        Failure::Message(why) => refuse(node, why),
        Failure::Path(error) => refuse(node, error.reason()),
    })?;
    let path: Path = path
        .map_points(|point| Ok(here.ctm.apply(point)))
        .map_err(|error| refuse(node, error.reason()))?;
    let mut shape = Shape::from_path(path, 0);
    let attributes = shape.attributes_mut();
    let text = |name: &str| {
        node.attribute(name)
            .filter(|value| !value.is_empty())
            .map(Arc::<str>::from)
    };
    attributes.id = text("id");
    attributes.class = text("class");
    attributes.group = here.group.as_deref().map(Arc::from);
    let painting = &here.painting;
    let mut drawing = Drawing::from(shape).fill(style::faded(painting.fill, painting.fill_opacity));
    if painting.stroke != crate::Color::NONE && painting.stroke_width > 0.0 {
        let scale = here.ctm.even_scale().ok_or_else(|| {
            refuse(
                node,
                "has a stroke under a transform that stretches one way or skews, which a stroke cannot follow; draw it without the transform",
            )
        })?;
        drawing = drawing.stroke(
            Stroke::new(style::faded(painting.stroke, painting.stroke_opacity))
                .width(painting.stroke_width * scale)
                .join(painting.join)
                .cap(painting.cap)
                .miter_limit(painting.miter_limit),
        );
    }
    if opacity.total_cmp(&1.0).is_ne() {
        drawing = drawing.opacity(opacity);
    }
    match drawing.error() {
        Some(error) => Err(refuse(node, error.reason())),
        None => Ok(drawing),
    }
}
