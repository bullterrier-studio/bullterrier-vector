//! `Flow`: a direction without sign at every point.

use super::octaves::NOISE_READ;
use super::outline::Outline;
use super::{Shapes, length, once, read, segment_budget};
use crate::geometry::Point;
use crate::key::{Key, Keyed, key_of};
use crate::math::{atan2, sin_cos_deg};
use crate::noise::Noise;
use crate::{Drawing, Error, Expr, Param, Values};
use std::fmt;
use std::sync::Arc;

/// The most flows one flow mixes (its leaves).
// constant: a flow is read at every step of every line, so its cost grows with its leaves; 64 is more than any
// hand-written blend and keeps a loop of `mix` calls from making a flow that is slow to read
pub(crate) const MAX_LEAVES: usize = 64;

/// The deepest nesting of mixes.
// constant: a document holds each nested mix two levels deep and has 40 in all (`MAX_DOCUMENT_DEPTH`), and reading
// one recurses this deep: 16 mixes fit in a document beside the call that holds them
pub(crate) const MAX_MIX_DEPTH: usize = 16;

/// What a flow is made of, as a document writes it: the numbers as written (they may read Params) and the drawings
/// it follows.
#[derive(Debug, Clone)]
pub(crate) enum FlowKind {
    Angle { degrees: Expr },
    Noise { wavelength: Expr, phase: Expr, seed: f64 },
    Contour { drawing: Drawing },
    Mix { flow: Flow, other: Flow, fraction: Expr },
}

/// What reads a flow at a point: the made flow, with its numbers worked out and its drawings drawn.
#[derive(Debug)]
enum Node {
    /// The doubled-angle unit vector of an angle, fixed.
    Angle {
        cos: f64,
        sin: f64,
    },
    Noise {
        wavelength: f64,
        phase: f64,
        noise: Noise,
    },
    Contour {
        outline: Outline,
    },
    Mix {
        flow: Flow,
        other: Flow,
        fraction: f64,
    },
}

/// A direction without sign at every point: a line through the point, not an arrow. A direction and its opposite are
/// the same, so flows mix them without cancelling. A value, built once and cheap to clone; [`Flow::at`] reads it
/// with any Param at its default, and [`Hatch`](crate::Hatch) draws lines along it (a `hatch` step reads it again with
/// the values it draws with).
#[derive(Debug, Clone)]
pub struct Flow {
    kind: Arc<FlowKind>,
    node: Arc<Node>,
    depth: usize,
    leaves: usize,
}

impl Flow {
    /// The flow `kind` says with the Params at their defaults.
    fn made(kind: FlowKind) -> Result<Self, Error> {
        Self::build(kind, &Values::new(), &mut Shapes::Defaults)
    }

    /// The flow `kind` says with the Params at `env` and the drawings from `shapes`.
    fn build(kind: FlowKind, env: &Values, shapes: &mut Shapes<'_>) -> Result<Self, Error> {
        let (node, depth, leaves) = match &kind {
            FlowKind::Angle { degrees } => (Node::angle(read("degrees", degrees, env)?), 0, 1),
            FlowKind::Noise {
                wavelength,
                phase,
                seed,
            } => {
                let wavelength = read("wavelength", wavelength, env)?;
                if wavelength <= 0.0 {
                    return Err(Error::parameter(
                        "wavelength",
                        format!("must be above 0, got {wavelength}"),
                    ));
                }
                let phase = read("phase", phase, env)?;
                let noise = Noise::new(*seed, 0)?;
                (
                    Node::Noise {
                        wavelength,
                        phase,
                        noise,
                    },
                    0,
                    1,
                )
            }
            FlowKind::Contour { drawing } => {
                let path = shapes.path_of(drawing)?;
                let outline = Outline::of(&path, "contour", &mut segment_budget("drawing"))?;
                (Node::Contour { outline }, 0, 1)
            }
            FlowKind::Mix { flow, other, fraction } => {
                let fraction = crate::geometry::fraction("fraction", read("fraction", fraction, env)?)?;
                let (flow, other) = (flow.bind(env, shapes)?, other.bind(env, shapes)?);
                let (depth, leaves) = (flow.depth.max(other.depth) + 1, flow.leaves + other.leaves);
                (Node::Mix { flow, other, fraction }, depth, leaves)
            }
        };
        Ok(Self {
            kind: Arc::new(kind),
            node: Arc::new(node),
            depth,
            leaves,
        })
    }

    /// This flow again with the Params at `env` and the drawings it follows from `shapes`.
    pub(crate) fn bind(&self, env: &Values, shapes: &mut Shapes<'_>) -> Result<Self, Error> {
        Self::build((*self.kind).clone(), env, shapes)
    }

