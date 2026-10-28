//! The one place the core does trigonometry: `libm` gives the same bits on every target, where the platform's own
//! `sin` and `cos` may differ (clippy.toml forbids them elsewhere).

/// The sine and cosine of `angle` (radians), in that order.
pub(crate) fn sin_cos(angle: f64) -> (f64, f64) {
    (libm::sin(angle), libm::cos(angle))
}
