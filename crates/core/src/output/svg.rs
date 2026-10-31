//! `Drawing::to_svg`: the drawing as an SVG document, one `<path>` per shape in paint order.

use super::Size;
use super::view::View;
use crate::path::number;
use crate::{Blend, Color, Drawing, Error, Shape, Values};
use std::fmt::Write as _;

/// The largest alpha byte of a colour, which is fully opaque.
// constant: an alpha byte is 0..=255
const OPAQUE: f64 = 255.0;

impl Drawing {
    /// The drawing as an SVG document of `size`, with the Params at the `values` given and at their defaults where
    /// none is given.
    ///
    /// The design frame is scaled to fit `size` and centred (a drawing without a frame is drawn at one pixel to the
    /// design pixel), and content outside the frame is not clipped. The coordinates and stroke widths are written in
    /// output pixels, not as a transform, so that a drawing at twice the size has numbers exactly twice as large; every
    /// number is the shortest text that reads back as the same value. A shape is one `<path>` whose `opacity` applies
    /// to its fill and stroke together, as the pixels composite them; a shape whose path is empty writes no element.
    /// The root has `isolation:isolate`, so that multiply blends with the picture and not with the page it is in.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `width` or `height` if a side of `size` is not a whole number from 1 to 100,000;
    /// naming the shape if its coordinates would be beyond ±1e100 at this size; otherwise what [`Drawing::shapes`]
    /// raises.
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
            element(&mut svg, shape, &view)?;
        }
        svg.push_str("</svg>\n");
        Ok(svg)
    }
}

/// Appends the `<path>` of `shape` seen through `view`, or nothing if its path is empty.
fn element(svg: &mut String, shape: &Shape, view: &View) -> Result<(), Error> {
    let data = view
        .path(shape.path())
        .map_err(|error| match error {
            Error::Parameter { message, .. } => Error::parameter(&format!("shape {}", shape.index()), message),
            other => other,
        })?
        .to_svg_path();
    if data.is_empty() {
        return Ok(());
    }
    let style = shape.style();
    let _ = write!(svg, "<path d=\"{data}\"");
    paint(svg, "fill", style.fill());
    if let Some(stroke) = style.stroke() {
        paint(svg, "stroke", stroke.color());
        let _ = write!(
            svg,
            " stroke-width=\"{}\" stroke-linejoin=\"{}\" stroke-linecap=\"{}\" stroke-miterlimit=\"{}\"",
            number(view.length(stroke.width())),
            stroke.join().name(),
            stroke.cap().name(),
            number(stroke.miter_limit()),
        );
    }
    if style.opacity() < 1.0 {
        let _ = write!(svg, " opacity=\"{}\"", number(style.opacity()));
    }
    if style.blend() == Blend::Multiply {
        svg.push_str(" style=\"mix-blend-mode:multiply\"");
    }
    svg.push_str("/>\n");
    Ok(())
}

/// Appends the attribute `name` (`fill` or `stroke`) for `color`, and `<name>-opacity` where its alpha is below 255.
fn paint(svg: &mut String, name: &str, color: Color) {
    let Some([red, green, blue, alpha]) = color.rgba() else {
        let _ = write!(svg, " {name}=\"none\"");
        return;
    };
    let _ = write!(svg, " {name}=\"#{red:02x}{green:02x}{blue:02x}\"");
    if alpha < u8::MAX {
        let _ = write!(svg, " {name}-opacity=\"{}\"", number(f64::from(alpha) / OPAQUE));
    }
}
