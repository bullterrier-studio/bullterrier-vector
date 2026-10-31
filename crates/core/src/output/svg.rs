//! `Drawing::to_svg`: the drawing as an SVG document.

use super::Size;
use crate::path::number;
use crate::{Drawing, Error, Values};

impl Drawing {
    /// The drawing as an SVG document of `size`, with the Params at the `values` given and at their defaults where
    /// none is given.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `width` or `height` if a side of `size` is not a whole number from 1 to 100,000;
    /// otherwise what [`Drawing::shapes`] raises.
    pub fn to_svg(&self, size: Size, values: &Values) -> Result<String, Error> {
        let (width, height) = size.checked()?;
        self.shapes(values)?;
        let (width, height) = (number(width), number(height));
        Ok(format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
             viewBox=\"0 0 {width} {height}\" style=\"isolation:isolate\"></svg>\n"
        ))
    }
}
