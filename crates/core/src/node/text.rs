//! `text`: a string in a font, one shape for every glyph with an outline.

use super::{Args, Context, Inputs, Node, choice};
use crate::limits::MAX_GLYPHS;
use crate::r#type::{Align, Settings, typeset};
use crate::{Arg, Domain, Error, Kind, Literal, Point, Reads, Role, Shape, Verb};

/// The shapes of a string.
#[derive(Debug)]
pub(crate) struct TextNode;

pub(crate) static TEXT: Verb = Verb {
    name: "text",
    role: Role::Source,
    summary: "one shape for every glyph of the string that has an outline, shaped with the font's kerning and ligatures, with glyph, char, word, line, order and origin",
    args: &[
        Arg {
            name: "content",
            kind: Kind::Text,
            default: None,
            reads: Reads::Literal,
            domain: Domain::Any,
        },
        Arg {
            name: "font",
            kind: Kind::Font,
            default: None,
            reads: Reads::Literal,
            domain: Domain::Any,
        },
        Arg {
            name: "size",
            kind: Kind::Number,
            default: Some(Literal::Number(100.0)),
            reads: Reads::Once,
            domain: Domain::Positive,
        },
        Arg {
            name: "tracking",
            kind: Kind::Number,
            default: Some(Literal::Number(0.0)),
            reads: Reads::Once,
            domain: Domain::Finite,
        },
        Arg {
            name: "leading",
            kind: Kind::Number,
            default: Some(Literal::Unset),
            reads: Reads::Once,
            domain: Domain::Finite,
        },
        Arg {
            name: "align",
            kind: Kind::Text,
            default: Some(Literal::Text("left")),
            reads: Reads::Literal,
            domain: Domain::Choice(Align::NAMES),
        },
        Arg {
            name: "anchor",
            kind: Kind::Point,
            default: Some(Literal::Unset),
            reads: Reads::Once,
            domain: Domain::Coordinate,
        },
    ],
    check: Some(content_fits),
};

/// A text has at most `MAX_GLYPHS` characters.
fn content_fits(args: &Args) -> Result<(), Error> {
    let characters = args.text("content")?.chars().count();
    if characters > MAX_GLYPHS {
        return Err(Error::parameter(
            "content",
            format!("has {characters} characters, but MAX_GLYPHS {MAX_GLYPHS} is the most accepted"),
        ));
    }
    Ok(())
}

/// The arguments of the call, read for this draw. The error is unplaced.
fn settings(args: &Args, cx: &Context<'_>) -> Result<Settings, Error> {
    Ok(Settings {
        size: args.once_number(cx, "size")?,
        tracking: args.once_number(cx, "tracking")?,
        leading: args.once_number_opt(cx, "leading")?,
        align: choice(args, "align", Align::named)?,
        anchor: args.once_point(cx, "anchor")?.unwrap_or(Point::new(0.0, 0.0)),
    })
}

impl Node for TextNode {
    fn verb(&self) -> &'static Verb {
        &TEXT
    }

    fn evaluate(&self, args: &Args, _: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let settings = settings(args, cx).map_err(|error| error.at_argument(&cx.place, None))?;
        typeset(args.font("font")?, args.text("content")?, &settings, cx)
    }
}