    /// The same direction everywhere: `degrees` clockwise from the right on screen, any finite number, or an
    /// expression of Params read once with the values of the draw; 30 and 210 are the same.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `degrees` if it is NaN or infinite, or a per-item number.
    pub fn angle(degrees: impl Into<Expr>) -> Result<Self, Error> {
        Self::made(FlowKind::Angle {
            degrees: once("degrees", &degrees.into())?,
        })
    }

    /// The angle `degrees`, already known to be finite.
    pub(crate) fn fixed(degrees: f64) -> Self {
        Self {
            kind: Arc::new(FlowKind::Angle {
                degrees: Expr::from(degrees),
            }),
            node: Arc::new(Node::angle(degrees)),
            depth: 0,
            leaves: 1,
        }
    }

    /// Smooth seeded noise: the direction turns through half a turn each way over a few `wavelength`s (pixels) and
    /// the pattern moves with `phase` (cycles; one cycle on is the same flow). Equal seeds make equal flows.
    /// `wavelength` and `phase` may be expressions of Params, read once with the values of the draw.
    ///
    /// # Errors
    /// [`Error::Parameter`] unless `wavelength` is above 0 and finite, `phase` finite (and neither a per-item number)
    /// and `seed` a whole number within ±(2^53 - 1).
    pub fn noise(wavelength: impl Into<Expr>, phase: impl Into<Expr>, seed: f64) -> Result<Self, Error> {
        Self::made(FlowKind::Noise {
            wavelength: once("wavelength", &wavelength.into())?,
            phase: once("phase", &phase.into())?,
            seed,
        })
    }

    /// The direction of the outline of `drawing` nearest the point: the outlines of all its shapes together, drawn
    /// with the Params at their defaults for [`Flow::at`] and with the values of the draw in a `hatch` step. A `Path`
    /// goes where a drawing is asked for. Where two outlines are about as near, as at the middle of a stroke, their
    /// directions blend, so the direction does not jump there.
    ///
    /// # Errors
    /// The drawing's own error if it cannot be drawn; [`Error::Geometry`] if it has no edge; [`Error::Parameter`] past
    /// `MAX_SEGMENTS` edges.
    pub fn contour(drawing: impl Into<Drawing>) -> Result<Self, Error> {
        Self::made(FlowKind::Contour {
            drawing: drawing.into(),
        })
    }

    /// This flow `fraction` of the way to `other`, 0 to 1 (a number or an expression of Params), blended in
    /// doubled-angle space so a direction and its opposite agree. Where the two cancel exactly, this flow's direction
    /// stands.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `fraction` unless it is from 0 to 1; naming `other` if the result would mix more
    /// than 64 flows or nest more than 16 mixes.
    pub fn mix(&self, other: &Self, fraction: impl Into<Expr>) -> Result<Self, Error> {
        let fraction = once("fraction", &fraction.into())?;
        let (depth, leaves) = (self.depth.max(other.depth) + 1, self.leaves + other.leaves);
        if depth > MAX_MIX_DEPTH || leaves > MAX_LEAVES {
            return Err(Error::parameter(
                "other",
                format!("a flow mixes at most {MAX_LEAVES} flows and nest at most {MAX_MIX_DEPTH} mixes"),
            ));
        }
        Self::made(FlowKind::Mix {
            flow: self.clone(),
            other: other.clone(),
            fraction,
        })
    }

    /// The drawings this flow follows, in order.
    pub(crate) fn drawings(&self) -> Vec<&Drawing> {
        match &*self.kind {
            FlowKind::Contour { drawing } => vec![drawing],
            FlowKind::Mix { flow, other, .. } => {
                let mut found = flow.drawings();
                found.extend(other.drawings());
                found
            }
            _ => Vec::new(),
        }
    }

    /// Adds the Params this flow reads to `into`.
    pub(crate) fn collect_params(&self, into: &mut Vec<Param>) {
        match &*self.kind {
            FlowKind::Angle { degrees } => degrees.collect_params(into),
            FlowKind::Noise { wavelength, phase, .. } => {
                wavelength.collect_params(into);
                phase.collect_params(into);
            }
            FlowKind::Contour { .. } => {}
            FlowKind::Mix { flow, other, fraction } => {
                fraction.collect_params(into);
                flow.collect_params(into);
                other.collect_params(into);
            }
        }
    }

    /// The direction at `point`, in degrees from 0 up to 180, clockwise from the right on screen.
    #[must_use]
    pub fn at(&self, point: impl Into<Point>) -> f64 {
        let (x, y) = self.doubled(point.into());
        let degrees = atan2(y, x).to_degrees() / 2.0;
        let wrapped = degrees.rem_euclid(180.0);
        if wrapped >= 180.0 { 0.0 } else { wrapped }
    }

