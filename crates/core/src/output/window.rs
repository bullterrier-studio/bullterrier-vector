//! The window of a pixel output: which rectangle of the whole canvas is drawn, and the larger region the rasteriser
//! must work in to draw it exactly. The one place that knows how far a pixel's colour depends on its neighbours.

use super::{MAX_PIXEL_SIDE, MAX_PIXELS};

/// The side of a tile of the rasteriser (`vello_common`'s `Tile`), in pixels.
const TILE: u32 = 4; // constant: the rasteriser's, measured as the alignment a window's rows need

/// How wide the rasteriser's wide tiles are, in pixels, in which it composites layers; a region starts at a multiple.
const WIDE_TILE: u32 = 256; // constant: the rasteriser's; a multiple of a tile, and of any decimation grid below it

/// The pixels around a window that the rasteriser draws too: a pixel within a tile of the edge of the picture it is
/// given is composited in a layer with a rounding of its own, so no pixel of a window is that near.
const EDGE: u32 = 2 * TILE; // constant: measured, a tile (4) is enough in the tests and this is twice that
use crate::Error;

/// A rectangle of the output, in whole pixels from its top left: what [`Drawing::to_rgba_window`] and
/// [`Drawing::to_png_window`] draw. It is checked where it is drawn, against the [`Size`] of the whole output.
///
/// [`Drawing::to_rgba_window`]: crate::Drawing::to_rgba_window
/// [`Drawing::to_png_window`]: crate::Drawing::to_png_window
/// [`Size`]: crate::Size
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Window {
    /// The `width` by `height` pixels whose top left is column `x` and row `y` of the output. The window must lie
    /// inside the output and hold between 1 and 65,000 pixels a side and at most 2^27 in all, or the drawing raises
    /// [`Error::Parameter`] naming `window`.
    #[must_use]
    pub const fn px(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    /// The window as whole pixels inside `canvas` (width, height).
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `window` if it is not whole pixels inside the canvas, a side is past
    /// [`MAX_PIXEL_SIDE`] or it holds more than [`MAX_PIXELS`].
    pub(crate) fn checked(self, canvas: (f64, f64)) -> Result<Cells, Error> {
        let refuse = |why: String| Error::parameter("window", why);
        let whole = |value: f64, least: f64, most: f64, what: &str| {
            if (least..=most).contains(&value) && value.fract() == 0.0 {
                Ok(value)
            } else {
                Err(refuse(format!(
                    "{what} must be a whole number from {least} to {most}, got {value}"
                )))
            }
        };
        let (x, y) = (
            whole(self.x, 0.0, canvas.0 - 1.0, "x")?,
            whole(self.y, 0.0, canvas.1 - 1.0, "y")?,
        );
        let largest = f64::from(MAX_PIXEL_SIDE);
        let width = whole(self.width, 1.0, (canvas.0 - x).min(largest), "width")?;
        let height = whole(self.height, 1.0, (canvas.1 - y).min(largest), "height")?;
        let cells = Cells::of(x, y, width, height);
        cells.within_limits().map(|()| cells)
    }
}

/// A rectangle of whole pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cells {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Cells {
    /// The rectangle of four whole numbers that fit 32 bits.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "whole numbers of at most 100,000"
    )]
    fn of(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x: x as u32,
            y: y as u32,
            width: width as u32,
            height: height as u32,
        }
    }

    /// The whole of a canvas.
    pub(crate) fn canvas(canvas: (f64, f64)) -> Self {
        Self::of(0.0, 0.0, canvas.0, canvas.1)
    }

    fn within_limits(self) -> Result<(), Error> {
        let side = u32::from(MAX_PIXEL_SIDE);
        if self.width > side || self.height > side || u64::from(self.width) * u64::from(self.height) > MAX_PIXELS {
            return Err(Error::parameter(
                "window",
                format!(
                    "{} by {} pixels is past {MAX_PIXEL_SIDE} a side or {MAX_PIXELS} in all",
                    self.width, self.height
                ),
            ));
        }
        Ok(())
    }

    /// The region the rasteriser works in to draw this window exactly: the window grown by `margin` pixels on every
    /// side, and by [`EDGE`] at least, its top left moved up and left to a multiple of `align` (which a blur's
    /// decimation grid needs to be the same as in the whole picture) and of the rasteriser's tiles, and kept inside
    /// the `canvas`, past which no pixel is wanted. A region is the whole picture's pixels at the same places in
    /// those tiles, with no pixel of the window at its edge, or at the canvas's own edge.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `window` if the region is past the limits of a pixel output.
    pub(crate) fn region(self, margin: u32, align: u32, canvas: Self) -> Result<Self, Error> {
        let margin = margin.max(EDGE);
        let (across, down) = Self::tiles(align);
        let left = self.x.saturating_sub(margin) / across * across;
        let top = self.y.saturating_sub(margin) / down * down;
        let right = self
            .x
            .saturating_add(self.width)
            .saturating_add(margin)
            .min(canvas.width);
        let bottom = self
            .y
            .saturating_add(self.height)
            .saturating_add(margin)
            .min(canvas.height);
        Self {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
        .within_region_limits(margin)
    }

    /// This region, with its top left moved up and left to the grid line at or before `corner`, if that is further.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `window` if the region is past the limits of a pixel output.
    pub(crate) fn reaching(self, corner: (f64, f64), align: u32, margin: u32) -> Result<Self, Error> {
        let (across, down) = Self::tiles(align);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a whole number from 0 to the canvas, saturating"
        )]
        let (x, y) = (corner.0.max(0.0).floor() as u32, corner.1.max(0.0).floor() as u32);
        let (left, top) = (self.x.min(x / across * across), self.y.min(y / down * down));
        Self {
            x: left,
            y: top,
            width: self.x + self.width - left,
            height: self.y + self.height - top,
        }
        .within_region_limits(margin.max(EDGE))
    }

    /// The tiles a region starts on, across and down, when the blurs ask for the grid `align`.
    fn tiles(align: u32) -> (u32, u32) {
        (align.max(WIDE_TILE), align.max(TILE))
    }

    fn within_region_limits(self, margin: u32) -> Result<Self, Error> {
        self.within_limits().map_err(|error| match error {
            Error::Parameter { message, .. } => Error::parameter(
                "window",
                format!("with the {margin} pixels it is drawn from around it, the work region is {message}"),
            ),
            other => other,
        })?;
        Ok(self)
    }

    /// The rows and columns of `pixels`, the RGBA of `region` (which holds this window), that are this window's.
    pub(crate) fn cut(self, region: Self, pixels: &[u8]) -> Vec<u8> {
        let row = |y: u32| (y - region.y) as usize * region.width as usize;
        let (from, to) = ((self.x - region.x) as usize, (self.x - region.x + self.width) as usize);
        (self.y..self.y + self.height)
            .flat_map(|y| pixels[(row(y) + from) * 4..(row(y) + to) * 4].iter().copied())
            .collect()
    }
}
