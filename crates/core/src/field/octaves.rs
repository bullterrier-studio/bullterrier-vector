//! `Field::noise`: the seeded value noise of `noise.rs` summed over octaves.

use super::{Field, FieldKind, once};
use crate::geometry::Point;
use crate::noise::{self, Noise};
use crate::{Error, Expr};

/// The most octaves a noise field sums.
// constant: the eighth octave has 1/128 of the first's wavelength and amplitude, below anything a ninth would show
pub(crate) const MAX_OCTAVES: usize = 8;

/// How much shorter each octave's wavelength is than the one before.
// constant: the standard lacunarity of fractal noise: each octave an octave in the musical sense, twice the frequency
const LACUNARITY: f64 = 2.0;

/// How much smaller each octave's amplitude is than the one before.
// constant: the standard gain of fractal noise (1 / lacunarity), so the detail keeps one slope at every scale
const GAIN: f64 = 0.5;

/// The cost of reading one octave, in units of `MAX_READ_WORK` (`Field::charge_reads`).
// constant: measured (release build, `to_map` of a million reads): one octave 38 ns a read, eight 138 ns, against the 100
// ns a unit of `MAX_READ_WORK` stands for (2 microseconds for a headline's distance of about 20 units); one whole unit
// an octave charges noise 3 to 6 times what it costs: on the safe side
pub(crate) const NOISE_READ: f64 = 1.0;

/// A made noise field: its numbers worked out.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Octaves {
    noise: Noise,
    wavelength: f64,
    phase: f64,
    count: usize,
}

impl Octaves {
    /// The noise of `seed` at `wavelength` and `phase`, summed over `count` octaves.
    pub(super) fn new(wavelength: f64, phase: f64, seed: f64, count: usize) -> Result<Self, Error> {
        Ok(Self {
            wavelength: noise::wavelength(wavelength)?,
            noise: Noise::new(seed, 0)?,
            phase,
            count,
        })
    }

    pub(super) fn count(self) -> usize {
        self.count
    }

    /// The sum of the octaves at `point`, divided by the sum of their amplitudes: within -1 and 1.
    pub(super) fn at(self, point: Point) -> f64 {
        let (mut total, mut weights, mut weight, mut scale) = (0.0, 0.0, 1.0, 1.0);
        for _ in 0..self.count {
            let (x, y) = (point.x * scale / self.wavelength, point.y * scale / self.wavelength);
            total += weight * self.noise.value(x, y, self.phase);
            weights += weight;
            weight *= GAIN;
            scale *= LACUNARITY;
        }
        total / weights
    }
}

impl Field {
    /// Smooth seeded noise from -1 to 1: the value noise `wobble` displaces by, one lattice cell `wavelength` design
    /// pixels wide, moved by `phase` (cycles; one cycle on is the same field to the bit). With `octaves` from 1 to 8,
    /// each octave adds the noise at half the wavelength and half the amplitude of the one before, and the sum is
    /// divided by the sum of the amplitudes, so it stays within -1 and 1; one octave is the plain noise. Equal seeds
    /// make equal fields. `wavelength` and `phase` may be expressions of Params, read once with the values of the draw.
    ///
    /// # Errors
    /// [`Error::Parameter`] unless `wavelength` is from 1e-200 to 1e100, `phase` finite (and neither a per-item number),
    /// `seed` a whole number within ±(2^53 - 1) and `octaves` from 1 to 8.
    pub fn noise(
        wavelength: impl Into<Expr>,
        phase: impl Into<Expr>,
        seed: f64,
        octaves: usize,
    ) -> Result<Self, Error> {
        if !(1..=MAX_OCTAVES).contains(&octaves) {
            return Err(Error::parameter(
                "octaves",
                format!("must be a whole number from 1 to {MAX_OCTAVES}, got {octaves}"),
            ));
        }
        Self::made(&FieldKind::Noise {
            wavelength: once("wavelength", &wavelength.into())?,
            phase: once("phase", &phase.into())?,
            seed,
            octaves,
        })
    }
}
