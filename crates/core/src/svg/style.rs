//! The presentation properties an element carries (attributes and the inline `style`) and what a group hands down.

use crate::{Cap, Color, Join};
use std::collections::BTreeMap;

/// The properties that are read.
const READ: [&str; 9] = [
    "fill",
    "fill-opacity",
    "stroke",
    "stroke-opacity",
    "stroke-width",
    "stroke-linejoin",
    "stroke-linecap",
    "stroke-miterlimit",
    "opacity",
];

/// The properties that change what is drawn and are not read: each with the values that change nothing, and what to
/// do instead. Any other value raises.
const REFUSED: [(&str, &[&str], &str); 12] = [
    (
        "fill-rule",
        &["nonzero"],
        "only the nonzero rule is read, because the renderer fills nonzero only; split the shape into separate paths that need no evenodd",
    ),
    ("clip-path", &["none"], "clip paths are not read"),
    ("mask", &["none"], "masks are not read"),
    ("filter", &["none"], "filters are not read"),
    ("marker", &["none"], "markers are not read"),
    ("marker-start", &["none"], "markers are not read"),
    ("marker-mid", &["none"], "markers are not read"),
    ("marker-end", &["none"], "markers are not read"),
    ("stroke-dasharray", &["none"], "dashes are not read"),
    ("mix-blend-mode", &["normal"], "blend modes are not read"),
    (
        "display",
        &["inline", "block"],
        "hidden elements are not read: delete them",
    ),
    ("visibility", &["visible"], "hidden elements are not read: delete them"),
];

/// The first refusal among `declared`, as `name="value": why`.
pub(super) fn refusal(declared: &BTreeMap<String, String>) -> Option<String> {
    REFUSED.iter().find_map(|(name, neutral, advice)| {
        let value = declared.get(*name)?;
        (!neutral.contains(&value.as_str())).then(|| format!("has {name}=\"{value}\": {advice}"))
    })
}

/// The properties named in `attributes` and in the declarations of `style`, the declarations winning, among those
/// that are read or refused. Others (`data-*`, `isolation`, `shape-rendering`, an editor's own) change nothing and are
/// left out.
///
/// # Errors
/// A declaration of `style` that has no colon.
pub(super) fn declared<'a>(
    attributes: impl Iterator<Item = (&'a str, &'a str)>,
    style: Option<&str>,
) -> Result<BTreeMap<String, String>, String> {
    let known = |name: &str| READ.contains(&name) || REFUSED.iter().any(|(refused, _, _)| *refused == name);
    let mut found: BTreeMap<String, String> = attributes
        .filter(|(name, _)| known(name))
        .map(|(name, value)| (name.to_owned(), value.trim().to_owned()))
        .collect();
    for declaration in style
        .unwrap_or_default()
        .split(';')
        .map(str::trim)
        .filter(|d| !d.is_empty())
    {
        let (name, value) = declaration
            .split_once(':')
            .ok_or_else(|| format!("has a style declaration without a colon: {declaration:?}"))?;
        if known(name.trim()) {
            found.insert(name.trim().to_owned(), value.trim().to_owned());
        }
    }
    Ok(found)
}

/// What is painted and how, as a group hands it down to what it holds.
#[derive(Debug, Clone)]
pub(super) struct Painting {
    pub(super) fill: Color,
    pub(super) fill_opacity: f64,
    pub(super) stroke: Color,
    pub(super) stroke_opacity: f64,
    pub(super) stroke_width: f64,
    pub(super) join: Join,
    pub(super) cap: Cap,
    pub(super) miter_limit: f64,
}

impl Default for Painting {
    /// SVG's own defaults: filled black, not stroked, a stroke 1 wide with mitered joins and butt caps.
    fn default() -> Self {
        Self {
            fill: Color::hex(0),
            fill_opacity: 1.0,
            stroke: Color::NONE,
            stroke_opacity: 1.0,
            stroke_width: 1.0,
            join: Join::Miter,
            cap: Cap::Butt,
            miter_limit: 4.0,
        }
    }
}

