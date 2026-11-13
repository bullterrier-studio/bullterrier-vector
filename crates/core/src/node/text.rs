//! `text`: a string in a font, one shape for every glyph with an outline.

use super::{Args, Context, Inputs, Node, choice};
use crate::limits::{MAX_GLYPHS, MAX_RUNS, MAX_SETTINGS};
use crate::path::Region;
use crate::r#type::{Align, Boundary, Fit, RunSettings, Settings, Wrap, typeset};
use crate::{Arg, Domain, Error, Kind, Literal, Point, Reads, Role, Shape, Values, Verb};

/// The shapes of a string.
#[derive(Debug)]
pub(crate) struct TextNode;

pub(crate) static TEXT: Verb = Verb {
    name: "text",
    role: Role::Source,
    summary: "one shape for every glyph of the string (or of its runs, each in its own font) that has an outline, shaped with the font's kerning and ligatures, with glyph, char, word, line, run, order and origin",
    args: &[
        Arg {
            name: "content",
            kind: Kind::Text,
            default: Some(Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::Any,
        },
        Arg {
            name: "font",
            kind: Kind::Font,
            default: Some(Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::Any,
        },
        Arg {
            name: "runs",
            kind: Kind::Runs,
            default: Some(Literal::Unset),
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

/// One run of a text made of runs: the arguments of a record of `runs`, not a call of its own. It is a source in
/// nothing: it is never registered, never written as a call, and draws nothing.
pub(crate) static RUN: Verb = Verb {
    name: "run",
    role: Role::Source,
    summary: "a stretch of a text with a font of its own, shaped apart from its neighbours; size and tracking default to the text's",
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
            default: Some(Literal::Unset),
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
            default: Some(Literal::Unset),
            reads: Reads::Once,
            domain: Domain::Finite,
        },
    ],
    check: Some(face_fits),
};

/// The axes and features a font is given are its own, in the numbers of settings accepted, with the values of the axes
/// that read no Param in the font's range (those that read one are checked when drawn). `args` is a `run` or a plain
/// `text`.
fn face_fits(args: &Args) -> Result<(), Error> {
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
    for (tag, expr) in axes {
        font.check_axis(tag, expr.constant())?;
    }
    for (tag, _) in features {
        font.check_feature(tag)?;
    }
    Ok(())
}

/// What a text is made of, as the arguments that carry a font: its runs, or the call itself if it has none.
fn faces(args: &Args) -> Result<Vec<&Args>, Error> {
    if args.given("runs")? {
        Ok(args.runs("runs")?.iter().collect())
    } else {
        Ok(vec![args])
    }
}

/// A text is the one string of a font, or runs with fonts of their own (at least one and at most `MAX_RUNS`, the
/// options giving none of their own); it has at most `MAX_GLYPHS` characters; and the axes and features of each face
/// are its font's.
fn text_fits(args: &Args) -> Result<(), Error> {
    let faces = faces(args)?;
    if args.given("runs")? {
        for given in ["content", "font", "axes", "features"] {
            if args.given(given)? {
                return Err(Error::parameter(
                    given,
                    "is given with runs, but a text of runs takes its content, fonts, axes and features from the runs: give one of them",
                ));
            }
        }
        if faces.is_empty() || faces.len() > MAX_RUNS {
            return Err(Error::parameter(
                "runs",
                format!("has {} runs, but a text takes 1 to MAX_RUNS {MAX_RUNS}", faces.len()),
            ));
        }
    } else {
        for needed in ["content", "font"] {
            if !args.given(needed)? {
                return Err(Error::parameter(
                    needed,
                    "is missing: a text is a content in a font, or runs",
                ));
            }
        }
        face_fits(args)?;
    }
    let mut characters = 0_usize;
    for face in &faces {
        characters = characters.saturating_add(face.text("content")?.chars().count());
    }
    if characters > MAX_GLYPHS {
        return Err(Error::parameter(
            "content",
            format!("has {characters} characters, but MAX_GLYPHS {MAX_GLYPHS} is the most accepted"),
        ));
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
            let mut first = None;
            let mut found = false;
            for face in &faces {
                match face.font("font")?.axis_range(fit, "fit") {
                    Ok(_) => {
                        found = true;
                        if face.written_axes("axes")?.iter().any(|(tag, _)| tag == fit) {
                            return Err(Error::parameter(
                                "fit",
                                format!("is the axis {fit:?}, which axes gives a value too: give one of them"),
                            ));
                        }
                    }
                    Err(error) => {
                        first.get_or_insert(error);
                    }
                }
            }
            if let (false, Some(error)) = (found, first) {
                return Err(error);
            }
        }
    }
    Ok(())
}

/// The content of the call and its settings, read for this draw: the runs, each at its own size (the text's if it gives
/// none) and with the axes, features and tracking of its own. The error is unplaced.
fn settings(args: &Args, cx: &Context<'_>) -> Result<(String, Settings), Error> {
    let size = args.once_number(cx, "size")?;
    let tracking = args.once_number(cx, "tracking")?;
    let mut content = String::new();
    let mut runs = Vec::new();
    for (number, face) in faces(args)?.into_iter().enumerate() {
        let text = face.text("content")?;
        let start = runs.last().map_or(0, |run: &RunSettings| run.characters.end);
        let characters = text.chars().count();
        content.push_str(text);
        runs.push(RunSettings {
            font: face.font("font")?.clone(),
            characters: start..start + characters,
            size: face.once_number_opt(cx, "size")?.unwrap_or(size),
            tracking: face.once_number_opt(cx, "tracking")?.unwrap_or(tracking),
            axes: face
                .axes(cx, "axes", characters)
                .map_err(|error| in_run(error, number))?,
            features: face.features("features")?.to_vec(),
        });
    }
    Ok((
        content,
        Settings {
            runs,
            leading: args.once_number_opt(cx, "leading")?,
            align: choice(args, "align", Align::named)?,
            anchor: args.once_point(cx, "anchor")?.unwrap_or(Point::new(0.0, 0.0)),
        },
    ))
}

/// `error` with the run it came from named, if the call has runs.
fn in_run(error: Error, run: usize) -> Error {
    match error {
        Error::Parameter { parameter, message } => Error::parameter(&parameter, format!("run {run}: {message}")),
        other => other,
    }
}

/// The wrapping the call asks for, or `None` if it gives no `width`: the width as drawn, and everything else the lines
/// depend on at the reference pose. The error is unplaced.
fn wrap(args: &Args, cx: &Context<'_>) -> Result<Option<Wrap>, Error> {
    let reference = Values::new();
    let pose = Context { env: &reference, ..*cx };
    let boundary = if args.given("inside")? {
        let place = cx.place.to_string();
        Boundary::Inside(Region::of(args.path("inside")?, &mut |count| {
            cx.work.made_segments(count, &place)
        })?)
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
        pose: settings(args, &pose)?.1,
        boundary,
    }))
}

impl Node for TextNode {
    fn verb(&self) -> &'static Verb {
        &TEXT
    }

    fn evaluate(&self, args: &Args, _: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let placed = |error: Error| error.at_argument(&cx.place, None);
        let (content, settings) = settings(args, cx).map_err(placed)?;
        let wrap = wrap(args, cx).map_err(placed)?;
        typeset(&content, &settings, wrap.as_ref(), cx)
    }
}
