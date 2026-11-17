//! `Flow`: a direction without sign at every point.

use super::outline::Outline;
use super::{length, segment_budget};
use crate::Error;
use crate::geometry::{Point, finite};
use crate::key::{Key, Keyed, key_of};
use crate::math::{atan2, sin_cos_deg};
use crate::noise::Noise;
use crate::path::Path;
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

/// What a flow is made of, as a document writes it.
#[derive(Debug)]
pub(crate) enum FlowKind {
    Angle { degrees: f64 },
    Noise { wavelength: f64, phase: f64, seed: f64 },
    Contour { path: Path },
    Mix { flow: Flow, other: Flow, fraction: f64 },
}

#[derive(Debug)]
enum Node {
    /// The doubled-angle unit vector of an angle, fixed.
    Angle {
        degrees: f64,
        cos: f64,
        sin: f64,
    },
    Noise {
        wavelength: f64,
        phase: f64,
        seed: f64,
        noise: Noise,
    },
    Contour {
        path: Path,
        outline: Outline,
    },
    Mix {
        flow: Flow,
        other: Flow,
        fraction: f64,
    },
}

/// A direction without sign at every point: a line through the point, not an arrow. A direction and its opposite are
/// the same, so flows mix them without cancelling. A value, built once and cheap to clone; [`Flow::at`] reads it, and
/// [`Hatch`](crate::Hatch) draws lines along it.
#[derive(Debug, Clone)]
pub struct Flow {
    node: Arc<Node>,
    depth: usize,
    leaves: usize,
}

impl Flow {
    fn leaf(node: Node) -> Self {
        Self {
            node: Arc::new(node),
            depth: 0,
            leaves: 1,
        }
    }

    /// The same direction everywhere: `degrees` clockwise from the right on screen (`y` points down), any finite
    /// number; 30 and 210 are the same.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `degrees` if it is NaN or infinite.
    pub fn angle(degrees: f64) -> Result<Self, Error> {
        Ok(Self::fixed(finite("degrees", degrees)?))
    }

    /// The angle `degrees`, already known to be finite.
    pub(crate) fn fixed(degrees: f64) -> Self {
        let (sin, cos) = sin_cos_deg(2.0 * (degrees % 180.0));
        Self::leaf(Node::Angle { degrees, cos, sin })
    }

    /// Smooth seeded noise: the direction turns through half a turn each way over a few `wavelength`s (pixels) and
    /// the pattern moves with `phase` (cycles; one cycle on is the same flow). Equal seeds make equal flows.
    ///
    /// # Errors
    /// [`Error::Parameter`] unless `wavelength` is above 0 and finite, `phase` finite and `seed` a whole number within
    /// ±(2^53 - 1).
    pub fn noise(wavelength: f64, phase: f64, seed: f64) -> Result<Self, Error> {
        if !(wavelength.is_finite() && wavelength > 0.0) {
            return Err(Error::parameter(
                "wavelength",
                format!("must be above 0, got {wavelength}"),
            ));
        }
        let phase = finite("phase", phase)?;
        let noise = Noise::new(seed, 0)?;
        Ok(Self::leaf(Node::Noise {
            wavelength,
            phase,
            seed,
            noise,
        }))
    }

    /// The direction of the outline of `path` nearest the point.
    ///
    /// # Errors
    /// [`Error::Geometry`] if the path has no edge; [`Error::Parameter`] past `MAX_SEGMENTS` edges.
    pub fn contour(path: &Path) -> Result<Self, Error> {
        let outline = Outline::of(path, "contour", &mut segment_budget("path"))?;
        Ok(Self::leaf(Node::Contour {
            path: path.clone(),
            outline,
        }))
    }

    /// This flow `fraction` of the way to `other`, 0 to 1, blended in doubled-angle space so a direction and its
    /// opposite agree. Where the two cancel exactly, this flow's direction stands.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `fraction` unless it is from 0 to 1; naming `other` if the result would mix more
    /// than 64 flows or nest more than 16 mixes.
    pub fn mix(&self, other: &Self, fraction: f64) -> Result<Self, Error> {
        let fraction = crate::geometry::fraction("fraction", fraction)?;
        let (depth, leaves) = (self.depth.max(other.depth) + 1, self.leaves + other.leaves);
        if depth > MAX_MIX_DEPTH || leaves > MAX_LEAVES {
            return Err(Error::parameter(
                "other",
                format!("a flow mixes at most {MAX_LEAVES} flows and nests at most {MAX_MIX_DEPTH} mixes"),
            ));
        }
        Ok(Self {
            node: Arc::new(Node::Mix {
                flow: self.clone(),
                other: other.clone(),
                fraction,
            }),
            depth,
            leaves,
        })
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
            Node::Angle { cos, sin, .. } => (*cos, *sin),
            Node::Noise {
                wavelength,
                phase,
                noise,
                ..
            } => {
                let value = noise.value(point.x / wavelength, point.y / wavelength, *phase);
                // the noise is from -1 to 1: it turns the direction through half a turn each way
                let (sin, cos) = sin_cos_deg(value * 360.0);
                (cos, sin)
            }
            Node::Contour { outline, .. } => {
                // as a unit vector, so a contour counts as much as an angle
                let (x, y) = outline.nearest(point).doubled;
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

    /// What this flow is made of.
    pub(crate) fn kind(&self) -> FlowKind {
        match &*self.node {
            Node::Angle { degrees, .. } => FlowKind::Angle { degrees: *degrees },
            Node::Noise {
                wavelength,
                phase,
                seed,
                ..
            } => FlowKind::Noise {
                wavelength: *wavelength,
                phase: *phase,
                seed: *seed,
            },
            Node::Contour { path, .. } => FlowKind::Contour { path: path.clone() },
            Node::Mix { flow, other, fraction } => FlowKind::Mix {
                flow: flow.clone(),
                other: other.clone(),
                fraction: *fraction,
            },
        }
    }
}

impl Keyed for Flow {
    fn write_key(&self, key: &mut Key) {
        match &*self.node {
            Node::Angle { degrees, .. } => {
                key.word(1);
                key.number(*degrees);
            }
            Node::Noise {
                wavelength,
                phase,
                seed,
                ..
            } => {
                key.word(2);
                key.number(*wavelength);
                key.number(*phase);
                key.number(*seed);
            }
            Node::Contour { path, .. } => {
                key.word(3);
                key.of(path);
            }
            Node::Mix { flow, other, fraction } => {
                key.word(4);
                key.number(*fraction);
                key.of(flow);
                key.of(other);
            }
        }
    }
}

impl PartialEq for Flow {
    fn eq(&self, other: &Self) -> bool {
        key_of(self) == key_of(other)
    }
}

/// The flow as the call that makes it reads in the Python package: `Flow.angle(45)`.
impl fmt::Display for Flow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.node {
            Node::Angle { degrees, .. } => write!(f, "Flow.angle({degrees})"),
            Node::Noise {
                wavelength,
                phase,
                seed,
                ..
            } => write!(f, "Flow.noise(wavelength={wavelength}, phase={phase}, seed={seed})"),
            Node::Contour { path, .. } => write!(f, "Flow.contour(Path.from_svg_path('{}'))", path.to_svg_path()),
            Node::Mix { flow, other, fraction } => write!(f, "{flow}.mix({other}, {fraction})"),
        }
    }
}
