//! The style steps: `fill`, `stroke`, `opacity`, `blend`, `shadow`, `knockout`, `halftone`. Each changes the paint of every shape and nothing else.

use super::offset::{PROFILE, profile_is_round};
use super::{Args, Context, Inputs, Node, choice, each_shape};
use crate::field::Shapes;
use crate::geometry::MAX_COORDINATE;
use crate::lines::{DEFAULT_AMPLITUDE, DEFAULT_WAVELENGTH};
use crate::path::check_profile;
use crate::shape::{
    Blend as BlendMode, Cap, Color, DEFAULT_MITER_LIMIT, DEFAULT_STROKE_WIDTH, Dot, HalftoneStyle, Join, Mark,
    ShadowStyle, StrokeStyle, Tone,
};
use crate::{Arg, Domain, Error, Family, Kind, Literal, Reads, Role, Shape, Verb};
use std::sync::Arc;

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

/// A colour or a gradient given when the call is made.
const fn paint(domain: Domain) -> Arg {
    Arg {
        kind: Kind::Paint,
        ..color(domain)
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

/// A number read for each shape, within `domain`, that a call may leave out: the step then does what its summary says,
/// and a call that gives it where nothing reads it is refused.
const fn optional_number(name: &'static str, domain: Domain) -> Arg {
    Arg {
        name,
        kind: Kind::Number,
        default: Some(Literal::Unset),
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
    summary: "fills every shape with a colour or a gradient, \"none\" for no fill; once a fill or stroke is set only what is set is painted",
    args: &[paint(Domain::Any)],
    check: None,
};

impl Node for Fill {
    fn verb(&self) -> &'static Verb {
        &FILL
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, _: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let paint = args.paint("color")?;
        Ok(inputs
            .previous
            .iter()
            .cloned()
            .map(|mut shape| {
                shape.style_mut().set_fill(paint.clone());
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
    summary: "strokes every outline with a colour or a gradient, `profile` [[position, factor], ...] varying the width along each sub-path as `outline` does (round joins and caps); once a fill or stroke is set only what is set is painted",
    args: &[
        paint(Domain::Painting),
        number("width", Some(DEFAULT_STROKE_WIDTH), Domain::Positive),
        word("join", Some("round"), Join::NAMES),
        word("cap", Some("round"), Cap::NAMES),
        number("miter_limit", Some(DEFAULT_MITER_LIMIT), Domain::AtLeast(1.0)),
        PROFILE,
    ],
    check: Some(profile_is_round),
};

impl Node for Stroke {
    fn verb(&self) -> &'static Verb {
        &STROKE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let paint = args.paint("color")?;
        let join = choice(args, "join", Join::named)?;
        let cap = choice(args, "cap", Cap::named)?;
        each_shape(inputs.previous, cx, |shape, item| {
            let width = args.number(cx, "width", item)?;
            let miter_limit = args.number(cx, "miter_limit", item)?;
            let profile = args.pairs(cx, "profile", item)?;
            if let Some(stops) = &profile {
                check_profile(stops)?;
            }
            let mut shape = shape.clone();
            shape.style_mut().set_stroke(
                StrokeStyle::new(paint.clone())
                    .finished(width, join, cap, miter_limit)
                    .profiled(profile),
            );
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
    summary: "paints every shape over what is below it with a CSS blend mode: \"normal\", \"multiply\", \"screen\", ..",
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

/// The row distance of a halftone when none is given, in design pixels.
// constant: api.md "Style" (`halftone(color, *, tone, spacing=8, angle=45, ...)`)
const DEFAULT_HALFTONE_SPACING: f64 = 8.0;

/// The turn of a halftone's lattice when none is given, in degrees.
// constant: api.md "Style" (`halftone(color, *, tone, spacing=8, angle=45, ...)`), the classic screen angle
const DEFAULT_HALFTONE_ANGLE: f64 = 45.0;

/// Fills every shape with a screen of dots.
#[derive(Debug)]
pub(crate) struct Halftone;

pub(crate) static HALFTONE: Verb = Verb {
    name: "halftone",
    role: Role::Step,
    summary: "fills every shape with a screen of dots of a colour, on rows `spacing` design pixels apart turned angle \
              degrees: `family` lines, waves (rows swung `amplitude` design pixels over a `wavelength` of design pixels), rings, spiral or \
              rays (about `center`, default the middle of the shape's bounds); each dot covers (1 - tone) of its own \
              cell, tone from 0 (solid) to 1 (no dot), a number or a field read at the dot's center; `shape` is the dot: \
              dot, square, ellipse (`aspect` times as long as wide), diamond, euclidean (round, diamond at 0.5, round hole) or \
              path (a closed outline of the caller's, `{\"path\": data}`); replaces any fill before, as fill replaces it",
    args: &[
        color(Domain::Painting),
        Arg {
            name: "tone",
            kind: Kind::Field,
            default: None,
            reads: Reads::PerItem,
            domain: Domain::Fraction,
        },
        number("spacing", Some(DEFAULT_HALFTONE_SPACING), Domain::Positive),
        optional_number("angle", Domain::Finite),
        Arg {
            name: "shape",
            kind: Kind::Mark,
            default: Some(Literal::Text("dot")),
            reads: Reads::Literal,
            domain: Domain::Choice(Mark::NAMES),
        },
        optional_number("aspect", Domain::AtLeast(1.0)),
        word("family", Some("lines"), Family::NAMES),
        Arg {
            name: "center",
            kind: Kind::Point,
            default: Some(Literal::Unset),
            reads: Reads::PerItem,
            domain: Domain::Coordinate,
        },
        optional_number("amplitude", Domain::Coordinate),
        optional_number("wavelength", Domain::PositiveUpTo(MAX_COORDINATE)),
    ],
    check: Some(halftone_reads_what_it_is_given),
};

/// An argument goes with what reads it: the layout family reads `angle`, `center`, `amplitude` and `wavelength` as
/// `bands` does, and only an ellipse reads `aspect`; one left out takes its default, one given and unread is refused.
fn halftone_reads_what_it_is_given(args: &Args) -> Result<(), Error> {
    let mark = args.mark("shape")?;
    // the outline is checked where it is given, not where the first output draws it
    Dot::new(&mark, 1.0)?;
    let family = Family::parse(args.text("family")?)?;
    for name in ["angle", "center", "amplitude", "wavelength"] {
        if args.given(name)? {
            family.refuse_unread(name)?;
        }
    }
    if args.given("aspect")? {
        mark.refuse_unread("aspect")?;
    }
    Ok(())
}

impl Node for Halftone {
    fn verb(&self) -> &'static Verb {
        &HALFTONE
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let placed = |error: Error| error.at_argument(&cx.place, None);
        let color = args.color("color")?;
        let mark = args.mark("shape")?;
        let family = Family::parse(args.text("family")?)?;
        // a field is bound once, with the values and the drawings of this draw, and shared by every shape
        let field = args
            .field_if_given("tone")?
            .map(|field| field.bind(cx.env, &mut Shapes::taken(inputs.taken)).map(Arc::new))
            .transpose()
            .map_err(placed)?;
        each_shape(inputs.previous, cx, |shape, item| {
            let tone = match &field {
                Some(field) => Tone::Field(Arc::clone(field)),
                None => Tone::Level(args.number(cx, "tone", item)?),
            };
            let halftone = HalftoneStyle {
                color,
                tone,
                spacing: args.number(cx, "spacing", item)?,
                angle: args.number_opt(cx, "angle", item)?.unwrap_or(DEFAULT_HALFTONE_ANGLE),
                mark: mark.clone(),
                aspect: args.number_opt(cx, "aspect", item)?.unwrap_or(1.0),
                family,
                center: args.point(cx, "center", item)?,
                amplitude: args.number_opt(cx, "amplitude", item)?.unwrap_or(DEFAULT_AMPLITUDE),
                wavelength: args.number_opt(cx, "wavelength", item)?.unwrap_or(DEFAULT_WAVELENGTH),
            };
            let mut shape = shape.clone();
            shape.style_mut().set_halftone(halftone);
            Ok(shape)
        })
    }
}
