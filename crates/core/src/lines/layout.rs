//! Where the rows of a layout lie: straight lines, waves, rings, a spiral or rays. The one owner of the maths that turns
//! a row and a place along it into a point of the design frame, and of the range of rows that cover a box. `bands` lays
//! lines along these rows and the halftone screens lay a dot in each cell of them, so a screen and the bands of the
//! same family agree by construction.
//!
//! The straight family sits in a frame turned by `angle`: `along` runs with the rows, `across` runs over them. The round
//! ones sit about a center, a row a ring or an arm or a ray. A turn is clockwise on the screen (`y` down).

use crate::geometry::Point;
use crate::math::{sin_cos, sin_cos_deg};
use crate::{Error, Rect, field::length};
use std::f64::consts::{PI, TAU};

/// The narrowest a ray cell may be at its middle, in spacings: the rays merge in pairs (a power of two at a time) until
/// none is narrower.
// constant: below half a spacing the cells of the rays are about as wide as a dot is thick at tone 0.5, which draws a
// blob; bullterrier-image keeps the same bound (`NARROWEST_RAY`)
const NARROWEST_RAY: f64 = 0.5;

/// Where the rows of `bands` and the cells of a `halftone` lie: the one word list both take as `family`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Straight rows along the angle, `spacing` apart.
    Lines,
    /// Straight rows swung across by a sine, `amplitude` design pixels high, one `wavelength` design pixels long.
    Waves,
    /// Rings round the center, `spacing` apart, the first half a spacing out.
    Rings,
    /// One Archimedean spiral out of the center, a `spacing` further out each turn.
    Spiral,
    /// Rays straight out of the center.
    Rays,
}

impl Family {
    /// The words a document uses, in the order of the variants.
    pub(crate) const NAMES: &'static [&'static str] = &["lines", "waves", "rings", "spiral", "rays"];

    /// The family a word names: `"lines"`, `"waves"`, `"rings"`, `"spiral"` or `"rays"`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `family` and listing the words.
    pub fn parse(name: &str) -> Result<Self, Error> {
        [Self::Lines, Self::Waves, Self::Rings, Self::Spiral, Self::Rays]
            .into_iter()
            .zip(Self::NAMES)
            .find(|(_, word)| **word == name)
            .map(|(family, _)| family)
            .ok_or_else(|| crate::shape::refused("family", Self::NAMES, name))
    }

    /// The word a document uses.
    #[must_use]
    pub fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    /// Whether the rows run about a center.
    pub(crate) fn is_round(self) -> bool {
        matches!(self, Self::Rings | Self::Spiral | Self::Rays)
    }

    /// Whether this family reads the optional argument `argument` (`angle`, `center`, `wavelength`, `amplitude`).
    pub(crate) fn reads(self, argument: &str) -> bool {
        match argument {
            "angle" => self != Self::Rings,
            "center" => self.is_round(),
            "wavelength" | "amplitude" => self == Self::Waves,
            _ => true,
        }
    }

    /// The refusal of the optional argument `argument` (`angle`, `center`, `wavelength`, `amplitude`) if this family
    /// does not read it: the one place a layout's unread arguments are refused, for `bands` and `halftone` alike.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `argument` if the family does not read it.
    pub fn refuse_unread(self, argument: &str) -> Result<(), Error> {
        if self.reads(argument) {
            Ok(())
        } else {
            Err(Error::parameter(
                argument,
                format!("is not read by the family \"{}\"", self.name()),
            ))
        }
    }
}

/// A family with its turn and, for waves, their size in rows: multiples of the spacing, as the cells of a screen are.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Layout {
    pub(crate) family: Family,
    /// The distance between rows.
    pub(crate) spacing: f64,
    /// The turn in degrees.
    pub(crate) angle: f64,
    /// The `(wavelength, amplitude)` of waves, in rows.
    pub(crate) wave: Option<(f64, f64)>,
}

/// The middle of `bounds`: where rings, a spiral and rays are about by default.
pub(crate) fn middle(bounds: Rect) -> Point {
    Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0)
}

/// The four corners of `bounds`.
pub(crate) fn corners(bounds: Rect) -> [Point; 4] {
    [
        Point::new(bounds.x, bounds.y),
        Point::new(bounds.x + bounds.width, bounds.y),
        Point::new(bounds.x, bounds.y + bounds.height),
        Point::new(bounds.x + bounds.width, bounds.y + bounds.height),
    ]
}

/// The point at `radius` and `angle` (radians, clockwise on the screen from the x axis) about `center`.
pub(crate) fn polar(center: Point, radius: f64, angle: f64) -> Point {
    let (sin, cos) = sin_cos(angle);
    Point::new(center.x + radius * cos, center.y + radius * sin)
}

/// How far a box lies from a center: the nearest and the farthest point of it, in design pixels.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Distances {
    pub(crate) near: f64,
    pub(crate) far: f64,
}

impl Distances {
    /// The distances from `center` to the box `bounds` with these `corners`.
    pub(crate) fn of(center: Point, bounds: Rect, corners: &[Point; 4]) -> Self {
        let near = Point::new(
            center.x.clamp(bounds.x, bounds.x + bounds.width),
            center.y.clamp(bounds.y, bounds.y + bounds.height),
        );
        Self {
            near: length(near.x - center.x, near.y - center.y),
            far: corners
                .iter()
                .map(|p| length(p.x - center.x, p.y - center.y))
                .fold(0.0, f64::max),
        }
    }
}

