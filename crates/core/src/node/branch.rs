//! The step `branch`: forking threads grown from the outline of every shape, one shape per root-to-tip path.

use super::{Args, Context, Inputs, Item, Node};
use crate::expr::share;
use crate::geometry::MAX_COORDINATE;
use crate::path::BranchPlan;
use crate::{Arg, Domain, Error, Kind, Literal, Path, Reads, Role, Shape, Verb};

/// The largest seed and count a number holds exactly.
// constant: 2^53 - 1, the largest whole number a double holds exactly
const MAX_WHOLE: f64 = 9_007_199_254_740_991.0;

const fn arg(name: &'static str, reads: Reads, default: Option<Literal>, domain: Domain) -> Arg {
    Arg {
        name,
        kind: Kind::Number,
        default,
        reads,
        domain,
    }
}

/// Threads grown from the outlines.
#[derive(Debug)]
pub(crate) struct Branch;

pub(crate) static BRANCH: Verb = Verb {
    name: "branch",
    role: Role::Step,
    summary: "replaces every shape by forking threads grown out of the outline of the region it fills: a root every spacing design pixels along each outline (holes too), at a seeded place in its stretch, each growing out along the outward normal with its heading drifting by wander degrees of smooth seeded noise over wavelength pixels; along the distance from the root, a thread forks once in every stretch of fork_spacing, turning angle degrees to a seeded side, until a path has taken forks forks; every path of a root ends length times its root's factor from it, the factor a seeded number from 1 - shorter to 1; one open shape per root-to-tip path, with root, forks, order and the root as origin, so slice(0, growth) grows a tree from its root",
    args: &[
        arg("spacing", Reads::Literal, None, Domain::PositiveUpTo(MAX_COORDINATE)),
        arg("length", Reads::Literal, None, Domain::PositiveUpTo(MAX_COORDINATE)),
        arg(
            "fork_spacing",
            Reads::Literal,
            Some(Literal::Number(80.0)),
            Domain::PositiveUpTo(MAX_COORDINATE),
        ),
        arg(
            "forks",
            Reads::Literal,
            Some(Literal::Number(2.0)),
            Domain::WholeBetween(0.0, MAX_WHOLE),
        ),
        arg("shorter", Reads::PerItem, Some(Literal::Number(0.0)), Domain::Fraction),
        arg("angle", Reads::PerItem, Some(Literal::Number(45.0)), Domain::Finite),
        arg("wander", Reads::PerItem, Some(Literal::Number(30.0)), Domain::Finite),
        arg(
            "wavelength",
            Reads::PerItem,
            Some(Literal::Number(80.0)),
            Domain::PositiveUpTo(MAX_COORDINATE),
        ),
        arg("seed", Reads::PerItem, Some(Literal::Number(0.0)), Domain::Finite),
    ],
    check: None,
};

impl Node for Branch {
    fn verb(&self) -> &'static Verb {
        &BRANCH
    }

    fn evaluate(&self, args: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        let count = inputs.previous.len();
        let mut made = Vec::new();
        for (index, shape) in inputs.previous.iter().enumerate() {
            let placed = |error: Error| error.at_argument(&cx.place, Some(index));
            let item = Item {
                index,
                count,
                order: shape.attributes().order().unwrap_or(0.0),
            };
            let number = |name: &str| args.number(cx, name, item).map_err(placed);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a whole number from 0 within 2^53, checked by its domain"
            )]
            let plan = BranchPlan {
                spacing: number("spacing")?,
                length: number("length")?,
                fork_spacing: number("fork_spacing")?,
                forks: number("forks")? as u64,
                shorter: number("shorter")?,
                angle: number("angle")?,
                wander: number("wander")?,
                wavelength: number("wavelength")?,
                seed: number("seed")?,
            };
            let stream = u64::try_from(index).unwrap_or(u64::MAX);
            let threads = shape
                .path()
                .branch_charged(&plan, stream, &mut charge)
                .map_err(placed)?;
            cx.work.made_shapes(threads.len(), &cx.place)?;
            let total = threads.len();
            for (k, thread) in threads.into_iter().enumerate() {
                let mut grown = shape.with_path(Path::polyline(thread.points.iter().copied(), false).map_err(placed)?);
                let attributes = grown.attributes_mut();
                attributes.root = Some(thread.root);
                attributes.forks = Some(thread.forks);
                attributes.origin = thread.points.first().copied();
                attributes.order = Some(share(k, total));
                made.push(grown);
            }
        }
        Ok(made)
    }
}
