//! `Path::stipple`: dots whose density follows a tone. One dot at most in each cell of a square lattice; a cell gets a
//! dot when a low-discrepancy threshold, spread evenly over the lattice, is under the density the tone asks for there, so
//! the dots of a tone are evenly spread and their count follows the tone to within a cell or two.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the cell counts are checked against MAX_CELLS before they are used"
)]

use crate::expr::{splitmix, whole_seed};
use crate::field::{Field, once, read, segment_budget};
use crate::geometry::{MAX_COORDINATE, Point};
use crate::path::Path;
use crate::{Error, Expr, Values};

/// The most cells one call may lay over a region.
// constant: 2 million cells make at most 2 million dots of 4 curves: a few hundred MB, and a print at 300 dpi of a
// letter page at a 1 px spacing is 8 million, so a finer lattice is a smaller piece
const MAX_CELLS: f64 = 2_000_000.0;

/// How far a dot may stand from the middle of its cell, as a share of the spacing.
// constant: 0.8 of the spacing in each direction breaks the grid so that no rows show, and keeps a dot in or next to its cell
const JITTER: f64 = 0.8;

/// The steps of the R2 sequence (Roberts 2018), the plastic number's reciprocals, which spread a threshold evenly over a
/// lattice in both directions.
// constant: 1 / phi_2 and 1 / phi_2^2 for the plastic number phi_2 = 1.32471795724474602596
const R2: (f64, f64) = (0.754_877_666_246_692_7, 0.569_840_290_998_053_2);

/// What to draw: the lattice spacing, the tone that sets the density and the radius, the radius and the density at
/// tone 0 and at tone 1, and a seed.
#[derive(Debug, Clone)]
pub struct Stipple {
    spacing: Expr,
    tone: Field,
    radius: (Expr, Expr),
    density: (Expr, Expr),
    seed: Expr,
}

impl Stipple {
    /// Round dots on a square lattice `spacing` apart, one at most in each cell: a dot's radius is `radius.0` design
    /// pixels where the `tone` is 0 and `radius.1` where it is 1, linear between and held beyond (read at the dot's
    /// center, as the density is; `(r, r)` for one size). By default the darkest tone (0) fills every cell and the
    /// lightest (1) none, linear between: see [`Stipple::density`]. Numbers are numbers or expressions of Params, read
    /// once.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `spacing` or `radius` if it is a per-item number.
    pub fn new(
        spacing: impl Into<Expr>,
        tone: Field,
        radius: (impl Into<Expr>, impl Into<Expr>),
    ) -> Result<Self, Error> {
        Ok(Self {
            spacing: once("spacing", &spacing.into())?,
            tone,
            radius: (once("radius", &radius.0.into())?, once("radius", &radius.1.into())?),
            density: (Expr::from(1.0), Expr::from(0.0)),
            seed: Expr::from(0.0),
        })
    }

    /// The share of cells that get a dot (0 to 1) where the tone is 0 and where it is 1, linear between and held beyond.
    #[must_use]
    pub fn density(mut self, at_tone_0: impl Into<Expr>, at_tone_1: impl Into<Expr>) -> Self {
        self.density = (at_tone_0.into(), at_tone_1.into());
        self
    }

    /// The seed that moves the dots (a whole number within ±(2^53 - 1), default 0).
    #[must_use]
    pub fn seed(mut self, seed: impl Into<Expr>) -> Self {
        self.seed = seed.into();
        self
    }

    pub(crate) fn spacing_of(&self) -> &Expr {
        &self.spacing
    }

    pub(crate) fn tone_of(&self) -> &Field {
        &self.tone
    }

    pub(crate) fn radius_of(&self) -> &(Expr, Expr) {
        &self.radius
    }

    pub(crate) fn density_of(&self) -> &(Expr, Expr) {
        &self.density
    }

    pub(crate) fn seed_of(&self) -> &Expr {
        &self.seed
    }
}

/// The numbers of a [`Stipple`] worked out.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Plan {
    pub(crate) spacing: f64,
    pub(crate) radius: (f64, f64),
    pub(crate) density: (f64, f64),
    pub(crate) seed: f64,
}

