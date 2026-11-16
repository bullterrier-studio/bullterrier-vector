//! The step `place_along`: the whole list is one stamp, copied to every stop along a guide.

use super::{Args, Context, Inputs, Node, choice};
use crate::geometry::{MAX_COORDINATE, Point};
use crate::path::{Guide, Spacing};
use crate::{Arg, Domain, Error, Kind, Literal, Path, Reads, Role, Shape, Verb};

/// What `turn` takes: carry the stamp round to the way the guide runs, or keep its orientation.
pub(crate) const TURNS: &[&str] = &["along", "none"];

/// The copies of the shapes at the stops along a guide.
#[derive(Debug)]
pub(crate) struct PlaceAlong;

pub(crate) static PLACE_ALONG: Verb = Verb {
    name: "place_along",
    role: Role::Step,
    summary: "makes a copy of all the shapes at each of count stops (both ends of an open guide) or at every spacing design pixels along a guide path, start (a fraction of the guide's length) moving them on; each copy is carried by the middle of its bounds (or the shapes' shared origin) and turned so its x axis runs along the guide, keeping the shapes' layout, and not bent; the shapes get copy, and a lone shape the order of its copy",
    args: &[
        Arg {
            name: "path",
            kind: Kind::Path,
            default: None,
            reads: Reads::Literal,
            domain: Domain::Any,
        },
        Arg {
            name: "count",
            kind: Kind::Number,
            default: Some(Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::AtLeast(1.0),
        },
        Arg {
            name: "spacing",
            kind: Kind::Number,
            default: Some(Literal::Unset),
            reads: Reads::Literal,
            domain: Domain::PositiveUpTo(MAX_COORDINATE),
        },
        Arg {
            name: "start",
            kind: Kind::Number,
            default: Some(Literal::Number(0.0)),
            reads: Reads::Once,
            domain: Domain::Finite,
        },
        Arg {
            name: "turn",
            kind: Kind::Text,
            default: Some(Literal::Text("along")),
            reads: Reads::Literal,
            domain: Domain::Choice(TURNS),
        },
        Arg {
            name: "near",
            kind: Kind::Point,
            default: Some(Literal::Unset),
            reads: Reads::Once,
            domain: Domain::Coordinate,
        },
    ],
    check: Some(count_or_spacing),
};

/// Exactly one of `count` and `spacing` is given.
fn count_or_spacing(args: &Args) -> Result<(), Error> {
    if args.given("count")? == args.given("spacing")? {
        return Err(Error::parameter(
            "count",
            "exactly one of count and spacing must be given",
        ));
    }
    Ok(())
}

/// The point every shape agrees is its origin, else the middle of the bounds of all of them; `None` if nothing is
/// drawn and no origin is shared.
fn anchor(shapes: &[Shape]) -> Point {
    let first = shapes.first().and_then(|shape| shape.attributes().origin());
    if let Some(origin) = first
        && shapes.iter().all(|shape| shape.attributes().origin() == Some(origin))
    {
        return origin;
    }
    let joined = Path::joined(shapes.iter().map(Shape::path));
    match joined.bounds() {
        Ok(bounds) => Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0),
        // nothing drawn: nothing to place, so any anchor will do
        Err(_) => Point::new(0.0, 0.0),
    }
}

impl Node for PlaceAlong {
    fn verb(&self) -> &'static Verb {
        &PLACE_ALONG
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let placed = |error: Error| error.at_argument(&cx.place, None);
        let start = args.once_number(cx, "start").map_err(placed)?;
        let near = args.once_point(cx, "near").map_err(placed)?;
        let turn = choice(args, "turn", |word| TURNS.iter().position(|known| *known == word)).map_err(placed)? == 0;
        let spacing = match (
            args.once_number_opt(cx, "count").map_err(placed)?,
            args.once_number_opt(cx, "spacing").map_err(placed)?,
        ) {
            (Some(count), None) => {
                if count.fract() != 0.0 {
                    return Err(placed(Error::parameter(
                        "count",
                        format!("must be a whole number, got {count}"),
                    )));
                }
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a whole number of at least 1, saturating past usize"
                )]
                Spacing::Count(count as usize)
            }
            (None, Some(every)) => Spacing::Every(every),
            _ => {
                return Err(placed(
                    count_or_spacing(args)
                        .err()
                        .unwrap_or_else(|| Error::parameter("count", "needs count or spacing")),
                ));
            }
        };
        let previous = inputs.previous;
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        let guide = Guide::new("place_along", args.path("path")?, near, &mut charge).map_err(placed)?;
        let stops = guide.stops(spacing).map_err(placed)?;
        // charged before any copy exists: the shapes, and the segments they hold, once for every stop (a stop of an
        // empty stamp counts as one, so a huge count cannot make the loop below spin on nothing)
        cx.work.made_shapes(stops.saturating_mul(previous.len().max(1)), &cx.place)?;
        let weight: usize = previous.iter().map(|shape| shape.path().weight()).sum();
        cx.work.made_segments(stops.saturating_mul(weight), &place)?;
        let anchor = anchor(previous);
        let mut shapes = Vec::with_capacity(stops * previous.len());
        for copy in 0..stops {
            let stop = guide.stop(spacing, stops, copy, start, turn);
            for shape in previous {
                let mut made = shape.with_path(shape.path().placed(&stop, anchor).map_err(placed)?);
                let attributes = made.attributes_mut();
                attributes.origin = attributes
                    .origin
                    .map(|origin| stop.carried(anchor, origin))
                    .transpose()
                    .map_err(placed)?;
                attributes.copy = u32::try_from(copy).ok();
                if previous.len() == 1 {
                    attributes.order = Some(crate::expr::share(copy, stops));
                }
                shapes.push(made);
            }
        }
        Ok(shapes)
    }
}
