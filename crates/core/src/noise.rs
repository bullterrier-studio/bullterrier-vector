//! Smooth seeded noise: the one field `wobble` displaces points by (and the one a flow field will turn by). Value noise
//! on a lattice of hashed values, blended by the quintic fade, so a field is smooth (continuous slope and curvature),
//! bounded by 1 in size, and a pure function of its seed and where it is read: integer hashing and `+ - *` only, the same
//! on every target, no state.
//!
//! The lattice has three axes: `x` and `y`, one cell a wavelength wide (the caller divides by the wavelength), and the
//! phase, four cells round one whole cycle. The cell of the phase wraps, so a phase one cycle on is the same field (to
//! the bit when `phase + 1` loses no bit of `phase`, as for 0.25; else up to that rounding of the caller's sum), and it
//! moves the field smoothly, across the end of the cycle too.

use crate::Error;
use crate::expr::{splitmix, whole_seed};
use crate::geometry::MAX_COORDINATE;

/// The wavelength, in design pixels, of `Flow.noise` and `Field.noise` when none is given (in Python and in documents).
// constant: a placeholder until the owner sets the defaults (`docs/api.md`, "Defaults"); `wobble`'s is 80, a separate
// interface pass unifies them
pub const NOISE_WAVELENGTH: f64 = 400.0;

/// The shortest wavelength of noise, in design pixels.
// constant: a coordinate within ±1e100 (`MAX_COORDINATE`) read at the eighth octave of `Field::noise` (128 times finer)
// over 1e-200 is 1.28e302, below the largest float (1.8e308); a shorter wavelength would overflow the cell to infinity,
// and the noise to NaN
const MIN_WAVELENGTH: f64 = 1e-200;

/// `wavelength`, or the error naming it unless it is from [`MIN_WAVELENGTH`] up to `MAX_COORDINATE`: the one rule of
/// `wobble`, `Flow::noise` and `Field::noise`.
pub(crate) fn wavelength(wavelength: f64) -> Result<f64, Error> {
    if (MIN_WAVELENGTH..=MAX_COORDINATE).contains(&wavelength) {
        Ok(wavelength)
    } else {
        Err(Error::parameter(
            "wavelength",
            format!("must be a number from {MIN_WAVELENGTH:e} to {MAX_COORDINATE:e}, got {wavelength}"),
        ))
    }
}

/// The cells of the phase axis in one cycle.
// constant: four cells make the field change fully in a quarter cycle: a pattern that is neither frozen nor racing
const PHASE_CELLS: i32 = 4;

/// A noise field. Equal seeds and streams make equal fields; another stream of the same seed is another field.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Noise {
    key: i64,
}

impl Noise {
    /// The field of `seed` and `stream` (two fields of one seed, as the two components of a displacement, differ by
    /// stream).
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `seed` unless it is a whole number within ±(2^53 - 1), as for `random`.
    pub(crate) fn new(seed: f64, stream: u64) -> Result<Self, Error> {
        Ok(Self {
            key: splitmix(whole_seed(seed)?, stream).cast_signed(),
        })
    }

    /// The field at `(x, y)` (in wavelengths) and `phase` (in cycles, any finite number: reduced to 0 up to 1 before
    /// anything else). A number in [-1, 1]; at most `3.75` per wavelength steeper than that in either direction, and
    /// `15` per cycle of phase.
    pub(crate) fn value(self, x: f64, y: f64, phase: f64) -> f64 {
        let t = phase.rem_euclid(1.0) * f64::from(PHASE_CELLS);
        let (floor_x, floor_y, floor_t) = (x.floor(), y.floor(), t.floor());
        let (along_x, along_y, along_t) = (fade(x - floor_x), fade(y - floor_y), fade(t - floor_t));
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a cell index; beyond i64 it saturates, as is fine"
        )]
        let (ix, iy, it) = (floor_x as i64, floor_y as i64, floor_t as i64);
        let corner = |dx: i64, dy: i64, dt: i64| self.lattice(ix.wrapping_add(dx), iy.wrapping_add(dy), it + dt);
        let layer = |dt: i64| {
            let low = lerp(corner(0, 0, dt), corner(1, 0, dt), along_x);
            let high = lerp(corner(0, 1, dt), corner(1, 1, dt), along_x);
            lerp(low, high, along_y)
        };
        lerp(layer(0), layer(1), along_t)
    }

    /// The hashed value of a lattice point, from -1 up to 1; the phase cell wraps round the cycle.
    #[expect(clippy::cast_precision_loss, reason = "53 bits make a float exactly")]
    fn lattice(self, ix: i64, iy: i64, it: i64) -> f64 {
        // constant: 2^-52, so that 53 bits make a number in [0, 2) exactly, then 1 below it
        const SCALE: f64 = 1.0 / 4_503_599_627_370_496.0;
        let mut h = self.key;
        for cell in [ix, iy, it.rem_euclid(i64::from(PHASE_CELLS))] {
            h = splitmix(h, cell.cast_unsigned()).cast_signed();
        }
        (h.cast_unsigned() >> 11) as f64 * SCALE - 1.0
    }
}

/// The quintic `6u^5 - 15u^4 + 10u^3`: 0 at 0, 1 at 1, flat at both ends in slope and curvature.
fn fade(u: f64) -> f64 {
    u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f64, b: f64, share: f64) -> f64 {
    a + (b - a) * share
}
