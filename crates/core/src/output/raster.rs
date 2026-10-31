//! `Drawing::to_rgba`: the drawing as straight-alpha RGBA bytes. The only module that uses the rasteriser crates
//! (`vello_cpu` and `kurbo`); nothing of them reaches the API.

use super::view::View;
use super::{Size, in_shape};
use crate::path::Element;
use crate::{Drawing, Error, Shape, Values};
use kurbo::BezPath;
use vello_cpu::color::AlphaColor;
use vello_cpu::peniko::{Fill, ImageAlphaType};
use vello_cpu::{Level, Pixmap, RasterizerSettings, RenderContext, RenderMode, RenderSettings, Resources};

impl Drawing {
    /// The drawing as straight-alpha RGBA bytes of `size`, with the Params at the `values` given and at their defaults
    /// where none is given: rows from the top, four bytes a pixel, so `width * height * 4` bytes; transparent black
    /// where nothing is drawn.
    ///
    /// Shapes are placed as in [`Drawing::to_svg`] and painted in order.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `width` or `height` if a side of `size` is not a whole number from 1 to 65,535,
    /// or `size` if it has more than 2^27 pixels; otherwise as [`Drawing::to_svg`].
    pub fn to_rgba(&self, size: Size, values: &Values) -> Result<Vec<u8>, Error> {
        let (width, height) = size.checked_pixels()?;
        let shapes = self.shapes(values)?;
        let view = View::fit(self.size(), (f64::from(width), f64::from(height)));
        rgba(&shapes, &view, width, height)
    }
}

/// The picture of `shapes` seen through `view`, `width` by `height` pixels.
fn rgba(shapes: &[Shape], view: &View, width: u16, height: u16) -> Result<Vec<u8>, Error> {
    let mut context = RenderContext::new_with(
        width,
        height,
        RenderSettings {
            level: Level::new(),
            num_threads: 0,
        },
    );
    context.set_fill_rule(Fill::NonZero);
    for shape in shapes {
        paint(&mut context, shape, view)?;
    }
    context.flush();
    let mut pixmap = Pixmap::new(width, height);
    context.render_with(
        &mut pixmap,
        &mut Resources::new(),
        RasterizerSettings {
            render_mode: RenderMode::OptimizeSpeed,
            ..RasterizerSettings::default()
        },
    );
    let mut bytes = pixmap.take_rgba8(ImageAlphaType::AlphaPremultiplied);
    unpremultiply(&mut bytes);
    Ok(bytes)
}

/// Paints `shape` seen through `view`.
fn paint(context: &mut RenderContext, shape: &Shape, view: &View) -> Result<(), Error> {
    let path = view.path(shape.path()).map_err(|error| in_shape(shape, "", error))?;
    let outline = bez_path(&path.elements());
    if let Some([red, green, blue, alpha]) = shape.style().fill().rgba() {
        context.set_paint(AlphaColor::from_rgba8(red, green, blue, alpha));
        context.fill_path(&outline);
    }
    Ok(())
}

/// `elements` as the rasteriser's path.
fn bez_path(elements: &[Element]) -> BezPath {
    let at = |point: crate::Point| kurbo::Point::new(point.x, point.y);
    let mut outline = BezPath::new();
    for element in elements {
        match *element {
            Element::Move(to) => outline.move_to(at(to)),
            Element::Line(to) => outline.line_to(at(to)),
            Element::Quad(control, to) => outline.quad_to(at(control), at(to)),
            Element::Cubic(first, second, to) => outline.curve_to(at(first), at(second), at(to)),
            Element::Close => outline.close_path(),
        }
    }
    outline
}

/// Turns premultiplied RGBA bytes into straight alpha, in place: each colour byte becomes `colour * 255 / alpha`
/// rounded to the nearest whole number (halves up), and a pixel with alpha 0 becomes all zeros.
fn unpremultiply(bytes: &mut [u8]) {
    let (pixels, _) = bytes.as_chunks_mut::<4>();
    for [red, green, blue, alpha] in pixels {
        let alpha = u32::from(*alpha);
        for channel in [red, green, blue] {
            // a premultiplied colour is at most its alpha, so the quotient is at most 255
            let straight = (u32::from(*channel) * 255 + alpha / 2).checked_div(alpha).unwrap_or(0);
            #[expect(clippy::cast_possible_truncation, reason = "at most 255")]
            let byte = straight as u8;
            *channel = byte;
        }
    }
}