impl Painting {
    /// This painting with the properties `declared` set over it.
    ///
    /// # Errors
    /// The property that cannot be read, as a phrase to follow the element.
    pub(super) fn with(mut self, declared: &BTreeMap<String, String>) -> Result<Self, String> {
        for (name, value) in declared {
            let value = value.as_str();
            match name.as_str() {
                "fill" => self.fill = colour(name, value)?,
                "stroke" => self.stroke = colour(name, value)?,
                "fill-opacity" => self.fill_opacity = fraction(name, value)?,
                "stroke-opacity" => self.stroke_opacity = fraction(name, value)?,
                "stroke-width" => self.stroke_width = length(name, value)?,
                "stroke-miterlimit" => self.miter_limit = number(name, value)?,
                "stroke-linejoin" => {
                    self.join = match value {
                        "miter" => Join::Miter,
                        "round" => Join::Round,
                        "bevel" => Join::Bevel,
                        _ => return Err(format!("has {name}=\"{value}\": it is miter, round or bevel")),
                    };
                }
                "stroke-linecap" => {
                    self.cap = match value {
                        "butt" => Cap::Butt,
                        "round" => Cap::Round,
                        "square" => Cap::Square,
                        _ => return Err(format!("has {name}=\"{value}\": it is butt, round or square")),
                    };
                }
                _ => {}
            }
        }
        Ok(self)
    }
}

/// The element's own `opacity`, which is not handed down.
///
/// # Errors
/// A value that is not a number from 0 to 1.
pub(super) fn opacity(declared: &BTreeMap<String, String>) -> Result<f64, String> {
    declared
        .get("opacity")
        .map_or(Ok(1.0), |value| fraction("opacity", value))
}

/// `color` with its alpha multiplied by `opacity`.
pub(super) fn faded(color: Color, opacity: f64) -> Color {
    color.rgba().map_or(color, |[red, green, blue, alpha]| {
        Color::from_bytes([red, green, blue, scaled(f64::from(alpha) * opacity)])
    })
}

/// A byte from `value`, which is from 0 to 255.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "an alpha times an opacity from 0 to 1 is rounded and within 0 to 255"
)]
fn scaled(value: f64) -> u8 {
    value.round() as u8
}

fn number(name: &str, value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("has {name}=\"{value}\": it is not a number"))
}

fn fraction(name: &str, value: &str) -> Result<f64, String> {
    let number = number(name, value)?;
    if (0.0..=1.0).contains(&number) {
        Ok(number)
    } else {
        Err(format!("has {name}=\"{value}\": it is from 0 to 1"))
    }
}

fn length(name: &str, value: &str) -> Result<f64, String> {
    let number = number(name, value.strip_suffix("px").unwrap_or(value))?;
    if number >= 0.0 {
        Ok(number)
    } else {
        Err(format!("has {name}=\"{value}\": it is not negative"))
    }
}

/// A colour as `#rgb`, `#rrggbb`, `#rrggbbaa` or `none`.
fn colour(name: &str, value: &str) -> Result<Color, String> {
    if let Some(digits) = value.strip_prefix('#').filter(|digits| digits.len() == 3) {
        let doubled: String = digits.chars().flat_map(|digit| [digit, digit]).collect();
        return Color::parse(&format!("#{doubled}")).map_err(|_| unreadable(name, value));
    }
    if value.starts_with("url(") {
        return Err(format!(
            "has {name}=\"{value}\": gradients and patterns are not read; paint with one colour"
        ));
    }
    Color::parse(value).map_err(|_| unreadable(name, value))
}

fn unreadable(name: &str, value: &str) -> String {
    format!(
        "has {name}=\"{value}\": a colour is read as #rgb, #rrggbb, #rrggbbaa or none (no names, no rgb(), no currentColor)"
    )
}
