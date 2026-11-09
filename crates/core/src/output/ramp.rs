//! `Ramp`: a gradient placed in an output, in output pixels. The SVG writer and the pixel writer both draw from it, so
//! that the geometry, the range check and the tint of a shadow are decided once.

use super::outline::within_device;
use super::view::View;
use crate::{Error, Gradient, GradientKind, Point};

/// A gradient in output pixels with its stops as `(offset, [red, green, blue, alpha])`.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Ramp {
    pub(super) kind: GradientKind,
    pub(super) stops: Vec<(f64, [u8; 4])>,
}

impl Ramp {
    /// `gradient` seen through `view`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `device` if a coordinate or the radius lies beyond [`MAX_DEVICE`] output pixels.
    ///
    /// [`MAX_DEVICE`]: super::outline::MAX_DEVICE
    pub(super) fn new(gradient: &Gradient, view: &View) -> Result<Self, Error> {
        let kind = match gradient.kind() {
            GradientKind::Linear { start, end } => GradientKind::Linear {
                start: view.point(start),
                end: view.point(end),
            },
            GradientKind::Radial { center, radius } => GradientKind::Radial {
                center: view.point(center),
                radius: radius * view.scale(),
            },
        };
        let reach = match kind {
            GradientKind::Linear { start, end } => [start.x, start.y, end.x, end.y].map(f64::abs),
            GradientKind::Radial { center, radius } => [center.x, center.y, radius, radius].map(f64::abs),
        };
        within_device("a gradient coordinate", reach.into_iter().fold(0.0, f64::max))?;
        Ok(Self {
            kind,
            stops: gradient.stops().to_vec(),
        })
    }

    /// This ramp with every stop in the colour `rgb` and its own alpha: the silhouette of a gradient's alpha, as a
    /// shadow is tinted.
    pub(super) fn tinted(&self, [red, green, blue]: [u8; 3]) -> Self {
        Self {
            kind: self.kind,
            stops: self
                .stops
                .iter()
                .map(|(offset, [_, _, _, alpha])| (*offset, [red, green, blue, *alpha]))
                .collect(),
        }
    }

    /// This ramp moved `dx` right and `dy` down.
    pub(super) fn shifted(&self, dx: f64, dy: f64) -> Self {
        let by = |point: Point| Point::new(point.x + dx, point.y + dy);
        Self {
            kind: match self.kind {
                GradientKind::Linear { start, end } => GradientKind::Linear {
                    start: by(start),
                    end: by(end),
                },
                GradientKind::Radial { center, radius } => GradientKind::Radial {
                    center: by(center),
                    radius,
                },
            },
            stops: self.stops.clone(),
        }
    }
}
