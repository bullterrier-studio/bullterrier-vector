//! `Flow`: a direction without sign at every point.

use super::outline::Outline;
use super::segment_budget;
use crate::Error;
use crate::geometry::{Point, finite};
use crate::key::{Key, Keyed};
use crate::math::{atan2, sin_cos_deg};
use crate::noise::Noise;
use crate::path::Path;
use std::sync::Arc;

/// The most parts one flow mixes.
// constant: a flow is read at every step of every line, so its cost grows with its parts; 64 is more than any
// hand-written blend and keeps a loop of `mix` calls from making a flow that is slow to read
pub(crate) const MAX_PARTS: usize = 64;

/// One thing a flow follows.
#[derive(Debug, Clone)]
pub(crate) enum FlowKind {
    Angle { degrees: f64 },
    Noise { wavelength: f64, phase: f64, seed: f64 },
    Contour { path: Path },
}

/// A part of a flow and how much it counts.
#[derive(Debug, Clone)]
pub(crate) struct FlowPart {
    pub(crate) weight: f64,
    pub(crate) kind: FlowKind,
    /// What `kind` reads at a point, built once.
    reader: Reader,
}

#[derive(Debug, Clone)]
enum Reader {
    /// The doubled-angle unit vector, fixed.
    Fixed(f64, f64),
    Noise(Noise),
    Outline(Arc<Outline>),
}

/// A direction without sign at every point: a line through the point, not an arrow. A direction and its opposite are
/// the same, so flows mix them without cancelling. A value, built once and cheap to clone; [`Flow::at`] reads it, and
/// [`Hatch`](crate::Hatch) draws lines along it.
#[derive(Debug, Clone)]
pub struct Flow {
    parts: Vec<FlowPart>,
}

impl Flow {
    fn single(kind: FlowKind, reader: Reader) -> Self {
        Self { parts: vec![FlowPart { weight: 1.0, kind, reader }] }
    }

    /// The same direction everywhere: `degrees` clockwise from the right on screen (`y` points down), any finite
    /// number; 30 and 210 are the same.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `degrees` if it is NaN or infinite.
    pub fn angle(degrees: f64) -> Result<Self, Error> {
        let degrees = finite("degrees", degrees)?;
        let (sin, cos) = sin_cos_deg(2.0 * (degrees % 180.0));
        Ok(Self::single(FlowKind::Angle { degrees }, Reader::Fixed(cos, sin)))
    }

    /// Smooth seeded noise: the direction turns through half a turn each way over a few `wavelength`s (pixels) and
    /// the pattern moves with `phase` (cycles; one cycle on is the same flow). Equal seeds make equal flows.
    ///
    /// # Errors
    /// [`Error::Parameter`] unless `wavelength` is above 0 and finite, `phase` finite and `seed` a whole number within
    /// ±(2^53 - 1).
    pub fn noise(wavelength: f64, phase: f64, seed: f64) -> Result<Self, Error> {
        if !(wavelength.is_finite() && wavelength > 0.0) {
            return Err(Error::parameter("wavelength", format!("must be above 0, got {wavelength}")));
        }
        let phase = finite("phase", phase)?;
        let noise = Noise::new(seed, 0)?;
        Ok(Self::single(FlowKind::Noise { wavelength, phase, seed }, Reader::Noise(noise)))
    }

    /// The direction of the outline of `path` nearest the point.
    ///
    /// # Errors
    /// [`Error::Geometry`] if the path has no edge; [`Error::Parameter`] past `MAX_SEGMENTS` edges.
    pub fn contour(path: &Path) -> Result<Self, Error> {
        let outline = Outline::of(path, "contour", &mut segment_budget("path"))?;
        Ok(Self::single(FlowKind::Contour { path: path.clone() }, Reader::Outline(Arc::new(outline))))
    }

