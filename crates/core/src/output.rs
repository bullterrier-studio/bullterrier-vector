//! Drawing a drawing: the output size every output takes, and the outputs (SVG so far). All of them place the design
//! frame in the output the same way (`view`), so that a picture is the same picture in each.

mod svg;

use crate::Error;
use crate::node::stack::MAX_FRAME;

/// The size of an output in whole pixels: what [`Drawing::to_svg`] draws into. The design frame of the drawing
/// ([`Drawing::with_size`]) is scaled to fit it, centred; a drawing without a frame is drawn at one output pixel to
/// the design pixel from the top left.
///
/// [`Drawing::to_svg`]: crate::Drawing::to_svg
/// [`Drawing::with_size`]: crate::Drawing::with_size
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    width: f64,
    height: f64,
}

impl Size {
    /// An output `width` by `height` pixels. The sides are checked where the output is drawn: each must be a whole
    /// number from 1 to 100,000, or the drawing raises [`Error::Parameter`] naming `width` or `height`.
    #[must_use]
    pub const fn px(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// The sides, or the error naming the first that is not a whole number of pixels within the limits.
    pub(crate) fn checked(self) -> Result<(f64, f64), Error> {
        Ok((side("width", self.width)?, side("height", self.height)?))
    }
}

/// `value`, or the error naming `parameter` unless it is a whole number from 1 to the largest side of a design frame
/// (the output limit is the same, formats.md "Limits and security").
fn side(parameter: &str, value: f64) -> Result<f64, Error> {
    if (1.0..=MAX_FRAME).contains(&value) && value.fract() == 0.0 {
        Ok(value)
    } else {
        Err(Error::parameter(
            parameter,
            format!("must be a whole number of pixels from 1 to {MAX_FRAME}, got {value}"),
        ))
    }
}
