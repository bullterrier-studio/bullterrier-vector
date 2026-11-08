//! `Drawing::to_svg`: the drawing as an SVG document, one `<path>` per shape in paint order.

use super::cast;
use super::view::View;
use super::{Size, in_shape};
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
    /// naming the shape if its coordinates or stroke width would be beyond ±1e100 at this size; otherwise what [`Drawing::shapes`]
    /// raises.
    pub fn to_svg(&self, size: Size, values: &Values) -> Result<String, Error> {
        let canvas = size.checked()?;
        let (width, height) = canvas;
        let shapes = self.shapes(values)?;
        let view = View::fit(self.size(), canvas);
        let (width, height) = (number(width), number(height));
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
             viewBox=\"0 0 {width} {height}\" style=\"isolation:isolate\">\n"
        );
        for shape in &shapes {
            element(&mut svg, shape, &view, canvas)?;
        }
        svg.push_str("</svg>\n");
        Ok(svg)
    }
}

/// Appends the element of `shape` seen through `view` on a canvas of `canvas` pixels, or nothing if its path is empty.
///
/// With a shadow the opacity and the blend belong to a `<g>` around the shadow and the shape, so that they apply once
/// to both. A blurred shadow is that group's `filter`: the blur, offset and tint of the shape's alpha, merged under the
/// shape. A shadow with no blur is a plain copy of the path before the shape, tinted.
fn element(svg: &mut String, shape: &Shape, view: &View, canvas: (f64, f64)) -> Result<(), Error> {
    let in_shape = |what: &'static str| move |error| in_shape(shape, what, error);
    let device = view.path(shape.path()).map_err(in_shape(""))?;
    let data = device.to_svg_path();
    if data.is_empty() {
        return Ok(());
    }
    let style = shape.style();
    let group = |svg: &mut String| {
        if style.opacity() < 1.0 {
            let _ = write!(svg, " opacity=\"{}\"", number(style.opacity()));
        }
        if style.blend() == Blend::Multiply {
            svg.push_str(" style=\"mix-blend-mode:multiply\"");
        }
    };
    let Some((shadow, tint)) = cast(style) else {
        svg.push_str("<path");
        body(svg, &data, shape, view, None)?;
        group(svg);
        svg.push_str("/>\n");
        return Ok(());
    };
    let (dx, dy) = (
        view.length(shadow.dx()).map_err(in_shape("shadow dx "))?,
        view.length(shadow.dy()).map_err(in_shape("shadow dy "))?,
    );
    if shadow.blur() > 0.0 {
        let sigma = view.length(shadow.blur() / 2.0).map_err(in_shape("shadow blur "))?;
        // the region the filter works in: the canvas, and the farthest the shadow reaches (3 deviations and the offset)
        let reach = 3.0 * sigma + dx.abs().max(dy.abs());
        let (x, y, width, height) = (-reach, -reach, canvas.0 + 2.0 * reach, canvas.1 + 2.0 * reach);
        let id = format!("shadow-{}", shape.index());
        let _ = writeln!(
            svg,
            "<filter id=\"{id}\" filterUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" \
             color-interpolation-filters=\"sRGB\">",
            number(x),
            number(y),
            number(width),
            number(height),
        );
        let _ = write!(
            svg,
            "<feGaussianBlur in=\"SourceAlpha\" stdDeviation=\"{}\"/>\
             <feOffset dx=\"{}\" dy=\"{}\" result=\"b\"/>",
            number(sigma),
            number(dx),
            number(dy),
        );
        let [red, green, blue, alpha] = tint;
        let _ = write!(svg, "<feFlood flood-color=\"#{red:02x}{green:02x}{blue:02x}\"");
        if alpha < u8::MAX {
            let _ = write!(svg, " flood-opacity=\"{}\"", number(f64::from(alpha) / OPAQUE));
        }
        svg.push_str("/>");
        svg.push_str(
            "<feComposite in2=\"b\" operator=\"in\" result=\"s\"/>\
             <feMerge><feMergeNode in=\"s\"/><feMergeNode in=\"SourceGraphic\"/></feMerge>\n</filter>\n",
        );
        let _ = write!(svg, "<g filter=\"url(#{id})\"");
        group(svg);
        svg.push_str(">\n<path");
        body(svg, &data, shape, view, None)?;
        svg.push_str("/>\n</g>\n");
        return Ok(());
    }
    let grouped = style.opacity() < 1.0 || style.blend() == Blend::Multiply;
    if grouped {
        svg.push_str("<g");
        group(svg);
        svg.push_str(">\n");
    }
    let copy = device.translate(dx, dy).map_err(in_shape("shadow "))?.to_svg_path();
    svg.push_str("<path");
    body(svg, &copy, shape, view, Some(tint))?;
    svg.push_str("/>\n<path");
    body(svg, &data, shape, view, None)?;
    svg.push_str("/>\n");
    if grouped {
        svg.push_str("</g>\n");
    }
    Ok(())
}

/// Appends the attributes of the path `data` that paint it: the fill and the stroke of `style`. With a `tint` they are
/// its silhouette instead, in the colour of `tint` and its own alphas, under an `opacity` of the tint's alpha.
fn body(svg: &mut String, data: &str, shape: &Shape, view: &View, tint: Option<[u8; 4]>) -> Result<(), Error> {
    let style = shape.style();
    let _ = write!(svg, " d=\"{data}\"");
    let tinted = |color: Color| match (tint, color.rgba()) {
        (Some([red, green, blue, _]), Some([_, _, _, alpha])) => Color::from_bytes([red, green, blue, alpha]),
        _ => color,
    };
    paint(svg, "fill", tinted(style.fill()));
    if let Some(stroke) = style.stroke() {
        paint(svg, "stroke", tinted(stroke.color()));
        let _ = write!(
            svg,
            " stroke-width=\"{}\" stroke-linejoin=\"{}\" stroke-linecap=\"{}\" stroke-miterlimit=\"{}\"",
            number(
                view.length(stroke.width())
                    .map_err(|error| in_shape(shape, "stroke width ", error))?
            ),
            stroke.join().name(),
            stroke.cap().name(),
            number(stroke.miter_limit()),
        );
    }
    if let Some([_, _, _, alpha]) = tint
        && alpha < u8::MAX
    {
        let _ = write!(svg, " opacity=\"{}\"", number(f64::from(alpha) / OPAQUE));
    }
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
