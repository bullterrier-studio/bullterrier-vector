//! Elliptical arcs as cubic Beziers: SVG's endpoint form, converted to centre form (SVG 1.1, appendix F.6.5 and F.6.6),
//! then cut into pieces of at most a quarter turn.

use super::Segment;
use crate::geometry::Point;
use crate::math::{atan2, sin_cos, sin_cos_deg, tan};
use std::f64::consts::{FRAC_PI_2, TAU};

/// How far past a quarter turn a piece may reach before the sweep takes one more piece, as a fraction of a quarter.
// constant: 1e-9 of a quarter turn is 1e-7 degrees, far below any visible effect, and keeps a sweep that rounding put
// a hair past an exact quarter, half or three quarters of a turn from being cut into one piece too many
const QUARTER_SLACK: f64 = 1e-9;
/// The most pieces a sweep takes: a full turn.
const MAX_PIECES: u32 = 4;

/// The arc from `from` to `to` with radii `rx` and `ry` on an ellipse turned by `rotation` degrees, as segments
/// ending on `to`: a straight line when a radius is 0, nothing when the ends are the same point, and otherwise one
/// cubic per quarter turn at most. `large` picks the sweep over a half turn, `sweep` the clockwise (positive angle)
/// way. Radii too small to reach are scaled up until they do.
pub(super) fn arc_segments(
    from: Point,
    (rx, ry): (f64, f64),
    rotation: f64,
    (large, sweep): (bool, bool),
    to: Point,
) -> Vec<Segment> {
    if from == to {
        return Vec::new();
    }
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx == 0.0 || ry == 0.0 {
        return vec![Segment::Line(to)];
    }
    let (sin_phi, cos_phi) = sin_cos_deg(rotation);
    // F.6.5 step 1: the chord's half, in the ellipse's own axes
    let (reach_x, reach_y) = ((from.x - to.x) / 2.0, (from.y - to.y) / 2.0);
    let x1 = cos_phi * reach_x + sin_phi * reach_y;
    let y1 = -sin_phi * reach_x + cos_phi * reach_y;
    // F.6.6: radii too small to span the chord grow until the ellipse just reaches both ends
    let overshoot = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    let scaled = overshoot > 1.0;
    if scaled {
        let growth = overshoot.sqrt();
        rx *= growth;
        ry *= growth;
    }
    // F.6.5 step 2: the centre in the ellipse's axes. Scaled-up radii reach the chord's ends with no room to spare,
    // so the centre is the chord's middle; otherwise the radicand is not below 0 but rounding may leave it a hair under
    let side = if large == sweep { -1.0 } else { 1.0 };
    let coefficient = if scaled {
        0.0
    } else {
        let radicand =
            (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1) / (rx * rx * y1 * y1 + ry * ry * x1 * x1);
        side * radicand.max(0.0).sqrt()
    };
    let (centre_x, centre_y) = (coefficient * rx * y1 / ry, -coefficient * ry * x1 / rx);
    // step 3: the centre in the drawing's axes
    let centre = Point::new(
        cos_phi * centre_x - sin_phi * centre_y + f64::midpoint(from.x, to.x),
        sin_phi * centre_x + cos_phi * centre_y + f64::midpoint(from.y, to.y),
    );
    // step 4: the start angle, and the sweep, which the flag decides the sign of
    let (ux, uy) = ((x1 - centre_x) / rx, (y1 - centre_y) / ry);
    let (vx, vy) = ((-x1 - centre_x) / rx, (-y1 - centre_y) / ry);
    let start = atan2(uy, ux);
    let mut sweep_angle = atan2(ux * vy - uy * vx, ux * vx + uy * vy);
    if !sweep && sweep_angle > 0.0 {
        sweep_angle -= TAU;
    } else if sweep && sweep_angle < 0.0 {
        sweep_angle += TAU;
    }
    let pieces = (1..=MAX_PIECES)
        .find(|pieces| sweep_angle.abs() <= f64::from(*pieces) * FRAC_PI_2 * (1.0 + QUARTER_SLACK))
        .unwrap_or(MAX_PIECES);
    let step = sweep_angle / f64::from(pieces);
    // the handle that makes a cubic follow a circular arc of `step`, as a multiple of the radius
    let handle = 4.0 / 3.0 * tan(step / 4.0);
    // the point of the ellipse at `angle`, and the direction it is travelled in there
    let at = |angle: f64| {
        let (sin, cos) = sin_cos(angle);
        let point = Point::new(
            centre.x + rx * cos * cos_phi - ry * sin * sin_phi,
            centre.y + rx * cos * sin_phi + ry * sin * cos_phi,
        );
        let direction = Point::new(
            -rx * sin * cos_phi - ry * cos * sin_phi,
            -rx * sin * sin_phi + ry * cos * cos_phi,
        );
        (point, direction)
    };
    (0..pieces)
        .map(|index| {
            let (begin, begin_direction) = at(start + f64::from(index) * step);
            let (finish, finish_direction) = at(start + f64::from(index + 1) * step);
            Segment::Cubic(
                Point::new(
                    begin.x + handle * begin_direction.x,
                    begin.y + handle * begin_direction.y,
                ),
                Point::new(
                    finish.x - handle * finish_direction.x,
                    finish.y - handle * finish_direction.y,
                ),
                // the last piece ends exactly where the arc was asked to
                if index + 1 == pieces { to } else { finish },
            )
        })
        .collect()
}
