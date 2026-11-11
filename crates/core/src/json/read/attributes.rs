//! Reading the attributes of a `shape` and the selections `only` and `without` ask about.

use super::{at, number_at, refuse, string};
use crate::json::value::Json;
use crate::shape::{Holds, SELECTABLE, Single};
use crate::{Attributes, Error, Point, Select, Wanted};

/// The attributes a `shape` carries: each of the nine a generator writes, and nothing else.
pub(super) fn attributes(pointer: &str, json: &Json) -> Result<Attributes, Error> {
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
pub(super) fn whole_u32(number: f64) -> Option<u32> {
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

pub(super) fn point(pointer: &str, json: &Json) -> Result<Point, Error> {
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
pub(super) fn select(pointer: &str, json: &Json) -> Result<Select, Error> {
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
