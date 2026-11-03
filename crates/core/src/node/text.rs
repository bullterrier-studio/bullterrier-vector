//! `text`: a string in a font, one shape for every glyph with an outline.

use super::{Args, Context, Inputs, Node, choice};
use crate::limits::{MAX_GLYPHS, MAX_SETTINGS};
use crate::path::Region;
use crate::r#type::{Align, Boundary, Fit, Settings, Wrap, typeset};
use crate::{Arg, Domain, Error, Kind, Literal, Point, Reads, Role, Shape, Values, Verb};

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
            name: "axes",
            kind: Kind::Axes,
            default: Some(Literal::Unset),
            reads: Reads::PerItem,
            domain: Domain::Finite,
        },
        Arg {
            name: "features",
            kind: Kind::Features,
            default: Some(Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::Any,
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
            domain: Domain::Choice(&Align::NAMES),
        },
        Arg {
            name: "anchor",
            kind: Kind::Point,
            default: Some(Literal::Unset),
            reads: Reads::Once,
            domain: Domain::Coordinate,
        },
        Arg {
            name: "width",
            kind: Kind::Number,
            default: Some(Literal::Unset),
            reads: Reads::Once,
            domain: Domain::Positive,
        },
        Arg {
            name: "fit",
            kind: Kind::Text,
            default: Some(Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::Any,
        },
        Arg {
            name: "inside",
            kind: Kind::Path,
            default: Some(Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::Any,
        },
    ],
    check: Some(text_fits),
};

/// A text has at most `MAX_GLYPHS` characters, and the axes and features it names are the font's, with the values of
/// the axes that read no Param in the font's range (those that read one are checked when drawn).
fn text_fits(args: &Args) -> Result<(), Error> {
    let characters = args.text("content")?.chars().count();
    if characters > MAX_GLYPHS {
        return Err(Error::parameter(
            "content",
            format!("has {characters} characters, but MAX_GLYPHS {MAX_GLYPHS} is the most accepted"),
        ));
    }
    let font = args.font("font")?;
    let axes = args.written_axes("axes")?;
    let features = args.features("features")?;
    for (name, given) in [("axes", axes.len()), ("features", features.len())] {
        if given > MAX_SETTINGS {
            return Err(Error::parameter(
                name,
                format!("has {given} entries, but MAX_SETTINGS {MAX_SETTINGS} is the most accepted"),
            ));
        }
    }
    if args.given("inside")? {
        // the shape sets the lines and where they stand
        for excluded in ["width", "fit", "anchor"] {
            if args.given(excluded)? {
                return Err(Error::parameter(
                    "inside",
                    format!(
                        "is given with {excluded}, but a text inside a shape takes its lines and place from the shape: give one of them"
                    ),
                ));
            }
        }
        let leading = args.expressions().find(|(arg, _)| arg.name == "leading");
        if let Some(leading) = leading.and_then(|(_, expr)| expr.constant())
            && leading <= 0.0
        {
            return Err(Error::parameter(
                "leading",
                format!("is {leading}, but a text inside a shape needs a leading above 0"),
            ));
        }
    }
    if let Some(fit) = args.text_opt("fit")? {
        if !args.given("width")? {
            return Err(Error::parameter(
                "fit",
                format!("is {fit:?}, but a fit needs a width to fit the lines to"),
            ));
        }
        if fit != "size" {
            font.axis_range(fit, "fit")?;
            if axes.iter().any(|(tag, _)| tag == fit) {
                return Err(Error::parameter(
                    "fit",
                    format!("is the axis {fit:?}, which axes gives a value too: give one of them"),
                ));
            }
        }
    }
    for (tag, expr) in axes {
        font.check_axis(tag, expr.constant())?;
    }
    for (tag, _) in features {
        font.check_feature(tag)?;
    }
    Ok(())
}

/// The arguments of the call, read for this draw. The error is unplaced.
fn settings(args: &Args, cx: &Context<'_>) -> Result<Settings, Error> {
    let characters = args.text("content")?.chars().count();
    Ok(Settings {
        size: args.once_number(cx, "size")?,
        tracking: args.once_number(cx, "tracking")?,
        leading: args.once_number_opt(cx, "leading")?,
        align: choice(args, "align", Align::named)?,
        anchor: args.once_point(cx, "anchor")?.unwrap_or(Point::new(0.0, 0.0)),
        axes: args.axes(cx, "axes", characters)?,
        features: args.features("features")?.to_vec(),
    })
}

/// The wrapping the call asks for, or `None` if it gives no `width`: the width as drawn, and everything else the lines
/// depend on at the reference pose. The error is unplaced.
fn wrap(args: &Args, cx: &Context<'_>) -> Result<Option<Wrap>, Error> {
    let reference = Values::new();
    let pose = Context { env: &reference, ..*cx };
    let boundary = if args.given("inside")? {
        let region = Region::of(args.path("inside")?);
        cx.work.made_segments(region.len(), &cx.place.to_string())?;
        Boundary::Inside(region)
    } else if let Some(width) = args.once_number_opt(cx, "width")? {
        Boundary::Width {
            width,
            fit: args.text_opt("fit")?.map(|fit| {
                if fit == "size" {
                    Fit::Size
                } else {
                    Fit::Axis(fit.to_owned())
                }
            }),
        }
    } else {
        return Ok(None);
    };
    Ok(Some(Wrap {
        pose: settings(args, &pose)?,
        boundary,
    }))
}

impl Node for TextNode {
    fn verb(&self) -> &'static Verb {
        &TEXT
    }

    fn evaluate(&self, args: &Args, _: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let placed = |error: Error| error.at_argument(&cx.place, None);
        let settings = settings(args, cx).map_err(placed)?;
        let wrap = wrap(args, cx).map_err(placed)?;
        typeset(args.font("font")?, args.text("content")?, &settings, wrap.as_ref(), cx)
    }
}
