//! `Paint` and `Gradient`: what fills or strokes a shape.

use super::color::Color;
use crate::geometry::{coordinate_point, finite};
use crate::key::{Key, Keyed};
use crate::{Error, Point};
use std::sync::Arc;

/// What a fill or a stroke is made of: one colour (none paints nothing) or a [`Gradient`].
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    /// One colour all over, [`Color::NONE`] for no paint.
    Color(Color),
    /// A colour that changes with the place.
    Gradient(Gradient),
}

impl Paint {
    /// No paint.
    pub const NONE: Self = Self::Color(Color::NONE);

    /// Whether this paints anything: false only for [`Color::NONE`].
    #[must_use]
    pub fn paints(&self) -> bool {
        !matches!(self, Self::Color(color) if color.rgba().is_none())
    }
}

impl PartialEq<Color> for Paint {
    /// Whether this is a plain paint of `other`.
    fn eq(&self, other: &Color) -> bool {
        matches!(self, Self::Color(color) if color == other)
    }
}

impl From<Color> for Paint {
    fn from(color: Color) -> Self {
        Self::Color(color)
    }
}

impl From<Gradient> for Paint {
    fn from(gradient: Gradient) -> Self {
        Self::Gradient(gradient)
    }
}

impl Keyed for Paint {
    fn write_key(&self, key: &mut Key) {
        match self {
            Self::Color(color) => {
                key.word(0);
                key.of(color);
            }
            Self::Gradient(gradient) => {
                key.word(1);
                key.of(gradient);
            }
        }
    }
}

/// Where a gradient's colours lie, in design pixels of the frame (not of the shape): a transform of the shape moves the
/// shape over the gradient, so set the paint after the transforms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GradientKind {
    /// Along the line from `start` (offset 0) to `end` (offset 1), constant across it.
    Linear {
        /// Where offset 0 lies.
        start: Point,
        /// Where offset 1 lies.
        end: Point,
    },
    /// Around `center`: offset 0 at the centre, offset 1 at `radius`.
    Radial {
        /// Where offset 0 lies.
        center: Point,
        /// The distance of offset 1, above 0.
        radius: f64,
    },
}

/// A colour that changes with the place: stops of an offset from 0 to 1 and a colour, interpolated in sRGB with the
/// alpha not premultiplied (as SVG does), and held at the first and last colour beyond them.
///
/// Built with [`Gradient::linear`] or [`Gradient::radial`], which refuse anything that cannot paint, so a `Gradient`
/// always has at least two stops, offsets that do not decrease, and colours that paint.
#[derive(Debug, Clone, PartialEq)]
pub struct Gradient(Arc<Parts>);

/// What a gradient is made of, behind one pointer so that a style stays small.
#[derive(Debug, PartialEq)]
struct Parts {
    kind: GradientKind,
    stops: Vec<(f64, [u8; 4])>,
}

impl Gradient {
    /// The gradient along the line from `start` to `end`, with `stops` as `(offset, colour)`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `stops` if there are fewer than two, an offset is outside 0 to 1 or below the one
    /// before, or a colour is [`Color::NONE`]; naming `start` or `end` if a coordinate is not within ±1e100 or the two
    /// points are the same.
    pub fn linear(
        start: impl Into<Point>,
        end: impl Into<Point>,
        stops: impl IntoIterator<Item = (f64, Color)>,
    ) -> Result<Self, Error> {
        let (start, end) = (coordinate_point("start", start)?, coordinate_point("end", end)?);
        if start == end {
            return Err(Error::parameter(
                "end",
                "must differ from start: a line of no length has no direction",
            ));
        }
        Self::new(GradientKind::Linear { start, end }, stops)
    }

    /// The gradient around `center` out to `radius`, with `stops` as `(offset, colour)`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `stops` as [`Gradient::linear`] does; naming `center` if a coordinate is not within
    /// ±1e100, `radius` if it is not a finite number above 0.
    pub fn radial(
        center: impl Into<Point>,
        radius: f64,
        stops: impl IntoIterator<Item = (f64, Color)>,
    ) -> Result<Self, Error> {
        let center = coordinate_point("center", center)?;
        if finite("radius", radius)? <= 0.0 {
            return Err(Error::parameter("radius", format!("must be above 0, got {radius}")));
        }
        Self::new(GradientKind::Radial { center, radius }, stops)
    }

    fn new(kind: GradientKind, stops: impl IntoIterator<Item = (f64, Color)>) -> Result<Self, Error> {
        let mut checked: Vec<(f64, [u8; 4])> = Vec::new();
        for (offset, color) in stops {
            if !(0.0..=1.0).contains(&offset) {
                return Err(Error::parameter(
                    "stops",
                    format!("an offset must be a number from 0 to 1, got {offset}"),
                ));
            }
            if checked.last().is_some_and(|(before, _)| offset < *before) {
                return Err(Error::parameter(
                    "stops",
                    format!(
                        "offsets must not decrease, got {offset} after {}",
                        checked[checked.len() - 1].0
                    ),
                ));
            }
            let rgba = color
                .rgba()
                .ok_or_else(|| Error::parameter("stops", "a stop's colour must paint, \"none\" does not"))?;
            checked.push((offset, rgba));
        }
        if checked.len() < 2 {
            return Err(Error::parameter(
                "stops",
                format!("need at least 2 stops, got {}", checked.len()),
            ));
        }
        Ok(Self(Arc::new(Parts { kind, stops: checked })))
    }

    /// Where the colours lie.
    #[must_use]
    pub fn kind(&self) -> GradientKind {
        self.0.kind
    }

    /// The stops, as `(offset, [red, green, blue, alpha])`, in order of offset.
    #[must_use]
    pub fn stops(&self) -> &[(f64, [u8; 4])] {
        &self.0.stops
    }
}

impl Keyed for Gradient {
    fn write_key(&self, key: &mut Key) {
        match self.0.kind {
            GradientKind::Linear { start, end } => {
                key.word(0);
                key.number(start.x);
                key.number(start.y);
                key.number(end.x);
                key.number(end.y);
            }
            GradientKind::Radial { center, radius } => {
                key.word(1);
                key.number(center.x);
                key.number(center.y);
                key.number(radius);
            }
        }
        key.count(self.0.stops.len());
        for (offset, rgba) in &self.0.stops {
            key.number(*offset);
            key.word(u64::from(u32::from_be_bytes(*rgba)));
        }
    }
}
