//! The walking skeleton: a regular polygon as an SVG document.

use crate::Error;
use crate::math::sin_cos;
use std::f64::consts::TAU;

/// The most sides a polygon may have.
const MAX_SIDES: u16 = 1000;
/// The fewest sides a polygon may have.
const MIN_SIDES: u16 = 3;
/// The largest radius accepted, in user units.
const MAX_RADIUS: f64 = 1_000_000.0;

/// A regular polygon of `sides` corners on a circle of `radius` around the origin, as an SVG document.
///
/// The first corner is straight up, at `(0, -radius)` (SVG's y axis points down); the rest follow clockwise. The
/// `viewBox` frames the circle. Numbers have at most six decimals, with no exponent.
///
/// # Errors
/// [`Error::Parameter`] if `sides` is not in 3 to 1000, or `radius` is not finite and in (0, 1 000 000].
pub fn polygon_svg(sides: i64, radius: f64) -> Result<String, Error> {
    let count = u16::try_from(sides)
        .ok()
        .filter(|count| (MIN_SIDES..=MAX_SIDES).contains(count))
        .ok_or_else(|| Error::Parameter {
            parameter: "sides",
            message: format!("must be a whole number from {MIN_SIDES} to {MAX_SIDES}, got {sides}"),
        })?;
    if !(radius.is_finite() && radius > 0.0 && radius <= MAX_RADIUS) {
        return Err(Error::Parameter {
            parameter: "radius",
            message: format!("must be more than 0 and at most {MAX_RADIUS}, got {radius}"),
        });
    }
    let corners = (0..count)
        .map(|index| {
            let (sin, cos) = sin_cos(TAU * f64::from(index) / f64::from(count));
            format!("{},{}", number(radius * sin), number(-radius * cos))
        })
        .collect::<Vec<_>>()
        .join(" ");
    let (low, span) = (number(-radius), number(2.0 * radius));
    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{low} {low} {span} {span}"><polygon points="{corners}"/></svg>"#
    ))
}

/// `value` as plain decimal text: rounded to 6 decimals, trailing zeros and point trimmed, `-0` written `0`.
// constant: 1e-6 px is below any screen or plotter resolution
fn number(value: f64) -> String {
    let fixed = format!("{value:.6}");
    let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "-0" {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}