/// The rows of a round layout that can reach a box, and a bound of the work they are.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RoundRows {
    /// The index of the first row.
    pub(crate) first: f64,
    /// How many rows there are, 0 if none reaches the box.
    pub(crate) rows: f64,
    /// An upper bound of the cells of the outermost row: for rays, how many rays there are.
    pub(crate) outermost: f64,
    /// An upper bound of the cells of every row together: the rows times the cells of the outermost, with no difference
    /// of squares to cancel.
    pub(crate) work: f64,
}

impl RoundRows {
    /// The rows' indices, first to last: loop over these, never add 1 to a float (past 2^53 that does not advance).
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "the rows are counted in floats and the work charge, which bounds them, comes before any loop over them"
    )]
    pub(crate) fn indices(&self) -> impl Iterator<Item = f64> {
        let first = self.first;
        (0..self.rows as usize).map(move |at| first + at as f64)
    }
}

impl Layout {
    /// The rows of this round layout about `center` that can reach the box `bounds` with something that reaches `reach`
    /// design pixels (0 for a line), and a bound of their work; `rays` is how many rays a layout of rays has at its
    /// widest (see [`Layout::ray_count`], [`ring_cells`]). `step` is the smallest distance a point of a row must
    /// move at the coordinates in play: where it would not, no row can be told from the next.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `center` if it lies so far from the box that a step would not move there.
    pub(crate) fn round_rows(
        &self,
        center: Point,
        bounds: Rect,
        reach: f64,
        step: f64,
        rays: f64,
    ) -> Result<RoundRows, Error> {
        let box_corners = corners(bounds);
        let distances = Distances::of(center, bounds, &box_corners);
        let furthest = center.x.abs().max(center.y.abs()) + distances.far;
        if furthest + step <= furthest {
            return Err(Error::parameter(
                "center",
                format!("is too far from the region: a step of the spacing would not move at {furthest}"),
            ));
        }
        let widened = Distances {
            near: (distances.near - reach).max(0.0),
            far: distances.far + reach,
        };
        let (first, last) = if self.family == Family::Spiral {
            (
                (widened.near / self.spacing - 1.5).floor().max(0.0),
                (widened.far / self.spacing - 0.5).floor(),
            )
        } else {
            self.rings_within(widened)
        };
        let rows = (last - first + 1.0).max(0.0);
        let outermost = match self.family {
            Family::Rays => rays,
            // a ring or arm of radius (k + 1/2) has at most 2 pi (k + 1/2) + 2 cells
            _ => TAU * (last + 1.0) + 2.0,
        };
        Ok(RoundRows {
            first,
            rows,
            outermost,
            work: if rows > 0.0 { rows * outermost } else { 0.0 },
        })
    }

    /// The turn in radians.
    pub(crate) fn turn(&self) -> f64 {
        self.angle.to_radians()
    }

    /// The place of `point` in the frame of the straight rows: `(along, across)`.
    pub(crate) fn frame(&self, point: Point) -> (f64, f64) {
        let (sin, cos) = sin_cos_deg(self.angle);
        (point.x * cos + point.y * sin, -point.x * sin + point.y * cos)
    }

    /// The point `along` the rows of the straight family and `across` them, a wave's lift added across.
    pub(crate) fn straight(&self, along: f64, across: f64) -> Point {
        let (sin, cos) = sin_cos_deg(self.angle);
        let lift = match self.wave {
            Some((length, height)) if self.family == Family::Waves => {
                height * self.spacing * sin_cos(TAU * along / (length * self.spacing)).0
            }
            _ => 0.0,
        };
        let n = across + lift;
        Point::new(along * cos - n * sin, along * sin + n * cos)
    }

    /// The radius of ring `k`, counting from 0 at the center: the first is half a spacing out.
    pub(crate) fn ring_radius(&self, k: f64) -> f64 {
        (k + 0.5) * self.spacing
    }

    /// The rings `(first, last)` whose radius lies within `distances` of the center; none if `last < first`.
    pub(crate) fn rings_within(&self, distances: Distances) -> (f64, f64) {
        let (low, high) = (distances.near / self.spacing, distances.far / self.spacing);
        ((low - 0.5).ceil().max(0.0), (high - 0.5).floor())
    }

    /// The radius of the spiral at `theta` radians of turn from where it leaves the center: a spacing more each turn.
    pub(crate) fn spiral_radius(&self, theta: f64) -> f64 {
        self.spacing / TAU * theta
    }

    /// How many rays are a spacing apart at `high` spacings from the center, at least 1.
    pub(crate) fn ray_count(high: f64) -> f64 {
        (TAU * high).ceil().max(1.0)
    }
}

/// The cells of the ring or spiral arm that lies `middle` spacings out on average: about one spacing long each, and an
/// even number so that a half turn maps a cell to a cell. The same rule gives the rays of a screen.
pub(crate) fn ring_cells(middle: f64) -> f64 {
    2.0 * (PI * middle).round().max(1.0)
}

/// The cells of a layout of `rays` rays at `middle` spacings out (a cell's middle): the rays merge in pairs, a power of
/// two at a time, until none is narrower than [`NARROWEST_RAY`], so there are `rays / step` of them: not whole when
/// `rays` is not a multiple of `step`, and then the last is the narrower and fills the rest of the turn.
pub(crate) fn ray_wedges(rays: f64, middle: f64) -> f64 {
    let mut step = 1.0;
    while step < rays && TAU * middle * step / rays < NARROWEST_RAY {
        step *= 2.0;
    }
    rays / step
}
