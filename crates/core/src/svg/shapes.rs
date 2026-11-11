//! The outline an element draws, before its transform.

use crate::{Error, Path, Point, Scale};
use roxmltree::Node;

/// Why an element has no outline.
pub(super) enum Failure {
    /// An attribute that cannot be read, as a phrase to follow the element.
    Message(String),
    /// The path the attributes describe is refused.
    Path(Error),
}

impl From<Error> for Failure {
    fn from(error: Error) -> Self {
        Self::Path(error)
    }
}

fn message(text: impl Into<String>) -> Failure {
    Failure::Message(text.into())
}

/// The path of the shape element `node` (`path`, `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon`).
pub(super) fn outline(node: Node<'_, '_>) -> Result<Path, Failure> {
    let number = |name: &str| length(node, name);
    let needed = |name: &str| number(name)?.ok_or_else(|| message(format!("needs {name}")));
    match node.tag_name().name() {
        "path" => Ok(Path::from_svg_path(
            node.attribute("d").ok_or_else(|| message("needs d"))?,
        )?),
        "rect" => {
            let (width, height) = (needed("width")?, needed("height")?);
            let (rx, ry) = (number("rx")?, number("ry")?);
            let (rx, ry) = (rx.or(ry).unwrap_or(0.0), ry.or(rx).unwrap_or(0.0));
            if rx.total_cmp(&ry).is_ne() {
                return Err(message(format!(
                    "has rx=\"{rx}\" and ry=\"{ry}\": only round corners (rx equal to ry) are read"
                )));
            }
            // SVG limits a corner to half the side
            let radius = rx.min(width / 2.0).min(height / 2.0);
            Ok(Path::rect(
                number("x")?.unwrap_or(0.0),
                number("y")?.unwrap_or(0.0),
                width,
                height,
                radius,
            )?)
        }
        "circle" => Ok(Path::circle(point(node, "cx", "cy")?, needed("r")?)?),
        "ellipse" => {
            let (rx, ry) = (needed("rx")?, needed("ry")?);
            let middle = point(node, "cx", "cy")?;
            // a circle stretched down, about its middle
            Ok(Path::circle(middle, rx)?.scale(Scale::from((1.0, ry / rx)), Some(middle))?)
        }
        "line" => Ok(Path::line(point(node, "x1", "y1")?, point(node, "x2", "y2")?)?),
        name => {
            let text = node.attribute("points").ok_or_else(|| message("needs points"))?;
            let numbers = text
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter(|word| !word.is_empty())
                .map(|word| {
                    word.parse::<f64>()
                        .ok()
                        .filter(|number| number.is_finite())
                        .ok_or_else(|| message(format!("has points holding {word:?}, which is not a number")))
                })
                .collect::<Result<Vec<f64>, Failure>>()?;
            if numbers.len() % 2 != 0 {
                return Err(message(format!(
                    "has points of {} numbers: they come in pairs",
                    numbers.len()
                )));
            }
            let points = numbers.as_chunks::<2>().0.iter().map(|&[x, y]| Point::new(x, y));
            Ok(Path::polyline(points, name == "polygon")?)
        }
    }
}

fn point(node: Node<'_, '_>, x: &str, y: &str) -> Result<Point, Failure> {
    Ok(Point::new(
        length(node, x)?.unwrap_or(0.0),
        length(node, y)?.unwrap_or(0.0),
    ))
}

/// The length the attribute `name` holds: a number of pixels (`px` may follow), or `None` if it is not there.
fn length(node: Node<'_, '_>, name: &str) -> Result<Option<f64>, Failure> {
    node.attribute(name)
        .map(|text| {
            text.trim()
                .trim_end_matches("px")
                .parse::<f64>()
                .ok()
                .filter(|number| number.is_finite())
                .ok_or_else(|| {
                    message(format!(
                        "has {name}=\"{text}\": a length is read as a number of pixels (no other unit, no percent)"
                    ))
                })
        })
        .transpose()
}