    /// This flow `fraction` of the way to `other`, 0 to 1, blended in doubled-angle space so a direction and its
    /// opposite agree. A mix of mixes is one flat list of parts, weighted. Where the parts cancel exactly, the first
    /// part's direction stands.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `fraction` unless it is from 0 to 1; naming `other` if the result would hold more
    /// than 64 parts.
    pub fn mix(&self, other: &Self, fraction: f64) -> Result<Self, Error> {
        let fraction = crate::geometry::fraction("fraction", fraction)?;
        let scaled = |flow: &Self, by: f64| -> Vec<FlowPart> {
            flow.parts
                .iter()
                .map(|part| FlowPart { weight: part.weight * by, ..part.clone() })
                .filter(|part| part.weight > 0.0)
                .collect()
        };
        let mut parts = scaled(self, 1.0 - fraction);
        parts.extend(scaled(other, fraction));
        if parts.len() > MAX_PARTS {
            return Err(Error::parameter("other", format!("a flow mixes at most {MAX_PARTS} parts")));
        }
        Ok(Self { parts })
    }

    /// The direction at `point`, in degrees from 0 up to 180, clockwise from the right on screen.
    #[must_use]
    pub fn at(&self, point: impl Into<Point>) -> f64 {
        let (x, y) = self.doubled(point.into());
        let degrees = atan2(y, x).to_degrees() / 2.0;
        let wrapped = degrees.rem_euclid(180.0);
        if wrapped >= 180.0 { 0.0 } else { wrapped }
    }

    /// The doubled-angle vector at `point`: the weighted sum of each part's unit vector.
    fn doubled(&self, point: Point) -> (f64, f64) {
        let (mut x, mut y) = (0.0, 0.0);
        let mut first = (1.0, 0.0);
        for (index, part) in self.parts.iter().enumerate() {
            let (px, py) = match &part.reader {
                Reader::Fixed(cos, sin) => (*cos, *sin),
                Reader::Noise(noise) => {
                    let FlowKind::Noise { wavelength, phase, .. } = part.kind else {
                        unreachable!("a noise reader is made for noise")
                    };
                    let value = noise.value(point.x / wavelength, point.y / wavelength, phase);
                    // the noise is from -1 to 1: it turns the direction through half a turn each way
                    let (sin, cos) = sin_cos_deg(value * 360.0);
                    (cos, sin)
                }
                Reader::Outline(outline) => {
                    let (dx, dy) = outline.nearest(point).tangent;
                    (dx * dx - dy * dy, 2.0 * dx * dy)
                }
            };
            if index == 0 {
                first = (px, py);
            }
            x += part.weight * px;
            y += part.weight * py;
        }
        // constant: below a thousandth of a unit the parts have cancelled, and the sum's direction is rounding
        if x.hypot(y) < 1e-3 { first } else { (x, y) }
    }

    /// The unit direction at `point` (either sign), from the half-angle formulas: no trigonometry in the loop that
    /// draws lines.
    pub(crate) fn direction(&self, point: Point) -> (f64, f64) {
        let (x, y) = self.doubled(point);
        let length = x.hypot(y);
        let (cos2, sin2) = (x / length, y / length);
        let cos = ((1.0 + cos2) / 2.0).sqrt();
        let sin = ((1.0 - cos2) / 2.0).sqrt();
        (cos, if sin2 < 0.0 { -sin } else { sin })
    }

    pub(crate) fn parts(&self) -> &[FlowPart] {
        &self.parts
    }
}

impl Keyed for Flow {
    fn write_key(&self, key: &mut Key) {
        key.count(self.parts.len());
        for part in &self.parts {
            key.number(part.weight);
            match &part.kind {
                FlowKind::Angle { degrees } => {
                    key.word(1);
                    key.number(*degrees);
                }
                FlowKind::Noise { wavelength, phase, seed } => {
                    key.word(2);
                    key.number(*wavelength);
                    key.number(*phase);
                    key.number(*seed);
                }
                FlowKind::Contour { path } => {
                    key.word(3);
                    key.of(path);
                }
            }
        }
    }
}