impl Path {
    /// Dots inside the region this path fills (nonzero rule): a lattice of cells `spacing` apart over the region's
    /// bounding box, each cell with one dot at most, its center inside the region and standing within 0.4 spacings of
    /// its cell's middle. The dot is there when the cell's share of the lattice (a threshold from 0 to 1 spread evenly
    /// over the lattice, moved by the seed) is below the density at the tone read at the dot's center; its radius is the
    /// radius at that tone. The result is round dots, closed, so set the fill after it. A Param is read at its default here; a `stipple` step reads it
    /// with the values it draws with. An empty path gives an empty path.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `spacing` if it is not above 0 and a coordinate, if the region needs more than 2
    /// million cells, or if a step of it would not move at the region's coordinates; naming `radius` unless both are
    /// above 0 and coordinates; `density` unless both are from 0 to 1; `seed` unless it is a whole number within
    /// ±(2^53 - 1); naming `spacing` past `MAX_SEGMENTS` points.
    pub fn stipple(&self, stipple: &Stipple) -> Result<Path, Error> {
        let values = Values::new();
        let plan = Plan {
            spacing: read("spacing", &stipple.spacing, &values)?,
            radius: (
                read("radius", &stipple.radius.0, &values)?,
                read("radius", &stipple.radius.1, &values)?,
            ),
            density: (
                read("density", &stipple.density.0, &values)?,
                read("density", &stipple.density.1, &values)?,
            ),
            seed: read("seed", &stipple.seed, &values)?,
        };
        self.stipple_charged(&plan, &stipple.tone, &mut segment_budget("spacing"))
    }

    /// As [`Path::stipple`] with the numbers worked out, calling `charge` with the points made before they are kept.
    pub(crate) fn stipple_charged(
        &self,
        plan: &Plan,
        tone: &Field,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Path, Error> {
        if !(plan.spacing.is_finite() && plan.spacing > 0.0 && plan.spacing <= MAX_COORDINATE) {
            return Err(Error::parameter(
                "spacing",
                format!("must be above 0, got {}", plan.spacing),
            ));
        }
        for radius in [plan.radius.0, plan.radius.1] {
            if !(radius.is_finite() && radius > 0.0 && radius <= MAX_COORDINATE) {
                return Err(Error::parameter("radius", format!("must be above 0, got {radius}")));
            }
        }
        for density in [plan.density.0, plan.density.1] {
            if !(0.0..=1.0).contains(&density) {
                return Err(Error::parameter(
                    "density",
                    format!("must be from 0 to 1, got {density}"),
                ));
            }
        }
        let seed = whole_seed(plan.seed)?;
        let Ok(bounds) = self.bounds() else {
            return Ok(Path::new());
        };
        if !self.may_fill() {
            return Ok(Path::new());
        }
        let reach = bounds
            .x
            .abs()
            .max(bounds.y.abs())
            .max((bounds.x + bounds.width).abs())
            .max((bounds.y + bounds.height).abs());
        if reach + plan.spacing * JITTER <= reach {
            return Err(Error::parameter(
                "spacing",
                format!("is too small for coordinates near {reach}: a step of it would not move"),
            ));
        }
        let (columns, rows) = (
            (bounds.width / plan.spacing).ceil().max(1.0),
            (bounds.height / plan.spacing).ceil().max(1.0),
        );
        if columns * rows > MAX_CELLS {
            return Err(Error::parameter(
                "spacing",
                format!("is too small for a region this large: it would need over {MAX_CELLS} cells"),
            ));
        }
        tone.charge_reads("spacing", "stipple", columns * rows)?;
        let outline = crate::field::Outline::of(self, "stipple", charge)?;
        // the seed moves the threshold sequence and the jitter alike
        let shift = unit(splitmix(seed, u64::MAX));
        let mut dots = Vec::new();
        for j in 0..rows as usize {
            for i in 0..columns as usize {
                let (jx, jy) = (
                    unit(splitmix(seed, 2 * (j as u64 * columns as u64 + i as u64))) - 0.5,
                    unit(splitmix(seed, 2 * (j as u64 * columns as u64 + i as u64) + 1)) - 0.5,
                );
                let at = Point::new(
                    bounds.x + (i as f64 + 0.5 + JITTER * jx) * plan.spacing,
                    bounds.y + (j as f64 + 0.5 + JITTER * jy) * plan.spacing,
                );
                let threshold = (shift + R2.0 * i as f64 + R2.1 * j as f64).fract();
                let share = tone.at(at).clamp(0.0, 1.0);
                let wanted = plan.density.0 + (plan.density.1 - plan.density.0) * share;
                if threshold < wanted && outline.contains(at) {
                    charge(5)?;
                    let radius = plan.radius.0 + (plan.radius.1 - plan.radius.0) * share;
                    dots.push(Path::circle(at, radius)?);
                }
            }
        }
        Ok(Path::joined(dots.iter()))
    }
}

/// A number from 0 up to 1 from the top 53 bits of `bits`.
fn unit(bits: u64) -> f64 {
    // constant: 2^-53, so that 53 bits make a number in [0, 1) exactly
    (bits >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
}
