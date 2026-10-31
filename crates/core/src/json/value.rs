//! The JSON tree both the reader and the writer work on, and its canonical text (RFC 8785).

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// A JSON value. An object's keys are kept sorted by code point, which is the order the canonical text writes them in.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    /// A string value.
    pub(crate) fn text(text: &str) -> Self {
        Self::String(text.to_owned())
    }

    /// An object of `members`.
    pub(crate) fn object<'a>(members: impl IntoIterator<Item = (&'a str, Json)>) -> Self {
        Self::Object(
            members
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    /// What kind of value this is, for an error: `a number`, `an object`...
    pub(crate) fn describe(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "a boolean",
            Self::Number(_) => "a number",
            Self::String(_) => "a string",
            Self::Array(_) => "an array",
            Self::Object(_) => "an object",
        }
    }

    /// The canonical text of this value: keys sorted, no whitespace, numbers in their shortest round-trip form.
    pub(crate) fn canonical(&self) -> String {
        let mut text = String::new();
        self.write_to(&mut text);
        text
    }

    /// Appends the canonical text. The recursion is as deep as the tree, which the reader and the writer keep within
    /// `MAX_DEPTH` levels of nesting plus one expression's depth.
    fn write_to(&self, into: &mut String) {
        match self {
            Self::Null => into.push_str("null"),
            Self::Bool(true) => into.push_str("true"),
            Self::Bool(false) => into.push_str("false"),
            Self::Number(number) => into.push_str(&number_text(*number)),
            Self::String(text) => string_text(text, into),
            Self::Array(items) => {
                into.push('[');
                for (at, item) in items.iter().enumerate() {
                    if at > 0 {
                        into.push(',');
                    }
                    item.write_to(into);
                }
                into.push(']');
            }
            Self::Object(members) => {
                into.push('{');
                for (at, (key, value)) in members.iter().enumerate() {
                    if at > 0 {
                        into.push(',');
                    }
                    string_text(key, into);
                    into.push(':');
                    value.write_to(into);
                }
                into.push('}');
            }
        }
    }
}

/// `value` as ECMAScript's `Number::toString` writes it, as RFC 8785 requires: the shortest digits that read back as the
/// same number, with an exponent only below 1e-6 and from 1e21; `-0` is `0`. The value must be finite.
pub(crate) fn number_text(value: f64) -> String {
    if value == 0.0 {
        return String::from("0");
    }
    // `{:e}` writes the shortest digits that round-trip, as `1.2345e-7`; it always has the `e` and a whole exponent
    let scientific = format!("{:e}", value.abs());
    let Some((mantissa, exponent)) = scientific.split_once('e') else {
        return value.to_string();
    };
    let Ok(exponent) = exponent.parse::<i32>() else {
        return value.to_string();
    };
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let sign = if value < 0.0 { "-" } else { "" };
    // the decimal point stands after `point` of the digits
    let point = exponent + 1;
    let count = i32::try_from(digits.len()).unwrap_or(i32::MAX);
    let zeros = |how_many: i32| "0".repeat(how_many.unsigned_abs() as usize);
    let body = if count <= point && point <= 21 {
        format!("{digits}{}", zeros(point - count))
    } else if 0 < point && point <= 21 {
        let (whole, fraction) = digits.split_at(point.unsigned_abs() as usize);
        format!("{whole}.{fraction}")
    } else if -6 < point && point <= 0 {
        format!("0.{}{digits}", zeros(point))
    } else {
        let shift = point - 1;
        let exponent = if shift < 0 {
            format!("-{}", shift.unsigned_abs())
        } else {
            format!("+{shift}")
        };
        match digits.split_at(1) {
            (first, "") => format!("{first}e{exponent}"),
            (first, rest) => format!("{first}.{rest}e{exponent}"),
        }
    };
    format!("{sign}{body}")
}

/// `text` as a JSON string with the fewest escapes: `"`, `\` and the control characters; everything else as it is.
fn string_text(text: &str, into: &mut String) {
    into.push('"');
    for character in text.chars() {
        match character {
            '"' => into.push_str("\\\""),
            '\\' => into.push_str("\\\\"),
            '\u{8}' => into.push_str("\\b"),
            '\t' => into.push_str("\\t"),
            '\n' => into.push_str("\\n"),
            '\u{c}' => into.push_str("\\f"),
            '\r' => into.push_str("\\r"),
            control if control < ' ' => {
                // writing to a String does not fail
                let _ = write!(into, "\\u{:04x}", u32::from(control));
            }
            other => into.push(other),
        }
    }
    into.push('"');
}

/// `segment` as one reference token of a JSON Pointer (RFC 6901): `~` is `~0` and `/` is `~1`.
pub(crate) fn pointer_token(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}
