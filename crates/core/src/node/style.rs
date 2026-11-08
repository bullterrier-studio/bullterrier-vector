//! The style steps: `fill`, `stroke`, `opacity`, `blend`, `shadow`, `knockout`. Each changes the paint of every shape and nothing else.

use super::{Args, Context, Inputs, Node, choice, each_shape};
use crate::shape::{
    Blend as BlendMode, Cap, Color, DEFAULT_MITER_LIMIT, DEFAULT_STROKE_WIDTH, Join, ShadowStyle, StrokeStyle,
};
use crate::{Arg, Domain, Error, Kind, Literal, Reads, Role, Shape, Verb};

/// A colour given when the call is made.
const fn color(domain: Domain) -> Arg {
    Arg {
        name: "color",
        kind: Kind::Color,
        default: None,
        reads: Reads::Literal,
        domain,
    }
}

/// The word `name` of a set, as the argument of a step, given when the call is made.
pub(super) const fn word(name: &'static str, default: Option<&'static str>, options: &'static [&'static str]) -> Arg {
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
pub(super) const fn number(name: &'static str, default: Option<f64>, domain: Domain) -> Arg {
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

/// Fills every shape.
#[derive(Debug)]
pub(crate) struct Fill;

pub(crate) static FILL: Verb = Verb {
    name: "fill",
    role: Role::Step,
    summary: "fills every shape with a colour, \"none\" for no fill; once a fill or stroke is set only what is set is painted",
    args: &[color(Domain::Any)],
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
        color(Domain::Painting),
        number("width", Some(DEFAULT_STROKE_WIDTH), Domain::Positive),
        word("join", Some("round"), Join::NAMES),
        word("cap", Some("round"), Cap::NAMES),
        number("miter_limit", Some(DEFAULT_MITER_LIMIT), Domain::AtLeast(1.0)),
    ],
    check: None,
};

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

/// The shadow every shape casts when `shadow` leaves out its colour.
// constant: api.md "Style" (`shadow(*, ..., color="#00000059")`), a black at 35% (0x59 of 0xff)
const DEFAULT_SHADOW_COLOR: Color = Color::from_bytes([0, 0, 0, 0x59]);

/// Gives every shape a drop shadow.
#[derive(Debug)]
pub(crate) struct Shadow;

pub(crate) static SHADOW: Verb = Verb {
    name: "shadow",
    role: Role::Step,
    summary: "casts a drop shadow behind every shape: its fill and stroke moved by dx and dy, blurred (blur is a CSS radius, \
              the Gaussian's deviation is half of it) and tinted color (default #00000059); replaces any shadow before",
    args: &[
        number("dx", Some(0.0), Domain::Finite),
        number("dy", Some(8.0), Domain::Finite),
        number("blur", Some(16.0), Domain::AtLeast(0.0)),
        Arg {
            default: Some(Literal::Unset),
            ..color(Domain::Painting)
        },
    ],
    check: None,
};

impl Node for Shadow {
    fn verb(&self) -> &'static Verb {
        &SHADOW
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let color = args.color_opt("color")?.unwrap_or(DEFAULT_SHADOW_COLOR);
        each_shape(inputs.previous, cx, |shape, item| {
            let shadow = ShadowStyle {
                dx: args.number(cx, "dx", item)?,
                dy: args.number(cx, "dy", item)?,
                blur: args.number(cx, "blur", item)?,
                color,
            };
            let mut shape = shape.clone();
            shape.style_mut().set_shadow(shadow);
            Ok(shape)
        })
    }
}

/// Makes every shape remove what is below it instead of painting.
#[derive(Debug)]
pub(crate) struct Knockout;

pub(crate) static KNOCKOUT: Verb = Verb {
    name: "knockout",
    role: Role::Step,
    summary: "makes every shape remove what is below it in the drawing (shadows included), by the area its fill and stroke \
              cover, instead of painting",
    args: &[],
    check: None,
};

impl Node for Knockout {
    fn verb(&self) -> &'static Verb {
        &KNOCKOUT
    }

    fn evaluate(&self, _: &Args, inputs: Inputs<'_>, _: &Context<'_>) -> Result<Vec<Shape>, Error> {
        Ok(inputs
            .previous
            .iter()
            .cloned()
            .map(|mut shape| {
                shape.style_mut().set_knockout();
                shape
            })
            .collect())
    }
}
