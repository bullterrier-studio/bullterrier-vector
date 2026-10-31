//! The one place the core does trigonometry: `libm` gives the same bits on every target, where the platform's own
//! `sin` and `cos` may differ (clippy.toml forbids them elsewhere).

/// The sine and cosine of `angle` (radians), in that order.
pub(crate) fn sin_cos(angle: f64) -> (f64, f64) {
    (libm::sin(angle), libm::cos(angle))
}

/// The sine and cosine of `degrees`, in that order, exact at the quarter turns (a multiple of 90 gives 0, 1 or -1, not
/// a residue of 1e-16).
#[expect(
    clippy::float_cmp,
    reason = "the quarter turns are matched exactly: that is the point"
)]
pub(crate) fn sin_cos_deg(degrees: f64) -> (f64, f64) {
    let turn = degrees.rem_euclid(360.0);
    if turn == 0.0 {
        (0.0, 1.0)
    } else if turn == 90.0 {
        (1.0, 0.0)
    } else if turn == 180.0 {
        (0.0, -1.0)
    } else if turn == 270.0 {
        (-1.0, 0.0)
    } else {
        sin_cos(turn.to_radians())
    }
}