    /// The doubled-angle vector at `point`: a leaf's unit vector, a mix's weighted sum. Where the sum has cancelled the
    /// first leaf's stands.
    fn doubled(&self, point: Point) -> (f64, f64) {
        let (x, y) = self.sum(point);
        // constant: below a thousandth of a unit the parts have cancelled, and the sum's direction is rounding
        if length(x, y) < 1e-3 { self.first(point) } else { (x, y) }
    }

    fn sum(&self, point: Point) -> (f64, f64) {
        match &*self.node {
            Node::Angle { cos, sin } => (*cos, *sin),
            Node::Noise {
                wavelength,
                phase,
                noise,
            } => {
                let value = noise.value(point.x / wavelength, point.y / wavelength, *phase);
                // the noise is from -1 to 1: it turns the direction through half a turn each way
                let (sin, cos) = sin_cos_deg(value * 360.0);
                (cos, sin)
            }
            Node::Contour { outline } => {
                // as a unit vector, so a contour counts as much as an angle
                let (x, y) = outline.direction(point);
                let size = length(x, y);
                if size < 1e-12 { (1.0, 0.0) } else { (x / size, y / size) }
            }
            Node::Mix { flow, other, fraction } => {
                let (a, b) = (flow.sum(point), other.sum(point));
                (
                    a.0 * (1.0 - fraction) + b.0 * fraction,
                    a.1 * (1.0 - fraction) + b.1 * fraction,
                )
            }
        }
    }

    /// The unit vector of the first leaf at `point`.
    fn first(&self, point: Point) -> (f64, f64) {
        match &*self.node {
            Node::Mix { flow, .. } => flow.first(point),
            _ => self.sum(point),
        }
    }

    /// The unit direction at `point` (either sign), from the half-angle formulas: no trigonometry in the loop that
    /// draws lines.
    pub(crate) fn direction(&self, point: Point) -> (f64, f64) {
        let (x, y) = self.doubled(point);
        let size = length(x, y);
        let (cos2, sin2) = (x / size, y / size);
        let cos = 1.0_f64.midpoint(cos2).sqrt();
        let sin = ((1.0 - cos2) / 2.0).sqrt();
        (cos, if sin2 < 0.0 { -sin } else { sin })
    }

    /// The cost of one read in units of `MAX_READ_WORK`, as a field's: an angle one unit, noise one octave
    /// ([`NOISE_READ`]), a contour its outline's nearest-edge search, a mix one unit and both its flows'.
    pub(super) fn cost(&self) -> f64 {
        match &*self.node {
            Node::Angle { .. } => 1.0,
            Node::Noise { .. } => NOISE_READ,
            Node::Contour { outline } => outline.read_cost(),
            Node::Mix { flow, other, .. } => 1.0 + flow.cost() + other.cost(),
        }
    }

    /// What this flow is made of.
    pub(crate) fn kind(&self) -> &FlowKind {
        &self.kind
    }
}

impl Node {
    /// The node of an angle: its doubled-angle unit vector.
    fn angle(degrees: f64) -> Self {
        let (sin, cos) = sin_cos_deg(2.0 * (degrees % 180.0));
        Self::Angle { cos, sin }
    }
}

impl Keyed for Flow {
    /// A drawing is told apart by the call that takes it (the cache and the graph), so the key holds none.
    fn write_key(&self, key: &mut Key) {
        match &*self.kind {
            FlowKind::Angle { degrees } => {
                key.word(1);
                key.of(degrees);
            }
            FlowKind::Noise {
                wavelength,
                phase,
                seed,
            } => {
                key.word(2);
                key.of(wavelength);
                key.of(phase);
                key.number(*seed);
            }
            FlowKind::Contour { .. } => key.word(3),
            FlowKind::Mix { flow, other, fraction } => {
                key.word(4);
                key.of(fraction);
                key.of(flow);
                key.of(other);
            }
        }
    }
}

impl PartialEq for Flow {
    /// The same call: equal numbers, and drawings that are the same (the same object, or each one path of the same data).
    fn eq(&self, other: &Self) -> bool {
        key_of(self) == key_of(other)
            && self
                .drawings()
                .iter()
                .zip(other.drawings())
                .all(|(a, b)| a.is_same_as(b))
    }
}

/// The flow as the call that makes it reads in the Python package: `Flow.angle(45)`.
impl fmt::Display for Flow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.kind {
            FlowKind::Angle { degrees } => write!(f, "Flow.angle({degrees})"),
            FlowKind::Noise {
                wavelength,
                phase,
                seed,
            } => write!(f, "Flow.noise(wavelength={wavelength}, phase={phase}, seed={seed})"),
            FlowKind::Contour { drawing } => match drawing.lone_path() {
                Some(path) => write!(f, "Flow.contour(Path.from_svg_path('{}'))", path.to_svg_path()),
                None => write!(f, "Flow.contour({drawing:?})"),
            },
            FlowKind::Mix { flow, other, fraction } => write!(f, "{flow}.mix({other}, {fraction})"),
        }
    }
}
