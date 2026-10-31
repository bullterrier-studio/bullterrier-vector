//! The smallest rectangle round a path.

use super::{Path, Segment};
use crate::Error;
use crate::geometry::{Point, Rect};

impl Path {
    /// The smallest rectangle that holds the whole outline. A curve counts where it bulges out, not where its
    /// control points are.
    ///
    /// # Errors
    /// [`Error::Geometry`] if the path is empty.
    pub fn bounds(&self) -> Result<Rect, Error> {
        let mut xs: Vec<f64> = Vec::new();
        let mut ys: Vec<f64> = Vec::new();
        let mut keep = |point: Point| {
            xs.push(point.x);
            ys.push(point.y);
        };
        for subpath in &self.subpaths {
            keep(subpath.start);
            let mut from = subpath.start;
            for segment in &subpath.segments {
                match *segment {
                    Segment::Line(end) => keep(end),
                    Segment::Quad(control, end) => {
                        keep(end);
                        for t in quad_turns(from.x, control.x, end.x)
                            .into_iter()
                            .chain(quad_turns(from.y, control.y, end.y))
                        {
                            keep(Point::new(
                                quad_at(from.x, control.x, end.x, t),
                                quad_at(from.y, control.y, end.y, t),
                            ));
                        }
                    }
                    Segment::Cubic(control1, control2, end) => {
                        keep(end);
                        for t in cubic_turns(from.x, control1.x, control2.x, end.x)
                            .into_iter()
                            .chain(cubic_turns(from.y, control1.y, control2.y, end.y))
                        {
                            keep(Point::new(
                                cubic_at(from.x, control1.x, control2.x, end.x, t),
                                cubic_at(from.y, control1.y, control2.y, end.y, t),
                            ));
                        }
                    }
                }
                from = segment.end();
            }
        }
        let (Some((left, right)), Some((top, bottom))) = (extent(&xs), extent(&ys)) else {
            return Err(Error::geometry("bounds", "the path is empty"));
        };
        Ok(Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })
    }
}

/// The smallest and largest of `values`; `None` if there are none.
fn extent(values: &[f64]) -> Option<(f64, f64)> {
    values.iter().copied().fold(None, |range, value| match range {
        None => Some((value, value)),
        Some((low, high)) => Some((low.min(value), high.max(value))),
    })
}

/// The parameters strictly between 0 and 1 where one coordinate of a quadratic Bezier turns round.
fn quad_turns(p0: f64, p1: f64, p2: f64) -> Option<f64> {
    let slope_change = p0 - 2.0 * p1 + p2;
    let t = (p0 - p1) / slope_change;
    (t > 0.0 && t < 1.0).then_some(t)
}

/// The parameters strictly between 0 and 1 where one coordinate of a cubic Bezier turns round: the roots of its
/// derivative, a quadratic `a t^2 + b t + c` (up to the factor 3).
fn cubic_turns(p0: f64, p1: f64, p2: f64, p3: f64) -> Vec<f64> {
    let a = p3 - 3.0 * p2 + 3.0 * p1 - p0;
    let b = 2.0 * (p2 - 2.0 * p1 + p0);
    let c = p1 - p0;
    let mut roots = Vec::new();
    if a == 0.0 {
        roots.push(-c / b);
    } else {
        let discriminant = b * b - 4.0 * a * c;
        if discriminant >= 0.0 {
            // the numerically stable form of the quadratic formula: no cancellation between b and the root
            let q = -0.5 * (b + discriminant.sqrt().copysign(b));
            roots.push(q / a);
            roots.push(c / q);
        }
    }
    roots.retain(|t| *t > 0.0 && *t < 1.0);
    roots
}

fn quad_at(p0: f64, p1: f64, p2: f64, t: f64) -> f64 {
    let u = 1.0 - t;
    u * u * p0 + 2.0 * u * t * p1 + t * t * p2
}

fn cubic_at(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let u = 1.0 - t;
    u * u * u * p0 + 3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t * p3
}
