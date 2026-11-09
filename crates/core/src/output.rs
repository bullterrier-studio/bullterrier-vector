//! Drawing a drawing: the output size every output takes, and the outputs (SVG, pixels and PNG so far). All of them place
//! the design frame in the output the same way (`view`), so that a picture is the same picture in each.

mod mesh;
mod outline;
mod png;
mod ramp;
mod raster;
mod screen;
mod svg;
mod view;

use crate::node::stack::MAX_FRAME;
use crate::{Error, ShadowStyle, Shape, Style};
pub use mesh::Mesh;

/// The longest side of a pixel output, in whole pixels.
// constant: the rasteriser's canvas is addressed by 16 bits and it panics when a row is padded past 65,535, which a
// width above 65,408 is (measured; the padding is a multiple of 128), so 65,000 keeps clear of it on every target; SVG
// has no such limit and keeps the design frame's
pub(crate) const MAX_PIXEL_SIDE: u16 = 65_000;

/// The most pixels a pixel output may have: 512 MiB of RGBA.
// constant: a 65,000 by 65,000 canvas would need 16 GiB; 2^27 pixels is the largest picture that fits the memory of
// the machines the library runs on, a browser tab included, where the image is held twice
pub(crate) const MAX_PIXELS: u64 = 1 << 27;

/// The size of an output in whole pixels: what [`Drawing::to_svg`] and [`Drawing::to_rgba`] draw into. The design frame
/// of the drawing ([`Drawing::with_size`]) is scaled to fit it, centred; a drawing without a frame is drawn at one
/// output pixel to the design pixel from the top left.
///
/// [`Drawing::to_svg`]: crate::Drawing::to_svg
/// [`Drawing::to_rgba`]: crate::Drawing::to_rgba
/// [`Drawing::with_size`]: crate::Drawing::with_size
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    width: f64,
    height: f64,
}

impl Size {
    /// An output `width` by `height` pixels. The sides are checked where the output is drawn: each must be a whole
    /// number from 1 to 100,000 for SVG, or from 1 to 65,000 (and at most 2^27 pixels in all) for pixels, or the
    /// drawing raises [`Error::Parameter`] naming `width` or `height`.
    #[must_use]
    pub const fn px(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// The sides of a vector output, or the error naming the first that is not a whole number of pixels from 1 to the
    /// largest side of a design frame (formats.md "Limits and security").
    pub(crate) fn checked(self) -> Result<(f64, f64), Error> {
        Ok((
            side("width", self.width, MAX_FRAME)?,
            side("height", self.height, MAX_FRAME)?,
        ))
    }

    /// The sides of a pixel output: whole pixels from 1 to [`MAX_PIXEL_SIDE`], [`MAX_PIXELS`] at most in all.
    pub(crate) fn checked_pixels(self) -> Result<(u16, u16), Error> {
        let largest = f64::from(MAX_PIXEL_SIDE);
        let (width, height) = (
            side("width", self.width, largest)?,
            side("height", self.height, largest)?,
        );
        // exact: both sides are below 2^16, so the product is below 2^32
        #[expect(clippy::cast_precision_loss, reason = "2^27 is exact in an f64")]
        if width * height > MAX_PIXELS as f64 {
            return Err(Error::parameter(
                "size",
                format!("width {width} by height {height} is more than {MAX_PIXELS} pixels"),
            ));
        }
        // both are whole numbers from 1 to 65,000, as `side` checked
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "whole numbers from 1 to 65,000"
        )]
        Ok((width as u16, height as u16))
    }
}

/// `value`, or the error naming `parameter` unless it is a whole number from 1 to `largest`.
fn side(parameter: &str, value: f64, largest: f64) -> Result<f64, Error> {
    if (1.0..=largest).contains(&value) && value.fract() == 0.0 {
        Ok(value)
    } else {
        Err(Error::parameter(
            parameter,
            format!("must be a whole number of pixels from 1 to {largest}, got {value}"),
        ))
    }
}

/// `error` as it concerns `shape`: a parameter error is renamed `shape N`, with `what` put before its message.
pub(crate) fn in_shape(shape: &Shape, what: &str, error: Error) -> Error {
    match error {
        Error::Parameter { message, .. } => {
            Error::parameter(&format!("shape {}", shape.index()), format!("{what}{message}"))
        }
        other => other,
    }
}

/// The shadow a shape casts and its colour's bytes: its own, if it has one that paints and the shape paints anything
/// for it to be the silhouette of.
pub(crate) fn cast(style: &Style) -> Option<(&ShadowStyle, [u8; 4])> {
    let shadow = style.shadow()?;
    let painted = fills(style) || style.stroke().is_some();
    painted.then(|| shadow.color().rgba().map(|rgba| (shadow, rgba)))?
}

/// Whether the fill of a style paints: a colour that is not none, a gradient, or a halftone.
pub(crate) fn fills(style: &Style) -> bool {
    style.fill().paints() || style.halftone().is_some()
}
