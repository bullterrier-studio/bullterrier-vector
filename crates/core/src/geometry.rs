//! The two plain values geometry is made of: a point and a rectangle.

use crate::Error;

/// A position in design pixels. `y` points down.
///
/// Anywhere a point goes in, a `(x, y)` tuple of numbers does too.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Distance to the right.
    pub x: f64,
    /// Distance down.
    pub y: f64,
}

impl Point {
    /// The point `(x, y)`.
    #[must_use]
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl From<(f64, f64)> for Point {
    fn from((x, y): (f64, f64)) -> Self {
        Self { x, y }
    }
}

/// An axis-aligned rectangle: its top-left corner and its size, in design pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Size to the right.
    pub width: f64,
    /// Size downwards.
    pub height: f64,
}

/// `value`, or the error naming `parameter` if it is NaN or infinite.
pub(crate) fn finite(parameter: &str, value: f64) -> Result<f64, Error> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::parameter(
            parameter,
            format!("must be a finite number, got {value}"),
        ))
    }
}

/// `point`, or the error naming `parameter` if a coordinate is NaN or infinite.
pub(crate) fn finite_point(parameter: &str, point: impl Into<Point>) -> Result<Point, Error> {
    let point = point.into();
    if point.x.is_finite() && point.y.is_finite() {
        Ok(point)
    } else {
        Err(Error::parameter(
            parameter,
            format!("must be finite numbers, got ({}, {})", point.x, point.y),
        ))
    }
}
