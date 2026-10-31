//! `Drawing::to_svg`: the drawing as an SVG document.

use super::Size;
use super::view::View;
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
        let shapes = self.shapes(values)?;
        let view = View::fit(self.size(), (width, height));
        let (width, height) = (number(width), number(height));
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
             viewBox=\"0 0 {width} {height}\" style=\"isolation:isolate\">\n"
        );
        for shape in &shapes {
            let data = view.path(shape.path())?.to_svg_path();
            if !data.is_empty() {
                svg.push_str(&format!("<path d=\"{data}\"/>\n"));
            }
        }
        svg.push_str("</svg>\n");
        Ok(svg)
    }
}
