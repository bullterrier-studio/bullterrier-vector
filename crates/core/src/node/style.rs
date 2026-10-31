//! The style steps: `fill`, `stroke`, `opacity`, `blend`. Each changes the paint of every shape and nothing else.

use super::{Args, Context, Inputs, Node, each_shape};
use crate::shape::{Blend as BlendMode, Cap, DEFAULT_MITER_LIMIT, DEFAULT_STROKE_WIDTH, Join, StrokeStyle};
use crate::{Arg, Domain, Error, Kind, Literal, Reads, Role, Shape, Verb};

/// A colour given when the call is made.
const COLOR: Arg = Arg {
    name: "color",
    kind: Kind::Color,
    default: None,
    reads: Reads::Literal,
    domain: Domain::Any,
};

/// The word `name` of a set, as the argument of a step, given when the call is made.
const fn word(name: &'static str, default: Option<&'static str>, options: &'static [&'static str]) -> Arg {
    Arg {
        name,
        kind: Kind::Text,
        default: match default {
            Some(text) => Some(Literal::Text(text)),
            None => None,
        },
        reads: Reads::Literal,
        domain: Domain::Choice(options),
    }
}

/// A number read for each shape, within `domain`.
const fn number(name: &'static str, default: Option<f64>, domain: Domain) -> Arg {
    Arg {
        name,
        kind: Kind::Number,
        default: match default {
            Some(value) => Some(Literal::Number(value)),
            None => None,
        },
        reads: Reads::PerItem,
        domain,
    }
}

/// The word of `name` read as one of `options`, found by `named`.
fn choice<T>(args: &Args, name: &str, named: impl Fn(&str) -> Option<T>) -> Result<T, Error> {
    let text = args.text(name)?;
    named(text).ok_or_else(|| Error::parameter(name, format!("is not one of the words it takes, got {text:?}")))
}

/// Fills every shape.
#[derive(Debug)]
pub(crate) struct Fill;

pub(crate) static FILL: Verb = Verb {
    name: "fill",
    role: Role::Step,
    summary: "fills every shape with a colour, \"none\" for no fill; once a fill or stroke is set only what is set is painted",
    args: &[COLOR],
    check: None,
};

impl Node for Fill {
    fn verb(&self) -> &'static Verb {
        &FILL
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, _: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let color = args.color("color")?;
        Ok(inputs
            .previous
            .iter()
            .cloned()
            .map(|mut shape| {
                shape.style_mut().set_fill(color);
                shape
            })
            .collect())
    }
}

/// Strokes every outline.
#[derive(Debug)]
pub(crate) struct Stroke;

pub(crate) static STROKE: Verb = Verb {
    name: "stroke",
    role: Role::Step,
    summary: "strokes every outline; once a fill or stroke is set only what is set is painted",
    args: &[
        COLOR,
        number("width", Some(DEFAULT_STROKE_WIDTH), Domain::Positive),
        word("join", Some("round"), Join::NAMES),
        word("cap", Some("round"), Cap::NAMES),
        number("miter_limit", Some(DEFAULT_MITER_LIMIT), Domain::AtLeast(1.0)),
    ],
    check: Some(paints),
};

/// A stroke of no colour paints nothing: that is a fill of none.
fn paints(args: &Args) -> Result<(), Error> {
    if args.color("color")?.rgba().is_none() {
        return Err(Error::parameter(
            "color",
            "must be \"#rrggbb\" or \"#rrggbbaa\": a stroke of \"none\" paints nothing",
        ));
    }
    Ok(())
}

impl Node for Stroke {
    fn verb(&self) -> &'static Verb {
        &STROKE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let color = args.color("color")?;
        let join = choice(args, "join", Join::named)?;
        let cap = choice(args, "cap", Cap::named)?;
        each_shape(inputs.previous, cx, |shape, item| {
            let width = args.number(cx, "width", item)?;
            let miter_limit = args.number(cx, "miter_limit", item)?;
            let mut shape = shape.clone();
            shape
                .style_mut()
                .set_stroke(StrokeStyle::new(color).finished(width, join, cap, miter_limit));
            Ok(shape)
        })
    }
}

/// Makes every shape more transparent.
#[derive(Debug)]
pub(crate) struct Opacity;

pub(crate) static OPACITY: Verb = Verb {
    name: "opacity",
    role: Role::Step,
    summary: "multiplies the opacity of every shape by alpha, from 0 to 1",
    args: &[number("alpha", None, Domain::Fraction)],
    check: None,
};

impl Node for Opacity {
    fn verb(&self) -> &'static Verb {
        &OPACITY
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        each_shape(inputs.previous, cx, |shape, item| {
            let alpha = args.number(cx, "alpha", item)?;
            let mut shape = shape.clone();
            shape.style_mut().multiply_opacity(alpha);
            Ok(shape)
        })
    }
}

/// Sets how every shape is painted over what is below it.
#[derive(Debug)]
pub(crate) struct Blend;

pub(crate) static BLEND: Verb = Verb {
    name: "blend",
    role: Role::Step,
    summary: "paints every shape over what is below it as \"normal\" or \"multiply\"",
    args: &[word("mode", None, BlendMode::NAMES)],
    check: None,
};

impl Node for Blend {
    fn verb(&self) -> &'static Verb {
        &BLEND
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, _: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let mode = choice(args, "mode", BlendMode::named)?;
        Ok(inputs
            .previous
            .iter()
            .cloned()
            .map(|mut shape| {
                shape.style_mut().set_blend(mode);
                shape
            })
            .collect())
    }
}
