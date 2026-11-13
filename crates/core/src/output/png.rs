//! `Drawing::to_png`: the pixels of `to_rgba` as a PNG file. The only module that uses the `png` crate; nothing of it
//! reaches the API.

use super::{Size, Window};
use crate::{Drawing, Error, Values};
use png::{BitDepth, ColorType, Compression, Encoder, Filter};

/// How the encoder squeezes the image.
// constant: the encoder's fast mode (a DEFLATE tuned for PNG) takes a fraction of the time of its balanced one for a
// file a few percent larger, and a flat vector picture is small either way; fixed here so that one version of the
// crate gives one file for one picture
const COMPRESSION: Compression = Compression::Fast;

/// Which of the five PNG row filters the encoder tries on each row.
// constant: the encoder's own per-row choice (its default) suits both the flat areas and the soft edges of a vector
// picture; named here so that it is a choice, not an accident of the crate's defaults
const FILTER: Filter = Filter::Adaptive;

impl Drawing {
    /// The drawing as a PNG file of `size`: 8 bits a channel, red, green, blue and alpha (colour type 6), with the
    /// pixels exactly those [`Drawing::to_rgba`] returns for the same `size` and `values`, so transparent where
    /// nothing is drawn and not premultiplied.
    ///
    /// The same call gives the same bytes. The file's bytes may change between versions of the library, as the encoder
    /// is updated; what they decode to does not.
    ///
    /// # Errors
    /// As [`Drawing::to_rgba`]. [`Error::Geometry`] naming `to_png` if the encoder fails, which it does not for the
    /// sizes `to_rgba` accepts.
    pub fn to_png(&self, size: Size, values: &Values) -> Result<Vec<u8>, Error> {
        let (width, height) = size.checked_pixels()?;
        let pixels = self.to_rgba(size, values)?;
        encode(&pixels, width, height)
    }
}

impl Drawing {
    /// The PNG file of `window`, a rectangle of the output `size`: the pixels of [`Drawing::to_rgba_window`] as
    /// [`Drawing::to_png`] writes them, so that a window's file decodes to the rows and columns of the whole picture.
    ///
    /// # Errors
    /// As [`Drawing::to_rgba_window`].
    pub fn to_png_window(&self, size: Size, window: Window, values: &Values) -> Result<Vec<u8>, Error> {
        let cells = window.checked(size.checked()?)?;
        let pixels = self.to_rgba_window(size, window, values)?;
        // checked: whole pixels, at most 65,000 a side
        #[expect(clippy::cast_possible_truncation, reason = "at most 65,000")]
        encode(&pixels, cells.width as u16, cells.height as u16)
    }
}

/// The PNG file of the straight-alpha `pixels`, `width` by `height`, rows from the top.
fn encode(pixels: &[u8], width: u16, height: u16) -> Result<Vec<u8>, Error> {
    let mut file = Vec::new();
    let mut encoder = Encoder::new(&mut file, u32::from(width), u32::from(height));
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_compression(COMPRESSION);
    encoder.set_filter(FILTER);
    let failed = |error: png::EncodingError| Error::geometry("to_png", format!("the PNG encoder failed: {error}"));
    let mut writer = encoder.write_header().map_err(failed)?;
    writer.write_image_data(pixels).map_err(failed)?;
    writer.finish().map_err(failed)?;
    Ok(file)
}
